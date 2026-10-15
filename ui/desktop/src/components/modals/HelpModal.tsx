import React from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  CircleHelp,
  X,
  Keyboard,
  BookOpen,
  Activity,
  GitBranch,
  ExternalLink,
  ShieldCheck,
} from 'lucide-react';
import { formatKeyCombo } from '@/lib/utils';

export interface HelpModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const HelpModal: React.FC<HelpModalProps> = ({ isOpen, onClose }) => {
  const shortcuts = [
    { label: 'Home / Sovereign Studio', combo: formatKeyCombo({ ctrlOrCmd: true, key: '1' }) },
    { label: 'Agent Fleet Dashboard', combo: formatKeyCombo({ ctrlOrCmd: true, key: '2' }) },
    { label: 'AI Providers & Models', combo: formatKeyCombo({ ctrlOrCmd: true, key: '3' }) },
    { label: 'Open Settings', combo: formatKeyCombo({ ctrlOrCmd: true, key: ',' }) },
    { label: 'Command Palette', combo: formatKeyCombo({ ctrlOrCmd: true, key: 'K' }) },
    { label: 'Toggle Workspace Sidebar', combo: formatKeyCombo({ ctrlOrCmd: true, key: 'B' }) },
    { label: 'New Chat Session', combo: formatKeyCombo({ ctrlOrCmd: true, key: 'N' }) },
    { label: 'Close Active Tab', combo: formatKeyCombo({ ctrlOrCmd: true, key: 'W' }) },
  ];

  return (
    <AnimatePresence>
      {isOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={onClose}
            className="absolute inset-0 bg-black/60 backdrop-blur-sm"
          />

          <motion.div
            initial={{ scale: 0.95, opacity: 0, y: 10 }}
            animate={{ scale: 1, opacity: 1, y: 0 }}
            exit={{ scale: 0.95, opacity: 0, y: 10 }}
            transition={{ type: 'spring', damping: 25, stiffness: 300 }}
            className="relative flex flex-col h-[600px] w-full max-w-xl overflow-hidden rounded-2xl border border-border-default/60 bg-[#19191c] text-editor-fg shadow-2xl z-10"
          >
            {/* Modal Header */}
            <div className="flex items-center justify-between border-b border-border-default/40 px-6 py-4">
              <div className="flex items-center gap-3">
                <div className="flex size-9 items-center justify-center rounded-xl bg-blue-500/10 text-blue-400">
                  <CircleHelp className="size-5" />
                </div>
                <div>
                  <h2 className="text-base font-semibold text-white tracking-tight">Help & Sovereign Documentation</h2>
                  <p className="text-xs text-fg-muted">Quick shortcuts, local daemon diagnostics, and architecture</p>
                </div>
              </div>
              <button
                type="button"
                onClick={onClose}
                className="rounded-lg p-1.5 text-fg-muted hover:bg-white/5 hover:text-white transition"
              >
                <X className="size-4" />
              </button>
            </div>

            {/* Modal Body */}
            <div className="flex-1 overflow-y-auto p-6 space-y-6">
              {/* Keyboard Shortcuts Section */}
              <div className="space-y-3">
                <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-fg-muted">
                  <Keyboard className="size-3.5" />
                  <span>Essential Keyboard Shortcuts</span>
                </div>
                <div className="grid grid-cols-2 gap-2">
                  {shortcuts.map((s) => (
                    <div
                      key={s.label}
                      className="flex items-center justify-between rounded-xl border border-border-default/40 bg-surface-1/60 px-3 py-2 text-xs"
                    >
                      <span className="text-white/80">{s.label}</span>
                      <kbd className="rounded border border-white/10 bg-white/5 px-1.5 py-0.5 font-mono text-[10px] text-fg-muted">
                        {s.combo}
                      </kbd>
                    </div>
                  ))}
                </div>
              </div>

              {/* Local Daemon Status Section */}
              <div className="space-y-3">
                <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-fg-muted">
                  <Activity className="size-3.5" />
                  <span>Sovereign Daemon Runtime</span>
                </div>
                <div className="rounded-xl border border-border-default/40 bg-surface-1/60 p-4 space-y-2">
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-fg-muted">Custos Daemon IPC</span>
                    <span className="flex items-center gap-1.5 font-mono text-emerald-400">
                      <span className="size-2 rounded-full bg-emerald-400 animate-pulse" />
                      127.0.0.1:58653 (Connected)
                    </span>
                  </div>
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-fg-muted">Architecture Standard</span>
                    <span className="text-white font-mono">Hexagonal · Zero-I/O Domain</span>
                  </div>
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-fg-muted">Security Isolation</span>
                    <span className="flex items-center gap-1 text-workbench-accent">
                      <ShieldCheck className="size-3.5" />
                      Sovereign Sandbox Active
                    </span>
                  </div>
                </div>
              </div>

              {/* Documentation & Links */}
              <div className="space-y-3">
                <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-fg-muted">
                  <BookOpen className="size-3.5" />
                  <span>Resources & External Links</span>
                </div>
                <div className="space-y-2">
                  <a
                    href="https://github.com/stablyai/orca"
                    target="_blank"
                    rel="noreferrer"
                    className="flex items-center justify-between rounded-xl border border-border-default/40 bg-surface-1/60 px-4 py-3 text-xs text-white/80 transition hover:border-border-default hover:bg-surface-1 hover:text-white"
                  >
                    <div className="flex items-center gap-2.5">
                      <GitBranch className="size-4 text-white/60" />
                      <div>
                        <div className="font-semibold text-white">Orca Reference Repository</div>
                        <div className="text-[11px] text-fg-muted">Inspect reference desktop and agent architecture</div>
                      </div>
                    </div>
                    <ExternalLink className="size-3.5 text-fg-muted" />
                  </a>
                </div>
              </div>
            </div>

            {/* Modal Footer */}
            <div className="flex items-center justify-between border-t border-border-default/40 px-6 py-3 bg-[#161619] text-xs text-fg-muted">
              <span>Custos Sovereign Workspace v0.3.0</span>
              <button
                type="button"
                onClick={onClose}
                className="rounded-lg bg-white/10 px-3 py-1 font-medium text-white hover:bg-white/15 transition"
              >
                Done
              </button>
            </div>
          </motion.div>
        </div>
      )}
    </AnimatePresence>
  );
};
