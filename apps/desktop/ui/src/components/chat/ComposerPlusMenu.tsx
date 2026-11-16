import React, { useEffect, useRef } from 'react';
import {
  Paperclip,
  Sparkles,
  Blocks,
  Plug,
  ChevronRight,
} from 'lucide-react';

export interface ComposerPlusMenuProps {
  isOpen: boolean;
  onClose: () => void;
  onAddFilesOrPhotos: () => void;
  onOpenSkills?: () => void;
  onAddConnector?: () => void;
  onAddPlugins?: () => void;
  position?: 'top' | 'bottom';
}

export const ComposerPlusMenu: React.FC<ComposerPlusMenuProps> = ({
  isOpen,
  onClose,
  onAddFilesOrPhotos,
  onOpenSkills,
  onAddConnector,
  onAddPlugins,
  position = 'top',
}) => {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isOpen) return;

    const handlePointerDown = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    window.addEventListener('mousedown', handlePointerDown);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('mousedown', handlePointerDown);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div
      ref={menuRef}
      className={`absolute ${
        position === 'top' ? 'bottom-full mb-2.5' : 'top-full mt-2.5'
      } left-0 w-64 rounded-2xl bg-[#1e1e22] border border-white/12 shadow-2xl p-1.5 text-xs text-white z-50 select-none animate-in fade-in zoom-in-95 duration-150 backdrop-blur-md`}
      role="menu"
      aria-label="Add options menu"
    >
      {/* 1. Add files or photos */}
      <button
        type="button"
        onClick={() => {
          onAddFilesOrPhotos();
          onClose();
        }}
        className="w-full flex items-center justify-between px-3 py-2.5 rounded-xl hover:bg-white/8 transition-colors text-left cursor-pointer group"
        role="menuitem"
      >
        <div className="flex items-center gap-2.5">
          <Paperclip className="size-4 text-white/60 group-hover:text-white transition-colors" />
          <span className="text-[13px] font-medium text-white/90 group-hover:text-white">
            Add files or photos
          </span>
        </div>
        <span className="text-[11px] font-mono text-white/40 tracking-wider">⌘U</span>
      </button>

      <div className="h-px bg-white/8 my-1 mx-1.5" />

      {/* 2. Skills */}
      <button
        type="button"
        onClick={() => {
          onOpenSkills?.();
          onClose();
        }}
        className="w-full flex items-center justify-between px-3 py-2.5 rounded-xl hover:bg-white/8 transition-colors text-left cursor-pointer group"
        role="menuitem"
      >
        <div className="flex items-center gap-2.5">
          <Sparkles className="size-4 text-white/60 group-hover:text-white transition-colors" />
          <span className="text-[13px] font-medium text-white/90 group-hover:text-white">
            Skills
          </span>
        </div>
        <ChevronRight className="size-3.5 text-white/40 group-hover:text-white/70 transition-colors" />
      </button>

      {/* 3. Add connector */}
      <button
        type="button"
        disabled={!onAddConnector}
        title={onAddConnector ? 'Configure a connector' : 'Connector configuration is not available in this composer'}
        onClick={() => {
          onAddConnector?.();
          onClose();
        }}
        className="w-full flex items-center justify-between px-3 py-2.5 rounded-xl hover:bg-white/8 transition-colors text-left cursor-pointer group"
        role="menuitem"
      >
        <div className="flex items-center gap-2.5">
          <Blocks className="size-4 text-white/60 group-hover:text-white transition-colors" />
          <span className="text-[13px] font-medium text-white/90 group-hover:text-white">
            Add connector
          </span>
        </div>
        <ChevronRight className="size-3.5 text-white/40 group-hover:text-white/70 transition-colors" />
      </button>

      {/* 4. Add plugins */}
      <button
        type="button"
        onClick={() => {
          onAddPlugins?.();
          onClose();
        }}
        className="w-full flex items-center justify-between px-3 py-2.5 rounded-xl hover:bg-white/8 transition-colors text-left cursor-pointer group"
        role="menuitem"
      >
        <div className="flex items-center gap-2.5">
          <Plug className="size-4 text-white/60 group-hover:text-white transition-colors" />
          <span className="text-[13px] font-medium text-white/90 group-hover:text-white">
            Add plugins
          </span>
        </div>
        <ChevronRight className="size-3.5 text-white/40 group-hover:text-white/70 transition-colors" />
      </button>
    </div>
  );
};
