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
  Wrench,
  FlaskConical,
  Table,
  Inbox,
} from 'lucide-react';

export type WorkspaceLens = 'coding' | 'research' | 'assistant';

export interface SlashCommandDef {
  command: string;
  label: string;
  description: string;
  jobId: string;
  lens: WorkspaceLens;
  icon: React.ElementType;
  hasArgs?: boolean;
  argPlaceholder?: string;
  getInputs: (args: string, context: { sessionId?: string; workspaceId?: string }) => Record<string, unknown>;
}

export const LENS_THEME_CONFIG: Record<
  WorkspaceLens,
  {
    badgeLabel: string;
    workspaceTitle: string;
    textClass: string;
    borderClass: string;
    bgClass: string;
    badgeClass: string;
    activeIconClass: string;
    indicatorClass: string;
  }
> = {
  coding: {
    badgeLabel: 'Coding',
    workspaceTitle: 'Engineering / Lập trình',
    textClass: 'text-sky-400',
    borderClass: 'border-sky-500/30',
    bgClass: 'bg-sky-500/10',
    badgeClass: 'border-sky-500/40 bg-sky-500/10 text-sky-400',
    activeIconClass: 'border-sky-500/50 bg-sky-500/20 text-sky-400 shadow-sky-500/10',
    indicatorClass: 'bg-sky-400',
  },
  research: {
    badgeLabel: 'Research',
    workspaceTitle: 'Research / Nghiên cứu',
    textClass: 'text-purple-400',
    borderClass: 'border-purple-500/30',
    bgClass: 'bg-purple-500/10',
    badgeClass: 'border-purple-500/40 bg-purple-500/10 text-purple-400',
    activeIconClass: 'border-purple-500/50 bg-purple-500/20 text-purple-400 shadow-purple-500/10',
    indicatorClass: 'bg-purple-400',
  },
  assistant: {
    badgeLabel: 'Copilot',
    workspaceTitle: 'Copilot / Trợ lý',
    textClass: 'text-amber-400',
    borderClass: 'border-amber-500/30',
    bgClass: 'bg-amber-500/10',
    badgeClass: 'border-amber-500/40 bg-amber-500/10 text-amber-400',
    activeIconClass: 'border-amber-500/50 bg-amber-500/20 text-amber-400 shadow-amber-500/10',
    indicatorClass: 'bg-amber-400',
  },
};

export const SLASH_COMMANDS: SlashCommandDef[] = [
  // Coding / Engineering Actions
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
    command: '/refactor',
    label: 'Refactor Scope',
    description: 'Improve codebase structure while strictly preserving behavioral parity',
    jobId: 'engineering__refactor',
    lens: 'coding',
    icon: Wrench,
    hasArgs: true,
    argPlaceholder: '<file or module path>',
    getInputs: (args) => ({
      scope: args.trim() || '.',
      invariants: 'Preserve API signatures, contracts, and tests parity',
    }),
  },

  // Research / Scientific Actions
  {
    command: '/deep-research',
    label: 'Deep Synthesis',
    description: 'Autonomous research synthesis across scientific corpus and papers',
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
  {
    command: '/audit',
    label: 'Dataset Audit',
    description: 'Audit dataset distribution, schema integrity, and anomaly split',
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
    command: '/hypothesis',
    label: 'Hypothesis Protocol',
    description: 'Formulate experimental protocol and statistical verification setup',
    jobId: 'research__hypothesis_test',
    lens: 'research',
    icon: FlaskConical,
    hasArgs: true,
    argPlaceholder: '<hypothesis statement>',
    getInputs: (args) => ({
      hypothesis: args.trim() || 'Core hypothesis under investigation',
      metrics: ['accuracy', 'p-value', 'f1'],
    }),
  },
  {
    command: '/claims',
    label: 'Claims Matrix',
    description: 'Extract and verify empirical claims across literature sources',
    jobId: 'research__claims_matrix',
    lens: 'research',
    icon: Table,
    hasArgs: true,
    argPlaceholder: '[domain scope]',
    getInputs: (args) => ({
      scope: args.trim() || 'Active literature corpus',
    }),
  },

  // Assistant / Copilot Actions
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
    command: '/triage',
    label: 'Inbox Triage',
    description: 'Categorize, summarize, and prioritize pending communications',
    jobId: 'assistant__inbox_triage',
    lens: 'assistant',
    icon: Inbox,
    hasArgs: false,
    getInputs: () => ({
      scope: 'pending',
      threshold: 'medium',
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
];

export interface SlashCommandMenuProps {
  query: string;
  isOpen: boolean;
  mode?: 'code' | 'research' | 'chat' | 'engineering' | 'copilot';
  onSelectCommand: (cmd: SlashCommandDef) => void;
  onClose: () => void;
}

export const SlashCommandMenu: React.FC<SlashCommandMenuProps> = ({
  query,
  isOpen,
  mode = 'engineering',
  onSelectCommand,
  onClose,
}) => {
  const [selectedIndex, setSelectedIndex] = useState(0);

  const activeLens: WorkspaceLens =
    mode === 'code' || mode === 'engineering' ? 'coding' : mode === 'research' ? 'research' : 'assistant';
  const currentTheme = LENS_THEME_CONFIG[activeLens];

  // Filter commands by the query string after '/'
  const normalizedQuery = query.startsWith('/') ? query.slice(1).toLowerCase().split(' ')[0] : '';

  const { activeCommands, otherCommands, flatCommands } = useMemo(() => {
    let filtered = SLASH_COMMANDS;
    if (normalizedQuery) {
      filtered = SLASH_COMMANDS.filter(
        (c) =>
          c.command.toLowerCase().includes(normalizedQuery) ||
          c.label.toLowerCase().includes(normalizedQuery) ||
          c.description.toLowerCase().includes(normalizedQuery) ||
          c.lens.toLowerCase().includes(normalizedQuery)
      );
    }

    const active = filtered.filter((c) => c.lens === activeLens);
    const other = filtered.filter((c) => c.lens !== activeLens);
    return {
      activeCommands: active,
      otherCommands: other,
      flatCommands: [...active, ...other],
    };
  }, [normalizedQuery, activeLens]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [flatCommands]);

  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex((prev) => (prev + 1) % Math.max(1, flatCommands.length));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex((prev) => (prev - 1 + flatCommands.length) % Math.max(1, flatCommands.length));
      } else if (e.key === 'Enter' || e.key === 'Tab') {
        if (flatCommands.length > 0) {
          e.preventDefault();
          onSelectCommand(flatCommands[selectedIndex]);
        }
      } else if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, selectedIndex, flatCommands, onSelectCommand, onClose]);

  if (!isOpen || flatCommands.length === 0) return null;

  return (
    <div className="absolute bottom-full left-0 right-0 mb-2 max-h-80 overflow-y-auto rounded-xl border border-[var(--color-border-subtle)] bg-[var(--color-surface-2)]/95 p-1.5 shadow-2xl backdrop-blur-xl z-50 animate-in fade-in-50 slide-in-from-bottom-2 duration-100">
      {/* Top Header with Active Workspace Context */}
      <div className="px-2.5 py-1.5 text-[10px] font-mono uppercase tracking-wider text-[var(--color-fg-subtle)] border-b border-[var(--color-border-subtle)]/60 mb-1.5 flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <span className={`inline-block h-1.5 w-1.5 rounded-full ${currentTheme.indicatorClass}`} />
          <span className="font-semibold text-[var(--color-editor-fg)]">{currentTheme.workspaceTitle}</span>
          <span className="text-[var(--color-fg-subtle)]">· Slash Actions</span>
        </div>
        <span className="text-[9px] text-[var(--color-fg-subtle)]">↑↓ Chọn • ↵ Kích hoạt</span>
      </div>

      <div className="space-y-1">
        {/* Section 1: Active Workspace Recommended Actions */}
        {activeCommands.length > 0 && (
          <div className="space-y-0.5">
            <div className="px-2.5 pt-1 pb-0.5 flex items-center justify-between text-[10px] font-medium text-[var(--color-fg-subtle)]">
              <span className="flex items-center gap-1">
                <span className={currentTheme.textClass}>★</span>
                <span>Khuyến nghị cho Không gian hiện tại</span>
              </span>
              <span className={`rounded px-1.5 py-0.2 text-[9px] font-mono uppercase border ${currentTheme.badgeClass}`}>
                {currentTheme.badgeLabel}
              </span>
            </div>

            {activeCommands.map((cmd) => {
              const flatIndex = flatCommands.indexOf(cmd);
              const isSelected = flatIndex === selectedIndex;
              const Icon = cmd.icon;
              const theme = LENS_THEME_CONFIG[cmd.lens];

              return (
                <button
                  key={cmd.command}
                  type="button"
                  onClick={() => onSelectCommand(cmd)}
                  onMouseEnter={() => setSelectedIndex(flatIndex)}
                  className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-left text-xs transition select-none ${
                    isSelected
                      ? `bg-[var(--color-surface-3)] text-[var(--color-editor-fg)] ring-1 ${theme.borderClass}`
                      : 'text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-3)]/60 hover:text-[var(--color-editor-fg)]'
                  }`}
                >
                  <div
                    className={`flex h-6 w-6 shrink-0 items-center justify-center rounded-md border transition ${
                      isSelected
                        ? theme.activeIconClass
                        : 'border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-[var(--color-fg-subtle)]'
                    }`}
                  >
                    <Icon className="h-3.5 w-3.5" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className={`font-mono font-semibold ${theme.textClass}`}>
                        {cmd.command}
                      </span>
                      {cmd.argPlaceholder && (
                        <span className="font-mono text-[10px] text-[var(--color-fg-subtle)] truncate">
                          {cmd.argPlaceholder}
                        </span>
                      )}
                      <span className="text-[11px] font-medium text-[var(--color-editor-fg)]">
                        {cmd.label}
                      </span>
                      <span
                        className={`ml-auto rounded px-1.5 py-0.2 text-[9px] font-mono uppercase border ${theme.badgeClass}`}
                      >
                        {theme.badgeLabel}
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
        )}

        {/* Section 2: Other Workspace Actions */}
        {otherCommands.length > 0 && (
          <div className="space-y-0.5 pt-1">
            {activeCommands.length > 0 && (
              <div className="px-2.5 pt-1.5 pb-0.5 border-t border-[var(--color-border-subtle)]/40 flex items-center justify-between text-[10px] font-medium text-[var(--color-fg-subtle)]">
                <span>Tác vụ Không gian làm việc khác</span>
                <span className="text-[9px] font-mono text-[var(--color-fg-subtle)] opacity-70">Toàn hệ thống</span>
              </div>
            )}

            {otherCommands.map((cmd) => {
              const flatIndex = flatCommands.indexOf(cmd);
              const isSelected = flatIndex === selectedIndex;
              const Icon = cmd.icon;
              const theme = LENS_THEME_CONFIG[cmd.lens];

              return (
                <button
                  key={cmd.command}
                  type="button"
                  onClick={() => onSelectCommand(cmd)}
                  onMouseEnter={() => setSelectedIndex(flatIndex)}
                  className={`w-full flex items-center gap-2.5 px-3 py-1.5 rounded-lg text-left text-xs transition select-none ${
                    isSelected
                      ? `bg-[var(--color-surface-3)] text-[var(--color-editor-fg)] ring-1 ${theme.borderClass}`
                      : 'text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-3)]/60 hover:text-[var(--color-editor-fg)] opacity-85 hover:opacity-100'
                  }`}
                >
                  <div
                    className={`flex h-6 w-6 shrink-0 items-center justify-center rounded-md border transition ${
                      isSelected
                        ? theme.activeIconClass
                        : 'border-[var(--color-border-subtle)] bg-[var(--color-surface-1)] text-[var(--color-fg-subtle)]'
                    }`}
                  >
                    <Icon className="h-3.5 w-3.5" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className={`font-mono font-medium ${theme.textClass}`}>
                        {cmd.command}
                      </span>
                      {cmd.argPlaceholder && (
                        <span className="font-mono text-[10px] text-[var(--color-fg-subtle)] truncate">
                          {cmd.argPlaceholder}
                        </span>
                      )}
                      <span className="text-[11px] text-[var(--color-editor-fg)]">
                        {cmd.label}
                      </span>
                      <span
                        className={`ml-auto rounded px-1.5 py-0.2 text-[9px] font-mono uppercase border ${theme.badgeClass}`}
                      >
                        {theme.badgeLabel}
                      </span>
                    </div>
                    <div className="truncate text-[10px] text-[var(--color-fg-subtle)] mt-0.5">
                      {cmd.description}
                    </div>
                  </div>
                </button>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};

export default SlashCommandMenu;
