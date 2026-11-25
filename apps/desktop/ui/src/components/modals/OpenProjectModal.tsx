import React, { useState, useEffect, useCallback } from 'react';
import {
  FolderGit2,
  Folder,
  FolderOpen,
  GitBranch,
  X,
  AlertCircle,
  Loader2,
  ArrowRight,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { WorkspaceDetectionResult } from '@/types/domain';

interface OpenProjectModalProps {
  isOpen: boolean;
  onClose: () => void;
  onProjectOpened: (project: { name: string; path: string; isGit: boolean; branch?: string }) => void;
  onShowToast?: (msg: string) => void;
}

export const OpenProjectModal: React.FC<OpenProjectModalProps> = ({
  isOpen,
  onClose,
  onProjectOpened,
  onShowToast,
}) => {
  const [folderPath, setFolderPath] = useState('');
  const [isDetecting, setIsDetecting] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [detection, setDetection] = useState<WorkspaceDetectionResult | null>(null);
  const [detectError, setDetectError] = useState<string | null>(null);
  const [recentWorkspaces, setRecentWorkspaces] = useState<{ label: string; path: string }[]>([]);

  useEffect(() => {
    if (!isOpen) {
      setFolderPath('');
      setDetection(null);
      setDetectError(null);
      return;
    }
    daemonClient
      .listWorkspaces()
      .then((list) => {
        setRecentWorkspaces(
          list
            .filter((w) => w.path && w.status !== 'archived')
            .map((w) => ({ label: w.name || w.path.split('/').pop() || w.id, path: w.path }))
        );
      })
      .catch(() => {});
  }, [isOpen]);

  const runDetection = useCallback(async (path: string) => {
    const trimmed = path.trim();
    if (!trimmed) {
      setDetection(null);
      setDetectError(null);
      return;
    }

    try {
      setIsDetecting(true);
      setDetectError(null);
      const res = await daemonClient.detectWorkspace(trimmed);
      setDetection(res);
    } catch (err: any) {
      setDetection(null);
      setDetectError(err?.message || 'Failed to analyze path');
    } finally {
      setIsDetecting(false);
    }
  }, []);

  const handlePathChange = (val: string) => {
    setFolderPath(val);
    if (val.trim().length > 2) {
      runDetection(val);
    } else {
      setDetection(null);
      setDetectError(null);
    }
  };

  const handleBrowseFolder = async () => {
    try {
      setIsDetecting(true);
      const chosen = await daemonClient.pickFolder();
      if (chosen) {
        setFolderPath(chosen);
        const res = await daemonClient.detectWorkspace(chosen);
        setDetection(res);

        // Auto open immediately without requiring a second click
        const targetPath = (res?.root_path || chosen).trim();
        const name = res?.name || targetPath.split('/').filter(Boolean).pop() || 'Project';
        const isGit = res?.is_git ?? false;
        const branch = res?.branch || 'main';

        await daemonClient.createWorkspace({
          name,
          path: targetPath,
          kind: isGit
            ? { type: 'git', repo_path: targetPath, branch }
            : { type: 'folder', path: targetPath },
        });

        onProjectOpened({
          name,
          path: targetPath,
          isGit,
          branch: isGit ? branch : undefined,
        });

        onShowToast?.(`Opened ${isGit ? 'Git repository' : 'folder'}: ${name}`);
        onClose();
        return;
      }
    } catch (err: any) {
      onShowToast?.(`Folder picker error: ${err?.message || err}`);
    } finally {
      setIsDetecting(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const targetPath = (detection?.root_path || folderPath).trim();
    if (!targetPath) return;

    try {
      setIsSubmitting(true);
      const name = detection?.name || targetPath.split('/').filter(Boolean).pop() || 'Project';
      const isGit = detection?.is_git ?? false;
      const branch = detection?.branch || 'main';

      // Register ExecutionWorkspace in daemon
      await daemonClient.createWorkspace({
        name,
        path: targetPath,
        kind: isGit
          ? {
              type: 'git',
              repo_path: targetPath,
              branch,
            }
          : {
              type: 'folder',
              path: targetPath,
            },
      });

      onProjectOpened({
        name,
        path: targetPath,
        isGit,
        branch: isGit ? branch : undefined,
      });

      onShowToast?.(`Opened ${isGit ? 'Git repository' : 'folder'}: ${name}`);
      onClose();
    } catch (err: any) {
      onShowToast?.(`Failed to open project: ${err?.message || err}`);
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-3 select-none">
      <div className="w-full max-w-lg bg-surface-1 border border-border-default rounded-2xl shadow-2xl flex flex-col overflow-hidden max-h-[90vh] animate-in fade-in zoom-in-95 duration-150">
        {/* Header */}
        <div className="h-12 border-b border-border-muted px-5 flex items-center justify-between shrink-0 bg-surface-1">
          <div className="flex items-center gap-2">
            <FolderGit2 className="w-4 h-4 workbench-accent" />
            <span className="text-sm font-semibold text-fg-editor">Open Project or Repository</span>
          </div>
          <button
            onClick={onClose}
            className="p-1 hover:bg-surface-2 rounded-md text-fg-muted hover:text-fg-editor transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Form Body */}
        <form onSubmit={handleSubmit} className="flex flex-col min-h-0 flex-1">
          <div className="p-5 space-y-4 text-xs overflow-y-auto">
            <p className="text-fg-muted text-[12px] leading-relaxed">
              Select a project directory or Git repository on your machine. Custos will automatically detect the repository root, active branch, dirty status, and runtime environment.
            </p>

            {/* Path Input & Browse Button */}
            <div className="space-y-1.5">
              <label className="font-medium text-fg-editor flex items-center justify-between">
                <span>Directory Path on Disk</span>
                <span className="text-[11px] text-fg-subtle">POSIX or ~ path</span>
              </label>
              <div className="flex items-center gap-2">
                <div className="relative flex-1">
                  <input
                    type="text"
                    value={folderPath}
                    onChange={(e) => handlePathChange(e.target.value)}
                    placeholder="/Users/username/Project/my-repo"
                    className="w-full bg-surface-0 border border-border-default rounded-lg pl-3 pr-8 py-2 text-fg-editor font-mono text-[11.5px] placeholder-fg-subtle focus:outline-none focus:border-border-emphasis"
                    autoFocus
                  />
                  {isDetecting && (
                    <Loader2 className="w-3.5 h-3.5 absolute right-2.5 top-2.5 animate-spin text-fg-muted" />
                  )}
                </div>
                <button
                  type="button"
                  onClick={handleBrowseFolder}
                  className="px-3 py-2 bg-surface-2 hover:bg-surface-3 border border-border-default rounded-lg text-fg-editor font-medium transition flex items-center gap-1.5 shrink-0"
                >
                  <FolderOpen className="w-3.5 h-3.5" />
                  <span>Browse...</span>
                </button>
              </div>
            </div>

            {/* Recent Workspaces */}
            {recentWorkspaces.length > 0 && (
              <div className="space-y-1">
                <div className="text-[11px] text-fg-subtle font-medium">Recent Workspaces:</div>
                <div className="flex flex-wrap gap-1.5">
                  {recentWorkspaces.map((ws) => (
                    <button
                      key={ws.path}
                      type="button"
                      onClick={() => {
                        setFolderPath(ws.path);
                        runDetection(ws.path);
                      }}
                      className={`px-2 py-1 rounded-md text-[11px] font-mono transition border ${
                        folderPath === ws.path
                          ? 'bg-accent text-fg-on-emphasis border-accent'
                          : 'bg-surface-2 hover:bg-surface-3 text-fg-muted hover:text-fg-editor border-border-muted'
                      }`}
                    >
                      {ws.label}
                    </button>
                  ))}
                </div>
              </div>
            )}

            {/* Detection Card */}
            {detection && (
              <div className="rounded-xl border border-border-default bg-surface-0 p-3.5 space-y-2.5 animate-in fade-in duration-100">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    {detection.is_git ? (
                      <FolderGit2 className="w-4 h-4 text-emerald-400" />
                    ) : (
                      <Folder className="w-4 h-4 text-blue-400" />
                    )}
                    <span className="font-semibold text-fg-editor text-[13px]">{detection.name}</span>
                    <span className="px-1.5 py-0.5 rounded text-[10px] bg-surface-2 border border-border-muted text-fg-muted font-medium">
                      {detection.project_type}
                    </span>
                  </div>

                  <span
                    className={`flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full font-medium ${
                      detection.is_dirty
                        ? 'bg-amber-500/10 text-amber-400 border border-amber-500/20'
                        : 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20'
                    }`}
                  >
                    <span
                      className={`w-1.5 h-1.5 rounded-full ${
                        detection.is_dirty ? 'bg-amber-400' : 'bg-emerald-400'
                      }`}
                    />
                    {detection.is_dirty ? `${detection.dirty_count} changed files` : 'Clean tree'}
                  </span>
                </div>

                <div className="space-y-1 text-[11px] text-fg-muted font-mono pt-1 border-t border-border-subtle">
                  <div className="truncate">
                    <span className="text-fg-subtle">Root:</span> {detection.root_path}
                  </div>
                  {detection.is_git && (
                    <div className="flex items-center gap-3">
                      <span className="flex items-center gap-1 text-fg-editor">
                        <GitBranch className="w-3 h-3 text-emerald-400" />
                        <span>{detection.branch || 'HEAD'}</span>
                      </span>
                      {detection.is_worktree && (
                        <span className="text-amber-400 bg-amber-400/10 px-1 rounded text-[10px]">
                          Linked worktree
                        </span>
                      )}
                      {detection.remote_url && (
                        <span className="truncate text-fg-subtle max-w-[220px]">
                          {detection.remote_url}
                        </span>
                      )}
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* Detect Error */}
            {detectError && (
              <div className="rounded-xl border border-red-500/20 bg-red-500/10 p-3 text-red-400 text-[11.5px] flex items-center gap-2">
                <AlertCircle className="w-4 h-4 shrink-0" />
                <span>{detectError}</span>
              </div>
            )}
          </div>

          {/* Footer */}
          <div className="h-12 border-t border-border-muted px-5 flex items-center justify-end gap-2 shrink-0 bg-surface-1">
            <button
              type="button"
              onClick={onClose}
              className="px-3 py-1.5 rounded-lg text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition text-xs font-medium"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={(!folderPath.trim() && !detection) || isSubmitting}
              className="px-4 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 disabled:opacity-40 text-white transition text-xs font-medium flex items-center gap-1.5 shadow-md shadow-emerald-950/20"
            >
              {isSubmitting ? (
                <>
                  <Loader2 className="w-3.5 h-3.5 animate-spin" />
                  <span>Opening...</span>
                </>
              ) : (
                <>
                  <span>Open Project</span>
                  <ArrowRight className="w-3.5 h-3.5" />
                </>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
