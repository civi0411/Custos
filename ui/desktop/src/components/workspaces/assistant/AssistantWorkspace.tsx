import React, { useState } from 'react';
import { 
  Bot, 
  Cpu, 
  Pause, 
  RotateCcw,
  Zap,
  Shield,
  Key,
  Clock,
  CheckCircle2,
  ArrowRight,
  LayoutGrid,
  GitBranch,
  AlertTriangle
} from 'lucide-react';

interface AgentTask {
  id: string;
  name: string;
  role: string;
  status: 'running' | 'idle' | 'completed' | 'error';
  progress: number;
  model: string;
  runtime: string;
  tokens: string;
  activePermit: string;
  invariantStatus: string;
}

interface OmniRouteRule {
  id: string;
  domain: string;
  primaryModel: string;
  fallbackModel: string;
  latencyMs: number;
  costPer1MTokens: string;
  reliability: number;
  status: 'active' | 'degraded';
}

interface CapabilityPermit {
  id: string;
  capability: string;
  actor: string;
  scope: string;
  ttlRemaining: string;
  status: 'granted' | 'fenced' | 'revoked';
}

const MOCK_AGENTS: AgentTask[] = [
  {
    id: 'agent-planner',
    name: 'S2 Master Architectural Planner',
    role: 'Hierarchical Decomposition & Task Routing',
    status: 'running',
    progress: 75,
    model: 'Claude 3.7 Sonnet',
    runtime: '1m 24s',
    tokens: '4.2k',
    activePermit: 'TICKET-8042-S2',
    invariantStatus: 'INV-01 Verified'
  },
  {
    id: 'agent-coder',
    name: 'Codex Code Synthesizer',
    role: 'Rust Invariant Implementation & AST Repair',
    status: 'running',
    progress: 45,
    model: 'Claude 3.7 Sonnet',
    runtime: '48s',
    tokens: '2.8k',
    activePermit: 'TICKET-8045-CODE',
    invariantStatus: 'INV-03 Leased'
  },
  {
    id: 'agent-guardian',
    name: 'Security & Policy Auditor',
    role: 'Zero-IO Sandboxing & API Key Guard',
    status: 'completed',
    progress: 100,
    model: 'DeepSeek V3',
    runtime: '12s',
    tokens: '850',
    activePermit: 'TICKET-8039-GUARD',
    invariantStatus: 'Proof Closed'
  },
  {
    id: 'agent-verifier',
    name: 'Deterministic Proof Verifier',
    role: 'CAS Ledger Hashing & Invariant Replay',
    status: 'idle',
    progress: 100,
    model: 'Gemini 2.5 Flash',
    runtime: '8s',
    tokens: '420',
    activePermit: 'TICKET-8012-AUDIT',
    invariantStatus: 'Clean'
  }
];

const OMNIROUTE_RULES: OmniRouteRule[] = [
  {
    id: 'route-code',
    domain: 'Complex Coding & Invariants',
    primaryModel: 'Claude 3.7 Sonnet',
    fallbackModel: 'DeepSeek V3',
    latencyMs: 380,
    costPer1MTokens: '$3.00',
    reliability: 99.9,
    status: 'active'
  },
  {
    id: 'route-fast',
    domain: 'Terminal Commands & Quick Triage',
    primaryModel: 'Gemini 2.5 Flash',
    fallbackModel: 'Claude 3.5 Haiku',
    latencyMs: 120,
    costPer1MTokens: '$0.15',
    reliability: 99.8,
    status: 'active'
  },
  {
    id: 'route-research',
    domain: 'Literature & Hypothesis Extraction',
    primaryModel: 'Claude 3.7 Sonnet',
    fallbackModel: 'Gemini 2.5 Flash',
    latencyMs: 420,
    costPer1MTokens: '$3.00',
    reliability: 99.7,
    status: 'active'
  },
  {
    id: 'route-local',
    domain: 'Local Offline Invariant Check',
    primaryModel: 'Local Llama 3.3 (Ollama)',
    fallbackModel: 'None',
    latencyMs: 45,
    costPer1MTokens: '$0.00',
    reliability: 100,
    status: 'active'
  }
];

const CAPABILITY_PERMITS: CapabilityPermit[] = [
  { id: 'TICKET-8045-CODE', capability: 'workspace:write', actor: 'Codex Code Synthesizer', scope: 'crates/custos-runtime/src', ttlRemaining: '18m 42s', status: 'granted' },
  { id: 'TICKET-8042-S2', capability: 'daemon:dispatch', actor: 'S2 Master Planner', scope: 'tasks:all', ttlRemaining: '44m 10s', status: 'granted' },
  { id: 'TICKET-8099-NET', capability: 'network:egress', actor: 'Research Ingestion', scope: 'api.semanticscholar.org', ttlRemaining: '2m 15s', status: 'fenced' },
  { id: 'TICKET-7991-SH', capability: 'shell:exec', actor: 'Manual User Session', scope: 'cargo build, git status', ttlRemaining: 'expired', status: 'revoked' }
];

export const AssistantWorkspace: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'fleet' | 'kanban' | 'omniroute' | 'permits'>('fleet');
  const [selectedAgent, setSelectedAgent] = useState<AgentTask>(MOCK_AGENTS[0]);
  const [permits, setPermits] = useState<CapabilityPermit[]>(CAPABILITY_PERMITS);

  const handleRevokePermit = (id: string) => {
    setPermits(prev => prev.map(p => p.id === id ? { ...p, status: 'revoked', ttlRemaining: 'revoked' } : p));
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
      {/* ── 1. Top Header Bar ── */}
      <div 
        className="h-9 px-3 flex items-center justify-between text-xs shrink-0"
        style={{
          background: 'var(--color-surface-1, #161b22)',
          borderBottom: '1px solid var(--color-border-default, #30363d)',
        }}
      >
        <div className="flex items-center gap-2">
          <div className="p-1 rounded" style={{ background: 'rgba(167,139,250,0.15)', color: 'var(--color-copilot, #a78bfa)' }}>
            <Bot className="w-3.5 h-3.5" />
          </div>
          <span className="font-semibold text-xs text-white">Supervised Agent Fleet</span>
          <span 
            className="text-[10px] px-2 py-0.2 rounded-full font-mono hidden sm:inline"
            style={{
              background: 'rgba(167,139,250,0.1)',
              color: 'var(--color-copilot, #a78bfa)',
              border: '1px solid rgba(167,139,250,0.2)'
            }}
          >
            4 Subagents · OmniRoute Active
          </span>
        </div>

        {/* Sub-tabs */}
        <div 
          className="flex items-center p-0.5 rounded text-[11px]"
          style={{
            background: 'var(--color-surface-0, #0d1117)',
            border: '1px solid var(--color-border-muted, #21262d)',
          }}
        >
          <button
            onClick={() => setActiveTab('fleet')}
            className="px-2.5 py-0.5 rounded transition font-medium flex items-center gap-1.5"
            style={{
              background: activeTab === 'fleet' ? 'var(--color-surface-2, #1c2128)' : 'transparent',
              color: activeTab === 'fleet' ? 'var(--color-editor-fg, #e6edf3)' : 'var(--color-fg-muted, #8b949e)',
              border: activeTab === 'fleet' ? '1px solid var(--color-border-default, #30363d)' : '1px solid transparent'
            }}
          >
            <Cpu className="w-3 h-3 text-[#a78bfa]" />
            <span>Agent Fleet</span>
          </button>

          <button
            onClick={() => setActiveTab('kanban')}
            className="px-2.5 py-0.5 rounded transition font-medium flex items-center gap-1.5"
            style={{
              background: activeTab === 'kanban' ? 'var(--color-surface-2, #1c2128)' : 'transparent',
              color: activeTab === 'kanban' ? 'var(--color-editor-fg, #e6edf3)' : 'var(--color-fg-muted, #8b949e)',
              border: activeTab === 'kanban' ? '1px solid var(--color-border-default, #30363d)' : '1px solid transparent'
            }}
          >
            <LayoutGrid className="w-3 h-3 text-[#3fb950]" />
            <span>Orca Kanban (O11)</span>
          </button>

          <button
            onClick={() => setActiveTab('omniroute')}
            className="px-2.5 py-0.5 rounded transition font-medium flex items-center gap-1.5"
            style={{
              background: activeTab === 'omniroute' ? 'var(--color-surface-2, #1c2128)' : 'transparent',
              color: activeTab === 'omniroute' ? 'var(--color-editor-fg, #e6edf3)' : 'var(--color-fg-muted, #8b949e)',
              border: activeTab === 'omniroute' ? '1px solid var(--color-border-default, #30363d)' : '1px solid transparent'
            }}
          >
            <Zap className="w-3 h-3 text-[#3fb950]" />
            <span>OmniRoute Matrix</span>
          </button>

          <button
            onClick={() => setActiveTab('permits')}
            className="px-2.5 py-0.5 rounded transition font-medium flex items-center gap-1.5"
            style={{
              background: activeTab === 'permits' ? 'var(--color-surface-2, #1c2128)' : 'transparent',
              color: activeTab === 'permits' ? 'var(--color-editor-fg, #e6edf3)' : 'var(--color-fg-muted, #8b949e)',
              border: activeTab === 'permits' ? '1px solid var(--color-border-default, #30363d)' : '1px solid transparent'
            }}
          >
            <Shield className="w-3 h-3 text-[#58a6ff]" />
            <span>Permit Leases ({permits.filter(p => p.status === 'granted').length})</span>
          </button>
        </div>
      </div>

      {/* ── 2. Content Area ── */}

      {/* TAB 1: AGENT FLEET */}
      {activeTab === 'fleet' && (
        <div className="flex-1 flex min-h-0">
          {/* Left Column: Subagents List */}
          <div 
            className="w-80 flex flex-col shrink-0 select-none"
            style={{
              background: 'var(--color-surface-1, #161b22)',
              borderRight: '1px solid var(--color-border-default, #30363d)',
            }}
          >
            <div 
              className="p-2.5 flex items-center justify-between text-xs text-[#8b949e]"
              style={{ borderBottom: '1px solid var(--color-border-default, #30363d)' }}
            >
              <span className="font-semibold uppercase text-[10px] tracking-wider">Supervised Hierarchy</span>
              <span className="text-[10px] font-mono text-[#3fb950]">PID: 4092 · IPC Ready</span>
            </div>

            <div className="flex-1 overflow-y-auto p-2 space-y-1.5">
              {MOCK_AGENTS.map((agent) => {
                const isSelected = selectedAgent.id === agent.id;
                return (
                  <div
                    key={agent.id}
                    onClick={() => setSelectedAgent(agent)}
                    className="p-2.5 rounded-lg cursor-pointer transition text-xs"
                    style={{
                      background: isSelected ? 'var(--color-surface-2, #1c2128)' : 'transparent',
                      border: `1px solid ${isSelected ? 'var(--color-copilot, #a78bfa)' : 'transparent'}`,
                      color: isSelected ? '#ffffff' : '#c9d1d9'
                    }}
                  >
                    <div className="flex items-center justify-between mb-1">
                      <div className="flex items-center gap-1.5 font-semibold text-white truncate">
                        <span 
                          className="w-2 h-2 rounded-full shrink-0" 
                          style={{
                            background: agent.status === 'running' ? '#3fb950' : agent.status === 'completed' ? '#58a6ff' : '#6e7681'
                          }} 
                        />
                        <span className="truncate">{agent.name}</span>
                      </div>
                      <span className="text-[10px] font-mono text-[#8b949e] shrink-0">{agent.runtime}</span>
                    </div>

                    <p className="text-[11px] text-[#8b949e] truncate mb-2">
                      {agent.role}
                    </p>

                    {/* Progress Bar */}
                    <div className="space-y-1">
                      <div className="w-full h-1 rounded-full overflow-hidden" style={{ background: 'var(--color-surface-0, #0d1117)' }}>
                        <div 
                          className="h-full transition-all duration-300"
                          style={{
                            width: `${agent.progress}%`,
                            background: agent.status === 'running' ? '#3fb950' : '#58a6ff'
                          }}
                        />
                      </div>
                      <div className="flex items-center justify-between text-[10px] text-[#8b949e] font-mono">
                        <span>{agent.model}</span>
                        <span>{agent.tokens} tokens</span>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Right Column: Agent Inspector */}
          <div 
            className="flex-1 flex flex-col min-w-0 overflow-y-auto p-6 space-y-6 select-text"
            style={{ background: 'var(--color-canvas, #0d1117)' }}
          >
            {/* Header */}
            <div className="flex items-center justify-between pb-4" style={{ borderBottom: '1px solid var(--color-border-default, #30363d)' }}>
              <div>
                <div className="flex items-center gap-2 mb-1">
                  <h2 className="text-base font-bold text-white">{selectedAgent.name}</h2>
                  <span 
                    className="text-[10px] font-mono uppercase px-2 py-0.5 rounded font-bold"
                    style={{
                      background: selectedAgent.status === 'running' ? 'rgba(63,185,80,0.15)' : 'rgba(88,166,255,0.15)',
                      color: selectedAgent.status === 'running' ? '#3fb950' : '#58a6ff',
                      border: `1px solid ${selectedAgent.status === 'running' ? '#3fb950' : '#58a6ff'}40`
                    }}
                  >
                    {selectedAgent.status}
                  </span>
                </div>
                <p className="text-xs text-[#8b949e]">{selectedAgent.role}</p>
              </div>

              <div className="flex items-center gap-2">
                <button 
                  className="p-1.5 rounded text-[#8b949e] hover:text-white transition"
                  style={{ background: 'var(--color-surface-2, #1c2128)', border: '1px solid var(--color-border-default, #30363d)' }}
                  title="Pause Agent"
                >
                  <Pause className="w-3.5 h-3.5" />
                </button>
                <button 
                  className="p-1.5 rounded text-[#8b949e] hover:text-white transition"
                  style={{ background: 'var(--color-surface-2, #1c2128)', border: '1px solid var(--color-border-default, #30363d)' }}
                  title="Restart Agent"
                >
                  <RotateCcw className="w-3.5 h-3.5" />
                </button>
              </div>
            </div>

            {/* Metrics Grid */}
            <div className="grid grid-cols-3 gap-3">
              <div 
                className="p-3 rounded-xl"
                style={{ background: 'var(--color-surface-1, #161b22)', border: '1px solid var(--color-border-default, #30363d)' }}
              >
                <div className="text-[10px] uppercase font-semibold text-[#8b949e] mb-1">Model Assigned</div>
                <div className="text-xs font-bold text-white font-mono">{selectedAgent.model}</div>
              </div>
              <div 
                className="p-3 rounded-xl"
                style={{ background: 'var(--color-surface-1, #161b22)', border: '1px solid var(--color-border-default, #30363d)' }}
              >
                <div className="text-[10px] uppercase font-semibold text-[#8b949e] mb-1">Active Permit Lease</div>
                <div className="text-xs font-bold text-[#3fb950] font-mono">{selectedAgent.activePermit}</div>
              </div>
              <div 
                className="p-3 rounded-xl"
                style={{ background: 'var(--color-surface-1, #161b22)', border: '1px solid var(--color-border-default, #30363d)' }}
              >
                <div className="text-[10px] uppercase font-semibold text-[#8b949e] mb-1">Invariant Contract</div>
                <div className="text-xs font-bold text-[#58a6ff] font-mono">{selectedAgent.invariantStatus}</div>
              </div>
            </div>

            {/* Live Streaming IPC Tool Call Log */}
            <div className="space-y-2">
              <div className="flex items-center justify-between text-xs font-semibold text-[#8b949e]">
                <span className="uppercase tracking-wider text-[10px]">Realtime Supervised Invocation Stream</span>
                <span className="text-[10px] font-mono text-[#3fb950] flex items-center gap-1">
                  <span className="w-1.5 h-1.5 rounded-full bg-[#3fb950] animate-pulse"></span>
                  Streaming IPC
                </span>
              </div>

              <div 
                className="p-4 rounded-xl font-mono text-[11px] space-y-2"
                style={{
                  background: 'var(--color-canvas-inset, #010409)',
                  border: '1px solid var(--color-border-default, #30363d)',
                  color: '#c9d1d9'
                }}
              >
                <div className="text-[#8b949e]">// Step 1: Pre-condition check: verify active lease ticket</div>
                <div>[0.10s] <span className="text-[#58a6ff]">permit_gate::check_lease</span>(ticket="{selectedAgent.activePermit}")</div>
                <div className="text-[#3fb950] pl-4">&rarr; Invariant lease valid, TTL = 18m remaining. Zero-IO fence established.</div>
                <div className="text-[#8b949e]">// Step 2: AST mutation dispatch under capability boundaries</div>
                <div>[0.34s] <span className="text-[#a78bfa]">synthesizer::dispatch</span>(crates/custos-runtime/src/workflow/dispatcher.rs)</div>
                <div className="text-[#c9d1d9] pl-4">&rarr; Applying AST patch: claim_ready_task with ticket acquisition</div>
                <div className="flex items-center gap-2 pt-2 text-[#3fb950]">
                  <CheckCircle2 className="w-3.5 h-3.5" />
                  <span>Synthesized patch verified against INV-01 compiler invariants.</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* TAB: ORCA AGENT KANBAN BOARD (Orca O11: Needs You / Working / Done / Idle) */}
      {activeTab === 'kanban' && (
        <div className="flex-1 flex flex-col p-5 overflow-y-auto space-y-4" style={{ background: 'var(--color-canvas, #0d1117)' }}>
          <div className="flex items-center justify-between pb-3" style={{ borderBottom: '1px solid var(--color-border-default, #30363d)' }}>
            <div>
              <h2 className="text-sm font-bold text-white flex items-center gap-2">
                <LayoutGrid className="w-4 h-4 text-[#3fb950]" />
                <span>Orca Agent Fleet Kanban (O11)</span>
              </h2>
              <p className="text-xs text-[#8b949e]">
                Fleet coordination across isolated worktrees, approval gates, and autonomous execution.
              </p>
            </div>
            <div className="flex items-center gap-2 text-xs font-mono text-[#3fb950]">
              <span className="w-2 h-2 rounded-full bg-[#3fb950] animate-pulse"></span>
              <span>4 Agents Registered · 1 Active Worktree</span>
            </div>
          </div>

          {/* 4 Kanban Columns */}
          <div className="grid grid-cols-1 md:grid-cols-4 gap-3 flex-1 min-h-[460px]">
            {/* Column 1: Needs You */}
            <div 
              className="flex flex-col rounded-xl p-3 space-y-3"
              style={{
                background: 'var(--color-surface-1, #161b22)',
                border: '1px solid rgba(248, 81, 73, 0.3)'
              }}
            >
              <div className="flex items-center justify-between pb-2 border-b border-[#30363d]">
                <div className="flex items-center gap-1.5 font-bold text-xs text-white">
                  <AlertTriangle className="w-3.5 h-3.5 text-[#f85149]" />
                  <span>Needs You</span>
                </div>
                <span className="px-1.5 py-0.2 rounded text-[10px] font-mono font-bold bg-red-500/15 text-[#f85149]">
                  1
                </span>
              </div>
              <p className="text-[10.5px] text-[#8b949e]">
                Blocked on human approval or missing capability permit
              </p>

              <div className="space-y-2 flex-1">
                <div 
                  className="p-3 rounded-lg space-y-2 select-text"
                  style={{ background: 'var(--color-surface-2, #1c2128)', border: '1px solid rgba(248, 81, 73, 0.4)' }}
                >
                  <div className="flex items-center justify-between text-[11px]">
                    <span className="font-semibold text-white">Codex Code Synthesizer</span>
                    <span className="text-[9.5px] font-mono text-[#f85149] font-bold">PERMIT BLOCKED</span>
                  </div>
                  <div className="text-[10px] font-mono text-[#8b949e] flex items-center gap-1">
                    <GitBranch className="w-3 h-3 text-[#3fb950]" />
                    <span>feat/simd-dispatch</span>
                  </div>
                  <p className="text-[10px] text-neutral-300">
                    Awaiting lease approval for <code className="text-[#e6edf3]">crates/.../dispatcher.rs</code> (+38 -4 lines)
                  </p>
                  <div className="pt-1 flex items-center justify-between text-[10px] border-t border-[#30363d]">
                    <span className="text-[#8b949e]">Model: Claude 3.7</span>
                    <span className="text-[#f85149] font-semibold">TICKET-8045</span>
                  </div>
                </div>
              </div>
            </div>

            {/* Column 2: Working */}
            <div 
              className="flex flex-col rounded-xl p-3 space-y-3"
              style={{
                background: 'var(--color-surface-1, #161b22)',
                border: '1px solid rgba(63, 185, 80, 0.3)'
              }}
            >
              <div className="flex items-center justify-between pb-2 border-b border-[#30363d]">
                <div className="flex items-center gap-1.5 font-bold text-xs text-white">
                  <span className="w-2 h-2 rounded-full bg-[#3fb950] animate-pulse"></span>
                  <span>Working</span>
                </div>
                <span className="px-1.5 py-0.2 rounded text-[10px] font-mono font-bold bg-emerald-500/15 text-[#3fb950]">
                  1
                </span>
              </div>
              <p className="text-[10.5px] text-[#8b949e]">
                Executing in isolated worktree with streaming PTY
              </p>

              <div className="space-y-2 flex-1">
                <div 
                  className="p-3 rounded-lg space-y-2 select-text"
                  style={{ background: 'var(--color-surface-2, #1c2128)', border: '1px solid rgba(63, 185, 80, 0.4)' }}
                >
                  <div className="flex items-center justify-between text-[11px]">
                    <span className="font-semibold text-white">S2 Master Planner</span>
                    <span className="text-[9.5px] font-mono text-[#3fb950] font-bold">75% PROGRESS</span>
                  </div>
                  <div className="text-[10px] font-mono text-[#8b949e] flex items-center gap-1">
                    <GitBranch className="w-3 h-3 text-[#3fb950]" />
                    <span>main (#a3f2d1e)</span>
                  </div>
                  <p className="text-[10px] text-neutral-300">
                    Synthesizing hierarchical Task DAG and verifying AST contracts
                  </p>
                  <div className="w-full bg-[#0d1117] h-1.5 rounded-full overflow-hidden">
                    <div className="bg-[#3fb950] h-full w-3/4 rounded-full"></div>
                  </div>
                  <div className="pt-1 flex items-center justify-between text-[10px] border-t border-[#30363d]">
                    <span className="text-[#8b949e]">Model: Claude 3.7</span>
                    <span className="text-[#3fb950]">Active 1m 24s</span>
                  </div>
                </div>
              </div>
            </div>

            {/* Column 3: Done */}
            <div 
              className="flex flex-col rounded-xl p-3 space-y-3"
              style={{
                background: 'var(--color-surface-1, #161b22)',
                border: '1px solid rgba(88, 166, 255, 0.3)'
              }}
            >
              <div className="flex items-center justify-between pb-2 border-b border-[#30363d]">
                <div className="flex items-center gap-1.5 font-bold text-xs text-white">
                  <CheckCircle2 className="w-3.5 h-3.5 text-[#58a6ff]" />
                  <span>Done</span>
                </div>
                <span className="px-1.5 py-0.2 rounded text-[10px] font-mono font-bold bg-blue-500/15 text-[#58a6ff]">
                  1
                </span>
              </div>
              <p className="text-[10.5px] text-[#8b949e]">
                Execution finished, awaiting criterion closure or merge
              </p>

              <div className="space-y-2 flex-1">
                <div 
                  className="p-3 rounded-lg space-y-2 select-text"
                  style={{ background: 'var(--color-surface-2, #1c2128)', border: '1px solid var(--color-border-default, #30363d)' }}
                >
                  <div className="flex items-center justify-between text-[11px]">
                    <span className="font-semibold text-white">Security & Policy Auditor</span>
                    <span className="text-[9.5px] font-mono text-[#58a6ff] font-bold">VERIFIED</span>
                  </div>
                  <div className="text-[10px] font-mono text-[#8b949e] flex items-center gap-1">
                    <GitBranch className="w-3 h-3 text-[#58a6ff]" />
                    <span>audit/zero-io</span>
                  </div>
                  <p className="text-[10px] text-neutral-300">
                    Proof closed: Zero-IO sandboxing checked, non-repudiation signed
                  </p>
                  <div className="pt-1 flex items-center justify-between text-[10px] border-t border-[#30363d]">
                    <span className="text-[#8b949e]">Model: DeepSeek V3</span>
                    <span className="text-[#58a6ff]">100%</span>
                  </div>
                </div>
              </div>
            </div>

            {/* Column 4: Idle */}
            <div 
              className="flex flex-col rounded-xl p-3 space-y-3"
              style={{
                background: 'var(--color-surface-1, #161b22)',
                border: '1px solid var(--color-border-default, #30363d)'
              }}
            >
              <div className="flex items-center justify-between pb-2 border-b border-[#30363d]">
                <div className="flex items-center gap-1.5 font-bold text-xs text-white">
                  <Clock className="w-3.5 h-3.5 text-[#8b949e]" />
                  <span>Idle</span>
                </div>
                <span className="px-1.5 py-0.2 rounded text-[10px] font-mono font-bold bg-neutral-800 text-[#8b949e]">
                  1
                </span>
              </div>
              <p className="text-[10.5px] text-[#8b949e]">
                Available in pool for immediate task dispatch
              </p>

              <div className="space-y-2 flex-1">
                <div 
                  className="p-3 rounded-lg space-y-2 select-text"
                  style={{ background: 'var(--color-surface-2, #1c2128)', border: '1px solid var(--color-border-default, #30363d)' }}
                >
                  <div className="flex items-center justify-between text-[11px]">
                    <span className="font-semibold text-white">Deterministic Proof Verifier</span>
                    <span className="text-[9.5px] font-mono text-[#8b949e]">STANDBY</span>
                  </div>
                  <div className="text-[10px] font-mono text-[#8b949e] flex items-center gap-1">
                    <Bot className="w-3 h-3 text-[#8b949e]" />
                    <span>Worker Pool #4</span>
                  </div>
                  <p className="text-[10px] text-[#8b949e]">
                    Ready for CAS ledger verification and evidence hashing
                  </p>
                  <div className="pt-1 flex items-center justify-between text-[10px] border-t border-[#30363d]">
                    <span className="text-[#8b949e]">Model: Gemini 2.5</span>
                    <span className="text-[#8b949e]">0% CPU</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* TAB 2: OMNIROUTE MATRIX */}
      {activeTab === 'omniroute' && (
        <div className="flex-1 flex flex-col p-6 overflow-y-auto space-y-4" style={{ background: 'var(--color-canvas, #0d1117)' }}>
          <div className="flex items-center justify-between pb-3" style={{ borderBottom: '1px solid var(--color-border-default, #30363d)' }}>
            <div>
              <h2 className="text-sm font-bold text-white flex items-center gap-2">
                <Zap className="w-4 h-4 text-[#3fb950]" />
                <span>OmniRoute Dynamic Provider Cascade</span>
              </h2>
              <p className="text-xs text-[#8b949e]">Automatic latency, cost, and reliability failover routing</p>
            </div>
            <div className="flex items-center gap-2 text-xs font-mono text-[#3fb950]">
              <span className="w-2 h-2 rounded-full bg-[#3fb950] animate-pulse"></span>
              <span>All 4 Routes Healthy</span>
            </div>
          </div>

          <div className="space-y-3">
            {OMNIROUTE_RULES.map((rule) => (
              <div 
                key={rule.id}
                className="p-4 rounded-xl flex items-center justify-between select-text"
                style={{
                  background: 'var(--color-surface-1, #161b22)',
                  border: '1px solid var(--color-border-default, #30363d)'
                }}
              >
                <div className="space-y-1">
                  <div className="text-sm font-semibold text-white">{rule.domain}</div>
                  <div className="flex items-center gap-2 font-mono text-xs text-[#8b949e]">
                    <span>Primary: <strong className="text-white">{rule.primaryModel}</strong></span>
                    <ArrowRight className="w-3 h-3 text-[#6e7681]" />
                    <span>Fallback: <strong className="text-[#8b949e]">{rule.fallbackModel}</strong></span>
                  </div>
                </div>

                <div className="flex items-center gap-6 font-mono text-xs">
                  <div>
                    <div className="text-[10px] text-[#8b949e]">Latency</div>
                    <div className="text-white font-bold">{rule.latencyMs}ms</div>
                  </div>
                  <div>
                    <div className="text-[10px] text-[#8b949e]">Cost / 1M</div>
                    <div className="text-[#3fb950] font-bold">{rule.costPer1MTokens}</div>
                  </div>
                  <div>
                    <div className="text-[10px] text-[#8b949e]">Reliability</div>
                    <div className="text-white font-bold">{rule.reliability}%</div>
                  </div>
                  <span 
                    className="px-2 py-0.5 rounded text-[11px] font-bold uppercase"
                    style={{ background: 'rgba(63,185,80,0.15)', color: '#3fb950', border: '1px solid rgba(63,185,80,0.3)' }}
                  >
                    {rule.status}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* TAB 3: CAPABILITY PERMITS */}
      {activeTab === 'permits' && (
        <div className="flex-1 flex flex-col p-6 overflow-y-auto space-y-4" style={{ background: 'var(--color-canvas, #0d1117)' }}>
          <div className="flex items-center justify-between pb-3" style={{ borderBottom: '1px solid var(--color-border-default, #30363d)' }}>
            <div>
              <h2 className="text-sm font-bold text-white flex items-center gap-2">
                <Shield className="w-4 h-4 text-[#58a6ff]" />
                <span>Authority Permit & Lease Registry</span>
              </h2>
              <p className="text-xs text-[#8b949e]">Active cryptographic capability leases granted by PermitGate</p>
            </div>
            <span className="text-xs font-mono text-[#8b949e]">Enforces INV-01 Double-Dispatch Fencing</span>
          </div>

          <div className="space-y-3">
            {permits.map((permit) => (
              <div 
                key={permit.id}
                className="p-4 rounded-xl flex items-center justify-between select-text"
                style={{
                  background: 'var(--color-surface-1, #161b22)',
                  border: '1px solid var(--color-border-default, #30363d)'
                }}
              >
                <div className="space-y-1">
                  <div className="flex items-center gap-2">
                    <Key className="w-3.5 h-3.5 text-[#58a6ff]" />
                    <span className="font-mono text-xs font-bold text-white">{permit.id}</span>
                    <span 
                      className="text-[10px] font-mono px-1.5 py-0.2 rounded font-bold uppercase"
                      style={{ background: 'rgba(88,166,255,0.15)', color: '#58a6ff' }}
                    >
                      {permit.capability}
                    </span>
                  </div>
                  <div className="text-xs text-[#8b949e]">
                    Actor: <strong className="text-white">{permit.actor}</strong> · Scope: <code className="text-neutral-300 font-mono">{permit.scope}</code>
                  </div>
                </div>

                <div className="flex items-center gap-4 font-mono text-xs">
                  <div className="flex items-center gap-1 text-[#8b949e]">
                    <Clock className="w-3 h-3 text-[#d29922]" />
                    <span>{permit.ttlRemaining}</span>
                  </div>

                  <span 
                    className="px-2 py-0.5 rounded text-[11px] font-bold uppercase"
                    style={{
                      background: permit.status === 'granted' ? 'rgba(63,185,80,0.15)' : permit.status === 'fenced' ? 'rgba(210,153,34,0.15)' : 'rgba(248,81,73,0.15)',
                      color: permit.status === 'granted' ? '#3fb950' : permit.status === 'fenced' ? '#d29922' : '#f85149'
                    }}
                  >
                    {permit.status}
                  </span>

                  {permit.status === 'granted' && (
                    <button
                      onClick={() => handleRevokePermit(permit.id)}
                      className="px-2 py-1 rounded text-[10px] text-[#f85149] hover:bg-red-500/10 transition border border-red-500/30"
                    >
                      Revoke Lease
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

export default AssistantWorkspace;
