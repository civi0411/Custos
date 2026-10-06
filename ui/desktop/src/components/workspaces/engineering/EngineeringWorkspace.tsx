import React, { useState, useRef, useEffect } from 'react';
import { 
  Folder, 
  FileCode, 
  Copy, 
  Check, 
  CheckCircle2, 
  ChevronRight,
  Maximize2,
  Minimize2,
  Sparkles,
  Zap,
  ArrowRight,
  X,
  GitBranch,
  ShieldCheck,
  FileDiff,
  Clock,
  CheckCheck,
  FolderPlus,
  Terminal
} from 'lucide-react';
import { Session } from '@/types';
import { WorktreeManagerModal, type ManagedWorktree } from '@/components/modals';

interface EngineeringWorkspaceProps {
  session?: Session | null;
  onAcceptAndRun?: () => void;
  onRejectDiff?: () => void;
  onCopyDiff?: () => void;
}

interface RepoFile {
  path: string;
  name: string;
  language: 'rust' | 'typescript' | 'toml' | 'markdown';
  status: 'M' | 'A' | 'clean';
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

    pub async fn release_ticket(&self, task_id: &str) -> Result<(), String> {
        let mut map = self.tickets.lock().await;
        map.remove(task_id);
        Ok(())
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

export const EngineeringWorkspace: React.FC<EngineeringWorkspaceProps> = ({
  session: _session,
  onAcceptAndRun,
  onRejectDiff: _onRejectDiff,
  onCopyDiff: _onCopyDiff
}) => {
  const [selectedFile, setSelectedFile] = useState<RepoFile>(REPO_FILES[0]);
  const [viewSubTab, setViewSubTab] = useState<'editor' | 'diff' | 'invariants'>('editor');
  const [terminalTab, setTerminalTab] = useState<'shell' | 'build' | 'tests' | 'invariants'>('shell');
  const [terminalInput, setTerminalInput] = useState('');
  const [isTerminalExpanded, setIsTerminalExpanded] = useState(false);
  const [isFileTreeOpen, setIsFileTreeOpen] = useState(true);

  // Orca O2: Managed Worktree State
  const [isWorktreeModalOpen, setIsWorktreeModalOpen] = useState(false);
  const [activeWorktree, setActiveWorktree] = useState<ManagedWorktree>({
    id: 'wt-simd',
    branch: 'feat/simd-dispatch',
    path: '.worktrees/feat-simd-dispatch',
    baseCommit: 'a3f2d1e',
    host: 'local',
    status: 'work',
    modifiedFilesCount: 3,
    assignedAgent: 'Claude Code (S2-Worker)',
    createdAt: '45m ago'
  });

  // Terminal Lines with Owner Label
  const [terminalLines, setTerminalLines] = useState<{ owner: 'mediated' | 'manual' | 'system'; text: string; time: string }[]>([
    { owner: 'system', text: 'Custos SADE Scoped Terminal — Mediated Execution Runtime v0.1.0', time: '02:00:12' },
    { owner: 'mediated', text: 'cargo check --workspace --tests', time: '02:01:04' },
    { owner: 'system', text: '   Compiling custos-domain v0.1.0 (/Users/mac/Project/AgentHub/Custos/crates/custos-domain)', time: '02:01:05' },
    { owner: 'system', text: '   Compiling custos-core v0.1.0 (/Users/mac/Project/AgentHub/Custos/crates/custos-core)', time: '02:01:06' },
    { owner: 'system', text: '   Compiling custos-runtime v0.1.0 (/Users/mac/Project/AgentHub/Custos/crates/custos-runtime)', time: '02:01:07' },
    { owner: 'system', text: '    Finished dev [unoptimized + debuginfo] target(s) in 2.14s (0 errors, 0 warnings)', time: '02:01:08' },
    { owner: 'mediated', text: 'custos check --invariants INV-01,INV-03', time: '02:01:45' },
    { owner: 'system', text: '   ✓ [INV-01 Double-Dispatch Fencing]: Proof closure valid (CAS: bafy2bzace4v3k99a)', time: '02:01:46' },
    { owner: 'system', text: '   ✓ [INV-03 Zero-IO Ticket Gate]: Strict lease active for task: vi', time: '02:01:46' },
    { owner: 'manual', text: 'git status -s', time: '02:02:10' },
    { owner: 'system', text: ' M crates/custos-runtime/src/workflow/dispatcher.rs', time: '02:02:10' },
  ]);

  const [copied, setCopied] = useState(false);

  // Claude Code Inline Assist State (Cmd+K)
  const [isCodexPromptOpen, setIsCodexPromptOpen] = useState(false);
  const [codexInput, setCodexInput] = useState('');
  const [isCodexGenerating, setIsCodexGenerating] = useState(false);
  const [codexSuggestion, setCodexSuggestion] = useState<string | null>(null);
  const codexInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isCodexPromptOpen) {
      setTimeout(() => codexInputRef.current?.focus(), 40);
    }
  }, [isCodexPromptOpen]);

  const handleCopyCode = () => {
    navigator.clipboard.writeText(selectedFile.code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleTerminalSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!terminalInput.trim()) return;
    const cmd = terminalInput.trim();
    const now = new Date().toTimeString().slice(0, 8);

    setTerminalLines(prev => [
      ...prev,
      { owner: 'manual', text: cmd, time: now },
      ...(cmd.startsWith('cargo test') ? [
        { owner: 'system' as const, text: 'running 14 tests in crates/custos-runtime...', time: now },
        { owner: 'system' as const, text: 'test workflow::dispatcher::test_claim_permit ... ok', time: now },
        { owner: 'system' as const, text: 'test authority::permits::test_double_fencing ... ok', time: now },
        { owner: 'system' as const, text: 'test result: ok. 14 passed; 0 failed; 0 ignored', time: now }
      ] : cmd.startsWith('git status') ? [
        { owner: 'system' as const, text: 'On branch main (tracking origin/main)\nChanges not staged for commit:\n  modified:   crates/custos-runtime/src/workflow/dispatcher.rs', time: now }
      ] : cmd.startsWith('custos') ? [
        { owner: 'system' as const, text: 'custos daemon: invariant status verified. 0 violations.', time: now }
      ] : cmd === 'clear' ? [] : [
        { owner: 'system' as const, text: `custos-sh: command '${cmd.split(' ')[0]}' executed with exit code 0`, time: now }
      ])
    ]);
    setTerminalInput('');
  };

  const handleTriggerCodex = (promptText?: string) => {
    const query = promptText || codexInput;
    if (!query.trim()) return;
    setIsCodexGenerating(true);
    setCodexSuggestion(null);

    setTimeout(() => {
      setIsCodexGenerating(false);
      setCodexSuggestion(
`// [Claude Code Synthesized Patch]
// Enforces atomic timeout on permit acquisition (INV-03)
pub async fn claim_with_timeout(&self, task_id: &str, timeout_ms: u64) -> Result<Action, String> {
    let lease = tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        self.permit_gate.acquire_ticket(task_id)
    ).await.map_err(|_| format!("Permit acquisition timed out after {}ms", timeout_ms))??;

    println!("Claimed task {} under invariant gate {:?}", task_id, lease.id());
    Ok(Action::Execute(task_id.to_string()))
}`
      );
    }, 600);
  };

  const applyCodexSuggestion = () => {
    setCodexSuggestion(null);
    setIsCodexPromptOpen(false);
    setCodexInput('');
    setTerminalLines(prev => [
      ...prev,
      { owner: 'mediated', text: 'custos patch apply --source=claude-code-synth --verify-invariants', time: new Date().toTimeString().slice(0, 8) },
      { owner: 'system', text: '   ✓ Applied patch to crates/custos-runtime/src/workflow/dispatcher.rs', time: new Date().toTimeString().slice(0, 8) },
      { owner: 'system', text: '   ✓ Re-verified invariants: 0 violations, proof closure preserved.', time: new Date().toTimeString().slice(0, 8) }
    ]);
  };

  // Syntax highlighter for Claude Code dark palette
  const renderSyntaxLine = (line: string) => {
    if (line.trim().startsWith('//')) {
      return <span style={{ color: 'var(--syntax-comment, #8b949e)', fontStyle: 'italic' }}>{line}</span>;
    }
    
    // Split tokens by keywords
    const keywords = ['pub struct', 'pub fn', 'impl', 'use ', 'pub async fn', 'async fn', 'fn ', 'let mut', 'let ', 'return ', 'match ', 'Ok', 'Err', 'Self'];
    const matchedKw = keywords.find(kw => line.includes(kw));

    if (matchedKw) {
      const parts = line.split(matchedKw);
      return (
        <span>
          <span style={{ color: 'var(--color-editor-fg, #e6edf3)' }}>{parts[0]}</span>
          <span style={{ color: 'var(--syntax-keyword, #ff7b72)', fontWeight: 600 }}>{matchedKw}</span>
          <span style={{ color: 'var(--color-editor-fg, #e6edf3)' }}>
            {parts.slice(1).join(matchedKw)}
          </span>
        </span>
      );
    }

    if (line.includes('Result<') || line.includes('Option<') || line.includes('Arc<') || line.includes('String') || line.includes('Self')) {
      return (
        <span style={{ color: 'var(--color-editor-fg, #e6edf3)' }}>
          {line.replace(/(Result|Option|Arc|String|PermitGate|Action|WorkflowDispatcher)/g, '%%$1%%')
            .split('%%').map((segment, i) => 
              ['Result', 'Option', 'Arc', 'String', 'PermitGate', 'Action', 'WorkflowDispatcher'].includes(segment)
                ? <span key={i} style={{ color: 'var(--syntax-type, #ffa657)', fontWeight: 500 }}>{segment}</span>
                : <span key={i}>{segment}</span>
            )}
        </span>
      );
    }

    return <span style={{ color: 'var(--color-editor-fg, #e6edf3)' }}>{line}</span>;
  };

  return (
    <div 
      className="flex-1 flex flex-col h-full overflow-hidden select-none"
      style={{
        background: 'var(--color-canvas, #0d1117)',
        color: 'var(--color-editor-fg, #e6edf3)',
        borderLeft: '1px solid var(--color-border-default, #30363d)'
      }}
    >
      {/* ── 1. Top Header Bar (Claude Code style) ── */}
      <div 
        className="h-9 px-3 flex items-center justify-between text-xs shrink-0"
        style={{
          background: 'var(--color-surface-1, #161b22)',
          borderBottom: '1px solid var(--color-border-default, #30363d)',
        }}
      >
        {/* Left: Scope Breadcrumbs */}
        <div className="flex items-center gap-1.5 font-mono text-[11px] truncate">
          <button
            onClick={() => setIsFileTreeOpen(!isFileTreeOpen)}
            className="p-1 rounded hover:bg-surface-2 transition flex items-center gap-1"
            title="Toggle File Scope Explorer"
            style={{ color: isFileTreeOpen ? 'var(--color-coding, #3fb950)' : 'var(--color-fg-muted, #8b949e)' }}
          >
            <Folder className="w-3.5 h-3.5" />
          </button>
          <span style={{ color: 'var(--color-fg-muted, #8b949e)' }}>Custos</span>
          <ChevronRight className="w-3 h-3 text-[#6e7681]" />
          <span style={{ color: 'var(--color-fg-muted, #8b949e)' }}>crates</span>
          <ChevronRight className="w-3 h-3 text-[#6e7681]" />
          <FileCode className="w-3.5 h-3.5" style={{ color: 'var(--color-coding, #3fb950)' }} />
          <span className="font-semibold" style={{ color: 'var(--color-editor-fg, #e6edf3)' }}>
            {selectedFile.name}
          </span>
          {selectedFile.status === 'M' && (
            <span 
              className="text-[10px] px-1 py-0.2 rounded font-mono font-bold"
              style={{ background: 'rgba(210,153,34,0.15)', color: '#d29922', border: '1px solid rgba(210,153,34,0.3)' }}
              title="Modified in current worktree"
            >
              M
            </span>
          )}
          <span className="text-[10px] ml-2 hidden md:inline font-mono text-[#6e7681]">
            <GitBranch className="w-3 h-3 inline mr-1 text-[#6e7681]" />
            main ● 2 ahead
          </span>
        </div>

        {/* Center: View Switcher (Editor / Diff Review / Invariants) */}
        <div className="flex items-center gap-2">
          {/* Claude Code Inline Assist (Cmd+K) */}
          <button
            onClick={() => setIsCodexPromptOpen(!isCodexPromptOpen)}
            className="px-2 py-1 rounded text-[11px] font-medium flex items-center gap-1.5 transition"
            style={{
              background: isCodexPromptOpen ? 'rgba(63,185,80,0.15)' : 'var(--color-surface-2, #1c2128)',
              border: `1px solid ${isCodexPromptOpen ? 'var(--color-coding, #3fb950)' : 'var(--color-border-default, #30363d)'}`,
              color: isCodexPromptOpen ? 'var(--color-coding, #3fb950)' : 'var(--color-editor-fg, #e6edf3)',
            }}
            title="Open Claude Code Assist (⌘K)"
          >
            <Sparkles className="w-3 h-3" style={{ color: 'var(--color-coding, #3fb950)' }} />
            <span>Claude Code</span>
            <kbd className="text-[9px] px-1 rounded font-mono text-[#8b949e]" style={{ background: 'var(--color-surface-0, #0d1117)' }}>⌘K</kbd>
          </button>

          {/* Sub-tabs */}
          <div 
            className="flex items-center p-0.5 rounded text-[11px]"
            style={{
              background: 'var(--color-surface-0, #0d1117)',
              border: '1px solid var(--color-border-muted, #21262d)',
            }}
          >
            <button
              onClick={() => setViewSubTab('editor')}
              className="px-2 py-0.5 rounded transition font-medium"
              style={{
                background: viewSubTab === 'editor' ? 'var(--color-surface-2, #1c2128)' : 'transparent',
                color: viewSubTab === 'editor' ? 'var(--color-editor-fg, #e6edf3)' : 'var(--color-fg-muted, #8b949e)',
                border: viewSubTab === 'editor' ? '1px solid var(--color-border-default, #30363d)' : '1px solid transparent'
              }}
            >
              Source Code
            </button>
            <button
              onClick={() => setViewSubTab('diff')}
              className="px-2 py-0.5 rounded transition font-medium flex items-center gap-1"
              style={{
                background: viewSubTab === 'diff' ? 'var(--color-surface-2, #1c2128)' : 'transparent',
                color: viewSubTab === 'diff' ? 'var(--color-editor-fg, #e6edf3)' : 'var(--color-fg-muted, #8b949e)',
                border: viewSubTab === 'diff' ? '1px solid var(--color-border-default, #30363d)' : '1px solid transparent'
              }}
            >
              <FileDiff className="w-3 h-3" />
              <span>Diff</span>
              <span className="text-[10px] text-[#3fb950] font-mono">+38 -4</span>
            </button>
            <button
              onClick={() => setViewSubTab('invariants')}
              className="px-2 py-0.5 rounded transition font-medium flex items-center gap-1"
              style={{
                background: viewSubTab === 'invariants' ? 'var(--color-surface-2, #1c2128)' : 'transparent',
                color: viewSubTab === 'invariants' ? 'var(--color-editor-fg, #e6edf3)' : 'var(--color-fg-muted, #8b949e)',
                border: viewSubTab === 'invariants' ? '1px solid var(--color-border-default, #30363d)' : '1px solid transparent'
              }}
            >
              <ShieldCheck className="w-3 h-3 text-[#3fb950]" />
              <span>Invariants (3)</span>
            </button>
          </div>

          {/* Action: Copy Code */}
          <button
            onClick={handleCopyCode}
            className="p-1 rounded text-[#8b949e] hover:text-[#e6edf3] transition"
            title="Copy code"
            style={{ border: '1px solid var(--color-border-muted, #21262d)' }}
          >
            {copied ? <Check className="w-3.5 h-3.5 text-[#3fb950]" /> : <Copy className="w-3.5 h-3.5" />}
          </button>
        </div>
      </div>

      {/* ── Orca ADE Worktree Lifecycle & Launch Fidelity Bar (O2, O3, O5) ── */}
      <div
        className="h-8 px-3 flex items-center justify-between text-[11px] font-mono shrink-0 select-none"
        style={{
          background: 'var(--color-surface-2, #1c2128)',
          borderBottom: '1px solid var(--color-border-default, #30363d)'
        }}
      >
        {/* Left: Worktree Branch & Lifecycle Status */}
        <div className="flex items-center gap-2">
          <button
            onClick={() => setIsWorktreeModalOpen(true)}
            className="flex items-center gap-1.5 px-2 py-0.5 rounded text-white hover:bg-surface-1 transition font-bold"
            style={{ background: 'rgba(63,185,80,0.15)', border: '1px solid rgba(63,185,80,0.3)', color: '#3fb950' }}
            title="Open Orca Worktree Manager (Create, Work, Review, Ship, Cleanup)"
          >
            <GitBranch className="w-3.5 h-3.5" />
            <span>{activeWorktree.branch}</span>
            <span className="text-[#8b949e] font-normal">#{activeWorktree.baseCommit}</span>
          </button>

          <div className="hidden lg:flex items-center gap-1 text-[10.5px]">
            <span className="text-[#8b949e]">Lifecycle:</span>
            <span className="text-blue-400">1. Create</span>
            <span className="text-[#6e7681]">→</span>
            <span className="text-[#3fb950] font-bold flex items-center gap-0.5">
              <span className="w-1.5 h-1.5 rounded-full bg-[#3fb950] animate-pulse"></span>
              2. Work (Active)
            </span>
            <span className="text-[#6e7681]">→</span>
            <span className="text-amber-400">3. Review Diff</span>
            <span className="text-[#6e7681]">→</span>
            <span className="text-purple-400">4. Ship</span>
            <span className="text-[#6e7681]">→</span>
            <span className="text-neutral-500">5. Clean</span>
          </div>
        </div>

        {/* Right: Agent Launch Fidelity (Orca O3) */}
        <div className="flex items-center gap-2 text-[10.5px]">
          <div 
            className="hidden sm:flex items-center gap-1.5 px-2 py-0.5 rounded border border-[#30363d]"
            style={{ background: 'var(--color-surface-0, #0d1117)' }}
          >
            <Terminal className="w-3 h-3 text-[#3fb950]" />
            <span className="text-[#8b949e]">Surface:</span>
            <span className="text-white font-medium">PTY-Mediated</span>
            <span className="text-[#8b949e]">·</span>
            <span className="text-[#3fb950]">[Custos AST]</span>
            <span className="text-[#8b949e]">·</span>
            <span className="text-[#58a6ff]">Prompt: Delivered</span>
          </div>

          <button
            onClick={() => setIsWorktreeModalOpen(true)}
            className="px-2 py-0.5 rounded text-[10px] text-[#e6edf3] hover:text-white bg-surface-1 border border-surface-border transition flex items-center gap-1"
          >
            <FolderPlus className="w-3 h-3 text-[#3fb950]" />
            <span>Worktrees</span>
          </button>
        </div>
      </div>

      {/* ── 2. Middle Stage (File Scope Tree + Editor / Diff) ── */}
      <div className="flex-1 flex min-h-0 overflow-hidden relative">
        {/* Collapsible Left Scope File Tree */}
        {isFileTreeOpen && (
          <div 
            className="w-56 flex flex-col shrink-0 overflow-y-auto select-none font-mono text-[11px]"
            style={{
              background: 'var(--color-surface-1, #161b22)',
              borderRight: '1px solid var(--color-border-default, #30363d)',
            }}
          >
            <div 
              className="px-3 py-2 text-[10px] font-semibold uppercase tracking-wider text-[#8b949e] flex items-center justify-between"
              style={{ borderBottom: '1px solid var(--color-border-muted, #21262d)' }}
            >
              <span>Workspace Files</span>
              <span className="text-[#3fb950]">{REPO_FILES.length} refs</span>
            </div>

            <div className="p-1 space-y-0.5">
              {REPO_FILES.map((f) => {
                const active = selectedFile.path === f.path;
                return (
                  <button
                    key={f.path}
                    onClick={() => setSelectedFile(f)}
                    className="w-full text-left px-2 py-1.5 rounded flex items-center justify-between transition group"
                    style={{
                      background: active ? 'var(--color-surface-2, #1c2128)' : 'transparent',
                      color: active ? 'var(--color-editor-fg, #e6edf3)' : 'var(--color-fg-muted, #8b949e)',
                      border: `1px solid ${active ? 'var(--color-border-default, #30363d)' : 'transparent'}`,
                    }}
                  >
                    <div className="flex items-center gap-1.5 truncate">
                      <FileCode 
                        className="w-3.5 h-3.5 shrink-0" 
                        style={{ color: active ? 'var(--color-coding, #3fb950)' : '#8b949e' }} 
                      />
                      <span className="truncate">{f.name}</span>
                    </div>
                    {f.status === 'M' && (
                      <span className="text-[9px] font-bold text-[#d29922] shrink-0 ml-1">M</span>
                    )}
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {/* Main Stage Content */}
        <div className="flex-1 flex flex-col min-w-0 overflow-hidden relative select-text">
          {/* Floating Claude Code Prompt Bar (Cmd+K) */}
          {isCodexPromptOpen && (
            <div 
              className="absolute top-3 left-4 right-4 z-30 rounded-xl shadow-2xl p-3 text-xs animate-in fade-in slide-in-from-top-2 duration-150"
              style={{
                background: 'rgba(22, 27, 34, 0.96)',
                backdropFilter: 'blur(8px)',
                border: '1px solid var(--color-coding, #3fb950)',
              }}
            >
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2">
                  <Sparkles className="w-3.5 h-3.5 text-[#3fb950]" />
                  <span className="font-semibold text-white text-[12px]">Claude Code Inline Assist</span>
                  <span className="text-[10px] px-1.5 py-0.5 rounded font-mono text-[#3fb950]" style={{ background: 'rgba(63,185,80,0.1)' }}>
                    Context: {selectedFile.path}
                  </span>
                </div>
                <button 
                  onClick={() => setIsCodexPromptOpen(false)}
                  className="text-neutral-400 hover:text-white p-0.5 rounded transition"
                >
                  <X className="w-3.5 h-3.5" />
                </button>
              </div>

              {/* Form Input */}
              <form onSubmit={(e) => { e.preventDefault(); handleTriggerCodex(); }} className="flex items-center gap-2 mb-2">
                <input
                  ref={codexInputRef}
                  type="text"
                  value={codexInput}
                  onChange={(e) => setCodexInput(e.target.value)}
                  placeholder="Instruct Claude Code (e.g. 'Add timeout handling to claim_ready_task', 'Check invariant INV-01')..."
                  className="flex-1 rounded-md px-3 py-1.5 text-white placeholder-[#6e7681] focus:outline-none font-mono text-xs"
                  style={{
                    background: 'var(--color-canvas, #0d1117)',
                    border: '1px solid var(--color-border-default, #30363d)',
                  }}
                />
                <button
                  type="submit"
                  disabled={isCodexGenerating || !codexInput.trim()}
                  className="px-3 py-1.5 rounded-md text-white font-medium text-xs flex items-center gap-1.5 transition disabled:opacity-50"
                  style={{ background: 'var(--color-coding, #3fb950)' }}
                >
                  {isCodexGenerating ? (
                    <>
                      <Zap className="w-3 h-3 animate-spin" />
                      <span>Synthesizing...</span>
                    </>
                  ) : (
                    <>
                      <span>Apply (↵)</span>
                      <ArrowRight className="w-3 h-3" />
                    </>
                  )}
                </button>
              </form>

              {/* Quick Prompt Chips */}
              <div className="flex items-center gap-1.5 flex-wrap pt-1 text-[11px]" style={{ borderTop: '1px solid var(--color-border-muted, #21262d)' }}>
                <span className="text-[#8b949e] text-[10px]">Shortcuts:</span>
                <button
                  type="button"
                  onClick={() => {
                    setCodexInput('Add invariant lease timeout to claim_ready_task');
                    handleTriggerCodex('Add invariant lease timeout to claim_ready_task');
                  }}
                  className="px-2 py-0.5 rounded text-[10px] text-neutral-300 hover:text-white transition"
                  style={{ background: 'var(--color-surface-2, #1c2128)', border: '1px solid var(--color-border-default, #30363d)' }}
                >
                  🛡️ Lease Timeout (INV-01)
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setCodexInput('Generate cargo test verifying permit gate fencing');
                    handleTriggerCodex('Generate cargo test verifying permit gate fencing');
                  }}
                  className="px-2 py-0.5 rounded text-[10px] text-neutral-300 hover:text-white transition"
                  style={{ background: 'var(--color-surface-2, #1c2128)', border: '1px solid var(--color-border-default, #30363d)' }}
                >
                  ⚡ Unit Tests
                </button>
              </div>

              {/* Proposed Patch Preview */}
              {codexSuggestion && (
                <div 
                  className="mt-3 p-2.5 rounded-lg font-mono text-[11px] space-y-2"
                  style={{
                    background: 'var(--color-canvas, #0d1117)',
                    border: '1px solid rgba(63,185,80,0.3)',
                  }}
                >
                  <div className="flex items-center justify-between pb-1" style={{ borderBottom: '1px solid var(--color-border-muted, #21262d)' }}>
                    <span className="text-[#3fb950] font-semibold flex items-center gap-1">
                      <CheckCircle2 className="w-3.5 h-3.5 text-[#3fb950]" />
                      Synthesized Invariant Patch
                    </span>
                    <div className="flex items-center gap-1.5">
                      <button
                        onClick={() => setCodexSuggestion(null)}
                        className="px-2 py-0.5 rounded text-[#8b949e] hover:text-white text-[10px] transition"
                      >
                        Discard
                      </button>
                      <button
                        onClick={applyCodexSuggestion}
                        className="px-2.5 py-0.5 rounded text-white font-medium text-[10px] flex items-center gap-1 transition"
                        style={{ background: 'var(--color-coding, #3fb950)' }}
                      >
                        <Check className="w-3 h-3" />
                        <span>Accept & Apply Patch</span>
                      </button>
                    </div>
                  </div>
                  <pre className="text-[#7ee787] overflow-x-auto p-1 leading-relaxed">
                    {codexSuggestion}
                  </pre>
                </div>
              )}
            </div>
          )}

          {/* VIEW: SOURCE CODE EDITOR */}
          {viewSubTab === 'editor' && (
            <div 
              className="flex-1 flex flex-col overflow-auto font-mono text-[12px] p-4 leading-relaxed"
              style={{ background: 'var(--color-canvas, #0d1117)' }}
            >
              <div className="space-y-0.5">
                {selectedFile.code.split('\n').map((line, idx) => (
                  <div key={idx} className="flex hover:bg-white/[0.02] rounded px-1 group">
                    <span 
                      className="w-10 text-right pr-4 select-none text-[11px] transition text-[#6e7681] group-hover:text-[#8b949e]"
                    >
                      {idx + 1}
                    </span>
                    <span className="flex-1">
                      {renderSyntaxLine(line)}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* VIEW: DIFF REVIEW */}
          {viewSubTab === 'diff' && (
            <div 
              className="flex-1 flex flex-col overflow-auto font-mono text-[12px] p-4 leading-relaxed"
              style={{ background: 'var(--color-canvas, #0d1117)' }}
            >
              {/* Diff Header */}
              <div 
                className="pb-2 text-[11px] font-mono mb-3 flex items-center justify-between"
                style={{ borderBottom: '1px solid var(--color-border-default, #30363d)' }}
              >
                <div className="flex items-center gap-2">
                  <span className="text-[#8b949e]">base commit:</span>
                  <span className="text-white bg-surface-2 px-1.5 py-0.2 rounded border border-surface-border">#a3f2d1e</span>
                  <span className="text-[#8b949e]">→ working tree</span>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-[#3fb950] font-semibold">+38 lines</span>
                  <span className="text-[#f85149] font-semibold">-4 lines</span>
                  {onAcceptAndRun && (
                    <button
                      onClick={onAcceptAndRun}
                      className="ml-2 px-2.5 py-1 rounded text-white font-medium text-[11px] flex items-center gap-1.5 transition"
                      style={{ background: 'var(--color-coding, #3fb950)' }}
                    >
                      <CheckCheck className="w-3.5 h-3.5" />
                      <span>Approve & Verify (INV-01)</span>
                    </button>
                  )}
                </div>
              </div>

              {/* Diff Hunk */}
              <div className="space-y-0.5">
                {[
                  { type: 'context', text: '     pub async fn claim_ready_task(&self, task_id: &str) -> Result<Action, String> {' },
                  { type: 'del', text: '-        let ticket = self.permit_gate.acquire(task_id).await?;' },
                  { type: 'add', text: '+        // [INV-01]: Atomic ticket acquisition prevents phantom double-execution' },
                  { type: 'add', text: '+        let ticket = self.permit_gate.acquire_ticket(task_id).await' },
                  { type: 'add', text: '+            .map_err(|e| format!("Permit acquisition failed: {:?}", e))?;' },
                  { type: 'add', text: '+        let mut guard = self.active_permits.write().await;' },
                  { type: 'add', text: '+        guard.push(ticket.id().to_string());' },
                  { type: 'context', text: '         println!("Claimed task {} under invariant gate {:?}", task_id, ticket.id());' },
                  { type: 'context', text: '         Ok(Action::Execute(task_id.to_string()))' },
                  { type: 'context', text: '     }' }
                ].map((line, idx) => (
                  <div 
                    key={idx} 
                    className="flex px-2 py-0.5 rounded font-mono"
                    style={{
                      background: line.type === 'add' ? 'rgba(86, 211, 100, 0.12)' : line.type === 'del' ? 'rgba(255, 161, 152, 0.12)' : 'transparent',
                      color: line.type === 'add' ? 'var(--syntax-inserted, #56d364)' : line.type === 'del' ? 'var(--syntax-deleted, #ffa198)' : 'var(--color-editor-fg, #e6edf3)',
                      borderLeft: line.type === 'add' ? '2px solid #56d364' : line.type === 'del' ? '2px solid #ffa198' : '2px solid transparent',
                      textDecoration: line.type === 'del' ? 'line-through' : 'none'
                    }}
                  >
                    <span className="w-6 select-none opacity-50 text-[11px]">
                      {line.type === 'add' ? '+' : line.type === 'del' ? '-' : ' '}
                    </span>
                    <span className="flex-1 overflow-x-auto">{line.text}</span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* VIEW: INVARIANTS & SAFETY CONTRACTS */}
          {viewSubTab === 'invariants' && (
            <div 
              className="flex-1 p-5 overflow-y-auto text-xs space-y-4"
              style={{ background: 'var(--color-canvas, #0d1117)' }}
            >
              <div 
                className="p-4 rounded-xl space-y-2"
                style={{
                  background: 'var(--color-surface-1, #161b22)',
                  border: '1px solid var(--color-border-default, #30363d)',
                }}
              >
                <div className="flex items-center gap-2 text-[#3fb950] font-semibold text-sm">
                  <ShieldCheck className="w-4 h-4 text-[#3fb950]" />
                  <span>SADE Invariant Gate: Zero-IO Proof Closure</span>
                </div>
                <p className="text-[#8b949e] text-[12px] leading-relaxed">
                  All memory mutations and actions dispatched through <code className="text-white font-mono bg-surface-2 px-1 rounded">{selectedFile.path}</code> are strictly fenced under signed capability leases. Non-repudiation ledger hash is verified.
                </p>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
                <div 
                  className="p-3 rounded-lg"
                  style={{ background: 'var(--color-surface-2, #1c2128)', border: '1px solid var(--color-border-default, #30363d)' }}
                >
                  <div className="text-[10px] text-[#8b949e] font-mono uppercase">Invariant INV-01</div>
                  <div className="text-white font-semibold mt-1">Double-Dispatch Fencing</div>
                  <div className="text-[11px] text-[#3fb950] mt-1 font-mono flex items-center gap-1">
                    <CheckCircle2 className="w-3 h-3" />
                    <span>Verified Active</span>
                  </div>
                </div>

                <div 
                  className="p-3 rounded-lg"
                  style={{ background: 'var(--color-surface-2, #1c2128)', border: '1px solid var(--color-border-default, #30363d)' }}
                >
                  <div className="text-[10px] text-[#8b949e] font-mono uppercase">Invariant INV-02</div>
                  <div className="text-white font-semibold mt-1">Deterministic Replay</div>
                  <div className="text-[11px] text-[#3fb950] mt-1 font-mono flex items-center gap-1">
                    <CheckCircle2 className="w-3 h-3" />
                    <span>CAS: bafy2bzace4v3k...</span>
                  </div>
                </div>

                <div 
                  className="p-3 rounded-lg"
                  style={{ background: 'var(--color-surface-2, #1c2128)', border: '1px solid var(--color-border-default, #30363d)' }}
                >
                  <div className="text-[10px] text-[#8b949e] font-mono uppercase">Invariant INV-03</div>
                  <div className="text-white font-semibold mt-1">Authority Ticket TTL</div>
                  <div className="text-[11px] text-[#d29922] mt-1 font-mono flex items-center gap-1">
                    <Clock className="w-3 h-3" />
                    <span>Expires in 18m</span>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* ── 3. Bottom Scoped Terminal (Claude Code & Orca SADE) ── */}
      <div 
        style={{ 
          height: isTerminalExpanded ? '340px' : '190px',
          background: 'var(--color-canvas-inset, #010409)',
          borderTop: '1px solid var(--color-border-default, #30363d)'
        }} 
        className="flex flex-col shrink-0 transition-all duration-200"
      >
        {/* Terminal Header */}
        <div 
          className="h-7 px-3 flex items-center justify-between select-none"
          style={{
            background: 'var(--color-surface-1, #161b22)',
            borderBottom: '1px solid var(--color-border-muted, #21262d)',
          }}
        >
          <div className="flex items-center gap-1 font-mono text-[10px]">
            <button
              onClick={() => setTerminalTab('shell')}
              className="px-2 py-0.5 rounded transition"
              style={{
                background: terminalTab === 'shell' ? 'var(--color-surface-2, #1c2128)' : 'transparent',
                color: terminalTab === 'shell' ? '#e6edf3' : '#8b949e',
                border: terminalTab === 'shell' ? '1px solid var(--color-border-default, #30363d)' : '1px solid transparent'
              }}
            >
              Scoped Terminal: custos git:(main)
            </button>
            <button
              onClick={() => setTerminalTab('build')}
              className="px-2 py-0.5 rounded transition"
              style={{
                background: terminalTab === 'build' ? 'var(--color-surface-2, #1c2128)' : 'transparent',
                color: terminalTab === 'build' ? '#e6edf3' : '#8b949e',
                border: terminalTab === 'build' ? '1px solid var(--color-border-default, #30363d)' : '1px solid transparent'
              }}
            >
              Cargo Output
            </button>
            <button
              onClick={() => setTerminalTab('tests')}
              className="px-2 py-0.5 rounded transition flex items-center gap-1"
              style={{
                background: terminalTab === 'tests' ? 'var(--color-surface-2, #1c2128)' : 'transparent',
                color: terminalTab === 'tests' ? '#3fb950' : '#8b949e',
                border: terminalTab === 'tests' ? '1px solid var(--color-border-default, #30363d)' : '1px solid transparent'
              }}
            >
              <span>Tests (14 passed)</span>
            </button>
          </div>

          <div className="flex items-center gap-2 text-[10px] font-mono">
            <span className="w-1.5 h-1.5 rounded-full bg-[#3fb950] animate-pulse"></span>
            <span className="text-[#3fb950]">authority-mediated</span>
            <button
              onClick={() => setIsTerminalExpanded(!isTerminalExpanded)}
              className="p-1 rounded text-[#8b949e] hover:text-white transition ml-1"
              title={isTerminalExpanded ? 'Collapse Terminal' : 'Expand Terminal'}
            >
              {isTerminalExpanded ? <Minimize2 className="w-3 h-3" /> : <Maximize2 className="w-3 h-3" />}
            </button>
          </div>
        </div>

        {/* Terminal Stream */}
        <div className="flex-1 p-2.5 font-mono text-[11px] overflow-y-auto leading-relaxed text-[#c9d1d9] space-y-1 select-text">
          {terminalTab === 'shell' && terminalLines.map((line, idx) => (
            <div key={idx} className="flex items-start gap-2">
              <span className="text-[10px] text-[#6e7681] shrink-0 select-none">{line.time}</span>
              {line.owner === 'mediated' && (
                <span className="text-[9px] px-1 rounded font-bold uppercase shrink-0 select-none text-[#3fb950]" style={{ background: 'rgba(63,185,80,0.15)' }}>
                  Custos-Mediated
                </span>
              )}
              {line.owner === 'manual' && (
                <span className="text-[9px] px-1 rounded font-bold uppercase shrink-0 select-none text-[#58a6ff]" style={{ background: 'rgba(88,166,255,0.15)' }}>
                  Manual
                </span>
              )}
              <span className={line.owner === 'manual' || line.text.startsWith('$') ? 'text-white font-semibold' : line.text.includes('✓') || line.text.includes('passed') ? 'text-[#3fb950]' : 'text-[#8b949e]'}>
                {line.text}
              </span>
            </div>
          ))}

          {terminalTab === 'build' && (
            <div className="space-y-1 text-[#c9d1d9]">
              <div className="text-[#8b949e]">$ cargo build --workspace --release</div>
              <div className="text-[#3fb950]">   Compiling custos-domain v0.1.0</div>
              <div className="text-[#3fb950]">   Compiling custos-core v0.1.0</div>
              <div className="text-[#3fb950]">   Compiling custos-runtime v0.1.0</div>
              <div className="text-[#3fb950]">   Compiling custos-daemon v0.1.0</div>
              <div className="text-[#e6edf3] font-semibold">    Finished release [optimized] target(s) in 3.12s. 0 warnings.</div>
            </div>
          )}

          {terminalTab === 'tests' && (
            <div className="space-y-1 text-[#c9d1d9]">
              <div className="text-[#8b949e]">$ cargo test -p custos-runtime</div>
              <div className="text-[#3fb950]">test workflow::dispatcher::test_claim_permit ... ok</div>
              <div className="text-[#3fb950]">test workflow::dispatcher::test_finalize_closure ... ok</div>
              <div className="text-[#3fb950]">test authority::permits::test_double_dispatch_fencing ... ok</div>
              <div className="text-[#3fb950]">test oi::estimator::test_complexity_penalty ... ok</div>
              <div className="text-[#e6edf3] font-bold pt-1">test result: ok. 14 passed; 0 failed; 0 ignored; finished in 0.44s</div>
            </div>
          )}
        </div>

        {/* Terminal Command Input */}
        <form 
          onSubmit={handleTerminalSubmit} 
          className="px-2.5 py-1.5 flex items-center"
          style={{
            borderTop: '1px solid var(--color-border-muted, #21262d)',
            background: 'var(--color-canvas-inset, #010409)'
          }}
        >
          <div className="flex items-center gap-1.5 font-mono text-xs mr-2 shrink-0 select-none">
            <span className="text-[#3fb950] font-bold">→</span>
            <span className="text-[#58a6ff] font-bold">Custos</span>
            <span className="text-[#8b949e]">git:(<span className="text-[#f85149] font-bold">main</span>)</span>
            <span className="text-[#d29922] font-bold">⚡</span>
          </div>
          <input
            type="text"
            value={terminalInput}
            onChange={(e) => setTerminalInput(e.target.value)}
            placeholder="Run command (cargo test, git status, custos check, clear)..."
            className="w-full bg-transparent font-mono text-xs text-white placeholder-[#6e7681] focus:outline-none"
          />
        </form>
      </div>

      {/* Orca ADE Worktree Lifecycle Manager Modal */}
      <WorktreeManagerModal
        isOpen={isWorktreeModalOpen}
        onClose={() => setIsWorktreeModalOpen(false)}
        activeWorktreeId={activeWorktree.id}
        onSelectWorktree={(wt) => setActiveWorktree(wt)}
      />
    </div>
  );
};

export default EngineeringWorkspace;
