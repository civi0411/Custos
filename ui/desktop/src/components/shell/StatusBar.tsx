import React, { useState, useRef, useEffect } from 'react';
import { Scaling } from 'lucide-react';
import { Tooltip } from '../Tooltip';
import { formatKeyCombo } from '@/lib/utils';

interface StatusBarProps {
  activeSessionTitle: string;
  activeModel: string;
  uiScale: number;
  onSetUiScale: (val: number) => void;
  onStepUiScale: (delta: number) => void;
}

export const StatusBar: React.FC<StatusBarProps> = ({
  activeSessionTitle,
  activeModel,
  uiScale,
  onSetUiScale,
  onStepUiScale
}) => {
  const [isScaleMenuOpen, setIsScaleMenuOpen] = useState(false);
  const scaleRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (scaleRef.current && !scaleRef.current.contains(e.target as Node)) {
        setIsScaleMenuOpen(false);
      }
    };
    document.addEventListener('click', handleClickOutside);
    return () => document.removeEventListener('click', handleClickOutside);
  }, []);

  const presets = [85, 90, 100, 110, 120, 130, 140, 150];

  return (
    <footer className="h-6 bg-surface-1 border-t border-border-muted px-3 flex items-center justify-between text-[11px] font-mono text-fg-muted shrink-0 z-30 select-none">
      {/* Active Session Info */}
      <div className="flex items-center gap-3 truncate">
        <span className="flex items-center gap-1.5 text-fg-editor font-medium">
          <span className="truncate max-w-[150px] sm:max-w-[250px]">{activeSessionTitle}</span>
        </span>
        <span className="text-border-default hidden sm:inline">|</span>
        <span className="hidden sm:inline text-fg-subtle">{activeModel}</span>
      </div>

      {/* Metrics & UI Scale Controller */}
      <div className="flex items-center gap-2.5 sm:gap-3 shrink-0">

        {/* UI Scale Popover Trigger */}
        <div className="relative inline-flex items-center" ref={scaleRef}>
          <Tooltip content={`Scale: Fonts & Components (${formatKeyCombo({ ctrlOrCmd: true, key: '+' })} / ${formatKeyCombo({ ctrlOrCmd: true, key: '-' })})`} position="top" align="end">
            <button 
              onClick={(e) => {
                e.stopPropagation();
                setIsScaleMenuOpen(!isScaleMenuOpen);
              }} 
              className={`flex items-center gap-1.5 px-2 py-0.5 rounded font-mono transition text-[11px] border cursor-pointer ${
                isScaleMenuOpen 
                  ? 'bg-surface-2 border-border-default text-fg-editor shadow-xs' 
                  : 'hover:bg-surface-2 text-fg-muted hover:text-fg-editor border-transparent hover:border-border-default'
              }`}
            >
              <Scaling className={`w-3 h-3 transition shrink-0 ${isScaleMenuOpen ? 'workbench-accent' : 'text-fg-subtle'}`} />
              <span>{uiScale}%</span>
            </button>
          </Tooltip>

          {isScaleMenuOpen && (
            <div className="absolute right-0 bottom-full mb-2 w-72 bg-surface-1 border border-border-default rounded-xl shadow-2xl p-1.5 text-xs z-50 select-none animate-in fade-in zoom-in-95 duration-100">
              {/* Header */}
              <div className="px-2.5 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-fg-subtle flex items-center justify-between border-b border-border-muted pb-1.5 mb-1.5">
                <div className="flex items-center gap-1.5">
                  <Scaling className="w-3.5 h-3.5 workbench-accent" />
                  <span className="font-semibold text-fg-editor">Scale UI & Components</span>
                </div>
                <Tooltip content={`Reset to 100% (${formatKeyCombo({ ctrlOrCmd: true, key: '0' })})`} position="top">
                  <button 
                    onClick={() => onSetUiScale(100)} 
                    className="text-fg-muted hover:text-fg-editor font-mono bg-surface-2 hover:bg-surface-3 px-1.5 py-0.5 rounded text-[10px] border border-border-default transition cursor-pointer"
                  >
                    Reset
                  </button>
                </Tooltip>
              </div>

              {/* Stepper & Slider */}
              <div className="bg-surface-0 border border-border-default rounded-lg p-2.5 mb-1.5 space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-[11px] text-fg-muted">Current Scale:</span>
                  <span className="font-mono text-xs font-bold text-fg-editor bg-surface-1 px-2 py-0.5 rounded border border-border-default">
                    {uiScale}%
                  </span>
                </div>

                <div className="flex items-center gap-2 pt-0.5">
                  <Tooltip content="Zoom Out (Ctrl+-)" position="top">
                    <button 
                      onClick={() => onStepUiScale(-5)} 
                      className="w-6 h-6 rounded-md bg-surface-1 hover:bg-surface-2 text-fg-editor border border-border-default flex items-center justify-center font-mono text-xs transition cursor-pointer"
                    >
                      -
                    </button>
                  </Tooltip>
                  <input 
                    type="range" 
                    min="75" 
                    max="150" 
                    step="5" 
                    value={uiScale} 
                    onChange={(e) => onSetUiScale(Number(e.target.value))}
                    className="flex-1 accent-[var(--workbench-accent)] h-1.5 bg-surface-2 rounded-lg cursor-pointer"
                  />
                  <Tooltip content="Zoom In (Ctrl++)" position="top">
                    <button 
                      onClick={() => onStepUiScale(5)} 
                      className="w-6 h-6 rounded-md bg-surface-1 hover:bg-surface-2 text-fg-editor border border-border-default flex items-center justify-center font-mono text-xs transition cursor-pointer"
                    >
                      +
                    </button>
                  </Tooltip>
                </div>
              </div>

              {/* Presets Grid */}
              <div className="space-y-1 mb-1.5 px-1">
                <div className="text-[10px] font-semibold uppercase tracking-wider text-fg-subtle">Presets</div>
                <div className="grid grid-cols-4 gap-1 font-mono text-[11px]">
                  {presets.map((s) => (
                    <button
                      key={s}
                      onClick={() => onSetUiScale(s)}
                      className={`px-1.5 py-1 rounded-lg border transition text-center cursor-pointer ${
                        uiScale === s
                          ? 'bg-surface-2 border-border-emphasis text-fg-editor font-semibold shadow-xs'
                          : 'bg-transparent hover:bg-surface-2 border-border-muted text-fg-muted hover:text-fg-editor'
                      }`}
                    >
                      {s}%
                    </button>
                  ))}
                </div>
              </div>

              {/* Shortcuts Hint */}
              <div className="px-2 pt-1.5 border-t border-border-muted flex items-center justify-between text-[10px] text-fg-subtle">
                <span className="flex items-center gap-1">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
                  <span>Fonts & Components</span>
                </span>
                <span className="font-mono">{formatKeyCombo({ ctrlOrCmd: true, key: '+' })} / {formatKeyCombo({ ctrlOrCmd: true, key: '-' })}</span>
              </div>
            </div>
          )}
        </div>

        <span className="text-border-default">|</span>
        <span className="text-fg-subtle">UTF-8</span>
      </div>
    </footer>
  );
};
