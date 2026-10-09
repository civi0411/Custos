import React, { useEffect, useState, useMemo } from 'react';
import {
  FileText,
  Plus,
  Save,
  History,
  Hash,
  Search,
  Tag,
  ShieldCheck,
  AlertCircle,
  Eye,
  Edit3,
  Columns,
  RotateCcw,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { NoteRecord, NoteVersionRecord } from '@/types/domain';

interface NotesWorkbenchPaneProps {
  sessionId?: string;
  onShowToast?: (msg: string) => void;
}

export const NotesWorkbenchPane: React.FC<NotesWorkbenchPaneProps> = ({
  sessionId,
  onShowToast,
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

  const selectNote = (note: NoteRecord) => {
    setSelectedNoteId(note.id);
    setTitle(note.title);
    setContent(note.content);
    setTagsInput((note.tags || []).join(', '));
    setIsDirty(false);
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

  const handleSave = async () => {
    if (!title.trim()) {
      onShowToast?.('Note title cannot be empty.');
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
        title,
        content,
        session_id: sessionId,
        tags,
      });

      onShowToast?.(`Note "${saved.title}" (v${saved.version}) saved with SHA-256 digest.`);
      setIsDirty(false);
      setSelectedNoteId(saved.id);

      // Refresh notes list
      const list = await daemonClient.listNotes(sessionId);
      setNotes(list);
    } catch (err: any) {
      setError(err?.message || String(err));
      onShowToast?.(`Save failed: ${err?.message || err}`);
    } finally {
      setIsSaving(false);
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
      {/* Top Header */}
      <div className="flex h-12 flex-shrink-0 items-center justify-between border-b border-[var(--color-border-default)] px-4 bg-[var(--color-surface-1)]">
        <div className="flex items-center gap-3">
          <FileText className="h-4 w-4 text-indigo-400" />
          <div className="flex flex-col">
            <span className="text-xs font-semibold uppercase tracking-wider text-[var(--color-editor-fg)]">
              Personal Notes & Knowledge
            </span>
            <span className="text-[10px] text-[var(--color-fg-subtle)]">
              Versioned markdown notes with cryptographic SHA-256 content hashes
            </span>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {/* View mode toggle */}
          <div className="flex rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] p-0.5">
            <button
              onClick={() => setViewMode('edit')}
              className={`p-1 rounded text-xs transition ${
                viewMode === 'edit'
                  ? 'bg-[var(--color-surface-1)] text-[var(--color-editor-fg)]'
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
                  ? 'bg-[var(--color-surface-1)] text-[var(--color-editor-fg)]'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
              title="Split Mode"
            >
              <Columns className="h-3.5 w-3.5" />
            </button>
            <button
              onClick={() => setViewMode('preview')}
              className={`p-1 rounded text-xs transition ${
                viewMode === 'preview'
                  ? 'bg-[var(--color-surface-1)] text-[var(--color-editor-fg)]'
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
                  ? 'border-indigo-500/50 bg-indigo-500/10 text-indigo-300'
                  : 'border-[var(--color-border-default)] bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)]'
              }`}
            >
              <History className="h-3.5 w-3.5" />
              History (v{selectedNote.version})
            </button>
          )}

          <button
            onClick={startNewNote}
            className="flex items-center gap-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1 text-xs font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] transition"
          >
            <Plus className="h-3.5 w-3.5" />
            New Note
          </button>

          <button
            onClick={handleSave}
            disabled={isSaving || !isDirty}
            className="flex items-center gap-1.5 rounded-lg bg-emerald-600 px-3 py-1 text-xs font-medium text-white hover:bg-emerald-500 transition disabled:opacity-40"
          >
            <Save className={`h-3.5 w-3.5 ${isSaving ? 'animate-spin' : ''}`} />
            {isSaving ? 'Saving...' : 'Save Note'}
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
                placeholder="Search notes or tags..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas)] py-1 pl-8 pr-2.5 text-xs text-[var(--color-editor-fg)] placeholder:text-[var(--color-fg-subtle)] focus:border-indigo-500 focus:outline-none"
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
                  className="mt-2 text-indigo-400 hover:underline text-[11px]"
                >
                  Create your first note
                </button>
              </div>
            )}

            {filteredNotes.map((note) => {
              const isSelected = selectedNoteId === note.id;
              return (
                <button
                  key={note.id}
                  onClick={() => selectNote(note)}
                  className={`w-full rounded-lg p-2.5 text-left transition border ${
                    isSelected
                      ? 'border-indigo-500/30 bg-indigo-500/10 text-[var(--color-editor-fg)]'
                      : 'border-transparent hover:border-[var(--color-border-muted)] hover:bg-[var(--color-surface-2)] text-[var(--color-fg-muted)]'
                  }`}
                >
                  <div className="flex items-start justify-between gap-1">
                    <span className="truncate text-xs font-medium text-[var(--color-editor-fg)]">
                      {note.title}
                    </span>
                    <span className="rounded bg-[var(--color-surface-3)] px-1 py-0.5 text-[9px] font-semibold text-[var(--color-editor-fg)]">
                      v{note.version}
                    </span>
                  </div>

                  <p className="mt-1 line-clamp-2 text-[10px] text-[var(--color-fg-muted)] leading-relaxed">
                    {note.content.replace(/^#+\s+/gm, '')}
                  </p>

                  <div className="mt-2 flex items-center justify-between text-[9px] text-[var(--color-fg-subtle)]">
                    <span className="flex items-center gap-1 font-mono">
                      <Hash className="h-2.5 w-2.5" />
                      {(note.content_hash || note.contentHash || '').slice(0, 8)}
                    </span>
                    <span>{new Date(note.updated_at * 1000).toLocaleDateString()}</span>
                  </div>
                </button>
              );
            })}
          </div>

          <div className="border-t border-[var(--color-border-muted)] p-2.5 text-[10px] text-[var(--color-fg-subtle)] flex items-center justify-between">
            <span>Notes: {notes.length}</span>
            <span className="flex items-center gap-1 text-emerald-500">
              <ShieldCheck className="h-3 w-3" />
              Lineage Tracked
            </span>
          </div>
        </div>

        {/* Center: Editor & Preview */}
        <div className="flex flex-1 flex-col overflow-hidden bg-[var(--color-canvas)]">
          {/* Metadata Row */}
          <div className="border-b border-[var(--color-border-default)] p-3 bg-[var(--color-surface-1)]">
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

              {selectedNote && (
                <div className="flex items-center gap-2 text-[10px] text-[var(--color-fg-muted)]">
                  <span className="font-mono rounded bg-[var(--color-surface-2)] px-2 py-0.5 border border-[var(--color-border-muted)]">
                    hash: {(selectedNote.content_hash || selectedNote.contentHash || '').slice(0, 12)}...
                  </span>
                  {isDirty && (
                    <span className="text-amber-400 font-medium">• Unsaved changes</span>
                  )}
                </div>
              )}
            </div>

            <div className="mt-2 flex items-center gap-2">
              <Tag className="h-3 w-3 text-[var(--color-fg-subtle)]" />
              <input
                type="text"
                placeholder="Tags (comma-separated, e.g. hypothesis, scaling, physics)"
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
                className="text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
              >
                Close
              </button>
            </div>

            <div className="mt-3 space-y-3">
              {versions.length === 0 ? (
                <p className="text-xs text-[var(--color-fg-muted)]">
                  No previous versions recorded. This is the initial version (v1).
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

                    <div className="mt-1 font-mono text-[9px] text-[var(--color-fg-subtle)] truncate">
                      hash: {ver.content_hash || ver.contentHash}
                    </div>

                    <button
                      onClick={() => {
                        setContent(ver.content);
                        setIsDirty(true);
                        onShowToast?.(`Loaded content from v${ver.version}. Click Save to commit as next version.`);
                      }}
                      className="mt-2 flex items-center gap-1 rounded bg-[var(--color-surface-3)] px-2 py-1 text-[10px] font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-4)] transition"
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
