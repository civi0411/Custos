import React, { useEffect, useRef, useState } from 'react';
import {
  CornerDownLeft,
  Plus,
  RefreshCw,
  Terminal,
  X,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { ExecutionWorkspace, TerminalSession } from '@/types/domain';

export const TerminalWorkbenchPane: React.FC = () => {
  const [workspaces, setWorkspaces] = useState<ExecutionWorkspace[]>([]);
  const [activeWorkspaceId, setActiveWorkspaceId] = useState<string>('');
  const [sessions, setSessions] = useState<TerminalSession[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string>('');
  const [outputLogs, setOutputLogs] = useState<Record<string, string>>({});
  const [nextSeqs, setNextSeqs] = useState<Record<string, number>>({});
  const [inputCommand, setInputCommand] = useState('');
  const [loading, setLoading] = useState(true);
  const [spawning, setSpawning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const outputEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // 1. Load registered workspaces
  const loadWorkspaces = async () => {
    try {
      const list = await daemonClient.listWorkspaces();
      setWorkspaces(list);
      if (list.length > 0 && !activeWorkspaceId) {
        const readyWs = list.find((w) => w.status === 'ready') || list[0];
        setActiveWorkspaceId(readyWs.id);
      }
    } catch (err: any) {
      setError(`Failed to fetch workspaces: ${err?.message || err}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadWorkspaces();
  }, []);

  // 2. Load active sessions for selected workspace
  const loadSessions = async () => {
    if (!activeWorkspaceId) return;
    try {
      const list = await daemonClient.listTerminals(activeWorkspaceId);
      setSessions(list);
      if (list.length > 0 && (!activeSessionId || !list.some((s) => s.id === activeSessionId))) {
        setActiveSessionId(list[0].id);
      }
    } catch (err: any) {
      console.warn('Failed to list terminal sessions:', err);
    }
  };

  useEffect(() => {
    loadSessions();
  }, [activeWorkspaceId]);

  // 3. Spawn a new terminal session
  const spawnSession = async (cmdOverride?: string) => {
    if (!activeWorkspaceId) return;
    setSpawning(true);
    setError(null);
    try {
      const session = await daemonClient.spawnTerminal({
        workspace_id: activeWorkspaceId,
        command: cmdOverride,
        cols: 100,
        rows: 30,
      });
      setSessions((prev) => [session, ...prev]);
      setActiveSessionId(session.id);
      setOutputLogs((prev) => ({ ...prev, [session.id]: '' }));
      setNextSeqs((prev) => ({ ...prev, [session.id]: 0 }));
    } catch (err: any) {
      setError(`Failed to spawn terminal: ${err?.message || err}`);
    } finally {
      setSpawning(false);
    }
  };

  // 4. Terminate a terminal session
  const terminateSession = async (sessionId: string, e?: React.MouseEvent) => {
    e?.stopPropagation();
    try {
      await daemonClient.terminateTerminal(sessionId);
      setSessions((prev) =>
        prev.map((s) => (s.id === sessionId ? { ...s, status: { type: 'terminated' } } : s))
      );
    } catch (err: any) {
      setError(`Failed to terminate session: ${err?.message || err}`);
    }
  };

  // 5. Send input bytes to active terminal
  const sendInput = async (data: string) => {
    if (!activeSessionId) return;
    try {
      await daemonClient.writeTerminal(activeSessionId, data);
    } catch (err: any) {
      setError(`Failed to write to terminal: ${err?.message || err}`);
    }
  };

  const handleCommandSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!inputCommand.trim() && inputCommand !== '') return;
    sendInput(inputCommand + '\n');
    setInputCommand('');
  };

  // 6. Polling loop for active session output stream
  useEffect(() => {
    if (!activeSessionId) return;
    let isSubscribed = true;

    const poll = async () => {
      const currentSeq = nextSeqs[activeSessionId] || 0;
      try {
        const chunk = await daemonClient.readTerminal(activeSessionId, currentSeq, 65536);
        if (!isSubscribed) return;

        if (chunk.data && chunk.data.length > 0) {
          setOutputLogs((prev) => ({
            ...prev,
            [activeSessionId]: (prev[activeSessionId] || '') + chunk.data,
          }));
          setNextSeqs((prev) => ({
            ...prev,
            [activeSessionId]: chunk.nextSeq ?? chunk.next_seq,
          }));
        }

        if (chunk.isEof ?? chunk.is_eof) {
          setSessions((prev) =>
            prev.map((s) =>
              s.id === activeSessionId && s.status.type === 'active'
                ? { ...s, status: { type: 'exited' } }
                : s
            )
          );
        }
      } catch (err) {
        console.warn('Terminal poll error:', err);
      }
    };

    poll();
    const interval = setInterval(poll, 180);
    return () => {
      isSubscribed = false;
      clearInterval(interval);
    };
  }, [activeSessionId, nextSeqs]);

  // 7. Auto-scroll output
  useEffect(() => {
    outputEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [outputLogs, activeSessionId]);

  const activeWs = workspaces.find((w) => w.id === activeWorkspaceId);
  const activeSess = sessions.find((s) => s.id === activeSessionId);
  const currentOutput = (activeSessionId && outputLogs[activeSessionId]) || '';

  return (
    <div className="flex h-full flex-col bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
      {/* Header & Workspace Affinity Bar */}
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] px-4 py-2.5">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <Terminal className="h-4 w-4 text-[var(--color-fg-muted)]" />
            <span className="text-xs font-semibold tracking-tight">PTY Terminal</span>
          </div>

          <div className="h-4 w-px bg-[var(--color-border-muted)]" />

          {/* Execution Workspace Selector */}
          <div className="flex items-center gap-1.5 text-xs">
            <span className="text-[10px] uppercase tracking-wider text-[var(--color-fg-subtle)]">
              Workspace:
            </span>
            <select
              value={activeWorkspaceId}
              onChange={(e) => setActiveWorkspaceId(e.target.value)}
              className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2 py-1 text-xs font-medium text-[var(--color-fg-default)] outline-none hover:bg-[var(--color-surface-3)]"
            >
              {workspaces.map((ws) => (
                <option key={ws.id} value={ws.id}>
                  {ws.name} ({ws.kind.type === 'git' ? 'git' : 'folder'})
                </option>
              ))}
            </select>
          </div>

          {activeWs && (
            <span className="hidden truncate font-mono text-[10px] text-[var(--color-fg-subtle)] md:inline-block max-w-[260px]">
              {activeWs.path}
            </span>
          )}

          {activeSess && (
            <span className="hidden font-mono text-[10px] text-[var(--color-fg-subtle)] lg:inline-block">
              {activeSess.cols}x{activeSess.rows} • {activeSess.status.type}
            </span>
          )}
        </div>

        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => spawnSession()}
            disabled={spawning || !activeWorkspaceId}
            className="flex items-center gap-1.5 rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1 text-xs font-medium text-[var(--color-fg-default)] hover:bg-[var(--color-surface-3)] disabled:opacity-50"
          >
            <Plus className="h-3 w-3" />
            <span>New PTY</span>
          </button>
          <button
            type="button"
            onClick={loadSessions}
            className="rounded border border-[var(--color-border-muted)] p-1 text-[var(--color-fg-subtle)] hover:text-[var(--color-fg-default)]"
            title="Refresh sessions"
          >
            <RefreshCw className="h-3 w-3" />
          </button>
        </div>
      </div>

      {error && (
        <div className="flex items-center justify-between border-b border-rose-500/20 bg-rose-500/10 px-4 py-1.5 text-xs text-rose-300">
          <span>{error}</span>
          <button type="button" onClick={() => setError(null)} className="text-rose-400 hover:text-rose-200">
            <X className="h-3 w-3" />
          </button>
        </div>
      )}

      {/* Terminal Session Tabs */}
      <div className="flex items-center gap-1 overflow-x-auto border-b border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2 py-1">
        {sessions.map((sess) => {
          const isActive = sess.id === activeSessionId;
          const isTermActive = sess.status.type === 'active';
          return (
            <div
              key={sess.id}
              onClick={() => setActiveSessionId(sess.id)}
              className={`group flex cursor-pointer items-center gap-2 rounded-t-md px-3 py-1.5 text-xs transition-colors ${
                isActive
                  ? 'bg-[var(--color-canvas)] text-[var(--color-fg-default)] font-medium shadow-sm'
                  : 'text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-3)]'
              }`}
            >
              <div
                className={`h-1.5 w-1.5 rounded-full ${
                  isTermActive ? 'bg-emerald-400 animate-pulse' : 'bg-[var(--color-fg-subtle)]'
                }`}
              />
              <span className="font-mono text-[11px] truncate max-w-[120px]">
                {sess.command.split('/').pop() || sess.id.slice(0, 8)}
              </span>
              <button
                type="button"
                onClick={(e) => terminateSession(sess.id, e)}
                className="opacity-0 group-hover:opacity-100 rounded p-0.5 hover:bg-[var(--color-surface-3)] text-[var(--color-fg-subtle)] hover:text-rose-400"
                title="Terminate terminal"
              >
                <X className="h-2.5 w-2.5" />
              </button>
            </div>
          );
        })}

        {sessions.length === 0 && !loading && (
          <div className="px-3 py-1 text-[11px] text-[var(--color-fg-subtle)]">
            No active terminal sessions. Click "New PTY" to launch one.
          </div>
        )}
      </div>

      {/* Terminal Output Viewport */}
      <div className="relative flex-1 overflow-hidden bg-black/90 p-4 font-mono text-xs leading-relaxed text-zinc-100">
        <div className="h-full overflow-y-auto space-y-1 select-text scrollbar-thin scrollbar-thumb-zinc-700">
          {currentOutput ? (
            <pre className="whitespace-pre-wrap break-words font-mono text-[11px] leading-5 text-zinc-200">
              {currentOutput}
            </pre>
          ) : (
            <div className="flex h-full items-center justify-center text-zinc-500 text-xs">
              {sessions.length === 0 ? (
                <div className="text-center">
                  <Terminal className="mx-auto h-8 w-8 mb-2 opacity-40" />
                  <p>No active terminal session</p>
                  <p className="text-[10px] mt-1 text-zinc-600">
                    Click "New PTY" above to start a session in{' '}
                    <span className="text-zinc-400">{activeWs?.name || 'workspace'}</span>
                  </p>
                </div>
              ) : (
                <div className="flex items-center gap-2">
                  <RefreshCw className="h-3.5 w-3.5 animate-spin" />
                  <span>Connecting to bounded PTY stream...</span>
                </div>
              )}
            </div>
          )}
          <div ref={outputEndRef} />
        </div>
      </div>

      {/* Quick Action Chips & Input Command Bar */}
      {activeSessionId && (
        <div className="border-t border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] p-2.5">
          {/* Quick Action Chips */}
          <div className="mb-2 flex flex-wrap items-center gap-1.5 text-[10px]">
            <span className="text-[var(--color-fg-subtle)] font-medium">Quick:</span>
            <button
              type="button"
              onClick={() => sendInput('\x03')}
              className="rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2 py-0.5 font-mono text-amber-400 hover:bg-[var(--color-surface-3)]"
              title="Send SIGINT (Ctrl+C)"
            >
              Ctrl+C
            </button>
            <button
              type="button"
              onClick={() => {
                setOutputLogs((prev) => ({ ...prev, [activeSessionId]: '' }));
              }}
              className="rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2 py-0.5 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-3)]"
            >
              Clear Buffer
            </button>
            <button
              type="button"
              onClick={() => sendInput('git status\n')}
              className="rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2 py-0.5 text-[var(--color-fg-default)] hover:bg-[var(--color-surface-3)]"
            >
              git status
            </button>
            <button
              type="button"
              onClick={() => sendInput('cargo check\n')}
              className="rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2 py-0.5 text-[var(--color-fg-default)] hover:bg-[var(--color-surface-3)]"
            >
              cargo check
            </button>
            <button
              type="button"
              onClick={() => sendInput('ls -la\n')}
              className="rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2 py-0.5 text-[var(--color-fg-default)] hover:bg-[var(--color-surface-3)]"
            >
              ls -la
            </button>
          </div>

          {/* Stdin Bar */}
          <form onSubmit={handleCommandSubmit} className="flex items-center gap-2">
            <span className="font-mono text-xs font-bold text-emerald-400">❯</span>
            <input
              ref={inputRef}
              type="text"
              value={inputCommand}
              onChange={(e) => setInputCommand(e.target.value)}
              placeholder="Type command or input and press Enter..."
              className="flex-1 rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3 py-1.5 font-mono text-xs text-[var(--color-fg-default)] outline-none focus:border-[var(--color-fg-default)]"
            />
            <button
              type="submit"
              className="rounded-md bg-[var(--color-surface-3)] p-1.5 text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-fg-default)]"
              title="Send (Enter)"
            >
              <CornerDownLeft className="h-3.5 w-3.5" />
            </button>
          </form>
        </div>
      )}
    </div>
  );
};
