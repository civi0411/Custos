import React, { useState, useEffect, useRef } from 'react';
import {
  Search,
  Terminal,
  FileDiff,
  FolderTree,
  GitBranch,
  Bot,
  ShieldCheck,
  BookOpen,
  FlaskConical,
  MessageSquare,
  Code2,
  Settings2,
  Plus,
  ArrowRight
} from 'lucide-react';
import type { AppWorkspaceMode } from './AppHeader';
import type { OrcaTabId } from '@/components/views/OrcaTabbedContainer';

export interface CommandPaletteProps {
  isOpen: boolean;
  onClose: () => void;
  onSwitchMode: (mode: AppWorkspaceMode) => void;
  onOpenTab: (id: OrcaTabId, title: string, url: string) => void;
  onNewSession: () => void;
  onOpenSettings: () => void;
  onOpenWorktrees: () => void;
  currentMode: AppWorkspaceMode;
}

interface PaletteItem {
  id: string;
  category: 'tabs' | 'worktrees' | 'modes' | 'actions';
  title: string;
  subtitle?: string;
  icon: React.ElementType;
  iconColor?: string;
  badge?: string;
  shortcut?: string;
  action: () => void;
}

export const CommandPalette: React.FC<CommandPaletteProps> = ({
  isOpen,
  onClose,
  onSwitchMode,
  onOpenTab,
  onNewSession,
  onOpenSettings,
  onOpenWorktrees,
  currentMode
}) => {
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isOpen) {
      setQuery('');
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [isOpen]);

  // Global Cmd+K / Cmd+P listener
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && (e.key === 'k' || e.key === 'p')) {
        e.preventDefault();
        if (isOpen) {
          onClose();
        } else {
          // Open handled by parent or toggled
        }
      }
      if (isOpen && e.key === 'Escape') {
        e.preventDefault();
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  const allItems: PaletteItem[] = [
    // Lenses
    {
      id: 'mode-chat',
      category: 'modes',
      title: 'Switch to Copilot Lens',
      subtitle: 'Conversational steering & architecture advisory',
      icon: MessageSquare,
      iconColor: 'text-[#cc785c]',
      badge: currentMode === 'chat' ? 'Current' : undefined,
      shortcut: '⌘1',
      action: () => { onSwitchMode('chat'); onClose(); }
    },
    {
      id: 'mode-code',
      category: 'modes',
      title: 'Switch to Coding Workbench (ADE)',
      subtitle: 'Worktrees, code editing, terminal & git diffs',
      icon: Code2,
      iconColor: 'text-[#58a6ff]',
      badge: currentMode === 'code' ? 'Current' : undefined,
      shortcut: '⌘2',
      action: () => { onSwitchMode('code'); onClose(); }
    },
    {
      id: 'mode-research',
      category: 'modes',
      title: 'Switch to Research Lab',
      subtitle: 'Literature corpus, claims matrix & reproducible runs',
      icon: FlaskConical,
      iconColor: 'text-[#a371f7]',
      badge: currentMode === 'research' ? 'Current' : undefined,
      shortcut: '⌘3',
      action: () => { onSwitchMode('research'); onClose(); }
    },

    // Workbench Tabs
    {
      id: 'tab-terminal',
      category: 'tabs',
      title: 'Open Terminal',
      subtitle: 'Bounded sovereign sandbox PTY shell',
      icon: Terminal,
      iconColor: 'text-[#58a6ff]',
      action: () => { onOpenTab('terminal', 'Terminal', 'custos://terminal'); onClose(); }
    },
    {
      id: 'tab-changes',
      category: 'tabs',
      title: 'Open Git Changes & Diffs',
      subtitle: 'Review staged and unstaged worktree patches',
      icon: FileDiff,
      iconColor: 'text-[#3fb950]',
      action: () => { onOpenTab('changes', 'Changes', 'custos://changes'); onClose(); }
    },
    {
      id: 'tab-files',
      category: 'tabs',
      title: 'Open Workspace Files',
      subtitle: 'Browse repository tree and source files',
      icon: FolderTree,
      iconColor: 'text-[#e6edf3]',
      action: () => { onOpenTab('files', 'Files', 'custos://files'); onClose(); }
    },
    {
      id: 'tab-literature',
      category: 'tabs',
      title: 'Open Literature Corpus',
      subtitle: 'Verified scientific papers, DOIs and passages',
      icon: BookOpen,
      iconColor: 'text-[#a371f7]',
      action: () => { onOpenTab('literature', 'Literature', 'custos://research/literature'); onClose(); }
    },
    {
      id: 'tab-claims',
      category: 'tabs',
      title: 'Open Claims Matrix',
      subtitle: 'Grounding levels, contradiction detection & evidence',
      icon: ShieldCheck,
      iconColor: 'text-[#388bfd]',
      action: () => { onOpenTab('claims', 'Claims Matrix', 'custos://research/claims'); onClose(); }
    },
    {
      id: 'tab-kanban',
      category: 'tabs',
      title: 'Open Fleet Kanban (O11)',
      subtitle: 'Needs You, Working, Done, and Idle agent dashboard',
      icon: Bot,
      iconColor: 'text-[#bc8cff]',
      shortcut: '⌘⌥K',
      action: () => { onOpenTab('kanban', 'Fleet Kanban', 'orca://kanban'); onClose(); }
    },

    // Worktrees
    {
      id: 'worktree-manager',
      category: 'worktrees',
      title: 'Manage Git Worktrees (OrCa Lifecycle)',
      subtitle: 'Switch branches, inspect modified files, merge or ship',
      icon: GitBranch,
      iconColor: 'text-[#58a6ff]',
      action: () => { onOpenWorktrees(); onClose(); }
    },

    // Quick Actions
    {
      id: 'action-new-task',
      category: 'actions',
      title: 'Create New Task / Session',
      subtitle: 'Start clean dialogue or bounded agent execution',
      icon: Plus,
      iconColor: 'text-[#3fb950]',
      shortcut: '⌘N',
      action: () => { onNewSession(); onClose(); }
    },
    {
      id: 'action-settings',
      category: 'actions',
      title: 'Open Settings & Providers',
      subtitle: 'Configure models, API keys, and sandbox permissions',
      icon: Settings2,
      iconColor: 'text-[#8b949e]',
      shortcut: '⌘,',
      action: () => { onOpenSettings(); onClose(); }
    }
  ];

  const filteredItems = allItems.filter((item) => {
    if (!query.trim()) return true;
    const q = query.toLowerCase();
    return (
      item.title.toLowerCase().includes(q) ||
      (item.subtitle && item.subtitle.toLowerCase().includes(q)) ||
      item.category.toLowerCase().includes(q)
    );
  });

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((prev) => (prev + 1) % Math.max(1, filteredItems.length));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((prev) => (prev - 1 + filteredItems.length) % Math.max(1, filteredItems.length));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (filteredItems[selectedIndex]) {
        filteredItems[selectedIndex].action();
      }
    }
  };

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-[100] flex items-start justify-center pt-24 px-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-150"
      onClick={onClose}
    >
      <div
        className="w-full max-w-2xl bg-surface-1 border border-border-default rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[75vh] animate-in zoom-in-95 duration-150"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Search Input Bar */}
        <div className="flex items-center gap-3 px-4 py-3 border-b border-border-muted bg-surface-1">
          <Search className="w-5 h-5 text-fg-muted shrink-0" />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setSelectedIndex(0);
            }}
            onKeyDown={handleKeyDown}
            placeholder="Type a command, search tabs, worktrees, or switch lens..."
            className="flex-1 bg-transparent border-none outline-none text-fg-editor placeholder-fg-subtle text-sm"
          />
          <kbd className="hidden sm:inline-flex items-center gap-0.5 px-2 py-0.5 text-[11px] font-mono text-fg-muted bg-surface-2 border border-border-default rounded">
            ESC
          </kbd>
        </div>

        {/* Results List */}
        <div className="flex-1 overflow-y-auto p-2 space-y-1">
          {filteredItems.length === 0 ? (
            <div className="py-12 text-center text-sm text-fg-muted">
              No matching commands or worktrees found for "{query}"
            </div>
          ) : (
            filteredItems.map((item, index) => {
              const isSelected = index === selectedIndex;
              const Icon = item.icon;
              return (
                <div
                  key={item.id}
                  onClick={item.action}
                  onMouseEnter={() => setSelectedIndex(index)}
                  className={`flex items-center justify-between px-3 py-2.5 rounded-xl cursor-pointer transition ${
                    isSelected ? 'bg-surface-2 text-fg-editor font-medium' : 'text-fg-muted hover:text-fg-editor hover:bg-surface-2/60'
                  }`}
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <div className={`p-1.5 rounded-lg bg-surface-2 border border-border-default ${item.iconColor || 'text-fg-muted'}`}>
                      <Icon className="w-4 h-4" />
                    </div>
                    <div className="min-w-0">
                      <div className="text-xs font-medium text-fg-editor flex items-center gap-2">
                        <span className="truncate">{item.title}</span>
                        {item.badge && (
                          <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-surface-3 text-fg-muted border border-border-default">
                            {item.badge}
                          </span>
                        )}
                      </div>
                      {item.subtitle && (
                        <div className="text-[11px] text-fg-muted truncate">{item.subtitle}</div>
                      )}
                    </div>
                  </div>

                  <div className="flex items-center gap-2 shrink-0 ml-3">
                    {item.shortcut && (
                      <kbd className="px-1.5 py-0.5 text-[10px] font-mono text-fg-muted bg-surface-2 border border-border-default rounded">
                        {item.shortcut}
                      </kbd>
                    )}
                    {isSelected && <ArrowRight className="w-3.5 h-3.5 workbench-accent" />}
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* Footer */}
        <div className="px-4 py-2 border-t border-border-muted bg-surface-1 flex items-center justify-between text-[11px] text-fg-muted">
          <div className="flex items-center gap-3">
            <span><kbd className="px-1 py-0.5 rounded bg-surface-2 border border-border-default">↑↓</kbd> to navigate</span>
            <span><kbd className="px-1 py-0.5 rounded bg-surface-2 border border-border-default">↵</kbd> to select</span>
          </div>
          <span className="font-mono text-[10px] text-fg-subtle">OrCa Jump Palette • Custos SADE</span>
        </div>
      </div>
    </div>
  );
};
