import React, { useState, useEffect, useCallback } from 'react';
import {
  BookOpen,
  Search,
  ExternalLink,
  ShieldCheck,
  Plus,
  FileText,
  Highlighter,
  MessageSquare,
  Sparkles,
  GitCompare,
  CheckSquare,
  Square,
  PanelLeftClose,
  PanelLeft,
  Copy,
  Check,
  Hash,
  X,
  Bookmark,
} from 'lucide-react';
import type { SourceRecord, PassageAnchor } from '@/types/research';
import { daemonClient } from '@/api/daemon_client';
import { JobActionButton } from '@/components/common';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';

interface LiteraturePaneProps {
  sessionId?: string;
  onShowToast?: (msg: string) => void;
  onOpenTab?: (id: OrcaTabId) => void;
}

export const LiteraturePane: React.FC<LiteraturePaneProps> = ({
  sessionId,
  onShowToast,
  onOpenTab,
}) => {
  const [sources, setSources] = useState<SourceRecord[]>([]);
  const [selectedSourceId, setSelectedSourceId] = useState<string>('');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedText, setSelectedText] = useState('');
  const [loadError, setLoadError] = useState<string | null>(null);
  const [checkedSourceIds, setCheckedSourceIds] = useState<Set<string>>(new Set());
  const [isSidebarOpen, setIsSidebarOpen] = useState(true);
  const [anchors, setAnchors] = useState<PassageAnchor[]>([]);
  const [loadingAnchors, setLoadingAnchors] = useState(false);
  const [copiedHash, setCopiedHash] = useState(false);
  const [isAddSourceOpen, setIsAddSourceOpen] = useState(false);
  const [newTitle, setNewTitle] = useState('');
  const [newDoi, setNewDoi] = useState('');
  const [newAuthors, setNewAuthors] = useState('');
  const [newYear, setNewYear] = useState(new Date().getFullYear().toString());
  const [newAbstract, setNewAbstract] = useState('');

  const loadSources = useCallback(() => {
    daemonClient
      .listResearchSources()
      .then((loaded) => {
        setSources(loaded || []);
        if (loaded && loaded.length > 0 && !selectedSourceId) {
          setSelectedSourceId(loaded[0].id);
        }
      })
      .catch((err) => {
        setLoadError(String(err));
        setSources([]);
      });
  }, [selectedSourceId]);

  useEffect(() => {
    loadSources();
  }, [loadSources]);

  const activeSource = sources.find((s) => s.id === selectedSourceId);

  // Load anchors for active source
  useEffect(() => {
    if (!activeSource?.id) {
      setAnchors([]);
      return;
    }
    setLoadingAnchors(true);
    daemonClient
      .listResearchAnchors(activeSource.id)
      .then((res) => {
        setAnchors(res || []);
      })
      .catch(() => {
        setAnchors([]);
      })
      .finally(() => setLoadingAnchors(false));
  }, [activeSource?.id]);

  const toggleSourceCheck = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setCheckedSourceIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const handleTextSelect = () => {
    const selection = window.getSelection();
    const excerpt = selection?.toString().trim() ?? '';
    if (excerpt.length > 5 && activeSource?.abstractText?.includes(excerpt)) {
      setSelectedText(excerpt);
    }
  };

  const handleSavePassage = async () => {
    if (!selectedText || !activeSource?.abstractText) return;

    const startOffset = activeSource.abstractText.indexOf(selectedText);
    if (startOffset < 0 || activeSource.abstractText.indexOf(selectedText, startOffset + 1) >= 0) {
      onShowToast?.('Passage location is ambiguous. Select a unique excerpt from the abstract.');
      return;
    }

    try {
      const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(selectedText));
      const passageHash = Array.from(new Uint8Array(digest), (byte) =>
        byte.toString(16).padStart(2, '0')
      ).join('');
      const anchor: PassageAnchor = {
        id: crypto.randomUUID(),
        sourceId: activeSource.id,
        sourceTitle: activeSource.title,
        sectionTitle: 'Abstract',
        startOffset,
        endOffset: startOffset + selectedText.length,
        exactText: selectedText,
        passageHash,
      };
      await daemonClient.saveResearchAnchor(anchor);
      onShowToast?.('Passage anchored to cryptographic corpus evidence.');
      setSelectedText('');
      // Reload anchors
      const updated = await daemonClient.listResearchAnchors(activeSource.id);
      setAnchors(updated || []);
    } catch (err) {
      onShowToast?.(`Could not save passage: ${String(err)}`);
    }
  };

  const handleCopyHash = (hash: string) => {
    void navigator.clipboard.writeText(hash);
    setCopiedHash(true);
    setTimeout(() => setCopiedHash(false), 2000);
    onShowToast?.('Content hash copied to clipboard.');
  };

  const handleCreateSource = async () => {
    if (!newTitle.trim()) return;
    try {
      const id = crypto.randomUUID();
      const content = `${newTitle}\n${newAbstract}`;
      const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(content));
      const contentHash = Array.from(new Uint8Array(digest), (b) =>
        b.toString(16).padStart(2, '0')
      ).join('');

      const sourceRecord: SourceRecord = {
        id,
        sourceType: 'paper',
        title: newTitle.trim(),
        doi: newDoi.trim() || undefined,
        authors: newAuthors.split(',').map((a) => a.trim()).filter(Boolean),
        year: parseInt(newYear, 10) || new Date().getFullYear(),
        contentHash,
        verified: false,
        abstractText: newAbstract.trim() || undefined,
        createdAt: Date.now(),
      };

      await daemonClient.saveResearchSource(sourceRecord);
      onShowToast?.(`Added "${sourceRecord.title}" to research corpus.`);
      setIsAddSourceOpen(false);
      setNewTitle('');
      setNewDoi('');
      setNewAuthors('');
      setNewAbstract('');
      loadSources();
      setSelectedSourceId(id);
    } catch (err: any) {
      onShowToast?.(`Failed to add source: ${err?.message || err}`);
    }
  };

  const filteredSources = sources.filter(
    (s) =>
      s.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      s.authors?.some((a: string) => a.toLowerCase().includes(searchQuery.toLowerCase())) ||
      s.doi?.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div className="wb-split flex h-full w-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] overflow-hidden select-none font-sans">
      {/* ── LEFT: SOURCE LIST (COLLAPSIBLE) ── */}
      {isSidebarOpen && (
        <div className="wb-master w-72 md:w-80 border-r border-[var(--color-border-muted)] flex flex-col bg-[var(--color-surface-1)] shrink-0 relative z-10 transition-all duration-150">
          {/* Header */}
          <div className="h-9 px-3 border-b border-[var(--color-border-muted)] flex items-center justify-between shrink-0 bg-[var(--color-surface-1)]">
            <div className="flex items-center gap-2">
              <BookOpen className="w-3.5 h-3.5 text-workbench-accent" />
              <span className="text-xs font-semibold text-[var(--color-editor-fg)]">Literature</span>
              <span className="rounded bg-[var(--color-surface-2)] px-1.5 py-0.2 font-mono text-[9px] text-[var(--color-fg-subtle)] border border-[var(--color-border-muted)]">
                {sources.length}
              </span>
            </div>
            <div className="flex items-center gap-1">
              <button
                type="button"
                onClick={() => setIsAddSourceOpen(true)}
                className="p-1 rounded-md text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
                title="Add paper to corpus"
              >
                <Plus size={14} />
              </button>
              <button
                type="button"
                onClick={() => setIsSidebarOpen(false)}
                className="p-1 rounded-md text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
                title="Collapse sources panel"
              >
                <PanelLeftClose size={14} />
              </button>
            </div>
          </div>

          {/* Search & Synthesize */}
          <div className="p-2.5 border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] flex items-center gap-1.5">
            <div className="relative group flex-1">
              <Search className="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--color-fg-subtle)] group-focus-within:text-[var(--color-editor-fg)] transition-colors" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search title, author, DOI…"
                className="w-full bg-[var(--color-canvas)] border border-[var(--color-border-muted)] rounded-lg py-1.5 pl-8 pr-2 text-xs text-[var(--color-editor-fg)] placeholder:text-[var(--color-fg-subtle)] outline-none focus:border-[var(--color-border-default)] transition"
              />
            </div>
            <JobActionButton
              jobId="research__deep_research"
              label="Synthesize"
              icon={Sparkles}
              getInputs={() => {
                const topic = searchQuery.trim() || activeSource?.title || '';
                if (!topic) return null;
                return {
                  topic,
                  search_scope: 'literature_corpus',
                };
              }}
              sessionId={sessionId}
              sourceTabId="literature"
              onNavigate={onOpenTab}
              notReadyTooltip="Enter a search term or select a paper to synthesize"
              readyTooltip="Synthesize literature on active topic"
              onShowToast={onShowToast}
              variant="toolbar"
              size="sm"
            />
          </div>

          {/* Source List */}
          <div className="flex-1 overflow-y-auto p-2 space-y-1.5 relative no-scrollbar">
            {loadError && (
              <div role="alert" className="p-2.5 rounded-lg border border-rose-500/30 bg-rose-500/10 text-xs text-rose-300">
                Sources error: {loadError}
              </div>
            )}
            {filteredSources.length === 0 && !loadError ? (
              <div className="p-4 text-center text-xs text-[var(--color-fg-subtle)]">
                {searchQuery ? 'No matching papers.' : 'No papers registered yet.'}
              </div>
            ) : (
              filteredSources.map((source) => {
                const isSelected = source.id === selectedSourceId;
                const isChecked = checkedSourceIds.has(source.id);
                return (
                  <div
                    key={source.id}
                    onClick={() => {
                      setSelectedSourceId(source.id);
                      setSelectedText('');
                    }}
                    className={`group p-2.5 rounded-xl border cursor-pointer transition text-xs ${
                      isSelected
                        ? 'bg-[var(--color-surface-2)] border-[var(--color-border-default)] text-[var(--color-editor-fg)] shadow-xs'
                        : 'bg-transparent border-transparent hover:bg-[var(--color-surface-2)]/60 hover:border-[var(--color-border-muted)]/50 text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                    }`}
                  >
                    <div className="flex items-start justify-between gap-1 mb-1">
                      <div className="flex items-center gap-1.5">
                        <button
                          type="button"
                          onClick={(e) => toggleSourceCheck(source.id, e)}
                          className="p-0.5 text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition"
                          title={isChecked ? 'Deselect source' : 'Select for comparison'}
                        >
                          {isChecked ? (
                            <CheckSquare className="w-3.5 h-3.5 text-emerald-400" />
                          ) : (
                            <Square className="w-3.5 h-3.5 text-[var(--color-fg-subtle)]" />
                          )}
                        </button>
                        <span className="font-mono text-[9px] uppercase px-1.5 py-0.2 rounded bg-[var(--color-surface-3)] text-[var(--color-fg-muted)] font-medium">
                          {source.sourceType}
                        </span>
                      </div>
                      {source.verified && (
                        <span className="flex items-center gap-1 text-[9px] text-emerald-400 font-mono font-medium bg-emerald-500/10 px-1.5 py-0.2 rounded border border-emerald-500/20">
                          <ShieldCheck size={10} /> Verified
                        </span>
                      )}
                    </div>

                    <h4 className="text-xs font-medium leading-snug mb-1 text-[var(--color-editor-fg)] line-clamp-2">
                      {source.title}
                    </h4>

                    <div className="flex items-center justify-between text-[10px] text-[var(--color-fg-subtle)]">
                      <span className="truncate max-w-[140px]">
                        {source.authors?.join(', ') || 'Unknown authors'}
                      </span>
                      <span className="font-mono">{source.year}</span>
                    </div>
                  </div>
                );
              })
            )}
          </div>

          {/* Floating Compare Action Bar when ≥ 2 sources checked */}
          {checkedSourceIds.size >= 2 && (
            <div className="p-2.5 border-t border-[var(--color-border-muted)] bg-[var(--color-surface-2)] flex items-center justify-between gap-2 shrink-0">
              <span className="text-[11px] font-mono font-medium text-[var(--color-fg-muted)]">
                {checkedSourceIds.size} selected
              </span>
              <JobActionButton
                jobId="research__compare"
                label={`Compare (${checkedSourceIds.size})`}
                icon={GitCompare}
                getInputs={() => ({
                  sources: Array.from(checkedSourceIds),
                  comparison_criteria:
                    'Methodological rigor, experimental datasets, reproducibility, and verified claims',
                })}
                sessionId={sessionId}
                sourceTabId="literature"
                onNavigate={onOpenTab}
                onShowToast={onShowToast}
                variant="toolbar"
                size="sm"
              />
            </div>
          )}
        </div>
      )}

      {/* ── RIGHT: DOCUMENT READER & PASSAGE ANCHORING ── */}
      <div className="flex-1 flex flex-col min-w-0 bg-[var(--color-canvas)] overflow-hidden relative">
        {/* Document Header */}
        <div className="h-9 px-3 border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] flex items-center justify-between shrink-0 z-20">
          <div className="flex items-center gap-2 min-w-0 flex-1">
            {!isSidebarOpen && (
              <button
                type="button"
                onClick={() => setIsSidebarOpen(true)}
                className="p-1 rounded-md text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition shrink-0"
                title="Show sources list"
              >
                <PanelLeft size={14} />
              </button>
            )}
            <FileText className="w-3.5 h-3.5 text-workbench-accent shrink-0" />
            <span className="font-semibold text-[var(--color-editor-fg)] text-xs truncate">
              {activeSource?.title || 'Literature Reader'}
            </span>
          </div>

          <div className="flex items-center gap-2 shrink-0">
            {activeSource && (
              <JobActionButton
                jobId="research__source_qa"
                label="Ask Paper"
                icon={MessageSquare}
                getInputs={() => {
                  return {
                    source_locator: activeSource.doi || activeSource.id,
                    question: `Analyze the primary claims, methodology, and empirical evidence in "${activeSource.title}"`,
                  };
                }}
                sessionId={sessionId}
                sourceTabId="literature"
                onNavigate={onOpenTab}
                notReadyTooltip="Select a paper to ask questions"
                readyTooltip={`Query claims and evidence in "${activeSource.title}"`}
                onShowToast={onShowToast}
                variant="toolbar"
              />
            )}
            {activeSource?.doi && (
              <a
                href={`https://doi.org/${activeSource.doi}`}
                target="_blank"
                rel="noreferrer"
                className="flex items-center gap-1.5 text-[11px] font-mono text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition px-2 py-0.5 rounded-md bg-[var(--color-surface-2)] border border-[var(--color-border-default)]"
              >
                <span>doi:{activeSource.doi}</span>
                <ExternalLink size={10} />
              </a>
            )}
          </div>
        </div>

        {/* Selected Passage Floating Banner */}
        {selectedText && (
          <div className="px-4 py-2 bg-[var(--color-surface-2)] border-b border-[var(--color-border-default)] flex items-center justify-between gap-4 z-30">
            <div className="flex items-center gap-2 min-w-0 flex-1">
              <Highlighter className="w-3.5 h-3.5 text-teal-400 shrink-0" />
              <div className="text-xs text-[var(--color-editor-fg)] truncate">
                <span className="font-semibold uppercase tracking-wider text-[9px] mr-2 text-[var(--color-fg-muted)]">
                  Selected Excerpt:
                </span>
                <span className="italic">"{selectedText}"</span>
              </div>
            </div>
            <div className="flex items-center gap-2 shrink-0">
              <button
                type="button"
                onClick={() => setSelectedText('')}
                className="px-2 py-1 text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
              >
                Clear
              </button>
              <button
                type="button"
                onClick={handleSavePassage}
                className="flex items-center gap-1.5 px-3 py-1 rounded-md bg-teal-600 hover:bg-teal-500 text-white text-xs font-medium transition"
              >
                <Bookmark size={12} />
                <span>Anchor as Evidence</span>
              </button>
            </div>
          </div>
        )}

        {/* Document Content Viewport */}
        <div className="flex-1 overflow-y-auto relative no-scrollbar">
          {!activeSource ? (
            <div className="flex h-full flex-col items-center justify-center p-8 text-center text-[var(--color-fg-subtle)] space-y-3">
              <BookOpen className="h-10 w-10 text-[var(--color-border-default)]" />
              <div className="text-sm font-medium text-[var(--color-editor-fg)]">
                No research paper selected
              </div>
              <p className="max-w-md text-xs leading-relaxed text-[var(--color-fg-muted)]">
                Add research papers to your corpus using the + button, or select a source on the left to read and ground evidence.
              </p>
              <button
                type="button"
                onClick={() => setIsAddSourceOpen(true)}
                className="mt-2 flex items-center gap-1.5 rounded-lg bg-teal-600 px-3.5 py-1.5 text-xs font-medium text-white hover:bg-teal-500 transition"
              >
                <Plus size={13} />
                Add Source to Corpus
              </button>
            </div>
          ) : (
            <div className="p-6 lg:p-8 select-text max-w-4xl mx-auto w-full space-y-6 relative z-10 font-sans">
              {/* Paper Title & Hero */}
              <div>
                <h1 className="text-lg lg:text-xl font-bold leading-snug text-[var(--color-editor-fg)] tracking-tight">
                  {activeSource.title}
                </h1>
                <div className="mt-2 flex flex-wrap items-center gap-2 text-xs text-[var(--color-fg-muted)]">
                  <span>{activeSource.authors?.join(', ') || 'Unknown Authors'}</span>
                  <span>·</span>
                  <span className="font-mono">{activeSource.year}</span>
                  {activeSource.doi && (
                    <>
                      <span>·</span>
                      <span className="font-mono text-[11px] text-teal-400">
                        DOI: {activeSource.doi}
                      </span>
                    </>
                  )}
                </div>
              </div>

              {/* Metadata Card */}
              <div className="p-3.5 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] flex flex-wrap items-center justify-between gap-3 text-xs">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-[10px] uppercase px-2 py-0.5 rounded bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] font-medium border border-[var(--color-border-muted)]">
                    {activeSource.sourceType}
                  </span>
                  {activeSource.verified && (
                    <span className="flex items-center gap-1 text-[10px] text-emerald-400 font-medium bg-emerald-500/10 px-2 py-0.5 rounded border border-emerald-500/20">
                      <ShieldCheck size={11} /> Verified Corpus Record
                    </span>
                  )}
                </div>

                <div className="flex items-center gap-2">
                  <span className="text-[10px] uppercase tracking-wider text-[var(--color-fg-subtle)] font-medium">
                    BLAKE3:
                  </span>
                  <button
                    type="button"
                    onClick={() => handleCopyHash(activeSource.contentHash)}
                    className="flex items-center gap-1 font-mono text-[10px] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] bg-[var(--color-surface-2)] px-2 py-0.5 rounded border border-[var(--color-border-muted)] transition"
                    title="Click to copy content hash"
                  >
                    <Hash size={10} />
                    <span>{activeSource.contentHash?.slice(0, 16)}…</span>
                    {copiedHash ? <Check size={10} className="text-emerald-400" /> : <Copy size={10} />}
                  </button>
                </div>
              </div>

              {/* Abstract Section */}
              <section className="space-y-3">
                <div className="flex items-center gap-2">
                  <span className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)]">
                    Abstract
                  </span>
                  <div className="h-px bg-[var(--color-border-muted)] flex-1" />
                  <span className="text-[10px] text-[var(--color-fg-subtle)]">
                    Select text to anchor passage
                  </span>
                </div>

                <div
                  onMouseUp={handleTextSelect}
                  className="text-xs lg:text-sm leading-relaxed text-[var(--color-editor-fg)] bg-[var(--color-surface-1)]/60 p-5 rounded-xl border border-[var(--color-border-muted)] shadow-xs select-text whitespace-pre-line"
                >
                  {activeSource.abstractText || (
                    <span className="text-[var(--color-fg-subtle)] italic">
                      No abstract text stored for this record.
                    </span>
                  )}
                </div>
              </section>

              {/* Anchored Passages Evidence Section */}
              <section className="space-y-3 pt-2">
                <div className="flex items-center gap-2">
                  <span className="text-xs font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)]">
                    Anchored Passages ({anchors.length})
                  </span>
                  <div className="h-px bg-[var(--color-border-muted)] flex-1" />
                </div>

                {loadingAnchors ? (
                  <div className="text-xs text-[var(--color-fg-subtle)] py-2">Loading anchors…</div>
                ) : anchors.length === 0 ? (
                  <div className="rounded-xl border border-dashed border-[var(--color-border-muted)] p-4 text-center text-xs text-[var(--color-fg-subtle)]">
                    No passage anchors saved yet. Highlight any key excerpt from the abstract above to anchor evidence.
                  </div>
                ) : (
                  <div className="space-y-2">
                    {anchors.map((anchor) => (
                      <div
                        key={anchor.id}
                        className="rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-3 text-xs space-y-1.5"
                      >
                        <div className="flex items-center justify-between text-[10px] text-[var(--color-fg-subtle)] font-mono">
                          <span className="text-teal-400 font-semibold">{anchor.sectionTitle || 'Abstract'} [+{anchor.startOffset}..{anchor.endOffset}]</span>
                          <span>hash: {anchor.passageHash?.slice(0, 12)}…</span>
                        </div>
                        <p className="text-[var(--color-editor-fg)] italic bg-[var(--color-surface-2)]/60 p-2.5 rounded-lg border border-[var(--color-border-muted)] leading-relaxed">
                          "{anchor.exactText}"
                        </p>
                      </div>
                    ))}
                  </div>
                )}
              </section>
            </div>
          )}
        </div>
      </div>

      {/* ── Add Source Modal ── */}
      {isAddSourceOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="w-full max-w-lg rounded-xl border border-[var(--color-border-default)] bg-[var(--color-canvas)] p-5 shadow-2xl space-y-4">
            <div className="flex items-center justify-between border-b border-[var(--color-border-muted)] pb-3">
              <h3 className="text-sm font-semibold text-[var(--color-editor-fg)] flex items-center gap-2">
                <BookOpen size={16} className="text-teal-400" />
                Add Literature Source to Corpus
              </h3>
              <button
                type="button"
                onClick={() => setIsAddSourceOpen(false)}
                className="text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
              >
                <X size={16} />
              </button>
            </div>

            <div className="space-y-3 text-xs">
              <div>
                <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
                  Paper Title *
                </label>
                <input
                  type="text"
                  value={newTitle}
                  onChange={(e) => setNewTitle(e.target.value)}
                  placeholder="e.g. Attention Is All You Need"
                  className="w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] outline-none"
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
                    DOI (optional)
                  </label>
                  <input
                    type="text"
                    value={newDoi}
                    onChange={(e) => setNewDoi(e.target.value)}
                    placeholder="10.1145/..."
                    className="w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] outline-none"
                  />
                </div>
                <div>
                  <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
                    Publication Year
                  </label>
                  <input
                    type="number"
                    value={newYear}
                    onChange={(e) => setNewYear(e.target.value)}
                    className="w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] outline-none"
                  />
                </div>
              </div>

              <div>
                <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
                  Authors (comma-separated)
                </label>
                <input
                  type="text"
                  value={newAuthors}
                  onChange={(e) => setNewAuthors(e.target.value)}
                  placeholder="Vaswani, Shazeer, Parmar, et al."
                  className="w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] outline-none"
                />
              </div>

              <div>
                <label className="block text-[11px] font-medium text-[var(--color-fg-muted)] mb-1">
                  Abstract Text
                </label>
                <textarea
                  rows={4}
                  value={newAbstract}
                  onChange={(e) => setNewAbstract(e.target.value)}
                  placeholder="Paste abstract for passage grounding…"
                  className="w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] outline-none"
                />
              </div>
            </div>

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-[var(--color-border-muted)]">
              <button
                type="button"
                onClick={() => setIsAddSourceOpen(false)}
                className="px-3 py-1.5 text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleCreateSource}
                disabled={!newTitle.trim()}
                className="rounded-lg bg-teal-600 px-4 py-1.5 text-xs font-medium text-white hover:bg-teal-500 disabled:opacity-50 transition"
              >
                Add to Corpus
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default LiteraturePane;
