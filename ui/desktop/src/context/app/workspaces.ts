import { useState, useCallback, useEffect, useRef } from 'react';
import { daemonClient } from '@/api/daemon_client';
import type { ExecutionWorkspace, WorkspaceDetectionResult } from '@/types/domain';

const STORAGE_KEY_ACTIVE_WORKSPACE = 'custos.active_workspace_id';
const LEGACY_STORAGE_KEY_ACTIVE_WORKSPACE = 'custos_active_workspace_id';

function getStoredActiveWorkspaceId(): string {
  try {
    return localStorage.getItem(STORAGE_KEY_ACTIVE_WORKSPACE)
      || localStorage.getItem(LEGACY_STORAGE_KEY_ACTIVE_WORKSPACE)
      || '';
  } catch {
    return '';
  }
}

export interface WorkspacesSlice {
  workspaces: ExecutionWorkspace[];
  activeWorkspaceId: string;
  setActiveWorkspaceId: (id: string) => void;
  activeWorkspace: ExecutionWorkspace | null;
  refreshWorkspaces: () => Promise<ExecutionWorkspace[]>;
  activeDetection: WorkspaceDetectionResult | null;
  setActiveDetection: React.Dispatch<React.SetStateAction<WorkspaceDetectionResult | null>>;
  refreshWorkspaceDetection: () => Promise<void>;
  handleProjectOpened: (project: { name: string; path: string; isGit: boolean; branch?: string }) => void;
  handleOpenFolderDirect: () => Promise<void>;
}

export interface WorkspaceSliceDeps {
  currentProject: string;
  setCurrentProject: (name: string) => void;
  onProjectRegistered: (projectName: string) => void;
  onModelSelect?: (model: string) => void;
  onOpenProjectFallback?: () => void;
  showToast: (msg: string) => void;
}

export function useWorkspacesSlice({
  setCurrentProject,
  onProjectRegistered,
  onModelSelect,
  onOpenProjectFallback,
  showToast,
}: WorkspaceSliceDeps): WorkspacesSlice {
  const [workspaces, setWorkspaces] = useState<ExecutionWorkspace[]>([]);
  const [activeWorkspaceId, setActiveWorkspaceIdState] = useState<string>(() => {
    return getStoredActiveWorkspaceId();
  });
  const [activeDetection, setActiveDetection] = useState<WorkspaceDetectionResult | null>(null);
  const refreshSeqRef = useRef(0);

  const setActiveWorkspaceId = useCallback((id: string) => {
    setActiveWorkspaceIdState(id);
    try {
      if (id) {
        localStorage.setItem(STORAGE_KEY_ACTIVE_WORKSPACE, id);
        const worktreeModel = localStorage.getItem(`custos.worktree_model.${id}`);
        if (worktreeModel) {
          onModelSelect?.(worktreeModel);
          localStorage.setItem('custos_selected_model', worktreeModel);
        }
      }
    } catch {}
  }, [onModelSelect]);

  const refreshWorkspaces = useCallback(async () => {
    const seq = ++refreshSeqRef.current;
    try {
      const list = await daemonClient.listWorkspaces();
      if (seq !== refreshSeqRef.current) return list;

      const valid = list.filter((w) => {
        const s = typeof w.status === 'string' ? w.status : (w.status as any)?.setup_failed ? 'failed' : 'unknown';
        return s !== 'archived';
      });

      // Live sync dirty manifest for active workspace so left badge, header, and diff are always synchronized
      const currentActive = getStoredActiveWorkspaceId() || activeWorkspaceId || '';
      if (currentActive && valid.some((w) => w.id === currentActive)) {
        try {
          const dirty = await daemonClient.inspectWorkspaceDirty(currentActive);
          if (seq === refreshSeqRef.current) {
            const idx = valid.findIndex((w) => w.id === currentActive);
            if (idx >= 0) {
              valid[idx] = { ...valid[idx], dirty_manifest: dirty, dirtyManifest: dirty };
            }
          }
        } catch {}
      }

      if (seq === refreshSeqRef.current) {
        setWorkspaces(valid);
        if (valid.length > 0) {
          setActiveWorkspaceIdState((prev) => {
            if (prev && valid.some((w) => w.id === prev)) return prev;
            const ready = valid.find((w) => w.status === 'ready') || valid[0];
            return ready.id;
          });
        }
      }
      return valid;
    } catch (e) {
      console.warn('[WorkspacesSlice] Failed to load workspaces:', e);
      // Preserve canonical state on transient failure instead of resetting to empty
      return [];
    }
  }, [activeWorkspaceId]);

  const activeWorkspace = workspaces.find((w) => w.id === activeWorkspaceId) || null;

  // Single authoritative polling loop
  useEffect(() => {
    void refreshWorkspaces();
    const interval = setInterval(() => {
      if (!document.hidden) void refreshWorkspaces();
    }, 4000);
    const onFocus = () => {
      void refreshWorkspaces();
    };
    window.addEventListener('focus', onFocus);
    return () => {
      clearInterval(interval);
      window.removeEventListener('focus', onFocus);
    };
  }, [refreshWorkspaces]);

  const refreshWorkspaceDetection = useCallback(async () => {
    const currentPath = localStorage.getItem('custos_current_project_path') || '.';
    try {
      const det = await daemonClient.detectWorkspace(currentPath);
      setActiveDetection(det);
      if (det.name && det.name !== 'Project') {
        setCurrentProject(det.name);
      }
    } catch (e) {
      console.warn('Failed to detect workspace:', e);
    }
  }, [setCurrentProject]);

  const handleProjectOpened = useCallback(
    (project: { name: string; path: string; isGit: boolean; branch?: string }) => {
      onProjectRegistered(project.name);
      setCurrentProject(project.name);
      try {
        localStorage.setItem('custos_current_project', project.name);
        localStorage.setItem('custos_current_project_path', project.path);
      } catch {}
      if (project.path) {
        daemonClient
          .detectWorkspace(project.path)
          .then((det) => {
            setActiveDetection(det);
          })
          .catch(() => {});
      }
    },
    [onProjectRegistered, setCurrentProject]
  );

  const handleOpenFolderDirect = useCallback(async () => {
    try {
      const chosen = await daemonClient.pickFolder();
      if (!chosen) return;
      const detection = await daemonClient.detectWorkspace(chosen);
      setActiveDetection(detection);
      const projectName = chosen.split('/').filter(Boolean).pop() || detection.name || 'Project';

      if (detection.sub_repositories && detection.sub_repositories.length > 0) {
        let firstWsId: string | null = null;
        for (const sub of detection.sub_repositories) {
          try {
            const ws = await daemonClient.createWorkspace({
              name: sub.name,
              path: sub.path,
              kind: {
                type: 'git',
                repo_path: sub.path,
                branch: sub.branch || 'main',
              },
            });
            if (!firstWsId && ws?.id) firstWsId = ws.id;
          } catch (e) {
            console.warn(`Failed to auto-create workspace for sub-repo ${sub.name}:`, e);
          }
        }

        handleProjectOpened({
          name: projectName,
          path: chosen,
          isGit: false,
        });

        const refreshed = await refreshWorkspaces();
        if (firstWsId) {
          setActiveWorkspaceId(firstWsId);
        } else if (refreshed && refreshed[0]) {
          setActiveWorkspaceId(refreshed[0].id);
        }

        showToast(`Opened project "${projectName}" with ${detection.sub_repositories.length} workspaces.`);
        return;
      }

      const name = detection.name || projectName;
      const isGit = detection.is_git;
      const branch = detection.branch || 'main';

      const ws = await daemonClient.createWorkspace({
        name,
        path: detection.root_path || chosen,
        kind: isGit
          ? {
              type: 'git',
              repo_path: detection.root_path || chosen,
              branch,
            }
          : {
              type: 'folder',
              path: detection.root_path || chosen,
            },
      });

      handleProjectOpened({
        name: projectName,
        path: detection.root_path || chosen,
        isGit,
        branch: isGit ? branch : undefined,
      });

      const refreshed = await refreshWorkspaces();
      if (ws?.id) {
        setActiveWorkspaceId(ws.id);
      } else if (refreshed && refreshed[0]) {
        setActiveWorkspaceId(refreshed[0].id);
      }
      showToast(`Opened folder "${name}" as a workspace.`);
    } catch (e) {
      console.warn('Direct pick folder fallback to dialog:', e);
      if (onOpenProjectFallback) {
        onOpenProjectFallback();
      } else {
        showToast(`Error opening folder: ${e instanceof Error ? e.message : String(e)}`);
      }
    }
  }, [handleProjectOpened, onOpenProjectFallback, refreshWorkspaces, setActiveWorkspaceId, showToast]);

  // Initial detection and auto-registration
  useEffect(() => {
    const initWorkspace = async () => {
      const currentPath = localStorage.getItem('custos_current_project_path');
      const candidates = [
        currentPath,
        '.',
        '..',
      ].filter(Boolean) as string[];

      for (const probePath of candidates) {
        try {
          const det = await daemonClient.detectWorkspace(probePath);
          if (det && ((det.sub_repositories && det.sub_repositories.length > 0) || det.is_git)) {
            setActiveDetection(det);
            const detectedPath = det.root_path || det.path || probePath;
            try {
              localStorage.setItem('custos_current_project_path', detectedPath);
            } catch {}
            const projName = (det.name && det.name !== 'Project') ? det.name : 'AgentHub';
            setCurrentProject(projName);
            onProjectRegistered(projName);

            if (det.sub_repositories && det.sub_repositories.length > 0) {
              const existingList = await daemonClient.listWorkspaces().catch(() => []);
              let firstId: string | null = null;
              for (const sub of det.sub_repositories) {
                const found = existingList.find((w) => w.path === sub.path);
                if (!found) {
                  const ws = await daemonClient.createWorkspace({
                    name: sub.name,
                    path: sub.path,
                    kind: {
                      type: 'git',
                      repo_path: sub.path,
                      branch: sub.branch || 'main',
                    },
                  }).catch(() => null);
                  if (!firstId && ws?.id) firstId = ws.id;
                }
              }
              const refreshed = await refreshWorkspaces();
              const storedActive = getStoredActiveWorkspaceId();
              if (!storedActive) {
                const preferred = refreshed.find((w) => w.name === 'Custos') || refreshed[0];
                if (preferred) setActiveWorkspaceId(preferred.id);
              }
            } else {
              const existingList = await daemonClient.listWorkspaces().catch(() => []);
              if (existingList.length === 0 && detectedPath) {
                const ws = await daemonClient.createWorkspace({
                  name: det.name || 'Workspace',
                  path: detectedPath,
                  kind: det.is_git ? {
                    type: 'git',
                    repo_path: detectedPath,
                    branch: det.branch || 'main',
                  } : {
                    type: 'folder',
                    path: detectedPath,
                  },
                }).catch(() => null);
                if (ws?.id) {
                  setActiveWorkspaceId(ws.id);
                  await refreshWorkspaces();
                }
              }
            }
            return;
          }
        } catch {}
      }
    };

    void initWorkspace();
  }, [refreshWorkspaces, setActiveWorkspaceId, setCurrentProject, onProjectRegistered]);

  return {
    workspaces,
    activeWorkspaceId,
    setActiveWorkspaceId,
    activeWorkspace,
    refreshWorkspaces,
    activeDetection,
    setActiveDetection,
    refreshWorkspaceDetection,
    handleProjectOpened,
    handleOpenFolderDirect,
  };
}
