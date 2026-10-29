import React, { useState } from 'react';
import { X } from 'lucide-react';
import type { RegisterHostParams } from '@/types/domain';

export interface RegisterHostModalProps {
  canRegister: boolean;
  onSubmit: (params: RegisterHostParams) => Promise<void>;
  onClose: () => void;
}

export const RegisterHostModal: React.FC<RegisterHostModalProps> = ({
  canRegister,
  onSubmit,
  onClose,
}) => {
  const [name, setName] = useState('');
  const [address, setAddress] = useState('');
  const [port, setPort] = useState(22);
  const [user, setUser] = useState('');
  const [keyPath, setKeyPath] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim() || !address.trim() || !user.trim()) return;

    await onSubmit({
      name: name.trim(),
      host: address.trim(),
      port,
      user: user.trim(),
      private_key_path: keyPath.trim() || undefined,
      labels: { managed_by: 'custos-daemon' },
    });
  };

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-xs flex items-center justify-center p-4 z-50">
      <div className="bg-[var(--color-surface-1)] border border-[var(--color-border-default)] rounded-xl w-full max-w-md p-5 space-y-4 shadow-xl">
        <div className="flex items-center justify-between border-b border-[var(--color-border-muted)] pb-2.5">
          <span className="font-semibold text-[var(--color-editor-fg)] text-xs">Register Remote SSH Host</span>
          <button
            onClick={onClose}
            className="text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
          >
            <X size={14} />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-3">
          <div>
            <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">Node Name</label>
            <input
              type="text"
              placeholder="e.g. Cluster GPU Node 01"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none"
              required
            />
          </div>

          <div className="grid grid-cols-3 gap-2">
            <div className="col-span-2">
              <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">Host IP / FQDN</label>
              <input
                type="text"
                value={address}
                onChange={(e) => setAddress(e.target.value)}
                placeholder="192.168.1.50 or host.internal"
                className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none"
                required
              />
            </div>
            <div>
              <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">Port</label>
              <input
                type="number"
                value={port}
                onChange={(e) => setPort(parseInt(e.target.value) || 22)}
                className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none"
              />
            </div>
          </div>

          <div>
            <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">SSH Username</label>
            <input
              type="text"
              placeholder="e.g. ubuntu, root, dev"
              value={user}
              onChange={(e) => setUser(e.target.value)}
              className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none"
              required
            />
          </div>

          <div>
            <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">Private Key Path (optional)</label>
            <input
              type="text"
              placeholder="~/.ssh/id_ed25519"
              value={keyPath}
              onChange={(e) => setKeyPath(e.target.value)}
              className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none font-mono"
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
              disabled={!canRegister}
              className="px-3.5 py-1.5 rounded-md bg-white hover:bg-white/90 text-black text-xs font-semibold shadow-sm disabled:opacity-40"
            >
              Register Node
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
