import React from 'react';

interface SplitterProps {
  onDragStart: (e: React.MouseEvent) => void;
  onDoubleClick: () => void;
}

export const Splitter: React.FC<SplitterProps> = ({ onDragStart, onDoubleClick }) => {
  return (
    <div 
      onMouseDown={onDragStart}
      onDoubleClick={onDoubleClick}
      className="w-1 hover:w-1.5 bg-surface-border hover:bg-brand-blue cursor-col-resize transition-all shrink-0 z-10 hidden md:block select-none" 
      title="Drag to resize panels (Double click to reset 50/50)"
    />
  );
};
