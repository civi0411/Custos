/**
 * Workbench lens presentation strategies for execution telemetry & watchdog alerts.
 */

export interface WorkbenchSupervisionDescriptor {
  id: string;
  name: string;
  accentClass: string;
  badgeClass: string;
  description: string;
  expectedRootAgent: string;
  remedyHints: Record<string, string>;
}

export const WORKBENCH_SUPERVISION_STRATEGIES: Record<string, WorkbenchSupervisionDescriptor> = {
  coding: {
    id: 'coding',
    name: 'Coding',
    accentClass: 'text-sky-400 border-sky-500/40 bg-sky-500/10',
    badgeClass: 'bg-sky-500/15 text-sky-300 border border-sky-500/30',
    description: 'Software engineering: fast loop detection & compiler stall tolerance',
    expectedRootAgent: 'coding-agent',
    remedyHints: {
      repeated_tool: 'Inspect file edit cycle or terminal test runner commands.',
      possible_loop: 'Break oscillation between search and edit by supplying more context.',
      repeated_error: 'Check compilation errors or permission barriers in the worktree.',
      long_running: 'Consider cancelling hung background test suites or build tasks.',
      no_activity: 'Check if the language server or CLI process is blocked on stdin.',
    },
  },
  research: {
    id: 'research',
    name: 'Research',
    accentClass: 'text-violet-400 border-violet-500/40 bg-violet-500/10',
    badgeClass: 'bg-violet-500/15 text-violet-300 border border-violet-500/30',
    description: 'Scientific research: allows extended notebook executions and literature sweeps',
    expectedRootAgent: 'research-agent',
    remedyHints: {
      repeated_tool: 'Check literature API rate limits or notebook cell retry loops.',
      possible_loop: 'Refine citation search queries to avoid cyclical bibliometric crawling.',
      repeated_error: 'Validate dataset paths and Python environment package versions.',
      long_running: 'Large dataset transformation in progress; monitor memory and disk.',
      no_activity: 'Notebook kernel may be waiting on remote GPU cluster or query API.',
    },
  },
  assistant: {
    id: 'assistant',
    name: 'Copilot',
    accentClass: 'text-amber-400 border-amber-500/40 bg-amber-500/10',
    badgeClass: 'bg-amber-500/15 text-amber-300 border border-amber-500/30',
    description: 'Copilot / Assistant: suspends stall alarms during human-in-the-loop approvals',
    expectedRootAgent: 'assistant-agent',
    remedyHints: {
      repeated_tool: 'Verify draft generation parameters or messaging tool arguments.',
      possible_loop: 'Human clarification needed to break agent conversational loop.',
      repeated_error: 'Check integration tokens and outbound credential access.',
      long_running: 'Run is likely waiting for human review in the Outbox.',
      no_activity: 'Idle session awaiting user input in the chat pane.',
    },
  },
  quant: {
    id: 'quant',
    name: 'Quant',
    accentClass: 'text-emerald-400 border-emerald-500/40 bg-emerald-500/10',
    badgeClass: 'bg-emerald-500/15 text-emerald-300 border border-emerald-500/30',
    description: 'Quantitative finance: long backtest tolerance with strict repeated-error alerting',
    expectedRootAgent: 'quant-agent',
    remedyHints: {
      repeated_tool: 'Ensure market data fetcher is not hammering closed exchanges.',
      possible_loop: 'Check portfolio weight optimization convergence parameters.',
      repeated_error: 'Verify ticker symbols and historical pricing data integrity.',
      long_running: 'Backtest simulation in progress across historical bars.',
      no_activity: 'Simulation engine calculating risk matrix.',
    },
  },
};

export function resolveWorkbenchStrategy(id?: string | null): WorkbenchSupervisionDescriptor {
  const norm = (id || 'coding').toLowerCase();
  return (
    WORKBENCH_SUPERVISION_STRATEGIES[norm] || {
      id: norm,
      name: norm.toUpperCase(),
      accentClass: 'text-white/70 border-white/20 bg-white/5',
      badgeClass: 'bg-white/10 text-white/80 border border-white/20',
      description: 'Default execution supervision profile',
      expectedRootAgent: 'agent',
      remedyHints: {},
    }
  );
}

