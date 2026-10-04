import React from 'react';

export type TooltipPosition = 'top' | 'bottom' | 'left' | 'right';
export type TooltipAlign = 'start' | 'center' | 'end';

interface TooltipProps {
  content: string;
  position?: TooltipPosition;
  align?: TooltipAlign;
  children: React.ReactNode;
  className?: string;
}

export const Tooltip: React.FC<TooltipProps> = ({
  content,
  position = 'bottom',
  align = 'center',
  children,
  className = '',
}) => {
  let positionClass = '';

  if (position === 'bottom') {
    if (align === 'start') {
      positionClass = 'top-full left-0 mt-1.5 tooltip-bottom tooltip-align-start';
    } else if (align === 'end') {
      positionClass = 'top-full right-0 mt-1.5 tooltip-bottom tooltip-align-end';
    } else {
      positionClass = 'top-full left-1/2 mt-1.5 tooltip-bottom tooltip-align-center';
    }
  } else if (position === 'top') {
    if (align === 'start') {
      positionClass = 'bottom-full left-0 mb-1.5 tooltip-top tooltip-align-start';
    } else if (align === 'end') {
      positionClass = 'bottom-full right-0 mb-1.5 tooltip-top tooltip-align-end';
    } else {
      positionClass = 'bottom-full left-1/2 mb-1.5 tooltip-top tooltip-align-center';
    }
  } else if (position === 'left') {
    if (align === 'start') {
      positionClass = 'right-full top-0 mr-1.5 tooltip-left tooltip-align-start';
    } else if (align === 'end') {
      positionClass = 'right-full bottom-0 mr-1.5 tooltip-left tooltip-align-end';
    } else {
      positionClass = 'right-full top-1/2 mr-1.5 tooltip-left tooltip-align-center';
    }
  } else if (position === 'right') {
    if (align === 'start') {
      positionClass = 'left-full top-0 ml-1.5 tooltip-right tooltip-align-start';
    } else if (align === 'end') {
      positionClass = 'left-full bottom-0 ml-1.5 tooltip-right tooltip-align-end';
    } else {
      positionClass = 'left-full top-1/2 ml-1.5 tooltip-right tooltip-align-center';
    }
  }

  return (
    <div className={`has-tooltip relative inline-flex items-center justify-center ${className}`}>
      {children}
      <span
        role="tooltip"
        className={`tooltip-label absolute ${positionClass} px-2 py-1 bg-surface-elevated border border-surface-border text-neutral-200 text-[11px] font-medium rounded-md whitespace-nowrap shadow-xl z-[9999] pointer-events-none`}
      >
        {content}
      </span>
    </div>
  );
};

export default Tooltip;
