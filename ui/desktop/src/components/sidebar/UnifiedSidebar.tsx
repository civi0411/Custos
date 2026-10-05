import React, { useState, useRef, useEffect } from 'react';
import { 
  MessageSquareCode, 
  KeyRound, 
  GitFork, 
  Activity, 
  Database, 
  SlidersHorizontal, 
  Plus, 
  Search, 
  PanelLeftClose, 
  PanelLeftOpen, 
  ChevronDown, 
  Terminal,
  BookOpen,
  Bot,
  Inbox,
  CheckCircle2,
  FolderGit2
} from 'lucide-react';
import { MainTab, Session } from '@/types';

interface UnifiedSidebarProps {
  currentTab: MainTab;
  onSwitchTab: (tab: MainTab) => void;
  currentProject: string;
  projectNames: string[];
  onSelectProject: (projectName: string) => void;
  onNewProjectPrompt: () => void;
  sessions: Session[];
  activeSessionId: string;
  onSelectSession: (id: string) => void;
  onOpenNewSessionModal: () => void;
  isCollapsed: boolean;
  onToggleCollapse: () => void;
  onOpenSettings?: () => void;
  onShowToast: (msg: string) => void;
}

interface NavItem {
  id: MainTab;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
}

const NAV_ITEMS: NavItem[] = [
  { id: 'studio', label: 'Studio & Workbenches', icon: MessageSquareCode },
  { id: 'providers', label: 'API Keys & Providers', icon: KeyRound },
  { id: 'chains', label: 'OmniRoute Chains', icon: GitFork },
  { id: 'telemetry', label: 'Logs & Telemetry', icon: Activity },
  { id: 'cache', label: 'Local KV Cache', icon: Database }
];

export const UnifiedSidebar: React.FC<UnifiedSidebarProps> = ({
  currentTab,
  onSwitchTab,
  currentProject,
  projectNames,
  onSelectProject,
  onNewProjectPrompt,
  sessions,
  activeSessionId,
  onSelectSession,
  onOpenNewSessionModal,
  isCollapsed,
  onToggleCollapse,
  onOpenSettings,
  onShowToast
}) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedPack, setSelectedPack] = useState<'all' | 'engineering' | 'research' | 'assistant'>('all');
  const [isProjectDropdownOpen, setIsProjectDropdownOpen] = useState(false);
  const projectDropdownRef = useRef<HTMLDivElement>(null);

  // Close project dropdown on outside click
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (projectDropdownRef.current && !projectDropdownRef.current.contains(e.target as Node)) {
        setIsProjectDropdownOpen(false);
      }
    };
    document.addEventListener('click', handleClickOutside);
    return () => document.removeEventListener('click', handleClickOutside);
  }, []);

  // Filter sessions by search query and pack filter
  const filteredSessions = sessions.filter((s) => {
    const matchesSearch = 
      s.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      s.preview.toLowerCase().includes(searchQuery.toLowerCase());
    
    if (!matchesSearch) return false;
    if (selectedPack === 'all') return true;

    // Detect pack from session title or metadata
    const title = s.title.toLowerCase();
    if (selectedPack === 'engineering') {
      return title.includes('runtime') || title.includes('code') || title.includes('audit') || title.includes('feat') || title.includes('fix');
    }
    if (selectedPack === 'research') {
      return title.includes('research') || title.includes('paper') || title.includes('arxiv') || title.includes('study');
    }
    if (selectedPack === 'assistant') {
      return title.includes('assistant') || title.includes('agent') || title.includes('fleet') || title.includes('prompt');
    }
    return true;
  });

  // -------------------------------------------------------------
  // 1. RENDER COLLAPSED (48px / w-12) ICON-ONLY RAIL (COMPACT MODE)
  // -------------------------------------------------------------
  if (isCollapsed) {
    return (
      <aside className="w-12 bg-[#0c0e14] border-r border-[#1c2130] flex flex-col items-center py-2 justify-between shrink-0 z-20 select-none transition-all duration-200">
        {/* Top: Expand Button + New Task Button */}
        <div className="flex flex-col items-center gap-2 w-full px-1">
          {/* Expand Sidebar Button */}
          <button
            onClick={onToggleCollapse}
            className="w-8 h-8 rounded-lg hover:bg-[#181d2c] text-neutral-400 hover:text-white flex items-center justify-center transition border border-transparent hover:border-[#232938]"
            title="Expand Custos Sidebar (⌘B)"
          >
            <PanelLeftOpen className="w-4 h-4" />
          </button>

          <div className="w-6 h-[1px] bg-[#1c2130] my-0.5" />

          {/* New Task / Session Button */}
          <button
            onClick={onOpenNewSessionModal}
            className="w-8 h-8 rounded-lg bg-[#141824] hover:bg-brand-blue/20 hover:border-brand-blue/50 text-neutral-300 hover:text-white flex items-center justify-center transition border border-[#232938]"
            title="New Custos Task (⌘N)"
          >
            <Plus className="w-4 h-4 text-brand-blue" />
          </button>

          <div className="w-6 h-[1px] bg-[#1c2130] my-0.5" />

          {/* Custos Module Nav Icons */}
          {NAV_ITEMS.map((item) => {
            const isSelected = currentTab === item.id;
            const Icon = item.icon;
            return (
              <div key={item.id} className="has-tooltip relative flex items-center justify-center">
                <button
                  onClick={() => onSwitchTab(item.id)}
                  className={`w-8 h-8 rounded-xl flex items-center justify-center transition ${
                    isSelected
                      ? 'bg-white text-black shadow-md border border-white font-semibold'
                      : 'text-neutral-400 hover:text-white hover:bg-[#181d2c] border border-transparent hover:border-[#232938]'
                  }`}
                >
                  <Icon className="w-4 h-4" />
                </button>
                <span className="tooltip-label absolute left-12 px-2 py-1 bg-[#181d2c] border border-[#232938] text-neutral-200 text-[11px] font-medium rounded-md whitespace-nowrap shadow-xl z-50 pointer-events-none">
                  {item.label}
                </span>
              </div>
            );
          })}
        </div>

        {/* Bottom: Settings & Brand Icon */}
        <div className="flex flex-col items-center gap-2">
          <button
            onClick={() => {
              if (onOpenSettings) onOpenSettings();
              else onSwitchTab('settings');
            }}
            className="w-8 h-8 rounded-lg text-neutral-400 hover:text-white hover:bg-[#181d2c] flex items-center justify-center transition border border-transparent hover:border-[#232938]"
            title="Preferences (⌘,)"
          >
            <SlidersHorizontal className="w-3.5 h-3.5" />
          </button>

          <div 
            onClick={onToggleCollapse}
            className="w-7 h-7 rounded-full bg-gradient-to-tr from-brand-blue to-purple-600 p-[1.5px] cursor-pointer"
            title="Custos SADE (Click to expand)"
          >
            <div className="w-full h-full rounded-full bg-[#141824] flex items-center justify-center text-[10px] font-bold text-white">
              C
            </div>
          </div>
        </div>
      </aside>
    );
  }

  // -------------------------------------------------------------
  // 2. RENDER EXPANDED (256px / w-64) CUSTOS SADE SESSIONS SIDEBAR
  // -------------------------------------------------------------
  return (
    <aside className="w-64 bg-[#0c0e14] border-r border-[#1c2130] flex flex-col shrink-0 z-20 select-none overflow-hidden transition-all duration-200 h-full font-sans text-xs">
      {/* 1. Header: Custos SADE Brand + New Task + Collapse Button */}
      <div className="h-10 px-3 flex items-center justify-between shrink-0 bg-[#090b10] border-b border-[#1c2130]">
        <div className="flex items-center gap-2">
          <div className="w-5 h-5 rounded-md bg-[#141824] border border-[#232938] flex items-center justify-center shrink-0 overflow-hidden">
            <img 
              src="/assets/custos-owl.png" 
              alt="Custos" 
              className="w-3.5 h-3.5 object-contain"
              onError={(e) => {
                (e.currentTarget as HTMLImageElement).src = '/assets/custos-logo.png';
              }} 
            />
          </div>
          <span className="font-semibold text-xs text-white tracking-tight">Custos SADE</span>
          <span className="text-[10px] font-mono text-neutral-500 bg-[#141824] px-1.5 py-0.5 rounded border border-[#232938]">v0.1</span>
        </div>

        <div className="flex items-center gap-1">
          <button
            onClick={onOpenNewSessionModal}
            className="p-1 rounded-md bg-[#141824] hover:bg-[#1a2030] text-brand-blue border border-[#232938] transition"
            title="Create Task / Session (⌘N)"
          >
            <Plus className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={onToggleCollapse}
            className="p-1 rounded-md text-neutral-400 hover:text-white hover:bg-[#1a2030] transition"
            title="Collapse Sidebar (⌘B)"
          >
            <PanelLeftClose className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* 2. Project Switcher Bar */}
      <div className="px-3 py-2 border-b border-[#1c2130] bg-[#0c0e14]" ref={projectDropdownRef}>
        <div className="relative">
          <button
            onClick={() => setIsProjectDropdownOpen(!isProjectDropdownOpen)}
            className="w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg bg-[#141824] border border-[#232938] hover:border-brand-blue/50 text-neutral-200 transition text-xs"
          >
            <div className="flex items-center gap-2 truncate">
              <FolderGit2 className="w-3.5 h-3.5 text-brand-blue shrink-0" />
              <span className="font-medium truncate">{currentProject}</span>
            </div>
            <ChevronDown className="w-3 h-3 text-neutral-500 shrink-0" />
          </button>

          {isProjectDropdownOpen && (
            <div className="absolute left-0 mt-1 w-full bg-[#0c0e14] border border-[#232938] rounded-xl shadow-2xl p-1 text-xs z-50">
              <div className="px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-neutral-500">
                Switch Project
              </div>
              {projectNames.map((proj) => {
                const isActive = proj === currentProject;
                return (
                  <div
                    key={proj}
                    onClick={() => {
                      onSelectProject(proj);
                      setIsProjectDropdownOpen(false);
                      onShowToast(`Switched project to ${proj}`);
                    }}
                    className={`flex items-center justify-between px-2 py-1 rounded-lg cursor-pointer transition ${
                      isActive 
                        ? 'bg-[#1a2030] text-white font-medium' 
                        : 'text-neutral-400 hover:text-white hover:bg-[#141824]'
                    }`}
                  >
                    <span className="truncate">{proj}</span>
                    {isActive && <CheckCircle2 className="w-3 h-3 text-emerald-400 shrink-0" />}
                  </div>
                );
              })}
              <div className="h-[1px] bg-[#1c2130] my-1" />
              <button
                onClick={() => {
                  setIsProjectDropdownOpen(false);
                  onNewProjectPrompt();
                }}
                className="w-full text-left px-2 py-1 rounded-lg hover:bg-[#141824] text-brand-blue flex items-center gap-1.5 transition text-[11px]"
              >
                <Plus className="w-3 h-3" />
                <span>New Project...</span>
              </button>
            </div>
          )}
        </div>
      </div>

      {/* 3. Custos 3-Packs Filter Chips (Coding / Research / Assistant) */}
      <div className="px-2 py-1.5 border-b border-[#1c2130] bg-[#0a0c12]">
        <div className="grid grid-cols-4 gap-1 text-[10.5px] font-medium text-center">
          <button
            onClick={() => setSelectedPack('all')}
            className={`py-1 rounded-md transition ${
              selectedPack === 'all'
                ? 'bg-[#1c2234] text-white font-semibold shadow-sm'
                : 'text-neutral-400 hover:text-white hover:bg-[#141824]'
            }`}
          >
            All
          </button>
          <button
            onClick={() => setSelectedPack('engineering')}
            className={`py-1 rounded-md flex items-center justify-center gap-1 transition ${
              selectedPack === 'engineering'
                ? 'bg-emerald-500/15 text-emerald-300 font-semibold border border-emerald-500/30'
                : 'text-neutral-400 hover:text-white hover:bg-[#141824]'
            }`}
            title="Coding Pack"
          >
            <Terminal className="w-3 h-3 shrink-0" />
            <span>Code</span>
          </button>
          <button
            onClick={() => setSelectedPack('research')}
            className={`py-1 rounded-md flex items-center justify-center gap-1 transition ${
              selectedPack === 'research'
                ? 'bg-purple-500/15 text-purple-300 font-semibold border border-purple-500/30'
                : 'text-neutral-400 hover:text-white hover:bg-[#141824]'
            }`}
            title="Research Pack"
          >
            <BookOpen className="w-3 h-3 shrink-0" />
            <span>Res</span>
          </button>
          <button
            onClick={() => setSelectedPack('assistant')}
            className={`py-1 rounded-md flex items-center justify-center gap-1 transition ${
              selectedPack === 'assistant'
                ? 'bg-amber-500/15 text-amber-300 font-semibold border border-amber-500/30'
                : 'text-neutral-400 hover:text-white hover:bg-[#141824]'
            }`}
            title="Assistant Pack"
          >
            <Bot className="w-3 h-3 shrink-0" />
            <span>Asst</span>
          </button>
        </div>
      </div>

      {/* 4. Search Bar */}
      <div className="px-3 py-1.5 border-b border-[#1c2130]">
        <div className="relative">
          <Search className="w-3 h-3 absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-500" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search tasks and conversations..."
            className="w-full bg-[#141824] border border-[#232938] rounded-lg pl-7 pr-2 py-1 text-xs text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-brand-blue transition font-sans"
          />
        </div>
      </div>

      {/* 5. Scrollable Real Custos Sessions / Tasks List */}
      <div className="flex-1 overflow-y-auto p-2 space-y-1.5 min-w-0">
        {filteredSessions.length === 0 ? (
          <div className="p-6 text-center text-neutral-500 text-xs">
            <Inbox className="w-6 h-6 mx-auto mb-2 opacity-50" />
            <p>No tasks found</p>
          </div>
        ) : (
          filteredSessions.map((session) => {
            const isActive = session.id === activeSessionId;
            return (
              <div
                key={session.id}
                onClick={() => onSelectSession(session.id)}
                className={`p-2.5 rounded-xl cursor-pointer transition border min-w-0 ${
                  isActive
                    ? 'bg-[#191e2c] border-brand-blue/60 text-white shadow-md'
                    : 'bg-[#121520] hover:bg-[#161a28] border-[#1e2332] text-neutral-300'
                }`}
              >
                <div className="flex items-center justify-between mb-1 min-w-0">
                  <span className={`font-semibold truncate text-xs ${isActive ? 'text-white' : 'text-neutral-200'}`}>
                    {session.title}
                  </span>
                  <span className="text-[10px] text-neutral-500 font-mono shrink-0 ml-1.5 bg-[#090b10] px-1 py-0.2 rounded border border-[#1e2332]">
                    {session.taskStatus || (session.source === 'demo' ? 'Demo' : session.time)}
                  </span>
                </div>

                <p className="text-[11px] text-neutral-400 truncate leading-snug">
                  {session.preview}
                </p>

                <div className="flex items-center justify-between mt-2 pt-1.5 border-t border-[#1e2332]/60 text-[10px] text-neutral-500 font-mono">
                  <span className="truncate max-w-[120px] text-neutral-400">{session.model}</span>
                  {session.diffLinesCount && (
                    <span className="text-emerald-400 font-medium shrink-0 bg-emerald-500/10 px-1 rounded">
                      {session.diffLinesCount}
                    </span>
                  )}
                </div>
              </div>
            );
          })
        )}
      </div>

      {/* 6. Custos Modules Footer & Provenance */}
      <div className="border-t border-[#1c2130] bg-[#090b10] p-2 space-y-1">
        <div className="flex items-center justify-between px-1 text-[10.5px] font-mono text-neutral-400">
          <div className="flex items-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
            <span>Zero-IO Invariant Active</span>
          </div>
          <button
            onClick={() => {
              if (onOpenSettings) onOpenSettings();
              else onSwitchTab('settings');
            }}
            className="p-1 rounded hover:bg-[#1a2030] text-neutral-400 hover:text-white transition"
            title="Settings (⌘,)"
          >
            <SlidersHorizontal className="w-3 h-3" />
          </button>
        </div>

        <div className="grid grid-cols-4 gap-1 text-[10px] font-medium text-center pt-0.5 text-neutral-400">
          <button
            onClick={() => onSwitchTab('providers')}
            className="py-0.5 rounded hover:bg-[#141824] hover:text-neutral-200 transition truncate"
            title="API Keys & Gateways"
          >
            Providers
          </button>
          <button
            onClick={() => onSwitchTab('chains')}
            className="py-0.5 rounded hover:bg-[#141824] hover:text-neutral-200 transition truncate"
            title="OmniRoute Cascade"
          >
            OmniRoute
          </button>
          <button
            onClick={() => onSwitchTab('telemetry')}
            className="py-0.5 rounded hover:bg-[#141824] hover:text-neutral-200 transition truncate"
            title="Logs & Receipts"
          >
            Logs
          </button>
          <button
            onClick={() => onSwitchTab('cache')}
            className="py-0.5 rounded hover:bg-[#141824] hover:text-neutral-200 transition truncate"
            title="Local KV Cache"
          >
            Cache
          </button>
        </div>
      </div>
    </aside>
  );
};

export default UnifiedSidebar;
