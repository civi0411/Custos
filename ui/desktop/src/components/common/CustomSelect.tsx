import React, { useState, useRef, useEffect } from 'react';
import { ChevronDown, Check } from 'lucide-react';

export interface CustomSelectOption {
  value: string | number;
  label: string;
  sublabel?: string;
  icon?: React.ReactNode;
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
  disabled
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

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
    <div className={`relative ${className || ''}`} ref={dropdownRef}>
      <button
        type="button"
        disabled={disabled}
        onClick={() => !disabled && setIsOpen(!isOpen)}
        className={`w-full bg-surface-0 hover:bg-surface-1 border ${
          isOpen ? 'border-border-emphasis ring-1 ring-border-emphasis/20' : 'border-border-default'
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
          <span className="truncate font-medium">
            {selectedOption ? selectedOption.label : placeholder || 'Select an option'}
          </span>
          {selectedOption?.sublabel && (
            <span className="text-[11px] text-fg-subtle font-mono shrink-0">
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
          className={`absolute left-0 right-0 mt-1.5 bg-surface-1 border border-border-default rounded-xl shadow-2xl z-[100] py-1.5 overflow-hidden animate-in fade-in zoom-in-95 duration-100 ${
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

          <div className="max-h-60 overflow-y-auto py-0.5 space-y-0.5 px-1">
            {options.map((opt) => {
              const isSelected = opt.value === value;
              return (
                <button
                  key={opt.value}
                  type="button"
                  onClick={() => {
                    onChange(opt.value);
                    setIsOpen(false);
                  }}
                  className={`w-full px-2.5 py-2 rounded-lg text-xs flex items-center justify-between text-left transition cursor-pointer ${
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
                    <span className="truncate">{opt.label}</span>
                    {opt.sublabel && (
                      <span className="text-[10px] text-fg-subtle font-mono shrink-0">
                        {opt.sublabel}
                      </span>
                    )}
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
