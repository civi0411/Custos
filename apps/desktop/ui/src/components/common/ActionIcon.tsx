import React from 'react';
import { Tooltip, TooltipPosition, TooltipAlign } from './Tooltip';

export interface ActionIconProps {
  icon: React.ElementType;
  label?: string;
  tooltip?: string | React.ReactNode;
  shortcut?: string;
  badge?: string | number;
  badgeColor?: string;
  onClick?: (e: React.MouseEvent) => void;
  active?: boolean;
  disabled?: boolean;
  variant?: 'default' | 'danger' | 'accent';
  position?: TooltipPosition;
  align?: TooltipAlign;
  size?: 'xs' | 'sm' | 'md' | 'lg';
  className?: string;
  iconClassName?: string;
  ariaLabel?: string;
  'aria-label'?: string;
}

export const ActionIcon: React.FC<ActionIconProps> = ({
  icon: Icon,
  label,
  tooltip,
  shortcut,
  badge,
  badgeColor = 'bg-workbench-accent',
  onClick,
  active = false,
  disabled = false,
  variant = 'default',
  position = 'bottom',
  align = 'center',
  size = 'sm',
  className = '',
  iconClassName = '',
  ariaLabel,
  'aria-label': ariaLabelProp,
}) => {
  const sizeClasses = {
    xs: 'size-5 p-0.5 rounded text-[10px]',
    sm: 'size-6 p-1 rounded-md text-xs',
    md: 'size-7 p-1.5 rounded-md text-xs',
    lg: 'size-8 p-2 rounded-lg text-sm',
  }[size];

  const iconSizes = {
    xs: 'size-3',
    sm: 'size-3.5',
    md: 'size-4',
    lg: 'size-4.5',
  }[size];

  const content = tooltip !== undefined ? tooltip : label || '';
  const tooltipNode = typeof content === 'string' ? (
    <div className="flex items-center gap-1.5 font-sans pointer-events-none select-none">
      <span className="text-[11px] font-medium text-white/95">{content}</span>
      {shortcut && (
        <kbd className="px-1 py-0.2 rounded bg-white/15 text-[9.5px] font-mono text-white/80 border border-white/10 shadow-xs">
          {shortcut}
        </kbd>
      )}
    </div>
  ) : (
    content
  );

  const variantStyle = {
    default: active
      ? 'bg-white/18 text-white shadow-xs font-semibold border border-white/12'
      : 'text-white/60 hover:text-white hover:bg-white/8 active:bg-white/14 disabled:opacity-35 disabled:cursor-not-allowed border border-transparent',
    danger: active
      ? 'bg-rose-500/20 text-rose-300 border border-rose-500/30 font-semibold'
      : 'text-rose-400/70 hover:text-rose-300 hover:bg-rose-500/10 active:bg-rose-500/20 disabled:opacity-35 disabled:cursor-not-allowed border border-transparent',
    accent: active
      ? 'bg-workbench-accent/20 text-workbench-accent border border-workbench-accent/30 font-semibold'
      : 'text-workbench-accent/80 hover:text-workbench-accent hover:bg-workbench-accent/10 active:bg-workbench-accent/20 disabled:opacity-35 disabled:cursor-not-allowed border border-transparent',
  }[variant] || '';

  const effectiveAriaLabel = ariaLabel || ariaLabelProp || (typeof content === 'string' ? content : typeof label === 'string' ? label : 'action');

  return (
    <Tooltip content={tooltipNode} position={position} align={align}>
      <button
        type="button"
        onClick={onClick}
        disabled={disabled}
        aria-label={effectiveAriaLabel}
        className={`relative inline-flex items-center justify-center transition cursor-pointer select-none ${sizeClasses} ${variantStyle} ${className}`}
      >
        <Icon className={`${iconSizes} shrink-0 ${iconClassName}`} />
        {badge !== undefined && badge !== null && (
          <span
            className={`absolute -top-1 -right-1 min-w-3.5 h-3.5 px-0.5 rounded-full text-[8.5px] font-mono font-bold text-white flex items-center justify-center shadow-xs border border-black/40 ${badgeColor}`}
          >
            {badge}
          </span>
        )}
      </button>
    </Tooltip>
  );
};

export default ActionIcon;
