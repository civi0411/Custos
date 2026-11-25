import React, { useState } from 'react';
import { Calendar, Users, Plus, ShieldCheck, X, AlertCircle, Search, ChevronLeft, ChevronRight, Pencil } from 'lucide-react';
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

export function rosterDateKey(value: string): string | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})(?:$|T|\s)/.exec(value);
  if (!match) return null;
  const year = Number(match[1]), month = Number(match[2]), day = Number(match[3]);
  const date = new Date(year, month - 1, day);
  return date.getFullYear() === year && date.getMonth() === month - 1 && date.getDate() === day ? `${match[1]}-${match[2]}-${match[3]}` : null;
}
const localDateKey = (date: Date) => `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;

export const ContactsCalendarPane: React.FC<ContactsCalendarPaneProps> = ({
  type,
  onShowToast,
  onOpenTab: _onOpenTab,
}) => {
  const isContacts = type === 'contacts';
  const storageKey = `custos.copilot.${type}.roster`;

  const [items, setItems] = useState<LocalRosterItem[]>(() => {
    try {
      const saved = localStorage.getItem(storageKey);
      const parsed: unknown = saved ? JSON.parse(saved) : [];
      const valid = Array.isArray(parsed)
        ? parsed.filter(
            (item): item is LocalRosterItem =>
              !!item &&
              typeof item.id === 'string' &&
              typeof item.name === 'string' &&
              typeof item.endpoint === 'string' &&
              typeof item.roleOrTime === 'string'
          )
        : [];
      return valid;
    } catch {
      return [];
    }
  });

  const [query, setQuery] = useState('');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [calendarView, setCalendarView] = useState<'month' | 'agenda'>('month');
  const [month, setMonth] = useState(() => new Date(new Date().getFullYear(), new Date().getMonth(), 1));
  const [selectedDay, setSelectedDay] = useState<string | null>(null);
  const filteredItems = items.filter(item => `${item.name} ${item.endpoint} ${item.roleOrTime}`.toLowerCase().includes(query.toLowerCase()));
  const visibleItems = !isContacts && calendarView === 'month' && selectedDay ? filteredItems.filter(item => rosterDateKey(item.endpoint) === selectedDay) : filteredItems;
  const firstWeekday = (month.getDay() + 6) % 7;
  const monthDays = new Date(month.getFullYear(), month.getMonth() + 1, 0).getDate();
  const shiftMonth = (delta: number) => { setMonth(new Date(month.getFullYear(), month.getMonth() + delta, 1)); setSelectedDay(null); };
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [formName, setFormName] = useState('');
  const [formEndpoint, setFormEndpoint] = useState('');
  const [formDetail, setFormDetail] = useState('');
  const [formError, setFormError] = useState<string | null>(null);

  const handleOpenAddModal = () => {
    setEditingId(null);
    setFormName('');
    setFormEndpoint(!isContacts && selectedDay ? `${selectedDay}T09:00` : '');
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
      id: editingId || crypto.randomUUID(),
      name: formName.trim(),
      endpoint: formEndpoint.trim(),
      roleOrTime: formDetail.trim() || (isContacts ? 'Recipient' : 'Planned Milestone'),
      status: 'pending_verification',
    };

    if (!isContacts && !rosterDateKey(formEndpoint)) { setFormError('Choose a valid date and time.'); return; }
    const next = editingId ? items.map(item => item.id === editingId ? newItem : item) : [newItem, ...items];
    try { localStorage.setItem(storageKey, JSON.stringify(next)); }
    catch { setFormError('Could not save locally. Your changes are still in this form.'); return; }
    setItems(next);
    setIsModalOpen(false);
    onShowToast?.(`Saved ${newItem.name} locally`);
  };

  const handleRemoveItem = (id: string, name: string) => {
    const next = items.filter((item) => item.id !== id);
    try { localStorage.setItem(storageKey, JSON.stringify(next)); }
    catch { onShowToast?.('Could not remove the local entry.'); return; }
    setItems(next);
    onShowToast?.(`Removed ${name}`);
  };

  return (
    <div className="flex h-full flex-col bg-canvas text-editor-fg">
      {/* Header Bar */}
      <div className="wb-pane-toolbar flex h-9 shrink-0 items-center justify-between border-b border-border-muted px-3 bg-surface-1/50">
        <div className="flex items-center gap-2">
          {isContacts ? (
            <Users className="h-4 w-4 text-fg-muted shrink-0" />
          ) : (
            <Calendar className="h-4 w-4 text-fg-muted shrink-0" />
          )}
          <span className="text-xs font-semibold">{isContacts ? 'Contacts' : 'Calendar'}</span>
          <span className="rounded bg-surface-2 px-1.5 py-0.2 text-[10px] font-mono text-fg-subtle border border-border-subtle">
            {items.length} entries
          </span>
        </div>

        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={handleOpenAddModal}
            className="flex items-center gap-1.5 rounded-md bg-surface-2 hover:bg-surface-3 border border-border-default px-2 py-0.5 text-xs font-medium text-fg-editor transition cursor-pointer"
          >
            <Plus className="h-3.5 w-3.5 text-fg-subtle" />
            <span>Add {isContacts ? 'Contact' : 'Milestone'}</span>
          </button>
        </div>
      </div>

      {/* Governed Boundary Notice Banner */}
      <div className="shrink-0 border-b border-border-subtle bg-surface-1/30 px-3 py-1 flex items-center justify-between text-[10px] text-fg-muted">
        <div className="flex items-center gap-1.5">
          <ShieldCheck className="h-3 w-3 text-emerald-400 shrink-0" />
          <span>
            {isContacts
              ? 'Local contact notes · No external auth'
              : 'Local milestones · No external sync'}
          </span>
        </div>
      </div>

      <div className="shrink-0 flex flex-wrap gap-2 px-3 py-2 border-b border-border-default">
        <label className="flex items-center gap-2 flex-1 min-w-32 rounded border border-border-default bg-surface-1 px-2 py-1.5"><Search size={13} className="text-fg-muted" /><input aria-label={isContacts ? 'Search contacts' : 'Search milestones'} value={query} onChange={event => setQuery(event.target.value)} placeholder={isContacts ? 'Search names, endpoints, roles…' : 'Search titles and categories…'} className="bg-transparent min-w-0 w-full text-xs outline-none" /></label>
        {!isContacts && <div className="flex gap-1"><button type="button" className="workbench-tool" aria-pressed={calendarView === 'month'} onClick={() => setCalendarView('month')}>Month</button><button type="button" className="workbench-tool" aria-pressed={calendarView === 'agenda'} onClick={() => setCalendarView('agenda')}>Agenda</button></div>}
      </div>
      {!isContacts && calendarView === 'month' && <section className="wb-calendar-section p-3 border-b border-border-default overflow-auto" aria-label="Milestone calendar">
        <div className="flex items-center justify-between mb-2"><strong className="text-xs">{month.toLocaleDateString(undefined, { month: 'long', year: 'numeric' })}</strong><div className="flex gap-1"><button type="button" className="canvas-icon" aria-label="Previous month" onClick={() => shiftMonth(-1)}><ChevronLeft size={14} /></button><button type="button" className="workbench-tool" onClick={() => { setMonth(new Date(new Date().getFullYear(), new Date().getMonth(), 1)); setSelectedDay(localDateKey(new Date())); }}>Today</button><button type="button" className="canvas-icon" aria-label="Next month" onClick={() => shiftMonth(1)}><ChevronRight size={14} /></button></div></div>
        <div className="wb-calendar-grid">{['Mon','Tue','Wed','Thu','Fri','Sat','Sun'].map(day => <span key={day} className="text-center text-[10px] text-fg-muted py-1">{day}</span>)}
          {Array.from({ length: firstWeekday }, (_, index) => <span key={`blank-${index}`} />)}
          {Array.from({ length: monthDays }, (_, index) => { const date = new Date(month.getFullYear(), month.getMonth(), index + 1); const key = localDateKey(date); const count = filteredItems.filter(item => rosterDateKey(item.endpoint) === key).length; return <button type="button" key={key} className="wb-calendar-day" aria-label={`${key}${count ? `, ${count} milestones` : ''}`} aria-pressed={selectedDay === key} aria-current={key === localDateKey(new Date()) ? 'date' : undefined} onClick={() => setSelectedDay(selectedDay === key ? null : key)}><span>{index + 1}</span>{count > 0 && <span className="text-[9px] text-accent">{count} planned</span>}</button>; })}
        </div>
        {selectedDay && <button type="button" className="workbench-tool mt-2" onClick={() => setSelectedDay(null)}>Showing {selectedDay} · Show all milestones</button>}
      </section>}

      {/* List or Empty State */}
      <div className="flex-1 overflow-y-auto p-4 space-y-2">
        {visibleItems.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center text-center text-fg-subtle p-6 select-none">
            {isContacts ? (
              <Users className="w-8 h-8 mb-2 opacity-30 text-fg-muted" />
            ) : (
              <Calendar className="w-8 h-8 mb-2 opacity-30 text-fg-muted" />
            )}
            <p className="text-xs font-semibold text-fg-editor">
              {items.length > 0 ? 'No matching entries' : isContacts ? 'No contacts yet' : 'No milestones yet'}
            </p>
            <button
              type="button"
              onClick={handleOpenAddModal}
              className="mt-3 inline-flex items-center gap-1.5 rounded-lg border border-border-default bg-surface-2 px-3 py-1.5 text-xs font-medium text-fg-editor hover:bg-surface-3 transition"
            >
              <Plus className="h-3.5 w-3.5" />
              <span>Register First {isContacts ? 'Contact' : 'Milestone'}</span>
            </button>
          </div>
        ) : (
          visibleItems.map((item) => (
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
                <span className="flex items-center gap-1 text-[10px] text-white/70 font-mono">
                  <span className="h-1.5 w-1.5 rounded-full bg-white/60" />
                  <span>{isContacts ? 'Local entry' : 'Planned'}</span>
                </span>
                <button type="button" className="canvas-icon" aria-label={`Edit ${item.name}`} onClick={() => { setEditingId(item.id); setFormName(item.name); setFormEndpoint(item.endpoint); setFormDetail(item.roleOrTime); setFormError(null); setIsModalOpen(true); }}><Pencil size={13} /></button>
                <button
                  type="button"
                  onClick={() => handleRemoveItem(item.id, item.name)}
                  className="text-fg-subtle hover:text-white/80 p-1 rounded hover:bg-surface-2 transition text-xs"
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
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4" role="dialog" aria-modal="true" aria-label={isContacts ? 'Contact editor' : 'Milestone editor'} onKeyDown={event => { if (event.key === 'Escape') setIsModalOpen(false); }}>
          <div className="w-full max-w-md rounded-2xl border border-border-default bg-canvas p-5 shadow-2xl space-y-4">
            <div className="flex items-center justify-between border-b border-border-muted pb-3">
              <h3 className="text-sm font-semibold text-fg-editor">
                {editingId ? 'Edit entry' : isContacts ? 'New contact' : 'New milestone'}
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
              <div className="flex items-center gap-2 rounded-lg bg-white/[0.04] border border-white/10 px-3 py-2 text-xs text-white/70">
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
                  aria-label={isContacts ? 'Contact name' : 'Milestone title'}
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
                  type={isContacts ? 'text' : 'datetime-local'}
                  required
                  aria-label={isContacts ? 'Contact endpoint' : 'Milestone date and time'}
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
                  aria-label={isContacts ? 'Contact context' : 'Milestone category'}
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
                  className="rounded-lg bg-white/10 hover:bg-white/15 px-4 py-1.5 text-xs font-medium text-white transition border border-white/10 shadow-sm"
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
