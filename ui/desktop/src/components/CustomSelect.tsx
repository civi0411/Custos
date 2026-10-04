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
    <div className="relative min-w-0 select-none" ref={dropdownRef}>
      <button
        type="button"
        disabled={disabled}
        onClick={(e) => {
          e.stopPropagation();
          if (!disabled) setIsOpen(!isOpen);
        }}
        className={`w-full flex items-center justify-between gap-2 px-3 py-2 rounded-lg text-xs font-medium transition border min-w-0 ${
          isOpen
            ? 'bg-surface-elevated border-surface-border text-white shadow-sm'
            : 'bg-surface-elevated hover:bg-surface-hover text-neutral-200 border-surface-border/60 hover:border-surface-border'
        } ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'} ${className || ''}`}
      >
        <div className="flex items-center gap-2 truncate min-w-0">
          {selectedOption?.icon && (
            <span className="shrink-0 flex items-center">{selectedOption.icon}</span>
          )}
          <span className="truncate">{selectedOption ? selectedOption.label : (placeholder || 'Select...')}</span>
        </div>
        <ChevronDown
          className={`w-3.5 h-3.5 text-neutral-400 shrink-0 transition-transform duration-200 ${
            isOpen ? 'rotate-180 text-brand-blue' : ''
          }`}
        />
      </button>

      {isOpen && (
        <div
          className={`absolute left-0 right-0 mt-1.5 min-w-full w-max max-w-[calc(100vw-2rem)] bg-surface-card/95 backdrop-blur-md border border-surface-border rounded-xl shadow-2xl p-1.5 text-xs z-50 animate-in fade-in zoom-in-95 duration-100 ${
            dropdownClassName || ''
          }`}
        >
          {headerTitle && (
            <div className="px-2.5 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-neutral-400 flex items-center justify-between border-b border-surface-border/40 pb-1.5 mb-1">
              <span>{headerTitle}</span>
              {headerBadge && (
                <span className="text-neutral-500 font-mono bg-surface-elevated px-1.5 py-0.5 rounded text-[10px]">
                  {headerBadge}
                </span>
              )}
            </div>
          )}

          <div className="space-y-0.5 max-h-56 overflow-y-auto">
            {options.map((opt) => {
              const isActive = opt.value === value;
              return (
                <div
                  key={String(opt.value)}
                  onClick={() => {
                    onChange(opt.value);
                    setIsOpen(false);
                  }}
                  className={`flex items-center justify-between px-2.5 py-2 rounded-lg cursor-pointer transition ${
                    isActive
                      ? 'bg-surface-elevated text-white font-medium shadow-sm'
                      : 'hover:bg-surface-hover text-neutral-400 hover:text-white'
                  }`}
                >
                  <div className="flex items-center gap-2 truncate min-w-0">
                    {opt.icon && <span className="shrink-0 flex items-center">{opt.icon}</span>}
                    <div className="truncate">
                      <span className="truncate">{opt.label}</span>
                      {opt.sublabel && (
                        <span className="ml-1.5 text-[10px] text-neutral-500 font-mono">{opt.sublabel}</span>
                      )}
                    </div>
                  </div>
                  {isActive && <Check className="w-3.5 h-3.5 text-brand-blue shrink-0 ml-2" />}
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};

export default CustomSelect;
