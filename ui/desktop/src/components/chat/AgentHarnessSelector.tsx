import React, { useState, useEffect, useRef } from 'react';
import {
  Bot,
  Terminal,
  Cpu,
  ChevronDown,
  Check,
  Zap,
  ShieldCheck,
  FlaskConical,
  BookOpen,
  FileSearch,
  Sparkles,
  Calendar,
  Send,
  UserCheck,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { HarnessDescriptor } from '@/types/domain';
import type { ActiveWorkbench } from '@/components/shell/AppHeader';

export interface AgentHarnessSelectorProps {
  mode?: ActiveWorkbench;
  currentHarnessId?: string;
  onSelectHarness: (harnessId: string) => void;
  className?: string;
}

export interface AgentOption {
  id: string;
  label: string;
  vendor: string;
  description: string;
  icon: React.ElementType;
  iconColor: string;
}

export const ENGINEERING_HARNESSES: AgentOption[] = [
  {
    id: 'claude-code',
    label: 'Claude Code CLI',
    vendor: 'Anthropic',
    description: 'Anthropic native sub-process coding harness with direct bash/edit integration.',
    icon: Terminal,
    iconColor: 'text-amber-400',
  },
  {
    id: 'codex',
    label: 'OpenAI Codex',
    vendor: 'OpenAI',
    description: 'Codex autonomous developer loop with multi-step file manipulation.',
    icon: Cpu,
    iconColor: 'text-emerald-400',
  },
  {
    id: 'goose',
    label: 'Block Goose',
    vendor: 'Block',
    description: 'Autonomous developer CLI harness with extensible tool plugins.',
    icon: Zap,
    iconColor: 'text-cyan-400',
  },
  {
    id: 'opencode',
    label: 'OpenCode Agent',
    vendor: 'OpenCode',
    description: 'Provider-governed native CLI agent loop.',
    icon: Bot,
    iconColor: 'text-blue-400',
  },
  {
    id: 'governed-agent-runtime',
    label: 'Custos Governed Runtime',
    vendor: 'Custos',
    description: 'Supervised local agent runtime with strict Permit boundaries and proof ledger.',
    icon: ShieldCheck,
    iconColor: 'text-purple-400',
  },
];

export const RESEARCH_AGENTS: AgentOption[] = [
  {
    id: 'literature-agent',
    label: 'Literature Discovery',
    vendor: 'OpenAlex/EuropePMC',
    description: 'Autonomous paper search, citation indexing, and full-text retrieval agent.',
    icon: BookOpen,
    iconColor: 'text-sky-400',
  },
  {
    id: 'science-copilot',
    label: 'Science Multi-DB',
    vendor: 'ChEMBL/PDB/AlphaFold',
    description: 'Deep queries across biomedical databases, molecular structures, and genomics.',
    icon: FlaskConical,
    iconColor: 'text-emerald-400',
  },
  {
    id: 'synthesis-engine',
    label: 'Claim & Evidence Synthesizer',
    vendor: 'Custos Lab',
    description: 'Extracts supported claims, correlates findings, and produces structured proofs.',
    icon: FileSearch,
    iconColor: 'text-purple-400',
  },
  {
    id: 'research-handoff',
    label: 'Engineering Handoff',
    vendor: 'Bridge',
    description: 'Converts validated research conclusions into engineering tasks & code requirements.',
    icon: Sparkles,
    iconColor: 'text-amber-400',
  },
];

export const COPILOT_AGENTS: AgentOption[] = [
  {
    id: 'executive-assistant',
    label: 'Executive Assistant',
    vendor: 'Custos Copilot',
    description: 'High-level orchestration, prioritization, task decomposition, and meeting notes.',
    icon: Bot,
    iconColor: 'text-blue-400',
  },
  {
    id: 'outbox-supervisor',
    label: 'Outbox Supervisor',
    vendor: 'Governance',
    description: 'Guards external actions, drafts approvals, verifies permit scopes.',
    icon: Send,
    iconColor: 'text-amber-400',
  },
  {
    id: 'agenda-coordinator',
    label: 'Agenda Coordinator',
    vendor: 'Personal',
    description: 'Schedules contacts, parses calendar conflicts, aligns daily goals.',
    icon: Calendar,
    iconColor: 'text-emerald-400',
  },
  {
    id: 'policy-auditor',
    label: 'Policy Auditor',
    vendor: 'Custos Security',
    description: 'Verifies receipts, enforces budget constraints and token quotas.',
    icon: UserCheck,
    iconColor: 'text-purple-400',
  },
];

export const AgentHarnessSelector: React.FC<AgentHarnessSelectorProps> = ({
  mode = 'engineering',
  currentHarnessId,
  onSelectHarness,
  className = '',
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const [harnesses, setHarnesses] = useState<HarnessDescriptor[]>([]);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const catalog =
    mode === 'research'
      ? RESEARCH_AGENTS
      : mode === 'copilot'
      ? COPILOT_AGENTS
      : ENGINEERING_HARNESSES;

  const defaultId = catalog[0]?.id || 'claude-code';
  const effectiveId = currentHarnessId || defaultId;

  useEffect(() => {
    let mounted = true;
    if (mode === 'engineering') {
      daemonClient
        .listHarnesses()
        .then((list) => {
          if (mounted) setHarnesses(list);
        })
        .catch(() => {});
    }
    return () => {
      mounted = false;
    };
  }, [mode]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };
    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isOpen]);

  const activeDef =
    catalog.find((h) => h.id === effectiveId) || {
      id: effectiveId,
      label: effectiveId,
      vendor: 'Agent',
      description: 'Active agent persona',
      icon: Bot,
      iconColor: 'text-fg-editor',
    };

  const Icon = activeDef.icon;
  const sectionTitle =
    mode === 'research'
      ? 'Specialized Research Agent'
      : mode === 'copilot'
      ? 'Executive Copilot Agent'
      : 'Native Coding Harness';

  return (
    <div ref={dropdownRef} className={`relative inline-block text-left ${className}`}>
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-surface-1 hover:bg-surface-2 border border-border-default text-[11px] text-fg-editor transition cursor-pointer shadow-2xs"
        title={`${sectionTitle}: ${activeDef.label} (${activeDef.vendor})`}
      >
        <Icon className={`w-3.5 h-3.5 ${activeDef.iconColor} shrink-0`} />
        <span className="font-medium truncate max-w-[120px]">{activeDef.label}</span>
        <ChevronDown className="w-3 h-3 text-fg-muted shrink-0" />
      </button>

      {isOpen && (
        <div
          role="dialog"
          aria-label={sectionTitle}
          className="absolute left-0 bottom-full mb-1 w-76 rounded-lg border border-border-default bg-surface-1 p-1.5 shadow-xl z-50 text-xs backdrop-blur-md"
        >
          <div className="px-2 py-1 text-[10px] uppercase font-semibold text-fg-muted tracking-wider border-b border-border-muted/50 mb-1">
            {sectionTitle}
          </div>
          <div className="space-y-0.5 max-h-64 overflow-y-auto">
            {catalog.map((agent) => {
              const ItemIcon = agent.icon;
              const isSelected = agent.id === effectiveId;
              const backendMatch = harnesses.find((h) => h.id === agent.id);
              const isAvailable = backendMatch ? backendMatch.is_available : true;

              return (
                <button
                  key={agent.id}
                  type="button"
                  onClick={() => {
                    onSelectHarness(agent.id);
                    setIsOpen(false);
                  }}
                  className={`flex w-full items-start gap-2.5 rounded-md px-2 py-2 text-left transition cursor-pointer ${
                    isSelected
                      ? 'bg-surface-2 font-medium text-fg-editor'
                      : 'hover:bg-surface-2/60 text-fg-muted hover:text-fg-editor'
                  }`}
                >
                  <ItemIcon className={`w-4 h-4 ${agent.iconColor} shrink-0 mt-0.5`} />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-1.5">
                      <span className="text-[12px] font-medium text-fg-editor truncate">{agent.label}</span>
                      <span className="text-[9px] px-1 py-0.2 rounded bg-surface-3 text-fg-muted font-mono flex items-center gap-1">
                        <span className={`w-1.5 h-1.5 rounded-full ${isAvailable ? 'bg-emerald-400' : 'bg-fg-muted'}`} />
                        {agent.vendor}
                      </span>
                    </div>
                    <p className="text-[10px] text-fg-subtle line-clamp-2 mt-0.5 leading-snug">{agent.description}</p>
                  </div>
                  {isSelected && <Check className="w-3.5 h-3.5 text-emerald-400 shrink-0 mt-0.5" />}
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};
