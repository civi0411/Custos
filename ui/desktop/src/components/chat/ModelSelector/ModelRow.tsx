import React from 'react';
import { ChevronRight, Info, AlertTriangle } from 'lucide-react';
import type { UnifiedModelItem } from './types';

export interface ModelRowProps {
  model: UnifiedModelItem;
  providerKey: string;
  index: number;
  isSelected: boolean;
  isHovered: boolean;
  loadingNativeModel: string | null;
  onHover: (e: React.MouseEvent<HTMLDivElement>, model: UnifiedModelItem) => void;
  onClick: (model: UnifiedModelItem) => void;
}

export const ModelRow: React.FC<ModelRowProps> = ({
  model: m,
  providerKey,
  index,
  isSelected,
  isHovered,
  loadingNativeModel,
  onHover,
  onClick,
}) => {
  return (
    <div
      key={`${providerKey}-${m.baseId || m.id}-${index}`}
      className="relative"
      onMouseEnter={(e) => onHover(e, m)}
    >
      <button
        type="button"
        onClick={() => onClick(m)}
        disabled={loadingNativeModel !== null}
        className={`w-full text-left px-2.5 py-1.5 rounded-lg flex items-center justify-between transition group select-none ${
          isSelected
            ? 'border border-blue-500 bg-[#252528] text-fg-editor shadow-sm'
            : isHovered
            ? 'bg-surface-2/80 text-fg-editor border border-transparent'
            : 'hover:bg-surface-2/50 text-fg-muted hover:text-fg-editor border border-transparent'
        }`}
      >
        <div className="flex items-center gap-2 min-w-0 flex-1">
          <span className="font-medium text-xs text-fg-editor truncate">
            {m.name}
          </span>

          {/* Current Effort Label */}
          {m.supportsEffort && (
            <span className="text-[11px] text-zinc-400 capitalize font-sans shrink-0">
              {m.selectedEffort}
            </span>
          )}

          {/* In RAM / Custom Badge */}
          {m.badge && (
            <span
              className={`px-1.5 py-0.5 rounded text-[9px] font-sans border shrink-0 ${
                m.badge === 'Loaded'
                  ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-400 font-medium'
                  : 'border-zinc-700/60 bg-zinc-800/80 text-zinc-400'
              }`}
            >
              {m.badge}
            </span>
          )}

          {/* Multi-account Pool Badge */}
          {new Set(m.routes.map((r) => r.accountId)).size > 1 && (
            <span
              className="px-1.5 py-0.5 rounded text-[9px] font-sans border border-purple-500/40 bg-purple-500/15 text-purple-300 font-medium shrink-0 flex items-center gap-1"
              title={`${new Set(m.routes.map((r) => r.accountId)).size} accounts pooled with automatic 429 failover`}
            >
              <span className="w-1.5 h-1.5 rounded-full bg-purple-400 animate-pulse" />
              {new Set(m.routes.map((r) => r.accountId)).size}x Pool
            </span>
          )}

          {/* Leaving Soon Badge */}
          {m.isLeavingSoon && (
            <span className="flex items-center gap-1 px-1.5 py-0.2 rounded-full text-[9px] font-sans border border-zinc-700/60 bg-zinc-800/80 text-zinc-400 shrink-0">
              Leaving Soon
              <Info size={9} />
            </span>
          )}

          {/* Notice Badge */}
          {m.notice && (
            <span className="flex items-center gap-1 px-1.5 py-0.2 rounded-full text-[9px] font-sans border border-amber-500/30 bg-amber-500/10 text-amber-400 shrink-0">
              <AlertTriangle size={9} />
              Notice
              <Info size={9} />
            </span>
          )}
        </div>

        <div className="flex items-center gap-1 shrink-0 ml-1">
          {/* Chevron Right for Effort Submenu */}
          {m.supportsEffort && (
            <ChevronRight
              size={13}
              className={`transition text-zinc-500 group-hover:text-fg-editor ${
                isHovered ? 'translate-x-0.5 text-blue-400' : ''
              }`}
            />
          )}
        </div>
      </button>
    </div>
  );
};
