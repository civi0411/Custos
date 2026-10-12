import React from 'react';
import { Loader2, Play } from 'lucide-react';
import { useWorkbenchJob } from '@/hooks/useWorkbenchJob';
import { Tooltip } from './Tooltip';
import { RunPacksJobResult } from '@/types/domain';

import type { OrcaTabId } from '@/components/views/OrcaTabbedContainer';

export interface JobActionButtonProps {
  jobId: string;
  label: string;
  icon?: React.ElementType;
  getInputs: () => Record<string, unknown> | null;
  sessionId?: string;
  workspaceId?: string;
  sourceTabId?: OrcaTabId;
  allowParallel?: boolean;
  navigateOnSuccess?: OrcaTabId;
  onNavigate?: (targetTab: OrcaTabId) => void;
  notReadyTooltip?: string;
  readyTooltip?: string;
  onSuccess?: (taskId: string, result: RunPacksJobResult) => void;
  onError?: (error: Error) => void;
  onShowToast?: (msg: string) => void;
  variant?: 'toolbar' | 'card' | 'inline' | 'context-menu';
  size?: 'sm' | 'md';
  className?: string;
  disabled?: boolean;
}

export const JobActionButton: React.FC<JobActionButtonProps> = ({
  jobId,
  label,
  icon: Icon = Play,
  getInputs,
  sessionId,
  workspaceId,
  sourceTabId,
  allowParallel = false,
  navigateOnSuccess,
  onNavigate,
  notReadyTooltip = 'Required context is missing to run this action',
  readyTooltip,
  onSuccess,
  onError,
  onShowToast,
  variant = 'toolbar',
  size = 'sm',
  className = '',
  disabled = false,
}) => {
  const { isReady, isRunning, jobMeta, run } = useWorkbenchJob({
    jobId,
    sessionId,
    workspaceId,
    sourceTabId,
    allowParallel,
    navigateOnSuccess,
    onNavigate,
    getInputs,
    onSuccess,
    onError,
    onShowToast,
  });


  const isPlanned = jobMeta?.status === 'planned';
  const effectiveDisabled = disabled || isRunning || !isReady || isPlanned;

  const handleClick = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (effectiveDisabled) return;
    await run();
  };

  // Variant styling
  const sizeClasses = size === 'sm' ? 'px-2 py-1 text-xs gap-1.5' : 'px-3 py-1.5 text-xs gap-2';

  let variantClasses = '';
  switch (variant) {
    case 'toolbar':
      variantClasses = `inline-flex items-center rounded-md border font-medium transition select-none ${
        effectiveDisabled
          ? 'border-[var(--color-border-subtle)] bg-[var(--color-surface-2)]/60 text-[var(--color-fg-subtle)] opacity-60 cursor-not-allowed'
          : 'border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] hover:border-emerald-500/50 hover:bg-emerald-500/10 hover:text-emerald-400 active:scale-[0.98] cursor-pointer shadow-xs'
      }`;
      break;
    case 'card':
      variantClasses = `inline-flex items-center rounded-lg border font-medium transition select-none ${
        effectiveDisabled
          ? 'border-[var(--color-border-subtle)] bg-[var(--color-surface-2)]/40 text-[var(--color-fg-subtle)] opacity-50 cursor-not-allowed'
          : 'border-[var(--color-border-subtle)] bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] hover:border-[var(--color-border-default)] hover:bg-[var(--color-surface-3)] active:scale-[0.98] cursor-pointer'
      }`;
      break;
    case 'inline':
      variantClasses = `inline-flex items-center rounded font-medium transition select-none ${
        effectiveDisabled
          ? 'text-[var(--color-fg-subtle)] opacity-40 cursor-not-allowed'
          : 'text-[var(--color-fg-muted)] hover:text-emerald-400 hover:bg-emerald-500/10 cursor-pointer'
      }`;
      break;
    case 'context-menu':
      variantClasses = `w-full flex items-center px-3 py-1.5 text-xs text-left transition select-none ${
        effectiveDisabled
          ? 'text-[var(--color-fg-subtle)] opacity-40 cursor-not-allowed'
          : 'text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:text-emerald-400 cursor-pointer'
      }`;
      break;
  }

  const buttonElement = (
    <button
      type="button"
      onClick={handleClick}
      disabled={effectiveDisabled}
      className={`${sizeClasses} ${variantClasses} ${className}`}
    >
      {isRunning ? (
        <Loader2 className="h-3.5 w-3.5 animate-spin text-emerald-400 shrink-0" />
      ) : (
        <Icon className={`h-3.5 w-3.5 shrink-0 ${!effectiveDisabled ? 'text-emerald-400' : ''}`} />
      )}
      <span className="truncate">
        {isRunning ? 'Running...' : label}
      </span>
      {isPlanned && (
        <span className="ml-1 rounded bg-[var(--color-surface-3)] px-1 py-0.2 text-[9px] font-mono uppercase text-[var(--color-fg-subtle)]">
          Soon
        </span>
      )}
    </button>
  );

  // Wrap in Tooltip if needed
  if (!isReady && notReadyTooltip) {
    return (
      <Tooltip content={notReadyTooltip} position="bottom">
        {buttonElement}
      </Tooltip>
    );
  }

  if (readyTooltip || jobMeta?.description) {
    const tooltipText = readyTooltip || jobMeta?.description || label;
    return (
      <Tooltip content={tooltipText} position="bottom">
        {buttonElement}
      </Tooltip>
    );
  }

  return buttonElement;
};

export default JobActionButton;
