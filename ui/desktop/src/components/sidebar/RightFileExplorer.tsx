import React, { useState } from 'react';
import { 
  Folder, 
  FolderOpen, 
  FileCode, 
  Search, 
  ChevronRight, 
  ChevronDown, 
  X,
  Files,
  GitBranch,
  ShieldAlert,
  FileCheck,
  Layers,
  CheckCircle2,
  ExternalLink,
  RotateCw,
  AlertTriangle,
  Check,
  Ban,
  Activity,
  DollarSign,
  FolderPlus,
  GitMerge
} from 'lucide-react';

interface FileTreeItem {
  id: string;
  name: string;
  type: 'file' | 'folder';
  status?: 'modified' | 'clean';
  children?: FileTreeItem[];
}

const REPO_FILE_TREE: FileTreeItem[] = [
  { id: '1', name: '.agents', type: 'folder' },
  { id: '2', name: '.cargo', type: 'folder' },
  { id: '3', name: '.custos', type: 'folder' },
  { id: '4', name: '.github', type: 'folder' },
  { id: '5', name: 'config', type: 'folder' },
  {
    id: '6',
    name: 'crates',
    type: 'folder',
    status: 'modified',
    children: [
      { id: '6-1', name: 'custos-core', type: 'folder' },
      { id: '6-2', name: 'custos-daemon', type: 'folder', status: 'modified' },
      { id: '6-3', name: 'custos-domain', type: 'folder' },
      { id: '6-4', name: 'custos-runtime', type: 'folder' }
    ]
  },
  { id: '7', name: 'dev_docs', type: 'folder', status: 'modified' },
  { id: '8', name: 'docs', type: 'folder', status: 'modified' },
  { id: '9', name: 'evals', type: 'folder' },
  { id: '10', name: 'examples', type: 'folder' },
  { id: '11', name: 'packages', type: 'folder' },
  { id: '12', name: 'schemas', type: 'folder' },
  { id: '13', name: 'scratch', type: 'folder' },
  { id: '14', name: 'scripts', type: 'folder' },
  { id: '15', name: 'target', type: 'folder' },
  { id: '16', name: 'tests', type: 'folder', status: 'modified' },
  { id: '17', name: 'tools', type: 'folder', status: 'modified' },
  {
    id: '18',
    name: 'ui',
    type: 'folder',
    status: 'modified',
    children: [
      { id: '18-1', name: 'desktop', type: 'folder', status: 'modified' }
    ]
  },
  { id: '19', name: 'xtask', type: 'folder' },
  { id: '20', name: '.cursorrules', type: 'file' },
  { id: '21', name: '.editorconfig', type: 'file' },
  { id: '22', name: '.gitignore', type: 'file' },
  { id: '23', name: 'AGENTS.md', type: 'file' },
  { id: '25', name: 'Cargo.lock', type: 'file', status: 'modified' },
  { id: '26', name: 'Cargo.toml', type: 'file' },
  { id: '27', name: 'CLAUDE.md', type: 'file' },
  { id: '29', name: 'Custos.md', type: 'file', status: 'modified' },
  { id: '34', name: 'package.json', type: 'file' },
  { id: '36', name: 'README.md', type: 'file', status: 'modified' }
];

export type RightSidebarTab = 'explorer' | 'worktrees' | 'orchestration' | 'permits' | 'evidence' | 'telemetry';

export interface ExecutionPermit {
  id: string;
  action: 'fs:write' | 'exec:bash' | 'net:fetch' | 'mcp:call';
  actor: 'S2Planner' | 'Worker-1' | 'DaemonKernel';
  target: string;
  payloadSummary: string;
  tainted: boolean;
  status: 'pending' | 'approved' | 'denied';
  timestamp: string;
}

export interface EvidenceCriterion {
  id: string;
  title: string;
  status: 'pass' | 'fail' | 'uncertain' | 'pending';
  evidenceType: 'CAS' | 'TestLog' | 'Ledger';
  casHash?: string;
  summary: string;
}

interface RightFileExplorerProps {
  isOpen: boolean;
  onClose: () => void;
  onSelectFile?: (fileName: string) => void;
  onOpenWorktreeModal?: () => void;
}

export const RightFileExplorer: React.FC<RightFileExplorerProps> = ({
  isOpen,
  onClose,
  onSelectFile,
  onOpenWorktreeModal
}) => {
  const [activeTab, setActiveTab] = useState<RightSidebarTab>('explorer');
  const [fileSearchMode, setFileSearchMode] = useState<'names' | 'contents'>('names');
  const [searchQuery, setSearchQuery] = useState('');
  const [openFolders, setOpenFolders] = useState<Record<string, boolean>>({
    '6': true,
    '18': true
  });
  const [selectedFile, setSelectedFile] = useState<string>('Custos.md');

  // Orca O2: Worktrees state
  const [worktrees] = useState([
    { branch: 'main', base: 'a3f2d1e', status: 'clean', isCurrent: false },
    { branch: 'feat/simd-dispatch', base: 'a3f2d1e', status: 'active (3 modified)', isCurrent: true },
    { branch: 'fix/permits-race', base: 'a3f2d1e', status: 'review diff', isCurrent: false }
  ]);

  // INV-03 Pending Permits state
  const [permits, setPermits] = useState<ExecutionPermit[]>([
    {
      id: 'PERMIT-8421',
      action: 'fs:write',
      actor: 'Worker-1',
      target: 'crates/custos-runtime/src/workflow/dispatcher.rs',
      payloadSummary: 'Patch 38 additions (+38 -4 lines) adding InvariantGate ticket verification',
      tainted: false,
      status: 'pending',
      timestamp: '2m ago'
    },
    {
      id: 'PERMIT-8422',
      action: 'exec:bash',
      actor: 'S2Planner',
      target: 'cargo test --package custos-runtime --lib',
      payloadSummary: 'Runs unit tests for ticket acquisition timeout and proof closure',
      tainted: false,
      status: 'pending',
      timestamp: '1m ago'
    },
    {
      id: 'PERMIT-8423',
      action: 'net:fetch',
      actor: 'Worker-1',
      target: 'https://api.crossref.org/works/10.1101/2024.992',
      payloadSummary: 'Fetch scientific DOI metadata for Research evidence anchor',
      tainted: true,
      status: 'pending',
      timestamp: 'Just now'
    }
  ]);

  // INV-05 & INV-06 Evidence Criteria state
  const [criteria] = useState<EvidenceCriterion[]>([
    {
      id: 'crit-1',
      title: 'Cargo Test Suite Closure',
      status: 'pass',
      evidenceType: 'TestLog',
      casHash: 'cas://bafy2bzace4v3k...9a',
      summary: '14/14 unit tests passed with exit code 0'
    },
    {
      id: 'crit-2',
      title: 'Invariant Ticket Acquisition Gate',
      status: 'pass',
      evidenceType: 'Ledger',
      casHash: 'ledger://ticket_8045_audit',
      summary: 'Non-repudiation signature verified in SQLite outbox'
    },
    {
      id: 'crit-3',
      title: 'Zero Untrusted Policy Injection (INV-04)',
      status: 'uncertain',
      evidenceType: 'CAS',
      casHash: 'cas://taint_check_e08',
      summary: 'Crossref fetch response carries Taint::Untrusted; pending policy boundary filter'
    }
  ]);

  if (!isOpen) return null;

  const pendingPermitsCount = permits.filter(p => p.status === 'pending').length;

  const handleApprovePermit = (id: string) => {
    setPermits(prev => prev.map(p => p.id === id ? { ...p, status: 'approved' } : p));
  };

  const handleDenyPermit = (id: string) => {
    setPermits(prev => prev.map(p => p.id === id ? { ...p, status: 'denied' } : p));
  };

  const handleApproveAll = () => {
    setPermits(prev => prev.map(p => ({ ...p, status: 'approved' })));
  };

  const toggleFolder = (id: string) => {
    setOpenFolders(prev => ({ ...prev, [id]: !prev[id] }));
  };

  const renderTree = (items: FileTreeItem[], depth = 0) => {
    return items
      .filter(item => item.name.toLowerCase().includes(searchQuery.toLowerCase()))
      .map(item => {
        const isFolder = item.type === 'folder';
        const isOpen = openFolders[item.id];
        const isSelected = selectedFile === item.name;

        return (
          <div key={item.id} className="select-none">
            <div
              onClick={() => {
                if (isFolder) {
                  toggleFolder(item.id);
                } else {
                  setSelectedFile(item.name);
                  if (onSelectFile) onSelectFile(item.name);
                }
              }}
              style={{ paddingLeft: `${depth * 12 + 8}px` }}
              className={`flex items-center justify-between py-1 pr-2.5 rounded text-[11.5px] font-mono cursor-pointer transition ${
                isSelected
                  ? 'bg-[#1f242c] text-white font-medium'
                  : 'text-neutral-400 hover:text-neutral-200 hover:bg-[#161b22]'
              }`}
            >
              <div className="flex items-center gap-1.5 truncate">
                {isFolder ? (
                  <>
                    {isOpen ? (
                      <ChevronDown className="w-3 h-3 text-[#6e7681] shrink-0" />
                    ) : (
                      <ChevronRight className="w-3 h-3 text-[#6e7681] shrink-0" />
                    )}
                    {isOpen ? (
                      <FolderOpen className="w-3.5 h-3.5 text-[#d29922] shrink-0" />
                    ) : (
                      <Folder className="w-3.5 h-3.5 text-[#d29922] shrink-0" />
                    )}
                  </>
                ) : (
                  <>
                    <span className="w-3 shrink-0" />
                    <FileCode className={`w-3.5 h-3.5 shrink-0 ${isSelected ? 'text-[#3fb950]' : 'text-[#8b949e]'}`} />
                  </>
                )}
                <span className={`truncate ${item.name.endsWith('.md') ? 'text-amber-200/90' : ''}`}>
                  {item.name}
                </span>
              </div>

              {item.status === 'modified' && (
                <span className="text-[10px] font-mono font-bold text-[#d29922] shrink-0 ml-1">
                  M
                </span>
              )}
            </div>

            {isFolder && isOpen && item.children && (
              <div>{renderTree(item.children, depth + 1)}</div>
            )}
          </div>
        );
      });
  };

  return (
    <aside 
      className="w-80 flex flex-col shrink-0 z-20 select-none overflow-hidden h-full font-sans"
      style={{
        background: 'var(--color-surface-1, #161b22)',
        borderLeft: '1px solid var(--color-border-default, #30363d)'
      }}
    >
      {/* 1. Top Activity Bar with Orca ADE Tooling */}
      <div 
        className="h-9 px-2 flex items-center justify-between shrink-0"
        style={{
          background: 'var(--color-surface-2, #1c2128)',
          borderBottom: '1px solid var(--color-border-default, #30363d)'
        }}
      >
        <div className="flex items-center gap-0.5">
          {/* Files Explorer */}
          <button
            onClick={() => setActiveTab('explorer')}
            className={`p-1.5 rounded transition ${
              activeTab === 'explorer'
                ? 'bg-surface-0 text-white shadow-sm'
                : 'text-neutral-400 hover:text-white hover:bg-surface-2'
            }`}
            title="File Explorer (⌘E)"
          >
            <Files className="w-3.5 h-3.5" />
          </button>

          {/* Orca O2: Worktrees & Git */}
          <button
            onClick={() => setActiveTab('worktrees')}
            className={`p-1.5 rounded transition ${
              activeTab === 'worktrees'
                ? 'bg-surface-0 text-white shadow-sm'
                : 'text-neutral-400 hover:text-white hover:bg-surface-2'
            }`}
            title="Worktrees & Git (Orca O2)"
          >
            <GitBranch className="w-3.5 h-3.5 text-[#3fb950]" />
          </button>

          {/* Orca O6: Orchestration & Task DAG */}
          <button
            onClick={() => setActiveTab('orchestration')}
            className={`p-1.5 rounded transition ${
              activeTab === 'orchestration'
                ? 'bg-surface-0 text-white shadow-sm'
                : 'text-neutral-400 hover:text-white hover:bg-surface-2'
            }`}
            title="Orchestration & Task DAG (Orca O6)"
          >
            <Layers className="w-3.5 h-3.5 text-[#58a6ff]" />
          </button>

          {/* INV-03 Pending Permits */}
          <button
            onClick={() => setActiveTab('permits')}
            className={`p-1.5 rounded transition relative ${
              activeTab === 'permits'
                ? 'bg-surface-0 text-white shadow-sm'
                : 'text-neutral-400 hover:text-white hover:bg-surface-2'
            }`}
            title="Authority Engine: Pending Permits (INV-03)"
          >
            <ShieldAlert className="w-3.5 h-3.5 text-[#d29922]" />
            {pendingPermitsCount > 0 && (
              <span className="absolute -top-0.5 -right-0.5 w-3.5 h-3.5 rounded-full bg-amber-500 text-black text-[9px] font-mono font-bold flex items-center justify-center">
                {pendingPermitsCount}
              </span>
            )}
          </button>

          {/* INV-05 Evidence Gate */}
          <button
            onClick={() => setActiveTab('evidence')}
            className={`p-1.5 rounded transition ${
              activeTab === 'evidence'
                ? 'bg-surface-0 text-white shadow-sm'
                : 'text-neutral-400 hover:text-white hover:bg-surface-2'
            }`}
            title="Evidence Gate & Criteria (INV-05)"
          >
            <FileCheck className="w-3.5 h-3.5 text-[#388bfd]" />
          </button>

          {/* Orca O10: Telemetry & Cost */}
          <button
            onClick={() => setActiveTab('telemetry')}
            className={`p-1.5 rounded transition ${
              activeTab === 'telemetry'
                ? 'bg-surface-0 text-white shadow-sm'
                : 'text-neutral-400 hover:text-white hover:bg-surface-2'
            }`}
            title="Telemetry & Cost Ledger (Orca O10)"
          >
            <DollarSign className="w-3.5 h-3.5 text-[#a371f7]" />
          </button>
        </div>

        {/* Close Button */}
        <button
          onClick={onClose}
          className="p-1 rounded text-neutral-400 hover:text-white hover:bg-surface-2 transition"
          title="Close Inspector (⌘E)"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      </div>

      {/* 2. BODY CONTENT */}

      {/* TAB 1: FILES EXPLORER */}
      {activeTab === 'explorer' && (
        <div className="flex-1 flex flex-col overflow-hidden min-h-0">
          <div className="p-2 border-b border-[#30363d] flex items-center justify-between gap-2 shrink-0">
            <div className="flex items-center gap-1 bg-[#0d1117] p-0.5 rounded border border-[#30363d] w-full">
              <button
                onClick={() => setFileSearchMode('names')}
                className={`flex-1 py-0.5 rounded text-[10.5px] font-medium transition text-center ${
                  fileSearchMode === 'names'
                    ? 'bg-[#1c2128] text-white font-semibold shadow-sm'
                    : 'text-neutral-400 hover:text-white'
                }`}
              >
                Names
              </button>
              <button
                onClick={() => setFileSearchMode('contents')}
                className={`flex-1 py-0.5 rounded text-[10.5px] font-medium transition text-center ${
                  fileSearchMode === 'contents'
                    ? 'bg-[#1c2128] text-white font-semibold shadow-sm'
                    : 'text-neutral-400 hover:text-white'
                }`}
              >
                Contents
              </button>
            </div>
          </div>

          <div className="px-2 pb-2 pt-1 shrink-0">
            <div className="relative">
              <Search className="w-3 h-3 absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-500" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder={fileSearchMode === 'names' ? "Find files... (⌘P)" : "Search in files... (⌘⇧F)"}
                className="w-full bg-[#0d1117] border border-[#30363d] rounded pl-7 pr-2 py-1 text-[11px] text-neutral-200 placeholder-neutral-500 focus:outline-none font-mono transition"
              />
            </div>
          </div>

          <div className="flex-1 overflow-y-auto px-2 space-y-0.5 min-w-0 font-mono text-[11px]">
            {renderTree(REPO_FILE_TREE)}
          </div>
        </div>
      )}

      {/* TAB 2: ORCA WORKTREES & GIT (O2 & O5) */}
      {activeTab === 'worktrees' && (
        <div className="flex-1 flex flex-col overflow-hidden min-h-0 p-3 space-y-3">
          <div className="flex items-center justify-between text-xs pb-1 border-b border-[#30363d]">
            <div className="flex items-center gap-1.5 font-semibold text-white">
              <GitBranch className="w-4 h-4 text-[#3fb950]" />
              <span>Worktrees & Git</span>
            </div>
            {onOpenWorktreeModal && (
              <button
                onClick={onOpenWorktreeModal}
                className="text-[10px] text-[#3fb950] hover:underline font-mono flex items-center gap-1"
              >
                <FolderPlus className="w-3 h-3" />
                <span>Manage</span>
              </button>
            )}
          </div>

          {/* Active Worktree Stage */}
          <div 
            className="p-3 rounded-xl space-y-2 font-mono text-[11px]"
            style={{
              background: 'var(--color-canvas, #0d1117)',
              border: '1px solid var(--color-border-default, #30363d)'
            }}
          >
            <div className="flex items-center justify-between">
              <span className="text-white font-bold flex items-center gap-1.5">
                <span className="w-2 h-2 rounded-full bg-[#3fb950] animate-pulse"></span>
                feat/simd-dispatch
              </span>
              <span className="px-1.5 py-0.2 rounded bg-emerald-500/10 text-emerald-400 text-[10px] font-bold">
                ACTIVE
              </span>
            </div>
            <div className="text-[10px] text-[#8b949e]">
              Base commit: <code className="text-[#3fb950]">#a3f2d1e</code>
            </div>
            <div className="text-[10px] text-[#8b949e]">
              Lifecycle: <span className="text-[#e6edf3]">2. Work → 3. Review Diff</span>
            </div>
          </div>

          {/* Managed Worktrees List */}
          <div className="space-y-1">
            <div className="text-[10px] font-semibold text-[#8b949e] uppercase px-1">
              Managed Worktrees ({worktrees.length})
            </div>
            {worktrees.map((wt) => (
              <div
                key={wt.branch}
                onClick={onOpenWorktreeModal}
                className={`p-2 rounded-lg font-mono text-[11px] flex items-center justify-between cursor-pointer transition ${
                  wt.isCurrent ? 'bg-[#1c2128] text-white border border-[#3fb950]' : 'hover:bg-[#161b22] text-[#8b949e]'
                }`}
              >
                <div className="flex items-center gap-1.5 truncate">
                  <GitBranch className={`w-3.5 h-3.5 shrink-0 ${wt.isCurrent ? 'text-[#3fb950]' : 'text-[#8b949e]'}`} />
                  <span className="truncate">{wt.branch}</span>
                </div>
                <span className="text-[9.5px] text-[#6e7681]">#{wt.base}</span>
              </div>
            ))}
          </div>

          {/* Changes in Worktree */}
          <div className="space-y-1.5 flex-1 overflow-y-auto">
            <div className="flex items-center justify-between text-[11px] text-neutral-400 font-semibold px-1">
              <span>WORKTREE CHANGES</span>
              <span className="font-mono text-[10px] text-[#3fb950] font-bold">+38 -4</span>
            </div>

            <div className="space-y-1 font-mono text-[11px]">
              {[
                { file: 'crates/.../dispatcher.rs', status: 'M', diff: '+34 -2' },
                { file: 'crates/.../permits.rs', status: 'M', diff: '+4 -2' },
                { file: 'crates/.../estimator.rs', status: 'A', diff: '+28 -0' }
              ].map((item) => (
                <div
                  key={item.file}
                  className="flex items-center justify-between px-2 py-1 rounded hover:bg-[#1c2128] cursor-pointer text-neutral-300 transition"
                >
                  <span className="truncate">{item.file}</span>
                  <div className="flex items-center gap-1.5 shrink-0 ml-1">
                    <span className="text-[10px] text-[#3fb950]">{item.diff}</span>
                    <span className="text-[10px] font-bold text-[#d29922]">{item.status}</span>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Quick Commit / Ship button */}
          <div className="pt-2 border-t border-[#30363d]">
            <button 
              onClick={onOpenWorktreeModal}
              className="w-full py-1.5 rounded-lg bg-surface-2 hover:bg-[#21262d] text-white border border-[#30363d] text-xs font-medium transition flex items-center justify-center gap-1.5"
            >
              <GitMerge className="w-3.5 h-3.5 text-[#a371f7]" />
              <span>Ship / Integrate Worktree</span>
            </button>
          </div>
        </div>
      )}

      {/* TAB 3: ORCA ORCHESTRATION & TASK DAG (O6) */}
      {activeTab === 'orchestration' && (
        <div className="flex-1 flex flex-col overflow-hidden min-h-0 p-3 space-y-3">
          <div className="flex items-center justify-between text-xs pb-1 border-b border-[#30363d]">
            <div className="flex items-center gap-1.5 font-semibold text-white">
              <Layers className="w-4 h-4 text-[#58a6ff]" />
              <span>Orchestration DAG</span>
            </div>
            <span className="text-[10px] font-mono text-[#3fb950] font-bold">RUN #1</span>
          </div>

          <div className="space-y-2 flex-1 overflow-y-auto">
            {/* Task Nodes */}
            <div 
              className="p-3 rounded-xl space-y-1.5 font-mono text-[11px]"
              style={{
                background: 'var(--color-canvas, #0d1117)',
                border: '1px solid var(--color-border-default, #30363d)'
              }}
            >
              <div className="flex items-center justify-between">
                <span className="font-bold text-white">T-01 • Invariant Gate</span>
                <span className="text-[9.5px] px-1.5 py-0.2 rounded bg-emerald-500/10 text-emerald-400 font-bold">
                  DONE
                </span>
              </div>
              <p className="text-[10px] text-[#8b949e]">
                Assignee: S2Planner · Proof closure: valid
              </p>
            </div>

            <div 
              className="p-3 rounded-xl space-y-1.5 font-mono text-[11px]"
              style={{
                background: 'var(--color-surface-2, #1c2128)',
                border: '1px solid var(--color-coding, #3fb950)'
              }}
            >
              <div className="flex items-center justify-between">
                <span className="font-bold text-white flex items-center gap-1">
                  <span className="w-1.5 h-1.5 rounded-full bg-blue-400 animate-pulse"></span>
                  T-02 • SIMD Dispatcher
                </span>
                <span className="text-[9.5px] px-1.5 py-0.2 rounded bg-blue-500/10 text-blue-400 font-bold">
                  RUNNING (ATT #2)
                </span>
              </div>
              <p className="text-[10px] text-[#8b949e]">
                Assignee: Claude Code · Depth: 1 · Epoch: 2
              </p>
            </div>

            <div 
              className="p-3 rounded-xl space-y-1.5 font-mono text-[11px]"
              style={{
                background: 'var(--color-canvas, #0d1117)',
                border: '1px solid var(--color-border-default, #30363d)'
              }}
            >
              <div className="flex items-center justify-between">
                <span className="font-bold text-neutral-400">T-03 • Closure & Verifier</span>
                <span className="text-[9.5px] px-1.5 py-0.2 rounded bg-neutral-800 text-neutral-400">
                  BLOCKED
                </span>
              </div>
              <p className="text-[10px] text-[#6e7681]">
                Awaiting completion of T-02
              </p>
            </div>
          </div>

          <div 
            className="p-2.5 rounded-lg text-[10.5px] font-mono text-[#3fb950] flex items-center gap-1.5"
            style={{
              background: 'rgba(63, 185, 80, 0.1)',
              border: '1px solid rgba(63, 185, 80, 0.2)'
            }}
          >
            <Activity className="w-3.5 h-3.5 shrink-0" />
            <span>Convergence: Active. 0 blocked cycles.</span>
          </div>
        </div>
      )}

      {/* TAB 4: INV-03 PENDING PERMITS */}
      {activeTab === 'permits' && (
        <div className="flex-1 flex flex-col overflow-hidden min-h-0 p-3 space-y-3">
          <div className="flex items-center justify-between text-xs pb-1 border-b border-[#30363d]">
            <div className="flex items-center gap-1.5 font-semibold text-white">
              <ShieldAlert className="w-4 h-4 text-[#d29922]" />
              <span>Authority Permits</span>
            </div>
            {pendingPermitsCount > 0 && (
              <button
                onClick={handleApproveAll}
                className="text-[10px] text-[#3fb950] hover:underline font-medium transition"
              >
                Approve All ({pendingPermitsCount})
              </button>
            )}
          </div>

          <p className="text-[10.5px] text-[#8b949e] leading-normal">
            Side effects intercepted by <code className="text-white font-mono">PermitGate</code>. Actions are blocked until approved.
          </p>

          <div className="flex-1 overflow-y-auto space-y-2.5">
            {permits.map((permit) => (
              <div 
                key={permit.id}
                className="p-3 rounded-xl border transition space-y-1.5"
                style={{
                  background: permit.status === 'approved' ? 'rgba(63,185,80,0.08)' : permit.status === 'denied' ? 'rgba(248,81,73,0.08)' : 'var(--color-canvas, #0d1117)',
                  borderColor: permit.status === 'approved' ? 'rgba(63,185,80,0.3)' : permit.status === 'denied' ? 'rgba(248,81,73,0.3)' : 'var(--color-border-default, #30363d)'
                }}
              >
                <div className="flex items-center justify-between text-[10px] font-mono">
                  <span className="font-bold text-white flex items-center gap-1">
                    <span className="px-1.5 py-0.2 rounded bg-amber-500/20 text-amber-300 font-bold">
                      {permit.action}
                    </span>
                    <span>{permit.id}</span>
                  </span>
                  <span className="text-neutral-500">{permit.timestamp}</span>
                </div>

                <div className="text-[11px] font-mono text-neutral-200 truncate" title={permit.target}>
                  {permit.target}
                </div>

                <p className="text-[10.5px] text-[#8b949e] leading-relaxed">
                  {permit.payloadSummary}
                </p>

                {permit.tainted && (
                  <div className="p-1.5 rounded bg-red-500/10 border border-red-500/20 flex items-center gap-1.5 text-[9.5px] text-red-300 font-mono">
                    <AlertTriangle className="w-3 h-3 text-red-400 shrink-0" />
                    <span>Taint::Untrusted input detected (INV-04)</span>
                  </div>
                )}

                <div className="flex items-center justify-between pt-1 border-t border-[#21262d] text-[10px]">
                  <span className="text-[#8b949e] font-mono">Actor: {permit.actor}</span>
                  
                  {permit.status === 'pending' ? (
                    <div className="flex items-center gap-1.5">
                      <button
                        onClick={() => handleDenyPermit(permit.id)}
                        className="px-2 py-0.5 rounded bg-red-500/10 hover:bg-red-500/20 text-red-400 border border-red-500/20 transition flex items-center gap-1"
                      >
                        <Ban className="w-2.5 h-2.5" />
                        Deny
                      </button>
                      <button
                        onClick={() => handleApprovePermit(permit.id)}
                        className="px-2.5 py-0.5 rounded bg-[#238636] hover:bg-[#2ea043] text-white font-medium shadow-sm transition flex items-center gap-1"
                      >
                        <Check className="w-2.5 h-2.5" />
                        Approve
                      </button>
                    </div>
                  ) : (
                    <span className={`font-mono font-medium ${permit.status === 'approved' ? 'text-emerald-400' : 'text-red-400'}`}>
                      {permit.status.toUpperCase()}
                    </span>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* TAB 5: INV-05 EVIDENCE GATE */}
      {activeTab === 'evidence' && (
        <div className="flex-1 flex flex-col overflow-hidden min-h-0 p-3 space-y-3">
          <div className="flex items-center justify-between text-xs pb-1 border-b border-[#30363d]">
            <div className="flex items-center gap-1.5 font-semibold text-white">
              <FileCheck className="w-4 h-4 text-[#388bfd]" />
              <span>Evidence Gate (INV-05)</span>
            </div>
            <span className="text-[10px] font-mono text-cyan-400">2 / 3 PASS</span>
          </div>

          <p className="text-[10.5px] text-[#8b949e] leading-normal">
            Completion criteria cannot pass without cryptographic CAS evidence and empirical logs.
          </p>

          <div className="flex-1 overflow-y-auto space-y-2">
            {criteria.map((crit) => (
              <div 
                key={crit.id}
                className="p-3 rounded-xl space-y-1.5"
                style={{
                  background: 'var(--color-canvas, #0d1117)',
                  border: '1px solid var(--color-border-default, #30363d)'
                }}
              >
                <div className="flex items-center justify-between">
                  <span className="text-[11px] font-medium text-white truncate max-w-[180px]">
                    {crit.title}
                  </span>
                  {crit.status === 'pass' && (
                    <span className="px-1.5 py-0.2 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 text-[9.5px] font-mono font-bold flex items-center gap-0.5">
                      <CheckCircle2 className="w-2.5 h-2.5" />
                      PASS
                    </span>
                  )}
                  {crit.status === 'uncertain' && (
                    <span className="px-1.5 py-0.2 rounded-full bg-amber-500/10 text-amber-400 border border-amber-500/20 text-[9.5px] font-mono font-bold flex items-center gap-0.5" title="Explicit Uncertainty (INV-06)">
                      <AlertTriangle className="w-2.5 h-2.5" />
                      UNCERTAIN
                    </span>
                  )}
                </div>

                <p className="text-[10px] text-[#8b949e] leading-relaxed">
                  {crit.summary}
                </p>

                {crit.casHash && (
                  <div className="text-[9.5px] font-mono text-cyan-400/90 bg-cyan-950/20 px-2 py-0.5 rounded border border-cyan-800/30 flex items-center justify-between truncate">
                    <span className="truncate">{crit.casHash}</span>
                    <ExternalLink className="w-2.5 h-2.5 shrink-0 ml-1 text-cyan-500" />
                  </div>
                )}
              </div>
            ))}
          </div>

          <div className="pt-2 border-t border-[#30363d]">
            <button className="w-full py-1.5 rounded-lg bg-surface-2 hover:bg-[#21262d] text-neutral-200 hover:text-white border border-[#30363d] text-xs font-medium transition flex items-center justify-center gap-1.5">
              <RotateCw className="w-3 h-3 text-[#388bfd]" />
              <span>Verify All Evidence Claims</span>
            </button>
          </div>
        </div>
      )}

      {/* TAB 6: ORCA O10 TELEMETRY & COST LEDGER */}
      {activeTab === 'telemetry' && (
        <div className="flex-1 flex flex-col overflow-hidden min-h-0 p-3 space-y-3">
          <div className="flex items-center justify-between text-xs pb-1 border-b border-[#30363d]">
            <div className="flex items-center gap-1.5 font-semibold text-white">
              <DollarSign className="w-4 h-4 text-[#a371f7]" />
              <span>Cost & Telemetry (Orca O10)</span>
            </div>
            <span className="text-[10px] font-mono text-[#3fb950] font-bold">$0.084 / $1.00</span>
          </div>

          <div className="space-y-2.5 font-mono text-[11px]">
            <div 
              className="p-3 rounded-xl space-y-2"
              style={{
                background: 'var(--color-canvas, #0d1117)',
                border: '1px solid var(--color-border-default, #30363d)'
              }}
            >
              <div className="flex items-center justify-between">
                <span className="text-[#8b949e]">Prompt Tokens</span>
                <span className="text-white font-bold">14,200</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[#8b949e]">Completion Tokens</span>
                <span className="text-white font-bold">4,220</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[#8b949e]">Cached KV Reads</span>
                <span className="text-[#3fb950] font-bold">11,800 (83%)</span>
              </div>
              <div className="flex items-center justify-between pt-1 border-t border-[#21262d]">
                <span className="text-[#8b949e]">Billed Cost</span>
                <span className="text-[#3fb950] font-bold">$0.084</span>
              </div>
            </div>

            <div 
              className="p-3 rounded-xl space-y-1.5"
              style={{
                background: 'var(--color-canvas, #0d1117)',
                border: '1px solid var(--color-border-default, #30363d)'
              }}
            >
              <div className="flex items-center justify-between">
                <span className="text-[#8b949e]">Runtime Latency p95</span>
                <span className="text-white">184ms</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[#8b949e]">IPC Heartbeat</span>
                <span className="text-[#3fb950]">Healthy (every 2s)</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-[#8b949e]">Active Workers</span>
                <span className="text-white">1 worker run</span>
              </div>
            </div>
          </div>
        </div>
      )}
    </aside>
  );
};

export default RightFileExplorer;
