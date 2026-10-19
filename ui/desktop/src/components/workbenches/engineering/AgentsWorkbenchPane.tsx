import { NativeSelect } from '@/components/common/NativeSelect';
import React, { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import {
  Bot,
  Play,
  RefreshCw,
  Terminal,
  ShieldAlert,
  ShieldCheck,
  CheckCircle2,
  AlertTriangle,
  Zap,
  Eye,
  Copy,
  Check,
  Search,
  X,
  Power,
  Sparkles,
  AlertCircle,
  RotateCcw,
  Trash2,
  FolderGit2,
  SlidersHorizontal,
} from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { daemonClient } from '@/api/daemon_client';
import {
  HarnessDescriptor,
  ExecutionWorkspace,
  RunHandle,
  ToolMediationLevel,
  TerminalSession,
} from '@/types/domain';
import { JobActionButton } from '@/components/common';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';
import { getAgentBrandIcon } from '@/components/common/AgentIcons';
import { XTerminalView } from '@/components/common/XTerminalView';

export interface AgentsWorkbenchPaneProps {
  sessionId?: string;
  tabId?: string;
  initialHarnessId?: string;
  initialWorkspaceId?: string;
  activeWorkspaceId?: string;
  onSelectWorkspace?: (id: string) => void;
  onShowToast?: (msg: string) => void;
  onOpenTab?: (id: OrcaTabId) => void;
}

const INSTALL_COMMANDS: Record<string, string> = {
  'claude-code': 'npm install -g @anthropic-ai/claude-code',
  codex: 'npm install -g @openai/codex',
  gemini: 'npm install -g @google/gemini-cli',
  goose: 'curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash',
  opencode: 'npm install -g opencode-ai',
  aider: 'pip install aider-chat',
  'copilot-cli': 'npm install -g @githubnext/github-copilot-cli',
  'qodo-cover': 'npm install -g qodo-cover',
  'kiro-cli': 'curl -fsSL https://kiro.dev/install.sh | bash',
  amp: 'npm install -g amp-cli',
  plandex: 'curl -sL https://plandex.ai/install.sh | bash',
  'devin-cli': 'npm install -g devin-cli',
  'agent-zero': 'pip install agent-zero-cli',
  supermaven: 'npm install -g supermaven-cli',
  'continue-cli': 'npm install -g @continuedev/cli',
  cursor: 'https://cursor.com',
  trae: 'https://trae.ai',
  windsurf: 'https://codeium.com/windsurf',
  ollama: 'brew install ollama',
  openhands: 'pip install openhands-ai',
  morph: 'pip install morph-cli',
  qwen: 'pip install qwen-agent',
  deepseek: 'ollama run deepseek-coder-v2',
};

export const AgentsWorkbenchPane: React.FC<AgentsWorkbenchPaneProps> = ({
  sessionId,
  tabId: _tabId,
  initialHarnessId,
  initialWorkspaceId,
  activeWorkspaceId: propActiveWorkspaceId,
  onSelectWorkspace: propOnSelectWorkspace,
  onShowToast,
  onOpenTab,
}) => {
  const {
    startNativeConversation,
    activeWorkspaceId: ctxActiveWorkspaceId,
    setActiveWorkspaceId: ctxSetActiveWorkspaceId,
  } = useAppContext();
  const activeWorkspaceId = propActiveWorkspaceId ?? ctxActiveWorkspaceId;
  const onSelectWorkspace = propOnSelectWorkspace ?? ctxSetActiveWorkspaceId;

  // View mode: 'interactive' (Live PTY Terminal like Orca) vs 'governed' (Custos Mediated Prompt/Jobs)
  const [viewMode, setViewMode] = useState<'interactive' | 'governed'>('interactive');
  const [showFleetSidebar, setShowFleetSidebar] = useState<boolean>(!initialHarnessId);

  // Harness & Workspace fleet state
  const [harnesses, setHarnesses] = useState<HarnessDescriptor[]>([]);
  const [loadingHarnesses, setLoadingHarnesses] = useState<boolean>(true);
  const [selectedHarnessId, setSelectedHarnessId] = useState<string>(initialHarnessId || 'claude-code');
  const [searchQuery, setSearchQuery] = useState('');
  const [filterFilter, setFilterFilter] = useState<'all' | 'installed' | 'missing'>('all');

  const [workspaces, setWorkspaces] = useState<ExecutionWorkspace[]>([]);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string>('');
  const appliedInitialWorkspace = useRef<string | undefined>(undefined);

  // Governed Mode State
  const subscription = useRef<AbortController | null>(null);
  const [activities, setActivities] = useState<import('@/types/domain').LiveRunSnapshot['activities']>([]);
  useEffect(() => () => subscription.current?.abort(), []);
  const [instruction, setInstruction] = useState<string>('');
  const [running, setRunning] = useState<boolean>(false);
  const [runResult, setRunResult] = useState<RunHandle | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [copiedStdout, setCopiedStdout] = useState<boolean>(false);
  const [copiedInstall, setCopiedInstall] = useState<boolean>(false);

  // Interactive PTY Terminal Mode State
  const [ptySessions, setPtySessions] = useState<TerminalSession[]>([]);
  const [activePtyId, setActivePtyId] = useState<string>('');
  const [ptySpawning, setPtySpawning] = useState<boolean>(false);
  const [ptyError, setPtyError] = useState<string | null>(null);

  // 1. Fetch Harnesses
  const fetchHarnesses = useCallback(async () => {
    try {
      setLoadingHarnesses(true);
      setErrorMessage(null);
      const list = await daemonClient.listHarnesses();
      setHarnesses(list);
      if (initialHarnessId && list.some((h) => h.id === initialHarnessId)) {
        setSelectedHarnessId(initialHarnessId);
      } else if (list.length > 0 && !list.some((h) => h.id === selectedHarnessId)) {
        setSelectedHarnessId(list[0].id);
      }
    } catch (err: any) {
      setErrorMessage(err?.message || 'Failed to list agent harnesses from daemon');
    } finally {
      setLoadingHarnesses(false);
    }
  }, [initialHarnessId, selectedHarnessId]);

  // 2. Fetch Workspaces
  const fetchWorkspaces = useCallback(async () => {
    try {
      const list = await daemonClient.listWorkspaces();
      const active = list.filter((w) => w.status !== 'archived');
      setWorkspaces(active);
      if (active.length > 0 && !selectedWorkspaceId) {
        setSelectedWorkspaceId(active[0].id);
      }
    } catch {
      // workspace optional
    }
  }, [selectedWorkspaceId]);

  useEffect(() => {
    fetchHarnesses();
    fetchWorkspaces();
  }, [fetchHarnesses, fetchWorkspaces]);

  useEffect(() => {
    if (activeWorkspaceId && activeWorkspaceId !== selectedWorkspaceId) {
      setSelectedWorkspaceId(activeWorkspaceId);
    }
  }, [activeWorkspaceId, selectedWorkspaceId]);

  useEffect(() => {
    if (
      initialWorkspaceId &&
      appliedInitialWorkspace.current !== initialWorkspaceId &&
      workspaces.some((workspace) => workspace.id === initialWorkspaceId)
    ) {
      appliedInitialWorkspace.current = initialWorkspaceId;
      setSelectedWorkspaceId(initialWorkspaceId);
    }
  }, [initialWorkspaceId, workspaces]);

  const selectedHarness = useMemo(
    () => harnesses.find((h) => h.id === selectedHarnessId),
    [harnesses, selectedHarnessId]
  );

  const selectedWorkspace = useMemo(
    () => workspaces.find((w) => w.id === selectedWorkspaceId),
    [workspaces, selectedWorkspaceId]
  );

  // 3. Load active PTY terminal sessions for workspace
  const loadPtySessions = useCallback(async () => {
    if (!selectedWorkspaceId) return;
    try {
      const list = await daemonClient.listTerminals(selectedWorkspaceId);
      setPtySessions(list);
      if (list.length > 0 && (!activePtyId || !list.some((s) => s.id === activePtyId))) {
        setActivePtyId(list[0].id);
      }
    } catch (err) {
      console.warn('Failed to list terminals:', err);
    }
  }, [selectedWorkspaceId, activePtyId]);

  useEffect(() => {
    loadPtySessions();
  }, [selectedWorkspaceId, loadPtySessions]);

  // 4. Spawn dedicated Agent PTY session
  const spawnAgentPty = async (customCmd?: string) => {
    if (!selectedWorkspaceId) {
      setPtyError('Please select a target workspace before launching agent');
      return;
    }
    const launchCommand =
      customCmd || selectedHarness?.launch_cmd || (selectedHarness as any)?.launchCmd || selectedHarness?.binary_path;

    if (!launchCommand) {
      setPtyError('No launch command configured for this harness');
      return;
    }

    setPtySpawning(true);
    setPtyError(null);
    try {
      const session = await daemonClient.spawnTerminal({
        workspace_id: selectedWorkspaceId,
        command: launchCommand,
        cols: 100,
        rows: 32,
      });

      setPtySessions((prev) => [session, ...prev]);
      setActivePtyId(session.id);
      onShowToast?.(`Spawned interactive session for ${selectedHarness?.name || selectedHarnessId}`);
    } catch (err: any) {
      setPtyError(`Failed to spawn interactive terminal: ${err?.message || err}`);
    } finally {
      setPtySpawning(false);
    }
  };

  // Auto-spawn interactive agent PTY session upon tab open (Orca parity)
  const autoSpawnDoneRef = useRef<string | null>(null);
  useEffect(() => {
    if (!selectedWorkspaceId || !selectedHarness || loadingHarnesses) return;
    const key = `${selectedWorkspaceId}:${selectedHarness.id}`;
    if (autoSpawnDoneRef.current === key) return;
    autoSpawnDoneRef.current = key;

    const cmd = selectedHarness.launch_cmd || (selectedHarness as any)?.launchCmd || selectedHarness.binary_path;
    const existing = ptySessions.find(
      (s) => s.workspace_id === selectedWorkspaceId && s.status.type === 'active' && (s.command === cmd || s.command?.includes(selectedHarness.id))
    );
    if (existing) {
      setActivePtyId(existing.id);
    } else {
      void spawnAgentPty(cmd);
    }
  }, [selectedWorkspaceId, selectedHarness, loadingHarnesses, ptySessions]);


  const handlePtyTerminate = async () => {
    if (!activePtyId) return;
    try {
      await daemonClient.terminateTerminal(activePtyId);
      setPtySessions((prev) =>
        prev.map((s) => (s.id === activePtyId ? { ...s, status: { type: 'terminated' } } : s))
      );
      onShowToast?.('Agent session terminated');
    } catch (err: any) {
      setPtyError(`Failed to terminate: ${err?.message || err}`);
    }
  };

  // Governed Mode: Execution Handler
  const handleRun = async () => {
    if (!instruction.trim()) {
      setErrorMessage('Please provide an instruction for the native harness run');
      return;
    }
    if (!selectedWorkspaceId) {
      setErrorMessage('Select a ready execution workspace before starting a native agent');
      return;
    }
    if (!selectedHarness?.is_available) {
      setErrorMessage('Install or select an available coding harness before starting a run');
      return;
    }

    setRunning(true);
    setErrorMessage(null);
    setStatusMessage(`Executing native instruction with '${selectedHarnessId}'...`);
    setRunResult(null);

    try {
      setActivities([]);
      const { run: result, sessionId } = await startNativeConversation(
        instruction.trim(),
        selectedHarnessId,
        selectedWorkspaceId
      );
      setRunResult(result);
      subscription.current?.abort();
      const controller = new AbortController();
      subscription.current = controller;
      let latest: import('@/types/domain').LiveRunSnapshot | null = null;
      let timer: number | undefined;

      const flush = () => {
        timer = undefined;
        if (!latest) return;
        setRunResult({
          ...result,
          output: latest.text,
          status: (['completed', 'failed', 'cancelled'].includes(latest.status)
            ? latest.status
            : 'active') as RunHandle['status'],
        });
        setStatusMessage(latest.reconnecting ? 'Reconnecting' : latest.status);
        setActivities(latest.activities);
        setErrorMessage(latest.error || null);
      };

      try {
        await daemonClient.watchRun(
          sessionId,
          result.run_id,
          (snapshot) => {
            latest = snapshot;
            if (['completed', 'failed', 'cancelled', 'interrupted'].includes(snapshot.status)) {
              if (timer !== undefined) window.clearTimeout(timer);
              flush();
            } else if (timer === undefined) timer = window.setTimeout(flush, 50);
          },
          controller.signal
        );
      } finally {
        if (timer !== undefined) window.clearTimeout(timer);
        if (!controller.signal.aborted) flush();
      }
    } catch (err: any) {
      setErrorMessage(err?.message || 'Execution error encountered');
      setStatusMessage(null);
    } finally {
      setRunning(false);
    }
  };

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedStdout(true);
    setTimeout(() => setCopiedStdout(false), 2000);
  };

  const handleCopyInstall = (cmd: string) => {
    navigator.clipboard.writeText(cmd);
    setCopiedInstall(true);
    setTimeout(() => setCopiedInstall(false), 2000);
    onShowToast?.('Install command copied to clipboard');
  };

  const renderMediationBadge = (tier: ToolMediationLevel | string) => {
    switch (tier) {
      case 'custos_mediated':
        return (
          <span className="inline-flex items-center gap-1.5 rounded-full border border-white/15 bg-white/[0.08] px-2 py-0.5 text-[10px] font-medium text-white/90">
            <ShieldCheck className="h-3 w-3 text-white/70" />
            Custos Mediated (Permit Enforced)
          </span>
        );
      case 'provider_governed':
        return (
          <span className="inline-flex items-center gap-1.5 rounded-full border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[10px] font-medium text-white/70">
            <ShieldAlert className="h-3 w-3 text-white/50" />
            Provider Governed (Gate 4 Bypass)
          </span>
        );
      case 'observe_only':
      default:
        return (
          <span className="inline-flex items-center gap-1.5 rounded-full border border-white/10 bg-white/[0.02] px-2 py-0.5 text-[10px] font-medium text-white/60">
            <Eye className="h-3 w-3 text-white/50" />
            Observe Only
          </span>
        );
    }
  };

  const renderMaturityBadge = (maturity?: 'verified_native' | 'discovered_cli') => {
    if (maturity === 'verified_native') {
      return (
        <span
          className="inline-flex items-center gap-1 rounded border border-white/15 bg-white/10 px-1.5 py-0.5 text-[10px] font-semibold text-white/90"
          title="Verified Native: Protocol conformance with structured tool events and live observer"
        >
          <Sparkles className="h-2.5 w-2.5 text-white/70" />
          Verified Native
        </span>
      );
    }
    return (
      <span
        className="inline-flex items-center gap-1 rounded border border-white/10 bg-white/[0.03] px-1.5 py-0.5 text-[10px] font-medium text-white/60"
        title="Discovered CLI: Standard host CLI executable; unmediated subprocess execution"
      >
        Discovered CLI
      </span>
    );
  };

  // Filtered Harnesses
  const filteredHarnesses = useMemo(() => {
    let list = harnesses;
    if (filterFilter === 'installed') {
      list = list.filter((h) => h.is_available ?? (h as any).isAvailable);
    } else if (filterFilter === 'missing') {
      list = list.filter((h) => !(h.is_available ?? (h as any).isAvailable));
    }
    if (!searchQuery.trim()) return list;
    const q = searchQuery.toLowerCase();
    return list.filter(
      (h) =>
        h.name.toLowerCase().includes(q) ||
        h.id.toLowerCase().includes(q) ||
        h.description.toLowerCase().includes(q) ||
        h.binary_path.toLowerCase().includes(q)
    );
  }, [harnesses, filterFilter, searchQuery]);

  const activePtySession = ptySessions.find((s) => s.id === activePtyId);
  const isAvailable = selectedHarness?.is_available ?? (selectedHarness as any)?.isAvailable;
  const installCmd = selectedHarness ? INSTALL_COMMANDS[selectedHarness.id] : null;

  return (
    <div className="flex h-full flex-col overflow-hidden bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
      {/* Unified 38px Toolbar: Identity, Worktree, Status, Actions */}
      <div className="flex h-10 shrink-0 items-center justify-between border-b border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-3 text-xs select-none">
        {/* Left: Agent Identity + Worktree + Session status */}
        <div className="flex items-center gap-2.5 min-w-0">
          <div className="flex items-center gap-1.5 font-medium text-[var(--color-fg-default)] shrink-0">
            {selectedHarness ? (
              getAgentBrandIcon(selectedHarness.id, 16, 'shrink-0')
            ) : (
              <Bot className="h-4 w-4 text-[var(--color-fg-muted)]" />
            )}
            <span className="text-xs font-semibold">
              {selectedHarness ? selectedHarness.name : 'Coding Agent'}
            </span>
          </div>

          <div className="h-3.5 w-px bg-[var(--color-border-subtle)] shrink-0" />

          {/* Worktree Target selector */}
          <div className="flex items-center gap-1.5 shrink-0">
            <FolderGit2 className="h-3.5 w-3.5 text-[var(--color-fg-subtle)]" />
            <NativeSelect
              value={selectedWorkspaceId}
              onChange={(e) => {
                const newId = e.target.value;
                setSelectedWorkspaceId(newId);
                onSelectWorkspace?.(newId);
              }}
              className="rounded border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2 py-0.5 text-[11px] font-medium text-[var(--color-fg-default)] outline-none hover:bg-[var(--color-surface-3)]"
            >
              <option value="">Current Working Directory</option>
              {workspaces.map((ws) => (
                <option key={ws.id} value={ws.id}>
                  {ws.name} ({ws.kind.type === 'git' ? 'git worktree' : 'directory'})
                </option>
              ))}
            </NativeSelect>
          </div>

          <div className="h-3.5 w-px bg-[var(--color-border-subtle)] shrink-0" />

          {/* Session indicator */}
          <div className="flex items-center gap-1.5 shrink-0">
            <span
              className={`h-2 w-2 rounded-full ${
                activePtySession?.status.type === 'active'
                  ? 'bg-white/80 animate-pulse'
                  : 'bg-zinc-500'
              }`}
            />
            <span className="text-[11px] text-[var(--color-fg-muted)]">
              {activePtySession ? (activePtySession.status.type === 'active' ? 'Active' : 'Stopped') : 'Idle'}
            </span>
          </div>

          {ptySessions.length > 1 && (
            <NativeSelect
              value={activePtyId}
              onChange={(e) => setActivePtyId(e.target.value)}
              className="rounded border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-1.5 py-0.5 text-[11px] text-[var(--color-fg-default)]"
            >
              {ptySessions.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.command} ({s.status.type})
                </option>
              ))}
            </NativeSelect>
          )}
        </div>

        {/* Right: Actions */}
        <div className="flex items-center gap-1.5 shrink-0">
          {/* Mode Switcher */}
          <div className="flex items-center rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] p-0.5 text-[11px]">
            <button
              type="button"
              onClick={() => setViewMode('interactive')}
              className={`flex items-center gap-1 rounded px-2 py-0.5 font-medium transition cursor-pointer ${
                viewMode === 'interactive'
                  ? 'bg-[var(--color-surface-1)] text-[var(--color-fg-default)] shadow-xs'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-fg-default)]'
              }`}
              title="Interactive PTY (Live Claude Code Terminal)"
            >
              <Terminal className="h-3 w-3" />
              Terminal
            </button>
            <button
              type="button"
              onClick={() => setViewMode('governed')}
              className={`flex items-center gap-1 rounded px-2 py-0.5 font-medium transition cursor-pointer ${
                viewMode === 'governed'
                  ? 'bg-[var(--color-surface-1)] text-[var(--color-fg-default)] shadow-xs'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-fg-default)]'
              }`}
              title="Autonomous Governed Jobs"
            >
              <ShieldCheck className="h-3 w-3" />
              Governed
            </button>
          </div>

          <div className="h-3.5 w-px bg-[var(--color-border-subtle)]" />

          {/* Restart / Spawn */}
          <button
            type="button"
            onClick={() => spawnAgentPty()}
            disabled={ptySpawning}
            className="flex items-center gap-1 rounded border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2 py-1 text-[11px] font-medium text-[var(--color-fg-default)] transition hover:bg-[var(--color-surface-3)] disabled:opacity-50 cursor-pointer"
            title="Restart or spawn agent CLI process"
          >
            {ptySpawning ? (
              <RefreshCw className="h-3 w-3 animate-spin" />
            ) : (
              <RotateCcw className="h-3 w-3" />
            )}
            <span>Restart</span>
          </button>

          {/* Kill / Terminate */}
          {activePtySession && activePtySession.status.type === 'active' && (
            <button
              type="button"
              onClick={handlePtyTerminate}
              className="flex items-center gap-1 rounded border border-rose-500/30 bg-rose-500/10 px-2 py-1 text-[11px] font-medium text-rose-400 hover:bg-rose-500/20 cursor-pointer"
              title="Terminate running process"
            >
              <Power className="h-3 w-3" />
              <span>Kill</span>
            </button>
          )}

          {/* Clear Output */}
          <button
            type="button"
            onClick={() => {
              if (activePtyId) {
                daemonClient.writeTerminal(activePtyId, '\x0c').catch(() => {});
              }
            }}
            className="flex items-center gap-1 rounded border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] px-2 py-1 text-[11px] text-[var(--color-fg-muted)] hover:text-[var(--color-fg-default)] cursor-pointer"
            title="Clear terminal buffer"
          >
            <Trash2 className="h-3 w-3" />
            <span>Clear</span>
          </button>

          <div className="h-3.5 w-px bg-[var(--color-border-subtle)]" />

          {/* Fleet directory toggle */}
          <button
            type="button"
            onClick={() => setShowFleetSidebar((prev) => !prev)}
            className={`flex items-center gap-1 rounded border px-2 py-1 text-[11px] font-medium transition cursor-pointer ${
              showFleetSidebar
                ? 'border-[var(--color-accent-emphasis)] bg-[var(--color-accent-subtle)] text-[var(--color-accent-fg)]'
                : 'border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:text-[var(--color-fg-default)]'
            }`}
            title={showFleetSidebar ? 'Hide harness directory' : 'Show harness directory to select another CLI agent'}
          >
            <SlidersHorizontal className="h-3 w-3" />
            <span>Harnesses</span>
          </button>
        </div>
      </div>

      {/* Main Container */}
      <div className="wb-split flex min-h-0 flex-1 overflow-hidden">
        {/* Left: Harness Fleet Sidebar (collapsible for focused Orca-style terminal) */}
        {showFleetSidebar && (
          <div className="wb-master w-80 flex-shrink-0 flex flex-col border-r border-[var(--color-border-default)] bg-[var(--color-surface-1)]">
          {/* Search & Filter Header */}
          <div className="p-3 border-b border-[var(--color-border-subtle)] space-y-2">
            <div className="relative">
              <Search className="pointer-events-none absolute left-2.5 top-2.5 h-3.5 w-3.5 text-[var(--color-fg-muted)]" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Filter coding agents..."
                className="w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-2)] pl-8 pr-3 py-1.5 text-xs text-[var(--color-fg-default)] placeholder:text-[var(--color-fg-subtle)] focus:border-[var(--color-border-focus)] focus:outline-none"
              />
              {searchQuery && (
                <button
                  type="button"
                  onClick={() => setSearchQuery('')}
                  className="absolute right-2 top-2 text-[var(--color-fg-muted)] hover:text-[var(--color-fg-default)]"
                >
                  <X className="h-3.5 w-3.5" />
                </button>
              )}
            </div>

            {/* Filter Chips & Rescan */}
            <div className="flex items-center justify-between text-[11px]">
              <div className="flex items-center gap-1">
                <button
                  type="button"
                  onClick={() => setFilterFilter('all')}
                  className={`px-2 py-0.5 rounded ${
                    filterFilter === 'all'
                      ? 'bg-[var(--color-surface-3)] text-[var(--color-fg-default)] font-semibold'
                      : 'text-[var(--color-fg-muted)] hover:text-[var(--color-fg-default)]'
                  }`}
                >
                  All ({harnesses.length})
                </button>
                <button
                  type="button"
                  onClick={() => setFilterFilter('installed')}
                  className={`px-2 py-0.5 rounded ${
                    filterFilter === 'installed'
                      ? 'bg-white/10 text-white font-semibold'
                      : 'text-[var(--color-fg-muted)] hover:text-[var(--color-fg-default)]'
                  }`}
                >
                  Installed ({harnesses.filter((h) => h.is_available ?? (h as any).isAvailable).length})
                </button>
                <button
                  type="button"
                  onClick={() => setFilterFilter('missing')}
                  className={`px-2 py-0.5 rounded ${
                    filterFilter === 'missing'
                      ? 'bg-white/10 text-white font-semibold'
                      : 'text-[var(--color-fg-muted)] hover:text-[var(--color-fg-default)]'
                  }`}
                >
                  Missing
                </button>
              </div>

              <button
                type="button"
                onClick={fetchHarnesses}
                disabled={loadingHarnesses}
                className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-fg-default)] hover:bg-[var(--color-surface-2)] transition disabled:opacity-50 cursor-pointer"
                title="Rescan system PATH for agent CLIs"
              >
                <RefreshCw className={`h-3 w-3 ${loadingHarnesses ? 'animate-spin' : ''}`} />
              </button>
            </div>
          </div>

          {/* List of Harnesses */}
          <div className="flex-1 overflow-y-auto p-2.5 space-y-1.5">
            {filteredHarnesses.map((harness) => {
              const isSelected = harness.id === selectedHarnessId;
              const available = harness.is_available ?? (harness as any).isAvailable;

              return (
                <div
                  key={harness.id}
                  onClick={() => setSelectedHarnessId(harness.id)}
                  className={`group relative cursor-pointer rounded-lg border p-2.5 transition ${
                    isSelected
                      ? 'border-[var(--color-border-focus)] bg-[var(--color-surface-2)] shadow-xs'
                      : 'border-transparent hover:border-[var(--color-border-subtle)] hover:bg-[var(--color-surface-2)]/60'
                  }`}
                >
                  <div className="flex items-center justify-between gap-2">
                    <div className="flex items-center gap-2.5 min-w-0">
                      <div className="flex h-6 w-6 shrink-0 items-center justify-center rounded bg-[var(--color-surface-3)]">
                        {getAgentBrandIcon(harness.id, 14, 'shrink-0')}
                      </div>
                      <div className="min-w-0">
                        <div className="truncate text-xs font-semibold text-[var(--color-fg-default)]">
                          {harness.name}
                        </div>
                        <div className="font-mono text-[10px] text-[var(--color-fg-muted)] truncate">
                          {harness.binary_path}
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center gap-1.5 shrink-0">
                      {renderMaturityBadge(harness.maturity)}
                      <div
                        className={`h-2 w-2 rounded-full ${
                          available ? 'bg-white/80 ring-2 ring-white/15' : 'bg-white/20'
                        }`}
                        title={available ? 'Installed on PATH' : 'Not found on PATH'}
                      />
                    </div>
                  </div>

                  <p className="mt-1 text-[11px] text-[var(--color-fg-muted)] line-clamp-1">
                    {harness.description}
                  </p>
                </div>
              );
            })}
          </div>
        </div>
        )}

        {/* Right: Active Console & Engine */}
        <div className="flex min-w-0 flex-1 flex-col overflow-hidden bg-[var(--color-canvas)]">
          {/* VIEW MODE 1: INTERACTIVE PTY TERMINAL (ORCA PARITY) */}
          {viewMode === 'interactive' && (
            <div className="flex flex-1 flex-col overflow-hidden">

              {/* Not Installed Banner / Install Helper */}
              {!isAvailable && (
                <div className="border-b border-amber-500/30 bg-amber-500/10 p-4">
                  <div className="flex items-start justify-between gap-4">
                    <div>
                      <div className="flex items-center gap-2 text-xs font-semibold text-amber-300">
                        <AlertTriangle className="h-4 w-4" />
                        <span>Binary '{selectedHarness?.binary_path}' not found on PATH</span>
                      </div>
                      <p className="mt-1 text-xs text-amber-200/80">
                        To run {selectedHarness?.name} interactively, install its CLI tool or add it to your PATH:
                      </p>
                      {installCmd && (
                        <div className="mt-2 flex items-center gap-2">
                          <code className="rounded bg-black/40 px-2 py-1 font-mono text-xs text-amber-100 border border-amber-500/30">
                            {installCmd}
                          </code>
                          <button
                            type="button"
                            onClick={() => handleCopyInstall(installCmd)}
                            className="flex items-center gap-1 rounded bg-white/10 px-2.5 py-1 text-xs font-medium text-white hover:bg-white/15 border border-white/10"
                          >
                            {copiedInstall ? (
                              <Check className="h-3.5 w-3.5 text-white" />
                            ) : (
                              <Copy className="h-3.5 w-3.5" />
                            )}
                            {copiedInstall ? 'Copied' : 'Copy Command'}
                          </button>
                        </div>
                      )}
                    </div>
                    <button
                      type="button"
                      onClick={() => fetchHarnesses()}
                      className="shrink-0 rounded-md border border-amber-500/40 bg-amber-500/20 px-3 py-1.5 text-xs font-medium text-amber-200 hover:bg-amber-500/30"
                    >
                      Rescan PATH
                    </button>
                  </div>
                </div>
              )}

              {/* Error Alert */}
              {ptyError && (
                <div className="border-b border-rose-500/30 bg-rose-500/10 p-3 text-xs text-rose-300 flex items-center gap-2">
                  <AlertTriangle className="h-4 w-4 shrink-0" />
                  <span>{ptyError}</span>
                </div>
              )}

              {/* PTY Interactive Terminal (xterm.js + FitAddon) */}
              <div className="relative flex-1 overflow-hidden bg-[#0a0a0c]">
                {activePtyId ? (
                  <XTerminalView
                    key={activePtyId}
                    sessionId={activePtyId}
                    onSessionTerminated={() => {
                      setPtySessions((prev) =>
                        prev.map((s) => (s.id === activePtyId ? { ...s, status: { type: 'exited' } } : s))
                      );
                    }}
                  />
                ) : (
                  <div className="flex h-full flex-col items-center justify-center text-center text-zinc-500">
                    <Terminal className="h-8 w-8 text-zinc-600 mb-2" />
                    <p className="text-sm font-medium text-zinc-400">
                      No active interactive process for {selectedHarness?.name || 'this agent'}
                    </p>
                    <p className="text-xs text-zinc-600 mt-1 max-w-sm">
                      Click &quot;Restart&quot; above to launch{' '}
                      <code>{selectedHarness?.launch_cmd || selectedHarness?.binary_path}</code> in{' '}
                      {selectedWorkspace?.name || 'the workspace'}.
                    </p>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* VIEW MODE 2: GOVERNED RUN & INSPECTOR (CUSTOS SADE MEDIATED) */}
          {viewMode === 'governed' && (
            <div className="flex flex-1 flex-col overflow-y-auto p-6 space-y-5">
              {selectedHarness && (
                <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-5">
                  <div className="flex items-center justify-between border-b border-[var(--color-border-muted)] pb-3">
                    <div className="flex items-center gap-3">
                      <div className="wb-pane-toolbar flex h-9 w-9 items-center justify-center rounded-lg bg-[var(--color-surface-2)]">
                        <Zap className="h-5 w-5 text-amber-400" />
                      </div>
                      <div>
                        <h2 className="text-sm font-semibold text-[var(--color-fg-default)]">
                          {selectedHarness.name} Mediated Runner
                        </h2>
                        <div className="text-xs text-[var(--color-fg-muted)]">
                          Binary:{' '}
                          <code className="rounded bg-[var(--color-canvas)] px-1 py-0.5 font-mono text-[11px]">
                            {selectedHarness.binary_path}
                          </code>{' '}
                          • Assurance Tier:{' '}
                          <span className="font-medium text-[var(--color-fg-default)]">
                            {selectedHarness.profile.tool_mediation}
                          </span>
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      {renderMaturityBadge(selectedHarness.maturity)}
                      {renderMediationBadge(selectedHarness.profile.tool_mediation)}
                    </div>
                  </div>

                  {selectedHarness.maturity === 'discovered_cli' && (
                    <div className="mt-3 flex items-start gap-2 rounded-lg border border-amber-500/20 bg-amber-500/10 p-2.5 text-xs text-amber-300">
                      <AlertCircle className="h-4 w-4 shrink-0 mt-0.5 text-amber-400" />
                      <div>
                        <span className="font-semibold">Discovered CLI Harness:</span> This agent executes as a standard host CLI sub-process without protocol conformance for structured tool events or live mid-turn steering. Best experienced in <strong>Interactive PTY</strong> mode.
                      </div>
                    </div>
                  )}

                  {/* Instruction Input & Autonomous Jobs */}
                  <div className="mt-4 space-y-4">
                    <div>
                      <div className="flex items-center justify-between mb-1">
                        <label className="text-xs font-medium text-[var(--color-fg-muted)]">
                          Instruction / Agent Goal Prompt
                        </label>
                        <div className="flex items-center gap-2">
                          <span className="text-[10px] text-[var(--color-fg-subtle)] uppercase tracking-wider font-semibold">
                            Autonomous Jobs:
                          </span>
                          <JobActionButton
                            jobId="engineering__bugfix"
                            label="Bugfix Job"
                            icon={AlertTriangle}
                            getInputs={() => {
                              if (!selectedWorkspaceId || !instruction.trim()) return null;
                              return {
                                issue_description: instruction.trim(),
                                allowed_paths: [selectedWorkspaceId],
                              };
                            }}
                            sessionId={sessionId}
                            workspaceId={selectedWorkspaceId}
                            sourceTabId="kanban"
                            onNavigate={onOpenTab}
                            notReadyTooltip="Enter an instruction and select a workspace to run Bugfix job"
                            readyTooltip="Run autonomous bugfix job with regression test verification"
                            onShowToast={onShowToast}
                            variant="toolbar"
                            size="sm"
                          />
                          <JobActionButton
                            jobId="engineering__feature"
                            label="Feature Job"
                            icon={Zap}
                            getInputs={() => {
                              if (!selectedWorkspaceId || !instruction.trim()) return null;
                              return {
                                specification: instruction.trim(),
                                contracts: 'Comply with domain schemas and unit test suites',
                              };
                            }}
                            sessionId={sessionId}
                            workspaceId={selectedWorkspaceId}
                            sourceTabId="kanban"
                            onNavigate={onOpenTab}
                            notReadyTooltip="Enter an instruction and select a workspace to run Feature job"
                            readyTooltip="Run autonomous feature implementation job"
                            onShowToast={onShowToast}
                            variant="toolbar"
                            size="sm"
                          />
                          <JobActionButton
                            jobId="engineering__diagnose"
                            label="Diagnose"
                            icon={ShieldAlert}
                            getInputs={() => {
                              if (!selectedWorkspaceId || !instruction.trim()) return null;
                              return {
                                symptoms: instruction.trim(),
                                environment: `Workspace: ${selectedWorkspaceId}, Harness: ${selectedHarnessId}`,
                              };
                            }}
                            sessionId={sessionId}
                            workspaceId={selectedWorkspaceId}
                            sourceTabId="kanban"
                            onNavigate={onOpenTab}
                            notReadyTooltip="Enter an instruction and select a workspace to diagnose"
                            readyTooltip="Investigate runtime errors, trace symptoms, and formulate hypotheses"
                            onShowToast={onShowToast}
                            variant="toolbar"
                            size="sm"
                          />
                        </div>
                      </div>
                      <textarea
                        rows={3}
                        value={instruction}
                        onChange={(e) => setInstruction(e.target.value)}
                        placeholder="e.g. Implement the new OAuth token exchange handler and add unit tests..."
                        className="w-full rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] p-3 font-mono text-xs text-[var(--color-fg-default)] placeholder:text-[var(--color-fg-subtle)] focus:border-[var(--color-border-focus)] focus:outline-none"
                      />
                    </div>

                    <div className="flex items-center justify-between pt-1">
                      <div className="text-[11px] text-[var(--color-fg-muted)]">
                        Governed runs validate Permits and enforce audit trail records in the SADE ledger.
                      </div>
                      <button
                        type="button"
                        onClick={handleRun}
                        disabled={
                          running ||
                          !instruction.trim() ||
                          !selectedHarness?.is_available ||
                          !selectedWorkspaceId
                        }
                        className="flex items-center gap-2 rounded-lg bg-[var(--color-btn-primary-bg)] px-4 py-2 text-xs font-medium text-[var(--color-btn-primary-fg)] transition hover:opacity-90 disabled:opacity-50"
                      >
                        {running ? (
                          <>
                            <RefreshCw className="h-3.5 w-3.5 animate-spin" />
                            Running...
                          </>
                        ) : (
                          <>
                            <Play className="h-3.5 w-3.5 fill-current" />
                            Start Governed Run
                          </>
                        )}
                      </button>
                    </div>
                  </div>
                </div>
              )}

              {/* Error Banner */}
              {errorMessage && (
                <div className="flex items-center gap-2 rounded-xl border border-rose-500/30 bg-rose-500/10 p-3 text-xs text-rose-400">
                  <AlertTriangle className="h-4 w-4 flex-shrink-0" />
                  <span>{errorMessage}</span>
                </div>
              )}

              {/* Status Banner */}
              {statusMessage && (
                <div className="flex items-center gap-2 rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-3 text-xs text-[var(--color-fg-default)]">
                  <CheckCircle2 className="h-4 w-4 flex-shrink-0 text-white/70" />
                  <span>{statusMessage}</span>
                </div>
              )}

              {running && runResult && (
                <button
                  type="button"
                  onClick={() =>
                    void daemonClient
                      .cancelRun({
                        run_id: runResult.run_id,
                        reason: 'Stopped from native inspector',
                      })
                      .catch((error) => setErrorMessage(String(error)))
                  }
                  className="rounded border border-[var(--color-border-default)] px-3 py-1 text-xs text-[var(--color-fg-default)]"
                >
                  Stop
                </button>
              )}

              {activities.map((activity, index) => (
                <details
                  key={index}
                  className="rounded border border-[var(--color-border-default)] p-2 text-xs"
                >
                  <summary className="cursor-pointer font-medium text-[var(--color-fg-default)]">
                    {activity.kind === 'reasoning_summary'
                      ? 'Reasoning summary'
                      : activity.kind === 'tool_input'
                      ? 'Tool input'
                      : 'Tool output'}
                  </summary>
                  <pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap font-mono text-[11px] text-[var(--color-fg-muted)]">
                    {JSON.stringify(activity.payload, null, 2)}
                  </pre>
                </details>
              ))}

              {/* Governed Run Output */}
              {runResult && (
                <div className="space-y-4">
                  <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-canvas)] p-4 shadow-sm">
                    <div className="flex items-center justify-between border-b border-[var(--color-border-muted)] pb-2.5">
                      <div className="flex items-center gap-2">
                        <Terminal className="h-4 w-4 text-[var(--color-fg-muted)]" />
                        <span className="text-xs font-semibold text-[var(--color-fg-default)]">
                          Task Run Output
                        </span>
                        <span
                          className={`rounded px-1.5 py-0.5 text-[10px] font-semibold ${
                            runResult.status === 'completed'
                              ? 'bg-white/10 text-white'
                              : 'bg-white/5 text-white/50'
                          }`}
                        >
                          {runResult.status}
                        </span>
                      </div>
                      <button
                        type="button"
                        onClick={() => handleCopy(runResult.output || '')}
                        className="flex items-center gap-1 rounded bg-[var(--color-surface-1)] px-2 py-1 text-[11px] text-[var(--color-fg-muted)] hover:text-[var(--color-fg-default)]"
                      >
                        {copiedStdout ? (
                          <Check className="h-3 w-3 text-white" />
                        ) : (
                          <Copy className="h-3 w-3" />
                        )}
                        {copiedStdout ? 'Copied' : 'Copy Output'}
                      </button>
                    </div>
                    <pre className="mt-3 max-h-64 overflow-y-auto font-mono text-[11px] leading-relaxed text-[var(--color-fg-default)]">
                      {runResult.output || '(No output produced)'}
                    </pre>
                  </div>

                  {/* Assurance & Lineage Boundary */}
                  <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-4">
                    <div className="mb-3 flex items-center justify-between">
                      <div>
                        <h3 className="text-xs font-semibold text-[var(--color-fg-default)]">
                          Run identity and assurance
                        </h3>
                        <p className="text-[11px] text-[var(--color-fg-muted)]">
                          Process completion is recorded, but patch acceptance still requires
                          independent diff and test evidence.
                        </p>
                      </div>
                      <span className="rounded bg-[var(--color-surface-2)] px-2 py-0.5 text-[10px] font-medium text-[var(--color-fg-muted)]">
                        {selectedHarness?.profile.tool_mediation || 'unknown'}
                      </span>
                    </div>
                    <div className="grid grid-cols-2 gap-2 text-[11px] text-[var(--color-fg-muted)]">
                      <div>
                        Task <code className="text-[var(--color-fg-default)]">{runResult.task_id}</code>
                      </div>
                      <div>
                        Run <code className="text-[var(--color-fg-default)]">{runResult.run_id}</code>
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
