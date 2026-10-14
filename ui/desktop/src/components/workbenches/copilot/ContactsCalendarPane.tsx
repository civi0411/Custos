import React, { useState } from 'react';
import { Calendar, Users, Plus, ShieldCheck, X, AlertCircle } from 'lucide-react';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';

export interface ContactsCalendarPaneProps {
  type: 'contacts' | 'calendar';
  onShowToast?: (msg: string) => void;
  onOpenTab?: (id: OrcaTabId) => void;
}

export interface LocalRosterItem {
  id: string;
  name: string;
  endpoint: string;
  roleOrTime: string;
  status: 'allowed' | 'pending_verification';
}

export const ContactsCalendarPane: React.FC<ContactsCalendarPaneProps> = ({
  type,
  onShowToast,
  onOpenTab,
}) => {
  const isContacts = type === 'contacts';
  const storageKey = `custos.copilot.${type}.roster`;

  const [items, setItems] = useState<LocalRosterItem[]>(() => {
    try {
      const saved = localStorage.getItem(storageKey);
      return saved ? JSON.parse(saved) : [];
    } catch {
      return [];
    }
  });

  const [isModalOpen, setIsModalOpen] = useState(false);
  const [formName, setFormName] = useState('');
  const [formEndpoint, setFormEndpoint] = useState('');
  const [formDetail, setFormDetail] = useState('');
  const [formError, setFormError] = useState<string | null>(null);

  const handleOpenAddModal = () => {
    setFormName('');
    setFormEndpoint('');
    setFormDetail('');
    setFormError(null);
    setIsModalOpen(true);
  };

  const handleSaveItem = (e: React.FormEvent) => {
    e.preventDefault();
    if (!formName.trim()) {
      setFormError(isContacts ? 'Contact name is required' : 'Event title is required');
      return;
    }
    if (!formEndpoint.trim()) {
      setFormError(isContacts ? 'Email or endpoint handle is required' : 'Date / time slot is required');
      return;
    }

    const newItem: LocalRosterItem = {
      id: crypto.randomUUID(),
      name: formName.trim(),
      endpoint: formEndpoint.trim(),
      roleOrTime: formDetail.trim() || (isContacts ? 'Recipient' : 'Planned Milestone'),
      status: 'allowed',
    };

    const next = [newItem, ...items];
    setItems(next);
    try {
      localStorage.setItem(storageKey, JSON.stringify(next));
    } catch {}

    setIsModalOpen(false);
    onShowToast?.(`Added ${newItem.name} to governed roster`);
  };

  const handleRemoveItem = (id: string, name: string) => {
    const next = items.filter((item) => item.id !== id);
    setItems(next);
    try {
      localStorage.setItem(storageKey, JSON.stringify(next));
    } catch {}
    onShowToast?.(`Removed ${name}`);
  };

  return (
    <div className="flex h-full flex-col bg-canvas text-editor-fg">
      {/* Header Bar */}
      <div className="flex h-10 shrink-0 items-center justify-between border-b border-border-muted px-4 bg-surface-1/50">
        <div className="flex items-center gap-2">
          {isContacts ? (
            <Users className="h-4 w-4 text-fg-muted shrink-0" />
          ) : (
            <Calendar className="h-4 w-4 text-fg-muted shrink-0" />
          )}
          <span className="text-xs font-semibold">{isContacts ? 'Governed Contact Roster' : 'Local Schedule & Milestones'}</span>
          <span className="rounded bg-surface-2 px-1.5 py-0.2 text-[10px] font-mono text-fg-subtle border border-border-subtle">
            {items.length} entries
          </span>
        </div>

        <div className="flex items-center gap-2">
          {onOpenTab && (
            <button
              type="button"
              onClick={() => onOpenTab('outbox')}
              className="text-[11px] text-fg-muted hover:text-fg-editor underline"
              title="Inspect governed outbox dispatches"
            >
              Outbox Ledger →
            </button>
          )}
          <button
            type="button"
            onClick={handleOpenAddModal}
            className="flex items-center gap-1.5 rounded-md bg-surface-2 hover:bg-surface-3 border border-border-default px-2.5 py-1 text-xs font-medium text-fg-editor transition cursor-pointer"
          >
            <Plus className="h-3.5 w-3.5 text-fg-subtle" />
            <span>Add {isContacts ? 'Contact' : 'Milestone'}</span>
          </button>
        </div>
      </div>

      {/* Governed Boundary Notice Banner */}
      <div className="shrink-0 border-b border-border-subtle bg-surface-1/30 px-4 py-2 flex items-center justify-between text-[11px] text-fg-muted">
        <div className="flex items-center gap-2">
          <ShieldCheck className="h-3.5 w-3.5 text-emerald-400 shrink-0" />
          <span>
            {isContacts
              ? 'Custos Assistant Gate: outbound notifications require user authorization in the Outbox.'
              : 'Daemon calendar scheduling connector: proposed events remain candidate drafts until confirmed.'}
          </span>
        </div>
      </div>

      {/* List or Empty State */}
      <div className="flex-1 overflow-y-auto p-4 space-y-2">
        {items.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center text-center text-fg-subtle p-8 select-none">
            {isContacts ? (
              <Users className="w-10 h-10 mb-3 opacity-30 text-fg-muted" />
            ) : (
              <Calendar className="w-10 h-10 mb-3 opacity-30 text-fg-muted" />
            )}
            <p className="text-xs font-semibold text-fg-editor">
              {isContacts ? 'No communication recipients registered' : 'No milestone schedules configured'}
            </p>
            <p className="text-[11px] text-fg-subtle mt-1.5 max-w-sm leading-relaxed">
              {isContacts
                ? 'Register allowed team email addresses or communication endpoints. When the copilot drafts an outreach, it references this governed roster.'
                : 'Define deadline targets, benchmark evaluations, or review checkpoints. External CalDAV sync requires configured connector.'}
            </p>
            <button
              type="button"
              onClick={handleOpenAddModal}
              className="mt-4 inline-flex items-center gap-1.5 rounded-lg border border-border-default bg-surface-2 px-3 py-1.5 text-xs font-medium text-fg-editor hover:bg-surface-3 transition"
            >
              <Plus className="h-3.5 w-3.5" />
              <span>Register First {isContacts ? 'Contact' : 'Milestone'}</span>
            </button>
          </div>
        ) : (
          items.map((item) => (
            <div
              key={item.id}
              className="flex items-center justify-between rounded-xl border border-border-subtle bg-surface-1 p-3 text-xs transition hover:border-border-default group"
            >
              <div className="min-w-0 flex-1 pr-3">
                <div className="flex items-center gap-2">
                  <span className="font-semibold text-fg-editor">{item.name}</span>
                  <span className="rounded bg-surface-2 px-1.5 py-0.2 text-[9px] font-mono text-fg-subtle border border-border-subtle">
                    {item.roleOrTime}
                  </span>
                </div>
                <div className="text-[11px] text-fg-muted font-mono mt-1 truncate">
                  {item.endpoint}
                </div>
              </div>
              <div className="flex items-center gap-2 shrink-0">
                <span className="flex items-center gap-1 text-[10px] text-emerald-400 font-mono">
                  <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
                  <span>Allowed</span>
                </span>
                <button
                  type="button"
                  onClick={() => handleRemoveItem(item.id, item.name)}
                  className="opacity-0 group-hover:opacity-100 text-fg-subtle hover:text-rose-400 p-1 rounded hover:bg-surface-2 transition text-xs"
                  title="Remove from roster"
                >
                  <X className="h-3.5 w-3.5" />
                </button>
              </div>
            </div>
          ))
        )}
      </div>

      {/* Real In-Pane Modal (No window.prompt mock) */}
      {isModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4">
          <div className="w-full max-w-md rounded-2xl border border-border-default bg-canvas p-5 shadow-2xl space-y-4">
            <div className="flex items-center justify-between border-b border-border-muted pb-3">
              <h3 className="text-sm font-semibold text-fg-editor">
                {isContacts ? 'Register Authorized Contact' : 'Schedule Milestone Event'}
              </h3>
              <button
                type="button"
                onClick={() => setIsModalOpen(false)}
                className="text-fg-subtle hover:text-fg-editor p-1 rounded hover:bg-surface-2"
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            {formError && (
              <div className="flex items-center gap-2 rounded-lg bg-rose-500/10 border border-rose-500/20 px-3 py-2 text-xs text-rose-300">
                <AlertCircle className="h-3.5 w-3.5 shrink-0" />
                <span>{formError}</span>
              </div>
            )}

            <form onSubmit={handleSaveItem} className="space-y-3">
              <div>
                <label className="block text-xs font-medium text-fg-muted mb-1">
                  {isContacts ? 'Full Name or Organization' : 'Milestone Title'}
                </label>
                <input
                  type="text"
                  autoFocus
                  required
                  value={formName}
                  onChange={(e) => setFormName(e.target.value)}
                  placeholder={isContacts ? 'e.g. Alice Johnson' : 'e.g. V1 Release Gate Verification'}
                  className="w-full rounded-lg border border-border-default bg-surface-1 px-3 py-2 text-xs text-fg-editor focus:border-border-focus focus:outline-none"
                />
              </div>

              <div>
                <label className="block text-xs font-medium text-fg-muted mb-1">
                  {isContacts ? 'Destination Email or Matrix Endpoint' : 'Target Date & Time / Slot'}
                </label>
                <input
                  type="text"
                  required
                  value={formEndpoint}
                  onChange={(e) => setFormEndpoint(e.target.value)}
                  placeholder={isContacts ? 'alice@company.internal' : '2026-10-20 14:00 UTC'}
                  className="w-full rounded-lg border border-border-default bg-surface-1 px-3 py-2 text-xs text-fg-editor focus:border-border-focus focus:outline-none font-mono"
                />
              </div>

              <div>
                <label className="block text-xs font-medium text-fg-muted mb-1">
                  {isContacts ? 'Role / Context (Optional)' : 'Category / Description (Optional)'}
                </label>
                <input
                  type="text"
                  value={formDetail}
                  onChange={(e) => setFormDetail(e.target.value)}
                  placeholder={isContacts ? 'Lead Reviewer' : 'Architecture Review'}
                  className="w-full rounded-lg border border-border-default bg-surface-1 px-3 py-2 text-xs text-fg-editor focus:border-border-focus focus:outline-none"
                />
              </div>

              <div className="flex items-center justify-end gap-2 pt-2 border-t border-border-muted">
                <button
                  type="button"
                  onClick={() => setIsModalOpen(false)}
                  className="rounded-lg border border-border-default bg-surface-1 px-3 py-1.5 text-xs font-medium text-fg-muted hover:bg-surface-2 transition"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="rounded-lg bg-emerald-600 hover:bg-emerald-500 px-4 py-1.5 text-xs font-medium text-white transition shadow-sm"
                >
                  Save Entry
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
