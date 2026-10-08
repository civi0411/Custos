import React, { useState, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { 
  PanelLeft, 
  ChevronDown, 
  Plus, 
  Settings,
  GitBranch,
  Search,
  FolderGit2
} from 'lucide-react';

interface HeaderProps {
  currentProject: string;
  projectNames: string[];
  onSelectProject: (projectName: string) => void;
  onNewProjectPrompt: () => void;
  isSessionsCollapsed: boolean;
  onToggleSessions: () => void;
  onOpenSettings?: () => void;
}

export const Header: React.FC<HeaderProps> = ({
  currentProject,
  projectNames,
  onSelectProject,
  onNewProjectPrompt,
  isSessionsCollapsed,
  onToggleSessions,
  onOpenSettings
}) => {
  const [isProjectDropdownOpen, setIsProjectDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const navigate = useNavigate();

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setIsProjectDropdownOpen(false);
      }
    };
    document.addEventListener('click', handleClickOutside);
    return () => document.removeEventListener('click', handleClickOutside);
  }, []);

  return (
    <header className="h-10 bg-canvas border-b border-border-default flex items-center justify-between px-3 shrink-0 z-30 min-w-0 select-none font-sans text-xs">
      {/* 1. Left: Brand Icon + Traffic Spacing + Toggle Sidebar + Project Switcher + Branch */}
      <div className="flex items-center gap-2 min-w-0">
        {/* Brand Icon */}
        <div 
          onClick={() => navigate('/studio')}
          className="flex items-center gap-1.5 shrink-0 cursor-pointer group pr-0.5" 
          title="Custos AIDE — Return to Studio"
        >
          <img 
            src="/assets/custos-owl.png" 
            alt="Custos" 
            className="w-4 h-4 object-contain group-hover:scale-105 transition"
            onError={(e) => {
              (e.currentTarget as HTMLImageElement).src = '/assets/custos-logo.png';
            }}
          />
          <span className="font-semibold text-fg-editor tracking-tight hidden sm:inline text-[11.5px]">
            Custos
          </span>
        </div>

        <div className="h-3 w-[1px] bg-border-muted shrink-0"></div>

        {/* Toggle Sessions Sidebar */}
        <button 
          onClick={onToggleSessions} 
          className={`p-1 rounded-md transition shrink-0 ${
            isSessionsCollapsed 
              ? 'workbench-accent bg-surface-2' 
              : 'text-fg-muted hover:text-fg-editor hover:bg-surface-2'
          }`}
          title="Toggle Tasks Sidebar (⌘B)"
        >
          <PanelLeft className="w-3.5 h-3.5" />
        </button>

        {/* Project Selector */}
        <div className="relative min-w-0" ref={dropdownRef}>
          <button 
            onClick={(e) => {
              e.stopPropagation();
              setIsProjectDropdownOpen(!isProjectDropdownOpen);
            }} 
            className="flex items-center gap-1.5 px-2 py-1 rounded-md hover:bg-surface-2 text-[11px] font-medium text-fg-muted transition border border-transparent hover:border-border-default min-w-0"
          >
            <FolderGit2 className="w-3 h-3 text-fg-subtle shrink-0" />
            <span className="font-medium text-fg-editor truncate max-w-[130px] sm:max-w-[200px]">
              {currentProject}
            </span>
            <ChevronDown className="w-2.5 h-2.5 text-fg-subtle shrink-0" />
          </button>

          {isProjectDropdownOpen && (
            <div className="absolute left-0 mt-1.5 w-64 bg-surface-1 border border-border-default rounded-xl shadow-2xl p-1.5 text-xs z-50">
              <div className="px-2.5 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-fg-subtle flex items-center justify-between">
                <span>Switch Project</span>
                <span className="text-fg-muted font-mono">{projectNames.length} active</span>
              </div>

              {projectNames.map((proj) => {
                const isActive = proj === currentProject;
                return (
                  <div 
                    key={proj}
                    onClick={() => {
                      onSelectProject(proj);
                      setIsProjectDropdownOpen(false);
                    }} 
                    className={`flex items-center justify-between px-2.5 py-1.5 rounded-lg cursor-pointer transition ${
                      isActive 
                        ? 'bg-surface-2 text-fg-editor font-medium border border-border-default' 
                        : 'hover:bg-surface-2 text-fg-muted hover:text-fg-editor'
                    }`}
                  >
                    <div className="flex items-center gap-2 truncate">
                      <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${isActive ? 'bg-emerald-500' : 'bg-[var(--color-fg-subtle)]'}`}></span>
                      <span className={`truncate ${isActive ? 'font-medium' : ''}`}>{proj}</span>
                    </div>
                  </div>
                );
              })}

              <div className="h-[1px] bg-border-muted my-1"></div>
              
              <button 
                onClick={() => {
                  setIsProjectDropdownOpen(false);
                  onNewProjectPrompt();
                }} 
                className="w-full text-left px-2.5 py-1.5 rounded-lg hover:bg-surface-2 workbench-accent flex items-center gap-2 transition"
              >
                <Plus className="w-3.5 h-3.5" />
                <span className="font-medium">Create New Project</span>
              </button>
            </div>
          )}
        </div>

        {/* Git Branch Badge */}
        <div className="hidden md:flex items-center gap-1 px-1.5 py-0.5 rounded bg-surface-1 border border-border-muted text-[10.5px] font-mono text-fg-muted">
          <GitBranch className="w-3 h-3 text-emerald-500" />
          <span className="text-fg-editor">vi</span>
          <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse ml-0.5"></span>
        </div>
      </div>

      {/* 2. Center: Quick Search Bar / Omnibox Trigger */}
      <div 
        onClick={() => {
          window.dispatchEvent(new KeyboardEvent('keydown', { key: 'e', metaKey: true }));
        }}
        className="hidden lg:flex items-center gap-2 px-3 py-1 rounded-lg bg-surface-1 hover:bg-surface-2 border border-border-default text-fg-muted text-[11px] cursor-pointer transition w-64 max-w-sm justify-between group"
      >
        <div className="flex items-center gap-1.5">
          <Search className="w-3 h-3 text-fg-subtle group-hover:text-fg-editor" />
          <span className="truncate">Search files, worktrees...</span>
        </div>
        <kbd className="text-[9.5px] bg-surface-2 px-1.5 py-0.2 rounded border border-border-default text-fg-muted font-mono">⌘P</kbd>
      </div>

      {/* 3. Right: View Switchers & Preferences */}
      <div className="flex items-center gap-1 shrink-0">

        {/* Preferences / Settings */}
        <button 
          onClick={() => {
            if (onOpenSettings) onOpenSettings();
            else navigate('/settings');
          }} 
          className="p-1 rounded-md text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition flex items-center gap-1"
          title="Platform Preferences (⌘,)"
        >
          <Settings className="w-3.5 h-3.5" />
          <kbd className="text-[9px] bg-surface-1 px-1 py-0.2 rounded border border-border-muted font-mono text-fg-muted hidden sm:inline-block">⌘,</kbd>
        </button>
      </div>
    </header>
  );
};
