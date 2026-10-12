import React, { useEffect, useState, useMemo } from 'react';
import {
  ShieldCheck,
  BookOpen,
  AlertTriangle,
  MessageSquare,
  Calendar,
  Clock,
  Database,
  Sparkles,
} from 'lucide-react';

export interface SlashCommandDef {
  command: string;
  label: string;
  description: string;
  jobId: string;
  lens: 'coding' | 'research' | 'assistant';
  icon: React.ElementType;
  hasArgs?: boolean;
  argPlaceholder?: string;
  getInputs: (args: string, context: { sessionId?: string; workspaceId?: string }) => Record<string, unknown>;
}

export const SLASH_COMMANDS: SlashCommandDef[] = [
  {
    command: '/review',
    label: 'Code Review',
    description: 'Audit active workspace diff for security, bugs, and invariants',
    jobId: 'engineering__review',
    lens: 'coding',
    icon: ShieldCheck,
    hasArgs: false,
    getInputs: (_args, ctx) => ({
      diff: 'Workspace uncommitted changes',
      base_commit: ctx.workspaceId || 'HEAD',
    }),
  },
  {
    command: '/explain',
    label: 'Repo Explain',
    description: 'Analyze repository architecture, symbols, or a specific path',
    jobId: 'engineering__repo_explain',
    lens: 'coding',
    icon: BookOpen,
    hasArgs: true,
    argPlaceholder: '[path or query]',
    getInputs: (args) => ({
      scope: args.trim() || '.',
      query: args.trim() ? `Explain ${args}` : 'Explain repository architecture and key modules',
    }),
  },
  {
    command: '/diagnose',
    label: 'Diagnose Bug',
    description: 'Investigate runtime errors, trace symptoms, and formulate hypotheses',
    jobId: 'engineering__diagnose',
    lens: 'coding',
    icon: AlertTriangle,
    hasArgs: true,
    argPlaceholder: '<symptoms or error message>',
    getInputs: (args, ctx) => ({
      symptoms: args.trim() || 'Unspecified runtime error',
      environment: ctx.workspaceId ? `Workspace: ${ctx.workspaceId}` : 'Desktop environment',
    }),
  },
  {
    command: '/qa',
    label: 'Personal QA',
    description: 'Ask personal assistant a focused question or query',
    jobId: 'assistant__personal_qa',
    lens: 'assistant',
    icon: MessageSquare,
    hasArgs: true,
    argPlaceholder: '<question>',
    getInputs: (args) => ({
      query: args.trim() || 'Provide summary of active task',
    }),
  },
  {
    command: '/schedule',
    label: 'Schedule Proposal',
    description: 'Draft schedule proposal with attendees and durations',
    jobId: 'assistant__schedule_proposal',
    lens: 'assistant',
    icon: Calendar,
    hasArgs: true,
    argPlaceholder: '<meeting details or agenda>',
    getInputs: (args) => ({
      attendees: ['team'],
      duration_minutes: 30,
      agenda: args.trim() || 'Sync agenda',
    }),
  },
  {
    command: '/remind',
    label: 'Reminder Automation',
    description: 'Automate a scheduled reminder or cron notification',
    jobId: 'assistant__reminder_automation',
    lens: 'assistant',
    icon: Clock,
    hasArgs: true,
    argPlaceholder: '<reminder text>',
    getInputs: (args) => ({
      cron_schedule: '0 9 * * *',
      target_action: args.trim() || 'Daily reminder',
    }),
  },
  {
    command: '/audit',
    label: 'Dataset Audit',
    description: 'Audit dataset distribution and schema integrity',
    jobId: 'research__dataset_audit',
    lens: 'research',
    icon: Database,
    hasArgs: true,
    argPlaceholder: '[dataset path]',
    getInputs: (args) => ({
      dataset_path: args.trim() || 'artifacts/dataset',
      benchmark_split: 'eval',
    }),
  },
  {
    command: '/deep-research',
    label: 'Deep Synthesis',
    description: 'Autonomous research synthesis across scientific corpus',
    jobId: 'research__deep_research',
    lens: 'research',
    icon: Sparkles,
    hasArgs: true,
    argPlaceholder: '<research topic>',
    getInputs: (args) => ({
      topic: args.trim() || 'Research landscape synthesis',
      search_scope: 'all',
    }),
  },
];

export interface SlashCommandMenuProps {
  query: string;
  isOpen: boolean;
  onSelectCommand: (cmd: SlashCommandDef) => void;
  onClose: () => void;
}

export const SlashCommandMenu: React.FC<SlashCommandMenuProps> = ({
  query,
  isOpen,
  onSelectCommand,
  onClose,
}) => {
  const [selectedIndex, setSelectedIndex] = useState(0);

  // Filter commands by the query string after '/'
  const normalizedQuery = query.startsWith('/') ? query.slice(1).toLowerCase().split(' ')[0] : '';

  const filteredCommands = useMemo(() => {
    if (!normalizedQuery) return SLASH_COMMANDS;
    return SLASH_COMMANDS.filter(
      (c) =>
        c.command.toLowerCase().includes(normalizedQuery) ||
        c.label.toLowerCase().includes(normalizedQuery) ||
        c.description.toLowerCase().includes(normalizedQuery)
    );
  }, [normalizedQuery]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [filteredCommands]);

  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex((prev) => (prev + 1) % Math.max(1, filteredCommands.length));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex((prev) => (prev - 1 + filteredCommands.length) % Math.max(1, filteredCommands.length));
      } else if (e.key === 'Enter' || e.key === 'Tab') {
        if (filteredCommands.length > 0) {
          e.preventDefault();
          onSelectCommand(filteredCommands[selectedIndex]);
        }
      } else if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, selectedIndex, filteredCommands, onSelectCommand, onClose]);

  if (!isOpen || filteredCommands.length === 0) return null;

  return (
    <div className="absolute bottom-full left-0 right-0 mb-2 max-h-72 overflow-y-auto rounded-xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)]/95 p-1.5 shadow-2xl backdrop-blur-xl z-50 animate-in fade-in-50 slide-in-from-bottom-2 duration-100">
      <div className="px-2.5 py-1 text-[10px] font-mono uppercase tracking-wider text-[var(--color-fg-subtle)] border-b border-[var(--color-border-subtle)]/60 mb-1 flex items-center justify-between">
        <span>Workbench Slash Commands</span>
        <span>↑↓ Navigate • ↵ Select</span>
      </div>
      <div className="space-y-0.5">
        {filteredCommands.map((cmd, idx) => {
          const Icon = cmd.icon;
          const isSelected = idx === selectedIndex;
          return (
            <button
              key={cmd.command}
              type="button"
              onClick={() => onSelectCommand(cmd)}
              onMouseEnter={() => setSelectedIndex(idx)}
              className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-left text-xs transition ${
                isSelected
                  ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)]'
                  : 'text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-3)]/60 hover:text-[var(--color-editor-fg)]'
              }`}
            >
              <div className={`flex h-6 w-6 shrink-0 items-center justify-center rounded-md border ${
                isSelected
                  ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-400'
                  : 'border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-[var(--color-fg-subtle)]'
              }`}>
                <Icon className="h-3.5 w-3.5" />
              </div>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="font-mono font-semibold text-emerald-400">
                    {cmd.command}
                  </span>
                  {cmd.argPlaceholder && (
                    <span className="font-mono text-[10px] text-[var(--color-fg-subtle)]">
                      {cmd.argPlaceholder}
                    </span>
                  )}
                  <span className="text-[11px] font-medium text-[var(--color-editor-fg)]">
                    {cmd.label}
                  </span>
                  <span className="ml-auto rounded bg-[var(--color-surface-1)] px-1.5 py-0.2 text-[9px] font-mono uppercase text-[var(--color-fg-subtle)]">
                    {cmd.lens}
                  </span>
                </div>
                <div className="truncate text-[11px] text-[var(--color-fg-subtle)] mt-0.5">
                  {cmd.description}
                </div>
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
};

export default SlashCommandMenu;
