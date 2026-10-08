import React from 'react';

interface SplitterProps {
  onDragStart: (e: React.MouseEvent) => void;
  onDoubleClick: () => void;
  isDragging?: boolean;
}

export const Splitter: React.FC<SplitterProps> = ({ onDragStart, onDoubleClick, isDragging }) => {
  return (
    <div 
      onMouseDown={onDragStart}
      onDoubleClick={onDoubleClick}
      className={`has-tooltip relative cursor-col-resize transition-all shrink-0 z-20 hover:z-30 hidden md:block select-none group ${
        isDragging 
          ? 'w-1 bg-[var(--workbench-accent)]' 
          : 'w-px hover:w-1 bg-border-muted hover:bg-[var(--workbench-accent)]'
      }`} 
    >
      {/* Invisible wider hit target padding for easy grabbing */}
      <div className="absolute inset-y-0 -left-1.5 -right-1.5 cursor-col-resize pointer-events-auto" />
      {!isDragging && (
        <span
          role="tooltip"
          className="tooltip-label absolute top-4 left-3 px-2 py-1 bg-surface-2 border border-border-default text-fg-editor text-[11px] font-medium rounded-md whitespace-nowrap shadow-xl z-[9999] pointer-events-none"
        >
          Drag to resize · Double-click to reset
        </span>
      )}
    </div>
  );
};
