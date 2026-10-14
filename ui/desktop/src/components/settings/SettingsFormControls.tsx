import React from 'react';

export function SettingsSubsectionHeader(props: {
  title: React.ReactNode;
  description?: string;
  action?: React.ReactNode;
}): React.ReactElement {
  return (
    <div className="flex items-start justify-between gap-4 pb-2 border-b border-border-muted/60 mb-3">
      <div>
        <h3 className="text-xs font-semibold text-fg-editor uppercase tracking-wider">{props.title}</h3>
        {props.description && (
          <p className="mt-0.5 text-[11px] text-fg-muted leading-relaxed">{props.description}</p>
        )}
      </div>
      {props.action && <div className="shrink-0">{props.action}</div>}
    </div>
  );
}

export function SettingsSwitchRow(props: {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}): React.ReactElement {
  return (
    <div className="flex items-center justify-between gap-4 py-2.5 px-3 rounded-xl bg-surface-1 border border-border-default/60 hover:border-border-default transition">
      <div className="min-w-0 flex-1">
        <div className="text-xs font-medium text-fg-editor">{props.label}</div>
        {props.description && (
          <div className="text-[11px] text-fg-muted leading-relaxed mt-0.5">{props.description}</div>
        )}
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={props.checked}
        disabled={props.disabled}
        onClick={() => props.onChange(!props.checked)}
        className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none ${
          props.checked ? 'bg-emerald-500' : 'bg-surface-3'
        } ${props.disabled ? 'opacity-40 cursor-not-allowed' : ''}`}
      >
        <span
          className={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-sm ring-0 transition duration-200 ease-in-out ${
            props.checked ? 'translate-x-4' : 'translate-x-0'
          }`}
        />
      </button>
    </div>
  );
}

export function SettingsSegmentedControl<T extends string>(props: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
  ariaLabel: string;
}): React.ReactElement {
  return (
    <div
      role="group"
      aria-label={props.ariaLabel}
      className="inline-flex items-center p-0.5 rounded-lg bg-surface-2 border border-border-default/80"
    >
      {props.options.map((opt) => {
        const isSelected = opt.value === props.value;
        return (
          <button
            key={opt.value}
            type="button"
            onClick={() => props.onChange(opt.value)}
            className={`px-3 py-1 text-xs rounded-md font-medium transition ${
              isSelected
                ? 'bg-surface-1 text-fg-editor shadow-xs border border-border-default'
                : 'text-fg-subtle hover:text-fg-editor border border-transparent'
            }`}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}

export function SettingsNumberField(props: {
  label: string;
  description?: string;
  value: number;
  min?: number;
  max?: number;
  step?: number;
  unit?: string;
  onChange: (value: number) => void;
}): React.ReactElement {
  return (
    <div className="flex items-center justify-between gap-4 py-2.5 px-3 rounded-xl bg-surface-1 border border-border-default/60 hover:border-border-default transition">
      <div className="min-w-0 flex-1">
        <div className="text-xs font-medium text-fg-editor">{props.label}</div>
        {props.description && (
          <div className="text-[11px] text-fg-muted leading-relaxed mt-0.5">{props.description}</div>
        )}
      </div>
      <div className="flex items-center gap-1.5 shrink-0">
        <input
          type="number"
          min={props.min}
          max={props.max}
          step={props.step || 1}
          value={props.value}
          onChange={(e) => {
            const num = parseFloat(e.target.value);
            if (!isNaN(num)) props.onChange(num);
          }}
          className="w-24 px-2.5 py-1 text-xs text-right bg-surface-2 border border-border-default rounded-lg text-fg-editor outline-none font-mono focus:border-emerald-500 transition"
        />
        {props.unit && <span className="text-[11px] text-fg-subtle font-mono">{props.unit}</span>}
      </div>
    </div>
  );
}
