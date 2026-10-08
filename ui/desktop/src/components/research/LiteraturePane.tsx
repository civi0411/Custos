import React, { useState, useEffect } from 'react';
import {
  BookOpen,
  Search,
  ExternalLink,
  ShieldCheck,
  Plus,
  FileText,
  Highlighter
} from 'lucide-react';
import { SourceRecord, PassageAnchor } from '@/types/research';
import { daemonClient } from '@/api/daemon_client';

interface LiteraturePaneProps {
  onShowToast?: (msg: string) => void;
}

export const LiteraturePane: React.FC<LiteraturePaneProps> = ({ onShowToast }) => {
  const [sources, setSources] = useState<SourceRecord[]>([]);
  const [selectedSourceId, setSelectedSourceId] = useState<string>('');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedText, setSelectedText] = useState('');
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    daemonClient
      .listResearchSources()
      .then((loaded) => {
        setSources(loaded);
        setSelectedSourceId(loaded[0]?.id ?? '');
      })
      .catch((err) => setLoadError(String(err)));
  }, []);

  const activeSource = sources.find((s) => s.id === selectedSourceId) ?? sources[0];

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
      const passageHash = Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('');
      const anchor: PassageAnchor = {
        id: crypto.randomUUID(),
        sourceId: activeSource.id,
        sourceTitle: activeSource.title,
        sectionTitle: 'Abstract',
        startOffset,
        endOffset: startOffset + selectedText.length,
        exactText: selectedText,
        passageHash
      };
      await daemonClient.saveResearchAnchor(anchor);
      onShowToast?.('Passage saved. Claim support has not been assessed.');
      setSelectedText('');
    } catch (err) {
      onShowToast?.(`Could not save passage: ${String(err)}`);
    }
  };

  const filteredSources = sources.filter(
    (s) =>
      s.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      s.authors?.some((a: string) => a.toLowerCase().includes(searchQuery.toLowerCase())) ||
      s.doi?.toLowerCase().includes(searchQuery.toLowerCase())
  );

  if (!activeSource) {
    return <div className="p-6 text-xs text-fg-subtle">{loadError ? `Research sources unavailable: ${loadError}` : 'No research sources yet.'}</div>;
  }

  return (
    <div className="flex h-full w-full bg-canvas text-fg-editor overflow-hidden select-none font-sans">
      {/* ── LEFT: SOURCE LIST (w-80) ── */}
      <div className="w-80 border-r border-border-muted flex flex-col bg-surface-1 shrink-0 relative z-10">
        {/* Source List Header */}
        <div className="h-11 px-4 border-b border-border-muted flex items-center justify-between shrink-0 bg-surface-1">
          <div className="flex items-center gap-2">
            <BookOpen className="w-4 h-4 workbench-accent" />
            <span className="text-xs font-semibold text-fg-editor">Sources</span>
            <span className="workbench-kicker">Corpus</span>
          </div>
          <button
            onClick={() => onShowToast?.('Add Source via DOI or PDF upload')}
            className="p-1 rounded-md text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition"
            title="Add Source (DOI / URL / PDF)"
          >
            <Plus size={15} />
          </button>
        </div>

        {/* Search Input */}
        <div className="p-3 border-b border-border-muted bg-surface-1">
          <div className="relative group">
            <Search className="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-fg-subtle group-focus-within:text-fg-editor transition-colors" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search by title, DOI..."
              className="w-full bg-canvas-inset border border-border-default rounded-lg py-1.5 pl-8 pr-3 text-xs text-fg-editor placeholder-fg-subtle outline-none focus:border-border-default focus:ring-1 focus:ring-border-default transition"
            />
          </div>
        </div>

        {/* Sources Scroll Area */}
        <div className="flex-1 overflow-y-auto p-2.5 space-y-1.5 relative no-scrollbar">
          {filteredSources.map((source) => {
            const isSelected = source.id === selectedSourceId;
            return (
              <div
                key={source.id}
                onClick={() => {
                  setSelectedSourceId(source.id);
                  setSelectedText('');
                }}
                className={`group p-3 rounded-xl border cursor-pointer transition text-xs ${
                  isSelected
                    ? 'bg-surface-2 border-border-default text-fg-editor shadow-xs'
                    : 'bg-transparent border-transparent hover:bg-surface-2/60 hover:border-border-muted/50 text-fg-muted hover:text-fg-editor'
                }`}
              >
                <div className="flex items-start justify-between gap-1 mb-1.5">
                  <span className="font-mono text-[10px] uppercase px-1.5 py-0.5 rounded bg-surface-3 text-fg-muted font-medium">
                    {source.sourceType}
                  </span>
                  {source.verified && (
                    <span className="flex items-center gap-1 text-[10px] text-emerald-400 font-mono font-medium bg-emerald-500/10 px-1.5 py-0.5 rounded border border-emerald-500/20">
                      <ShieldCheck size={11} /> Checked
                    </span>
                  )}
                </div>
                
                <h4 className="text-[12.5px] font-medium leading-snug mb-1.5 text-fg-editor">
                  {source.title}
                </h4>
                
                <div className="flex items-center justify-between text-[11px] text-fg-subtle">
                  <span className="truncate max-w-[150px]">
                    {source.authors?.join(', ') || 'Unknown'}
                  </span>
                  <span className="font-mono">{source.year}</span>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* ── RIGHT: DOCUMENT READER & PASSAGE ANCHORING ── */}
      <div className="flex-1 flex flex-col min-w-0 bg-canvas overflow-hidden relative">
        {/* Document Header */}
        <div className="h-11 px-5 border-b border-border-muted bg-surface-1 flex items-center justify-between shrink-0 z-20">
          <div className="flex items-center gap-2.5 truncate">
            <FileText className="w-4 h-4 workbench-accent shrink-0" />
            <span className="font-semibold text-fg-editor text-[13px] truncate">
              {activeSource.title}
            </span>
          </div>

          <div className="flex items-center gap-3 shrink-0">
            {activeSource.doi && (
              <a
                href={`https://doi.org/${activeSource.doi}`}
                target="_blank"
                rel="noreferrer"
                className="flex items-center gap-1.5 text-[11px] font-mono text-fg-muted hover:text-fg-editor transition px-2.5 py-1 rounded-md bg-surface-2 border border-border-default"
              >
                <span>doi:{activeSource.doi}</span>
                <ExternalLink size={11} />
              </a>
            )}
          </div>
        </div>

        {/* Selected Passage Floating Banner */}
        {selectedText && (
          <div className="px-5 py-2.5 bg-surface-2 border-b border-border-default flex items-center justify-between gap-4 z-30">
            <div className="flex items-center gap-2.5 min-w-0 flex-1">
              <Highlighter className="w-3.5 h-3.5 workbench-accent shrink-0" />
              <div className="text-xs text-fg-editor truncate">
                <span className="font-semibold uppercase tracking-wider text-[10px] mr-2 text-fg-muted">Selected:</span>
                <span className="italic">"{selectedText}"</span>
              </div>
            </div>
            <button
              onClick={handleSavePassage}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-surface-3 hover:bg-surface-2 border border-border-default text-fg-editor text-xs font-medium transition shrink-0"
            >
              <span>Save passage</span>
            </button>
          </div>
        )}

        {/* Document Content Viewport */}
        <div className="flex-1 overflow-y-auto relative no-scrollbar">
          <div
            className="p-8 select-text max-w-3xl mx-auto w-full space-y-8 relative z-10 font-sans"
          >
            {/* Metadata Card */}
            <div className="p-4 rounded-xl bg-surface-1 border border-border-muted flex flex-col md:flex-row md:items-center justify-between gap-3 text-xs">
              <div className="space-y-0.5">
                <div className="text-[10px] font-semibold uppercase tracking-wider text-fg-subtle">Authors</div>
                <div className="text-fg-editor font-medium">
                  {activeSource.authors?.join(', ')} <span className="text-fg-subtle">({activeSource.year})</span>
                </div>
              </div>
              <div className="space-y-0.5 md:text-right">
                <div className="text-[10px] font-semibold uppercase tracking-wider text-fg-subtle">Digest (BLAKE3)</div>
                <div className="font-mono text-[11px] text-fg-muted bg-surface-2 px-2 py-0.5 rounded border border-border-muted inline-block">
                  {activeSource.contentHash}
                </div>
              </div>
            </div>

            {/* Abstract Section */}
            <section className="space-y-3">
              <div className="flex items-center gap-2">
                <span className="workbench-kicker">Abstract</span>
                <div className="h-px bg-border-muted flex-1" />
              </div>
              
              <p onMouseUp={handleTextSelect} className="text-[14px] leading-relaxed text-fg-editor bg-surface-1/50 p-5 rounded-xl border border-border-muted">
                {activeSource.abstractText}
              </p>
            </section>

          </div>
        </div>
      </div>
    </div>
  );
};

export default LiteraturePane;
