import React from 'react';
import { Activity, CircleCheck, CircleDashed } from 'lucide-react';
import { cn } from '@/lib/utils';
import { AgentQuestionIcon } from './AgentQuestionIcon';
import { AgentWorkingSpinner } from './AgentWorkingSpinner';

export type AgentDotState =
  | 'working'
  | 'monitoring'
  | 'blocked'
  | 'waiting'
  | 'interrupted'
  | 'failed'
  | 'done'
  | 'idle'
  | 'unverifiable'
  | 'unconfirmed'
  | 'permission';

export function agentStateLabel(state: AgentDotState): string {
  switch (state) {
    case 'working':
      return 'Working (Autonomous Turn)';
    case 'monitoring':
      return 'Monitoring background tasks';
    case 'blocked':
      return 'Blocked by policy or verifier';
    case 'waiting':
      return 'Waiting for human input';
    case 'interrupted':
      return 'Interrupted';
    case 'failed':
      return 'Failed';
    case 'done':
      return 'Completed';
    case 'idle':
      return 'Idle';
    case 'unverifiable':
      return 'No recent update (Uncertain)';
    case 'unconfirmed':
      return 'Could not confirm';
    case 'permission':
      return 'Supervised: Needs approval';
  }
}

interface AgentStateDotProps {
  state: AgentDotState;
  size?: 'sm' | 'md';
  className?: string;
  title?: string | null;
}

export const AgentStateDot = React.memo(function AgentStateDot({
  state,
  size = 'sm',
  className,
  title,
}: AgentStateDotProps): React.JSX.Element {
  const box = size === 'md' ? 'h-3.5 w-3.5' : 'h-2.5 w-2.5';
  const inner = size === 'md' ? 'size-2.5' : 'size-2';
  const icon = size === 'md' ? 'size-3.5' : 'size-2.5';
  const tooltipLabel = title === null ? null : (title ?? agentStateLabel(state));

  let indicator: React.JSX.Element;

  if (state === 'working') {
    indicator = (
      <span
        className={cn('inline-flex shrink-0 items-center justify-center', box, className)}
        aria-label={agentStateLabel(state)}
      >
        <AgentWorkingSpinner className={inner} />
      </span>
    );
  } else if (state === 'monitoring') {
    indicator = (
      <span
        className={cn('inline-flex shrink-0 items-center justify-center', box, className)}
        aria-label={agentStateLabel(state)}
      >
        <Activity className={cn('text-amber-400', icon)} aria-hidden="true" />
      </span>
    );
  } else if (state === 'done') {
    indicator = (
      <span
        className={cn('inline-flex shrink-0 items-center justify-center', box, className)}
        aria-label={agentStateLabel(state)}
      >
        <CircleCheck className={cn('text-emerald-400', icon)} aria-hidden="true" />
      </span>
    );
  } else if (state === 'unverifiable' || state === 'unconfirmed') {
    indicator = (
      <span
        className={cn('inline-flex shrink-0 items-center justify-center', box, className)}
        aria-label={agentStateLabel(state)}
      >
        <CircleDashed className={cn('text-amber-400 animate-pulse', icon)} aria-hidden="true" />
      </span>
    );
  } else if (state === 'permission' || state === 'waiting') {
    indicator = (
      <span
        className={cn('inline-flex shrink-0 items-center justify-center', box, className)}
        aria-label={agentStateLabel(state)}
      >
        <AgentQuestionIcon className={icon} />
      </span>
    );
  } else {
    indicator = (
      <span
        className={cn('inline-flex shrink-0 items-center justify-center', box, className)}
        aria-label={agentStateLabel(state)}
      >
        <span
          className={cn(
            'block rounded-full shadow-sm',
            inner,
            state === 'blocked' || state === 'failed'
              ? 'bg-rose-500 shadow-rose-500/50'
              : state === 'interrupted'
              ? 'bg-neutral-500'
              : 'bg-neutral-400/40'
          )}
        />
      </span>
    );
  }

  return (
    <div className="relative group inline-flex items-center">
      {indicator}
      {tooltipLabel && (
        <div className="pointer-events-none absolute bottom-full left-1/2 -translate-x-1/2 mb-1 hidden group-hover:block z-50 whitespace-nowrap rounded bg-surface-2 px-2 py-0.5 text-2xs font-medium text-fg-editor shadow-lg border border-border-default">
          {tooltipLabel}
        </div>
      )}
    </div>
  );
});
