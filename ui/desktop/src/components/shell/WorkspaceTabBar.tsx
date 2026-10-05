import React, { useState, useRef, useEffect, useMemo } from 'react';
import { createPortal } from 'react-dom';
import { 
  BookOpen, 
  Columns2, 
  FileDiff, 
  MessageSquare, 
  PanelRight, 
  Plus, 
  X,
  Bot,
  Activity,
  Terminal,
  Globe,
  FileText,
  Smartphone,
  Settings as SettingsIcon,
  Search,
  LayoutGrid,
  Square,
  Columns3,
  Grid2X2,
  Code2,
  BrainCircuit
} from 'lucide-react';
import { 
  OpenAIIcon, 
  ClaudeIcon, 
  GeminiIcon, 
  DeepSeekIcon 
} from '@/components/common';
import { useAppContext } from '@/context/AppContext';
import type { WorkspacePaneType, WorkbenchLens } from '@/types';

export type WorkspacePane = {
  id: WorkspacePaneType;
  title: string;
};

export type LayoutMode = 'single' | 'split-2' | 'split-3' | 'split-4' | 'split-5';

interface WorkbenchMeta {
  id: WorkspacePaneType;
  label: string;
  tabTitle: string;
  sublabel: string;
  badge: string;
  description: string;
  shortcut: string;
  icon: React.ComponentType<{ className?: string; size?: number }>;
  accentColor: string;
  statusInfo: string;
}

const WORKBENCH_DETAILS: Record<WorkspacePaneType, WorkbenchMeta> = {
  chat: {
    id: 'chat',
    label: 'Conversation',
    tabTitle: 'AI Co-Pilot',
    sublabel: 'Autonomous Dialogue',
    badge: 'Co-Pilot',
    description: 'AI dialog with markdown, thinking traces, prompt starters and context integration.',
    shortcut: '⌘3',
    icon: MessageSquare,
    accentColor: 'text-brand-blue',
    statusInfo: 'Ready for instructions'
  },
  engineering: {
    id: 'engineering',
    label: 'Coding',
    tabTitle: '..entHub/Custos',
    sublabel: 'Codex AI IDE & Terminal',
    badge: 'Codex IDE',
    description: 'Monaco editor, inline Cmd+K AI assistant, multi-tab terminal & invariant verification.',
    shortcut: '⌘1',
    icon: Terminal,
    accentColor: 'text-emerald-400',
    statusInfo: 'Rust & TS Runtime Active'
  },
  research: {
    id: 'research',
    label: 'Research',
    tabTitle: 'Research Lab',
    sublabel: 'Literature & Graph',
    badge: 'Scholar',
    description: 'ArXiv & BioRxiv literature browser, citation network graph and AI synthesis notebook.',
    shortcut: '⌘2',
    icon: BookOpen,
    accentColor: 'text-purple-400',
    statusInfo: '3 Sources Indexed'
  },
  assistant: {
    id: 'assistant',
    label: 'Assistant',
    tabTitle: 'Fleet Assistant',
    sublabel: 'Multi-Agent Fleet',
    badge: 'Orchestrator',
    description: 'Multi-agent orchestration, OmniRoute dispatch pipelines and cache telemetry.',
    shortcut: '⌘5',
    icon: Bot,
    accentColor: 'text-amber-400',
    statusInfo: '3 Subagents Active'
  },
  diff: {
    id: 'diff',
    label: 'Changes',
    tabTitle: 'Diff Review',
    sublabel: 'Patch Inspection',
    badge: 'Diff Viewer',
    description: 'Unified and side-by-side diff inspector, hunk navigation and live patch applicator.',
    shortcut: '⌘4',
    icon: FileDiff,
    accentColor: 'text-blue-400',
    statusInfo: 'Live Worktree Diff (+34 -2)'
  },
  browser: {
    id: 'browser',
    label: 'Browser',
    tabTitle: 'Local Browser',
    sublabel: 'Web & Preview',
    badge: 'Preview',
    description: 'Embedded Web Browser with URL navigation, viewport switcher, and live dev preview.',
    shortcut: '⌘⇧B',
    icon: Globe,
    accentColor: 'text-cyan-400',
    statusInfo: '200 OK • Live Server'
  },
  markdown: {
    id: 'markdown',
    label: 'Markdown',
    tabTitle: 'Notes.md',
    sublabel: 'Document Scratchpad',
    badge: 'Docs',
    description: 'Interactive Markdown editor with live preview, formatting toolbar, and task checklists.',
    shortcut: '⌘⇧M',
    icon: FileText,
    accentColor: 'text-amber-400',
    statusInfo: 'Live Document Synced'
  }
};

interface MenuItem {
  id: string;
  label: string;
  category: 'tool' | 'agent' | 'action';
  shortcut?: string;
  icon: React.ComponentType<{ className?: string; size?: number }>;
  iconColor?: string;
  action: () => void;
}

export function WorkspaceTabBar({
  panes,
  selected,
  split: _unusedSplit,
  layoutMode = 'split-2',
  onChangeLayoutMode,
  inspectorOpen,
  onSelect,
  onClose,
  onAdd,
  onToggleSplit: _unusedToggleSplit,
  onToggleInspector,
  onOpenSettings,
  onSelectAgentModel,
  activeLens = 'coding',
  activeTask,
  onSwitchLens,
  onOpenDelegateModal,
  onOpenTaskDetailsModal
}: {
  panes: WorkspacePane[];
  selected: WorkspacePaneType;
  split: boolean;
  layoutMode?: LayoutMode;
  onChangeLayoutMode?: (mode: LayoutMode) => void;
  inspectorOpen: boolean;
  onSelect: (pane: WorkspacePaneType) => void;
  onClose: (pane: WorkspacePaneType) => void;
  onAdd: (pane: WorkspacePaneType) => void;
  onToggleSplit: () => void;
  onToggleInspector: () => void;
  onOpenSettings?: () => void;
  onSelectAgentModel?: (model: string) => void;
  activeLens?: WorkbenchLens;
  activeTask?: any;
  onSwitchLens?: (lens: WorkbenchLens) => void;
  onOpenDelegateModal?: () => void;
  onOpenTaskDetailsModal?: () => void;
}) {
  const { setIsSettingsOpen } = useAppContext();

  const [hoveredTab, setHoveredTab] = useState<WorkspacePaneType | null>(null);
  const [isLauncherOpen, setIsLauncherOpen] = useState(false);
  const [isLayoutMenuOpen, setIsLayoutMenuOpen] = useState(false);
  const [buttonRect, setButtonRect] = useState<DOMRect | null>(null);
  const [layoutButtonRect, setLayoutButtonRect] = useState<DOMRect | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  
  const launcherRef = useRef<HTMLDivElement>(null);
  const layoutBtnRef = useRef<HTMLDivElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const layoutMenuRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const hoverTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const toggleLauncher = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!isLauncherOpen && launcherRef.current) {
      setButtonRect(launcherRef.current.getBoundingClientRect());
      setIsLauncherOpen(true);
      setIsLayoutMenuOpen(false);
    } else {
      setIsLauncherOpen(false);
    }
  };

  const toggleLayoutMenu = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!isLayoutMenuOpen && layoutBtnRef.current) {
      setLayoutButtonRect(layoutBtnRef.current.getBoundingClientRect());
      setIsLayoutMenuOpen(true);
      setIsLauncherOpen(false);
    } else {
      setIsLayoutMenuOpen(false);
    }
  };

  useEffect(() => {
    if (isLauncherOpen) {
      if (launcherRef.current) {
        setButtonRect(launcherRef.current.getBoundingClientRect());
      }
      setTimeout(() => searchInputRef.current?.focus(), 50);
      setSelectedIndex(0);
    }
  }, [isLauncherOpen]);

  useEffect(() => {
    const handleOutsideClick = (e: MouseEvent) => {
      const target = e.target as Node;
      if (
        popoverRef.current && 
        !popoverRef.current.contains(target) &&
        launcherRef.current &&
        !launcherRef.current.contains(target)
      ) {
        setIsLauncherOpen(false);
      }
      if (
        layoutMenuRef.current && 
        !layoutMenuRef.current.contains(target) &&
        layoutBtnRef.current && 
        !layoutBtnRef.current.contains(target)
      ) {
        setIsLayoutMenuOpen(false);
      }
    };
    document.addEventListener('mousedown', handleOutsideClick);
    return () => document.removeEventListener('mousedown', handleOutsideClick);
  }, []);

  const menuItems: MenuItem[] = useMemo(() => [
    // Section 1: Workspaces & Tools
    {
      id: 'new-terminal',
      label: 'New Terminal',
      category: 'tool',
      shortcut: '⌘T',
      icon: Terminal,
      action: () => onAdd('engineering')
    },
    {
      id: 'new-browser',
      label: 'New Browser Tab',
      category: 'tool',
      shortcut: '⌘⇧B',
      icon: Globe,
      iconColor: 'text-cyan-400',
      action: () => onAdd('browser')
    },
    {
      id: 'new-markdown',
      label: 'New Markdown',
      category: 'tool',
      shortcut: '⌘⇧M',
      icon: FileText,
      iconColor: 'text-amber-400',
      action: () => onAdd('markdown')
    },
    {
      id: 'new-research',
      label: 'New Research Lab',
      category: 'tool',
      shortcut: '⌘R',
      icon: BookOpen,
      iconColor: 'text-purple-400',
      action: () => onAdd('research')
    },
    {
      id: 'new-emulator',
      label: 'New Mobile Emulator',
      category: 'tool',
      shortcut: '⌘⌥⇧B',
      icon: Smartphone,
      action: () => onAdd('browser')
    },
    {
      id: 'new-diff',
      label: 'New Diff Review',
      category: 'tool',
      shortcut: '⌘D',
      icon: FileDiff,
      iconColor: 'text-blue-400',
      action: () => onAdd('diff')
    },

    // Section 2: AI Assistants & Agents
    {
      id: 'agent-claude',
      label: 'Claude',
      category: 'agent',
      icon: ClaudeIcon,
      action: () => {
        onAdd('chat');
        if (onSelectAgentModel) onSelectAgentModel('Claude 3.7 Sonnet');
      }
    },
    {
      id: 'agent-codex',
      label: 'Codex',
      category: 'agent',
      icon: OpenAIIcon,
      action: () => {
        onAdd('engineering');
        if (onSelectAgentModel) onSelectAgentModel('Codex / GPT-4o');
      }
    },
    {
      id: 'agent-gemini',
      label: 'Gemini 2.5 Pro',
      category: 'agent',
      icon: GeminiIcon,
      action: () => {
        onAdd('chat');
        if (onSelectAgentModel) onSelectAgentModel('Gemini 2.5 Pro');
      }
    },
    {
      id: 'agent-deepseek',
      label: 'DeepSeek V3',
      category: 'agent',
      icon: DeepSeekIcon,
      action: () => {
        onAdd('chat');
        if (onSelectAgentModel) onSelectAgentModel('DeepSeek V3');
      }
    },

    // Section 3: Settings
    {
      id: 'agent-settings',
      label: 'Agent settings...',
      category: 'action',
      icon: SettingsIcon,
      action: () => {
        if (onOpenSettings) onOpenSettings();
        else setIsSettingsOpen(true);
      }
    }
  ], [onAdd, onSelectAgentModel, onOpenSettings, setIsSettingsOpen]);

  const filteredMenuItems = useMemo(() => {
    if (!searchQuery.trim()) return menuItems;
    const q = searchQuery.toLowerCase();
    return menuItems.filter(item => 
      item.label.toLowerCase().includes(q) || 
      (item.shortcut && item.shortcut.toLowerCase().includes(q))
    );
  }, [menuItems, searchQuery]);

  const handleKeyDownPopover = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex(prev => (prev + 1) % filteredMenuItems.length);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex(prev => (prev - 1 + filteredMenuItems.length) % filteredMenuItems.length);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const item = filteredMenuItems[selectedIndex];
      if (item) {
        item.action();
        setIsLauncherOpen(false);
        setSearchQuery('');
      }
    } else if (e.key === 'Escape') {
      setIsLauncherOpen(false);
    }
  };

  const handleMouseEnterTab = (tab: WorkspacePaneType) => {
    if (hoverTimeoutRef.current) clearTimeout(hoverTimeoutRef.current);
    hoverTimeoutRef.current = setTimeout(() => {
      setHoveredTab(tab);
    }, 250);
  };

  const handleMouseLeaveTab = () => {
    if (hoverTimeoutRef.current) clearTimeout(hoverTimeoutRef.current);
    setHoveredTab(null);
  };

  const splitOptions: { id: LayoutMode; label: string; shortcut: string; icon: React.ComponentType<{ className?: string }> }[] = [
    { id: 'single', label: 'Single Pane (Focus)', shortcut: '⌘0', icon: Square },
    { id: 'split-2', label: 'Split 2 (Dual Columns)', shortcut: '⌘\\', icon: Columns2 },
    { id: 'split-3', label: 'Split 3 (Triple View)', shortcut: '⌘⌥3', icon: Columns3 },
    { id: 'split-4', label: 'Split 4 (2x2 Grid)', shortcut: '⌘⌥4', icon: Grid2X2 },
    { id: 'split-5', label: 'Split 5 (Master + 4 Tiles)', shortcut: '⌘⌥5', icon: LayoutGrid }
  ];

  return (
    <div className="flex flex-col border-b border-[#1c2130] bg-[#090b10] z-20 select-none font-sans text-xs shrink-0">
      {/* 1. Session & Task Status Strip */}
      <div className="flex h-7 items-center justify-between px-3 border-b border-[#1c2130] bg-[#06080a]">
        <div className="flex items-center gap-4 text-[10.5px] font-mono text-neutral-400">
          <div className="flex items-center gap-2">
            <span className="text-neutral-500">Session:</span>
            <span className="text-white font-medium truncate max-w-[250px]">{activeTask ? activeTask.title : 'New Conversation'}</span>
          </div>
          {activeTask && activeTask.taskStatus ? (
            <>
              <div className="w-px h-3 bg-[#1c2130]"></div>
              <div className="flex items-center gap-2">
                <span className="text-neutral-500">Task:</span>
                <span className="text-emerald-400">{activeTask.taskStatus || 'In Progress'}</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-neutral-500">Executor:</span>
                <span className="text-blue-400">{activeTask.model || 'Codex'}</span>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-neutral-500">Budget:</span>
                <span className="text-amber-400">$0.04 / $2.00</span>
              </div>
            </>
          ) : (
            <>
              <div className="w-px h-3 bg-[#1c2130]"></div>
              <span className="text-neutral-500">Mode: Assist (Direct Response)</span>
            </>
          )}
        </div>
        <div className="flex items-center gap-2">
          {activeTask && activeTask.taskStatus ? (
            <button
              onClick={onOpenTaskDetailsModal}
              className="px-2 py-0.5 rounded-sm bg-blue-500/10 text-blue-400 text-[10px] font-medium border border-blue-500/20 cursor-pointer hover:bg-blue-500/20 transition flex items-center gap-1"
            >
              Task Details
            </button>
          ) : (
            <button
              onClick={onOpenDelegateModal}
              className="px-2 py-0.5 rounded-sm bg-white/10 text-white text-[10px] hover:bg-white/20 transition flex items-center gap-1"
            >
              + Delegate Task
            </button>
          )}
        </div>
      </div>

      {/* 2. Lens Switcher & Workspace Tabs */}
      <div className="flex h-9 shrink-0 items-center justify-between px-2">
        {/* Lens Switcher */}
        <div className="flex items-center gap-1 bg-[#11141e] p-0.5 rounded-md border border-[#1c2130] mr-3">
          <button
            onClick={() => onSwitchLens?.('copilot')}
            className={`px-3 py-1 rounded-sm text-[11px] font-medium transition flex items-center gap-1.5 ${
              activeLens === 'copilot' ? 'bg-[#262c3e] text-white shadow-sm' : 'text-neutral-400 hover:text-white'
            }`}
          >
            <Bot size={13} className={activeLens === 'copilot' ? 'text-amber-400' : ''} />
            Copilot
          </button>
          <button
            onClick={() => onSwitchLens?.('coding')}
            className={`px-3 py-1 rounded-sm text-[11px] font-medium transition flex items-center gap-1.5 ${
              activeLens === 'coding' ? 'bg-[#262c3e] text-white shadow-sm' : 'text-neutral-400 hover:text-white'
            }`}
          >
            <Code2 size={13} className={activeLens === 'coding' ? 'text-emerald-400' : ''} />
            Coding
          </button>
          <button
            onClick={() => onSwitchLens?.('research')}
            className={`px-3 py-1 rounded-sm text-[11px] font-medium transition flex items-center gap-1.5 ${
              activeLens === 'research' ? 'bg-[#262c3e] text-white shadow-sm' : 'text-neutral-400 hover:text-white'
            }`}
          >
            <BrainCircuit size={13} className={activeLens === 'research' ? 'text-purple-400' : ''} />
            Research
          </button>
        </div>

        {/* Existing TabStrip */}
        <div className="flex min-w-0 flex-1 items-end gap-1 overflow-x-auto h-full pt-1 no-scrollbar border-l border-[#1c2130] pl-2">
          {panes.map((pane) => {
          const meta = WORKBENCH_DETAILS[pane.id] || WORKBENCH_DETAILS.chat;
          const Icon = meta.icon;
          const isActive = selected === pane.id;
          const displayTitle = pane.id === 'engineering' ? '..entHub/Custos' : meta.tabTitle;

          return (
            <div
              key={pane.id}
              className="relative shrink-0 flex items-end h-full"
              onMouseEnter={() => handleMouseEnterTab(pane.id)}
              onMouseLeave={handleMouseLeaveTab}
            >
              <div
                className={`group flex items-center gap-1.5 px-3 py-1 rounded-t-md text-xs font-mono transition duration-150 cursor-pointer border-t border-x ${
                  isActive
                    ? 'border-[#262c3e] bg-[#090b10] text-white font-medium shadow-sm -mb-[1px] pb-1.5'
                    : 'border-transparent bg-[#11141e]/50 text-neutral-400 hover:bg-[#161a28] hover:text-neutral-200'
                }`}
                onClick={() => onSelect(pane.id)}
              >
                <Icon className={`h-3.5 w-3.5 ${isActive ? meta.accentColor : 'text-neutral-500 group-hover:text-neutral-400'}`} size={14} />
                <span className="max-w-36 truncate text-[11.5px] font-sans font-medium">{displayTitle}</span>

                {isActive && (
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse ml-0.5"></span>
                )}

                {pane.id !== 'chat' && (
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      onClose(pane.id);
                    }}
                    aria-label={`Close ${meta.label}`}
                    className="ml-1 rounded p-0.5 text-neutral-500 hover:bg-white/10 hover:text-white transition opacity-60 group-hover:opacity-100"
                  >
                    <X className="h-3 w-3" />
                  </button>
                )}
              </div>

              {/* Orca-Style Tab Hover Preview Card */}
              {hoveredTab === pane.id && (
                <div className="absolute left-0 top-full mt-2 w-72 rounded-xl border border-surface-border bg-surface-card p-3 shadow-2xl z-50 animate-in fade-in zoom-in-95 duration-150 font-sans">
                  <div className="flex items-center justify-between pb-2 border-b border-surface-border mb-2">
                    <div className="flex items-center gap-2">
                      <div className="p-1.5 rounded-md bg-surface-elevated border border-surface-border">
                        <Icon className={`w-4 h-4 ${meta.accentColor}`} size={16} />
                      </div>
                      <div>
                        <div className="text-xs font-semibold text-white flex items-center gap-1.5 font-mono">
                          <span>{displayTitle}</span>
                          <span className="text-[10px] px-1.5 py-0.2 rounded bg-surface-elevated text-neutral-300 font-mono">
                            {meta.shortcut}
                          </span>
                        </div>
                        <div className="text-[10.5px] text-neutral-400">{meta.sublabel}</div>
                      </div>
                    </div>
                  </div>

                  <p className="text-[11px] text-neutral-300 leading-relaxed mb-2.5">
                    {meta.description}
                  </p>

                  <div className="flex items-center justify-between text-[10px] pt-1.5 border-t border-surface-border font-mono text-neutral-400">
                    <div className="flex items-center gap-1.5">
                      <Activity className="w-3 h-3 text-emerald-400" />
                      <span>{meta.statusInfo}</span>
                    </div>
                    <span className="text-brand-blue font-medium">Click to focus</span>
                  </div>
                </div>
              )}
            </div>
          );
        })}

        {/* Exact Orca (+) Button in rounded rectangle */}
        <div className="relative shrink-0 pb-1" ref={launcherRef}>
          <button
            type="button"
            onClick={toggleLauncher}
            aria-label="Add Workbench or Agent"
            className={`w-7 h-6 rounded-md flex items-center justify-center text-xs transition border ${
              isLauncherOpen
                ? 'border-neutral-500 bg-[#1e2334] text-white'
                : 'border-[#22283a] bg-[#121622] hover:bg-[#181d2e] text-neutral-300 hover:text-white'
            }`}
            title="Open Tab or Agent Palette (⌘T)"
          >
            <Plus className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      {/* Render Portal for the Popover outside to prevent ANY overflow clipping */}
      {isLauncherOpen && buttonRect && createPortal(
        <div 
          ref={popoverRef}
          onKeyDown={handleKeyDownPopover}
          style={{
            position: 'fixed',
            top: `${buttonRect.bottom + 6}px`,
            left: `${Math.min(buttonRect.left, window.innerWidth - 360)}px`,
            zIndex: 99999,
          }}
          className="w-[350px] rounded-xl border border-[#232938] bg-[#0c0e14] shadow-[0_20px_50px_rgba(0,0,0,0.8)] text-xs font-sans overflow-hidden animate-in fade-in zoom-in-95 duration-100 py-1 select-none"
        >
          {/* Search Bar at the Top */}
          <div className="px-3 py-2 border-b border-[#1c2130] flex items-center gap-2 bg-[#090b10]">
            <Search className="w-3.5 h-3.5 text-neutral-500 shrink-0" />
            <input
              ref={searchInputRef}
              type="text"
              value={searchQuery}
              onChange={(e) => {
                setSearchQuery(e.target.value);
                setSelectedIndex(0);
              }}
              placeholder="Search open tabs, history, files, URLs, agents..."
              className="w-full bg-transparent text-xs text-white placeholder-neutral-500 focus:outline-none font-sans"
            />
          </div>

          {/* Menu Items List - Guaranteed Full Height & Scrollable */}
          <div className="max-h-[380px] overflow-y-auto p-1 space-y-0.5">
            {filteredMenuItems.map((item, index) => {
              const ItemIcon = item.icon;
              const isSelected = index === selectedIndex;
              const prevItem = filteredMenuItems[index - 1];
              const showSeparator = prevItem && prevItem.category !== item.category;

              return (
                <React.Fragment key={item.id}>
                  {showSeparator && (
                    <div className="my-1 border-t border-[#1c2130]" />
                  )}

                  <div
                    onClick={() => {
                      item.action();
                      setIsLauncherOpen(false);
                      setSearchQuery('');
                    }}
                    onMouseEnter={() => setSelectedIndex(index)}
                    className={`flex items-center justify-between px-3 py-1.5 rounded-lg cursor-pointer transition text-xs ${
                      isSelected
                        ? 'bg-[#1b2030] text-white'
                        : 'text-neutral-300 hover:bg-[#151926] hover:text-white'
                    }`}
                  >
                    <div className="flex items-center gap-2.5 truncate">
                      <ItemIcon 
                        className={`w-4 h-4 shrink-0 ${item.iconColor || 'text-neutral-400'}`} 
                        size={16} 
                      />
                      <span className={`truncate ${item.category === 'agent' ? 'font-medium text-white' : ''}`}>
                        {item.label}
                      </span>
                    </div>

                    {item.shortcut && (
                      <span className="font-mono text-[11px] text-neutral-500 shrink-0 ml-2">
                        {item.shortcut}
                      </span>
                    )}
                  </div>
                </React.Fragment>
              );
            })}

            {filteredMenuItems.length === 0 && (
              <div className="p-4 text-center text-neutral-500 text-xs">
                No matching actions or agents found
              </div>
            )}
          </div>
        </div>,
        document.body
      )}

      {/* 2. Right Controls: Multi-Pane Split Layout Selector & File Explorer */}
      <div className="flex items-center gap-1 shrink-0 pl-2">
        {/* Dynamic Multi-Pane Split Layout Button with Popover */}
        <div className="relative" ref={layoutBtnRef}>
          <button
            type="button"
            onClick={toggleLayoutMenu}
            aria-label="Split Screen Layout (1, 2, 3, 4, 5 Panes)"
            title={`Layout Mode: ${layoutMode.toUpperCase()} (Click to change 1-5 split)`}
            className={`rounded-md px-1.5 py-1 text-xs transition border flex items-center gap-1 ${
              layoutMode !== 'single'
                ? 'border-brand-blue/50 bg-brand-blue/10 text-brand-blue'
                : 'border-transparent text-neutral-400 hover:bg-[#161a28] hover:text-white'
            }`}
          >
            {layoutMode === 'single' && <Square className="h-3.5 w-3.5" />}
            {layoutMode === 'split-2' && <Columns2 className="h-3.5 w-3.5" />}
            {layoutMode === 'split-3' && <Columns3 className="h-3.5 w-3.5" />}
            {layoutMode === 'split-4' && <Grid2X2 className="h-3.5 w-3.5" />}
            {layoutMode === 'split-5' && <LayoutGrid className="h-3.5 w-3.5" />}
            <span className="font-mono text-[10px] font-semibold">
              {layoutMode === 'single' ? '1' : layoutMode.replace('split-', '')}
            </span>
          </button>
        </div>

        {/* Multi-Pane Layout Selection Portal Menu */}
        {isLayoutMenuOpen && layoutButtonRect && createPortal(
          <div
            ref={layoutMenuRef}
            style={{
              position: 'fixed',
              top: `${layoutButtonRect.bottom + 6}px`,
              right: `${window.innerWidth - layoutButtonRect.right}px`,
              zIndex: 99999
            }}
            className="w-56 rounded-xl border border-[#232938] bg-[#0c0e14] shadow-[0_20px_50px_rgba(0,0,0,0.8)] text-xs font-sans overflow-hidden p-1.5 select-none"
          >
            <div className="px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-neutral-500 border-b border-[#1c2130] mb-1">
              Select Multi-Pane Split
            </div>

            {splitOptions.map((opt) => {
              const Icon = opt.icon;
              const isSelected = layoutMode === opt.id;
              return (
                <div
                  key={opt.id}
                  onClick={() => {
                    if (onChangeLayoutMode) onChangeLayoutMode(opt.id);
                    setIsLayoutMenuOpen(false);
                  }}
                  className={`flex items-center justify-between px-2.5 py-1.5 rounded-lg cursor-pointer transition ${
                    isSelected
                      ? 'bg-[#1b2030] text-white font-medium'
                      : 'text-neutral-400 hover:text-white hover:bg-[#141824]'
                  }`}
                >
                  <div className="flex items-center gap-2">
                    <Icon className="w-3.5 h-3.5 text-brand-blue" />
                    <span>{opt.label}</span>
                  </div>
                  <span className="font-mono text-[10.5px] text-neutral-500">{opt.shortcut}</span>
                </div>
              );
            })}
          </div>,
          document.body
        )}

        {/* Task Inspector & File Explorer Toggle */}
        <button
          type="button"
          onClick={onToggleInspector}
          aria-label={inspectorOpen ? 'Hide Right File Explorer' : 'Show Right File Explorer'}
          title={inspectorOpen ? 'Hide File Explorer (⌘E)' : 'Show File Explorer (⌘E)'}
          className={`rounded-md p-1.5 text-xs transition border ${
            inspectorOpen
              ? 'border-[#262c3e] bg-[#161a28] text-white'
              : 'border-transparent text-neutral-400 hover:bg-[#161a28] hover:text-white'
          }`}
        >
          <PanelRight className="h-3.5 w-3.5" />
        </button>
      </div>
      </div>
    </div>
  );
}

export default WorkspaceTabBar;
