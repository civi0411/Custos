import React, { useEffect, useRef, useState, useMemo, useCallback } from 'react';
import {
  Terminal,
  Plus,
  X,
  RefreshCw,
  CornerDownLeft,
  Trash2,
  Copy,
  Check,
  FolderGit2,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { useAppContext } from '@/context/AppContext';
import type { ExecutionWorkspace, TerminalSession } from '@/types/domain';

export interface TerminalWorkbenchPaneProps {
  initialWorkspaceId?: string;
  activeWorkspaceId?: string;
  onSelectWorkspace?: (id: string) => void;
  initialCommand?: string;
}

interface AnsiSegment {
  text: string;
  className?: string;
}

// Lightweight ANSI SGR color code parser for clean terminal rendering
function parseAnsiOutput(raw: string): AnsiSegment[] {
  if (!raw || !raw.includes('\u001b[')) {
    return [{ text: raw }];
  }

  // Strip non-SGR escape sequences (cursor movements, screen clear, etc.)
  const cleaned = raw.replace(/\u001b\[[0-9;]*[A-HJ-Za-ln-z]/g, '');

  const parts = cleaned.split(/(\u001b\[[0-9;]*m)/g);
  const segments: AnsiSegment[] = [];
  let currentColor = '';
  let isBold = false;

  for (const part of parts) {
    if (!part) continue;
    const match = part.match(/^\u001b\[([0-9;]*)m$/);
    if (match) {
      const codes = match[1] ? match[1].split(';').map(Number) : [0];
      for (const code of codes) {
        if (code === 0) {
          currentColor = '';
          isBold = false;
        } else if (code === 1) {
          isBold = true;
        } else if (code === 31 || code === 91) {
          currentColor = 'text-rose-400';
        } else if (code === 32 || code === 92) {
          currentColor = 'text-emerald-400';
        } else if (code === 33 || code === 93) {
          currentColor = 'text-amber-400';
        } else if (code === 34 || code === 94) {
          currentColor = 'text-sky-400';
        } else if (code === 35 || code === 95) {
          currentColor = 'text-purple-400';
        } else if (code === 36 || code === 96) {
          currentColor = 'text-cyan-400';
        } else if (code === 37 || code === 97) {
          currentColor = 'text-white';
        } else if (code === 90) {
          currentColor = 'text-zinc-500';
        }
      }
    } else {
      const classes = [currentColor, isBold ? 'font-semibold' : ''].filter(Boolean).join(' ');
      segments.push({
        text: part,
        className: classes || undefined,
      });
    }
  }

  return segments;
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
  const [outputLogs, setOutputLogs] = useState<Record<string, string>>({});
  const [nextSeqs, setNextSeqs] = useState<Record<string, number>>({});
  const [inputCommand, setInputCommand] = useState('');
  const [spawning, setSpawning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  // Command history memory (Up/Down arrow navigation)
  const [history, setHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState<number>(-1);
  const historyDraftRef = useRef<string>('');

  const outputContainerRef = useRef<HTMLDivElement>(null);
  const outputEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const activeWorkspaceRef = useRef(activeWorkspaceId);
  activeWorkspaceRef.current = activeWorkspaceId;
  const appliedInitialWorkspace = useRef<string | undefined>(undefined);
  const sessionLoadEpoch = useRef(0);
  const autoSpawnDoneRef = useRef<string | null>(null);

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
      setOutputLogs((prev) => ({ ...prev, [session.id]: '' }));
      setNextSeqs((prev) => ({ ...prev, [session.id]: 0 }));
      setTimeout(() => inputRef.current?.focus(), 100);
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
    } catch (err: any) {
      setError(`Failed to terminate session: ${err?.message || err}`);
    }
  }, []);

  // 5. Send input bytes to active terminal
  const sendInput = useCallback(async (data: string) => {
    if (!activeSessionId) return;
    try {
      await daemonClient.writeTerminal(activeSessionId, data);
    } catch (err: any) {
      setError(`Failed to write to terminal: ${err?.message || err}`);
    }
  }, [activeSessionId]);

  const handleCommandSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!inputCommand.trim() && inputCommand !== '') return;
    const cmd = inputCommand;
    sendInput(cmd + '\n');
    if (cmd.trim()) {
      setHistory((prev) => [cmd, ...prev.filter((h) => h !== cmd)].slice(0, 50));
    }
    setHistoryIndex(-1);
    setInputCommand('');
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (history.length === 0) return;
      if (historyIndex === -1) {
        historyDraftRef.current = inputCommand;
      }
      const nextIndex = Math.min(historyIndex + 1, history.length - 1);
      setHistoryIndex(nextIndex);
      setInputCommand(history[nextIndex] || '');
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (historyIndex > 0) {
        const nextIndex = historyIndex - 1;
        setHistoryIndex(nextIndex);
        setInputCommand(history[nextIndex] || '');
      } else if (historyIndex === 0) {
        setHistoryIndex(-1);
        setInputCommand(historyDraftRef.current);
      }
    } else if (e.ctrlKey && e.key === 'c') {
      e.preventDefault();
      sendInput('\x03');
    } else if (e.ctrlKey && e.key === 'l') {
      e.preventDefault();
      if (activeSessionId) {
        setOutputLogs((prev) => ({ ...prev, [activeSessionId]: '' }));
      }
    }
  };

  const nextSeqsRef = useRef(nextSeqs);
  useEffect(() => {
    nextSeqsRef.current = nextSeqs;
  }, [nextSeqs]);

  // 6. Polling loop for active session output stream
  useEffect(() => {
    if (!activeSessionId) return;
    let isSubscribed = true;

    const poll = async () => {
      const currentSeq = nextSeqsRef.current[activeSessionId] || 0;
      try {
        const chunk = await daemonClient.readTerminal(activeSessionId, currentSeq, 65536);
        if (!isSubscribed) return;

        if (chunk.data && chunk.data.length > 0) {
          setOutputLogs((prev) => ({
            ...prev,
            [activeSessionId]: (prev[activeSessionId] || '') + chunk.data,
          }));
          const newSeq = chunk.nextSeq ?? chunk.next_seq ?? currentSeq;
          nextSeqsRef.current = { ...nextSeqsRef.current, [activeSessionId]: newSeq };
          setNextSeqs((prev) => ({
            ...prev,
            [activeSessionId]: newSeq,
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

    void poll();
    const interval = setInterval(poll, 300);
    return () => {
      isSubscribed = false;
      clearInterval(interval);
    };
  }, [activeSessionId]);

  // 7. Auto-scroll output on new logs
  useEffect(() => {
    outputEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [outputLogs, activeSessionId]);

  // 8. Copy entire terminal output buffer
  const handleCopyOutput = useCallback(() => {
    const raw = (activeSessionId && outputLogs[activeSessionId]) || '';
    if (!raw) return;
    navigator.clipboard.writeText(raw);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, [activeSessionId, outputLogs]);

  // 9. Focus input on click in output area
  const handleViewportClick = () => {
    const selection = window.getSelection();
    if (selection && selection.toString().length > 0) return;
    inputRef.current?.focus();
  };

  const activeWs = workspaces.find((w) => w.id === activeWorkspaceId);
  const activeSess = sessions.find((s) => s.id === activeSessionId);
  const currentOutput = (activeSessionId && outputLogs[activeSessionId]) || '';
  const parsedSegments = useMemo(() => parseAnsiOutput(currentOutput), [currentOutput]);

  return (
    <div className="flex h-full w-full flex-col bg-[#0b0c0e] text-zinc-200 select-none overflow-hidden font-sans">
      {/* ── TOP UNIFIED HEADER: REPOSITORY BADGE, TABS, ACTIONS ── */}
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
              className="size-6 flex items-center justify-center rounded text-white/50 hover:text-white hover:bg-white/[0.06] transition disabled:opacity-30 shrink-0"
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

          {/* Quick Interrupt */}
          {activeSessionId && (
            <button
              type="button"
              onClick={() => sendInput('\x03')}
              className="px-2 py-0.5 rounded text-[10px] font-mono bg-white/[0.04] hover:bg-white/[0.08] text-white/70 hover:text-white border border-white/8 transition"
              title="Send SIGINT (Ctrl+C)"
            >
              Ctrl+C
            </button>
          )}

          {/* Clear Buffer */}
          {activeSessionId && (
            <button
              type="button"
              onClick={() => setOutputLogs((prev) => ({ ...prev, [activeSessionId]: '' }))}
              className="size-7 flex items-center justify-center rounded text-white/50 hover:text-white hover:bg-white/[0.06] transition"
              title="Clear Buffer (Ctrl+L)"
            >
              <Trash2 className="size-3.5" />
            </button>
          )}

          {/* Copy Output */}
          {activeSessionId && (
            <button
              type="button"
              onClick={handleCopyOutput}
              className="size-7 flex items-center justify-center rounded text-white/50 hover:text-white hover:bg-white/[0.06] transition"
              title={copied ? 'Copied!' : 'Copy Terminal Output'}
            >
              {copied ? <Check className="size-3.5 text-emerald-400" /> : <Copy className="size-3.5" />}
            </button>
          )}

          {/* Refresh Sessions */}
          <button
            type="button"
            onClick={() => void loadSessions()}
            className="size-7 flex items-center justify-center rounded text-white/50 hover:text-white hover:bg-white/[0.06] transition"
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

      {/* ── MAIN TERMINAL VIEWPORT ── */}
      <div
        ref={outputContainerRef}
        onClick={handleViewportClick}
        className="relative flex-1 overflow-y-auto bg-[#0a0a0c] p-3.5 font-mono text-[12px] leading-[1.5] text-zinc-100 select-text cursor-text scrollbar-thin scrollbar-thumb-white/10"
      >
        {currentOutput ? (
          <pre className="whitespace-pre-wrap break-words font-mono text-[11.5px] leading-5 text-zinc-200 select-text">
            {parsedSegments.map((seg, i) => (
              <span key={i} className={seg.className}>
                {seg.text}
              </span>
            ))}
          </pre>
        ) : (
          <div className="flex h-full flex-col items-center justify-center text-center text-zinc-500 py-12 select-none">
            {sessions.length === 0 ? (
              <div className="space-y-2 max-w-sm">
                <Terminal className="mx-auto size-9 opacity-30 text-white/50" />
                <p className="text-sm font-medium text-white/80">No active terminal session</p>
                <p className="text-xs text-white/40">
                  Spawn a live PTY session for{' '}
                  <span className="text-workbench-accent font-semibold">{activeWs?.name || 'workspace'}</span>
                </p>
                <button
                  type="button"
                  onClick={() => void spawnSession()}
                  disabled={spawning || !activeWorkspaceId}
                  className="mt-2 inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-white/10 hover:bg-white/15 text-white text-xs font-medium transition cursor-pointer"
                >
                  <Plus className="size-3.5" />
                  <span>Launch Terminal</span>
                </button>
              </div>
            ) : (
              <div className="flex items-center gap-2 text-xs text-white/50">
                <RefreshCw className="size-3.5 animate-spin" />
                <span>Connecting to live PTY stream…</span>
              </div>
            )}
          </div>
        )}
        <div ref={outputEndRef} />
      </div>

      {/* ── STDIN INPUT COMMAND BAR ── */}
      {activeSessionId && (
        <div className="border-t border-white/8 bg-[#111215] px-3 py-2 shrink-0">
          <form onSubmit={handleCommandSubmit} className="flex items-center gap-2">
            <span className="font-mono text-xs font-bold text-workbench-accent select-none shrink-0">
              ❯
            </span>
            <input
              ref={inputRef}
              type="text"
              value={inputCommand}
              onChange={(e) => setInputCommand(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Type command or input (↑/↓ history, Ctrl+C cancel, Enter to run)…"
              className="flex-1 bg-transparent px-1 py-0.5 font-mono text-xs text-white placeholder-white/25 outline-none focus:placeholder-white/10"
              autoFocus
            />
            <button
              type="submit"
              disabled={!inputCommand.trim() && inputCommand === ''}
              className="size-6 flex items-center justify-center rounded text-white/40 hover:text-white hover:bg-white/[0.08] transition cursor-pointer shrink-0 disabled:opacity-20"
              title="Execute (Enter)"
            >
              <CornerDownLeft className="size-3.5" />
            </button>
          </form>
        </div>
      )}
    </div>
  );
};
