import React from 'react';
import { X } from 'lucide-react';
import { NativeSelect } from '@/components/common/NativeSelect';

export interface CreateJobModalProps {
  isOpen: boolean;
  canCreate: boolean;
  newJobName: string;
  newJobTargetType: string;
  newJobCommand: string;
  onClose: () => void;
  onNameChange: (v: string) => void;
  onTargetTypeChange: (v: string) => void;
  onCommandChange: (v: string) => void;
  onSubmit: (e: React.FormEvent) => void;
}

export const CreateJobModal: React.FC<CreateJobModalProps> = ({
  isOpen,
  canCreate,
  newJobName,
  newJobTargetType,
  newJobCommand,
  onClose,
  onNameChange,
  onTargetTypeChange,
  onCommandChange,
  onSubmit,
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-xs flex items-center justify-center p-4 z-50">
      <div className="bg-[var(--color-surface-1)] border border-[var(--color-border-default)] rounded-xl w-full max-w-md p-5 space-y-4 shadow-xl">
        <div className="flex items-center justify-between border-b border-[var(--color-border-muted)] pb-2.5">
          <span className="font-semibold text-[var(--color-editor-fg)] text-xs">Create Headless Job</span>
          <button
            onClick={onClose}
            className="text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
          >
            <X size={14} />
          </button>
        </div>

        <form onSubmit={onSubmit} className="space-y-3">
          <div>
            <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">Job Name</label>
            <input
              type="text"
              placeholder="e.g. Nightly Benchmark Sweep"
              value={newJobName}
              onChange={(e) => onNameChange(e.target.value)}
              className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none"
              required
            />
          </div>

          <div>
            <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">Target Execution Type</label>
            <NativeSelect
              value={newJobTargetType}
              onChange={(e) => onTargetTypeChange(e.target.value)}
              className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none"
            >
              <option value="local_process">local_process (Host Process)</option>
              <option value="remote_ssh">remote_ssh (Fleet Node)</option>
              <option value="browser_session">browser_session (Headless Browser)</option>
            </NativeSelect>
          </div>

          <div>
            <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">Command or Script</label>
            <textarea
              rows={3}
              value={newJobCommand}
              onChange={(e) => onCommandChange(e.target.value)}
              className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none font-mono"
              required
            />
          </div>

          <div className="flex justify-end gap-2 pt-2 border-t border-[var(--color-border-muted)]">
            <button
              type="button"
              onClick={onClose}
              className="px-3 py-1.5 rounded-md bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-0)] text-[var(--color-editor-fg)] text-xs border border-[var(--color-border-default)]"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!canCreate}
              className="px-3.5 py-1.5 rounded-md bg-white hover:bg-white/90 text-black text-xs font-semibold shadow-sm"
            >
              Save Job
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
