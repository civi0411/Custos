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
  History,
  Radio,
  Sparkles,
  CheckCircle2,
  ExternalLink,
  RotateCw,
  AlertTriangle,
  Check,
  Ban
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
  { id: '23', name: '.windsurfrules', type: 'file' },
  { id: '24', name: 'AGENTS.md', type: 'file' },
  { id: '25', name: 'Cargo.lock', type: 'file', status: 'modified' },
  { id: '26', name: 'Cargo.toml', type: 'file' },
  { id: '27', name: 'CLAUDE.md', type: 'file' },
  { id: '28', name: 'custos.cmd', type: 'file' },
  { id: '29', name: 'Custos.md', type: 'file', status: 'modified' },
  { id: '30', name: 'custos.ps1', type: 'file' },
  { id: '31', name: 'deny.toml', type: 'file' },
  { id: '32', name: 'justfile', type: 'file' },
  { id: '33', name: 'LICENSE', type: 'file' },
  { id: '34', name: 'package.json', type: 'file' },
  { id: '35', name: 'pnpm-lock.yaml', type: 'file' },
  { id: '36', name: 'README.md', type: 'file', status: 'modified' },
  { id: '37', name: 'rust-toolchain.toml', type: 'file' },
  { id: '38', name: 'rustfmt.toml', type: 'file' }
];

export type RightSidebarTab = 'explorer' | 'permits' | 'evidence' | 'git' | 'audit' | 'ports';

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

export interface AuditEvent {
  id: string;
  actor: 'User' | 'DaemonKernel' | 'S2Planner' | 'WorkerRun';
  event: string;
  transport: 'DirectVendorSdk' | 'LocalInference' | 'McpSampling' | 'KernelIpc';
  cost: string;
  time: string;
}

interface RightFileExplorerProps {
  isOpen: boolean;
  onClose: () => void;
  onSelectFile?: (fileName: string) => void;
}

export const RightFileExplorer: React.FC<RightFileExplorerProps> = ({
  isOpen,
  onClose,
  onSelectFile
}) => {
  const [activeTab, setActiveTab] = useState<RightSidebarTab>('explorer');
  const [fileSearchMode, setFileSearchMode] = useState<'names' | 'contents'>('names');
  const [searchQuery, setSearchQuery] = useState('');
  const [commitMessage, setCommitMessage] = useState('');
  const [openFolders, setOpenFolders] = useState<Record<string, boolean>>({
    '6': true,
    '18': true
  });
  const [selectedFile, setSelectedFile] = useState<string>('Custos.md');

  // INV-03 Pending Permits state
  const [permits, setPermits] = useState<ExecutionPermit[]>([
    {
      id: 'PERMIT-8421',
      action: 'fs:write',
      actor: 'Worker-1',
      target: 'crates/custos-runtime/src/workflow/dispatcher.rs',
      payloadSummary: 'Patch 34 additions (+34 -2 lines) adding InvariantGate ticket verification',
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

  // INV-01 & INV-09 Audit Events
  const [auditEvents] = useState<AuditEvent[]>([
    {
      id: 'aud-1',
      actor: 'User',
      event: 'Injected turn anchor in Session vi',
      transport: 'KernelIpc',
      cost: '$0.00',
      time: '12m ago'
    },
    {
      id: 'aud-2',
      actor: 'S2Planner',
      event: 'Synthesized execution plan (3 steps)',
      transport: 'DirectVendorSdk',
      cost: '$0.018',
      time: '8m ago'
    },
    {
      id: 'aud-3',
      actor: 'WorkerRun',
      event: 'Dispatched code refactor to dispatcher.rs',
      transport: 'LocalInference',
      cost: '$0.00',
      time: '2m ago'
    },
    {
      id: 'aud-4',
      actor: 'DaemonKernel',
      event: 'Fenced network call: generated PERMIT-8423',
      transport: 'KernelIpc',
      cost: '$0.00',
      time: 'Just now'
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
                  ? 'bg-brand-blue/15 text-white font-medium'
                  : 'text-neutral-400 hover:text-neutral-200 hover:bg-[#161a26]'
              }`}
            >
              <div className="flex items-center gap-1.5 truncate">
                {isFolder ? (
                  <>
                    {isOpen ? (
                      <ChevronDown className="w-3 h-3 text-neutral-500 shrink-0" />
                    ) : (
                      <ChevronRight className="w-3 h-3 text-neutral-500 shrink-0" />
                    )}
                    {isOpen ? (
                      <FolderOpen className="w-3.5 h-3.5 text-amber-400/80 shrink-0" />
                    ) : (
                      <Folder className="w-3.5 h-3.5 text-amber-400/80 shrink-0" />
                    )}
                  </>
                ) : (
                  <>
                    <span className="w-3 shrink-0" />
                    <FileCode className={`w-3.5 h-3.5 shrink-0 ${isSelected ? 'text-brand-blue' : 'text-neutral-500'}`} />
                  </>
                )}
                <span className={`truncate ${item.name.endsWith('.md') ? 'text-amber-200/90' : ''}`}>
                  {item.name}
                </span>
              </div>

              {item.status === 'modified' && (
                <span className="text-[10px] font-mono font-bold text-amber-400 shrink-0 ml-1">
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
    <aside className="w-80 bg-[#0c0e14] border-l border-[#1c202c] flex flex-col shrink-0 z-20 select-none overflow-hidden h-full font-sans">
      {/* 1. Top Activity Bar */}
      <div className="h-9 border-b border-[#1c202c] px-2 flex items-center justify-between shrink-0 bg-[#0c0e14]">
        <div className="flex items-center gap-0.5">
          {/* Files Explorer */}
          <button
            onClick={() => setActiveTab('explorer')}
            className={`p-1.5 rounded-md transition ${
              activeTab === 'explorer'
                ? 'bg-[#181d28] text-white shadow-sm'
                : 'text-neutral-500 hover:text-neutral-300 hover:bg-[#141722]'
            }`}
            title="File Explorer (⌘E)"
          >
            <Files className="w-3.5 h-3.5" />
          </button>

          {/* INV-03 Pending Permits */}
          <button
            onClick={() => setActiveTab('permits')}
            className={`p-1.5 rounded-md transition relative ${
              activeTab === 'permits'
                ? 'bg-[#181d28] text-white shadow-sm'
                : 'text-neutral-500 hover:text-neutral-300 hover:bg-[#141722]'
            }`}
            title="Authority Engine: Pending Permits (INV-03)"
          >
            <ShieldAlert className="w-3.5 h-3.5 text-amber-400" />
            {pendingPermitsCount > 0 && (
              <span className="absolute -top-0.5 -right-0.5 w-3.5 h-3.5 rounded-full bg-amber-500 text-black text-[9px] font-mono font-bold flex items-center justify-center">
                {pendingPermitsCount}
              </span>
            )}
          </button>

          {/* INV-05 Evidence Gate */}
          <button
            onClick={() => setActiveTab('evidence')}
            className={`p-1.5 rounded-md transition ${
              activeTab === 'evidence'
                ? 'bg-[#181d28] text-white shadow-sm'
                : 'text-neutral-500 hover:text-neutral-300 hover:bg-[#141722]'
            }`}
            title="Evidence Gate & Criteria (INV-05)"
          >
            <FileCheck className="w-3.5 h-3.5 text-cyan-400" />
          </button>

          {/* Git Source Control */}
          <button
            onClick={() => setActiveTab('git')}
            className={`p-1.5 rounded-md transition relative ${
              activeTab === 'git'
                ? 'bg-[#181d28] text-white shadow-sm'
                : 'text-neutral-500 hover:text-neutral-300 hover:bg-[#141722]'
            }`}
            title="Source Control (⌘⇧G)"
          >
            <GitBranch className="w-3.5 h-3.5" />
          </button>

          {/* INV-01 & INV-09 Audit Trail */}
          <button
            onClick={() => setActiveTab('audit')}
            className={`p-1.5 rounded-md transition ${
              activeTab === 'audit'
                ? 'bg-[#181d28] text-white shadow-sm'
                : 'text-neutral-500 hover:text-neutral-300 hover:bg-[#141722]'
            }`}
            title="Audit Trail & Actor Attribution (INV-01, INV-09)"
          >
            <History className="w-3.5 h-3.5 text-purple-400" />
          </button>

          {/* Ports */}
          <button
            onClick={() => setActiveTab('ports')}
            className={`p-1.5 rounded-md transition ${
              activeTab === 'ports'
                ? 'bg-[#181d28] text-white shadow-sm'
                : 'text-neutral-500 hover:text-neutral-300 hover:bg-[#141722]'
            }`}
            title="Listening Ports"
          >
            <Radio className="w-3.5 h-3.5" />
          </button>
        </div>

        {/* Close Button */}
        <button
          onClick={onClose}
          className="p-1 rounded-md hover:bg-[#181d28] text-neutral-500 hover:text-white transition"
          title="Close Inspector (⌘E)"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      </div>

      {/* 2. BODY CONTENT BASED ON ACTIVE TAB */}

      {/* TAB 1: FILES EXPLORER */}
      {activeTab === 'explorer' && (
        <div className="flex-1 flex flex-col overflow-hidden min-h-0">
          <div className="p-2 border-b border-[#1c202c] flex items-center justify-between gap-2 shrink-0">
            <div className="flex items-center gap-1 bg-[#131620] p-0.5 rounded-lg border border-[#202534] w-full">
              <button
                onClick={() => setFileSearchMode('names')}
                className={`flex-1 py-0.5 rounded-md text-[10.5px] font-medium transition text-center ${
                  fileSearchMode === 'names'
                    ? 'bg-[#1c2130] text-white font-semibold shadow-sm'
                    : 'text-neutral-400 hover:text-white'
                }`}
              >
                Names
              </button>
              <button
                onClick={() => setFileSearchMode('contents')}
                className={`flex-1 py-0.5 rounded-md text-[10.5px] font-medium transition text-center ${
                  fileSearchMode === 'contents'
                    ? 'bg-[#1c2130] text-white font-semibold shadow-sm'
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
                className="w-full bg-[#131620] border border-[#202534] rounded-lg pl-7 pr-2 py-1 text-[11px] text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-brand-blue font-mono transition"
              />
            </div>
          </div>

          <div className="flex-1 overflow-y-auto px-2 space-y-0.5 min-w-0 font-mono text-[11px]">
            {renderTree(REPO_FILE_TREE)}
          </div>
        </div>
      )}

      {/* TAB 2: INV-03 PENDING PERMITS (AUTHORITY ENGINE) */}
      {activeTab === 'permits' && (
        <div className="flex-1 flex flex-col overflow-hidden min-h-0 p-3 space-y-3">
          <div className="flex items-center justify-between text-xs pb-1 border-b border-[#1c202c]">
            <div className="flex items-center gap-1.5 font-semibold text-white">
              <ShieldAlert className="w-4 h-4 text-amber-400" />
              <span>Authority Engine</span>
            </div>
            {pendingPermitsCount > 0 && (
              <button
                onClick={handleApproveAll}
                className="text-[10px] text-emerald-400 hover:text-emerald-300 font-medium transition"
              >
                Approve All ({pendingPermitsCount})
              </button>
            )}
          </div>

          <p className="text-[10.5px] text-neutral-400 leading-normal">
            Side effects intercepted by <code className="text-neutral-300 font-mono">PermitGate</code>. Actions are blocked until granted.
          </p>

          <div className="flex-1 overflow-y-auto space-y-2.5">
            {permits.map((permit) => (
              <div 
                key={permit.id}
                className={`p-3 rounded-xl border transition ${
                  permit.status === 'approved'
                    ? 'bg-emerald-950/10 border-emerald-500/30'
                    : permit.status === 'denied'
                    ? 'bg-red-950/10 border-red-500/30 opacity-60'
                    : 'bg-[#131622] border-[#22283a]'
                }`}
              >
                <div className="flex items-center justify-between text-[10px] font-mono mb-1.5">
                  <span className="font-bold text-white flex items-center gap-1">
                    <span className="px-1.5 py-0.2 rounded bg-amber-500/20 text-amber-300 font-bold">
                      {permit.action}
                    </span>
                    <span>{permit.id}</span>
                  </span>
                  <span className="text-neutral-500">{permit.timestamp}</span>
                </div>

                <div className="text-[11px] font-mono text-neutral-200 truncate mb-1" title={permit.target}>
                  {permit.target}
                </div>

                <p className="text-[10.5px] text-neutral-400 leading-relaxed mb-2">
                  {permit.payloadSummary}
                </p>

                {permit.tainted && (
                  <div className="mb-2 p-1.5 rounded bg-red-500/10 border border-red-500/20 flex items-center gap-1.5 text-[9.5px] text-red-300 font-mono">
                    <AlertTriangle className="w-3 h-3 text-red-400 shrink-0" />
                    <span>Taint::Untrusted input detected (INV-04)</span>
                  </div>
                )}

                <div className="flex items-center justify-between pt-1 border-t border-[#1c2232] text-[10px]">
                  <span className="text-neutral-500 font-mono">Actor: {permit.actor}</span>
                  
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
                        className="px-2.5 py-0.5 rounded bg-emerald-600 hover:bg-emerald-500 text-white font-medium shadow-sm transition flex items-center gap-1"
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

      {/* TAB 3: INV-05 EVIDENCE GATE & COMPLETION CLAIMS */}
      {activeTab === 'evidence' && (
        <div className="flex-1 flex flex-col overflow-hidden min-h-0 p-3 space-y-3">
          <div className="flex items-center justify-between text-xs pb-1 border-b border-[#1c202c]">
            <div className="flex items-center gap-1.5 font-semibold text-white">
              <FileCheck className="w-4 h-4 text-cyan-400" />
              <span>Evidence Gate (INV-05)</span>
            </div>
            <span className="text-[10px] font-mono text-cyan-400">2 / 3 PASS</span>
          </div>

          <p className="text-[10.5px] text-neutral-400 leading-normal">
            Completion criteria cannot pass without cryptographic CAS evidence and empirical logs.
          </p>

          <div className="flex-1 overflow-y-auto space-y-2">
            {criteria.map((crit) => (
              <div 
                key={crit.id}
                className="p-3 rounded-xl bg-[#131622] border border-[#202636] space-y-1.5"
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

                <p className="text-[10px] text-neutral-400 leading-relaxed">
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

          <div className="pt-2 border-t border-[#1c202c]">
            <button className="w-full py-1.5 rounded-lg bg-[#181d28] hover:bg-[#202736] text-neutral-200 hover:text-white border border-[#262e40] text-xs font-medium transition flex items-center justify-center gap-1.5">
              <RotateCw className="w-3 h-3 text-cyan-400" />
              <span>Verify All Evidence Claims</span>
            </button>
          </div>
        </div>
      )}

      {/* TAB 4: GIT SOURCE CONTROL */}
      {activeTab === 'git' && (
        <div className="flex-1 flex flex-col overflow-hidden min-h-0 p-3 space-y-3">
          <div className="flex items-center justify-between text-xs text-neutral-400">
            <span className="font-semibold text-neutral-200">Source Control</span>
            <span className="font-mono text-[10px] px-1.5 py-0.5 rounded bg-[#181d28] border border-[#232838] text-neutral-300">
              git:(vi)
            </span>
          </div>

          <div className="space-y-1.5">
            <textarea
              value={commitMessage}
              onChange={(e) => setCommitMessage(e.target.value)}
              placeholder="Commit message (⌘Enter to commit)"
              rows={3}
              className="w-full bg-[#131620] border border-[#202534] rounded-lg p-2 text-xs text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-brand-blue font-sans resize-none transition"
            />
            <div className="flex items-center justify-between">
              <button
                onClick={() => setCommitMessage('feat(governance): enforce INV-03 permits and INV-05 evidence gates in UI')}
                className="flex items-center gap-1 text-[10.5px] text-brand-blue hover:text-blue-400 transition"
              >
                <Sparkles className="w-3 h-3" />
                <span>Generate message</span>
              </button>
              <button
                disabled={!commitMessage.trim()}
                className="px-2.5 py-1 rounded-md bg-brand-blue disabled:opacity-40 text-white text-[11px] font-medium transition hover:bg-blue-600"
              >
                Commit
              </button>
            </div>
          </div>

          <div className="space-y-1 flex-1 overflow-y-auto">
            <div className="flex items-center justify-between text-[11px] text-neutral-400 font-semibold px-1">
              <span>CHANGES</span>
              <span className="font-mono text-[10px]">6</span>
            </div>

            <div className="space-y-0.5 font-mono text-[11px]">
              {[
                { file: 'WorkspaceTabBar.tsx', status: 'M' },
                { file: 'RightFileExplorer.tsx', status: 'M' },
                { file: 'DelegateTaskModal.tsx', status: 'A' },
                { file: 'TaskDetailsModal.tsx', status: 'A' },
                { file: 'ResearchWorkspace.tsx', status: 'M' },
                { file: 'Custos.md', status: 'M' }
              ].map((item) => (
                <div
                  key={item.file}
                  className="flex items-center justify-between px-2 py-1 rounded hover:bg-[#141722] cursor-pointer text-neutral-300"
                >
                  <span className="truncate">{item.file}</span>
                  <span className={`text-[10px] font-bold ${item.status === 'M' ? 'text-amber-400' : 'text-emerald-400'}`}>
                    {item.status}
                  </span>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* TAB 5: INV-01 & INV-09 AUDIT TRAIL */}
      {activeTab === 'audit' && (
        <div className="flex-1 flex flex-col overflow-hidden min-h-0 p-3 space-y-3">
          <div className="flex items-center justify-between text-xs pb-1 border-b border-[#1c202c]">
            <div className="flex items-center gap-1.5 font-semibold text-white">
              <History className="w-4 h-4 text-purple-400" />
              <span>Audit Ledger (INV-01)</span>
            </div>
            <span className="text-[10px] font-mono text-purple-400">4 Events</span>
          </div>

          <p className="text-[10.5px] text-neutral-400 leading-normal">
            Every state transition is attributed to a verifiable Actor. No anonymous execution.
          </p>

          <div className="flex-1 overflow-y-auto space-y-2">
            {auditEvents.map((ev) => (
              <div
                key={ev.id}
                className="p-2.5 rounded-xl bg-[#131622] border border-[#202636] space-y-1 text-xs"
              >
                <div className="flex items-center justify-between text-[10px] font-mono">
                  <span className="font-semibold text-purple-300">{ev.actor}</span>
                  <span className="text-neutral-500">{ev.time}</span>
                </div>
                <div className="text-[11px] text-neutral-200">
                  {ev.event}
                </div>
                <div className="flex items-center justify-between text-[9.5px] font-mono text-neutral-400 pt-1 border-t border-[#1c2232]">
                  <span>Transport: {ev.transport}</span>
                  <span className="text-emerald-400 font-medium">{ev.cost}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* TAB 6: PORTS */}
      {activeTab === 'ports' && (
        <div className="flex-1 flex flex-col overflow-hidden min-h-0 p-3 space-y-3">
          <div className="flex items-center justify-between text-xs text-neutral-400">
            <span className="font-semibold text-neutral-200">Listening Ports</span>
            <span className="text-[10px] font-mono text-emerald-400">3 active</span>
          </div>

          <div className="space-y-2">
            {[
              { port: '3000', process: 'vite (desktop web)', status: 'Listening' },
              { port: '4000', process: 'custos-daemon (IPC/RPC)', status: 'Listening' },
              { port: '1420', process: 'tauri dev server', status: 'Listening' }
            ].map((p) => (
              <div
                key={p.port}
                className="p-2.5 rounded-lg bg-[#131620] border border-[#202534] flex items-center justify-between text-xs"
              >
                <div>
                  <div className="font-mono text-white text-[11px]">localhost:{p.port}</div>
                  <div className="text-[10px] text-neutral-500">{p.process}</div>
                </div>
                <span className="text-[10px] font-mono text-emerald-400 flex items-center gap-1">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
                  {p.status}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </aside>
  );
};
