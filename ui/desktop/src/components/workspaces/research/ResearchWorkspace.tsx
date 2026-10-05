import React, { useState } from 'react';
import { 
  BookOpen, 
  Search, 
  ExternalLink, 
  FileText, 
  Network, 
  Sparkles, 
  Quote, 
  Download,
  Bookmark,
  Plus,
  FileCheck,
  Filter
} from 'lucide-react';

interface PaperItem {
  id: string;
  title: string;
  authors: string;
  source: string;
  year: number;
  citations: number;
  abstract: string;
  tags: string[];
}

interface GraphNode {
  id: string;
  label: string;
  type: 'paper' | 'claim' | 'evidence' | 'code';
  x: number;
  y: number;
  summary: string;
  casHash?: string;
}

interface GraphEdge {
  from: string;
  to: string;
  label: string;
}

const MOCK_PAPERS: PaperItem[] = [
  {
    id: 'arxiv-2403.1189',
    title: 'Constitutional AI and Autonomous Multi-Agent Consensus in Critical Systems',
    authors: 'A. Vaswani, D. Silver, et al.',
    source: 'arXiv:2403.1189 [cs.AI]',
    year: 2025,
    citations: 142,
    abstract: 'We explore automated verification mechanisms for LLM agent chains utilizing non-repudiation cryptographic ledgers and self-correcting feedback loops.',
    tags: ['Multi-Agent', 'Safety', 'OmniRoute']
  },
  {
    id: 'biorxiv-2024.992',
    title: 'High-Throughput Deep Mutational Scanning Using Deep Generative Priors',
    authors: 'E. Lander, J. Doudna, et al.',
    source: 'bioRxiv 10.1101/2024.992',
    year: 2025,
    citations: 89,
    abstract: 'Systematic characterization of 150,000 missense variants in human tumor suppressors using structural embeddings from AlphaFold 3.',
    tags: ['Genomics', 'Bioinformatics', 'AlphaFold']
  },
  {
    id: 'pnas-129402',
    title: 'Adaptive Semantic Caching with Sub-millisecond Vector Indexes in Edge Runtimes',
    authors: 'M. Stonebraker, J. Dean, et al.',
    source: 'Proceedings of the ACM on Management of Data',
    year: 2024,
    citations: 215,
    abstract: 'Demonstrating 78% reduction in inference cost and 4x speedup across heterogeneous multi-turn conversational agents.',
    tags: ['Vector DB', 'Semantic Cache', 'Systems']
  }
];

const GRAPH_NODES: GraphNode[] = [
  { id: 'n-paper-1', label: 'arXiv:2403.1189 (Constitutional AI)', type: 'paper', x: 120, y: 140, summary: 'Paper on non-repudiation multi-agent consensus ledgers.' },
  { id: 'n-claim-1', label: 'Claim: Double-Dispatch Fencing (INV-03)', type: 'claim', x: 380, y: 100, summary: 'Requires atomic ticket acquisition before side effects execute.' },
  { id: 'n-ev-1', label: 'Evidence: CAS bafy2... (Test Logs)', type: 'evidence', x: 620, y: 110, casHash: 'cas://bafy2bzace4v3k99a', summary: '14 unit tests passed verifying permit acquisition.' },
  { id: 'n-code-1', label: 'Code: PermitGate::acquire_ticket', type: 'code', x: 500, y: 250, summary: 'Rust implementation in crates/custos-core/src/authority.' },
  { id: 'n-paper-2', label: 'ACM:2024.Adaptive Caching', type: 'paper', x: 140, y: 320, summary: 'Sub-millisecond semantic vector index paper.' },
  { id: 'n-claim-2', label: 'Claim: 78% Cost Reduction', type: 'claim', x: 370, y: 340, summary: 'Demonstrated reduction in edge agent inference cost.' }
];

const GRAPH_EDGES: GraphEdge[] = [
  { from: 'n-paper-1', to: 'n-claim-1', label: 'cites' },
  { from: 'n-claim-1', to: 'n-code-1', label: 'implemented_by' },
  { from: 'n-code-1', to: 'n-ev-1', label: 'verified_by' },
  { from: 'n-paper-2', to: 'n-claim-2', label: 'hypothesizes' }
];

export const ResearchWorkspace: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'sources' | 'graph' | 'notebook'>('sources');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedPaper, setSelectedPaper] = useState<PaperItem>(MOCK_PAPERS[0]);
  const [savedPapers, setSavedPapers] = useState<string[]>([MOCK_PAPERS[0].id]);
  
  // Knowledge Graph State
  const [selectedNode, setSelectedNode] = useState<GraphNode>(GRAPH_NODES[1]);
  const [graphFilter, setGraphFilter] = useState<'all' | 'paper' | 'claim' | 'evidence' | 'code'>('all');
  
  // Notebook State
  const [notebookContent, setNotebookContent] = useState<string>(
`# Custos Autonomous Agents: Research Synthesis
*Generated from active literature and Evidence Gate (INV-05)*

## 1. Core Synthesis Findings
- **Consensus & Non-Repudiation**: Per Vaswani et al. (2025), autonomous multi-agent systems must record side effects into an append-only ledger to prevent phantom distributed atomicity (satisfies **INV-07**).
- **Zero-IO Execution**: Invariant Gate tickets must be acquired prior to file mutation or network dispatch (**INV-03**).

## 2. Integrated Evidence Claims
- **Criterion 1**: Unit test pass proof stored at \`cas://bafy2bzace4v3k...9a\`.
- **Criterion 2**: Pre-condition gate validated against SQLite outbox.`
  );
  const [copiedNote, setCopiedNote] = useState(false);

  const toggleSave = (id: string) => {
    setSavedPapers(prev => 
      prev.includes(id) ? prev.filter(p => p !== id) : [...prev, id]
    );
  };

  const filteredNodes = graphFilter === 'all' 
    ? GRAPH_NODES 
    : GRAPH_NODES.filter(n => n.type === graphFilter);

  return (
    <div className="flex-1 flex flex-col bg-[#090b10] text-neutral-300 h-full overflow-hidden font-sans border-l border-surface-border">
      {/* Workspace Header */}
      <div className="h-10 border-b border-surface-border px-4 flex items-center justify-between shrink-0 bg-surface">
        <div className="flex items-center gap-2">
          <div className="p-1 rounded-md bg-purple-500/10 text-purple-400 border border-purple-500/20">
            <BookOpen className="w-3.5 h-3.5" />
          </div>
          <span className="text-xs font-semibold text-white">Research & Knowledge Lab</span>
          <span className="text-[10px] text-purple-400 bg-purple-500/10 px-2 py-0.5 rounded-full border border-purple-500/20 font-mono">
            3 Papers • 6 Evidence Claims
          </span>
        </div>

        {/* Workspace Sub-tabs */}
        <div className="flex items-center gap-1 bg-[#131622] p-0.5 rounded-lg border border-[#202534]">
          <button
            onClick={() => setActiveTab('sources')}
            className={`px-2.5 py-1 rounded-md text-[11px] font-medium transition flex items-center gap-1.5 ${
              activeTab === 'sources'
                ? 'bg-surface-elevated text-white shadow-sm'
                : 'text-neutral-400 hover:text-white'
            }`}
          >
            <FileText className="w-3 h-3 text-purple-400" />
            <span>Literature</span>
          </button>

          <button
            onClick={() => setActiveTab('graph')}
            className={`px-2.5 py-1 rounded-md text-[11px] font-medium transition flex items-center gap-1.5 ${
              activeTab === 'graph'
                ? 'bg-surface-elevated text-white shadow-sm'
                : 'text-neutral-400 hover:text-white'
            }`}
          >
            <Network className="w-3 h-3 text-cyan-400" />
            <span>Evidence Graph</span>
          </button>

          <button
            onClick={() => setActiveTab('notebook')}
            className={`px-2.5 py-1 rounded-md text-[11px] font-medium transition flex items-center gap-1.5 ${
              activeTab === 'notebook'
                ? 'bg-surface-elevated text-white shadow-sm'
                : 'text-neutral-400 hover:text-white'
            }`}
          >
            <Sparkles className="w-3 h-3 text-emerald-400" />
            <span>Synthesis Notes</span>
          </button>
        </div>
      </div>

      {/* Main Content Area */}

      {/* TAB 1: LITERATURE SOURCES */}
      {activeTab === 'sources' && (
        <div className="flex-1 flex min-h-0">
          {/* Left Column: Paper Search & List */}
          <div className="w-80 border-r border-surface-border flex flex-col bg-surface/50">
            {/* Search Bar */}
            <div className="p-3 border-b border-surface-border">
              <div className="relative">
                <Search className="w-3.5 h-3.5 absolute left-3 top-1/2 -translate-y-1/2 text-neutral-500" />
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Search ArXiv, PubMed, Crossref..."
                  className="w-full bg-[#131622] border border-[#202534] rounded-lg pl-8 pr-3 py-1.5 text-xs text-white placeholder-neutral-500 focus:outline-none focus:border-purple-500 transition"
                />
              </div>
            </div>

            {/* Paper List */}
            <div className="flex-1 overflow-y-auto p-2 space-y-2">
              {MOCK_PAPERS.map((paper) => {
                const isSelected = selectedPaper.id === paper.id;
                const isSaved = savedPapers.includes(paper.id);
                return (
                  <div
                    key={paper.id}
                    onClick={() => setSelectedPaper(paper)}
                    className={`p-3 rounded-xl cursor-pointer transition border text-xs ${
                      isSelected
                        ? 'bg-surface-elevated border-purple-500/50 shadow-md text-white'
                        : 'bg-surface-card hover:bg-surface-elevated/70 border-surface-border text-neutral-300'
                    }`}
                  >
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-[10px] font-mono text-purple-400">{paper.source}</span>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          toggleSave(paper.id);
                        }}
                        className="text-neutral-500 hover:text-purple-400 transition"
                      >
                        <Bookmark className={`w-3.5 h-3.5 ${isSaved ? 'fill-purple-400 text-purple-400' : ''}`} />
                      </button>
                    </div>

                    <h4 className="font-semibold text-neutral-200 line-clamp-2 leading-snug mb-1.5">
                      {paper.title}
                    </h4>

                    <div className="flex items-center justify-between text-[10px] text-neutral-500">
                      <span>{paper.year} • {paper.citations} cites</span>
                      <div className="flex gap-1">
                        {paper.tags.slice(0, 2).map(tag => (
                          <span key={tag} className="px-1.5 py-0.2 rounded bg-surface border border-surface-border text-neutral-400">
                            {tag}
                          </span>
                        ))}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Right Column: Active Paper Detail */}
          <div className="flex-1 overflow-y-auto p-6 space-y-5 bg-[#090b10]">
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-xs font-mono text-purple-400 bg-purple-500/10 px-2 py-0.5 rounded-full border border-purple-500/20">
                  {selectedPaper.source}
                </span>

                <div className="flex items-center gap-2">
                  <button className="px-2.5 py-1 rounded-lg bg-surface-card hover:bg-surface-elevated border border-surface-border text-xs text-neutral-300 flex items-center gap-1.5 transition">
                    <Quote className="w-3.5 h-3.5 text-purple-400" />
                    <span>Cite</span>
                  </button>
                  <button className="px-2.5 py-1 rounded-lg bg-surface-card hover:bg-surface-elevated border border-surface-border text-xs text-neutral-300 flex items-center gap-1.5 transition">
                    <ExternalLink className="w-3.5 h-3.5 text-neutral-400" />
                    <span>View PDF</span>
                  </button>
                </div>
              </div>

              <h1 className="text-base font-bold text-white leading-tight">
                {selectedPaper.title}
              </h1>

              <div className="text-xs text-neutral-400 flex items-center gap-4 border-b border-surface-border pb-3">
                <span><strong className="text-neutral-300">Authors:</strong> {selectedPaper.authors}</span>
                <span><strong className="text-neutral-300">Citations:</strong> {selectedPaper.citations}</span>
                <span><strong className="text-neutral-300">Published:</strong> {selectedPaper.year}</span>
              </div>
            </div>

            {/* AI Executive Summary Card */}
            <div className="p-4 rounded-xl bg-purple-500/5 border border-purple-500/20 space-y-2">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2 text-xs font-semibold text-purple-300">
                  <Sparkles className="w-4 h-4 text-purple-400" />
                  <span>Custos Evidence Synthesis</span>
                </div>
                <button className="px-2 py-0.5 rounded bg-purple-500/20 hover:bg-purple-500/30 text-purple-300 text-[10px] font-medium transition flex items-center gap-1">
                  <Plus className="w-3 h-3" />
                  <span>Extract as Evidence Claim</span>
                </button>
              </div>
              <p className="text-xs text-neutral-300 leading-relaxed">
                Key implications for your active session and invariant verification gates:
              </p>
              <ul className="text-xs text-neutral-300 space-y-1.5 list-disc list-inside">
                <li>Demonstrates resilience against rogue agent divergence using cryptographically signed consensus.</li>
                <li>Reduces communication token overhead by 41% using streaming diff compression.</li>
                <li>Provides empirical baseline for PermitGate ticket expiration timeout.</li>
              </ul>
            </div>

            {/* Abstract */}
            <div className="space-y-2">
              <h3 className="text-xs font-semibold uppercase tracking-wider text-neutral-400">
                Original Abstract
              </h3>
              <p className="text-xs text-neutral-300 leading-relaxed font-serif bg-surface-card p-4 rounded-xl border border-surface-border select-text">
                {selectedPaper.abstract}
              </p>
            </div>
          </div>
        </div>
      )}

      {/* TAB 2: INTERACTIVE KNOWLEDGE & EVIDENCE GRAPH */}
      {activeTab === 'graph' && (
        <div className="flex-1 flex flex-col min-h-0 bg-[#090b10] relative overflow-hidden">
          {/* Top Filter Bar */}
          <div className="h-9 border-b border-surface-border px-4 flex items-center justify-between bg-surface shrink-0 text-xs">
            <div className="flex items-center gap-1.5">
              <Filter className="w-3.5 h-3.5 text-neutral-400" />
              <span className="text-[11px] text-neutral-400 font-medium">Filter Nodes:</span>
              {(['all', 'paper', 'claim', 'evidence', 'code'] as const).map(type => (
                <button
                  key={type}
                  onClick={() => setGraphFilter(type)}
                  className={`px-2 py-0.5 rounded text-[10.5px] font-mono transition capitalize ${
                    graphFilter === type
                      ? 'bg-purple-500/20 text-purple-300 border border-purple-500/30 font-semibold'
                      : 'text-neutral-400 hover:text-white'
                  }`}
                >
                  {type}
                </button>
              ))}
            </div>

            <div className="text-[10px] text-neutral-500 font-mono">
              6 Nodes • 4 Non-Repudiation Edges
            </div>
          </div>

          <div className="flex-1 flex min-h-0 relative">
            {/* SVG Interactive Canvas */}
            <div className="flex-1 relative overflow-hidden bg-[radial-gradient(#1c2234_1px,transparent_1px)] [background-size:20px_20px]">
              <svg className="w-full h-full absolute inset-0">
                <defs>
                  <marker
                    id="arrowhead"
                    markerWidth="8"
                    markerHeight="6"
                    refX="7"
                    refY="3"
                    orient="auto"
                  >
                    <polygon points="0 0, 8 3, 0 6" fill="#4a5568" />
                  </marker>
                </defs>

                {/* Render Edges */}
                {GRAPH_EDGES.map((edge, idx) => {
                  const source = GRAPH_NODES.find(n => n.id === edge.from);
                  const target = GRAPH_NODES.find(n => n.id === edge.to);
                  if (!source || !target) return null;
                  return (
                    <g key={idx}>
                      <line
                        x1={source.x + 80}
                        y1={source.y + 20}
                        x2={target.x + 80}
                        y2={target.y + 20}
                        stroke="#2d3748"
                        strokeWidth="2"
                        strokeDasharray="4 2"
                        markerEnd="url(#arrowhead)"
                      />
                      <text
                        x={(source.x + target.x) / 2 + 80}
                        y={(source.y + target.y) / 2 + 15}
                        fill="#718096"
                        fontSize="9"
                        fontFamily="monospace"
                        textAnchor="middle"
                      >
                        {edge.label}
                      </text>
                    </g>
                  );
                })}
              </svg>

              {/* Render Node Cards */}
              {filteredNodes.map((node) => {
                const isSelected = selectedNode.id === node.id;
                const nodeColors = {
                  paper: 'border-purple-500/40 bg-purple-950/20 text-purple-300',
                  claim: 'border-amber-500/40 bg-amber-950/20 text-amber-300',
                  evidence: 'border-cyan-500/40 bg-cyan-950/20 text-cyan-300',
                  code: 'border-emerald-500/40 bg-emerald-950/20 text-emerald-300'
                };

                return (
                  <div
                    key={node.id}
                    onClick={() => setSelectedNode(node)}
                    style={{ left: `${node.x}px`, top: `${node.y}px` }}
                    className={`absolute w-56 p-2.5 rounded-xl border cursor-pointer transition shadow-lg select-none backdrop-blur-md ${
                      isSelected
                        ? 'ring-2 ring-purple-400 scale-105 z-20 ' + nodeColors[node.type]
                        : 'hover:scale-102 z-10 ' + nodeColors[node.type]
                    }`}
                  >
                    <div className="flex items-center justify-between text-[9px] font-mono uppercase mb-1">
                      <span>{node.type}</span>
                      {node.casHash && <span className="text-cyan-400">CAS</span>}
                    </div>
                    <div className="text-[11px] font-semibold text-white truncate">
                      {node.label}
                    </div>
                  </div>
                );
              })}
            </div>

            {/* Right Node Inspector */}
            {selectedNode && (
              <div className="w-80 border-l border-surface-border bg-surface p-4 flex flex-col space-y-4 shrink-0 overflow-y-auto">
                <div className="flex items-center justify-between pb-2 border-b border-surface-border">
                  <div className="flex items-center gap-2">
                    <span className="text-[10px] font-mono uppercase px-2 py-0.5 rounded bg-purple-500/10 text-purple-400 border border-purple-500/20">
                      {selectedNode.type}
                    </span>
                    <span className="text-xs font-semibold text-white">Node Inspector</span>
                  </div>
                </div>

                <div>
                  <h3 className="text-xs font-bold text-white mb-1">{selectedNode.label}</h3>
                  <p className="text-[11px] text-neutral-400 leading-relaxed">
                    {selectedNode.summary}
                  </p>
                </div>

                {selectedNode.casHash && (
                  <div className="p-2.5 rounded-xl bg-cyan-500/5 border border-cyan-500/20 space-y-1">
                    <span className="text-[9.5px] font-mono text-cyan-400 uppercase">Cryptographic CAS Link</span>
                    <div className="text-[10.5px] font-mono text-cyan-200 truncate">{selectedNode.casHash}</div>
                  </div>
                )}

                <div className="pt-2 border-t border-surface-border space-y-2">
                  <button className="w-full py-1.5 rounded-lg bg-purple-600 hover:bg-purple-500 text-white text-xs font-medium transition flex items-center justify-center gap-1.5 shadow-sm">
                    <FileCheck className="w-3.5 h-3.5" />
                    <span>Attach to Task Evidence Gate</span>
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* TAB 3: SYNTHESIS NOTEBOOK */}
      {activeTab === 'notebook' && (
        <div className="flex-1 flex flex-col bg-[#090b10] p-6 overflow-y-auto">
          <div className="max-w-3xl mx-auto w-full space-y-4">
            <div className="flex items-center justify-between border-b border-surface-border pb-3">
              <div className="flex items-center gap-2">
                <Sparkles className="w-4 h-4 text-emerald-400" />
                <h2 className="text-sm font-bold text-white">Working Research Synthesis Note</h2>
              </div>
              <div className="flex items-center gap-2">
                <button 
                  onClick={() => {
                    navigator.clipboard.writeText(notebookContent);
                    setCopiedNote(true);
                    setTimeout(() => setCopiedNote(false), 2000);
                  }}
                  className="px-2.5 py-1 rounded-lg bg-surface-card hover:bg-surface-elevated border border-surface-border text-xs text-neutral-300 flex items-center gap-1.5 transition"
                >
                  <Download className="w-3.5 h-3.5" />
                  <span>{copiedNote ? 'Copied!' : 'Copy Markdown'}</span>
                </button>
              </div>
            </div>

            <textarea
              value={notebookContent}
              onChange={(e) => setNotebookContent(e.target.value)}
              rows={14}
              className="w-full bg-[#111420] border border-surface-border rounded-xl p-4 text-xs font-mono text-neutral-200 focus:outline-none focus:border-purple-500 leading-relaxed resize-none transition"
            />
          </div>
        </div>
      )}
    </div>
  );
};

export default ResearchWorkspace;
