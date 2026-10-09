import { ModeConfig, OperationalMode, RiskLevel } from '../types';

export const CUSTOS_VERSION = '0.1.0';

export const MASCOT_ASSETS = {
  mascot: '',
  coder: '',
  inspector: '',
  steward: '',
};

export const OPERATIONAL_MODES: Record<OperationalMode, ModeConfig> = {
  Custos: {
    mode: 'Custos',
    name: 'Custos',
    mascotName: 'Custos',
    mascotImage: '',
    badgeColor: '#e2e8f0',
    tagline: 'General conversation',
    description: 'Neutral chat mode before selecting a specialized operational engine.',
    capabilities: ['Free-form chat', 'Slash command palette', 'Operational mode selection'],
  },
  Code: {
    mode: 'Code',
    name: 'Code',
    mascotName: 'Code Engine',
    mascotImage: '',
    badgeColor: '#38bdf8',
    tagline: 'Software engineering, refactoring, and code analysis',
    description: 'Autonomous programming engine specialized in AST inspection, patch generation, test execution, and safe git worktree mutations.',
    capabilities: [
      'Worktree Isolation & Sandboxing',
      'Unified Diff Synthesis & Verification',
      'Automated Test Suite Runner',
      'Linting & Style Enforcement',
    ],
  },
  Research: {
    mode: 'Research',
    name: 'Research',
    mascotName: 'Research Engine',
    mascotImage: '',
    badgeColor: '#f59e0b',
    tagline: 'Deep investigation, architecture deliberation, and synthesis',
    description: 'Analytical investigator inspecting codebase topologies, security posture, concurrency invariants, and end-to-end slice reasoning.',
    capabilities: [
      'Repository Topology Mapping',
      'Dependency Graph & Security Auditing',
      'End-to-End Vertical Slice Tracing',
      'Architecture Decision Synthesis',
    ],
  },
  Assitant: {
    mode: 'Assitant',
    name: 'Assistant',
    mascotName: 'Assistant Engine',
    mascotImage: '',
    badgeColor: '#10b981',
    tagline: 'Autonomous workflow orchestration and task execution',
    description: 'High-level supervisor coordinating multi-agent lifecycles, human operator permits, task queues, and immutable SQLite audit spans.',
    capabilities: [
      'Human-in-the-Loop Approval Permits',
      'Epoch Optimistic Lock & Replay Defense',
      'Span-Level Audit Logging & SQLite Traces',
      'Background Task Queue Coordination',
    ],
  },
};

export const CUSTOS_PILLARS = [
  {
    icon: '❄',
    title: 'Intelligence Engine',
    desc: 'Specialized operational cognitive tiers & autonomous agents',
    color: '#fbbf24',
  },
  {
    icon: '🛡️',
    title: 'Concurrency Defense',
    desc: 'Epoch Optimistic Locking & Replay Protection against split-brain execution',
    color: '#7dd3fc',
  },
  {
    icon: '🔲',
    title: 'Worktree Isolation',
    desc: 'Seatbelt & Bubblewrap OS Sandbox protecting production workspaces',
    color: '#38bdf8',
  },
  {
    icon: '⚡',
    title: 'Human-in-the-Loop',
    desc: 'Mandatory ExecutionPermit grants before mutating disk files or running shell code',
    color: '#c084fc',
  },
  {
    icon: '📜',
    title: 'Audit Trail',
    desc: 'Immutable SQLite task & span journals recording every agent deliberation',
    color: '#34d399',
  },
];

export const ASCII_BANNER = `
   ██████╗ ██╗   ██╗ ███████╗ ████████╗  ██████╗  ███████╗
  ██╔════╝ ██║   ██║ ██╔════╝ ╚══██╔══╝ ██╔═══██╗ ██╔════╝
  ██║      ██║   ██║ ███████╗    ██║    ██║   ██║ ███████╗
  ██║      ██║   ██║ ╚════██║    ██║    ██║   ██║ ╚════██║
  ╚██████╗ ╚██████╔╝ ███████║    ██║    ╚██████╔╝ ███████║
   ╚═════╝  ╚═════╝  ╚══════╝    ╚═╝     ╚═════╝  ╚══════╝

  ❄ Custos v0.1.1 — Guardian of Agentic Work
  Human-governed runtime for specialized agentic workflows
  (Nhấn '/' để chọn mode: $custos-code, $custos-research, $custos-assistant)
`;

export const SAMPLE_DIFF = {
  filePath: 'crates/runtime/src/handler.rs',
  oldCode: `fn handle_request() {
    todo!();
}`,
  newCode: `pub fn handle_request() -> Result<(), DomainError> {
    tracing::info!("Executing verified task payload in isolated sandbox");
    verify_execution_permit()?;
    commit_audit_span("handler_exec")?;
    Ok(())
}`,
};

export const RISK_BADGES: Record<RiskLevel, { text: string; bg: string; color: string; border: string }> = {
  Low: {
    text: '[LOW RISK]',
    bg: 'rgba(56, 189, 248, 0.15)',
    color: '#38bdf8',
    border: 'rgba(56, 189, 248, 0.4)',
  },
  Medium: {
    text: '[MEDIUM RISK]',
    bg: 'rgba(245, 158, 11, 0.15)',
    color: '#f59e0b',
    border: 'rgba(245, 158, 11, 0.4)',
  },
  High: {
    text: '[HIGH RISK]',
    bg: 'rgba(239, 68, 68, 0.15)',
    color: '#ef4444',
    border: 'rgba(239, 68, 68, 0.4)',
  },
  Critical: {
    text: '[CRITICAL RISK]',
    bg: 'rgba(220, 38, 38, 0.85)',
    color: '#ffffff',
    border: '#ef4444',
  },
};
