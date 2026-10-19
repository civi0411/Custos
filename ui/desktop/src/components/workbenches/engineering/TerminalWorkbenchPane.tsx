import React, { useEffect, useRef, useState, useCallback } from 'react';
import {
  Terminal,
  Plus,
  Columns2,
  RotateCcw,
  Trash2,
  FolderGit2,
  X,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { useAppContext } from '@/context/AppContext';
import { XTerminalView, getClientPlatform } from '@/components/common/XTerminalView';
import type { ExecutionWorkspace, TerminalSession } from '@/types/domain';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';

// Module-level cache keeping session affinity per canvas tab across re-renders
const tabSessionMap = new Map<string, string>();

export interface TerminalWorkbenchPaneProps {
  tabId?: string;
  initialWorkspaceId?: string;
  activeWorkspaceId?: string;
  onSelectWorkspace?: (id: string) => void;
  initialCommand?: string;
  onOpenTab?: (
    id: OrcaTabId,
    customUrl?: string,
    customTitle?: string,
    targetPane?: 'primary' | 'split',
    targetWorkspaceId?: string
  ) => void;
}

export const TerminalWorkbenchPane: React.FC<TerminalWorkbenchPaneProps> = ({
  tabId,
  initialWorkspaceId,
  activeWorkspaceId: propActiveWorkspaceId,
  onSelectWorkspace: propOnSelectWorkspace,
  initialCommand,
  onOpenTab,
}) => {
  const { activeWorkspaceId: ctxActiveWorkspaceId, setActiveWorkspaceId: ctxSetActiveWorkspaceId } = useAppContext();
  const currentActiveWorkspaceId = propActiveWorkspaceId ?? ctxActiveWorkspaceId;
  const onSelectWorkspace = propOnSelectWorkspace ?? ctxSetActiveWorkspaceId;

  const [workspaces, setWorkspaces] = useState<ExecutionWorkspace[]>([]);
  const [activeWorkspaceId, setActiveWorkspaceId] = useState<string>(
    initialWorkspaceId || currentActiveWorkspaceId || ''
  );
  const [session, setSession] = useState<TerminalSession | null>(null);
  const [spawning, setSpawning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const activeWorkspaceRef = useRef(activeWorkspaceId);
  activeWorkspaceRef.current = activeWorkspaceId;
  const spawnAttemptedRef = useRef(false);

  const platform = getClientPlatform();
  const platformLabel =
    platform === 'macos' ? 'macOS (zsh)' : platform === 'windows' ? 'Windows (powershell)' : 'Linux (bash)';

  // Stable tab key identifier
  const tabKey = tabId || `default-terminal-${initialWorkspaceId || 'main'}`;

  // 1. Load registered workspaces
  const loadWorkspaces = useCallback(async () => {
    try {
      const list = await daemonClient.listWorkspaces();
      setWorkspaces(list);
      if (list.length > 0 && !activeWorkspaceId && !currentActiveWorkspaceId) {
        const readyWs = list.find((w) => w.status === 'ready') || list[0];
        setActiveWorkspaceId(readyWs.id);
        onSelectWorkspace?.(readyWs.id);
      }
    } catch (err: any) {
      setError(`Failed to fetch workspaces: ${err?.message || err}`);
    }
  }, [activeWorkspaceId, currentActiveWorkspaceId, onSelectWorkspace]);

  useEffect(() => {
    void loadWorkspaces();
  }, [loadWorkspaces]);

  // Synchronize when activeWorkspaceId changes externally (if tab isn't locked to specific workspace)
  useEffect(() => {
    if (!initialWorkspaceId && currentActiveWorkspaceId && currentActiveWorkspaceId !== activeWorkspaceId) {
      setActiveWorkspaceId(currentActiveWorkspaceId);
    }
  }, [currentActiveWorkspaceId, activeWorkspaceId, initialWorkspaceId]);

  // 2. Spawn a dedicated terminal session for this canvas tab
  const spawnSessionForTab = useCallback(async (workspaceId: string, cmdOverride?: string) => {
    if (!workspaceId) return;
    setSpawning(true);
    setError(null);
    try {
      const newSession = await daemonClient.spawnTerminal({
        workspace_id: workspaceId,
        command: cmdOverride,
        cols: 100,
        rows: 30,
      });
      tabSessionMap.set(tabKey, newSession.id);
      setSession(newSession);
    } catch (err: any) {
      setError(`Failed to spawn terminal: ${err?.message || err}`);
    } finally {
      setSpawning(false);
    }
  }, [tabKey]);

  // 3. Mount or bind existing session for this tab
  useEffect(() => {
    if (!activeWorkspaceId) return;
    let isCancelled = false;

    const initSession = async () => {
      const cachedSessionId = tabSessionMap.get(tabKey);
      if (cachedSessionId) {
        try {
          const list = await daemonClient.listTerminals(activeWorkspaceId);
          if (isCancelled) return;
          const found = list?.find((s) => s.id === cachedSessionId && s.status.type === 'active');
          if (found) {
            setSession(found);
            return;
          }
        } catch {
          // If check fails, fallback to spawning
        }
      }

      if (!spawnAttemptedRef.current && !isCancelled) {
        spawnAttemptedRef.current = true;
        void spawnSessionForTab(activeWorkspaceId, initialCommand);
      }
    };

    void initSession();

    return () => {
      isCancelled = true;
    };
  }, [activeWorkspaceId, tabKey, initialCommand, spawnSessionForTab]);

  // 4. Action: Restart session
  const handleRestart = async () => {
    if (session?.id) {
      try {
        await daemonClient.terminateTerminal(session.id);
      } catch {
        // Best effort
      }
    }
    tabSessionMap.delete(tabKey);
    spawnAttemptedRef.current = false;
    void spawnSessionForTab(activeWorkspaceId, initialCommand);
  };

  // 5. Action: Clear terminal
  const handleClear = () => {
    if (session?.id) {
      daemonClient.writeTerminal(session.id, '\x0c').catch(() => {});
    }
  };

  // 6. Action: Open a new terminal tab at the top-level canvas tab bar
  const handleNewTab = () => {
    onOpenTab?.('terminal', undefined, undefined, 'primary', activeWorkspaceId);
  };

  // 7. Action: Split terminal to the right
  const handleSplit = () => {
    onOpenTab?.('terminal', undefined, undefined, 'split', activeWorkspaceId);
  };

  const activeWs = workspaces.find((w) => w.id === activeWorkspaceId);
  const isSessionActive = session?.status.type === 'active';

  return (
    <div className="flex h-full w-full flex-col bg-[#0a0a0c] text-zinc-200 select-none overflow-hidden font-sans">
      {/* ── TOP UNIFIED TOOLBAR: REPOSITORY BADGE, STATUS, ACTIONS ── */}
      <div className="flex items-center justify-between gap-2 border-b border-white/8 bg-[#111215] px-3 py-1.5 shrink-0 min-h-[38px]">
        {/* Left: Repository Affinity + Shell Pill + Live Status */}
        <div className="flex items-center gap-2 min-w-0 flex-1 overflow-x-auto no-scrollbar">
          {/* Active Repository Tag */}
          <div className="flex items-center gap-1.5 px-2 py-1 rounded bg-white/[0.04] border border-white/6 shrink-0 text-xs text-white/90">
            <FolderGit2 className="size-3.5 text-workbench-accent shrink-0" />
            <span className="font-semibold text-[11px] truncate max-w-[150px]" title={activeWs?.name || 'Workspace'}>
              {activeWs?.name || 'Workspace'}
            </span>
            {activeWs?.active_branch && (
              <span className="px-1 py-0.2 rounded text-[9.5px] font-mono bg-white/10 text-white/70 shrink-0">
                {activeWs.active_branch}
              </span>
            )}
          </div>

          {/* Platform Shell Pill */}
          <span className="hidden sm:inline-block px-1.5 py-0.5 rounded text-[9.5px] font-mono bg-white/[0.03] text-white/50 border border-white/5 shrink-0">
            {platformLabel}
          </span>

          <div className="h-3.5 w-px bg-white/10 shrink-0" />

          {/* Live Process Status Indicator */}
          <div className="flex items-center gap-1.5 shrink-0">
            <span
              className={`size-2 rounded-full ${
                isSessionActive ? 'bg-emerald-400 animate-pulse' : 'bg-zinc-500'
              }`}
            />
            <span className="font-mono text-[11px] text-white/60">
              {isSessionActive ? 'Running' : session ? 'Exited' : spawning ? 'Launching...' : 'Idle'}
            </span>
          </div>
        </div>

        {/* Right: Actions Cluster (New Tab, Split, Clear, Restart) */}
        <div className="flex items-center gap-1 shrink-0">
          {/* Open New Terminal Tab in Canvas */}
          <button
            type="button"
            onClick={handleNewTab}
            disabled={!activeWorkspaceId}
            className="flex items-center gap-1 px-2 py-1 rounded text-[11px] font-medium text-white/80 hover:text-white bg-white/[0.04] hover:bg-white/[0.08] border border-white/8 transition cursor-pointer disabled:opacity-30"
            title="Mở thêm Tab Terminal mới trên thanh Tab (⌘T)"
          >
            <Plus className="size-3.5 text-workbench-accent" />
            <span className="hidden sm:inline">New Tab</span>
          </button>

          {/* Split Terminal Right */}
          <button
            type="button"
            onClick={handleSplit}
            disabled={!activeWorkspaceId}
            className="flex items-center gap-1 p-1.5 rounded text-white/60 hover:text-white hover:bg-white/[0.06] transition cursor-pointer disabled:opacity-30"
            title="Chia đôi Terminal sang khung bên phải (⌘\)"
          >
            <Columns2 className="size-3.5" />
          </button>

          <div className="h-3.5 w-px bg-white/10 mx-0.5" />

          {/* Clear Terminal Buffer */}
          <button
            type="button"
            onClick={handleClear}
            disabled={!isSessionActive}
            className="flex items-center gap-1 p-1.5 rounded text-white/60 hover:text-white hover:bg-white/[0.06] transition cursor-pointer disabled:opacity-30"
            title="Xóa nội dung màn hình (Clear / ⌃L)"
          >
            <Trash2 className="size-3.5" />
          </button>

          {/* Restart Shell */}
          <button
            type="button"
            onClick={handleRestart}
            disabled={spawning || !activeWorkspaceId}
            className="flex items-center gap-1 p-1.5 rounded text-white/60 hover:text-white hover:bg-white/[0.06] transition cursor-pointer disabled:opacity-30"
            title="Khởi động lại phiên làm việc (Restart shell)"
          >
            <RotateCcw className={`size-3.5 ${spawning ? 'animate-spin' : ''}`} />
          </button>
        </div>
      </div>

      {/* Error alert banner */}
      {error && (
        <div className="flex items-center justify-between border-b border-rose-500/20 bg-rose-500/10 px-3 py-1.5 text-xs text-rose-300 shrink-0">
          <span className="truncate">{error}</span>
          <button type="button" onClick={() => setError(null)} className="text-rose-400 hover:text-rose-200">
            <X className="size-3" />
          </button>
        </div>
      )}

      {/* ── REAL XTERM.JS TERMINAL VIEWPORT (100% Canvas) ── */}
      <div className="relative flex-1 overflow-hidden bg-[#0a0a0c]">
        {session?.id ? (
          <XTerminalView
            key={session.id}
            sessionId={session.id}
            onSessionTerminated={() => {
              setSession((prev) => (prev ? { ...prev, status: { type: 'exited' } } : null));
            }}
          />
        ) : (
          <div className="flex h-full flex-col items-center justify-center text-center text-zinc-500 py-12 select-none">
            <div className="space-y-3 max-w-sm">
              <Terminal className="mx-auto size-10 opacity-30 text-white/50" />
              <p className="text-sm font-medium text-white/80">
                {spawning ? 'Đang khởi động phiên terminal...' : 'Chưa có tiến trình terminal hoạt động'}
              </p>
              <p className="text-xs text-white/40">
                Khởi chạy shell cho{' '}
                <span className="text-workbench-accent font-semibold">{activeWs?.name || 'workspace'}</span>
              </p>
              <button
                type="button"
                onClick={() => void spawnSessionForTab(activeWorkspaceId, initialCommand)}
                disabled={spawning || !activeWorkspaceId}
                className="mt-2 inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-white/10 hover:bg-white/15 text-white text-xs font-medium transition cursor-pointer disabled:opacity-40"
              >
                <Plus className="size-3.5" />
                <span>Khởi chạy {platform === 'windows' ? 'PowerShell' : 'Shell'}</span>
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
