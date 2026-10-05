import React, { useState, useRef, useEffect } from 'react';
import { 
  FileCode, 
  Copy, 
  Check, 
  Folder, 
  Play, 
  CheckCircle2, 
  ChevronRight,
  Maximize2,
  Minimize2,
  Sparkles,
  Zap,
  ArrowRight,
  X
} from 'lucide-react';
import { Session } from '@/types';

interface EngineeringWorkspaceProps {
  session?: Session | null;
  onAcceptAndRun?: () => void;
  onRejectDiff?: () => void;
  onCopyDiff?: () => void;
}

export const EngineeringWorkspace: React.FC<EngineeringWorkspaceProps> = ({
  session,
  onAcceptAndRun,
  onRejectDiff: _onRejectDiff,
  onCopyDiff: _onCopyDiff
}) => {
  const [viewSubTab, setViewSubTab] = useState<'editor' | 'diff' | 'invariants'>('editor');
  const [terminalTab, setTerminalTab] = useState<'shell' | 'build' | 'tests'>('shell');
  const [terminalInput, setTerminalInput] = useState('');
  const [terminalLines, setTerminalLines] = useState<string[]>([
    '$ cargo build --workspace',
    '   Compiling custos-core v0.1.0 (/Users/mac/Project/AgentHub/Custos/crates/custos-core)',
    '   Compiling custos-daemon v0.1.0 (/Users/mac/Project/AgentHub/Custos/crates/custos-daemon)',
    '    Finished dev [unoptimized + debuginfo] target(s) in 1.48s',
    `$ custos session status ${session?.id || 'vi'}`,
    '   ✓ Invariant checks verified: 0 violations, zero-IO proof closure established.',
    '$ tauri dev --ready'
  ]);
  const [copied, setCopied] = useState(false);
  const [isTerminalExpanded, setIsTerminalExpanded] = useState(false);

  // Codex AI Assistant Inline State
  const [isCodexPromptOpen, setIsCodexPromptOpen] = useState(false);
  const [codexInput, setCodexInput] = useState('');
  const [isCodexGenerating, setIsCodexGenerating] = useState(false);
  const [codexSuggestion, setCodexSuggestion] = useState<string | null>(null);

  const codexInputRef = useRef<HTMLInputElement>(null);

  const fileName = session?.fileName || 'crates/custos-runtime/src/workflow/dispatcher.rs';

  const defaultCode = `// Custos Workflow Dispatcher Runtime
use std::sync::Arc;
use custos_domain::action::Action;
use custos_core::authority::permits::PermitGate;

pub struct WorkflowDispatcher {
    permit_gate: Arc<PermitGate>,
    max_inflight_actions: usize,
}

impl WorkflowDispatcher {
    pub fn new(permit_gate: Arc<PermitGate>) -> Self {
        Self {
            permit_gate,
            max_inflight_actions: 64,
        }
    }

    /// Claim ready task and verify pre-condition invariant contracts
    pub async fn claim_ready_task(&self, task_id: &str) -> Result<Action, String> {
        let permit = self.permit_gate.acquire_ticket(task_id).await?;
        println!("Claimed task {} under invariant gate {:?}", task_id, permit);
        Ok(Action::Execute(task_id.to_string()))
    }
}`;

  useEffect(() => {
    if (isCodexPromptOpen) {
      setTimeout(() => codexInputRef.current?.focus(), 50);
    }
  }, [isCodexPromptOpen]);

  const handleCopyCode = () => {
    navigator.clipboard.writeText(defaultCode);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleTerminalSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!terminalInput.trim()) return;
    const cmd = terminalInput.trim();
    setTerminalLines(prev => [
      ...prev, 
      `$ ${cmd}`, 
      cmd.startsWith('cargo test')
        ? 'running 14 tests ... test result: ok. 14 passed; 0 failed; 0 ignored'
        : cmd.startsWith('git')
        ? 'On branch vi (Custos autonomous tree)\nnothing to commit, working tree clean'
        : `custos: command '${cmd.split(' ')[0]}' executed with code 0.`
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
        `// [Codex Synthesized] Invariant-guarded claim logic\npub async fn claim_with_timeout(&self, task_id: &str, timeout_ms: u64) -> Result<Action, String> {\n    let permit = tokio::time::timeout(\n        std::time::Duration::from_millis(timeout_ms),\n        self.permit_gate.acquire_ticket(task_id)\n    ).await.map_err(|_| "Permit timeout expired".to_string())??;\n    Ok(Action::Execute(task_id.to_string()))\n}`
      );
    }, 800);
  };

  const applyCodexSuggestion = () => {
    setCodexSuggestion(null);
    setIsCodexPromptOpen(false);
    setCodexInput('');
    setTerminalLines(prev => [
      ...prev,
      '$ custos codex apply --patch=inline-synth-01',
      '   ✓ Successfully merged Codex synthesized invariant into working tree.'
    ]);
  };

  return (
    <div className="flex-1 flex flex-col bg-[#090b10] text-neutral-300 h-full overflow-hidden font-sans border-l border-surface-border">
      {/* 1. Engineering Header & Breadcrumbs */}
      <div className="h-10 border-b border-surface-border bg-surface px-3 flex items-center justify-between text-xs shrink-0 select-none">
        {/* Breadcrumb path */}
        <div className="flex items-center gap-1.5 text-neutral-400 font-mono text-[11px] truncate">
          <Folder className="w-3.5 h-3.5 text-neutral-500 shrink-0" />
          <span className="text-neutral-400">Custos</span>
          <ChevronRight className="w-3 h-3 text-neutral-600 shrink-0" />
          <FileCode className="w-3.5 h-3.5 text-blue-400 shrink-0" />
          <span className="font-semibold text-white truncate">{fileName}</span>
          <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 ml-1 shrink-0 animate-pulse"></span>
        </div>

        {/* View Sub-Tabs: Editor | Diff Review | Invariants */}
        <div className="flex items-center gap-2 shrink-0">
          {/* Codex Assist Toggle Button */}
          <button
            onClick={() => setIsCodexPromptOpen(!isCodexPromptOpen)}
            className={`px-2 py-1 rounded-md text-[11px] font-medium flex items-center gap-1.5 transition border ${
              isCodexPromptOpen
                ? 'bg-blue-500/10 border-blue-500/40 text-blue-400'
                : 'bg-surface-elevated hover:bg-surface-hover border-surface-border text-neutral-300 hover:text-white'
            }`}
            title="Open Codex Inline AI (⌘K)"
          >
            <Sparkles className="w-3 h-3 text-blue-400" />
            <span>Codex AI</span>
            <kbd className="text-[9px] bg-surface-card px-1 py-0.2 rounded border border-surface-border text-neutral-400 font-mono">⌘K</kbd>
          </button>

          <div className="flex items-center bg-[#141722] p-0.5 rounded-lg border border-surface-border text-[11px]">
            <button
              onClick={() => setViewSubTab('editor')}
              className={`px-2 py-0.5 rounded-md transition font-medium ${
                viewSubTab === 'editor'
                  ? 'bg-surface-elevated text-white shadow-sm'
                  : 'text-neutral-400 hover:text-white'
              }`}
            >
              Editor
            </button>
            <button
              onClick={() => setViewSubTab('diff')}
              className={`px-2 py-0.5 rounded-md transition font-medium ${
                viewSubTab === 'diff'
                  ? 'bg-surface-elevated text-white shadow-sm'
                  : 'text-neutral-400 hover:text-white'
              }`}
            >
              Diff ({session?.diffLinesCount || '+34 -2'})
            </button>
            <button
              onClick={() => setViewSubTab('invariants')}
              className={`px-2 py-0.5 rounded-md transition font-medium ${
                viewSubTab === 'invariants'
                  ? 'bg-surface-elevated text-white shadow-sm'
                  : 'text-neutral-400 hover:text-white'
              }`}
            >
              Invariants
            </button>
          </div>

          {/* Action buttons */}
          {onAcceptAndRun && (
            <button
              onClick={onAcceptAndRun}
              className="px-2.5 py-1 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white font-medium text-[11px] flex items-center gap-1.5 shadow-sm transition"
              title="Apply patch and execute"
            >
              <Play className="w-3 h-3 fill-current" />
              <span>Accept & Run</span>
            </button>
          )}

          <button
            onClick={handleCopyCode}
            className="p-1 rounded-md hover:bg-surface-elevated text-neutral-400 hover:text-white border border-transparent hover:border-surface-border transition"
            title="Copy code"
          >
            {copied ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
          </button>
        </div>
      </div>

      {/* 2. Main Stage: Editor or Diff View or Invariants */}
      <div className="flex-1 flex flex-col min-w-0 overflow-hidden bg-[#090b10] relative select-text">
        {/* Floating Codex Inline Prompt Bar (Codex / Cursor Style) */}
        {isCodexPromptOpen && (
          <div className="absolute top-3 left-4 right-4 z-30 bg-[#121520]/95 backdrop-blur-md border border-blue-500/40 rounded-xl shadow-2xl p-3 text-xs animate-in fade-in slide-in-from-top-2 duration-150">
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <Sparkles className="w-3.5 h-3.5 text-blue-400" />
                <span className="font-semibold text-white text-[12px]">Codex Inline Prompt</span>
                <span className="text-[10px] text-blue-400 bg-blue-500/10 px-1.5 py-0.5 rounded font-mono">
                  Context: {fileName}
                </span>
              </div>
              <button 
                onClick={() => setIsCodexPromptOpen(false)}
                className="text-neutral-400 hover:text-white p-0.5 rounded hover:bg-surface-elevated transition"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            </div>

            {/* Input Form */}
            <form onSubmit={(e) => { e.preventDefault(); handleTriggerCodex(); }} className="flex items-center gap-2 mb-2">
              <input
                ref={codexInputRef}
                type="text"
                value={codexInput}
                onChange={(e) => setCodexInput(e.target.value)}
                placeholder="Instruct Codex (e.g., 'Add timeout handling to claim_ready_task', 'Generate unit tests')..."
                className="flex-1 bg-[#090b10] border border-surface-border rounded-lg px-3 py-1.5 text-white placeholder-neutral-500 focus:outline-none focus:border-blue-500 font-mono text-xs"
              />
              <button
                type="submit"
                disabled={isCodexGenerating || !codexInput.trim()}
                className="px-3 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white font-medium text-xs flex items-center gap-1.5 transition disabled:opacity-50"
              >
                {isCodexGenerating ? (
                  <>
                    <Zap className="w-3 h-3 animate-spin" />
                    <span>Synthesizing...</span>
                  </>
                ) : (
                  <>
                    <span>Generate</span>
                    <ArrowRight className="w-3 h-3" />
                  </>
                )}
              </button>
            </form>

            {/* Quick Starters Chips */}
            <div className="flex items-center gap-1.5 flex-wrap pt-1 border-t border-surface-border/50 text-[11px]">
              <span className="text-neutral-500 text-[10px]">Quick Actions:</span>
              <button
                type="button"
                onClick={() => {
                  setCodexInput('Add unit test verifying permit drop behavior');
                  handleTriggerCodex('Add unit test verifying permit drop behavior');
                }}
                className="px-2 py-0.5 rounded bg-surface-card hover:bg-surface-elevated text-neutral-300 border border-surface-border text-[10px] transition"
              >
                ⚡ Unit Tests
              </button>
              <button
                type="button"
                onClick={() => {
                  setCodexInput('Verify zero-IO invariant proof gate contracts');
                  handleTriggerCodex('Verify zero-IO invariant proof gate contracts');
                }}
                className="px-2 py-0.5 rounded bg-surface-card hover:bg-surface-elevated text-neutral-300 border border-surface-border text-[10px] transition"
              >
                🛡️ Invariant Check
              </button>
              <button
                type="button"
                onClick={() => {
                  setCodexInput('Refactor into async tokio select pipeline');
                  handleTriggerCodex('Refactor into async tokio select pipeline');
                }}
                className="px-2 py-0.5 rounded bg-surface-card hover:bg-surface-elevated text-neutral-300 border border-surface-border text-[10px] transition"
              >
                🚀 Async Pipeline
              </button>
            </div>

            {/* Codex Suggestion Diff Output */}
            {codexSuggestion && (
              <div className="mt-3 p-2.5 rounded-lg bg-[#07080c] border border-blue-500/30 font-mono text-[11px] space-y-2">
                <div className="flex items-center justify-between text-neutral-400 pb-1 border-b border-[#1b1f2b]">
                  <span className="text-blue-400 font-semibold flex items-center gap-1">
                    <CheckCircle2 className="w-3 h-3 text-emerald-400" />
                    Codex Proposed Synthesized Patch
                  </span>
                  <div className="flex items-center gap-1.5">
                    <button
                      onClick={() => setCodexSuggestion(null)}
                      className="px-2 py-0.5 rounded hover:bg-surface-elevated text-neutral-400 text-[10px] transition"
                    >
                      Discard
                    </button>
                    <button
                      onClick={applyCodexSuggestion}
                      className="px-2.5 py-0.5 rounded bg-emerald-600 hover:bg-emerald-500 text-white font-medium text-[10px] flex items-center gap-1 transition shadow-sm"
                    >
                      <Check className="w-3 h-3" />
                      <span>Apply Patch (⌘↵)</span>
                    </button>
                  </div>
                </div>
                <pre className="text-emerald-300 overflow-x-auto p-1 leading-relaxed">
                  {codexSuggestion}
                </pre>
              </div>
            )}
          </div>
        )}

        {viewSubTab === 'editor' && (
          <div className="flex-1 flex flex-col overflow-auto font-mono text-[12px] p-4 leading-relaxed text-neutral-200">
            <div className="space-y-0.5">
              {defaultCode.split('\n').map((line, idx) => (
                <div key={idx} className="flex hover:bg-white/[0.02] rounded px-1 group">
                  <span className="w-8 text-neutral-600 text-right pr-4 select-none text-[11px] group-hover:text-neutral-400 transition">
                    {idx + 1}
                  </span>
                  <span className="flex-1 text-neutral-300">
                    {line.startsWith('//') ? (
                      <span className="text-neutral-500 italic">{line}</span>
                    ) : line.includes('pub struct') || line.includes('pub fn') || line.includes('impl') ? (
                      <span className="text-purple-400 font-semibold">{line}</span>
                    ) : line.includes('use ') ? (
                      <span className="text-cyan-400">{line}</span>
                    ) : line.includes('println!') ? (
                      <span className="text-amber-400">{line}</span>
                    ) : (
                      line
                    )}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}

        {viewSubTab === 'diff' && (
          <div className="flex-1 flex flex-col overflow-auto font-mono text-[12px] p-4 leading-relaxed bg-[#0a0c12]">
            <div className="text-neutral-500 pb-2 text-[11px] font-mono border-b border-[#1b1f2b] mb-2 flex items-center justify-between">
              <span>{session?.diffHunk || '@@ -42,7 +42,12 @@ pub async fn claim_ready_task'}</span>
              <span className="text-emerald-400">{session?.diffLinesCount || '+34 -2 lines'}</span>
            </div>
            <div className="space-y-0.5">
              {(session?.diffCode || [
                { type: 'context', text: '     pub async fn claim_ready_task(&self, task_id: &str) -> Result<Action, String> {' },
                { type: 'del', text: '-        let permit = self.permit_gate.acquire(task_id).await?;' },
                { type: 'add', text: '+        // Enhanced zero-IO invariant ticket verification' },
                { type: 'add', text: '+        let permit = self.permit_gate.acquire_ticket(task_id).await?;' },
                { type: 'add', text: '+        println!("Claimed task {} under invariant gate {:?}", task_id, permit);' },
                { type: 'context', text: '         Ok(Action::Execute(task_id.to_string()))' },
                { type: 'context', text: '     }' }
              ]).map((line, idx) => (
                <div 
                  key={idx} 
                  className={`flex px-2 py-0.5 rounded font-mono ${
                    line.type === 'add'
                      ? 'bg-emerald-500/10 text-emerald-300 border-l-2 border-emerald-500'
                      : line.type === 'del'
                      ? 'bg-red-500/10 text-red-300 border-l-2 border-red-500 line-through'
                      : 'text-neutral-300'
                  }`}
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

        {viewSubTab === 'invariants' && (
          <div className="flex-1 p-4 overflow-y-auto text-xs font-sans space-y-3">
            <div className="p-3 rounded-xl bg-surface-card border border-surface-border">
              <div className="flex items-center gap-2 text-emerald-400 font-semibold mb-1">
                <CheckCircle2 className="w-4 h-4" />
                <span>Zero-IO Invariant Gate Verified</span>
              </div>
              <p className="text-neutral-400 text-[11px] leading-relaxed">
                All memory mutations within <code className="text-white font-mono">{fileName}</code> adhere to the safe sandbox contracts. No external network I/O permitted without signed tickets.
              </p>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-2.5">
              <div className="p-2.5 rounded-lg bg-surface border border-surface-border">
                <div className="text-[10px] text-neutral-500 font-mono uppercase">Permit Ticket</div>
                <div className="text-white font-mono mt-0.5">AUTH-TICKET-8045-G0</div>
              </div>
              <div className="p-2.5 rounded-lg bg-surface border border-surface-border">
                <div className="text-[10px] text-neutral-500 font-mono uppercase">Proof Closure</div>
                <div className="text-emerald-400 font-mono mt-0.5">100% Deterministic</div>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* 3. Bottom Embedded Terminal (Orca-Style Multi-tab Pane) */}
      <div 
        style={{ height: isTerminalExpanded ? '320px' : '190px' }} 
        className="border-t border-surface-border bg-[#07080c] flex flex-col shrink-0 transition-all duration-200"
      >
        {/* Terminal Header */}
        <div className="h-7 border-b border-surface-border flex items-center justify-between px-3 bg-surface select-none">
          <div className="flex items-center gap-2">
            <div className="flex items-center gap-1 font-mono text-[10px]">
              <button
                onClick={() => setTerminalTab('shell')}
                className={`px-2 py-0.5 rounded transition ${
                  terminalTab === 'shell'
                    ? 'bg-surface-elevated text-white font-semibold'
                    : 'text-neutral-400 hover:text-white'
                }`}
              >
                Terminal: custos git:(vi)
              </button>
              <button
                onClick={() => setTerminalTab('build')}
                className={`px-2 py-0.5 rounded transition ${
                  terminalTab === 'build'
                    ? 'bg-surface-elevated text-white font-semibold'
                    : 'text-neutral-400 hover:text-white'
                }`}
              >
                Cargo Output
              </button>
              <button
                onClick={() => setTerminalTab('tests')}
                className={`px-2 py-0.5 rounded transition ${
                  terminalTab === 'tests'
                    ? 'bg-surface-elevated text-white font-semibold'
                    : 'text-neutral-400 hover:text-white'
                }`}
              >
                Tests (14 passed)
              </button>
            </div>
          </div>

          <div className="flex items-center gap-2 text-[10px] font-mono">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
            <span className="text-emerald-400">ready</span>
            <button
              onClick={() => setIsTerminalExpanded(!isTerminalExpanded)}
              className="p-1 hover:bg-surface-elevated rounded text-neutral-500 hover:text-white transition ml-1"
              title={isTerminalExpanded ? 'Collapse' : 'Expand'}
            >
              {isTerminalExpanded ? <Minimize2 className="w-3 h-3" /> : <Maximize2 className="w-3 h-3" />}
            </button>
          </div>
        </div>

        {/* Terminal Console Output */}
        <div className="flex-1 p-2.5 font-mono text-[11px] overflow-y-auto leading-relaxed text-neutral-300 space-y-0.5 select-text">
          {terminalTab === 'shell' && terminalLines.map((line, idx) => (
            <div 
              key={idx} 
              className={
                line.startsWith('$') 
                  ? 'text-white font-bold' 
                  : line.includes('Compiling') 
                  ? 'text-neutral-400' 
                  : line.includes('Finished') || line.includes('passed') || line.includes('--ready') 
                  ? 'text-emerald-400' 
                  : 'text-neutral-300'
              }
            >
              {line}
            </div>
          ))}

          {terminalTab === 'build' && (
            <div className="text-neutral-300 space-y-1">
              <div className="text-neutral-400">$ cargo check --workspace --tests</div>
              <div className="text-emerald-400">✓ Checked 11 crates in 0.88s. 0 errors, 0 warnings.</div>
            </div>
          )}

          {terminalTab === 'tests' && (
            <div className="text-neutral-300 space-y-1">
              <div className="text-neutral-400">$ cargo test -p custos-runtime</div>
              <div className="text-emerald-400">test workflow::graph_runtime ... ok</div>
              <div className="text-emerald-400">test cognitive::s1::judge ... ok</div>
              <div className="text-emerald-400">test oi::engine ... ok</div>
              <div className="text-white font-bold pt-1">test result: ok. 14 passed; 0 failed.</div>
            </div>
          )}
        </div>

        {/* Terminal Input matching exact Orca screenshot */}
        <form onSubmit={handleTerminalSubmit} className="border-t border-surface-border px-2.5 py-1.5 flex items-center bg-[#07080c]">
          <div className="flex items-center gap-1.5 font-mono text-xs mr-2 shrink-0 select-none">
            <span className="text-emerald-400 font-bold">→</span>
            <span className="text-cyan-400 font-bold">Custos</span>
            <span className="text-blue-400">git:(<span className="text-rose-400 font-bold">vi</span>)</span>
            <span className="text-amber-400 font-bold">✗</span>
          </div>
          <input
            type="text"
            value={terminalInput}
            onChange={(e) => setTerminalInput(e.target.value)}
            placeholder="Type command (e.g. cargo check, git diff, custos run)..."
            className="w-full bg-transparent font-mono text-xs text-white placeholder-neutral-600 focus:outline-none"
          />
        </form>
      </div>
    </div>
  );
};

export default EngineeringWorkspace;
