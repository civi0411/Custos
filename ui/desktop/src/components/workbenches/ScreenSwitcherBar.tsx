import React, { useEffect, useMemo, useRef, useState } from 'react';
import {
  BookOpen,
  Check,
  ChevronDown,
  Code,
  Cpu,
  GitCompare,
  Layers,
  Monitor,
  Pin,
  Plus,
  Terminal,
  X,
} from 'lucide-react';
import type { Screen, ScreenPreset } from '@/types/screen';
import { screensForWorkbench, WORKBENCH_SCREEN_PRESETS } from '@/types/screen';

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

const screenIcons: Record<string, React.ElementType> = {
  GitCompare,
  Code,
  BookOpen,
  Cpu,
  Terminal,
};

export const ScreenSwitcherBar: React.FC<ScreenSwitcherBarProps> = ({
  screens,
  activeScreenId,
  onSelectScreen,
  onCreateScreen,
  onCloseScreen,
  onTogglePinScreen,
  onRenameScreen,
  currentWorkbench,
  className = 'flex h-8 min-w-0 items-center gap-1 px-2 text-xs',
}) => {
  const [isPresetDropdownOpen, setIsPresetDropdownOpen] = useState(false);
  const [editingScreenId, setEditingScreenId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState('');
  const cancelRenameOnBlur = useRef(false);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const visibleScreens = useMemo(() => screensForWorkbench(screens, currentWorkbench), [screens, currentWorkbench]);
  const relevantPresets = useMemo(
    () => WORKBENCH_SCREEN_PRESETS.filter((preset) => preset.workbench === currentWorkbench),
    [currentWorkbench],
  );

  useEffect(() => {
    if (!isPresetDropdownOpen) return;
    const closeOnPointerDown = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsPresetDropdownOpen(false);
      }
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setIsPresetDropdownOpen(false);
    };
    window.addEventListener('mousedown', closeOnPointerDown);
    window.addEventListener('keydown', closeOnEscape);
    return () => {
      window.removeEventListener('mousedown', closeOnPointerDown);
      window.removeEventListener('keydown', closeOnEscape);
    };
  }, [isPresetDropdownOpen]);

  const rename = (screenId: string) => {
    if (cancelRenameOnBlur.current) {
      cancelRenameOnBlur.current = false;
      setEditingScreenId(null);
      return;
    }
    const nextName = editingName.trim();
    if (nextName && onRenameScreen) onRenameScreen(screenId, nextName);
    cancelRenameOnBlur.current = true;
    setEditingScreenId(null);
  };

  const focusAdjacentTab = (event: React.KeyboardEvent<HTMLButtonElement>) => {
    if (event.key !== 'ArrowRight' && event.key !== 'ArrowLeft' && event.key !== 'Home' && event.key !== 'End') return;
    event.preventDefault();
    const tabs = Array.from(event.currentTarget.closest('[data-screen-switcher]')?.querySelectorAll<HTMLButtonElement>('[data-screen-switch]') || []);
    const currentIndex = tabs.indexOf(event.currentTarget);
    if (currentIndex < 0 || tabs.length === 0) return;
    const nextIndex = event.key === 'Home'
      ? 0
      : event.key === 'End'
        ? tabs.length - 1
        : (currentIndex + (event.key === 'ArrowRight' ? 1 : -1) + tabs.length) % tabs.length;
    tabs[nextIndex]?.focus();
    tabs[nextIndex]?.click();
  };

  return (
    <div className={className}>
      <div role="group" aria-label={`${currentWorkbench} screens`} data-screen-switcher className="flex h-full min-w-0 flex-1 items-center gap-1 overflow-x-auto no-scrollbar">
        {visibleScreens.length === 0 ? (
          <span className="px-2 text-[11px] text-fg-subtle">No screen yet</span>
        ) : visibleScreens.map((screen) => {
          const isActive = screen.id === activeScreenId;
          const isEditing = editingScreenId === screen.id;
          const Icon = screenIcons[screen.icon || ''] || Monitor;
          return (
            <div
              key={screen.id}
              className={`group flex h-7 shrink-0 items-center rounded-md border transition-colors ${isActive
                ? 'border-border-emphasis bg-surface-2 text-fg-editor'
                : 'border-transparent text-fg-muted hover:bg-surface-1 hover:text-fg-editor'
                }`}
            >
              {isEditing ? (
                <input
                  aria-label={`Rename ${screen.name}`}
                  autoFocus
                  value={editingName}
                  onChange={(event) => setEditingName(event.target.value)}
                  onBlur={() => rename(screen.id)}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter') {
                      event.preventDefault();
                      rename(screen.id);
                    }
                    if (event.key === 'Escape') {
                      event.preventDefault();
                      cancelRenameOnBlur.current = true;
                      setEditingScreenId(null);
                    }
                  }}
                  className="h-6 w-28 rounded border border-border-emphasis bg-surface-0 px-1.5 text-[11px] text-fg-editor outline-none focus-visible:ring-2 focus-visible:ring-accent"
                />
              ) : (
                <button
                  type="button"
                  data-screen-switch
                  aria-pressed={isActive}
                  tabIndex={isActive ? 0 : -1}
                  onClick={() => onSelectScreen(screen.id)}
                  onDoubleClick={() => {
                    cancelRenameOnBlur.current = false;
                    setEditingScreenId(screen.id);
                    setEditingName(screen.name);
                  }}
                  onKeyDown={(event) => {
                    focusAdjacentTab(event);
                    if (event.key === 'F2') {
                      event.preventDefault();
                      cancelRenameOnBlur.current = false;
                      setEditingScreenId(screen.id);
                      setEditingName(screen.name);
                    }
                  }}
                  className="flex h-full min-w-0 items-center gap-1.5 rounded-l-md px-2 outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-accent"
                  title={`${screen.name}${screen.anchor?.branch ? ` · ${screen.anchor.branch}` : ''}`}
                >
                  <Icon aria-hidden="true" className="size-3.5 shrink-0 opacity-70" />
                  <span className="max-w-40 truncate text-[11px] font-medium">{screen.name}</span>
                  {screen.anchor?.branch && screen.anchor.branch !== screen.name && (
                    <span className="max-w-24 truncate rounded bg-surface-3 px-1 font-mono text-[9px] text-fg-muted">{screen.anchor.branch}</span>
                  )}
                </button>
              )}
              {onTogglePinScreen && (
                <button
                  type="button"
                  onClick={() => onTogglePinScreen(screen.id)}
                  className={`mx-0.5 flex size-5 items-center justify-center rounded text-fg-subtle hover:bg-surface-3 hover:text-fg-editor focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent ${screen.isPinned ? 'text-fg-editor' : 'opacity-0 group-hover:opacity-100 group-focus-within:opacity-100'}`}
                  title={screen.isPinned ? 'Unpin screen' : 'Pin screen'}
                  aria-label={screen.isPinned ? `Unpin ${screen.name}` : `Pin ${screen.name}`}
                >
                  <Pin aria-hidden="true" className={`size-3 ${screen.isPinned ? 'rotate-45' : ''}`} />
                </button>
              )}
              {visibleScreens.length > 1 && !screen.isPinned && (
                <button
                  type="button"
                  onClick={() => onCloseScreen(screen.id)}
                  className="mr-1 flex size-5 items-center justify-center rounded text-fg-subtle opacity-0 hover:bg-surface-3 hover:text-fg-editor focus-visible:opacity-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent group-hover:opacity-100 group-focus-within:opacity-100"
                  title={`Close ${screen.name}`}
                  aria-label={`Close ${screen.name}`}
                >
                  <X aria-hidden="true" className="size-3" />
                </button>
              )}
            </div>
          );
        })}
      </div>

      <div ref={dropdownRef} className="relative flex shrink-0 items-center gap-0.5">
        <button
          type="button"
          onClick={() => onCreateScreen()}
          className="flex size-7 items-center justify-center rounded-md text-fg-muted hover:bg-surface-2 hover:text-fg-editor focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent"
          title="New screen"
          aria-label="New screen"
        >
          <Plus aria-hidden="true" className="size-4" />
        </button>
        <button
          type="button"
          aria-expanded={isPresetDropdownOpen}
          onClick={() => setIsPresetDropdownOpen((open) => !open)}
          className="flex size-7 items-center justify-center rounded-md text-fg-subtle hover:bg-surface-2 hover:text-fg-editor focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent"
          title="Screen layouts"
          aria-label="Screen layout presets"
        >
          <ChevronDown aria-hidden="true" className="size-3.5" />
        </button>
        {isPresetDropdownOpen && (
          <div className="absolute right-0 top-full z-50 mt-1 w-64 rounded-lg border border-border-default bg-surface-overlay p-1.5 text-xs shadow-xl">
            <div className="flex items-center gap-1.5 border-b border-border-muted px-2 pb-1.5 pt-1 text-[10px] font-semibold uppercase tracking-wide text-fg-subtle">
              <Layers aria-hidden="true" className="size-3.5" />
              <span>Start with a layout</span>
            </div>
            {relevantPresets.map((preset) => (
              <button
                key={preset.id}
                type="button"
                onClick={() => {
                  onCreateScreen(preset);
                  setIsPresetDropdownOpen(false);
                }}
                className="flex w-full items-start gap-2 rounded-md px-2 py-2 text-left text-fg-muted hover:bg-surface-2 hover:text-fg-editor focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent"
              >
                <Check aria-hidden="true" className="mt-0.5 size-3.5 shrink-0 opacity-50" />
                <span className="min-w-0">
                  <span className="block text-[11px] font-medium">{preset.name}</span>
                  <span className="mt-0.5 block text-[10px] leading-snug text-fg-subtle">{preset.description}</span>
                </span>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default ScreenSwitcherBar;
