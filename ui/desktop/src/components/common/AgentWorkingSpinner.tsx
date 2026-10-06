import React from 'react';
import { cn } from '@/lib/utils';

const SPINNER_ANIMATION_NAME = 'agent-spinner-rotate';

function syncSpinnerPhase(el: HTMLSpanElement | null): void {
  if (el === null || typeof el.getAnimations !== 'function') {
    return;
  }

  const animation = el
    .getAnimations()
    .find(
      (candidate) =>
        'animationName' in candidate && candidate.animationName === SPINNER_ANIMATION_NAME
    );
  if (animation !== undefined) {
    animation.startTime = 0;
  }
}

function handleSpinnerAnimationStart(event: React.AnimationEvent<HTMLSpanElement>): void {
  if (event.animationName === SPINNER_ANIMATION_NAME) {
    syncSpinnerPhase(event.currentTarget);
  }
}

export function AgentWorkingSpinner({ className }: { className?: string }): React.JSX.Element {
  return (
    <span
      onAnimationStart={handleSpinnerAnimationStart}
      data-agent-spinner=""
      className={cn(
        'agent-working-spinner block rounded-full border-2 border-amber-400 border-t-transparent motion-reduce:border-t-amber-400',
        className
      )}
    />
  );
}
