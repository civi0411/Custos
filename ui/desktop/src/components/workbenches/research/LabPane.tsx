import React, { useState } from 'react';
import { NotebookWorkbenchPane, type NotebookWorkbenchPaneProps } from './NotebookWorkbenchPane';
import { RunsLedgerPane } from './RunsLedgerPane';
import { TrainingRunsPane } from './TrainingRunsPane';
export type LabPaneProps = NotebookWorkbenchPaneProps;
/** Navigation never launches, cancels or retargets a run. */
export const LabPane: React.FC<LabPaneProps> = (props) => {
  const [view, setView] = useState<'notebook' | 'runs' | 'training'>('notebook');
  const views = ['notebook', 'runs', 'training'] as const;
  const navigation = <nav aria-label="Lab views" className="flex gap-1 text-xs shrink-0" onKeyDown={event => {
    if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey || !['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    const buttons = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>('button'));
    const index = buttons.indexOf(event.target as HTMLButtonElement);
    if (index < 0) return;
    event.preventDefault();
    const next = event.key === 'Home' ? 0 : event.key === 'End' ? views.length - 1 : (index + (event.key === 'ArrowRight' ? 1 : -1) + views.length) % views.length;
    // Keep navigation mounted while switching so keyboard focus is retained.
    setView(views[next]); buttons[next]?.focus();
  }}>
    {views.map(item => <button key={item} type="button" className="workbench-tool" aria-pressed={view === item} onClick={() => setView(item)}>{item === 'notebook' ? 'Notebook' : item === 'runs' ? 'Runs' : 'Training'}</button>)}
  </nav>;
  return <div className="h-full min-h-0 flex flex-col relative bg-canvas">
    <div className="px-3 py-1 border-b border-border-default">{navigation}</div>
    <div className="flex-1 min-h-0 relative" hidden={view !== 'notebook'}><NotebookWorkbenchPane {...props} /></div>
    {view === 'runs' && <div className="flex-1 min-h-0"><RunsLedgerPane sessionId={props.sessionId} onShowToast={props.onShowToast} onOpenTab={props.onOpenTab} /></div>}
    {view === 'training' && <div className="flex-1 min-h-0"><TrainingRunsPane key={props.sessionId ?? 'no-session'} sessionId={props.sessionId} /></div>}
  </div>;
};
