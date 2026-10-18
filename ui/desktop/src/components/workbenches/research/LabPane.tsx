import React, { useState } from 'react';
import { NotebookWorkbenchPane, type NotebookWorkbenchPaneProps } from './NotebookWorkbenchPane';
import { RunsLedgerPane } from './RunsLedgerPane';
export type LabPaneProps = NotebookWorkbenchPaneProps;
/** Lab is navigation over the existing notebook and recorded-run resources. */
export const LabPane: React.FC<LabPaneProps> = (props) => {
  const [view, setView] = useState<'notebook' | 'runs'>('notebook');
  const navigation = <nav aria-label="Lab views" className="flex gap-1 text-xs shrink-0">
    <button type="button" className="workbench-tool" aria-pressed={view === 'notebook'} onClick={() => setView('notebook')}>Notebook</button>
    <button type="button" className="workbench-tool" aria-pressed={view === 'runs'} onClick={() => setView('runs')}>Runs</button>
  </nav>;
  return <div className="h-full min-h-0 flex flex-col relative bg-canvas">
    <div className="flex-1 min-h-0 relative" hidden={view !== 'notebook'}><NotebookWorkbenchPane {...props} toolbarNavigation={navigation} /></div>
    {view === 'runs' && <div className="flex-1 min-h-0 flex flex-col"><div className="px-3 py-1 border-b border-border-default">{navigation}</div><div className="flex-1 min-h-0"><RunsLedgerPane sessionId={props.sessionId} onShowToast={props.onShowToast} onOpenTab={props.onOpenTab} /></div></div>}
  </div>;
};
