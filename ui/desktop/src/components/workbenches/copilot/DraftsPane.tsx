import React, { useState, useEffect, useCallback, useRef } from 'react';
import {
  FileText,
  Lock,
  CheckCircle2,
  Trash2,
  Copy,
  Plus,
  MoreVertical,
  Edit3,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { AssistantDraft } from '@/types/domain';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';

// Unsaved buffers survive tab/lens navigation for the lifetime of the desktop session.
const draftEdits = new Map<string, { title: string; content: string }>();

export interface DraftsPaneProps {
  taskId?: string;
  sessionId?: string;
  onShowToast?: (msg: string) => void;
  onOpenTab?: (id: OrcaTabId) => void;
}

export const DraftsPane: React.FC<DraftsPaneProps> = ({
  taskId,
  sessionId,
  onShowToast,
}) => {
  const [drafts, setDrafts] = useState<AssistantDraft[]>([]);
  const [selectedDraftId, setSelectedDraftId] = useState<string | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [editingContent, setEditingContent] = useState<string>('');
  const [editingTitle, setEditingTitle] = useState<string>('');
  const [saving, setSaving] = useState<boolean>(false);
  const [showSecondaryMenu, setShowSecondaryMenu] = useState<boolean>(false);

  const loadEpoch = useRef(0);
  const loadDrafts = useCallback(async () => {
    const epoch = ++loadEpoch.current;
    setLoading(true);
    setLoadError(null);
    try {
      const list = sessionId ? await daemonClient.listDrafts(sessionId) : [];
      if (epoch !== loadEpoch.current) return;
      const effective = list && list.length > 0 ? list : [];
      setDrafts(effective);
      setSelectedDraftId((prev) => (prev && effective.some((d) => d.id === prev) ? prev : effective[0]?.id || null));
    } catch (err: any) {
      if (epoch === loadEpoch.current) {
        setLoadError(err instanceof Error ? err.message : String(err));
        setDrafts([]);
        setSelectedDraftId(null);
      }
    } finally {
      if (epoch === loadEpoch.current) setLoading(false);
    }
  }, [sessionId]);

  useEffect(() => {
    void loadDrafts();
    return () => { loadEpoch.current++; };
  }, [loadDrafts]);

  const activeDraft = drafts.find((d) => d.id === selectedDraftId) || drafts[0];

  useEffect(() => {
    if (activeDraft) {
      const edited = draftEdits.get(activeDraft.id);
      setEditingTitle(edited?.title ?? activeDraft.title);
      setEditingContent(edited?.content ?? activeDraft.content);
    }
  }, [activeDraft]);

  const isLocked = Boolean(activeDraft?.payload_lock || activeDraft?.payloadLock);
  const isModified = !!activeDraft && (editingTitle !== activeDraft.title || editingContent !== activeDraft.content);
  useEffect(() => {
    if (!isModified) return;
    const warn = (event: BeforeUnloadEvent) => { event.preventDefault(); };
    window.addEventListener('beforeunload', warn);
    return () => window.removeEventListener('beforeunload', warn);
  }, [isModified]);

  // Local review is separate from outbound authority and queue creation.
  const handlePrimaryAction = async () => {
    if (!activeDraft || saving || loading) return;
    setSaving(true);
    try {
      const updated = await daemonClient.saveDraft({
        ...activeDraft,
        title: editingTitle,
        content: editingContent,
        status: 'approved',
        payload_lock: 'locked',
      });
      setDrafts((prev) => prev.map((d) => (d.id === updated.id ? updated : d)));

      draftEdits.delete(activeDraft.id);
      onShowToast?.(`Draft "${updated.title}" reviewed and locked. No outbound action was queued.`);
    } catch (err: any) {
      onShowToast?.(`Approval failed: ${err?.message || err}`);
    } finally {
      setSaving(false);
    }
  };

  const handleSaveDraft = async () => {
    if (!activeDraft || saving || loading) return;
    setShowSecondaryMenu(false);
    setSaving(true);
    try {
      const updated = await daemonClient.saveDraft({
        ...activeDraft,
        title: editingTitle,
        content: editingContent,
      });
      setDrafts((prev) => prev.map((d) => (d.id === updated.id ? updated : d)));
      draftEdits.delete(activeDraft.id);
      onShowToast?.('Draft saved.');
    } catch (err: any) {
      onShowToast?.(`Save failed: ${err?.message || err}`);
    } finally {
      setSaving(false);
    }
  };

  const handleCreateDraft = async () => {
    if (!sessionId || saving || loading) return;
    const epoch = loadEpoch.current;
    setSaving(true);
    setShowSecondaryMenu(false);
    try {
      const created = await daemonClient.saveDraft({
        title: 'New Communication Draft',
        content: '',
        kind: 'email',
        session_id: sessionId,
        task_id: taskId,
      });
      if (epoch !== loadEpoch.current) return;
      setDrafts((prev) => [created, ...prev]);
      setSelectedDraftId(created.id);
      onShowToast?.('Created new draft.');
    } catch (err: any) {
      onShowToast?.(`Create failed: ${err?.message || err}`);
    } finally { setSaving(false); }
  };

  const handleDeleteDraft = async () => {
    if (!activeDraft || saving || loading) return;
    setShowSecondaryMenu(false);
    if (!confirm(`Delete draft "${activeDraft.title}"?`)) return;
    setSaving(true);
    try {
      await daemonClient.deleteDraft(activeDraft.id);
      draftEdits.delete(activeDraft.id);
      setDrafts((prev) => prev.filter((d) => d.id !== activeDraft.id));
      setSelectedDraftId(null);
      onShowToast?.('Draft deleted.');
    } catch (err: any) {
      onShowToast?.(`Delete failed: ${err?.message || err}`);
    } finally { setSaving(false); }
  };

  const handleCopyContent = () => {
    setShowSecondaryMenu(false);
    if (editingContent) {
      navigator.clipboard.writeText(editingContent);
      onShowToast?.('Copied draft content to clipboard.');
    }
  };

  return (
    <div className="flex h-full flex-col bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
      {/* Header */}
      <div className="wb-pane-toolbar flex h-9 shrink-0 items-center justify-between border-b border-[var(--color-border-subtle)] px-3 bg-[var(--color-surface-1)]/50">
        <div className="flex items-center gap-2">
          <FileText className="h-4 w-4 text-[var(--color-fg-muted)] shrink-0" />
          <span className="text-xs font-semibold">Drafts</span>
          <span className="rounded bg-[var(--color-surface-2)] px-1.5 py-0.5 text-[10px] font-mono text-[var(--color-fg-subtle)]">
            {drafts.length} items
          </span>
        </div>

        <div className="flex items-center gap-2">
          <button type="button" className="canvas-open-button" disabled={!sessionId || saving || loading} onClick={handleCreateDraft}><Plus size={12} />New draft</button>
          {activeDraft && <button type="button" disabled={!isModified || saving || isLocked} onClick={handleSaveDraft} className="canvas-open-button disabled:opacity-40">{saving ? 'Saving…' : isModified ? 'Save changes' : 'Saved'}</button>}
          {/* Local draft review; outbound action is a separate governed flow. */}
          <button
            type="button"
            disabled={!activeDraft || saving || isLocked}
            onClick={handlePrimaryAction}
            className="canvas-open-button"
          >
            <CheckCircle2 className="h-3.5 w-3.5" />
            <span>Review & lock</span>
          </button>

          {/* Secondary Actions in ··· Menu */}
          <div className="relative">
            <button
              type="button"
              onClick={() => setShowSecondaryMenu(!showSecondaryMenu)}
              className="flex h-7 w-7 items-center justify-center rounded border border-[var(--color-border-subtle)] hover:bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition"
              title="More actions"
              aria-label="More draft actions"
              aria-expanded={showSecondaryMenu}
            >
              <MoreVertical className="h-3.5 w-3.5" />
            </button>

            {showSecondaryMenu && (
              <div
                className="absolute right-0 top-8 z-50 w-44 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-1)] py-1 shadow-lg text-xs"
                onMouseLeave={() => setShowSecondaryMenu(false)}
              >
                <button
                  type="button"
                  disabled={!activeDraft || saving || loading || isLocked || !isModified}
                  onClick={handleSaveDraft}
                  className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)]"
                >
                  <Edit3 className="h-3.5 w-3.5" />
                  <span>Save Draft</span>
                </button>
                <button
                  type="button"
                  onClick={handleCopyContent}
                  className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)]"
                >
                  <Copy className="h-3.5 w-3.5" />
                  <span>Copy Content</span>
                </button>
                <button
                  type="button"
                  disabled={!activeDraft || saving || loading}
                  onClick={handleDeleteDraft}
                  className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-rose-400 hover:bg-[var(--color-surface-2)]"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                  <span>Delete Draft</span>
                </button>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Main Content Area */}
      <div className="wb-split flex flex-1 min-h-0">
        {/* Left Column: Drafts List */}
        <div className="wb-master w-64 shrink-0 border-r border-[var(--color-border-subtle)] overflow-y-auto p-2 space-y-1.5 bg-[var(--color-surface-1)]/20">
          {loadError && <div role="alert" className="p-3 text-xs text-rose-400">Could not load drafts: {loadError}</div>}
          {drafts.length === 0 && !loading && !loadError && (
            <div className="p-4 text-center text-xs text-[var(--color-fg-subtle)]">
              No drafts recorded.
            </div>
          )}

          {drafts.map((draft) => {
            const isSelected = draft.id === selectedDraftId;
            return (
              <button
                key={draft.id}
                type="button"
                disabled={saving}
                onClick={() => setSelectedDraftId(draft.id)}
                className={`w-full text-left rounded-lg p-2.5 transition text-xs border ${
                  isSelected
                    ? 'border-emerald-500/40 bg-[var(--color-surface-2)] shadow-xs'
                    : 'border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] hover:border-[var(--color-border-default)]'
                }`}
              >
                <div className="flex items-center justify-between gap-1 mb-1">
                  <span className="font-semibold text-xs truncate text-[var(--color-editor-fg)]">
                    {draft.title}
                  </span>
                  {draft.payload_lock ? (
                    <Lock className="h-3 w-3 text-purple-400 shrink-0" />
                  ) : null}
                </div>
                <div className="flex items-center justify-between text-[10px] text-[var(--color-fg-subtle)]">
                  <span className="uppercase">{draft.kind}</span>
                  <span
                    className={`rounded px-1.5 py-0.2 font-semibold ${
                      draft.status === 'approved'
                        ? 'bg-emerald-500/10 text-emerald-400'
                        : 'bg-zinc-500/10 text-[var(--color-fg-muted)]'
                    }`}
                  >
                    {draft.status}
                  </span>
                </div>
              </button>
            );
          })}
        </div>

        {/* Right Column: Draft Editor */}
        <div className="flex-1 overflow-y-auto p-4 flex flex-col space-y-3">
          {activeDraft ? (
            <>
              {/* Title Input */}
              <input
                type="text"
                value={editingTitle}
                disabled={isLocked || saving || loading}
                onChange={(e) => { setEditingTitle(e.target.value); draftEdits.set(activeDraft.id, { title: e.target.value, content: editingContent }); }}
                aria-label="Draft title"
                className="wb-draft-title"
                placeholder="Draft Title"
              />

              {/* Content Textarea */}
              <textarea
                value={editingContent}
                disabled={isLocked || saving || loading}
                onChange={(e) => { setEditingContent(e.target.value); draftEdits.set(activeDraft.id, { title: editingTitle, content: e.target.value }); }}
                aria-label="Draft content"
                className="wb-draft-content"
                placeholder="Draft content..."
              />

              {isLocked && (
                <div className="flex items-center gap-2 rounded bg-purple-500/5 border border-purple-500/20 px-3 py-1.5 text-xs text-purple-300">
                  <Lock className="h-3.5 w-3.5 shrink-0" />
                  <span>Draft is locked after local review. This does not authorize or confirm sending.</span>
                </div>
              )}
            </>
          ) : (
            <div className="flex h-full items-center justify-center text-xs text-[var(--color-fg-subtle)]">
              Select or create a draft to start reviewing.
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
