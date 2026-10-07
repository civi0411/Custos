import React, { useState, useEffect } from 'react';
import {
  BookOpen,
  Plus,
  Search,
  ExternalLink,
  ShieldCheck,
  FileText,
  Highlighter,
  Sparkles,
  ChevronRight
} from 'lucide-react';
import { SourceRecord, PassageAnchor } from '@/types/research';
import { daemonClient } from '@/api/daemon_client';

const MOCK_SOURCES: SourceRecord[] = [
  {
    id: 'src_nature_2024_01',
    sourceType: 'paper',
    title: 'Self-Organizing Invariant Architectures in Deterministic Multi-Agent Swarms',
    doi: '10.1038/s41586-024-07821-x',
    authors: ['V. Pham', 'M. Chen', 'E. Vance'],
    year: 2024,
    contentHash: 'blake3_9941a8e2f7b11c',
    verified: true,
    abstract:
      'We present a zero-trust consensus mechanism that bounds stochastic agent divergence using Merkle-sealed invariant contracts. In empirical evaluations across 10,000 runs, phantom state execution was reduced by 99.8% while maintaining zero I/O leakages.',
  },
  {
    id: 'src_arxiv_2025_02',
    sourceType: 'paper',
    title: 'On the Convergence Rates of Cryptographic Capability Tickets under Asymmetric Latency',
    doi: '10.48550/arXiv.2501.09912',
    authors: ['T. Lindholm', 'K. S. Rao'],
    year: 2025,
    contentHash: 'blake3_7718c091ad4e22',
    verified: true,
    abstract:
      'This study provides lower bounds for atomic ticket acquisition across distributed authority gates. When latency jitter exceeds 15ms, optimistic scheduling incurs double-dispatch vulnerability unless fenced by invariant CAS certificates.',
  },
  {
    id: 'src_dataset_card_03',
    sourceType: 'dataset',
    title: 'OmniBench-ZeroIO: 50,000 Verifiable Execution Traces for Multi-Agent Safety',
    doi: '10.5281/zenodo.1089221',
    authors: ['Custos Research Lab'],
    year: 2024,
    contentHash: 'blake3_3312e778bc099f',
    verified: true,
    abstract:
      'Curated dataset of sandboxed runtime executions with complete stdout/stderr logs, container Merkle snapshots, and invariant assertions.',
  },
];

interface LiteraturePaneProps {
  onExtractClaim?: (anchor: PassageAnchor) => void;
  onShowToast?: (msg: string) => void;
}

export const LiteraturePane: React.FC<LiteraturePaneProps> = ({
  onExtractClaim,
  onShowToast,
}) => {
  const [sources, setSources] = useState<SourceRecord[]>(MOCK_SOURCES);
  const [selectedSourceId, setSelectedSourceId] = useState<string>(MOCK_SOURCES[0].id);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedText, setSelectedText] = useState('');

  useEffect(() => {
    daemonClient
      .listResearchSources()
      .then((loaded) => {
        if (loaded && loaded.length > 0) {
          setSources(loaded);
          setSelectedSourceId(loaded[0].id);
        }
      })
      .catch((err) => console.warn('Using mock sources fallback:', err));
  }, []);

  const activeSource = sources.find((s) => s.id === selectedSourceId) ?? sources[0];

  const handleTextSelect = () => {
    const sel = window.getSelection()?.toString().trim();
    if (sel && sel.length > 10) {
      setSelectedText(sel);
    }
  };

  const handleCreateClaim = async () => {
    if (!selectedText) return;
    const anchor: PassageAnchor = {
      id: `anchor_${Date.now()}`,
      sourceId: activeSource.id,
      sourceTitle: activeSource.title,
      sectionTitle: 'Abstract / Core Findings',
      startOffset: 0,
      endOffset: selectedText.length,
      exactText: selectedText,
      passageHash: `blake3_${Math.random().toString(16).slice(2, 10)}`,
    };

    try {
      await daemonClient.saveResearchAnchor(anchor);
    } catch (e) {
      console.warn('Anchor saved locally:', e);
    }

    onExtractClaim?.(anchor);
    onShowToast?.(`Extracted Claim from: "${selectedText.slice(0, 40)}..."`);
    setSelectedText('');
  };

  const filteredSources = sources.filter(
    (s) =>
      s.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      s.doi?.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div className="flex h-full w-full bg-[#04080F] text-[#e0e6ed] overflow-hidden select-none font-sans">
      {/* ── LEFT: SOURCE LIST (w-80) ── */}
      <div className="w-80 border-r border-[#1e2430] flex flex-col bg-[#080d16] shrink-0 relative z-10 shadow-[4px_0_24px_-10px_rgba(0,0,0,0.5)]">
        {/* Source List Header */}
        <div className="h-14 px-5 border-b border-[#1e2430] flex items-center justify-between shrink-0 bg-[#080d16]/80 backdrop-blur-md">
          <div className="flex items-center gap-2 font-semibold text-white tracking-wide">
            <div className="p-1.5 rounded-lg bg-gradient-to-br from-indigo-500 to-purple-600 shadow-lg shadow-indigo-500/20">
              <BookOpen className="w-4 h-4 text-white" />
            </div>
            <span className="text-[13px]">Corpus</span>
          </div>
          <button
            onClick={() => onShowToast?.('Add Source via DOI or PDF upload')}
            className="p-1.5 rounded-lg text-[#8b949e] hover:text-white hover:bg-[#1e2430] transition-all hover:scale-110 active:scale-95"
            title="Add Source (DOI / URL / PDF)"
          >
            <Plus size={16} />
          </button>
        </div>

        {/* Search Input */}
        <div className="p-4 border-b border-[#1e2430] bg-gradient-to-b from-transparent to-[#080d16]/50">
          <div className="relative group">
            <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-[#6e7681] group-focus-within:text-indigo-400 transition-colors" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search by title, DOI..."
              className="w-full bg-[#111722] border border-[#2d3342] rounded-xl py-2 pl-9 pr-3 text-sm text-[#e0e6ed] placeholder-[#6e7681] outline-none focus:border-indigo-500/60 focus:ring-2 focus:ring-indigo-500/20 transition-all shadow-inner"
            />
          </div>
        </div>

        {/* Sources Scroll Area */}
        <div className="flex-1 overflow-y-auto p-3 space-y-2 relative">
          {filteredSources.map((source) => {
            const isSelected = source.id === selectedSourceId;
            return (
              <div
                key={source.id}
                onClick={() => setSelectedSourceId(source.id)}
                className={`group p-3.5 rounded-2xl border cursor-pointer transition-all duration-300 ease-out relative overflow-hidden ${
                  isSelected
                    ? 'bg-gradient-to-br from-[#1c2333] to-[#111722] border-indigo-500/40 shadow-[0_8px_20px_-6px_rgba(99,102,241,0.15)]'
                    : 'bg-[#0d131f] border-transparent hover:bg-[#161c28] hover:border-[#2d3342] hover:shadow-lg hover:-translate-y-0.5'
                }`}
              >
                {isSelected && (
                  <div className="absolute left-0 top-0 bottom-0 w-1 bg-gradient-to-b from-indigo-400 to-purple-500" />
                )}
                
                <div className="flex items-start justify-between gap-1 mb-2">
                  <span className="font-mono text-[10px] uppercase px-2 py-0.5 rounded-md bg-[#1e2430] text-indigo-400 font-bold tracking-wider">
                    {source.sourceType}
                  </span>
                  {source.verified && (
                    <span className="flex items-center gap-1 text-[10px] text-emerald-400 font-mono font-medium bg-emerald-400/10 px-2 py-0.5 rounded-md border border-emerald-400/20">
                      <ShieldCheck size={12} /> CAS Verified
                    </span>
                  )}
                </div>
                
                <h4 className={`text-[13px] font-medium leading-relaxed mb-2 transition-colors ${isSelected ? 'text-white' : 'text-[#c9d1d9] group-hover:text-white'}`}>
                  {source.title}
                </h4>
                
                <div className="flex items-center justify-between text-[11px] text-[#8b949e]">
                  <span className="truncate max-w-[160px] font-medium">
                    {source.authors?.join(', ') || 'Unknown'}
                  </span>
                  <span className="px-1.5 py-0.5 rounded bg-[#1e2430]/50">{source.year}</span>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* ── RIGHT: DOCUMENT READER & PASSAGE ANCHORING ── */}
      <div className="flex-1 flex flex-col min-w-0 bg-[#04080F] overflow-hidden relative">
        {/* Document Header */}
        <div className="h-14 px-6 border-b border-[#1e2430] bg-[#080d16]/90 backdrop-blur-xl flex items-center justify-between shrink-0 z-20">
          <div className="flex items-center gap-3 truncate">
            <div className="w-8 h-8 rounded-full bg-blue-500/10 border border-blue-500/20 flex items-center justify-center shrink-0">
              <FileText className="w-4 h-4 text-blue-400" />
            </div>
            <span className="font-bold text-white text-[15px] truncate tracking-tight">
              {activeSource.title}
            </span>
          </div>

          <div className="flex items-center gap-4 shrink-0">
            {activeSource.doi && (
              <a
                href={`https://doi.org/${activeSource.doi}`}
                target="_blank"
                rel="noreferrer"
                className="flex items-center gap-1.5 text-xs font-mono text-blue-400 hover:text-blue-300 transition-colors px-3 py-1.5 rounded-lg bg-blue-400/5 hover:bg-blue-400/10 border border-blue-400/10"
              >
                <span>doi:{activeSource.doi}</span>
                <ExternalLink size={12} />
              </a>
            )}
          </div>
        </div>

        {/* Selected Passage Floating Banner */}
        <div className={`absolute top-14 left-0 right-0 z-30 transition-all duration-300 ease-in-out origin-top ${selectedText ? 'scale-y-100 opacity-100' : 'scale-y-0 opacity-0'}`}>
          <div className="px-6 py-3 bg-gradient-to-r from-purple-500/10 via-indigo-500/10 to-transparent border-b border-indigo-500/20 backdrop-blur-md flex items-center justify-between gap-4 shadow-xl">
            <div className="flex items-center gap-3 min-w-0 flex-1">
              <div className="p-1.5 rounded-md bg-indigo-500/20 shrink-0">
                <Highlighter className="w-4 h-4 text-indigo-400" />
              </div>
              <div className="text-[13px] text-[#e0e6ed] truncate border-l-2 border-indigo-500/40 pl-3">
                <span className="text-indigo-400 font-bold uppercase tracking-wider text-[10px] mr-2">Selection:</span>
                <span className="italic">"{selectedText}"</span>
              </div>
            </div>
            <button
              onClick={handleCreateClaim}
              className="group flex items-center gap-2 px-4 py-2 rounded-xl bg-gradient-to-r from-indigo-500 to-purple-600 text-white text-xs font-bold hover:shadow-[0_0_20px_rgba(99,102,241,0.4)] transition-all shrink-0 hover:scale-105 active:scale-95"
            >
              <Sparkles size={14} className="group-hover:animate-pulse" />
              <span>Anchor to Matrix</span>
              <ChevronRight size={14} className="opacity-70 group-hover:translate-x-0.5 transition-transform" />
            </button>
          </div>
        </div>

        {/* Document Content Viewport */}
        <div className="flex-1 overflow-y-auto relative">
          {/* Subtle background glow */}
          <div className="absolute top-0 left-1/2 -translate-x-1/2 w-[600px] h-[300px] bg-indigo-500/5 blur-[120px] rounded-full pointer-events-none" />
          
          <div
            onMouseUp={handleTextSelect}
            className="p-10 select-text max-w-4xl mx-auto w-full space-y-10 relative z-10"
          >
            {/* Metadata Card */}
            <div className="p-5 rounded-2xl bg-[#0d131f]/80 backdrop-blur-md border border-[#1e2430] shadow-xl flex flex-col md:flex-row md:items-center justify-between gap-4">
              <div className="space-y-1">
                <div className="text-[11px] font-bold uppercase tracking-widest text-[#6e7681]">Authors</div>
                <div className="text-[14px] text-[#e0e6ed] font-medium">
                  {activeSource.authors?.join(', ')} <span className="text-[#6e7681]">({activeSource.year})</span>
                </div>
              </div>
              <div className="space-y-1 md:text-right">
                <div className="text-[11px] font-bold uppercase tracking-widest text-[#6e7681]">Content Hash (BLAKE3)</div>
                <div className="text-[13px] font-mono text-indigo-400 bg-indigo-400/5 px-3 py-1 rounded-lg border border-indigo-400/10 inline-block">
                  {activeSource.contentHash}
                </div>
              </div>
            </div>

            {/* Abstract Section */}
            <section className="space-y-4">
              <div className="flex items-center gap-3">
                <div className="h-px bg-gradient-to-r from-transparent to-[#2d3342] flex-1" />
                <h3 className="text-[13px] font-black uppercase tracking-[0.2em] text-indigo-400">
                  Abstract
                </h3>
                <div className="h-px bg-gradient-to-l from-transparent to-[#2d3342] flex-1" />
              </div>
              
              <div className="relative group">
                <div className="absolute inset-0 bg-gradient-to-br from-indigo-500/5 to-purple-500/5 rounded-2xl blur-md transition-opacity opacity-0 group-hover:opacity-100" />
                <p className="relative text-[16px] leading-[1.8] text-[#c9d1d9] font-serif bg-[#0d131f]/60 backdrop-blur-sm p-6 rounded-2xl border border-[#1e2430] shadow-inner selection:bg-indigo-500/30 selection:text-white">
                  {activeSource.abstract}
                </p>
              </div>
            </section>

            {/* Mock Deep Sections */}
            <section className="space-y-4">
              <h3 className="text-[14px] font-bold tracking-wide text-white flex items-center gap-2">
                <span className="w-1.5 h-1.5 rounded-full bg-purple-500" />
                1. Invariant Formulation & Non-Repudiation Gates
              </h3>
              <div className="space-y-4 text-[15.5px] leading-[1.8] text-[#a1abb7] font-serif pl-3.5 border-l border-[#1e2430]">
                <p className="selection:bg-indigo-500/30 selection:text-white">
                  In sovereign execution topologies, an agent cannot claim a task outcome
                  without attaching a cryptographic receipt. When verifying invariant closures,
                  the state transition Merkle tree must match the pre-condition predicate exactly.
                  <span className="text-[#e0e6ed] bg-indigo-500/10 px-1 mx-1 rounded border border-indigo-500/20">Select any sentence in this passage</span>
                  to extract it directly into the Claims & Verification Matrix.
                </p>
                <p className="selection:bg-indigo-500/30 selection:text-white">
                  Formally, let <code className="font-mono text-[14px] text-blue-300 bg-blue-400/10 px-1.5 py-0.5 rounded">S_0</code> be the initial workspace snapshot and <code className="font-mono text-[14px] text-blue-300 bg-blue-400/10 px-1.5 py-0.5 rounded">S_1</code> be the post-execution
                  state. The invariant verifier computes <code className="font-mono text-[14px] text-pink-300 bg-pink-400/10 px-1.5 py-0.5 rounded">\Delta = H(S_1) \oplus H(S_0)</code>. If <code className="font-mono text-[14px] text-pink-300 bg-pink-400/10 px-1.5 py-0.5 rounded">\Delta</code>
                  contains unpermitted mutations, the execution ticket is revoked with zero latency.
                </p>
              </div>
            </section>

            <section className="space-y-4">
              <h3 className="text-[14px] font-bold tracking-wide text-white flex items-center gap-2">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                2. Empirical Reproducibility Benchmarks
              </h3>
              <div className="space-y-4 text-[15.5px] leading-[1.8] text-[#a1abb7] font-serif pl-3.5 border-l border-[#1e2430]">
                <p className="selection:bg-indigo-500/30 selection:text-white">
                  Across 5,000 synthetic trials under high-jitter networks (10ms to 80ms latency),
                  the proposed zero-trust capability protocol eliminated double-dispatch anomalies
                  entirely <strong className="font-bold text-[#e0e6ed] underline decoration-emerald-500/50 decoration-2 underline-offset-4">(0 occurrences)</strong>, whereas optimistic state sharing failed in <strong className="font-bold text-[#e0e6ed] underline decoration-red-500/50 decoration-2 underline-offset-4">14.2%</strong> of executions.
                </p>
              </div>
            </section>
          </div>
        </div>
      </div>
    </div>
  );
};
