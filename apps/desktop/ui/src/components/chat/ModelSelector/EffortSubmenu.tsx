import React from 'react';
import { createPortal } from 'react-dom';
import { Check } from 'lucide-react';
import type { UnifiedModelItem, ThinkingEffort } from './types';

export interface EffortSubmenuProps {
  hoveredEffortItem: {
    model: UnifiedModelItem;
    top: number;
    left: number;
  };
  onSelectEffort: (model: UnifiedModelItem, effort: ThinkingEffort) => void;
  onMouseEnter: () => void;
  onMouseLeave: () => void;
  submenuRef: React.RefObject<HTMLDivElement | null>;
}

export const EffortSubmenu: React.FC<EffortSubmenuProps> = ({
  hoveredEffortItem,
  onSelectEffort,
  onMouseEnter,
  onMouseLeave,
  submenuRef,
}) => {
  return createPortal(
    <div
      ref={submenuRef}
      style={{
        position: 'fixed',
        top: `${hoveredEffortItem.top}px`,
        left: `${hoveredEffortItem.left}px`,
        zIndex: 100,
      }}
      className="w-32 rounded-xl border border-zinc-700/60 bg-[#252528]/98 backdrop-blur-2xl p-1 shadow-2xl animate-in fade-in-50 duration-75 select-none"
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      {/* 12px hover bridge to prevent mouseleave across 6px gap */}
      <div className="absolute -left-3 top-0 w-3 h-full pointer-events-auto" />
      <div className="space-y-0.5 max-h-[60vh] overflow-y-auto">
        {hoveredEffortItem.model.efforts.map((effort) => {
          const isEffortActive = hoveredEffortItem.model.selectedEffort === effort;
          return (
            <button
              key={effort}
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onSelectEffort(hoveredEffortItem.model, effort);
              }}
              className={`w-full text-left px-3 py-1.5 rounded-lg text-xs capitalize transition flex items-center justify-between select-none ${
                isEffortActive
                  ? 'bg-zinc-700/70 text-white font-medium border border-zinc-600/40 shadow-xs'
                  : 'text-zinc-300 hover:bg-zinc-800/80 hover:text-white'
              }`}
            >
              <span>{effort}</span>
              {isEffortActive && (
                <Check size={12} className="text-blue-400 stroke-[2.5]" />
              )}
            </button>
          );
        })}
      </div>
    </div>,
    document.body
  );
};
