import React, { useState } from 'react';
import { 
  Bot, 
  Cpu, 
  Workflow, 
  Database, 
  Pause, 
  RotateCcw,
  Zap
} from 'lucide-react';
import { AgentWorkingSpinner, AgentStateDot } from '@/components/common';

interface AgentTask {
  id: string;
  name: string;
  role: string;
  status: 'running' | 'idle' | 'completed' | 'error';
  progress: number;
  model: string;
  runtime: string;
  tokens: string;
}

const MOCK_AGENTS: AgentTask[] = [
  {
    id: 'agent-planner',
    name: 'Master Architectural Planner',
    role: 'Decomposition & Task Routing',
    status: 'running',
    progress: 75,
    model: 'Claude 3.7 Sonnet',
    runtime: '1m 24s',
    tokens: '4.2k'
  },
  {
    id: 'agent-reviewer',
    name: 'Code Synthesizer & Tester',
    role: 'TypeScript & Rust Verification',
    status: 'idle',
    progress: 100,
    model: 'Gemini 2.5 Flash',
    runtime: '45s',
    tokens: '1.8k'
  },
  {
    id: 'agent-guardian',
    name: 'Security & Policy Auditor',
    role: 'Sandboxing & API Key Guard',
    status: 'completed',
    progress: 100,
    model: 'DeepSeek V3',
    runtime: '12s',
    tokens: '850'
  }
];

export const AssistantWorkspace: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'fleet' | 'omniroute' | 'memory'>('fleet');
  const [selectedAgent, setSelectedAgent] = useState<AgentTask>(MOCK_AGENTS[0]);

  return (
    <div className="flex-1 flex flex-col bg-surface text-neutral-300 h-full overflow-hidden font-sans border-l border-surface-border">
      {/* Workspace Header */}
      <div className="h-11 border-b border-surface-border px-4 flex items-center justify-between shrink-0 bg-surface-card">
        <div className="flex items-center gap-2">
          <div className="p-1 rounded-md bg-amber-500/10 text-amber-400 border border-amber-500/20">
            <Bot className="w-4 h-4" />
          </div>
          <span className="text-xs font-semibold text-white">Assistant & Fleet Workspace</span>
          <span className="text-[10px] text-amber-400 bg-amber-500/10 px-2 py-0.5 rounded-full border border-amber-500/20 font-mono">
            3 Subagents Active
          </span>
        </div>

        {/* Sub-tabs */}
        <div className="flex items-center gap-1 bg-surface-elevated p-0.5 rounded-lg border border-surface-border">
          <button
            onClick={() => setActiveTab('fleet')}
            className={`px-2.5 py-1 rounded-md text-[11px] font-medium transition flex items-center gap-1.5 ${
              activeTab === 'fleet'
                ? 'bg-surface text-white shadow-sm border border-surface-border/80'
                : 'text-neutral-400 hover:text-white'
            }`}
          >
            <Cpu className="w-3 h-3 text-amber-400" />
            <span>Agent Fleet</span>
          </button>

          <button
            onClick={() => setActiveTab('omniroute')}
            className={`px-2.5 py-1 rounded-md text-[11px] font-medium transition flex items-center gap-1.5 ${
              activeTab === 'omniroute'
                ? 'bg-surface text-white shadow-sm border border-surface-border/80'
                : 'text-neutral-400 hover:text-white'
            }`}
          >
            <Workflow className="w-3 h-3 text-emerald-400" />
            <span>OmniRoute Pipeline</span>
          </button>

          <button
            onClick={() => setActiveTab('memory')}
            className={`px-2.5 py-1 rounded-md text-[11px] font-medium transition flex items-center gap-1.5 ${
              activeTab === 'memory'
                ? 'bg-surface text-white shadow-sm border border-surface-border/80'
                : 'text-neutral-400 hover:text-white'
            }`}
          >
            <Database className="w-3 h-3 text-cyan-400" />
            <span>KV Memory</span>
          </button>
        </div>
      </div>

      {/* Main Tab Content */}
      {activeTab === 'fleet' && (
        <div className="flex-1 flex min-h-0">
          {/* Left Column: Subagent List */}
          <div className="w-80 border-r border-surface-border flex flex-col bg-surface/50">
            <div className="p-3 border-b border-surface-border flex items-center justify-between text-xs text-neutral-400">
              <span className="font-semibold uppercase text-[10px] tracking-wider">Subagent Hierarchy</span>
              <span className="text-[10px] font-mono text-emerald-400">Daemon PID: 4092</span>
            </div>

            <div className="flex-1 overflow-y-auto p-2 space-y-2">
              {MOCK_AGENTS.map((agent) => {
                const isSelected = selectedAgent.id === agent.id;
                return (
                  <div
                    key={agent.id}
                    onClick={() => setSelectedAgent(agent)}
                    className={`p-3 rounded-xl cursor-pointer transition border text-xs ${
                      isSelected
                        ? 'bg-surface-elevated border-amber-500/50 shadow-md text-white'
                        : 'bg-surface-card hover:bg-surface-elevated/70 border-surface-border text-neutral-300'
                    }`}
                  >
                    <div className="flex items-center justify-between mb-1.5">
                      <div className="flex items-center gap-1.5">
                        <AgentStateDot 
                          state={agent.status === 'running' ? 'working' : agent.status === 'completed' ? 'done' : 'failed'} 
                          size="sm" 
                        />
                        <span className="font-semibold text-neutral-200">{agent.name}</span>
                      </div>
                      <span className="text-[10px] font-mono text-neutral-500">{agent.runtime}</span>
                    </div>

                    <p className="text-[11px] text-neutral-400 truncate mb-2">
                      {agent.role}
                    </p>

                    {/* Progress Bar */}
                    <div className="space-y-1">
                      <div className="w-full bg-surface-elevated h-1 rounded-full overflow-hidden">
                        <div 
                          className={`h-full transition-all duration-300 ${
                            agent.status === 'running' ? 'bg-amber-400' : 'bg-emerald-400'
                          }`}
                          style={{ width: `${agent.progress}%` }}
                        />
                      </div>
                      <div className="flex items-center justify-between text-[10px] text-neutral-500 font-mono">
                        <span>{agent.model}</span>
                        <span>{agent.tokens} tokens</span>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Right Column: Active Agent Inspector */}
          <div className="flex-1 flex flex-col bg-canvas min-w-0 overflow-y-auto p-6 space-y-6">
            <div className="flex items-center justify-between border-b border-surface-border pb-4">
              <div>
                <div className="flex items-center gap-2 mb-1">
                  <span className="text-base font-bold text-white">{selectedAgent.name}</span>
                  <span className="text-[10px] font-mono uppercase px-2 py-0.5 rounded bg-amber-500/10 text-amber-400 border border-amber-500/20">
                    {selectedAgent.status}
                  </span>
                </div>
                <p className="text-xs text-neutral-400">{selectedAgent.role}</p>
              </div>

              <div className="flex items-center gap-2">
                <button className="p-2 rounded-lg bg-surface-card hover:bg-surface-elevated border border-surface-border text-neutral-300 transition" title="Pause Agent">
                  <Pause className="w-3.5 h-3.5" />
                </button>
                <button className="p-2 rounded-lg bg-surface-card hover:bg-surface-elevated border border-surface-border text-neutral-300 transition" title="Restart Agent">
                  <RotateCcw className="w-3.5 h-3.5" />
                </button>
              </div>
            </div>

            {/* Live Metrics Grid */}
            <div className="grid grid-cols-3 gap-3">
              <div className="p-3 bg-surface-card rounded-xl border border-surface-border">
                <div className="text-[10px] uppercase font-semibold text-neutral-500 mb-1">Target Model</div>
                <div className="text-xs font-bold text-white font-mono">{selectedAgent.model}</div>
              </div>
              <div className="p-3 bg-surface-card rounded-xl border border-surface-border">
                <div className="text-[10px] uppercase font-semibold text-neutral-500 mb-1">Total Runtime</div>
                <div className="text-xs font-bold text-emerald-400 font-mono">{selectedAgent.runtime}</div>
              </div>
              <div className="p-3 bg-surface-card rounded-xl border border-surface-border">
                <div className="text-[10px] uppercase font-semibold text-neutral-500 mb-1">Consumed Tokens</div>
                <div className="text-xs font-bold text-amber-400 font-mono">{selectedAgent.tokens}</div>
              </div>
            </div>

            {/* Live Agent Execution Stream */}
            <div className="space-y-2">
              <div className="flex items-center justify-between text-xs font-semibold text-neutral-400">
                <span className="uppercase tracking-wider text-[10px]">Realtime Tool Invocation Log</span>
                <span className="text-[10px] font-mono text-emerald-400 flex items-center gap-1">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
                  Streaming IPC
                </span>
              </div>

              <div className="bg-[#0c0d12] p-4 rounded-xl border border-surface-border font-mono text-[11px] text-neutral-300 space-y-2 select-text">
                <div className="text-neutral-500">// Step 1: Query local codebase vector index</div>
                <div>[0.12s] <span className="text-cyan-400">tool_call:</span> semantic_search(query="Tauri IPC command registration")</div>
                <div className="text-emerald-400 pl-4">&rarr; Match found in `src-tauri/src/main.rs:42` (similarity: 0.94)</div>
                <div className="text-neutral-500">// Step 2: Validate Rust compilation flags</div>
                <div>[0.48s] <span className="text-cyan-400">tool_call:</span> run_command(cargo check --lib)</div>
                <div className="text-neutral-400 pl-4">&rarr; `Finished dev profile in 0.42s`</div>
                <div className="flex items-center gap-2 pt-2 text-amber-400">
                  <AgentWorkingSpinner className="w-3.5 h-3.5" />
                  <span>Synthesizing multi-file refactoring diff...</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Tab: OmniRoute Pipeline */}
      {activeTab === 'omniroute' && (
        <div className="flex-1 flex flex-col p-6 bg-canvas overflow-y-auto space-y-6">
          <div className="flex items-center justify-between border-b border-surface-border pb-3">
            <div>
              <h2 className="text-sm font-bold text-white flex items-center gap-2">
                <Zap className="w-4 h-4 text-emerald-400" />
                Live OmniRoute Cascade
              </h2>
              <p className="text-xs text-neutral-400">Dynamic model failover and cost optimization engine</p>
            </div>
            <span className="text-xs font-mono text-emerald-400 bg-emerald-500/10 px-2.5 py-1 rounded-md border border-emerald-500/20">
              Active Strategy: Smart-Cost (99.98% SLA)
            </span>
          </div>

          <div className="space-y-3">
            <div className="p-4 rounded-xl bg-surface-card border border-blue-500/30 flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="w-2.5 h-2.5 rounded-full bg-blue-400"></div>
                <div>
                  <div className="text-xs font-bold text-white">Tier 1: Claude 3.7 Sonnet (Primary)</div>
                  <div className="text-[11px] text-neutral-400">Complex coding & architectural planning</div>
                </div>
              </div>
              <div className="text-right font-mono text-xs text-emerald-400">42ms avg</div>
            </div>

            <div className="p-4 rounded-xl bg-surface-card border border-emerald-500/30 flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="w-2.5 h-2.5 rounded-full bg-emerald-400"></div>
                <div>
                  <div className="text-xs font-bold text-white">Tier 2: Gemini 2.5 Flash (Fallback / Fast Tool Calling)</div>
                  <div className="text-[11px] text-neutral-400">Realtime search, summarization, and file viewing</div>
                </div>
              </div>
              <div className="text-right font-mono text-xs text-emerald-400">28ms avg</div>
            </div>

            <div className="p-4 rounded-xl bg-surface-card border border-neutral-700 flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="w-2.5 h-2.5 rounded-full bg-neutral-500"></div>
                <div>
                  <div className="text-xs font-bold text-white">Tier 3: Local Ollama / Qwen 2.5 (Offline Emergency Fallback)</div>
                  <div className="text-[11px] text-neutral-400">Zero network required, completely private</div>
                </div>
              </div>
              <div className="text-right font-mono text-xs text-neutral-400">Standby</div>
            </div>
          </div>
        </div>
      )}

      {/* Tab: KV Memory */}
      {activeTab === 'memory' && (
        <div className="flex-1 flex flex-col p-6 bg-canvas overflow-y-auto space-y-4">
          <div className="flex items-center justify-between border-b border-surface-border pb-3">
            <h2 className="text-sm font-bold text-white flex items-center gap-2">
              <Database className="w-4 h-4 text-cyan-400" />
              Agent Context & KV Memory Store
            </h2>
            <span className="text-xs font-mono text-neutral-400">Total Keys: 14 | Cache Hit: 94.2%</span>
          </div>

          <div className="space-y-2">
            <div className="p-3 bg-surface-card border border-surface-border rounded-xl text-xs font-mono">
              <span className="text-cyan-400">session.active_workspace</span> = <span className="text-emerald-400">"engineering"</span>
            </div>
            <div className="p-3 bg-surface-card border border-surface-border rounded-xl text-xs font-mono">
              <span className="text-cyan-400">user.preferred_palette</span> = <span className="text-emerald-400">"custos_dark_blue"</span>
            </div>
            <div className="p-3 bg-surface-card border border-surface-border rounded-xl text-xs font-mono">
              <span className="text-cyan-400">agent.fleet_topology</span> = <span className="text-emerald-400">"hierarchical_consensus"</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default AssistantWorkspace;
