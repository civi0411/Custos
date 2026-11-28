import React, { useEffect, useState } from 'react';
import { Loader2, ShieldAlert } from 'lucide-react';

/**
 * Standardized UI latency feedback thresholds (in ms):
 * - 0–100ms: instant/idle (no UI flicker)
 * - 100ms+: disable interactive triggers immediately
 * - 200ms+: show spinner (avoids flash locally while staying responsive over SSH)
 * - 3000ms+: surface explicit step label
 */
export const UiFeedbackTimingPolicy = {
  instantMaxMs: 100,
  spinnerDelayMs: 200,
  stepLabelThresholdMs: 3000,
} as const;

export type FeedbackPhase = 'idle' | 'disabled' | 'spinner' | 'step';

export interface FeedbackTimingState {
  phase: FeedbackPhase;
  disabled: boolean;
  showSpinner: boolean;
  showStepLabel: boolean;
  label?: string;
}

export function useFeedbackTiming(pending: boolean, stepLabel?: string): FeedbackTimingState {
  const [elapsedMs, setElapsedMs] = useState(0);

  useEffect(() => {
    if (!pending) {
      setElapsedMs(0);
      return;
    }
    const start = Date.now();
    const t1 = window.setTimeout(() => setElapsedMs(Date.now() - start), UiFeedbackTimingPolicy.instantMaxMs);
    const t2 = window.setTimeout(() => setElapsedMs(Date.now() - start), UiFeedbackTimingPolicy.spinnerDelayMs);
    const t3 = window.setTimeout(() => setElapsedMs(Date.now() - start), UiFeedbackTimingPolicy.stepLabelThresholdMs);
    return () => {
      window.clearTimeout(t1);
      window.clearTimeout(t2);
      window.clearTimeout(t3);
    };
  }, [pending]);

  if (!pending || elapsedMs < UiFeedbackTimingPolicy.instantMaxMs) {
    return { phase: 'idle', disabled: false, showSpinner: false, showStepLabel: false };
  }
  if (elapsedMs < UiFeedbackTimingPolicy.spinnerDelayMs) {
    return { phase: 'disabled', disabled: true, showSpinner: false, showStepLabel: false };
  }
  if (elapsedMs < UiFeedbackTimingPolicy.stepLabelThresholdMs) {
    return { phase: 'spinner', disabled: true, showSpinner: true, showStepLabel: false };
  }
  return { phase: 'step', disabled: true, showSpinner: true, showStepLabel: true, label: stepLabel };
}

export interface SecuritySurfaceProps {
  title: string;
  subtitle?: string;
  badge?: string;
  pending?: boolean;
  pendingLabel?: string;
  children: React.ReactNode;
}

/**
 * Trust-boundary container for Permit, Outbox, and MCP approval surfaces.
 * Styles are locked inline so external themes or plugins cannot spoof them.
 */
export const SecuritySurface: React.FC<SecuritySurfaceProps> = ({
  title,
  subtitle,
  badge = 'CUSTOS TRUST BOUNDARY',
  pending = false,
  pendingLabel = 'Waiting for kernel confirmation…',
  children,
}) => {
  const timing = useFeedbackTiming(pending, pendingLabel);

  return (
    <section
      data-security-surface="locked"
      aria-label={title}
      style={{
        backgroundColor: '#121316',
        border: '1px solid rgba(255, 255, 255, 0.16)',
        color: '#f4f4f5',
        borderRadius: '8px',
        boxShadow: '0 8px 24px rgba(0, 0, 0, 0.45)',
      }}
      className="overflow-hidden"
    >
      <header
        style={{
          backgroundColor: '#18191e',
          borderBottom: '1px solid rgba(255, 255, 255, 0.10)',
          padding: '6px 12px',
        }}
        className="flex items-center justify-between gap-2 text-[11px]"
      >
        <div className="flex items-center gap-2 min-w-0">
          <ShieldAlert className="size-3.5 shrink-0 text-amber-300" />
          <span className="font-mono text-[10px] uppercase tracking-wider font-semibold text-amber-200/90 shrink-0">
            {badge}
          </span>
          <span className="font-semibold text-white truncate">{title}</span>
          {subtitle && <span className="text-white/60 truncate">{subtitle}</span>}
        </div>
        {timing.showSpinner && (
          <span className="flex items-center gap-1.5 text-[10px] font-mono text-white/75 shrink-0">
            <Loader2 className="size-3 animate-spin" />
            {timing.showStepLabel && timing.label ? <span>{timing.label}</span> : null}
          </span>
        )}
      </header>
      <div className="p-2.5">{children}</div>
    </section>
  );
};

export interface GhostDismissButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  label?: string;
}

/**
 * Quiet ghost-styled dismiss/cancel button for security & review surfaces.
 * Never styled as destructive red so cancel actions stay visually calm.
 */
export const GhostDismissButton: React.FC<GhostDismissButtonProps> = ({
  label = 'Cancel',
  className = '',
  children,
  ...rest
}) => (
  <button
    type="button"
    {...rest}
    className={`px-2.5 py-1 rounded text-[11px] font-medium text-white/70 hover:text-white hover:bg-white/8 transition cursor-pointer disabled:opacity-40 ${className}`}
  >
    {children ?? label}
  </button>
);

