import { useCallback, useEffect, useRef, useState } from 'react';
import { Columns2, Eraser, Play, Plus, RefreshCw, Square } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { useAppContext } from '@/context/AppContext';
import { useTerminalSession } from '@/hooks/useTerminalSession';
import { getAgentBrandIcon } from '@/components/common/AgentIcons';
import { XTerminalView } from '@/components/common/XTerminalView';
import { MarkdownViewer } from '@/components/viewer/MarkdownViewer';
import { WorkbenchPane, WorkbenchEmptyState, WorkbenchError } from '@/components/common/WorkbenchPane';
import type { HarnessDescriptor, LiveRunSnapshot, RunHandle } from '@/types/domain';
import type { OrcaTabId } from '../OrcaTabbedContainer';

export interface AgentsWorkbenchPaneProps {
  sessionId?: string;
  runId?: string;
  tabId?: string;
  initialHarnessId?: string;
  initialWorkspaceId?: string;
  activeWorkspaceId?: string;
  isActive?: boolean;
  onSelectWorkspace?: (id: string) => void;
  onShowToast?: (message: string) => void;
  onRunStarted?: (sessionId: string, run: RunHandle) => void;
  onOpenTab?: (id: OrcaTabId, url?: string, title?: string, pane?: 'primary' | 'split', workspaceId?: string) => void;
}

const terminalRunStatuses = new Set(['completed', 'failed', 'cancelled', 'interrupted']);

export function AgentsWorkbenchPane({ sessionId, runId, tabId = 'agents', initialHarnessId, initialWorkspaceId, activeWorkspaceId, isActive = true, onRunStarted, onOpenTab }: AgentsWorkbenchPaneProps) {
  const { workspaces, startNativeConversation } = useAppContext();
  const workspaceId = initialWorkspaceId || activeWorkspaceId || '';
  const workspace = workspaces.find(item => item.id === workspaceId);
  const [harnesses, setHarnesses] = useState<HarnessDescriptor[]>([]);
  const [harnessId, setHarnessId] = useState(initialHarnessId || '');
  const [loading, setLoading] = useState(true);
  const [loadAttempt, setLoadAttempt] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<'terminal' | 'run'>(runId ? 'run' : 'terminal');
  const [instruction, setInstruction] = useState('');
  const [starting, setStarting] = useState(false);
  const startingRef = useRef(false);
  const [cancelling, setCancelling] = useState(false);
  const [run, setRun] = useState<{ handle: RunHandle; sessionId: string } | null>(null);
  const [snapshot, setSnapshot] = useState<LiveRunSnapshot | null>(null);
  const [clearToken, setClearToken] = useState(0);
  const mounted = useRef(true);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    daemonClient.listHarnesses().then(records => {
      if (cancelled) return;
      setHarnesses(records);
      setHarnessId(previous => records.some(item => item.id === previous) ? previous : records.find(item => item.is_available)?.id || records[0]?.id || '');
      setError(null);
    }).catch(reason => { if (!cancelled) setError(String(reason)); }).finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [loadAttempt]);
  useEffect(() => { if (initialHarnessId) setHarnessId(initialHarnessId); }, [initialHarnessId]);
  const harness = harnesses.find(item => item.id === harnessId);
  const terminal = useTerminalSession(tabId, workspaceId, harness?.launch_cmd || harness?.binary_path);
  const terminalRunning = terminal.session?.status.type === 'active';
  const runSessionId = run?.sessionId || sessionId;
  const currentRunId = run?.handle.run_id || runId;
  const runActive = starting || (!!currentRunId && (!snapshot || !terminalRunStatuses.has(snapshot.status)));

  useEffect(() => {
    if (!runSessionId || !currentRunId) return;
    const controller = new AbortController();
    setSnapshot(null);
    void daemonClient.watchRun(runSessionId, currentRunId, setSnapshot, controller.signal)
      .catch(reason => { if (!controller.signal.aborted) setError(String(reason)); });
    return () => controller.abort();
  }, [runSessionId, currentRunId]);

  const launchRun = async () => {
    if (!instruction.trim() || !harness?.is_available || !workspace || startingRef.current || runActive) return;
    startingRef.current = true;
    setStarting(true); setError(null);
    try {
      const result = await startNativeConversation(instruction.trim(), harness.id, workspace.id);
      onRunStarted?.(result.sessionId, result.run);
      if (mounted.current) { setRun({ handle: result.run, sessionId: result.sessionId }); setInstruction(''); }
    } catch (reason) { if (mounted.current) setError(String(reason)); }
    finally { startingRef.current = false; if (mounted.current) setStarting(false); }
  };
  const cancelRun = async () => {
    if (!currentRunId || cancelling) return;
    setCancelling(true);
    try { await daemonClient.cancelRun({ run_id: currentRunId, reason: 'Stopped from agent workbench' }); }
    catch (reason) { setError(String(reason)); }
    finally { if (mounted.current) setCancelling(false); }
  };
  const onExit = useCallback(() => { void terminal.refresh(); }, [terminal.refresh]);
  const unavailable = !workspace ? 'Select a workspace and reopen this agent there.' : !harness?.is_available ? 'This CLI is not available on the execution host. Configure it in Settings → Agent runtimes, then refresh.' : null;

  return <WorkbenchPane toolbar={<>
    <span className="wb-scope" title={workspace?.path}>{workspace?.name || 'No workspace'}</span>
    <select className="wb-inline-select max-w-48" aria-label="Coding agent" value={harnessId} disabled={loading || terminalRunning || runActive} onChange={event => setHarnessId(event.target.value)}>
      {!harnesses.length && <option value="">{loading ? 'Loading agents…' : 'No agents found'}</option>}
      {harnesses.map(item => <option key={item.id} value={item.id}>{item.name}{item.is_available ? '' : ' · unavailable'}</option>)}
    </select>
    <div className="wb-toolbar-spacer" />
    <div className="wb-segmented" role="group" aria-label="Agent interaction">
      <button type="button" aria-pressed={mode === 'terminal'} onClick={() => setMode('terminal')}>Terminal</button>
      <button type="button" aria-pressed={mode === 'run'} onClick={() => setMode('run')}>Task run</button>
    </div>
    <button className="wb-icon-button" type="button" title="Refresh agents" aria-label="Refresh agent availability" disabled={loading} onClick={() => setLoadAttempt(value => value + 1)}><RefreshCw size={14} /></button>
    {onOpenTab && <button className="wb-icon-button" type="button" title="New agent tab" aria-label="New agent tab" onClick={() => onOpenTab('agents', undefined, 'Agents', 'primary', workspaceId)}><Plus size={14} /></button>}
  </>}>
    <WorkbenchError message={error || (mode === 'terminal' ? terminal.error : null)} onRetry={() => { setLoadAttempt(value => value + 1); void terminal.refresh(); }} />
    <div className="wb-context-line">
      {harness && getAgentBrandIcon(harness.id, 13, 'shrink-0')}
      <span>{mode === 'terminal' ? 'Interactive CLI · provider governed' : 'Task run · ' + (harness?.profile.tool_mediation?.replace(/_/g, ' ') || 'assurance unavailable')}</span>
      <span className="wb-toolbar-spacer" />
      {mode === 'terminal' ? <>
        <span role="status">{terminal.busy ? 'Connecting…' : terminal.session?.status.type || 'Not started'}</span>
        <button type="button" className="wb-icon-button" aria-label="Clear agent terminal display" title="Clear display" disabled={!terminal.session} onClick={() => setClearToken(value => value + 1)}><Eraser size={13} /></button>
        {onOpenTab && <button type="button" className="wb-icon-button" aria-label="Open shell beside agent" title="Shell beside agent" onClick={() => onOpenTab('terminal', undefined, 'Terminal', 'split', workspaceId)}><Columns2 size={13} /></button>}
        {terminalRunning
          ? <button type="button" className="canvas-open-button" disabled={terminal.busy} onClick={() => void terminal.stop()}><Square size={11} />Stop</button>
          : <button type="button" className="canvas-open-button" disabled={!!unavailable || loading || terminal.busy} onClick={() => void terminal.start()}><Play size={11} />{terminal.error ? 'Retry launch' : 'Start agent'}</button>}
      </> : <span role="status">{snapshot?.reconnecting ? 'Reconnecting…' : starting ? 'Starting…' : snapshot?.status || (currentRunId ? 'Connecting…' : 'Ready')}</span>}
    </div>
    {mode === 'terminal' ? terminal.session
      ? <div className="min-h-0 flex-1 overflow-hidden"><XTerminalView sessionId={terminal.session.id} visible={isActive} clearToken={clearToken} onSessionTerminated={onExit} /></div>
      : <WorkbenchEmptyState title={loading ? 'Finding installed agents…' : terminal.busy ? 'Connecting to agent…' : harness?.name || 'Choose a coding agent'} busy={loading || terminal.busy}
          description={unavailable || <>Start an interactive session in <code>{workspace?.path}</code>. The agent uses its own tools and permission settings.</>}
          action={!unavailable && !terminal.busy && !loading ? <button type="button" className="canvas-open-button" onClick={() => void terminal.start()}><Play size={13} />Start {harness?.name}</button> : undefined} />
    : <>
      <div className="wb-agent-output">
        {snapshot?.text ? <MarkdownViewer content={snapshot.text} /> : <WorkbenchEmptyState title={runActive ? 'Agent is working…' : 'Delegate work to this agent'} busy={runActive} description={unavailable || 'Describe the outcome. Custos records the conversation, Task and Run in the selected workspace.'} />}
        {!!snapshot?.activities.length && <details className="wb-disclosure">
          <summary>{snapshot.activities.length} recorded activities</summary>
          {snapshot.activities.map((activity, index) => <pre key={index} className="wb-code-block">{activity.kind}{'\n'}{JSON.stringify(activity.payload, null, 2)}</pre>)}
        </details>}
        {snapshot?.error && <WorkbenchError message={snapshot.error} />}
      </div>
      <form className="wb-agent-composer" onSubmit={event => { event.preventDefault(); void launchRun(); }}>
        <label className="sr-only" htmlFor={'agent-instruction-' + tabId}>Task instruction</label>
        <textarea id={'agent-instruction-' + tabId} value={instruction} onChange={event => setInstruction(event.target.value)} placeholder="Describe the work to do…" rows={3} disabled={runActive}
          onKeyDown={event => { if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') { event.preventDefault(); void launchRun(); } }} />
        <div className="flex items-center gap-2">
          <span className="wb-meta min-w-0 flex-1 truncate" title={currentRunId}>{currentRunId ? 'Run ' + currentRunId.slice(-12) : '⌘ / Ctrl + Enter to start'}</span>
          {runActive ? <button type="button" className="canvas-open-button" disabled={starting || cancelling || !harness?.profile.supports_cancel} onClick={() => void cancelRun()}><Square size={12} />{cancelling ? 'Stopping…' : 'Stop run'}</button>
            : <button type="submit" className="canvas-open-button" disabled={!!unavailable || !instruction.trim() || loading || harness?.maturity === 'discovered_cli'}><Play size={12} />Start task</button>}
        </div>
        {harness?.maturity === 'discovered_cli' && <p className="wb-meta">This CLI supports interactive Terminal mode. Task-run integration is not available.</p>}
      </form>
    </>}
  </WorkbenchPane>;
}
