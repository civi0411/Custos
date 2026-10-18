import React, { useState, useRef, useEffect, useMemo } from 'react';
import {
  Monitor,
  Plus,
  X,
  Pin,
  ChevronDown,
  Layers,
  LayoutGrid,
  GitCompare,
  Code,
  BookOpen,
  Cpu,
  Terminal,
} from 'lucide-react';
import type { Screen, ScreenPreset } from '@/types/screen';
import { WORKBENCH_SCREEN_PRESETS } from '@/types/screen';

export interface ScreenSwitcherBarProps {
  screens: Screen[];
  activeScreenId: string;
  onSelectScreen: (screenId: string) => void;
  onCreateScreen: (preset?: ScreenPreset) => void;
  onCloseScreen: (screenId: string) => void;
  onTogglePinScreen?: (screenId: string) => void;
  onRenameScreen?: (screenId: string, newName: string) => void;
  currentWorkbench: 'engineering' | 'research' | 'copilot';
  className?: string;
}

export const ScreenSwitcherBar: React.FC<ScreenSwitcherBarProps> = ({
  screens,
  activeScreenId,
  onSelectScreen,
  onCreateScreen,
  onCloseScreen,
  onTogglePinScreen,
  onRenameScreen,
  currentWorkbench,
  className = 'h-8 px-2 flex items-center justify-between bg-[#141417] border-b border-white/8 shrink-0 select-none z-10 text-xs',
}) => {
  const [isPresetDropdownOpen, setIsPresetDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const [editingScreenId, setEditingScreenId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState('');

  // Close dropdown on outside click
  useEffect(() => {
    const handleOutsideClick = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setIsPresetDropdownOpen(false);
      }
    };
    window.addEventListener('mousedown', handleOutsideClick);
    return () => window.removeEventListener('mousedown', handleOutsideClick);
  }, []);

  const relevantPresets = WORKBENCH_SCREEN_PRESETS.filter(
    (p) => p.workbench === currentWorkbench
  );

  const visibleScreens = useMemo<Screen[]>(() => {
    const filtered = screens.filter(
      (s) => s.homeWorkbench === currentWorkbench || (!s.homeWorkbench && currentWorkbench === 'engineering')
    );
    return filtered.length > 0 ? filtered : screens.slice(0, 1);
  }, [screens, currentWorkbench]);

  const getScreenIcon = (screen: Screen) => {
    if (screen.icon === 'GitCompare') return GitCompare;
    if (screen.icon === 'Code') return Code;
    if (screen.icon === 'BookOpen') return BookOpen;
    if (screen.icon === 'Cpu') return Cpu;
    if (screen.icon === 'Terminal') return Terminal;
    return Monitor;
  };

  return (
    <div className={className}>
      {/* ── Screen Tabs List (Adaptive: Single Work Context Pill when 1 screen, Browser Tabs when > 1 screen) ── */}
      <div className="flex items-center justify-start gap-1 overflow-x-auto no-scrollbar flex-1 min-w-0 h-full">
        {visibleScreens.length === 1 ? (
          <div
            className="flex items-center gap-1.5 h-6.5 px-2 rounded bg-white/4 border border-white/8 text-white/80 select-none text-xs shrink-0 font-sans cursor-default"
            title={`Active Working View: ${visibleScreens[0].name}${visibleScreens[0].anchor?.branch ? ` (${visibleScreens[0].anchor.branch})` : ''}`}
          >
            {(() => {
              const SingleIcon = getScreenIcon(visibleScreens[0]);
              return <SingleIcon className="size-3.5 text-white/50 shrink-0" />;
            })()}
            <span className="font-medium text-[11px] truncate max-w-[200px] text-white/90">
              {visibleScreens[0].name}
            </span>
            {visibleScreens[0].anchor?.branch && visibleScreens[0].anchor.branch !== visibleScreens[0].name && (
              <span className="px-1 py-0.2 rounded bg-white/6 font-mono text-[9px] text-white/50">
                {visibleScreens[0].anchor.branch}
              </span>
            )}
          </div>
        ) : (
          visibleScreens.map((screen) => {
            const isActive = screen.id === activeScreenId;
            const Icon = getScreenIcon(screen);
            const isEditing = editingScreenId === screen.id;

            return (
              <div
                key={screen.id}
                onClick={() => onSelectScreen(screen.id)}
                onDoubleClick={() => {
                  setEditingScreenId(screen.id);
                  setEditingName(screen.name);
                }}
                className={`group relative flex items-center gap-1.5 h-6.5 px-2 rounded cursor-pointer transition select-none text-xs shrink-0 border ${
                  isActive
                    ? 'bg-white/10 text-white border-white/15 font-medium'
                    : 'text-white/50 hover:text-white/80 hover:bg-white/4 border-transparent'
                }`}
              >
                <Icon className={`size-3.5 shrink-0 ${isActive ? 'text-white' : 'text-white/40'}`} />

                {isEditing ? (
                  <input
                    type="text"
                    value={editingName}
                    onChange={(e) => setEditingName(e.target.value)}
                    onBlur={() => {
                      if (editingName.trim() && onRenameScreen) {
                        onRenameScreen(screen.id, editingName.trim());
                      }
                      setEditingScreenId(null);
                    }}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') {
                        if (editingName.trim() && onRenameScreen) {
                          onRenameScreen(screen.id, editingName.trim());
                        }
                        setEditingScreenId(null);
                      } else if (e.key === 'Escape') {
                        setEditingScreenId(null);
                      }
                    }}
                    autoFocus
                    className="bg-black/60 border border-white/20 rounded px-1 text-xs text-white outline-none w-24"
                    onClick={(e) => e.stopPropagation()}
                  />
                ) : (
                  <span className="truncate max-w-[130px] font-medium text-[11px]">{screen.name}</span>
                )}

                {/* Pin indicator / toggle button */}
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onTogglePinScreen?.(screen.id);
                  }}
                  className={`p-0.5 rounded hover:bg-white/10 transition shrink-0 ${
                    screen.isPinned
                      ? 'text-white/80'
                      : 'opacity-0 group-hover:opacity-100 text-white/30 hover:text-white/70'
                  }`}
                  title={screen.isPinned ? "Unpin screen" : "Pin screen"}
                  aria-label={screen.isPinned ? "Unpin screen" : "Pin screen"}
                >
                  <Pin className={`size-2.5 ${screen.isPinned ? 'rotate-45 fill-white/20' : ''}`} />
                </button>

                {/* Close Screen button (when > 1 screen) */}
                {screens.length > 1 && !screen.isPinned && (
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      onCloseScreen(screen.id);
                    }}
                    className="opacity-0 group-hover:opacity-100 p-0.5 rounded text-white/30 hover:text-white hover:bg-white/10 transition shrink-0 ml-0.5"
                    title="Close Screen"
                    aria-label="Close Screen"
                  >
                    <X className="size-2.5" />
                  </button>
                )}
              </div>
            );
          })
        )}

        {/* ── Browser-style '+' New Screen Button ── */}
        <div className="relative shrink-0 flex items-center ml-0.5" ref={dropdownRef}>
          <button
            type="button"
            className="flex items-center justify-center size-6 rounded text-white/40 hover:text-white hover:bg-white/8 transition cursor-pointer"
            title="New Screen (⌘T)"
            aria-label="New Screen"
            onClick={() => onCreateScreen()}
          >
            <Plus size={13} />
          </button>
          <button
            type="button"
            onClick={() => setIsPresetDropdownOpen((prev) => !prev)}
            className="flex items-center justify-center size-6 rounded text-white/30 hover:text-white hover:bg-white/8 transition cursor-pointer -ml-0.5"
            title="Screen Layout Presets"
            aria-label="Screen Layout Presets"
          >
            <ChevronDown size={10} />
          </button>

          {/* Preset Selection Popover */}
          {isPresetDropdownOpen && (
            <div className="absolute top-full left-0 mt-1 w-64 rounded bg-[#18181b] border border-white/10 shadow-xl p-1 z-50 text-xs">
              <div className="px-2 py-1 text-[10px] font-semibold text-white/40 uppercase tracking-wider flex items-center gap-1 border-b border-white/6 mb-1 pb-1">
                <Layers className="size-3 text-white/40" />
                <span>Screen Layout Presets</span>
              </div>
              {relevantPresets.map((preset) => (
                <button
                  key={preset.id}
                  type="button"
                  onClick={() => {
                    onCreateScreen(preset);
                    setIsPresetDropdownOpen(false);
                  }}
                  className="w-full flex items-start gap-2 px-2 py-1.5 rounded text-left text-white/70 hover:text-white hover:bg-white/6 transition cursor-pointer"
                >
                  <LayoutGrid className="size-3.5 text-white/40 shrink-0 mt-0.5" />
                  <div>
                    <div className="font-medium text-[11px] text-white/85">{preset.name}</div>
                    <div className="text-[10px] text-white/40 leading-tight">{preset.description}</div>
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default ScreenSwitcherBar;
