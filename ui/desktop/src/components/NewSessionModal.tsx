import React, { useState } from 'react';
import { PlusCircle, X } from 'lucide-react';

interface NewSessionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreateSession: (title: string, task: string) => void;
}

export const NewSessionModal: React.FC<NewSessionModalProps> = ({
  isOpen,
  onClose,
  onCreateSession
}) => {
  const [title, setTitle] = useState('');
  const [task, setTask] = useState('');

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) return;
    onCreateSession(title.trim(), task.trim());
    setTitle('');
    setTask('');
  };

  return (
    <div className="fixed inset-0 bg-black/80 backdrop-blur-sm z-50 flex items-center justify-center p-3 select-none">
      <div className="w-full max-w-md bg-surface-card border border-surface-border rounded-2xl shadow-2xl flex flex-col overflow-hidden max-h-[90vh]">
        <div className="h-12 border-b border-surface-border px-5 flex items-center justify-between shrink-0 bg-surface">
          <div className="flex items-center gap-2">
            <PlusCircle className="w-4 h-4 text-brand-blue" />
            <span className="text-sm font-semibold text-white">Create New Session</span>
          </div>
          <button 
            onClick={onClose} 
            className="p-1 hover:bg-surface-elevated rounded-md text-neutral-400 hover:text-white transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="p-5 space-y-4 text-xs overflow-y-auto">
            <div className="space-y-1.5">
              <label className="font-medium text-neutral-300">Session Name</label>
              <input 
                type="text" 
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                placeholder="e.g. Optimize SQL Query, Cache Token Handler..." 
                className="w-full bg-surface border border-surface-border rounded-lg px-3 py-2 text-white placeholder-neutral-500 focus:outline-none focus:border-brand-blue"
                autoFocus
              />
            </div>

            <div className="space-y-1.5">
              <label className="font-medium text-neutral-300">Initial Request / Task</label>
              <textarea 
                rows={3} 
                value={task}
                onChange={(e) => setTask(e.target.value)}
                placeholder="What code or task should this session focus on?" 
                className="w-full bg-surface border border-surface-border rounded-lg p-3 text-white placeholder-neutral-500 focus:outline-none focus:border-brand-blue resize-none"
              />
            </div>
          </div>

          <div className="h-12 border-t border-surface-border px-5 flex items-center justify-end gap-2 bg-surface shrink-0">
            <button 
              type="button"
              onClick={onClose} 
              className="px-3 py-1.5 rounded-lg hover:bg-surface-elevated text-neutral-400 transition text-xs"
            >
              Cancel
            </button>
            <button 
              type="submit" 
              className="px-3.5 py-1.5 rounded-lg bg-brand-blue hover:bg-blue-600 text-white font-medium transition text-xs"
            >
              Start Session
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
