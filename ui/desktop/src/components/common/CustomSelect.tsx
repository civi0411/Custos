import React, { useState, useRef, useEffect, useId } from 'react';
import ChevronDown from '~icons/lucide/chevron-down';
import Check from '~icons/lucide/check';

export interface CustomSelectOption {
  value: string | number;
  label: string;
  sublabel?: string;
  icon?: React.ReactNode;
  disabled?: boolean;
}

export interface CustomSelectProps {
  value: string | number;
  onChange: (value: any) => void;
  options: CustomSelectOption[];
  placeholder?: string;
  className?: string;
  dropdownClassName?: string;
  headerTitle?: string;
  headerBadge?: string;
  disabled?: boolean;
  ariaLabel?: string;
}

export const CustomSelect: React.FC<CustomSelectProps> = ({
  value,
  onChange,
  options,
  placeholder,
  className,
  dropdownClassName,
  headerTitle,
  headerBadge,
  disabled,
  ariaLabel
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const [opensUp, setOpensUp] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const optionRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const listId = useId();
  const openMenu = () => {
    const root = dropdownRef.current;
    if (!root) return;
    const rect = root.getBoundingClientRect();
    let top = 0;
    let bottom = window.innerHeight;
    for (let parent = root.parentElement; parent; parent = parent.parentElement) {
      if (/(auto|scroll|hidden|clip)/.test(getComputedStyle(parent).overflowY)) {
        const bounds = parent.getBoundingClientRect();
        top = Math.max(top, bounds.top);
        bottom = Math.min(bottom, bounds.bottom);
      }
    }
    const estimatedHeight = Math.min(240, options.length * 52) + (headerTitle || headerBadge ? 36 : 0) + 16;
    setOpensUp(bottom - rect.bottom < estimatedHeight && rect.top - top > bottom - rect.bottom);
    setIsOpen(true);
  };
  useEffect(() => {
    if (isOpen) {
      const selected = options.findIndex(option => option.value === value && !option.disabled);
      optionRefs.current[selected >= 0 ? selected : options.findIndex(option => !option.disabled)]?.focus();
    }
  }, [isOpen]);

  const selectedOption = options.find((opt) => opt.value === value);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setIsOpen(false);
        triggerRef.current?.focus();
      }
    };

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
      document.addEventListener('keydown', handleKeyDown);
    }
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [isOpen]);

  return (
    <div className={`relative ${className || ''}`} ref={dropdownRef} onKeyDown={event => {
      if (isOpen && event.key === 'Escape') {
        event.preventDefault(); event.stopPropagation(); setIsOpen(false); triggerRef.current?.focus();
      }
    }}>
      <button
        ref={triggerRef}
        type="button"
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        aria-controls={isOpen ? listId : undefined}
        onKeyDown={event => {
          if (event.key === 'ArrowDown' || event.key === 'ArrowUp') { event.preventDefault(); openMenu(); }
        }}
        disabled={disabled}
        onClick={() => { if (!disabled) { if (isOpen) setIsOpen(false); else openMenu(); } }}
        className={`app-select-trigger w-full bg-surface-0 hover:bg-surface-1 border ${
          isOpen ? 'border-border-emphasis' : 'border-border-default'
        } rounded-xl px-3.5 py-2.5 text-xs text-left flex items-center justify-between text-fg-editor transition shadow-xs ${
          disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'
        }`}
      >
        <div className="flex items-center gap-2.5 min-w-0">
          {selectedOption?.icon && (
            <div className="shrink-0 flex items-center justify-center">
              {selectedOption.icon}
            </div>
          )}
          <span className="truncate font-medium shrink-0">
            {selectedOption ? selectedOption.label : placeholder || 'Select an option'}
          </span>
          {selectedOption?.sublabel && (
            <span className="text-[11px] text-fg-subtle font-mono min-w-0 truncate">
              {selectedOption.sublabel}
            </span>
          )}
        </div>
        <ChevronDown
          className={`w-3.5 h-3.5 text-fg-muted transition-transform duration-200 shrink-0 ml-2 ${
            isOpen ? 'rotate-180 text-fg-editor' : ''
          }`}
        />
      </button>

      {isOpen && (
        <div
          className={`app-dropdown-menu absolute left-0 right-0 ${opensUp ? 'bottom-full mb-1.5' : 'top-full mt-1.5'} bg-surface-1 border border-border-default rounded-xl shadow-2xl z-[100] py-1.5 overflow-hidden animate-in fade-in zoom-in-95 duration-100 ${
            dropdownClassName || ''
          }`}
        >
          {(headerTitle || headerBadge) && (
            <div className="px-3.5 py-1.5 border-b border-border-muted flex items-center justify-between mb-1">
              {headerTitle && (
                <span className="text-[10px] font-semibold text-fg-subtle uppercase tracking-wider">
                  {headerTitle}
                </span>
              )}
              {headerBadge && (
                <span className="text-[9px] px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-500 font-mono">
                  {headerBadge}
                </span>
              )}
            </div>
          )}

          <div id={listId} role="listbox" aria-label={headerTitle || placeholder || 'Options'} className="max-h-60 overflow-y-auto py-0.5 space-y-0.5 px-1"
            onKeyDown={event => {
              const index = optionRefs.current.indexOf(document.activeElement as HTMLButtonElement);
              let next = index;
              if (event.key === 'ArrowDown') next = Math.min(options.length - 1, index + 1);
              else if (event.key === 'ArrowUp') next = Math.max(0, index - 1);
              else if (event.key === 'Home') next = 0;
              else if (event.key === 'End') next = options.length - 1;
              else if (event.key === 'Tab') { setIsOpen(false); triggerRef.current?.focus(); return; }
              else return;
              const direction = event.key === 'ArrowUp' || event.key === 'End' ? -1 : 1;
              while (next >= 0 && next < options.length && options[next]?.disabled) next += direction;
              event.preventDefault(); optionRefs.current[next]?.focus();
            }}>

            {options.map((opt, index) => {
              const isSelected = opt.value === value;
              return (
                <button
                  key={opt.value}
                  ref={element => { optionRefs.current[index] = element; }}
                  role="option"
                  aria-selected={isSelected}
                  disabled={opt.disabled}
                  tabIndex={-1}
                  type="button"
                  onClick={() => {
                    onChange(opt.value);
                    setIsOpen(false);
                    triggerRef.current?.focus();
                  }}
                  className={`app-dropdown-option disabled:opacity-40 disabled:cursor-not-allowed w-full px-2.5 py-2 rounded-lg text-xs flex items-center justify-between text-left transition cursor-pointer ${
                    isSelected
                      ? 'bg-surface-2 text-fg-editor font-medium'
                      : 'hover:bg-surface-2/60 text-fg-muted hover:text-fg-editor'
                  }`}
                >
                  <div className="flex items-center gap-2.5 min-w-0">
                    {opt.icon && (
                      <div className="shrink-0 flex items-center justify-center">
                        {opt.icon}
                      </div>
                    )}
                    <div className="min-w-0 flex-1">
                      <span className="block truncate font-medium">{opt.label}</span>
                      {opt.sublabel && <span className="block mt-0.5 truncate text-[10px] text-fg-subtle">{opt.sublabel}</span>}
                    </div>
                  </div>
                  {isSelected && <Check className="w-3.5 h-3.5 workbench-accent shrink-0 ml-2" />}
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};

export default CustomSelect;
