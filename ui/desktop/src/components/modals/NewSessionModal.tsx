import React, { useEffect, useState } from 'react';
import { MessageSquare, ListTodo, X } from 'lucide-react';

interface NewSessionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreateSession: (title: string, pack: 'engineering' | 'research' | 'assistant') => Promise<void>;
  onCreateChat: (pack: 'engineering' | 'research' | 'assistant') => Promise<void>;
  activeWorkbench: 'copilot' | 'engineering' | 'research';
}

export const NewSessionModal: React.FC<NewSessionModalProps> = ({
  isOpen,
  onClose,
  onCreateSession,
  onCreateChat,
  activeWorkbench,
}) => {
  const [title, setTitle] = useState('');
  const [kind, setKind] = useState<'chat' | 'task'>('chat');
  const [creating, setCreating] = useState(false);
  const pack = activeWorkbench === 'copilot' ? 'assistant' : activeWorkbench;

  useEffect(() => {
    if (!isOpen) return;
    setKind('chat');
    setTitle('');
  }, [isOpen]);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (creating || (kind === 'task' && !title.trim())) return;
    setCreating(true);
    try {
      if (kind === 'task') await onCreateSession(title.trim(), pack);
      else await onCreateChat(pack);
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-3 select-none">
      <div className="w-full max-w-md bg-surface-1 border border-border-default rounded-2xl shadow-2xl flex flex-col overflow-hidden max-h-[90vh]">
        <div className="h-12 border-b border-border-muted px-5 flex items-center justify-between shrink-0 bg-surface-1">
          <div className="flex items-center gap-2">
            <MessageSquare className="w-4 h-4 workbench-accent" />
            <span className="text-sm font-semibold text-fg-editor">New in {activeWorkbench === 'engineering' ? 'Coding' : activeWorkbench === 'research' ? 'Research' : 'Copilot'}</span>
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
            <div className="grid grid-cols-2 gap-2" aria-label="Create type">
              <button type="button" aria-pressed={kind === 'chat'} onClick={() => setKind('chat')} className={`rounded-lg border p-3 text-left ${kind === 'chat' ? 'border-border-emphasis bg-surface-2' : 'border-border-default hover:bg-surface-2'}`}>
                <MessageSquare className="mb-2 h-4 w-4" /><span className="block font-medium">Chat session</span><span className="mt-1 block text-[11px] text-fg-muted">Start a conversation. No tracked Task yet.</span>
              </button>
              <button type="button" aria-pressed={kind === 'task'} onClick={() => setKind('task')} className={`rounded-lg border p-3 text-left ${kind === 'task' ? 'border-border-emphasis bg-surface-2' : 'border-border-default hover:bg-surface-2'}`}>
                <ListTodo className="mb-2 h-4 w-4" /><span className="block font-medium">Task</span><span className="mt-1 block text-[11px] text-fg-muted">Track a goal with an attached conversation.</span>
              </button>
            </div>
            {kind === 'task' && <div className="space-y-1.5">
              <label className="font-medium text-fg-editor" htmlFor="new-task-title">Task name</label>
              <input 
                id="new-task-title"
                type="text" 
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                placeholder="e.g. Optimize SQL Query, Cache Token Handler..." 
                className="w-full bg-surface-0 border border-border-default rounded-lg px-3 py-2 text-fg-editor placeholder-fg-subtle focus:outline-none focus:border-border-emphasis"
              />
            </div>}
            <p className="text-[11px] text-fg-subtle">This opens in the current workbench. Select a workspace separately when work needs a repo or worktree.</p>
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
              disabled={creating || (kind === 'task' && !title.trim())}
              className="btn-primary px-3.5 py-1.5 rounded-lg font-medium transition text-xs"
            >
              {creating ? 'Creating…' : kind === 'task' ? 'Create task' : 'Start chat'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
