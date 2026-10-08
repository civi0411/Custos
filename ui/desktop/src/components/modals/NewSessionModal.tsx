import React, { useState } from 'react';
import { PlusCircle, X } from 'lucide-react';

interface NewSessionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreateSession: (title: string, pack: 'engineering' | 'research' | 'assistant') => void;
}

export const NewSessionModal: React.FC<NewSessionModalProps> = ({
  isOpen,
  onClose,
  onCreateSession
}) => {
  const [title, setTitle] = useState('');
  const [pack, setPack] = useState<'engineering' | 'research' | 'assistant'>('engineering');

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) return;
    onCreateSession(title.trim(), pack);
    setTitle('');
  };

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-3 select-none">
      <div className="w-full max-w-md bg-surface-1 border border-border-default rounded-2xl shadow-2xl flex flex-col overflow-hidden max-h-[90vh]">
        <div className="h-12 border-b border-border-muted px-5 flex items-center justify-between shrink-0 bg-surface-1">
          <div className="flex items-center gap-2">
            <PlusCircle className="w-4 h-4 workbench-accent" />
            <span className="text-sm font-semibold text-fg-editor">Create Task</span>
          </div>
          <button 
            onClick={onClose} 
            className="p-1 hover:bg-surface-2 rounded-md text-fg-muted hover:text-fg-editor transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="p-5 space-y-4 text-xs overflow-y-auto">
            <div className="space-y-1.5">
              <label className="font-medium text-fg-editor">Task name</label>
              <input 
                type="text" 
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                placeholder="e.g. Optimize SQL Query, Cache Token Handler..." 
                className="w-full bg-surface-0 border border-border-default rounded-lg px-3 py-2 text-fg-editor placeholder-fg-subtle focus:outline-none focus:border-border-emphasis"
                autoFocus
              />
            </div>

            <div className="space-y-1.5">
              <label className="font-medium text-fg-editor" htmlFor="task-pack">Workbench</label>
              <select
                id="task-pack"
                value={pack}
                onChange={(event) => setPack(event.target.value as typeof pack)}
                className="w-full rounded-lg border border-border-default bg-surface-0 px-3 py-2 text-fg-editor outline-none"
              >
                <option value="engineering">Coding</option>
                <option value="research">Research</option>
                <option value="assistant">Assistant</option>
              </select>
              <p className="text-[11px] text-fg-subtle">Write the first instruction in the conversation after creating the task.</p>
            </div>
          </div>

          <div className="h-12 border-t border-border-muted px-5 flex items-center justify-end gap-2 bg-surface-1 shrink-0">
            <button 
              type="button"
              onClick={onClose} 
              className="px-3 py-1.5 rounded-lg hover:bg-surface-2 text-fg-muted hover:text-fg-editor transition text-xs"
            >
              Cancel
            </button>
            <button 
              type="submit" 
              className="workbench-primary-action px-3.5 py-1.5 rounded-lg font-medium transition text-xs"
            >
              Create Task
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
