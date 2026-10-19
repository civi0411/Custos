import React, { useEffect, useRef, useState, useCallback } from 'react';
import {
  Terminal,
  Plus,
  X,
  RefreshCw,
  FolderGit2,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { useAppContext } from '@/context/AppContext';
import { XTerminalView, getClientPlatform } from '@/components/common/XTerminalView';
import type { ExecutionWorkspace, TerminalSession } from '@/types/domain';

export interface TerminalWorkbenchPaneProps {
  initialWorkspaceId?: string;
  activeWorkspaceId?: string;
  onSelectWorkspace?: (id: string) => void;
  initialCommand?: string;
}

export const TerminalWorkbenchPane: React.FC<TerminalWorkbenchPaneProps> = ({
  initialWorkspaceId,
  activeWorkspaceId: propActiveWorkspaceId,
  onSelectWorkspace: propOnSelectWorkspace,
  initialCommand,
}) => {
  const { activeWorkspaceId: ctxActiveWorkspaceId, setActiveWorkspaceId: ctxSetActiveWorkspaceId } = useAppContext();
  const currentActiveWorkspaceId = propActiveWorkspaceId ?? ctxActiveWorkspaceId;
  const onSelectWorkspace = propOnSelectWorkspace ?? ctxSetActiveWorkspaceId;

  const [workspaces, setWorkspaces] = useState<ExecutionWorkspace[]>([]);
  const [activeWorkspaceId, setActiveWorkspaceId] = useState<string>(currentActiveWorkspaceId || initialWorkspaceId || '');
  const [sessions, setSessions] = useState<TerminalSession[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string>('');
  const [spawning, setSpawning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const activeWorkspaceRef = useRef(activeWorkspaceId);
  activeWorkspaceRef.current = activeWorkspaceId;
  const appliedInitialWorkspace = useRef<string | undefined>(undefined);
  const sessionLoadEpoch = useRef(0);
  const autoSpawnDoneRef = useRef<string | null>(null);

  const platform = getClientPlatform();
  const platformLabel =
    platform === 'macos' ? 'macOS (zsh)' : platform === 'windows' ? 'Windows (powershell)' : 'Linux (bash)';

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

  // Synchronize when activeWorkspaceId changes externally (e.g. from Left Sidebar)
  useEffect(() => {
    if (currentActiveWorkspaceId && currentActiveWorkspaceId !== activeWorkspaceId) {
      setActiveSessionId('');
      setSessions([]);
      setActiveWorkspaceId(currentActiveWorkspaceId);
    }
  }, [currentActiveWorkspaceId, activeWorkspaceId]);

  useEffect(() => {
    if (initialWorkspaceId && appliedInitialWorkspace.current !== initialWorkspaceId && workspaces.some((w) => w.id === initialWorkspaceId)) {
      appliedInitialWorkspace.current = initialWorkspaceId;
      setActiveSessionId('');
      setSessions([]);
      setActiveWorkspaceId(initialWorkspaceId);
    }
  }, [initialWorkspaceId, workspaces]);

  // 2. Spawn a new terminal session
  const spawnSession = useCallback(async (cmdOverride?: string) => {
    if (!activeWorkspaceId) return;
    const workspaceId = activeWorkspaceId;
    setSpawning(true);
    setError(null);
    try {
      const session = await daemonClient.spawnTerminal({
        workspace_id: workspaceId,
        command: cmdOverride,
        cols: 100,
        rows: 30,
      });
      if (activeWorkspaceRef.current !== workspaceId) return;
      setSessions((prev) => [session, ...prev]);
      setActiveSessionId(session.id);
    } catch (err: any) {
      setError(`Failed to spawn terminal: ${err?.message || err}`);
    } finally {
      setSpawning(false);
    }
  }, [activeWorkspaceId]);

  // 3. Load active sessions for selected workspace
  const loadSessions = useCallback(async () => {
    const workspaceId = activeWorkspaceRef.current;
    if (!workspaceId) {
      setSessions([]);
      setActiveSessionId('');
      return;
    }
    const epoch = ++sessionLoadEpoch.current;
    try {
      const list = await daemonClient.listTerminals(workspaceId);
      if (epoch !== sessionLoadEpoch.current || activeWorkspaceRef.current !== workspaceId) return;
      const activeList = list ? list.filter((s) => s.status.type === 'active') : [];
      if (activeList.length > 0) {
        setSessions(list);
        setActiveSessionId((previous) => (activeList.some((s) => s.id === previous) ? previous : activeList[0].id));
      } else {
        // Automatically spawn a real live PTY session for this workspace
        if (autoSpawnDoneRef.current !== workspaceId) {
          autoSpawnDoneRef.current = workspaceId;
          void spawnSession(initialCommand);
        } else {
          setSessions(list || []);
          setActiveSessionId(list && list.length > 0 ? list[0].id : '');
        }
      }
      setError(null);
    } catch {
      if (epoch === sessionLoadEpoch.current) {
        if (autoSpawnDoneRef.current !== workspaceId) {
          autoSpawnDoneRef.current = workspaceId;
          void spawnSession(initialCommand);
        } else {
          setSessions([]);
          setActiveSessionId('');
        }
      }
    }
  }, [initialCommand, spawnSession]);

  useEffect(() => {
    setSessions([]);
    setActiveSessionId('');
    void loadSessions();
    return () => { sessionLoadEpoch.current++; };
  }, [activeWorkspaceId, loadSessions]);

  // 4. Terminate a terminal session
  const terminateSession = useCallback(async (sessionId: string, e?: React.MouseEvent) => {
    e?.stopPropagation();
    try {
      await daemonClient.terminateTerminal(sessionId);
      setSessions((prev) =>
        prev.map((s) => (s.id === sessionId ? { ...s, status: { type: 'terminated' } } : s))
      );
      if (activeSessionId === sessionId) {
        const remaining = sessions.filter((s) => s.id !== sessionId && s.status.type === 'active');
        if (remaining.length > 0) {
          setActiveSessionId(remaining[0].id);
        }
      }
    } catch (err: any) {
      setError(`Failed to terminate session: ${err?.message || err}`);
    }
  }, [activeSessionId, sessions]);

  const activeWs = workspaces.find((w) => w.id === activeWorkspaceId);
  const activeSess = sessions.find((s) => s.id === activeSessionId);

  return (
    <div className="flex h-full w-full flex-col bg-[#0a0a0c] text-zinc-200 select-none overflow-hidden font-sans">
      {/* ── TOP UNIFIED HEADER: REPOSITORY BADGE, MULTI-TABS, ACTIONS ── */}
      <div className="flex items-center justify-between gap-2 border-b border-white/8 bg-[#111215] px-3 py-1.5 shrink-0 min-h-[38px]">
        {/* Left: Repository Affinity + Session Tabs */}
        <div className="flex items-center gap-2 min-w-0 flex-1 overflow-x-auto no-scrollbar">
          {/* Active Repository Tag */}
          <div className="flex items-center gap-1.5 px-2 py-1 rounded bg-white/[0.04] border border-white/6 shrink-0 text-xs text-white/90">
            <FolderGit2 className="size-3.5 text-workbench-accent shrink-0" />
            <span className="font-semibold text-[11px] truncate max-w-[130px]" title={activeWs?.name || 'Workspace'}>
              {activeWs?.name || 'Workspace'}
            </span>
            {activeWs?.active_branch && (
              <span className="px-1 py-0.2 rounded text-[9.5px] font-mono bg-white/10 text-white/70 shrink-0">
                {activeWs.active_branch}
              </span>
            )}
          </div>

          {/* Platform Shell Pill */}
          <span className="hidden sm:inline-block px-1.5 py-0.5 rounded text-[9px] font-mono bg-white/[0.03] text-white/40 border border-white/5 shrink-0">
            {platformLabel}
          </span>

          <div className="h-4 w-px bg-white/10 shrink-0" />

          {/* Terminal Tabs */}
          <div className="flex items-center gap-1 min-w-0">
            {sessions.map((sess, idx) => {
              const isActive = sess.id === activeSessionId;
              const isTermActive = sess.status.type === 'active';
              const name = sess.command.split('/').pop() || `term-${idx + 1}`;

              return (
                <div
                  key={sess.id}
                  onClick={() => setActiveSessionId(sess.id)}
                  className={`group flex cursor-pointer items-center gap-1.5 rounded px-2.5 py-1 text-xs transition select-none ${
                    isActive
                      ? 'bg-white/[0.08] text-white border border-white/12 font-medium shadow-xs'
                      : 'text-white/60 hover:bg-white/[0.04] hover:text-white/90 border border-transparent'
                  }`}
                  title={`${sess.command} (${sess.cols}x${sess.rows})`}
                >
                  <span
                    className={`size-1.5 rounded-full shrink-0 ${
                      isTermActive ? 'bg-emerald-400 animate-pulse' : 'bg-zinc-500'
                    }`}
                  />
                  <span className="font-mono text-[11px] truncate max-w-[90px]">
                    {name}
                  </span>
                  <button
                    type="button"
                    onClick={(e) => void terminateSession(sess.id, e)}
                    className="opacity-0 group-hover:opacity-100 rounded p-0.5 hover:bg-white/10 text-white/40 hover:text-white transition"
                    title="Terminate session"
                  >
                    <X className="size-2.5" />
                  </button>
                </div>
              );
            })}

            {/* New Terminal Button */}
            <button
              type="button"
              onClick={() => void spawnSession()}
              disabled={spawning || !activeWorkspaceId}
              className="size-6 flex items-center justify-center rounded text-white/50 hover:text-white hover:bg-white/[0.06] transition disabled:opacity-30 shrink-0 cursor-pointer"
              title="New Terminal Session"
            >
              <Plus className="size-3.5" />
            </button>
          </div>
        </div>

        {/* Right: Actions Cluster */}
        <div className="flex items-center gap-1.5 shrink-0">
          {activeSess && (
            <span className="hidden font-mono text-[10px] text-white/40 md:inline-block px-1.5 py-0.5 rounded bg-white/[0.03] border border-white/5">
              {activeSess.cols}x{activeSess.rows} • {activeSess.status.type}
            </span>
          )}

          {/* Refresh Sessions */}
          <button
            type="button"
            onClick={() => void loadSessions()}
            className="size-7 flex items-center justify-center rounded text-white/50 hover:text-white hover:bg-white/[0.06] transition cursor-pointer"
            title="Refresh sessions"
          >
            <RefreshCw className="size-3.5" />
          </button>
        </div>
      </div>

      {/* Error alert banner */}
      {error && (
        <div className="flex items-center justify-between border-b border-rose-500/20 bg-rose-500/10 px-3 py-1 text-xs text-rose-300 shrink-0">
          <span className="truncate">{error}</span>
          <button type="button" onClick={() => setError(null)} className="text-rose-400 hover:text-rose-200">
            <X className="size-3" />
          </button>
        </div>
      )}

      {/* ── REAL XTERM.JS TERMINAL VIEWPORT ── */}
      <div className="relative flex-1 overflow-hidden bg-[#0a0a0c]">
        {activeSessionId ? (
          <XTerminalView
            key={activeSessionId}
            sessionId={activeSessionId}
            onSessionTerminated={() => {
              setSessions((prev) =>
                prev.map((s) => (s.id === activeSessionId ? { ...s, status: { type: 'exited' } } : s))
              );
            }}
          />
        ) : (
          <div className="flex h-full flex-col items-center justify-center text-center text-zinc-500 py-12 select-none">
            <div className="space-y-2 max-w-sm">
              <Terminal className="mx-auto size-9 opacity-30 text-white/50" />
              <p className="text-sm font-medium text-white/80">No active terminal session</p>
              <p className="text-xs text-white/40">
                Launch a live shell session for{' '}
                <span className="text-workbench-accent font-semibold">{activeWs?.name || 'workspace'}</span>
              </p>
              <button
                type="button"
                onClick={() => void spawnSession()}
                disabled={spawning || !activeWorkspaceId}
                className="mt-2 inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-white/10 hover:bg-white/15 text-white text-xs font-medium transition cursor-pointer"
              >
                <Plus className="size-3.5" />
                <span>Launch {platform === 'windows' ? 'PowerShell' : 'Shell'}</span>
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
