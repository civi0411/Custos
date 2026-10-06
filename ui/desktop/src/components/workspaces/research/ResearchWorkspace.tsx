import React, { useState } from 'react';
import { 
  BookOpen, 
  Search, 
  ExternalLink, 
  FileText, 
  Sparkles, 
  Bookmark, 
  Plus, 
  CheckCircle2, 
  HelpCircle, 
  XCircle, 
  FlaskConical, 
  Copy, 
  Check, 
  ShieldCheck, 
  Activity
} from 'lucide-react';

interface PaperItem {
  id: string;
  title: string;
  authors: string;
  source: string;
  year: number;
  citations: number;
  doi: string;
  abstract: string;
  category: 'Multi-Agent' | 'Systems' | 'Genomics' | 'Safety';
  claimsCount: number;
  keyFinding: string;
}

export interface ClaimItem {
  id: string;
  title: string;
  statement: string;
  status: 'supported' | 'unknown' | 'contradicted';
  locator: string;
  sourcePaperId: string;
  sourceTitle: string;
  evidenceCasHash?: string;
  mappedInvariant?: string;
  pScore?: number;
}

interface ExperimentTrial {
  id: string;
  name: string;
  parameters: string;
  reproducibility: number;
  status: 'passed' | 'running' | 'diverged';
  duration: string;
}

const MOCK_PAPERS: PaperItem[] = [
  {
    id: 'arxiv-2403.1189',
    title: 'Constitutional AI and Autonomous Multi-Agent Consensus in Critical Systems',
    authors: 'A. Vaswani, D. Silver, et al.',
    source: 'arXiv:2403.1189 [cs.AI]',
    year: 2025,
    citations: 142,
    doi: '10.48550/arXiv.2403.1189',
    category: 'Multi-Agent',
    claimsCount: 3,
    keyFinding: 'Demonstrates that append-only non-repudiation cryptographic ledgers eliminate phantom distributed state drift in multi-agent tool execution.',
    abstract: 'We explore automated verification mechanisms for LLM agent chains utilizing non-repudiation cryptographic ledgers and self-correcting feedback loops. In high-stakes autonomous execution, single-point authorization is insufficient. We introduce a dual-consensus permit protocol where agents must acquire cryptographic tickets before invoking filesystem or network mutations, providing 99.98% audit completeness.'
  },
  {
    id: 'acm-2024.cache',
    title: 'Adaptive Semantic Caching with Sub-millisecond Vector Indexes in Edge Runtimes',
    authors: 'M. Stonebraker, J. Dean, et al.',
    source: 'Proc. ACM SIGMOD 2024',
    year: 2024,
    citations: 215,
    doi: '10.1145/3626246.3653372',
    category: 'Systems',
    claimsCount: 2,
    keyFinding: 'Demonstrating a 78% reduction in token consumption and 4.2x speedup across heterogeneous multi-turn conversational agents with zero loss in output correctness.',
    abstract: 'Edge-deployed LLM agents incur heavy latency and financial penalties from redundant prompt evaluation. We present an adaptive vector index caching architecture capable of sub-millisecond semantic similarity lookups across cached turn hierarchies. In multi-agent pipelines, 78% of context window tokens are safely reused under strict TTL bounds.'
  },
  {
    id: 'biorxiv-2024.992',
    title: 'High-Throughput Deep Mutational Scanning Using Deep Generative Priors',
    authors: 'E. Lander, J. Doudna, et al.',
    source: 'bioRxiv 10.1101/2024.992',
    year: 2025,
    citations: 89,
    doi: '10.1101/2024.03.14.584992',
    category: 'Genomics',
    claimsCount: 2,
    keyFinding: 'AlphaFold 3 structural embeddings predict clinical pathogenicity with AUROC 0.94 across 150,000 human missense variants.',
    abstract: 'Systematic characterization of missense mutations in human tumor suppressors remains a bottleneck. Integrating AlphaFold 3 structural confidence metrics with generative sequence priors provides calibrated pathogenicity predictions without requiring physical in vitro assays for every single-nucleotide variant.'
  },
  {
    id: 'custos-mono-2025',
    title: 'Zero-IO Proof Closures in Local-First Agent Supervision Environments',
    authors: 'Custos Research Group',
    source: 'Custos Technical Monograph',
    year: 2025,
    citations: 34,
    doi: 'custos://doi/2025.01.sade',
    category: 'Safety',
    claimsCount: 4,
    keyFinding: 'Formal proof that mediated authority ticket gates prevent rogue write operations without sacrificing LLM developer ergonomics.',
    abstract: 'We formalize Supervised Agent Development Environments (SADE). By decoupling the conversation spine from capability permits and enforcing invariant proof closures, agents operate with high autonomy while catastrophic mutations are fenced deterministically.'
  }
];

const INITIAL_CLAIMS: ClaimItem[] = [
  {
    id: 'CLM-01',
    title: 'Atomic Double-Dispatch Fencing',
    statement: 'Acquiring an exclusive permit ticket prior to action dispatch guarantees zero concurrent execution races across distributed agent workers.',
    status: 'supported',
    locator: 'arXiv:2403.1189 [p.4 §3.2]',
    sourcePaperId: 'arxiv-2403.1189',
    sourceTitle: 'Constitutional AI and Multi-Agent Consensus',
    evidenceCasHash: 'cas://bafy2bzace4v3k99apermitfenced01',
    mappedInvariant: 'INV-01',
    pScore: 0.998
  },
  {
    id: 'CLM-02',
    title: 'Semantic Cache Correctness Invariant',
    statement: 'Sub-millisecond cosine similarity caching on prompt prefixes above 0.96 cosine threshold yields zero semantic drift in generated Rust code.',
    status: 'supported',
    locator: 'ACM SIGMOD 2024 [p.8 §5.1]',
    sourcePaperId: 'acm-2024.cache',
    sourceTitle: 'Adaptive Semantic Caching',
    evidenceCasHash: 'cas://bafy2bzace99q87cacheverify02',
    mappedInvariant: 'INV-04',
    pScore: 0.974
  },
  {
    id: 'CLM-03',
    title: 'Autonomous Mutex Lease Timeout Bounds',
    statement: 'A hard 30-second TTL on capability leases prevents deadlocks during ungraceful daemon restarts without interrupting long-running code generation.',
    status: 'unknown',
    locator: 'Custos Monograph [p.11 §4.3]',
    sourcePaperId: 'custos-mono-2025',
    sourceTitle: 'Zero-IO Proof Closures',
    mappedInvariant: 'INV-03',
    pScore: undefined
  },
  {
    id: 'CLM-04',
    title: 'Uncalibrated Temperature Invariance',
    statement: 'Increasing sampling temperature beyond 0.7 does not degrade formal invariant compliance when using runtime AST validation.',
    status: 'contradicted',
    locator: 'arXiv:2403.1189 [p.14 §7.4]',
    sourcePaperId: 'arxiv-2403.1189',
    sourceTitle: 'Constitutional AI and Multi-Agent Consensus',
    evidenceCasHash: 'cas://bafy2bzace001counterevidenceast',
    mappedInvariant: 'INV-02',
    pScore: 0.12
  }
];

const EXPERIMENTS: ExperimentTrial[] = [
  { id: 'exp-01', name: 'PermitGate Fencing Stress Test (1000 concurrent agents)', parameters: 'concurrency=1000, timeout=500ms', reproducibility: 100, status: 'passed', duration: '1.42s' },
  { id: 'exp-02', name: 'Semantic Cache Invalidation on Git Branch Switch', parameters: 'vector_dim=1536, threshold=0.96', reproducibility: 99.4, status: 'passed', duration: '3.18s' },
  { id: 'exp-03', name: 'Autonomous Mutex Lease Expiry under Network Partition', parameters: 'lease_ttl=30s, packet_loss=40%', reproducibility: 68.2, status: 'diverged', duration: '32.1s' }
];

export const ResearchWorkspace: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'sources' | 'claims' | 'experiments' | 'notes'>('sources');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const [selectedPaper, setSelectedPaper] = useState<PaperItem>(MOCK_PAPERS[0]);
  const [savedPapers, setSavedPapers] = useState<string[]>([MOCK_PAPERS[0].id]);
  const [claims, setClaims] = useState<ClaimItem[]>(INITIAL_CLAIMS);
  const [claimFilter, setClaimFilter] = useState<'all' | 'supported' | 'unknown' | 'contradicted'>('all');
  
  const [copiedNote, setCopiedNote] = useState(false);
  const [transferredToTask, setTransferredToTask] = useState(false);

  const [notes, setNotes] = useState<string>(
`# Custos SADE Research Synthesis & Invariant Contracts
*Derived from Literature Corpus & Evidence Ledger (INV-01, INV-03)*

## 1. Verified Core Claims
- **[CLM-01 Supported]**: Atomic ticket acquisition in \`PermitGate\` prevents phantom double-dispatches across multi-agent turns. (arXiv:2403.1189 [p.4 §3.2])
- **[CLM-02 Supported]**: 78% reduction in token consumption with sub-millisecond semantic caching. (ACM SIGMOD 2024 [p.8 §5.1])

## 2. Invariant Specifications for Engineering Workbench
- \`INV-01\`: Every execution of \`claim_ready_task\` must acquire lease before file write.
- \`INV-03\`: Lease timeout must have fallback recovery handler in tokio select.
`
  );

  const toggleSave = (id: string) => {
    setSavedPapers(prev => 
      prev.includes(id) ? prev.filter(p => p !== id) : [...prev, id]
    );
  };

  const handleExtractClaim = () => {
    const newClaim: ClaimItem = {
      id: `CLM-0${claims.length + 1}`,
      title: `Extracted from ${selectedPaper.source}`,
      statement: selectedPaper.keyFinding,
      status: 'unknown',
      locator: `${selectedPaper.source} [Abstract §1]`,
      sourcePaperId: selectedPaper.id,
      sourceTitle: selectedPaper.title,
      mappedInvariant: 'INV-05'
    };
    setClaims(prev => [newClaim, ...prev]);
    setActiveTab('claims');
  };

  const handleTransferToTask = () => {
    setTransferredToTask(true);
    setTimeout(() => setTransferredToTask(false), 3000);
  };

  const filteredPapers = MOCK_PAPERS.filter(p => {
    const matchesSearch = p.title.toLowerCase().includes(searchQuery.toLowerCase()) || 
                          p.authors.toLowerCase().includes(searchQuery.toLowerCase()) ||
                          p.abstract.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesCat = selectedCategory === 'all' || p.category === selectedCategory;
    return matchesSearch && matchesCat;
  });

  const filteredClaims = claims.filter(c => {
    if (claimFilter === 'all') return true;
    return c.status === claimFilter;
  });

  return (
    <div 
      className="flex-1 flex flex-col h-full overflow-hidden select-none"
      style={{
        background: 'var(--color-canvas, #0d1117)',
        color: 'var(--color-editor-fg, #e6edf3)',
        borderLeft: '1px solid var(--color-border-default, #30363d)'
      }}
    >
      {/* ── 1. Top Header Bar (Claude Science style) ── */}
      <div 
        className="h-9 px-3 flex items-center justify-between text-xs shrink-0"
        style={{
          background: 'var(--color-surface-1, #161b22)',
          borderBottom: '1px solid var(--color-border-default, #30363d)',
        }}
      >
        {/* Left: Active Corpus info */}
        <div className="flex items-center gap-2">
          <div className="p-1 rounded" style={{ background: 'rgba(88,166,255,0.15)', color: 'var(--color-research, #58a6ff)' }}>
            <FlaskConical className="w-3.5 h-3.5" />
          </div>
          <span className="font-semibold text-xs text-white">Research & Evidence Lab</span>
          <span 
            className="text-[10px] px-2 py-0.2 rounded-full font-mono hidden sm:inline"
            style={{
              background: 'rgba(88,166,255,0.1)',
              color: 'var(--color-research, #58a6ff)',
              border: '1px solid rgba(88,166,255,0.2)'
            }}
          >
            {MOCK_PAPERS.length} Sources · {claims.length} Claims Verified
          </span>
        </div>

        {/* Right: Subtabs (Literature / Claims / Experiments / Notes) */}
        <div 
          className="flex items-center p-0.5 rounded text-[11px]"
          style={{
            background: 'var(--color-surface-0, #0d1117)',
            border: '1px solid var(--color-border-muted, #21262d)',
          }}
        >
          <button
            onClick={() => setActiveTab('sources')}
            className="px-2.5 py-0.5 rounded transition font-medium flex items-center gap-1.5"
            style={{
              background: activeTab === 'sources' ? 'var(--color-surface-2, #1c2128)' : 'transparent',
              color: activeTab === 'sources' ? 'var(--color-editor-fg, #e6edf3)' : 'var(--color-fg-muted, #8b949e)',
              border: activeTab === 'sources' ? '1px solid var(--color-border-default, #30363d)' : '1px solid transparent'
            }}
          >
            <BookOpen className="w-3 h-3 text-[#58a6ff]" />
            <span>Literature</span>
          </button>

          <button
            onClick={() => setActiveTab('claims')}
            className="px-2.5 py-0.5 rounded transition font-medium flex items-center gap-1.5"
            style={{
              background: activeTab === 'claims' ? 'var(--color-surface-2, #1c2128)' : 'transparent',
              color: activeTab === 'claims' ? 'var(--color-editor-fg, #e6edf3)' : 'var(--color-fg-muted, #8b949e)',
              border: activeTab === 'claims' ? '1px solid var(--color-border-default, #30363d)' : '1px solid transparent'
            }}
          >
            <ShieldCheck className="w-3 h-3 text-[#3fb950]" />
            <span>Claim Matrix ({claims.length})</span>
          </button>

          <button
            onClick={() => setActiveTab('experiments')}
            className="px-2.5 py-0.5 rounded transition font-medium flex items-center gap-1.5"
            style={{
              background: activeTab === 'experiments' ? 'var(--color-surface-2, #1c2128)' : 'transparent',
              color: activeTab === 'experiments' ? 'var(--color-editor-fg, #e6edf3)' : 'var(--color-fg-muted, #8b949e)',
              border: activeTab === 'experiments' ? '1px solid var(--color-border-default, #30363d)' : '1px solid transparent'
            }}
          >
            <Activity className="w-3 h-3 text-[#d29922]" />
            <span>Experiments</span>
          </button>

          <button
            onClick={() => setActiveTab('notes')}
            className="px-2.5 py-0.5 rounded transition font-medium flex items-center gap-1.5"
            style={{
              background: activeTab === 'notes' ? 'var(--color-surface-2, #1c2128)' : 'transparent',
              color: activeTab === 'notes' ? 'var(--color-editor-fg, #e6edf3)' : 'var(--color-fg-muted, #8b949e)',
              border: activeTab === 'notes' ? '1px solid var(--color-border-default, #30363d)' : '1px solid transparent'
            }}
          >
            <FileText className="w-3 h-3 text-[#a78bfa]" />
            <span>Synthesis</span>
          </button>
        </div>
      </div>

      {/* ── 2. Content Area ── */}

      {/* TAB 1: LITERATURE SOURCES */}
      {activeTab === 'sources' && (
        <div className="flex-1 flex min-h-0">
          {/* Left Column: Paper Search & List */}
          <div 
            className="w-80 flex flex-col shrink-0 select-none"
            style={{
              background: 'var(--color-surface-1, #161b22)',
              borderRight: '1px solid var(--color-border-default, #30363d)',
            }}
          >
            {/* Search Input */}
            <div className="p-2.5" style={{ borderBottom: '1px solid var(--color-border-default, #30363d)' }}>
              <div className="relative">
                <Search className="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-[#6e7681]" />
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Filter papers, arXiv, DOIs..."
                  className="w-full pl-8 pr-2 py-1 text-xs rounded text-white placeholder-[#6e7681] focus:outline-none font-mono"
                  style={{
                    background: 'var(--color-canvas, #0d1117)',
                    border: '1px solid var(--color-border-default, #30363d)'
                  }}
                />
              </div>

              {/* Category Pills */}
              <div className="flex items-center gap-1 mt-2 overflow-x-auto text-[10px]">
                {['all', 'Multi-Agent', 'Systems', 'Safety'].map((cat) => (
                  <button
                    key={cat}
                    onClick={() => setSelectedCategory(cat)}
                    className="px-2 py-0.5 rounded transition shrink-0 font-medium"
                    style={{
                      background: selectedCategory === cat ? 'var(--color-research, #58a6ff)' : 'var(--color-surface-2, #1c2128)',
                      color: selectedCategory === cat ? '#ffffff' : '#8b949e',
                      border: '1px solid var(--color-border-muted, #21262d)'
                    }}
                  >
                    {cat}
                  </button>
                ))}
              </div>
            </div>

            {/* Paper Cards List */}
            <div className="flex-1 overflow-y-auto p-2 space-y-1.5">
              {filteredPapers.map((paper) => {
                const isSelected = selectedPaper.id === paper.id;
                const isSaved = savedPapers.includes(paper.id);
                return (
                  <div
                    key={paper.id}
                    onClick={() => setSelectedPaper(paper)}
                    className="p-2.5 rounded-lg cursor-pointer transition text-xs"
                    style={{
                      background: isSelected ? 'var(--color-surface-2, #1c2128)' : 'transparent',
                      border: `1px solid ${isSelected ? 'var(--color-research, #58a6ff)' : 'transparent'}`,
                      color: isSelected ? '#ffffff' : '#c9d1d9'
                    }}
                  >
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-[10px] font-mono text-[#58a6ff]">{paper.source}</span>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          toggleSave(paper.id);
                        }}
                        className="text-[#6e7681] hover:text-[#58a6ff] transition"
                      >
                        <Bookmark className={`w-3.5 h-3.5 ${isSaved ? 'fill-[#58a6ff] text-[#58a6ff]' : ''}`} />
                      </button>
                    </div>

                    <h4 className="font-semibold text-white line-clamp-2 leading-snug mb-1 text-[12px]">
                      {paper.title}
                    </h4>

                    <div className="flex items-center justify-between text-[10px] text-[#8b949e] font-mono mt-1.5">
                      <span>{paper.year} · {paper.citations} cites</span>
                      <span className="px-1.5 py-0.2 rounded" style={{ background: 'var(--color-surface-0, #0d1117)', border: '1px solid var(--color-border-muted, #21262d)' }}>
                        {paper.category}
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Right Column: Paper Deep Reader */}
          <div 
            className="flex-1 overflow-y-auto p-6 space-y-5 select-text"
            style={{ background: 'var(--color-canvas, #0d1117)' }}
          >
            {/* Header info */}
            <div className="space-y-3 pb-4" style={{ borderBottom: '1px solid var(--color-border-default, #30363d)' }}>
              <div className="flex items-center justify-between">
                <span 
                  className="text-xs font-mono px-2 py-0.5 rounded text-[#58a6ff]"
                  style={{ background: 'rgba(88,166,255,0.1)', border: '1px solid rgba(88,166,255,0.2)' }}
                >
                  {selectedPaper.source} · DOI: {selectedPaper.doi}
                </span>

                <div className="flex items-center gap-2">
                  <button 
                    onClick={handleExtractClaim}
                    className="px-2.5 py-1 rounded text-white font-medium text-xs flex items-center gap-1.5 transition"
                    style={{ background: 'var(--color-research, #58a6ff)' }}
                    title="Extract highlighted statement into Claim Matrix"
                  >
                    <Plus className="w-3.5 h-3.5" />
                    <span>Extract Claim</span>
                  </button>
                  <a
                    href={`https://doi.org/${selectedPaper.doi}`}
                    target="_blank"
                    rel="noreferrer"
                    className="px-2.5 py-1 rounded text-xs text-[#8b949e] hover:text-white flex items-center gap-1 transition"
                    style={{ background: 'var(--color-surface-2, #1c2128)', border: '1px solid var(--color-border-default, #30363d)' }}
                  >
                    <ExternalLink className="w-3.5 h-3.5" />
                    <span>Open DOI</span>
                  </a>
                </div>
              </div>

              <h1 className="text-lg font-bold text-white leading-tight">
                {selectedPaper.title}
              </h1>

              <div className="text-xs text-[#8b949e] flex items-center gap-4 font-mono">
                <span>Authors: <strong className="text-white">{selectedPaper.authors}</strong></span>
                <span>Published: {selectedPaper.year}</span>
                <span>Citations: {selectedPaper.citations}</span>
              </div>
            </div>

            {/* Key Empirical Finding Box */}
            <div 
              className="p-4 rounded-xl space-y-2"
              style={{
                background: 'rgba(88,166,255,0.06)',
                border: '1px solid rgba(88,166,255,0.25)',
              }}
            >
              <div className="flex items-center gap-2 text-xs font-semibold text-[#58a6ff]">
                <Sparkles className="w-4 h-4 text-[#58a6ff]" />
                <span>Empirical Finding & Locator</span>
              </div>
              <p className="text-xs text-white leading-relaxed font-mono">
                "{selectedPaper.keyFinding}"
              </p>
              <div className="text-[11px] text-[#8b949e] font-mono pt-1 flex items-center gap-2">
                <span>Locator: <strong className="text-white">[{selectedPaper.source} p.3 §2.1]</strong></span>
                <span>·</span>
                <span className="text-[#3fb950]">Replicated in SADE Testbench</span>
              </div>
            </div>

            {/* Abstract */}
            <div className="space-y-2">
              <h3 className="text-xs font-semibold uppercase tracking-wider text-[#8b949e]">Abstract</h3>
              <p className="text-xs text-[#c9d1d9] leading-relaxed font-sans">
                {selectedPaper.abstract}
              </p>
            </div>
          </div>
        </div>
      )}

      {/* TAB 2: SADE 3-STATE CLAIM MATRIX */}
      {activeTab === 'claims' && (
        <div className="flex-1 flex flex-col p-6 overflow-y-auto space-y-4" style={{ background: 'var(--color-canvas, #0d1117)' }}>
          {/* Header & Filter Controls */}
          <div className="flex items-center justify-between pb-3" style={{ borderBottom: '1px solid var(--color-border-default, #30363d)' }}>
            <div>
              <h2 className="text-sm font-bold text-white flex items-center gap-2">
                <ShieldCheck className="w-4 h-4 text-[#3fb950]" />
                <span>SADE Invariant Claim Matrix</span>
              </h2>
              <p className="text-xs text-[#8b949e]">
                All scientific claims linked to evidence hashes and AST invariant contracts
              </p>
            </div>

            {/* 3-State Filter Tabs */}
            <div className="flex items-center gap-1.5 font-mono text-[11px]">
              <button
                onClick={() => setClaimFilter('all')}
                className="px-2.5 py-1 rounded transition"
                style={{
                  background: claimFilter === 'all' ? 'var(--color-surface-2, #1c2128)' : 'transparent',
                  color: claimFilter === 'all' ? '#ffffff' : '#8b949e',
                  border: '1px solid var(--color-border-default, #30363d)'
                }}
              >
                All ({claims.length})
              </button>
              <button
                onClick={() => setClaimFilter('supported')}
                className="px-2.5 py-1 rounded transition flex items-center gap-1"
                style={{
                  background: claimFilter === 'supported' ? 'rgba(63,185,80,0.15)' : 'transparent',
                  color: claimFilter === 'supported' ? '#3fb950' : '#8b949e',
                  border: '1px solid rgba(63,185,80,0.3)'
                }}
              >
                <CheckCircle2 className="w-3 h-3 text-[#3fb950]" />
                <span>Supported ({claims.filter(c => c.status === 'supported').length})</span>
              </button>
              <button
                onClick={() => setClaimFilter('unknown')}
                className="px-2.5 py-1 rounded transition flex items-center gap-1"
                style={{
                  background: claimFilter === 'unknown' ? 'rgba(210,153,34,0.15)' : 'transparent',
                  color: claimFilter === 'unknown' ? '#d29922' : '#8b949e',
                  border: '1px solid rgba(210,153,34,0.3)'
                }}
              >
                <HelpCircle className="w-3 h-3 text-[#d29922]" />
                <span>Unknown ({claims.filter(c => c.status === 'unknown').length})</span>
              </button>
              <button
                onClick={() => setClaimFilter('contradicted')}
                className="px-2.5 py-1 rounded transition flex items-center gap-1"
                style={{
                  background: claimFilter === 'contradicted' ? 'rgba(248,81,73,0.15)' : 'transparent',
                  color: claimFilter === 'contradicted' ? '#f85149' : '#8b949e',
                  border: '1px solid rgba(248,81,73,0.3)'
                }}
              >
                <XCircle className="w-3 h-3 text-[#f85149]" />
                <span>Contradicted ({claims.filter(c => c.status === 'contradicted').length})</span>
              </button>
            </div>
          </div>

          {/* Claims List Table */}
          <div className="space-y-3">
            {filteredClaims.map((claim) => {
              const statusBg = claim.status === 'supported' ? 'rgba(63,185,80,0.1)' : claim.status === 'unknown' ? 'rgba(210,153,34,0.1)' : 'rgba(248,81,73,0.1)';
              const statusColor = claim.status === 'supported' ? '#3fb950' : claim.status === 'unknown' ? '#d29922' : '#f85149';
              const StatusIcon = claim.status === 'supported' ? CheckCircle2 : claim.status === 'unknown' ? HelpCircle : XCircle;

              return (
                <div 
                  key={claim.id}
                  className="p-4 rounded-xl space-y-2 select-text"
                  style={{
                    background: 'var(--color-surface-1, #161b22)',
                    border: '1px solid var(--color-border-default, #30363d)'
                  }}
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <span className="font-mono text-xs text-[#8b949e] font-bold">{claim.id}</span>
                      <span className="font-semibold text-white text-sm">{claim.title}</span>
                      {claim.mappedInvariant && (
                        <span 
                          className="font-mono text-[10px] px-1.5 py-0.2 rounded font-bold"
                          style={{ background: 'rgba(88,166,255,0.15)', color: '#58a6ff', border: '1px solid rgba(88,166,255,0.3)' }}
                        >
                          {claim.mappedInvariant}
                        </span>
                      )}
                    </div>

                    {/* State Badge with text + dot */}
                    <div 
                      className="px-2.5 py-0.5 rounded-full font-mono text-[11px] font-semibold flex items-center gap-1.5"
                      style={{ background: statusBg, color: statusColor, border: `1px solid ${statusColor}40` }}
                    >
                      <StatusIcon className="w-3.5 h-3.5" />
                      <span className="uppercase">{claim.status}</span>
                    </div>
                  </div>

                  <p className="text-xs text-[#c9d1d9] leading-relaxed">
                    {claim.statement}
                  </p>

                  <div className="pt-2 flex items-center justify-between text-[11px] font-mono text-[#8b949e]" style={{ borderTop: '1px solid var(--color-border-muted, #21262d)' }}>
                    <div className="flex items-center gap-3">
                      <span>Source: <strong className="text-white">{claim.locator}</strong></span>
                      {claim.evidenceCasHash && (
                        <span>Proof CAS: <code className="text-[#3fb950]">{claim.evidenceCasHash.slice(0, 24)}...</code></span>
                      )}
                    </div>
                    {claim.pScore && (
                      <span className="text-[#3fb950]">p-value / score: {claim.pScore}</span>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* TAB 3: EXPERIMENTS & REPRODUCIBILITY */}
      {activeTab === 'experiments' && (
        <div className="flex-1 flex flex-col p-6 overflow-y-auto space-y-4" style={{ background: 'var(--color-canvas, #0d1117)' }}>
          <div className="flex items-center justify-between pb-3" style={{ borderBottom: '1px solid var(--color-border-default, #30363d)' }}>
            <div>
              <h2 className="text-sm font-bold text-white flex items-center gap-2">
                <FlaskConical className="w-4 h-4 text-[#d29922]" />
                <span>Deterministic Invariant Trials</span>
              </h2>
              <p className="text-xs text-[#8b949e]">Run parameter grid validation against Custos daemon</p>
            </div>
            <button 
              className="px-3 py-1.5 rounded text-white text-xs font-medium flex items-center gap-1.5"
              style={{ background: 'var(--color-coding, #3fb950)' }}
            >
              <Plus className="w-3.5 h-3.5" />
              <span>New Experiment Trial</span>
            </button>
          </div>

          <div className="space-y-3">
            {EXPERIMENTS.map((exp) => (
              <div 
                key={exp.id}
                className="p-4 rounded-xl flex items-center justify-between select-text"
                style={{
                  background: 'var(--color-surface-1, #161b22)',
                  border: '1px solid var(--color-border-default, #30363d)'
                }}
              >
                <div className="space-y-1">
                  <div className="flex items-center gap-2">
                    <span className="font-mono text-xs text-[#8b949e]">{exp.id}</span>
                    <span className="font-semibold text-white text-xs">{exp.name}</span>
                  </div>
                  <div className="font-mono text-[11px] text-[#6e7681]">Params: {exp.parameters}</div>
                </div>

                <div className="flex items-center gap-4 font-mono text-xs">
                  <span className="text-[#8b949e]">{exp.duration}</span>
                  <div className="text-right">
                    <div className="text-[10px] text-[#8b949e]">Reproducibility</div>
                    <div className={exp.reproducibility > 95 ? 'text-[#3fb950] font-bold' : 'text-[#d29922] font-bold'}>
                      {exp.reproducibility}%
                    </div>
                  </div>
                  <span 
                    className="px-2 py-0.5 rounded text-[11px] font-bold uppercase"
                    style={{
                      background: exp.status === 'passed' ? 'rgba(63,185,80,0.15)' : 'rgba(210,153,34,0.15)',
                      color: exp.status === 'passed' ? '#3fb950' : '#d29922'
                    }}
                  >
                    {exp.status}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* TAB 4: SYNTHESIS NOTES & TASK TRANSFER */}
      {activeTab === 'notes' && (
        <div className="flex-1 flex flex-col p-6 overflow-hidden" style={{ background: 'var(--color-canvas, #0d1117)' }}>
          <div className="flex items-center justify-between pb-3 shrink-0" style={{ borderBottom: '1px solid var(--color-border-default, #30363d)' }}>
            <div>
              <h2 className="text-sm font-bold text-white flex items-center gap-2">
                <FileText className="w-4 h-4 text-[#a78bfa]" />
                <span>Synthesis Notes & Coding Bridge</span>
              </h2>
              <p className="text-xs text-[#8b949e]">Draft specifications and export directly to Coding Workbench</p>
            </div>

            <div className="flex items-center gap-2">
              <button
                onClick={() => {
                  navigator.clipboard.writeText(notes);
                  setCopiedNote(true);
                  setTimeout(() => setCopiedNote(false), 2000);
                }}
                className="px-2.5 py-1 rounded text-xs text-[#8b949e] hover:text-white flex items-center gap-1 transition"
                style={{ background: 'var(--color-surface-2, #1c2128)', border: '1px solid var(--color-border-default, #30363d)' }}
              >
                {copiedNote ? <Check className="w-3.5 h-3.5 text-[#3fb950]" /> : <Copy className="w-3.5 h-3.5" />}
                <span>{copiedNote ? 'Copied' : 'Copy Markdown'}</span>
              </button>

              <button
                onClick={handleTransferToTask}
                className="px-3 py-1 rounded text-white text-xs font-medium flex items-center gap-1.5 transition"
                style={{ background: 'var(--color-coding, #3fb950)' }}
                title="Create an Engineering Coding Task bound to these verified claims"
              >
                <span>{transferredToTask ? '✓ Task Created in Coding Lens' : 'Transfer Claims to Coding Task →'}</span>
              </button>
            </div>
          </div>

          <textarea
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            className="flex-1 w-full mt-3 p-4 rounded-xl font-mono text-xs leading-relaxed text-[#c9d1d9] resize-none focus:outline-none select-text"
            style={{
              background: 'var(--color-surface-1, #161b22)',
              border: '1px solid var(--color-border-default, #30363d)',
            }}
          />
        </div>
      )}
    </div>
  );
};

export default ResearchWorkspace;
