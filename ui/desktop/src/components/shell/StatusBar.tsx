import React, { useState, useRef, useEffect } from 'react';
import { Scaling } from 'lucide-react';
import { Tooltip } from '../Tooltip';

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
    <footer className="h-6 bg-surface border-t border-surface-border px-3 flex items-center justify-between text-[11px] font-mono text-neutral-400 shrink-0 z-30 select-none">
      {/* Active Session Info */}
      <div className="flex items-center gap-3 truncate">
        <span className="flex items-center gap-1.5 text-neutral-300 font-medium">
          <span className="truncate max-w-[150px] sm:max-w-[250px]">{activeSessionTitle}</span>
        </span>
        <span className="text-neutral-700 hidden sm:inline">|</span>
        <span className="hidden sm:inline text-neutral-400">{activeModel}</span>
      </div>

      {/* Metrics & UI Scale Controller */}
      <div className="flex items-center gap-2.5 sm:gap-3 shrink-0">

        {/* UI Scale Popover Trigger */}
        <div className="relative inline-flex items-center" ref={scaleRef}>
          <Tooltip content="Scale: Fonts & Components (Ctrl++ / Ctrl+-)" position="top" align="end">
            <button 
              onClick={(e) => {
                e.stopPropagation();
                setIsScaleMenuOpen(!isScaleMenuOpen);
              }} 
              className={`flex items-center gap-1.5 px-2 py-0.5 rounded-lg font-mono transition text-[11px] border cursor-pointer ${
                isScaleMenuOpen 
                  ? 'bg-surface-elevated border-surface-border text-white shadow-sm' 
                  : 'hover:bg-surface-elevated text-neutral-300 border-surface-border/60 hover:border-surface-border'
              }`}
            >
              <Scaling className={`w-3 h-3 transition shrink-0 ${isScaleMenuOpen ? 'text-brand-blue' : 'text-neutral-400'}`} />
              <span>{uiScale}%</span>
            </button>
          </Tooltip>

          {isScaleMenuOpen && (
            <div className="absolute right-0 bottom-full mb-2 w-72 bg-surface-card/95 backdrop-blur-md border border-surface-border rounded-xl shadow-2xl p-1.5 text-xs z-50 select-none animate-in fade-in zoom-in-95 duration-100">
              {/* Header */}
              <div className="px-2.5 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-neutral-400 flex items-center justify-between border-b border-surface-border/40 pb-1.5 mb-1.5">
                <div className="flex items-center gap-1.5">
                  <Scaling className="w-3.5 h-3.5 text-brand-blue" />
                  <span className="font-semibold text-white">Scale UI & Components</span>
                </div>
                <Tooltip content="Reset to 100% (Ctrl+0)" position="top">
                  <button 
                    onClick={() => onSetUiScale(100)} 
                    className="text-neutral-400 hover:text-white font-mono bg-surface-elevated hover:bg-surface-hover px-1.5 py-0.5 rounded text-[10px] border border-surface-border/60 transition cursor-pointer"
                  >
                    Reset
                  </button>
                </Tooltip>
              </div>

              {/* Stepper & Slider */}
              <div className="bg-surface-elevated/40 border border-surface-border/50 rounded-lg p-2.5 mb-1.5 space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-[11px] text-neutral-400">Current Scale:</span>
                  <span className="font-mono text-xs font-bold text-white bg-surface-card px-2 py-0.5 rounded border border-surface-border">
                    {uiScale}%
                  </span>
                </div>

                <div className="flex items-center gap-2 pt-0.5">
                  <Tooltip content="Zoom Out (Ctrl+-)" position="top">
                    <button 
                      onClick={() => onStepUiScale(-5)} 
                      className="w-6 h-6 rounded-md bg-surface-card hover:bg-surface-hover hover:text-white text-neutral-300 border border-surface-border flex items-center justify-center font-mono text-xs transition cursor-pointer"
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
                    className="flex-1 accent-brand-blue h-1.5 bg-neutral-800 rounded-lg cursor-pointer"
                  />
                  <Tooltip content="Zoom In (Ctrl++)" position="top">
                    <button 
                      onClick={() => onStepUiScale(5)} 
                      className="w-6 h-6 rounded-md bg-surface-card hover:bg-surface-hover hover:text-white text-neutral-300 border border-surface-border flex items-center justify-center font-mono text-xs transition cursor-pointer"
                    >
                      +
                    </button>
                  </Tooltip>
                </div>
              </div>

              {/* Presets Grid */}
              <div className="space-y-1 mb-1.5 px-1">
                <div className="text-[10px] font-semibold uppercase tracking-wider text-neutral-400">Presets</div>
                <div className="grid grid-cols-4 gap-1 font-mono text-[11px]">
                  {presets.map((s) => (
                    <button
                      key={s}
                      onClick={() => onSetUiScale(s)}
                      className={`px-1.5 py-1 rounded-lg border transition text-center cursor-pointer ${
                        uiScale === s
                          ? 'bg-surface-elevated border-brand-blue/60 text-white font-semibold shadow-sm'
                          : 'bg-surface-elevated/30 hover:bg-surface-hover border-surface-border/50 text-neutral-400 hover:text-white'
                      }`}
                    >
                      {s}%
                    </button>
                  ))}
                </div>
              </div>

              {/* Shortcuts Hint */}
              <div className="px-2 pt-1.5 border-t border-surface-border/40 flex items-center justify-between text-[10px] text-neutral-400">
                <span className="flex items-center gap-1">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 shadow-sm shadow-emerald-400/50"></span>
                  <span>Fonts & Components</span>
                </span>
                <span className="text-neutral-500 font-mono">Ctrl++ / Ctrl+-</span>
              </div>
            </div>
          )}
        </div>

        <span className="text-neutral-700">|</span>
        <span className="text-neutral-400">UTF-8</span>
      </div>
    </footer>
  );
};
