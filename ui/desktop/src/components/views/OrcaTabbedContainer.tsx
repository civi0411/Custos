import React, { useState } from 'react';
import { createPortal } from 'react-dom';
import {
  X,
  Plus,
  RotateCw,
  ArrowLeft,
  ArrowRight,
  Maximize2,
  Minimize2,
  MessageSquare,
  MoreHorizontal,
  FileDiff,
  Terminal as TerminalIcon,
  FolderTree,
  GitBranch,
  Bot,
  ShieldCheck,
  Activity,
  Globe,
  Search,
  CheckCircle2,
  Copy,
  Check,
  FileCode,
  Sliders,
  Sparkles,
  PanelRight
} from 'lucide-react';
import { Session } from '@/types';
import { WorktreeManagerModal, TaskDetailsModal } from '@/components/modals';
import { BrowserWorkspace } from '@/components/workspaces/browser/BrowserWorkspace';
import { MarkdownWorkspace } from '@/components/workspaces/markdown/MarkdownWorkspace';
import {
  LiteraturePane,
  ClaimsMatrixPane,
  NotebookWorkspacePane,
  RunsLedgerPane,
  DeepInspectorPane,
} from '@/components/research';


export type OrcaTabId = 'tools' | 'changes' | 'terminal' | 'files' | 'worktrees' | 'kanban' | 'evidence' | 'dag' | 'browser' | 'notes' | 'artifacts' | 'knowledge' | 'literature' | 'claims' | 'experiments' | 'synthesis';

export interface OrcaTab {
  id: OrcaTabId;
  title: string;
  url: string;
}

export interface ResourceTabsState {
  tabs: OrcaTab[];
  activeTabId: OrcaTabId;
  setTabs: React.Dispatch<React.SetStateAction<OrcaTab[]>>;
  setActiveTabId: React.Dispatch<React.SetStateAction<OrcaTabId>>;
}

interface RepoFile {
  path: string;
  name: string;
  language: string;
  status: 'M' | 'clean';
  code: string;
}

const REPO_FILES: RepoFile[] = [
  {
    path: 'crates/custos-runtime/src/workflow/dispatcher.rs',
    name: 'dispatcher.rs',
    language: 'rust',
    status: 'M',
    code: `// Custos Workflow Dispatcher Runtime
// Invariant Contract: INV-01 Double-Dispatch Fencing
use std::sync::Arc;
use custos_domain::action::Action;
use custos_core::authority::permits::PermitGate;
use tokio::sync::RwLock;

pub struct WorkflowDispatcher {
    permit_gate: Arc<PermitGate>,
    max_inflight_actions: usize,
    active_permits: RwLock<Vec<String>>,
}

impl WorkflowDispatcher {
    pub fn new(permit_gate: Arc<PermitGate>) -> Self {
        Self {
            permit_gate,
            max_inflight_actions: 64,
            active_permits: RwLock::new(Vec::new()),
        }
    }

    /// Claim ready task and verify pre-condition invariant contracts
    /// [INV-01]: Atomic ticket acquisition prevents phantom double-execution
    pub async fn claim_ready_task(&self, task_id: &str) -> Result<Action, String> {
        let ticket = self.permit_gate.acquire_ticket(task_id).await
            .map_err(|e| format!("Permit acquisition failed: {:?}", e))?;

        println!("Claimed task {} under invariant gate {:?}", task_id, ticket.id());
        
        let mut guard = self.active_permits.write().await;
        guard.push(ticket.id().to_string());

        Ok(Action::Execute(task_id.to_string()))
    }

    /// Release permit and seal execution proof into CAS ledger
    pub async fn finalize_task(&self, task_id: &str) -> Result<(), String> {
        self.permit_gate.release_ticket(task_id).await?;
        println!("Finalized task {} with zero-IO proof closure", task_id);
        Ok(())
    }
}`
  },
  {
    path: 'crates/custos-core/src/authority/permits.rs',
    name: 'permits.rs',
    language: 'rust',
    status: 'clean',
    code: `// Custos Authority & Permit Gate
// Enforces capability boundaries and execution permits
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct PermitTicket {
    id: String,
    task_id: String,
    capability: String,
    expires_at_epoch: u64,
}

pub struct PermitGate {
    tickets: Arc<Mutex<HashMap<String, PermitTicket>>>,
    strict_fencing: bool,
}

impl PermitGate {
    pub fn new(strict_fencing: bool) -> Self {
        Self {
            tickets: Arc::new(Mutex::new(HashMap::new())),
            strict_fencing,
        }
    }

    pub async fn acquire_ticket(&self, task_id: &str) -> Result<PermitTicket, String> {
        let mut map = self.tickets.lock().await;
        if map.contains_key(task_id) && self.strict_fencing {
            return Err(format!("Task {} already has an active lease", task_id));
        }
        let ticket = PermitTicket {
            id: format!("ticket-{}", uuid::Uuid::new_v4().simple()),
            task_id: task_id.to_string(),
            capability: "workspace:write".to_string(),
            expires_at_epoch: 1769820000,
        };
        map.insert(task_id.to_string(), ticket.clone());
        Ok(ticket)
    }
}`
  },
  {
    path: 'crates/custos-runtime/src/oi/estimator.rs',
    name: 'estimator.rs',
    language: 'rust',
    status: 'clean',
    code: `// Operator Induction (OI) Estimator
use custos_domain::oi::Candidate;

pub struct OiEstimator {
    temperature: f32,
    p_success_threshold: f32,
}

impl OiEstimator {
    pub fn new() -> Self {
        Self {
            temperature: 0.2,
            p_success_threshold: 0.85,
        }
    }

    pub fn evaluate_candidate(&self, candidate: &Candidate) -> f32 {
        let complexity_penalty = (candidate.step_count as f32) * 0.04;
        let base_score = candidate.prior_score.clamp(0.0, 1.0);
        (base_score - complexity_penalty).max(0.0)
    }
}`
  },
  {
    path: 'crates/custos-domain/src/workflow.rs',
    name: 'workflow.rs',
    language: 'rust',
    status: 'clean',
    code: `// Workflow Domain Entities
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSpec {
    pub id: String,
    pub name: String,
    pub strategy: String,
    pub invariant_checks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    Execute(String),
    AwaitApproval(String),
    Abort(String),
}`
  }
];

interface OrcaTabbedContainerProps {
  resourceTabs: ResourceTabsState;
  isPaneOpen: boolean;
  session?: Session | null;
  onAcceptAndRun?: () => void;
  onRejectDiff?: () => void;
  onCopyDiff?: () => void;
  onShowToast?: (msg: string) => void;
  isExpanded?: boolean;
  onToggleExpand?: () => void;
  mode?: 'chat' | 'code' | 'research';
  onAskAgent?: (draftText: string) => void;
  onHandoffToCoding?: (claims: any[]) => void;
  splitPercent?: number;
  onToggleWebTab?: () => void;
}

export const OrcaTabbedContainer: React.FC<OrcaTabbedContainerProps> = ({
  resourceTabs,
  isPaneOpen,
  session,
  onAcceptAndRun,
  onRejectDiff,
  onCopyDiff: _onCopyDiff,
  onShowToast,
  isExpanded = false,
  onToggleExpand,
  mode = 'code',
  onAskAgent,
  onHandoffToCoding,
  splitPercent = 45,
  onToggleWebTab,
}) => {
  
  // Tabs management
  const { tabs, setTabs, activeTabId, setActiveTabId } = resourceTabs;
  
  const [urlInput, setUrlInput] = useState<string>('');
  const [isUrlEditing, setIsUrlEditing] = useState<boolean>(false);
  const [copiedCode, setCopiedCode] = useState<boolean>(false);

  // File editor states (Vinh's EngineeringWorkspace)
  const [selectedFile, setSelectedFile] = useState<RepoFile>(REPO_FILES[0]);
  const [isCodexPromptOpen, setIsCodexPromptOpen] = useState<boolean>(false);
  const [codexInput, setCodexInput] = useState<string>('');
  const [codexGenerating, setCodexGenerating] = useState<boolean>(false);

  // Worktree lifecycle modal state
  const [isWorktreeModalOpen, setIsWorktreeModalOpen] = useState(false);
  const [activeWorktreeBranch, setActiveWorktreeBranch] = useState('wt-simd');

  // Task Details Modal state
  const [isTaskDetailsModalOpen, setIsTaskDetailsModalOpen] = useState(false);
  const [selectedTask, setSelectedTask] = useState({
    id: 'T-01',
    title: 'Invariant Contract Fencing Gate in Dispatcher',
    status: 'Running',
    model: 'Claude 3.7 Sonnet',
    budgetUsed: 0.42,
    budgetLimit: 1.50,
    turnsUsed: 3,
    maxTurns: 8,
    actor: 'Codex Synthesizer',
    criteria: [
      { id: 'c1', label: 'INV-01 Double-Dispatch Fencing Proof Closed', status: 'pass' as const, evidenceHash: 'bafy2bzace4v3k99a' },
      { id: 'c2', label: 'Cargo build and tests pass', status: 'pass' as const, evidenceHash: 'bafy2bzace77t211x' },
      { id: 'c3', label: 'Zero-IO strict barrier verified', status: 'pending' as const }
    ]
  });

  // Terminal state with 4 subtabs (Vinh's Scoped Mediated Terminal)
  const [terminalTab, setTerminalTab] = useState<'shell' | 'build' | 'tests' | 'invariants'>('shell');
  const [terminalLines, setTerminalLines] = useState<{ owner: 'mediated' | 'manual' | 'system'; text: string; time: string }[]>([
    { owner: 'system', text: 'Custos SADE Scoped Terminal — Mediated Execution Runtime v0.1.0', time: '13:20:12' },
    { owner: 'mediated', text: 'cargo check --workspace --tests', time: '13:21:04' },
    { owner: 'system', text: '   Compiling custos-domain v0.1.0 (/Users/mac/Project/AgentHub/Custos/crates/custos-domain)', time: '13:21:05' },
    { owner: 'system', text: '   Compiling custos-core v0.1.0 (/Users/mac/Project/AgentHub/Custos/crates/custos-core)', time: '13:21:06' },
    { owner: 'system', text: '   Compiling custos-runtime v0.1.0 (/Users/mac/Project/AgentHub/Custos/crates/custos-runtime)', time: '13:21:07' },
    { owner: 'system', text: '    Finished dev [unoptimized + debuginfo] target(s) in 1.84s (0 errors, 0 warnings)', time: '13:21:08' },
    { owner: 'mediated', text: 'custos check --invariants INV-01,INV-03', time: '13:21:45' },
    { owner: 'system', text: '   ✓ [INV-01 Double-Dispatch Fencing]: Proof closure valid (CAS: bafy2bzace4v3k99a)', time: '13:21:46' },
    { owner: 'system', text: '   ✓ [INV-03 Zero-IO Ticket Gate]: Strict lease active for task: wt-simd', time: '13:21:46' },
    { owner: 'manual', text: 'git status -s', time: '13:22:10' },
    { owner: 'system', text: ' M crates/custos-runtime/src/workflow/dispatcher.rs', time: '13:22:10' },
  ]);
  const [termInput, setTermInput] = useState('');

  // Active Execution Permits (Vinh's Zero-IO capability gate)
  const [permits, setPermits] = useState([
    { id: 'pmt-01', action: 'fs:write', actor: 'Codex Synthesizer', target: 'crates/custos-runtime/src/workflow/dispatcher.rs', status: 'approved' as 'pending' | 'approved' | 'denied', timestamp: '2m ago' },
    { id: 'pmt-02', action: 'exec:bash', actor: 'Worker-1', target: 'cargo test --lib', status: 'approved' as 'pending' | 'approved' | 'denied', timestamp: '1m ago' },
    { id: 'pmt-03', action: 'net:fetch', actor: 'Research Oracle', target: 'api.arxiv.org/query', status: 'pending' as 'pending' | 'approved' | 'denied', timestamp: 'Just now' },
    { id: 'pmt-04', action: 'mcp:call', actor: 'AST Specialist', target: 'mcp://repo-intelligence/symbols', status: 'approved' as 'pending' | 'approved' | 'denied', timestamp: '5m ago' }
  ]);

  // Kanban agent states
  const [kanbanAgents] = useState([
    { id: 'ag-1', name: 'S2 Master Planner', status: 'working', role: 'Hierarchical Task Decomposition', model: 'Sonnet 5.5', tokens: '4.2k' },
    { id: 'ag-2', name: 'Codex Synthesizer', status: 'working', role: 'Rust Invariant Implementation', model: 'Sonnet 5.5', tokens: '6.8k' },
    { id: 'ag-3', name: 'Permit Auditor', status: 'needs_you', role: 'Pending review for fs:write permit', model: 'Custos Policy', tokens: '1.1k' },
    { id: 'ag-4', name: 'CAS Proof Sealer', status: 'done', role: 'Proof closed bafy2bzace4v3k99a', model: 'DeepSeek V3', tokens: '890' },
    { id: 'ag-5', name: 'Benchmarker Worker', status: 'idle', role: 'Standby on SIMD runner', model: 'Local Daemon', tokens: '0' }
  ]);

  // Open or switch to a tool tab
  const handleOpenTool = (toolId: OrcaTabId, title: string, url: string) => {
    const existing = tabs.find((t) => t.id === toolId);
    if (existing) {
      setActiveTabId(toolId);
    } else {
      setTabs((prev) => [...prev, { id: toolId, title, url }]);
      setActiveTabId(toolId);
    }
    setUrlInput(url);
    if (onShowToast) onShowToast(`Opened ${title}`);
  };

  const handleCloseTab = (e: React.MouseEvent, tabId: OrcaTabId) => {
    e.stopPropagation();
    if (tabs.length <= 1) {
      setTabs([{ id: 'tools', title: 'Resources', url: 'custos://resources' }]);
      setActiveTabId('tools');
      setUrlInput('');
      return;
    }
    const nextTabs = tabs.filter((t) => t.id !== tabId);
    setTabs(nextTabs);
    if (activeTabId === tabId) {
      setActiveTabId(nextTabs[nextTabs.length - 1].id);
    }
  };

  const handleAddNewTab = () => {
    setTabs((prev) => prev.some((tab) => tab.id === 'tools') ? prev : [...prev, { id: 'tools', title: 'Resources', url: 'custos://resources' }]);
    setActiveTabId('tools');
    setUrlInput('');
  };

  const activeTab = tabs.find((t) => t.id === activeTabId) || tabs[0];

  // Tool Card component matching Screenshot 2
  const ToolCard = ({
    title,
    shortcut,
    icon: Icon,
    onClick,
    description
  }: {
    title: string;
    shortcut?: string;
    icon: React.ComponentType<{ className?: string }>;
    onClick: () => void;
    description?: string;
  }) => (
    <div
      onClick={onClick}
      className="flex items-center justify-between p-3.5 rounded-xl border border-[#21262d] bg-[#161b22]/70 hover:bg-[#21262d] hover:border-[#384252] cursor-pointer transition group shadow-sm"
    >
      <div className="flex items-center gap-3">
        <div className="w-8 h-8 rounded-lg bg-[#0d1117] border border-[#30363d] flex items-center justify-center text-[#8b949e] group-hover:text-white group-hover:border-[#58a6ff] transition">
          <Icon className="w-4 h-4" />
        </div>
        <div>
          <div className="text-[13px] font-medium text-white group-hover:text-[#58a6ff] transition">
            {title}
          </div>
          <div className="text-[11px] text-[#8b949e] font-sans">
            {description}
          </div>
        </div>
      </div>
      <div className="flex items-center gap-2">
        {shortcut && (
          <kbd className="px-1.5 py-0.5 rounded bg-[#0d1117] border border-[#30363d] text-[10px] font-mono text-[#8b949e]">
            {shortcut}
          </kbd>
        )}
        <MoreHorizontal className="w-4 h-4 text-[#484f58] group-hover:text-[#8b949e]" />
      </div>
    </div>
  );


  const portalTarget = typeof document !== 'undefined' ? document.getElementById('app-header-tabs-portal') : null;
  const tabStrip = (
    <div className="h-full flex w-full overflow-hidden">
      {/* ── LEFT: CHAT HEADER (Matches Chat Column Width) ── */}
      {!isExpanded && isPaneOpen && (
        <div style={{ width: `${splitPercent}%` }} className="h-full flex items-center px-4 border-r border-[#262c36] shrink-0 bg-[#090d13]">
          <div className="flex items-center gap-2 text-[12px] text-[#8b949e] hover:text-[#c9d1d9] transition cursor-pointer">
            <MessageSquare className="w-3.5 h-3.5" />
            <span className="truncate font-medium">{session?.title || (mode === 'code' ? 'Engineering Copilot' : 'Research Assistant')}</span>
          </div>
        </div>
      )}

      {/* ── RIGHT: TABS ROW ── */}
      <div style={{ width: isExpanded || !isPaneOpen ? '100%' : `${100 - splitPercent}%` }} className="h-full flex items-center justify-between px-2 gap-1 overflow-hidden shrink-0">
        <div className="flex items-center gap-1 min-w-0 flex-1 h-full overflow-x-auto no-scrollbar pl-1">
          {/* Chat Tab (Only visible when expanded, clicking it un-expands) */}
          {isExpanded && (
            <>
              <div
                onClick={onToggleExpand}
                className="group flex items-center gap-2 px-3 py-1.5 rounded-md cursor-pointer transition max-w-[180px] shrink-0 text-[12px] h-7 bg-transparent text-[#8b949e] hover:text-white hover:bg-[#161b22]"
                title="Back to Chat"
              >
                <MessageSquare className="w-3.5 h-3.5" />
                <span className="truncate font-medium">{session?.title || 'Chat'}</span>
              </div>
              <div className="w-px h-4 bg-[#30363d] mx-1 shrink-0" />
            </>
          )}

          {tabs.map((tab) => {
            const isActive = tab.id === activeTabId;
            return (
              <div
                key={tab.id}
                onClick={() => {
                  setActiveTabId(tab.id);
                  setUrlInput(tab.url);
                }}
                className={`group flex items-center gap-2 px-3 py-1.5 rounded-md cursor-pointer transition max-w-[180px] shrink-0 text-[12px] h-7 ${
                  isActive
                    ? 'bg-[#21262d] text-white font-medium shadow-sm'
                    : 'bg-transparent text-[#8b949e] hover:text-[#c9d1d9] hover:bg-[#161b22]'
                }`}
              >
                <div className="flex items-center gap-1.5 truncate">
                  {tab.id === 'tools' && <Globe className="w-3.5 h-3.5 text-[#58a6ff]" />}
                  {tab.id === 'changes' && <FileDiff className="w-3.5 h-3.5 text-[#3fb950]" />}
                  {tab.id === 'terminal' && <TerminalIcon className="w-3.5 h-3.5 text-[#e3b341]" />}
                  {tab.id === 'files' && <FolderTree className="w-3.5 h-3.5 text-[#58a6ff]" />}
                  {tab.id === 'worktrees' && <GitBranch className="w-3.5 h-3.5 text-[#a371f7]" />}
                  {tab.id === 'kanban' && <Bot className="w-3.5 h-3.5 text-[#bc8cff]" />}
                  {tab.id === 'evidence' && <ShieldCheck className="w-3.5 h-3.5 text-[#3fb950]" />}
                  {tab.id === 'dag' && <Activity className="w-3.5 h-3.5 text-[#58a6ff]" />}
                  {tab.id === 'browser' && <Globe className="w-3.5 h-3.5 text-[#58a6ff]" />}
                  {tab.id === 'notes' && <FileCode className="w-3.5 h-3.5 text-[#e3b341]" />}
                  {tab.id === 'literature' && <FolderTree className="w-3.5 h-3.5 text-[#a371f7]" />}
                  {tab.id === 'claims' && <ShieldCheck className="w-3.5 h-3.5 text-[#a371f7]" />}
                  {tab.id === 'experiments' && <Activity className="w-3.5 h-3.5 text-[#a371f7]" />}
                  {tab.id === 'synthesis' && <FileCode className="w-3.5 h-3.5 text-[#a371f7]" />}
                  <span className="truncate">{tab.title}</span>
                </div>
                <button
                  onClick={(e) => handleCloseTab(e, tab.id)}
                  className="opacity-0 group-hover:opacity-100 hover:text-white p-0.5 rounded transition text-[#8b949e] shrink-0"
                  title="Close tab"
                >
                  <X className="w-3 h-3" />
                </button>
              </div>
            );
          })}

          <button
            onClick={handleAddNewTab}
            className="p-1 rounded-md text-[#8b949e] hover:text-white hover:bg-[#21262d] transition ml-1 shrink-0"
            title="New tab"
          >
            <Plus className="w-3.5 h-3.5" />
          </button>
        </div>

        {/* Tab Right Controls */}
        <div className="flex items-center gap-1 shrink-0 text-[#8b949e] px-2 border-l border-[#262c36]/60">
          {onToggleExpand && (
            <button
              onClick={onToggleExpand}
              className="p-1 rounded-md hover:text-white hover:bg-[#21262d] transition"
              title={isExpanded ? 'Restore pane' : 'Maximize tab pane'}
            >
              {isExpanded ? <Minimize2 className="w-3.5 h-3.5" /> : <Maximize2 className="w-3.5 h-3.5" />}
            </button>
          )}
          {onToggleWebTab && (
            <button
              onClick={onToggleWebTab}
              className="p-1 rounded-md hover:text-white hover:bg-[#21262d] transition"
              title="Close resource pane"
            >
              <PanelRight className="w-3.5 h-3.5" />
            </button>
          )}
        </div>
      </div>
    </div>
  );

  return (
    <div className="flex flex-col h-full w-full bg-[#04080F] border-l border-[#262c36] overflow-hidden select-none font-sans text-xs">
      {isPaneOpen && portalTarget && createPortal(tabStrip, portalTarget)}

      {/* 2. Sub-navigation Address Bar (Screenshot 2 Sub-header) */}
      <div className="h-9 bg-[#080d16] border-b border-[#262c36]/60 flex items-center px-2.5 gap-2 shrink-0">
        {activeTabId !== 'tools' && !['literature', 'claims', 'experiments', 'synthesis'].includes(activeTabId) && (
          <span className="shrink-0 rounded border border-amber-600/40 px-1.5 py-0.5 text-[10px] text-amber-300" title="This resource view still contains prototype fixtures">Demo preview</span>
        )}
        <div className="flex items-center gap-0.5 text-[#8b949e]">
          <button
            onClick={() => setActiveTabId('tools')}
            className="p-1 rounded hover:text-white hover:bg-[#21262d] transition"
            title="Back"
          >
            <ArrowLeft className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={() => {
              if (onShowToast) onShowToast('Forward');
            }}
            className="p-1 rounded hover:text-white hover:bg-[#21262d] transition"
            title="Forward"
          >
            <ArrowRight className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={() => {
              if (onShowToast) onShowToast('Reloaded tab');
            }}
            className="p-1 rounded hover:text-white hover:bg-[#21262d] transition"
            title="Reload"
          >
            <RotateCw className="w-3.5 h-3.5" />
          </button>
        </div>

        {/* Address / Search Bar */}
        <div className="flex-1 flex items-center h-6 px-2.5 rounded-lg bg-[#0d1117] border border-[#30363d] focus-within:border-[#58a6ff] transition">
          <Search className="w-3 h-3 text-[#484f58] mr-2 shrink-0" />
          <input
            type="text"
            placeholder="Search or enter a URL"
            value={isUrlEditing ? urlInput : (activeTab.url || '')}
            onFocus={() => setIsUrlEditing(true)}
            onBlur={() => setIsUrlEditing(false)}
            onChange={(e) => setUrlInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                if (urlInput.startsWith('http')) {
                  handleOpenTool('browser', 'Browser Preview', urlInput);
                } else if (urlInput.toLowerCase().includes('term')) {
                  handleOpenTool('terminal', 'Terminal', 'orca://terminal');
                } else if (urlInput.toLowerCase().includes('change') || urlInput.toLowerCase().includes('diff')) {
                  handleOpenTool('changes', 'Changes', 'orca://changes');
                } else if (urlInput.toLowerCase().includes('file')) {
                  handleOpenTool('files', 'Files', 'orca://files');
                } else if (urlInput.toLowerCase().includes('worktree')) {
                  handleOpenTool('worktrees', 'Worktrees', 'orca://worktrees');
                } else if (urlInput.toLowerCase().includes('kanban') || urlInput.toLowerCase().includes('agent')) {
                  handleOpenTool('kanban', 'Fleet Kanban', 'orca://kanban');
                }
              }
            }}
            className="w-full bg-transparent border-none outline-none text-[#c9d1d9] text-[11px] placeholder-[#484f58]"
          />
        </div>

        {/* More Actions */}
        <button
          onClick={() => { if (onShowToast) onShowToast('Site Info'); }}
          className="p-1 mr-1 rounded text-[#8b949e] hover:text-white hover:bg-[#21262d] transition"
        >
          <ShieldCheck className="w-4 h-4" />
        </button>
        <button
          onClick={() => {
            if (onShowToast) onShowToast('ADE Tools options');
          }}
          className="p-1 rounded text-[#8b949e] hover:text-white hover:bg-[#21262d] transition"
        >
          <MoreHorizontal className="w-4 h-4" />
        </button>
      </div>

      {/* 3. Tab Body Container */}
      <div className="flex-1 overflow-y-auto overflow-x-hidden min-h-0 bg-[#04080F]">
        {/* VIEW A: TOOLS HOME PAGE (Exact Screenshot 2 Tools Page) */}
        {activeTabId === 'tools' && (
          <div className="px-6 py-4 max-w-3xl mx-auto space-y-1 select-text">
            <div>
              <h2 className="text-[12px] font-semibold text-[#8b949e] uppercase tracking-wider mb-2">
                Tools
              </h2>
            </div>

            {/* Tight vertical list matching Screenshot 3 */}
            <div className="flex flex-col space-y-0.5">
              {mode === 'code' && (
                <>
                  <ToolCard title="Changes" shortcut="^⇧G" icon={FileDiff} onClick={() => handleOpenTool('changes', 'Changes', 'orca://changes')} />
                  <ToolCard title="Terminal" shortcut="^`" icon={TerminalIcon} onClick={() => handleOpenTool('terminal', 'Terminal', 'orca://terminal')} />
                  <ToolCard title="Files" shortcut="⌘P" icon={FolderTree} onClick={() => handleOpenTool('files', 'Files', 'orca://files')} />
                  <ToolCard title="Worktrees" shortcut="⌘⌥W" icon={GitBranch} onClick={() => handleOpenTool('worktrees', 'Worktrees', 'orca://worktrees')} />
                  <ToolCard title="Fleet Kanban" shortcut="⌘⌥K" icon={Bot} onClick={() => handleOpenTool('kanban', 'Fleet Kanban', 'orca://kanban')} />
                  <ToolCard title="DAG & Telemetry" shortcut="⌘⌥D" icon={Activity} onClick={() => handleOpenTool('dag', 'DAG & Telemetry', 'orca://dag')} />
                </>
              )}
              {mode === 'chat' && (
                <>
                  <ToolCard title="Artifacts" shortcut="⌘⌥A" icon={FolderTree} onClick={() => handleOpenTool('artifacts', 'Artifacts', 'orca://artifacts')} />
                  <ToolCard title="Knowledge Base" shortcut="⌘⌥K" icon={ShieldCheck} onClick={() => handleOpenTool('knowledge', 'Knowledge Base', 'orca://knowledge')} />
                  <ToolCard title="Fleet Kanban" shortcut="⌘⌥K" icon={Bot} onClick={() => handleOpenTool('kanban', 'Fleet Kanban', 'orca://kanban')} />
                </>
              )}
              {mode === 'research' && (
                <>
                  <ToolCard title="Browser Preview" shortcut="⌘⌥B" icon={Globe} onClick={() => handleOpenTool('browser', 'Browser Preview', 'https://')} />
                  <ToolCard title="Literature" shortcut="⌘⌥L" icon={FolderTree} onClick={() => handleOpenTool('literature', 'Literature', 'orca://literature')} />
                  <ToolCard title="Claims" icon={ShieldCheck} onClick={() => handleOpenTool('claims', 'Claims', 'custos://research/claims')} />
                  <ToolCard title="Experiments" icon={Activity} onClick={() => handleOpenTool('experiments', 'Experiments', 'custos://research/experiments')} />
                  <ToolCard title="Synthesis" icon={FileCode} onClick={() => handleOpenTool('synthesis', 'Synthesis', 'custos://research/synthesis')} />
                </>
              )}
            </div>
          </div>
        )}

        {/* VIEW B: CHANGES / DIFF REVIEW */}
        {activeTabId === 'changes' && (
          <div className="h-full flex flex-col p-4 space-y-4 select-text">
            <div className="flex items-center justify-between border-b border-[#21262d] pb-3">
              <div>
                <div className="flex items-center gap-2">
                  <span className="font-semibold text-white text-[13px]">
                    {session?.fileName || 'crates/custos-runtime/src/workflow/dispatcher.rs'}
                  </span>
                  <span className="px-1.5 py-0.5 rounded text-[10px] bg-emerald-500/10 text-emerald-400 font-mono">
                    {session?.diffLinesCount || '+34 -2 lines'}
                  </span>
                </div>
                <div className="text-[11px] text-[#8b949e] mt-0.5">
                  Invariant: INV-01 Atomic Double-Dispatch Fencing
                </div>
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={onRejectDiff}
                  className="px-2.5 py-1 rounded-md bg-[#21262d] hover:bg-[#30363d] text-[#c9d1d9] text-[11px] font-medium transition"
                >
                  Reject
                </button>
                <button
                  onClick={onAcceptAndRun}
                  className="flex items-center gap-1.5 px-3 py-1 rounded-md bg-emerald-600 hover:bg-emerald-500 text-white text-[11px] font-semibold transition shadow-sm"
                >
                  <CheckCircle2 className="w-3.5 h-3.5" />
                  Accept & Run
                </button>
              </div>
            </div>

            {/* Code diff lines */}
            <div className="flex-1 rounded-xl bg-[#010409] border border-[#21262d] font-mono text-[11px] p-3 overflow-auto leading-relaxed">
              {(session?.diffCode || [
                { type: 'context', text: 'impl WorkflowDispatcher {' },
                { type: 'context', text: '    pub async fn claim_ready_task(&self, task_id: &str) -> Result<Action, String> {' },
                { type: 'del', text: '-       let ticket = self.permit_gate.acquire(task_id).await?;' },
                { type: 'add', text: '+       // [INV-01]: Atomic ticket acquisition prevents phantom double-execution' },
                { type: 'add', text: '+       let ticket = self.permit_gate.acquire_ticket(task_id).await' },
                { type: 'add', text: '+           .map_err(|e| format!("Permit acquisition failed: {:?}", e))?;' },
                { type: 'add', text: '+       let mut guard = self.active_permits.write().await;' },
                { type: 'add', text: '+       guard.push(ticket.id().to_string());' },
                { type: 'context', text: '        Ok(Action::Execute(task_id.to_string()))' },
                { type: 'context', text: '    }' },
                { type: 'context', text: '}' }
              ]).map((line, idx) => {
                const isAdd = line.type === 'add';
                const isDel = line.type === 'del';
                return (
                  <div
                    key={idx}
                    className={`flex items-center px-2 py-0.5 ${
                      isAdd ? 'bg-emerald-500/15 text-emerald-300' : isDel ? 'bg-rose-500/15 text-rose-300' : 'text-[#8b949e]'
                    }`}
                  >
                    <span className="w-8 shrink-0 text-[#484f58] select-none text-[10px]">{idx + 1}</span>
                    <span className="w-4 shrink-0 font-bold select-none">{isAdd ? '+' : isDel ? '-' : ' '}</span>
                    <span className="whitespace-pre">{line.text.replace(/^[+-]/, '')}</span>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* VIEW C: TERMINAL (4-Subtab Mediated Scoped Terminal) */}
        {activeTabId === 'terminal' && (
          <div className="h-full flex flex-col bg-[#010409] font-mono text-[11px] text-[#c9d1d9] select-text">
            {/* Top Subtab Bar */}
            <div className="flex items-center justify-between px-3 py-2 border-b border-[#21262d] bg-[#0d1117]">
              <div className="flex items-center gap-1 bg-[#161b22] p-0.5 rounded-lg border border-[#30363d]">
                {(['shell', 'build', 'tests', 'invariants'] as const).map((sub) => (
                  <button
                    key={sub}
                    onClick={() => setTerminalTab(sub)}
                    className={`px-2.5 py-1 rounded text-[10.5px] uppercase font-semibold transition ${
                      terminalTab === sub
                        ? 'bg-[#21262d] text-white shadow-sm'
                        : 'text-[#8b949e] hover:text-white'
                    }`}
                  >
                    {sub}
                  </button>
                ))}
              </div>

              {/* Quick Actions */}
              <div className="flex items-center gap-1.5">
                {[
                  { cmd: 'cargo check', tab: 'build' },
                  { cmd: 'npm run dev', tab: 'shell' },
                  { cmd: 'cargo test', tab: 'tests' },
                  { cmd: 'custos verify', tab: 'invariants' }
                ].map(({ cmd, tab }) => (
                  <button
                    key={cmd}
                    onClick={() => {
                      const now = new Date().toLocaleTimeString();
                      setTerminalLines((prev) => [
                        ...prev,
                        { owner: 'manual', text: cmd, time: now },
                        { owner: 'system', text: `✓ [${tab}] Execution completed with status 0`, time: now }
                      ]);
                      if (onShowToast) onShowToast(`Executed: ${cmd}`);
                    }}
                    className="px-2 py-0.5 rounded bg-[#161b22] hover:bg-[#21262d] text-[10px] text-[#58a6ff] border border-[#30363d] transition"
                  >
                    {cmd}
                  </button>
                ))}
              </div>
            </div>

            {/* Terminal Lines Stream */}
            <div className="flex-1 overflow-auto p-3 space-y-1">
              {terminalLines.map((line, i) => {
                const ownerColor =
                  line.owner === 'mediated'
                    ? 'bg-[#a371f7]/15 text-[#bc8cff] border-[#a371f7]/30'
                    : line.owner === 'manual'
                    ? 'bg-[#58a6ff]/15 text-[#58a6ff] border-[#58a6ff]/30'
                    : 'bg-[#21262d] text-[#8b949e] border-[#30363d]';
                return (
                  <div key={i} className="flex items-start gap-2 leading-relaxed">
                    <span className="text-[#484f58] select-none text-[9.5px] shrink-0">{line.time}</span>
                    <span className={`px-1.5 py-0.1 text-[9px] uppercase font-semibold rounded border shrink-0 ${ownerColor}`}>
                      {line.owner}
                    </span>
                    <span className="text-[#e6edf3] whitespace-pre-wrap">{line.text}</span>
                  </div>
                );
              })}
            </div>

            {/* Prompt input */}
            <div className="flex items-center gap-2 px-3 py-2 border-t border-[#21262d] bg-[#0d1117]">
              <span className="text-emerald-400 font-bold select-none">$</span>
              <input
                type="text"
                value={termInput}
                onChange={(e) => setTermInput(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && termInput.trim()) {
                    const now = new Date().toLocaleTimeString();
                    setTerminalLines((prev) => [
                      ...prev,
                      { owner: 'manual', text: termInput, time: now },
                      { owner: 'system', text: `[daemon-shell] returncode 0 for ${termInput}`, time: now }
                    ]);
                    setTermInput('');
                  }
                }}
                placeholder="type cargo, git, or custos command..."
                className="w-full bg-transparent border-none outline-none text-[#e6edf3] font-mono text-[11px]"
              />
            </div>
          </div>
        )}

        {/* VIEW D: FILES / EDITOR (Vinh's Multi-File Repo Browser + Inline Cmd+K) */}
        {activeTabId === 'files' && (
          <div className="h-full flex flex-col bg-[#0d1117] select-text">
            {/* File Switcher Tabs */}
            <div className="flex items-center justify-between px-3 py-1.5 border-b border-[#21262d] bg-[#161b22]/70 shrink-0 overflow-x-auto">
              <div className="flex items-center gap-1">
                {REPO_FILES.map((file) => {
                  const isSelected = selectedFile.path === file.path;
                  return (
                    <button
                      key={file.path}
                      onClick={() => setSelectedFile(file)}
                      className={`flex items-center gap-2 px-2.5 py-1 rounded-lg text-xs font-mono transition ${
                        isSelected
                          ? 'bg-[#21262d] text-white shadow-sm border border-[#30363d]'
                          : 'text-[#8b949e] hover:text-white hover:bg-[#21262d]/50'
                      }`}
                    >
                      <FileCode className={`w-3.5 h-3.5 ${isSelected ? 'text-[#58a6ff]' : 'text-[#8b949e]'}`} />
                      <span>{file.name}</span>
                      {file.status === 'M' && (
                        <span className="text-[10px] text-amber-400 font-bold">M</span>
                      )}
                    </button>
                  );
                })}
              </div>

              <div className="flex items-center gap-2 shrink-0">
                <button
                  onClick={() => setIsCodexPromptOpen(!isCodexPromptOpen)}
                  className={`flex items-center gap-1.5 px-2 py-1 rounded-lg text-xs font-medium border transition ${
                    isCodexPromptOpen
                      ? 'bg-[#58a6ff]/20 border-[#58a6ff] text-[#58a6ff]'
                      : 'bg-[#21262d] border-[#30363d] text-[#c9d1d9] hover:text-white'
                  }`}
                  title="Inline Codex Assist (Cmd+K)"
                >
                  <Sparkles className="w-3.5 h-3.5 text-[#58a6ff]" />
                  <span>Cmd+K Assist</span>
                </button>
                <button
                  onClick={() => {
                    navigator.clipboard.writeText(selectedFile.code);
                    setCopiedCode(true);
                    setTimeout(() => setCopiedCode(false), 2000);
                  }}
                  className="flex items-center gap-1 text-xs text-[#8b949e] hover:text-white px-2 py-1 rounded bg-[#21262d] transition"
                >
                  {copiedCode ? <Check className="w-3 h-3 text-[#3fb950]" /> : <Copy className="w-3 h-3" />}
                  <span>{copiedCode ? 'Copied' : 'Copy'}</span>
                </button>
              </div>
            </div>

            {/* Inline Cmd+K Prompt Drawer */}
            {isCodexPromptOpen && (
              <div className="p-3 bg-[#161b22] border-b border-[#30363d] space-y-2 animate-in slide-in-from-top-1 duration-150">
                <div className="flex items-center gap-2">
                  <Sparkles className="w-4 h-4 text-[#58a6ff]" />
                  <input
                    type="text"
                    value={codexInput}
                    onChange={(e) => setCodexInput(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' && codexInput.trim()) {
                        setCodexGenerating(true);
                        setTimeout(() => {
                          setCodexGenerating(false);
                          if (onShowToast) onShowToast('Codex applied invariant patch to ' + selectedFile.name);
                          setIsCodexPromptOpen(false);
                          setCodexInput('');
                        }, 800);
                      }
                    }}
                    placeholder={`Instruct Codex on ${selectedFile.name} (e.g. "Add strict zero-IO permit check")...`}
                    className="flex-1 bg-[#0d1117] border border-[#30363d] rounded-lg px-2.5 py-1 text-xs text-white placeholder-[#8b949e] outline-none focus:border-[#58a6ff]"
                  />
                  <button
                    onClick={() => {
                      if (!codexInput.trim()) return;
                      setCodexGenerating(true);
                      setTimeout(() => {
                        setCodexGenerating(false);
                        if (onShowToast) onShowToast('Codex applied invariant patch to ' + selectedFile.name);
                        setIsCodexPromptOpen(false);
                        setCodexInput('');
                      }, 800);
                    }}
                    disabled={codexGenerating}
                    className="px-3 py-1 rounded-lg bg-[#58a6ff] hover:bg-[#3182ce] text-white text-xs font-semibold transition"
                  >
                    {codexGenerating ? 'Synthesizing...' : 'Apply Patch'}
                  </button>
                </div>
              </div>
            )}

            {/* Code Content with Line Numbers */}
            <div className="flex-1 overflow-auto bg-[#010409] font-mono text-[11px] leading-relaxed p-3 text-[#e6edf3]">
              <div className="table w-full">
                {selectedFile.code.split('\n').map((line, idx) => (
                  <div key={idx} className="table-row hover:bg-[#161b22]/50">
                    <span className="table-cell pr-4 text-right text-[#484f58] select-none w-10">
                      {idx + 1}
                    </span>
                    <span className="table-cell whitespace-pre">{line}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        {/* VIEW E: WORKTREES (FULL ORCA LIFECYCLE) */}
        {activeTabId === 'worktrees' && (
          <div className="h-full flex flex-col p-6 space-y-4 select-text">
            <div className="flex items-center justify-between border-b border-[#21262d] pb-3">
              <div>
                <h3 className="text-sm font-semibold text-white flex items-center gap-2">
                  <GitBranch className="w-4 h-4 text-[#a371f7]" />
                  Orca Worktree Lifecycle Manager
                </h3>
                <p className="text-[11px] text-[#8b949e] mt-0.5">
                  5-Stage Lifecycle: Create → Work → Review → Ship → Cleanup
                </p>
              </div>
              <button
                onClick={() => setIsWorktreeModalOpen(true)}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#21262d] hover:bg-[#30363d] text-white font-medium text-[11px] transition"
              >
                <Sliders className="w-3.5 h-3.5 text-[#58a6ff]" />
                Full Modal View
              </button>
            </div>

            {/* Worktree Cards */}
            <div className="space-y-3">
              {[
                { id: 'wt-simd', branch: 'feat/simd-dispatch', status: 'Work', commits: '4 ahead', path: '.worktrees/feat-simd' },
                { id: 'wt-kernel', branch: 'fix/permit-gate-fencing', status: 'Review', commits: '2 ahead', path: '.worktrees/fix-permit' },
                { id: 'wt-auth', branch: 'feat/sovereign-auth', status: 'Ship', commits: 'Ready to merge', path: '.worktrees/feat-auth' }
              ].map((wt) => {
                const isActive = wt.id === activeWorktreeBranch;
                return (
                  <div
                    key={wt.id}
                    onClick={() => {
                      setActiveWorktreeBranch(wt.id);
                      if (onShowToast) onShowToast(`Switched active worktree to ${wt.branch}`);
                    }}
                    className={`p-3.5 rounded-xl border cursor-pointer transition ${
                      isActive
                        ? 'bg-[#161b22] border-[#58a6ff] shadow-md'
                        : 'bg-[#161b22]/50 border-[#21262d] hover:border-[#384252]'
                    }`}
                  >
                    <div className="flex items-center justify-between mb-2">
                      <div className="flex items-center gap-2 font-mono text-[12px] font-semibold text-white">
                        <GitBranch className="w-3.5 h-3.5 text-[#a371f7]" />
                        <span>{wt.branch}</span>
                        {isActive && (
                          <span className="px-1.5 py-0.2 rounded bg-emerald-500/20 text-emerald-400 text-[10px]">Active</span>
                        )}
                      </div>
                      <span className="px-2 py-0.5 rounded-full text-[10px] font-medium bg-[#21262d] text-[#58a6ff] border border-[#30363d]">
                        Stage: {wt.status}
                      </span>
                    </div>
                    <div className="flex items-center justify-between text-[11px] text-[#8b949e]">
                      <span className="font-mono text-[10px]">{wt.path}</span>
                      <span>{wt.commits}</span>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* VIEW F: FLEET KANBAN */}
        {activeTabId === 'kanban' && (
          <div className="h-full flex flex-col p-4 space-y-4 select-text">
            <div className="flex items-center justify-between border-b border-[#21262d] pb-2">
              <h3 className="font-semibold text-white text-[13px] flex items-center gap-2">
                <Bot className="w-4 h-4 text-[#bc8cff]" />
                Orca 4-Column Agent Fleet Board
              </h3>
              <span className="text-[11px] text-[#8b949e]">5 Agents active</span>
            </div>

            <div className="grid grid-cols-4 gap-3 flex-1 overflow-auto min-h-[300px]">
              {[
                { title: 'Needs You', filter: 'needs_you', color: 'border-amber-500/40 text-amber-400' },
                { title: 'Working', filter: 'working', color: 'border-emerald-500/40 text-emerald-400' },
                { title: 'Done', filter: 'done', color: 'border-blue-500/40 text-blue-400' },
                { title: 'Idle', filter: 'idle', color: 'border-neutral-500/40 text-neutral-400' }
              ].map((col) => (
                <div key={col.title} className="flex flex-col rounded-xl bg-[#161b22]/40 border border-[#21262d] p-2.5">
                  <div className={`text-[11px] font-semibold pb-2 border-b border-[#21262d] mb-2 flex items-center justify-between ${col.color}`}>
                    <span>{col.title}</span>
                    <span className="text-[10px] px-1.5 py-0.2 rounded bg-[#21262d] text-white">
                      {kanbanAgents.filter((a) => a.status === col.filter).length}
                    </span>
                  </div>
                  <div className="space-y-2 overflow-auto flex-1">
                    {kanbanAgents
                      .filter((a) => a.status === col.filter)
                      .map((agent) => (
                        <div key={agent.id} className="p-2.5 rounded-lg bg-[#161b22] border border-[#30363d] space-y-1">
                          <div className="font-medium text-white text-[12px]">{agent.name}</div>
                          <div className="text-[10.5px] text-[#8b949e] line-clamp-2">{agent.role}</div>
                          <div className="flex items-center justify-between text-[9.5px] font-mono text-[#58a6ff] pt-1 border-t border-[#21262d]">
                            <span>{agent.model}</span>
                            <span>{agent.tokens} tokens</span>
                          </div>
                          <button
                            onClick={() => {
                              setSelectedTask({
                                id: agent.id,
                                title: agent.role,
                                status: agent.status,
                                model: agent.model,
                                budgetUsed: 0.35,
                                budgetLimit: 1.0,
                                turnsUsed: 2,
                                maxTurns: 5,
                                actor: agent.name,
                                criteria: [
                                  { id: 'c1', label: 'Invariant Check Closure', status: 'pass' as const, evidenceHash: 'bafy2bzace4v3k99a' }
                                ]
                              });
                              setIsTaskDetailsModalOpen(true);
                            }}
                            className="w-full text-center py-1 rounded bg-[#21262d] hover:bg-[#30363d] text-[10px] text-neutral-300 font-medium transition"
                          >
                            Inspect Task Details
                          </button>
                        </div>
                      ))}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* VIEW G: EVIDENCE & PERMITS (With interactive permit approvals) */}
        {activeTabId === 'evidence' && (
          <div className="h-full flex flex-col p-6 space-y-6 select-text overflow-y-auto">
            {/* Header */}
            <div>
              <h3 className="font-semibold text-white text-sm flex items-center gap-2">
                <ShieldCheck className="w-4 h-4 text-[#3fb950]" />
                Sovereign Invariant Gates & Capability Permits
              </h3>
              <p className="text-xs text-[#8b949e] mt-1">
                Zero-IO capability tickets, CAS content-addressed ledger, and Merkle proof closures.
              </p>
            </div>

            {/* Active Execution Permits Section */}
            <div className="space-y-3">
              <h4 className="text-xs font-semibold text-[#c9d1d9] uppercase tracking-wider">
                Active Execution Permits
              </h4>
              <div className="space-y-2">
                {permits.map((p) => {
                  const isApproved = p.status === 'approved';
                  return (
                    <div key={p.id} className="p-3 rounded-xl bg-[#161b22] border border-[#21262d] flex items-center justify-between">
                      <div className="space-y-1">
                        <div className="flex items-center gap-2">
                          <span className="font-mono text-[10.5px] px-1.5 py-0.2 rounded bg-[#21262d] text-[#58a6ff]">
                            {p.action}
                          </span>
                          <span className="text-xs font-semibold text-white">{p.actor}</span>
                          <span className="text-[10px] text-[#8b949e]">({p.timestamp})</span>
                        </div>
                        <div className="font-mono text-[10.5px] text-[#8b949e] truncate max-w-md">
                          {p.target}
                        </div>
                      </div>

                      <div className="flex items-center gap-2 shrink-0">
                        {p.status === 'pending' ? (
                          <>
                            <button
                              onClick={() => {
                                setPermits((prev) => prev.map((item) => item.id === p.id ? { ...item, status: 'denied' as const } : item));
                                if (onShowToast) onShowToast(`Permit denied for ${p.actor}`);
                              }}
                              className="px-2.5 py-1 rounded-md bg-[#21262d] hover:bg-rose-500/20 text-rose-400 text-xs font-medium transition"
                            >
                              Deny
                            </button>
                            <button
                              onClick={() => {
                                setPermits((prev) => prev.map((item) => item.id === p.id ? { ...item, status: 'approved' as const } : item));
                                if (onShowToast) onShowToast(`Permit approved for ${p.actor}`);
                              }}
                              className="px-2.5 py-1 rounded-md bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-medium transition"
                            >
                              Approve
                            </button>
                          </>
                        ) : (
                          <span className={`px-2 py-0.5 rounded text-[10.5px] font-mono font-medium ${
                            isApproved ? 'bg-emerald-500/15 text-emerald-400' : 'bg-rose-500/15 text-rose-400'
                          }`}>
                            {p.status.toUpperCase()}
                          </span>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>

            {/* Invariant Gates */}
            <div className="space-y-3">
              <h4 className="text-xs font-semibold text-[#c9d1d9] uppercase tracking-wider">
                Invariant Ledger Proofs
              </h4>
              <div className="space-y-2">
              {[
                { inv: 'INV-01', name: 'Double-Dispatch Fencing', status: 'Passed', proof: 'cas://bafy2bzace4v3k99a' },
                { inv: 'INV-02', name: 'Deterministic Replay Proof', status: 'Passed', proof: 'cas://bafy2bzace4v78q1b' },
                { inv: 'INV-03', name: 'Zero-IO Sandbox Closure', status: 'Passed', proof: 'cas://bafy2bzace4v91z3c' }
              ].map((gate) => (
                <div key={gate.inv} className="p-3.5 rounded-xl bg-[#161b22] border border-[#21262d] flex items-center justify-between">
                  <div>
                    <div className="font-medium text-white text-[12.5px] flex items-center gap-2">
                      <span className="font-mono text-[11px] text-[#58a6ff]">{gate.inv}</span>
                      <span>{gate.name}</span>
                    </div>
                    <div className="font-mono text-[10px] text-[#8b949e] mt-1">{gate.proof}</div>
                  </div>
                  <span className="px-2 py-0.5 rounded bg-emerald-500/15 text-emerald-400 font-mono text-[10px] font-semibold">
                    {gate.status}
                  </span>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

        {/* VIEW H: DAG & TELEMETRY */}
        {activeTabId === 'dag' && (
          <div className="h-full flex flex-col p-6 space-y-4 select-text">
            <h3 className="font-semibold text-white text-sm flex items-center gap-2">
              <Activity className="w-4 h-4 text-[#58a6ff]" />
              Run → Task → Dispatch Attempt DAG Epochs
            </h3>
            <div className="p-4 rounded-xl bg-[#161b22] border border-[#21262d] space-y-3 font-mono text-[11px]">
              <div className="flex items-center gap-2 text-[#58a6ff]">
                <span>[Epoch 4]</span>
                <span className="text-white font-semibold">Run ID: rn_9921_sovereign</span>
              </div>
              <div className="pl-4 border-l-2 border-[#30363d] space-y-2 text-[#8b949e]">
                <div>├─ Task 1: Analyze Invariants (Depth: 0, Status: Completed)</div>
                <div>├─ Task 2: Synthesize Rust Gate (Depth: 1, Status: Running)</div>
                <div>└─ Task 3: Seal Non-Repudiation Proof (Depth: 2, Status: Pending)</div>
              </div>
            </div>
          </div>
        )}

        {/* VIEW I: BROWSER PREVIEW (Vinh's Integrated Browser Workspace) */}
        {activeTabId === 'browser' && (
          <div className="h-full flex flex-col overflow-hidden">
            <BrowserWorkspace />
          </div>
        )}

        {/* VIEW J: ARCHITECTURE NOTES (Vinh's Integrated Markdown Workspace) */}
        {activeTabId === 'notes' && (
          <div className="h-full flex flex-col overflow-hidden">
            <MarkdownWorkspace />
          </div>
        )}

        {/* VIEW K: RESEARCH - LITERATURE & CORPUS */}
        {activeTabId === 'literature' && (
          <div className="h-full flex flex-col overflow-hidden">
            <LiteraturePane
              onExtractClaim={(anchor) => {
                if (onShowToast) onShowToast(`Extracted claim from ${anchor.sourceTitle || 'source'}`);
              }}
              onShowToast={onShowToast}
            />
          </div>
        )}

        {/* VIEW L: RESEARCH - CLAIMS & INVARIANT MATRIX */}
        {activeTabId === 'claims' && (
          <div className="h-full flex flex-col overflow-hidden">
            <ClaimsMatrixPane
              onHandoffToCoding={(selectedClaims) => {
                if (onHandoffToCoding) onHandoffToCoding(selectedClaims);
                else if (onShowToast) onShowToast(`Handed off ${selectedClaims.length} claim(s) to Coding Workbench`);
              }}
              onShowToast={onShowToast}
            />
          </div>
        )}

        {/* VIEW M: RESEARCH - COMPUTATIONAL NOTEBOOK */}
        {activeTabId === 'experiments' && (
          <div className="h-full flex flex-col overflow-hidden">
            <NotebookWorkspacePane
              onAskAgent={(draft) => {
                if (onAskAgent) onAskAgent(draft);
                else if (onShowToast) onShowToast('Drafted cell into Research Chat');
              }}
              onShowToast={onShowToast}
            />
          </div>
        )}

        {/* VIEW N: RESEARCH - EXPERIMENT RUNS LEDGER & ARTIFACTS */}
        {activeTabId === 'synthesis' && (
          <div className="h-full flex flex-col overflow-hidden">
            <RunsLedgerPane
              onReproduce={(run) => {
                if (onAskAgent) onAskAgent(`Reproduce run ${run.runId}:\n\`${run.command}\``);
                else if (onShowToast) onShowToast(`Drafted reproduction prompt for ${run.runId}`);
              }}
              onShowToast={onShowToast}
            />
          </div>
        )}

        {activeTabId === 'artifacts' && (
          <div className="h-full flex flex-col overflow-hidden">
            <DeepInspectorPane onShowToast={onShowToast} />
          </div>
        )}
      </div>

      {/* Modal for full worktree manager */}
      <WorktreeManagerModal
        isOpen={isWorktreeModalOpen}
        onClose={() => setIsWorktreeModalOpen(false)}
        activeWorktreeId={activeWorktreeBranch}
        onSelectWorktree={(wt) => {
          setActiveWorktreeBranch(wt.id);
          if (onShowToast) onShowToast(`Active worktree: ${wt.branch}`);
        }}
      />

      {/* Task Details Modal (Vinh's SADE Invariant Inspector) */}
      <TaskDetailsModal
        isOpen={isTaskDetailsModalOpen}
        onClose={() => setIsTaskDetailsModalOpen(false)}
        task={selectedTask}
      />
    </div>
  );
};
