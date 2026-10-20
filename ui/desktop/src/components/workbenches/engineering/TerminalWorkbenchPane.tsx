import { useCallback, useEffect, useState } from 'react';
import { Columns2, Eraser, Play, Plus, RefreshCw, Square } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { useAppContext } from '@/context/AppContext';
import { useTerminalSession } from '@/hooks/useTerminalSession';
import { XTerminalView } from '@/components/common/XTerminalView';
import { WorkbenchPane, WorkbenchEmptyState, WorkbenchError } from '@/components/common/WorkbenchPane';
import type { TerminalSession } from '@/types/domain';
import type { OrcaTabId } from '../OrcaTabbedContainer';

export interface TerminalWorkbenchPaneProps {
  tabId?: string;
  initialWorkspaceId?: string;
  activeWorkspaceId?: string;
  onSelectWorkspace?: (id: string) => void;
  initialCommand?: string;
  isActive?: boolean;
  onOpenTab?: (id: OrcaTabId, url?: string, title?: string, pane?: 'primary' | 'split', workspaceId?: string) => void;
}

export function TerminalWorkbenchPane({ tabId = 'terminal', initialWorkspaceId, activeWorkspaceId, initialCommand, onOpenTab, isActive = true }: TerminalWorkbenchPaneProps) {
  const { workspaces } = useAppContext();
  const workspaceId = initialWorkspaceId || activeWorkspaceId || '';
  const workspace = workspaces.find(item => item.id === workspaceId);
  const terminal = useTerminalSession(tabId, workspaceId, initialCommand);
  const [sessions, setSessions] = useState<TerminalSession[]>([]);
  const [listError, setListError] = useState<string | null>(null);
  const [clearToken, setClearToken] = useState(0);
  const refreshSessions = useCallback(async () => {
    if (!workspaceId) return;
    try { setSessions(await daemonClient.listTerminals(workspaceId)); setListError(null); }
    catch (reason) { setListError(String(reason)); }
  }, [workspaceId]);
  useEffect(() => { if (isActive) void refreshSessions(); }, [refreshSessions, isActive, terminal.session?.id]);
  const running = terminal.session?.status.type === 'active';
  const start = async () => { await terminal.start(); await refreshSessions(); };
  const stop = async () => { await terminal.stop(); await refreshSessions(); };
  const open = (pane: 'primary' | 'split') => onOpenTab?.('terminal', undefined, 'Terminal', pane, workspaceId);

  return <WorkbenchPane toolbar={<>
    <span className="wb-scope" title={workspace?.path}>{workspace?.name || 'No workspace'}</span>
    <span className="wb-meta" role="status">{terminal.busy ? 'Connecting…' : running ? 'Running' : terminal.session ? 'Exited' : 'No process'}</span>
    <div className="wb-toolbar-spacer" />
    <button className="wb-icon-button" type="button" aria-label="Refresh terminal sessions" title="Refresh" onClick={() => { void terminal.refresh(); void refreshSessions(); }} disabled={terminal.busy}><RefreshCw size={14} /></button>
    <button className="wb-icon-button" type="button" aria-label="Clear terminal display" title="Clear display" onClick={() => setClearToken(value => value + 1)} disabled={!terminal.session}><Eraser size={14} /></button>
    {onOpenTab && <>
      <button className="wb-icon-button" type="button" aria-label="New terminal tab" title="New terminal" onClick={() => open('primary')}><Plus size={14} /></button>
      <button className="wb-icon-button" type="button" aria-label="Split terminal right" title="Split right" onClick={() => open('split')}><Columns2 size={14} /></button>
    </>}
    {running
      ? <button type="button" className="canvas-open-button" disabled={terminal.busy} onClick={() => void stop()}><Square size={12} />Stop</button>
      : <button type="button" className="canvas-open-button" disabled={terminal.busy || !workspace} onClick={() => void start()}><Play size={12} />{terminal.error ? 'Retry start' : 'Start shell'}</button>}
  </>}>
    <WorkbenchError message={terminal.error || listError} onRetry={() => { void terminal.refresh(); void refreshSessions(); }} />
    {!!sessions.length && <div className="wb-context-line">
      <label htmlFor={'terminal-session-' + tabId}>Session</label>
      <select id={'terminal-session-' + tabId} className="wb-inline-select" value={terminal.session?.id || ''} disabled={terminal.busy} onChange={event => { if (event.target.value) void terminal.attach(event.target.value); }}>
        <option value="">Choose an existing terminal…</option>
        {sessions.map(item => <option key={item.id} value={item.id}>{item.command} · {item.id.slice(-8)} · {item.status.type}</option>)}
      </select>
    </div>}
    {terminal.session ? <div className="min-h-0 flex-1 overflow-hidden">
      <XTerminalView sessionId={terminal.session.id} visible={isActive} clearToken={clearToken} onSessionTerminated={terminal.refresh} />
    </div> : <WorkbenchEmptyState title={terminal.busy ? 'Connecting to terminal…' : 'A shell for this workspace'} busy={terminal.busy}
      description={workspace ? <>Run commands in <code>{workspace.path}</code>. Start a shell or attach to an existing session above.</> : 'Select a workspace and open Terminal from that workspace.'}
      action={workspace && !terminal.busy ? <button type="button" className="canvas-open-button" onClick={() => void start()}><Play size={13} />{terminal.error ? 'Retry start' : 'Start shell'}</button> : undefined} />}
  </WorkbenchPane>;
}
