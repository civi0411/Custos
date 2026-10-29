import React, { useState } from 'react';
import { X } from 'lucide-react';
import { NativeSelect } from '@/components/common/NativeSelect';
import type { CreateHeadlessJobParams } from '@/types/domain';

export interface CreateJobModalProps {
  canCreate: boolean;
  onSubmit: (params: CreateHeadlessJobParams) => Promise<void>;
  onClose: () => void;
}

export const CreateJobModal: React.FC<CreateJobModalProps> = ({
  canCreate,
  onSubmit,
  onClose,
}) => {
  const [name, setName] = useState('');
  const [command, setCommand] = useState('cargo check --workspace');
  const [targetType, setTargetType] = useState('local_process');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim() || !command.trim()) return;

    await onSubmit({
      name: name.trim(),
      spec: {
        target_type: targetType,
        command_or_script: command.trim(),
        timeout_secs: 600,
        env: {},
        required_evidence: ['EXECUTION-EVIDENCE-01'],
      },
      trigger: { type: 'manual' },
    });
  };

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

        <form onSubmit={handleSubmit} className="space-y-3">
          <div>
            <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">Job Name</label>
            <input
              type="text"
              placeholder="e.g. Nightly Benchmark Sweep"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none"
              required
            />
          </div>

          <div>
            <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">Target Execution Type</label>
            <NativeSelect
              value={targetType}
              onChange={(e) => setTargetType(e.target.value)}
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
              value={command}
              onChange={(e) => setCommand(e.target.value)}
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
              className="px-3.5 py-1.5 rounded-md bg-white hover:bg-white/90 text-black text-xs font-semibold shadow-sm disabled:opacity-40"
            >
              Save Job
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
