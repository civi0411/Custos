import React, { useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { ArrowLeft, CheckSquare, ExternalLink, MessageSquare, RefreshCw } from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { useScreenContext } from '@/context/ScreenContext';
import { daemonClient } from '@/api/daemon_client';
import type { Session as WorkspaceSession, ProjectData } from '@/types';
import type { Session as SavedSession, SessionJournalEntry, Task } from '@/types/domain';

type ConversationRow = { id: string; title: string; project?: string; session?: WorkspaceSession; taskId?: string; pack?: string; createdAt: string };
/** The daemon list determines existence; local projections supply scope, never extra records. */
export function conversationHistoryRows(saved: SavedSession[], projects: ProjectData): ConversationRow[] {
  const local = new Map<string, { project: string; session: WorkspaceSession }>();
  for (const [project, sessions] of Object.entries(projects)) for (const session of sessions) {
    if (!local.has(session.id)) local.set(session.id, { project, session });
  }
  return [...new Map(saved.map(session => [session.id, session])).values()].map(session => {
    const known = local.get(session.id);
    return { id: session.id, title: known?.session.title || session.title || session.id,
      project: known?.project, session: known?.session, taskId: known?.session.taskId || session.attached_to || session.task_id,
      pack: known?.session.pack, createdAt: session.created_at };
  });
}
export function historyWorkbench(pack?: string): 'engineering' | 'research' | 'copilot' | undefined {
  if (['engineering', 'coding', 'code'].includes(pack || '')) return 'engineering';
  if (pack === 'research') return 'research';
  if (['assistant', 'copilot', 'chat'].includes(pack || '')) return 'copilot';
  return undefined;
}
export function userHistoryTasks(tasks: Task[]): Task[] {
  return tasks.filter(task => !task.metadata?.turn_task && task.metadata?.kind !== 'conversation_chat_turn');
}

/** Cross-workbench history browser. Opening it never changes the active workspace. */
export const HistoryPage: React.FC<{ kind: 'conversations' | 'tasks' }> = ({ kind }) => {
  const app = useAppContext();
  const { updateActiveScreen } = useScreenContext();
  const navigate = useNavigate();
  const [saved, setSaved] = useState<SavedSession[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [query, setQuery] = useState('');
  const [project, setProject] = useState('');
  const [lens, setLens] = useState('');
  const [selectedId, setSelectedId] = useState('');
  const [journal, setJournal] = useState<SessionJournalEntry[]>([]);
  const [journalError, setJournalError] = useState('');
  const [journalLoading, setJournalLoading] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);
  const generation = useRef(0);
  useEffect(() => {
    const request = ++generation.current;
    setLoading(true); setError(''); setSelectedId(''); setSaved([]); setTasks([]);
    Promise.all([daemonClient.listSessions(), kind === 'tasks' ? daemonClient.listTasks() : Promise.resolve([] as Task[])])
      .then(([sessions, records]) => { if (request === generation.current) { setSaved(sessions); setTasks(userHistoryTasks(records)); } })
      .catch(cause => { if (request === generation.current) setError(cause instanceof Error ? cause.message : String(cause)); })
      .finally(() => { if (request === generation.current) setLoading(false); });
    return () => { generation.current++; };
  }, [kind, refreshKey]);
  useEffect(() => { setSelectedId(''); }, [project, lens, query]);
  const conversations = useMemo(() => conversationHistoryRows(saved, app.projectData), [saved, app.projectData]);
  const taskProject = (id: string) => conversations.find(row => row.taskId === id)?.project;
  const visibleConversations = conversations.filter(row => (!project || row.project === project) && (!lens || historyWorkbench(row.pack) === lens) && `${row.title} ${row.id}`.toLowerCase().includes(query.toLowerCase())).sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  const visibleTasks = tasks.filter(task => (!project || taskProject(task.id) === project) && (!lens || historyWorkbench(task.contract?.pack_id || task.metadata?.pack_id || task.contract?.pack) === lens) && `${task.title} ${task.id} ${task.status}`.toLowerCase().includes(query.toLowerCase())).sort((a, b) => b.updated_at.localeCompare(a.updated_at));
  const selectedConversation = kind === 'conversations' ? visibleConversations.find(row => row.id === selectedId) : undefined;
  const selectedTask = kind === 'tasks' ? visibleTasks.find(task => task.id === selectedId) : undefined;
  useEffect(() => {
    setJournal([]); setJournalError(''); setJournalLoading(false);
    if (!selectedConversation) return;
    let live = true; setJournalLoading(true);
    daemonClient.getSessionJournal(selectedConversation.id).then(value => { if (live) setJournal(value); }, cause => { if (live) setJournalError(String(cause?.message || cause)); }).finally(() => { if (live) setJournalLoading(false); });
    return () => { live = false; };
  }, [selectedConversation?.id, refreshKey]);
  const openConversation = (row: ConversationRow) => {
    if (!row.project || !row.session) return;
    const homeWorkbench = historyWorkbench(row.pack) || (row.session.openedLenses?.[0] === 'research' ? 'research' : row.session.openedLenses?.[0] === 'code' ? 'engineering' : 'copilot');
    app.setCurrentProject(row.project); app.setActiveSessionId(row.id); app.setActiveWorkbench(homeWorkbench);
    updateActiveScreen({ activeSessionId: row.id, selectedTaskId: null, sidebarView: 'chats', homeWorkbench });
    navigate(`/studio/${encodeURIComponent(row.id)}?view=chats`);
  };
  const openTask = (task: Task) => {
    const owner = taskProject(task.id); if (owner) app.setCurrentProject(owner);
    updateActiveScreen({ selectedTaskId: task.id, sidebarView: 'tasks' });
    navigate(`/studio?view=tasks&task=${encodeURIComponent(task.id)}`);
  };
  const title = kind === 'conversations' ? 'Conversations' : 'Tasks';
  const Icon = kind === 'conversations' ? MessageSquare : CheckSquare;
  return <main className="flex h-full min-h-0 flex-col bg-canvas text-fg-editor">
    <header className="flex shrink-0 flex-wrap items-center gap-3 border-b border-border-muted p-4"><button type="button" aria-label="Back to workspace" onClick={() => navigate('/studio')} className="rounded p-2 hover:bg-surface-2"><ArrowLeft size={16} /></button><Icon size={18} /><div className="mr-auto"><h1 className="text-sm font-semibold">{title}</h1><p className="text-xs text-fg-muted">All saved {title.toLowerCase()} across Projects and workbenches</p></div><button type="button" disabled={loading} onClick={() => setRefreshKey(value => value + 1)} className="flex items-center gap-2 rounded border border-border-default px-3 py-2 text-xs disabled:opacity-40"><RefreshCw size={13} />Refresh</button></header>
    <div className="flex shrink-0 flex-wrap gap-2 border-b border-border-muted p-3"><input aria-label={`Search ${title.toLowerCase()}`} value={query} onChange={event => setQuery(event.target.value)} placeholder="Search title, ID or Task status" className="min-w-40 flex-1 rounded border border-border-default bg-surface-1 px-3 py-2 text-xs" /><select aria-label="History Project" value={project} onChange={event => setProject(event.target.value)} className="rounded border border-border-default bg-surface-1 px-2 text-xs"><option value="">All Projects / unfiled</option>{Object.keys(app.projectData).map(name => <option key={name}>{name}</option>)}</select><select aria-label="History workbench" value={lens} onChange={event => setLens(event.target.value)} className="rounded border border-border-default bg-surface-1 px-2 text-xs"><option value="">All workbenches</option><option value="engineering">Coding</option><option value="research">Research</option><option value="copilot">Copilot</option></select></div>
    {error && <p role="alert" className="p-3 text-xs text-rose-500">Could not load history: {error}</p>}
    {app.savedWorkError && <p role="alert" className="p-3 text-xs text-amber-500">{app.savedWorkError} Project membership may be incomplete. <button type="button" className="underline" onClick={app.retrySavedWork}>Reload saved work</button></p>}
    <div className="grid min-h-0 flex-1 grid-cols-1 overflow-auto lg:grid-cols-[minmax(250px,360px)_minmax(0,1fr)] lg:overflow-hidden">
      <section aria-label={`${title} history`} className="space-y-1 overflow-y-auto border-r border-border-muted p-3">
        {loading && <p role="status" className="p-3 text-xs text-fg-muted">Loading saved {title.toLowerCase()}…</p>}
        {(kind === 'conversations' ? visibleConversations : visibleTasks).map(item => {
          const conversation = kind === 'conversations' ? item as ConversationRow : undefined;
          const task = kind === 'tasks' ? item as Task : undefined;
          return <button key={item.id} type="button" aria-pressed={selectedId === item.id} onClick={() => setSelectedId(item.id)} className={`block w-full rounded-lg border p-3 text-left hover:bg-surface-2 ${selectedId === item.id ? 'border-[var(--color-border-focus)] bg-surface-2' : 'border-border-muted bg-surface-1'}`}><span className="block truncate text-xs font-medium">{item.title}</span><span className="mt-1 block text-[11px] text-fg-muted">{conversation?.project || (task && taskProject(task.id)) || 'Unfiled'} · {conversation ? historyWorkbench(conversation.pack) || 'Unclassified' : task?.status}</span><span className="mt-1 block truncate font-mono text-[10px] text-fg-muted">{item.id}</span></button>;
        })}
        {!loading && !error && !(kind === 'conversations' ? visibleConversations.length : visibleTasks.length) && <p className="p-3 text-xs text-fg-muted">No saved {title.toLowerCase()} match these filters.</p>}
      </section>
      <section aria-label="History detail" className="min-h-72 space-y-4 overflow-auto p-5">
        {selectedConversation ? <><div className="flex flex-wrap items-center justify-between gap-3"><h2 className="text-sm font-semibold">{selectedConversation.title}</h2><button type="button" disabled={!selectedConversation.project} onClick={() => openConversation(selectedConversation)} className="flex items-center gap-2 rounded border border-border-default px-3 py-2 text-xs disabled:opacity-40"><ExternalLink size={13} />Open in workspace</button></div>{!selectedConversation.project && <p className="text-xs text-fg-muted">Project binding is not loaded. <button type="button" onClick={app.retrySavedWork} className="underline">Reload saved work</button> to resolve it; history remains readable here.</p>}{journalLoading && <p role="status" className="text-xs text-fg-muted">Loading conversation journal…</p>}{journalError && <p role="alert" className="text-xs text-rose-500">{journalError}</p>}{journal.map((entry, index) => <article key={entry.id || entry.entry_id || index} className="rounded-lg border border-border-muted bg-surface-1 p-3"><h3 className="mb-2 text-[10px] uppercase text-fg-muted">{entry.role || entry.entry_type || 'Journal entry'}</h3><pre className="whitespace-pre-wrap break-words text-xs leading-5">{entry.content || entry.entry_data || 'No text recorded'}</pre></article>)}{!journalLoading && !journalError && !journal.length && <p className="text-xs text-fg-muted">No journal entries recorded.</p>}</> : selectedTask ? <><div className="flex items-center justify-between gap-3"><h2 className="text-sm font-semibold">{selectedTask.title}</h2><button type="button" onClick={() => openTask(selectedTask)} className="rounded border border-border-default px-3 py-2 text-xs">Open Task in workspace</button></div><p className="text-xs text-fg-muted">{selectedTask.status} · {selectedTask.contract?.pack_id || selectedTask.contract?.pack || 'Pack not reported'}</p><p className="whitespace-pre-wrap text-xs">{selectedTask.contract?.description || 'No description recorded.'}</p><h3 className="text-xs font-medium">Acceptance criteria</h3>{selectedTask.contract?.criteria?.map((criterion, index) => <p key={index} className="text-xs text-fg-muted">{criterion.name}: {criterion.description || 'No details recorded'}</p>)}<h3 className="text-xs font-medium">Linked conversations</h3>{conversations.filter(row => row.taskId === selectedTask.id).map(row => <button key={row.id} type="button" disabled={!row.project} onClick={() => openConversation(row)} className="block rounded border border-border-muted px-3 py-2 text-xs disabled:opacity-40">{row.title}</button>)}</> : <div className="p-8 text-center text-xs text-fg-muted">Select a saved {kind === 'tasks' ? 'Task' : 'conversation'} to inspect it. Browsing history does not start work or switch the execution workspace.</div>}
      </section>
    </div>
  </main>;
};
