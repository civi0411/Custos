import React, { useEffect, useState, useMemo, useRef, useCallback } from 'react';
import {
  FileText,
  Plus,
  Save,
  History,
  Search,
  Tag,
  Eye,
  Edit3,
  Columns,
  RotateCcw,
  Send,
  Check,
  RefreshCw,
  AlertCircle,
  X,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { NoteRecord, NoteVersionRecord } from '@/types/domain';

// Module-level draft cache to prevent data loss on tab close or screen switch
const noteDrafts = new Map<string, { title: string; content: string; tagsInput: string }>();

interface NotesWorkbenchPaneProps {
  sessionId?: string;
  onShowToast?: (msg: string) => void;
  onAskAgent?: (draft: string) => void;
}

export const NotesWorkbenchPane: React.FC<NotesWorkbenchPaneProps> = ({
  sessionId,
  onShowToast,
  onAskAgent,
}) => {
  const [notes, setNotes] = useState<NoteRecord[]>([]);
  const [selectedNoteId, setSelectedNoteId] = useState<string | null>(null);
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [tagsInput, setTagsInput] = useState('');
  const [viewMode, setViewMode] = useState<'edit' | 'preview' | 'split'>('split');
  const [searchQuery, setSearchQuery] = useState('');
  const [showHistory, setShowHistory] = useState(false);
  const [versions, setVersions] = useState<NoteVersionRecord[]>([]);
  const [isSaving, setIsSaving] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [isDirty, setIsDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [justSaved, setJustSaved] = useState(false);
  const [justInserted, setJustInserted] = useState(false);

  const autoSaveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const fetchNotes = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const list = await daemonClient.listNotes(sessionId);
      setNotes(list);
      if (!selectedNoteId && list.length > 0) {
        selectNote(list[0]);
      }
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const latestStateRef = useRef({ title, content, tagsInput, selectedNoteId, isDirty, sessionId });
  useEffect(() => {
    latestStateRef.current = { title, content, tagsInput, selectedNoteId, isDirty, sessionId };
  }, [title, content, tagsInput, selectedNoteId, isDirty, sessionId]);

  useEffect(() => {
    return () => {
      const state = latestStateRef.current;
      if (state.isDirty && state.title.trim()) {
        const tags = state.tagsInput
          .split(',')
          .map((t) => t.trim())
          .filter((t) => t.length > 0);
        void daemonClient.saveNote({
          id: state.selectedNoteId || undefined,
          title: state.title.trim(),
          content: state.content,
          session_id: state.sessionId,
          tags,
        });
      }
    };
  }, []);

  const selectNote = (note: NoteRecord) => {
    const current = latestStateRef.current;
    if (current.isDirty && current.selectedNoteId) {
      noteDrafts.set(current.selectedNoteId, {
        title: current.title,
        content: current.content,
        tagsInput: current.tagsInput,
      });
    }

    setSelectedNoteId(note.id);
    const draft = noteDrafts.get(note.id);
    if (draft) {
      setTitle(draft.title);
      setContent(draft.content);
      setTagsInput(draft.tagsInput);
      setIsDirty(true);
    } else {
      setTitle(note.title);
      setContent(note.content);
      setTagsInput((note.tags || []).join(', '));
      setIsDirty(false);
    }
    setShowHistory(false);
  };

  const startNewNote = () => {
    setSelectedNoteId(null);
    setTitle('Untitled Note');
    setContent('# Untitled Note\n\nWrite your thoughts, hypothesis, or task notes here...');
    setTagsInput('');
    setIsDirty(true);
    setShowHistory(false);
  };

  useEffect(() => {
    fetchNotes();
  }, [sessionId]);

  const selectedNote = useMemo(
    () => notes.find((n) => n.id === selectedNoteId) || null,
    [notes, selectedNoteId]
  );

  const fetchHistory = async () => {
    if (!selectedNoteId) return;
    try {
      const list = await daemonClient.listNoteVersions(selectedNoteId);
      setVersions(list);
      setShowHistory(true);
    } catch (err: any) {
      onShowToast?.(`Failed to load version history: ${err?.message || err}`);
    }
  };

  const executeSave = useCallback(
    async (options?: { silent?: boolean }) => {
      if (!title.trim()) {
        if (!options?.silent) {
          onShowToast?.('Note title cannot be empty.');
        }
        return;
      }
      setIsSaving(true);
      setError(null);
      try {
        const tags = tagsInput
          .split(',')
          .map((t) => t.trim())
          .filter((t) => t.length > 0);

        const saved = await daemonClient.saveNote({
          id: selectedNoteId || undefined,
          title: title.trim(),
          content,
          session_id: sessionId,
          tags,
        });

        setIsDirty(false);
        setSelectedNoteId(saved.id);
        noteDrafts.delete(saved.id);
        setJustSaved(true);
        setTimeout(() => setJustSaved(false), 2000);

        if (!options?.silent) {
          onShowToast?.(`Saved note "${saved.title}" (v${saved.version})`);
        }

        // Refresh notes list quietly
        const list = await daemonClient.listNotes(sessionId);
        setNotes(list);
      } catch (err: any) {
        setError(err?.message || String(err));
        if (!options?.silent) {
          onShowToast?.(`Save failed: ${err?.message || err}`);
        }
      } finally {
        setIsSaving(false);
      }
    },
    [title, content, tagsInput, selectedNoteId, sessionId, onShowToast]
  );

  // Debounced auto-save effect
  useEffect(() => {
    if (!isDirty) return;

    if (autoSaveTimerRef.current) {
      clearTimeout(autoSaveTimerRef.current);
    }

    autoSaveTimerRef.current = setTimeout(() => {
      executeSave({ silent: true });
    }, 1500);

    return () => {
      if (autoSaveTimerRef.current) {
        clearTimeout(autoSaveTimerRef.current);
      }
    };
  }, [isDirty, executeSave]);

  const handleInsertToChat = () => {
    if (!content.trim()) {
      onShowToast?.('Note is empty.');
      return;
    }
    const snippet = `Context Note [${title}]:\n\`\`\`markdown\n${content}\n\`\`\``;
    if (onAskAgent) {
      onAskAgent(snippet);
      setJustInserted(true);
      setTimeout(() => setJustInserted(false), 2000);
      onShowToast?.(`Inserted note "${title}" into chat input.`);
    } else {
      navigator.clipboard.writeText(snippet);
      onShowToast?.(`Copied note "${title}" to clipboard.`);
    }
  };

  const filteredNotes = useMemo(() => {
    if (!searchQuery.trim()) return notes;
    const q = searchQuery.toLowerCase();
    return notes.filter(
      (n) =>
        n.title.toLowerCase().includes(q) ||
        n.content.toLowerCase().includes(q) ||
        (n.tags || []).some((t) => t.toLowerCase().includes(q))
    );
  }, [notes, searchQuery]);

  return (
    <div className="flex h-full w-full flex-col bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
      {/* Compact Top Bar */}
      <div className="flex h-9 flex-shrink-0 items-center justify-between border-b border-[var(--color-border-default)] px-3 bg-[var(--color-surface-1)]">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <FileText className="h-4 w-4 text-white/70" />
            <span className="text-xs font-semibold text-[var(--color-editor-fg)]">
              Scratchpad & Notes
            </span>
          </div>

          <div className="h-3.5 w-px bg-[var(--color-border-muted)]" />

          {/* Clean Status Pill */}
          <div className="flex items-center text-[11px]">
            {isSaving ? (
              <span className="flex items-center gap-1.5 text-[var(--color-fg-muted)]">
                <RefreshCw className="h-3 w-3 animate-spin text-white/60" />
                Saving...
              </span>
            ) : isDirty ? (
              <span className="flex items-center gap-1.5 font-medium text-white/80">
                <span className="h-1.5 w-1.5 rounded-full bg-white/50" />
                Unsaved changes
              </span>
            ) : justSaved ? (
              <span className="flex items-center gap-1.5 font-medium text-white">
                <Check className="h-3 w-3 text-white/80" />
                Saved
              </span>
            ) : (
              <span className="flex items-center gap-1.5 text-[var(--color-fg-subtle)]">
                <span className="h-1.5 w-1.5 rounded-full bg-white/70" />
                Auto-saved
              </span>
            )}
          </div>
        </div>

        <div className="flex items-center gap-2">
          {/* Segmented View Mode Toggle */}
          <div className="flex rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] p-0.5">
            <button
              onClick={() => setViewMode('edit')}
              className={`p-1 rounded text-xs transition ${
                viewMode === 'edit'
                  ? 'bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] shadow-sm'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
              title="Edit Mode"
            >
              <Edit3 className="h-3.5 w-3.5" />
            </button>
            <button
              onClick={() => setViewMode('split')}
              className={`p-1 rounded text-xs transition ${
                viewMode === 'split'
                  ? 'bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] shadow-sm'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
              title="Split View"
            >
              <Columns className="h-3.5 w-3.5" />
            </button>
            <button
              onClick={() => setViewMode('preview')}
              className={`p-1 rounded text-xs transition ${
                viewMode === 'preview'
                  ? 'bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] shadow-sm'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
              title="Preview Mode"
            >
              <Eye className="h-3.5 w-3.5" />
            </button>
          </div>

          {selectedNote && (
            <button
              onClick={fetchHistory}
              className={`flex items-center gap-1.5 rounded-lg border px-2.5 py-1 text-xs font-medium transition ${
                showHistory
                  ? 'border-white/20 bg-white/10 text-white'
                  : 'border-[var(--color-border-default)] bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)]'
              }`}
              title="Version history"
            >
              <History className="h-3.5 w-3.5" />
              v{selectedNote.version}
            </button>
          )}

          <button
            onClick={startNewNote}
            className="flex items-center gap-1.5 rounded-md border border-white/15 bg-white/5 px-2.5 py-1 text-xs font-medium text-white/90 hover:bg-white/10 hover:text-white transition"
          >
            <Plus className="h-3.5 w-3.5" />
            New Note
          </button>

          {/* Primary Action Button: Insert to Chat */}
          <button
            onClick={handleInsertToChat}
            className="flex items-center gap-1.5 rounded-md bg-white hover:bg-white/90 px-3 py-1 text-xs font-semibold text-black transition shadow-sm"
            title="Insert this note into prompt input"
          >
            {justInserted ? (
              <>
                <Check className="h-3.5 w-3.5 text-black stroke-[2.5]" />
                <span>Inserted</span>
              </>
            ) : (
              <>
                <Send className="h-3.5 w-3.5 text-black" />
                <span>Insert to Chat</span>
              </>
            )}
          </button>
        </div>
      </div>

      {/* Main Container */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left Sidebar: Notes List */}
        <div className="flex w-64 flex-shrink-0 flex-col border-r border-[var(--color-border-default)] bg-[var(--color-surface-1)]">
          <div className="border-b border-[var(--color-border-muted)] p-2.5">
            <div className="relative">
              <Search className="absolute left-2.5 top-2 h-3.5 w-3.5 text-[var(--color-fg-muted)]" />
              <input
                type="text"
                placeholder="Search notes..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas)] py-1 pl-8 pr-2.5 text-xs text-[var(--color-editor-fg)] placeholder:text-[var(--color-fg-subtle)] focus:border-white/30 focus:outline-none"
              />
            </div>
          </div>

          <div className="flex-1 overflow-y-auto p-2 space-y-1">
            {error && (
              <div className="m-2 rounded-lg border border-red-500/20 bg-red-500/10 p-2 text-xs text-red-400">
                <AlertCircle className="inline mr-1 h-3.5 w-3.5" />
                {error}
              </div>
            )}
            {filteredNotes.length === 0 && !isLoading && (
              <div className="py-8 text-center text-xs text-[var(--color-fg-muted)]">
                <FileText className="mx-auto mb-2 h-6 w-6 opacity-40" />
                <p>No notes found.</p>
                <button
                  onClick={startNewNote}
                  className="mt-2 text-white/80 hover:text-white hover:underline text-[11px]"
                >
                  Create your first note
                </button>
              </div>
            )}

            {filteredNotes.map((note) => {
              const isSelected = selectedNoteId === note.id;
              const dateStr = note.updated_at
                ? new Date(note.updated_at * 1000).toLocaleDateString(undefined, {
                    month: 'short',
                    day: 'numeric',
                  })
                : '';

              return (
                <button
                  key={note.id}
                  onClick={() => selectNote(note)}
                  className={`w-full rounded-lg p-2.5 text-left transition border ${
                    isSelected
                      ? 'border-white/20 bg-white/10 text-[var(--color-editor-fg)]'
                      : 'border-transparent hover:border-[var(--color-border-muted)] hover:bg-[var(--color-surface-2)] text-[var(--color-fg-muted)]'
                  }`}
                >
                  <div className="flex items-start justify-between gap-1">
                    <span className="truncate text-xs font-medium text-[var(--color-editor-fg)]">
                      {note.title}
                    </span>
                    <span className="rounded bg-[var(--color-surface-3)] px-1 py-0.5 text-[9px] font-semibold text-[var(--color-fg-muted)]">
                      v{note.version}
                    </span>
                  </div>

                  <p className="mt-1 line-clamp-2 text-[10px] text-[var(--color-fg-muted)] leading-relaxed">
                    {note.content.replace(/^#+\s+/gm, '')}
                  </p>

                  <div className="mt-2 flex items-center justify-between text-[9px] text-[var(--color-fg-subtle)]">
                    <div className="flex items-center gap-1 truncate">
                      {(note.tags || []).slice(0, 2).map((t, i) => (
                        <span
                          key={i}
                          className="rounded bg-[var(--color-surface-2)] px-1 py-0.5 text-[9px] text-[var(--color-fg-muted)]"
                        >
                          #{t}
                        </span>
                      ))}
                    </div>
                    <span>{dateStr}</span>
                  </div>
                </button>
              );
            })}
          </div>

          <div className="border-t border-[var(--color-border-muted)] px-3 py-2 text-[10px] text-[var(--color-fg-subtle)] flex items-center justify-between">
            <span>Notes: {notes.length}</span>
            <span>Markdown</span>
          </div>
        </div>

        {/* Center: Editor & Preview */}
        <div className="flex flex-1 flex-col overflow-hidden bg-[var(--color-canvas)]">
          {/* Metadata Bar */}
          <div className="border-b border-[var(--color-border-default)] px-4 py-2.5 bg-[var(--color-surface-1)]">
            <div className="flex items-center gap-3">
              <input
                type="text"
                placeholder="Note Title"
                value={title}
                onChange={(e) => {
                  setTitle(e.target.value);
                  setIsDirty(true);
                }}
                className="flex-1 bg-transparent text-sm font-semibold text-[var(--color-editor-fg)] placeholder:text-[var(--color-fg-subtle)] focus:outline-none"
              />

              {isDirty && (
                <button
                  onClick={() => executeSave()}
                  disabled={isSaving}
                  className="flex items-center gap-1 rounded-md bg-white hover:bg-white/90 px-2.5 py-1 text-xs font-semibold text-black transition shadow-sm"
                  title="Save now"
                >
                  <Save className="h-3 w-3 text-black" />
                  Save
                </button>
              )}
            </div>

            <div className="mt-1.5 flex items-center gap-2">
              <Tag className="h-3 w-3 text-[var(--color-fg-subtle)] shrink-0" />
              <input
                type="text"
                placeholder="Tags (comma-separated, e.g. task, prompt, experiment)"
                value={tagsInput}
                onChange={(e) => {
                  setTagsInput(e.target.value);
                  setIsDirty(true);
                }}
                className="w-full bg-transparent text-[11px] text-[var(--color-fg-muted)] placeholder:text-[var(--color-fg-subtle)] focus:outline-none"
              />
            </div>
          </div>

          {/* Editor Workspace */}
          <div className="flex flex-1 overflow-hidden">
            {/* Markdown Textarea */}
            {(viewMode === 'edit' || viewMode === 'split') && (
              <div
                className={`flex-1 h-full p-4 overflow-y-auto ${
                  viewMode === 'split' ? 'border-r border-[var(--color-border-default)]' : ''
                }`}
              >
                <textarea
                  value={content}
                  onChange={(e) => {
                    setContent(e.target.value);
                    setIsDirty(true);
                  }}
                  placeholder="Start writing markdown notes..."
                  className="h-full w-full resize-none bg-transparent font-mono text-xs leading-relaxed text-[var(--color-editor-fg)] placeholder:text-[var(--color-fg-subtle)] focus:outline-none"
                />
              </div>
            )}

            {/* Markdown Rendered Preview */}
            {(viewMode === 'preview' || viewMode === 'split') && (
              <div className="flex-1 h-full p-6 overflow-y-auto prose prose-invert prose-xs max-w-none">
                <div className="text-xs leading-relaxed text-[var(--color-editor-fg)] whitespace-pre-wrap font-sans">
                  {content}
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Right Drawer: Version History */}
        {showHistory && (
          <div className="w-72 flex-shrink-0 border-l border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-4 overflow-y-auto">
            <div className="flex items-center justify-between pb-3 border-b border-[var(--color-border-muted)]">
              <span className="text-xs font-semibold text-[var(--color-editor-fg)]">
                Version History
              </span>
              <button
                onClick={() => setShowHistory(false)}
                className="text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] p-1 rounded hover:bg-[var(--color-surface-2)]"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </div>

            <div className="mt-3 space-y-3">
              {versions.length === 0 ? (
                <p className="text-xs text-[var(--color-fg-muted)]">
                  No previous versions recorded.
                </p>
              ) : (
                versions.map((ver) => (
                  <div
                    key={ver.id}
                    className="rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-2)] p-2.5 text-xs"
                  >
                    <div className="flex items-center justify-between">
                      <span className="font-semibold text-[var(--color-editor-fg)]">
                        v{ver.version}
                      </span>
                      <span className="text-[10px] text-[var(--color-fg-muted)]">
                        {new Date(ver.created_at * 1000).toLocaleTimeString()}
                      </span>
                    </div>

                    <button
                      onClick={() => {
                        setContent(ver.content);
                        setIsDirty(true);
                        onShowToast?.(`Loaded content from v${ver.version}.`);
                      }}
                      className="mt-2.5 flex items-center gap-1 rounded bg-[var(--color-surface-3)] px-2 py-1 text-[10px] font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-4)] transition w-full justify-center"
                    >
                      <RotateCcw className="h-3 w-3" />
                      Restore Content
                    </button>
                  </div>
                ))
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
