import React, { useState, useRef, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { 
  PanelLeft, 
  ChevronDown, 
  Plus, 
  Columns2, 
  MessageSquare, 
  FileDiff, 
  Settings 
} from 'lucide-react';
import { ViewMode } from '../types';

interface HeaderProps {
  currentProject: string;
  projectNames: string[];
  onSelectProject: (projectName: string) => void;
  onNewProjectPrompt: () => void;
  isSessionsCollapsed: boolean;
  onToggleSessions: () => void;
  viewMode: ViewMode;
  onSetViewMode: (mode: ViewMode) => void;
  onOpenSettings?: () => void;
  onShowToast?: (msg: string) => void;
}

export const Header: React.FC<HeaderProps> = ({
  currentProject,
  projectNames,
  onSelectProject,
  onNewProjectPrompt,
  isSessionsCollapsed,
  onToggleSessions,
  viewMode,
  onSetViewMode,
  onOpenSettings,
  onShowToast: _onShowToast
}) => {
  const [isProjectDropdownOpen, setIsProjectDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const navigate = useNavigate();
  const location = useLocation();

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
    <header className="h-12 bg-surface border-b border-surface-border flex items-center justify-between px-3 shrink-0 z-30 min-w-0 select-none">
      {/* Left Section: Brand Logo + Sidebar Toggle + Project Selector */}
      <div className="flex items-center gap-2 sm:gap-2.5 min-w-0">
        {/* Brand Logo */}
        <div 
          onClick={() => navigate('/studio')}
          className="flex items-center gap-2 pr-1 shrink-0 group cursor-pointer" 
          title="Custos Platform - Return to Studio"
        >
          <img 
            src="/assets/custos-logo.png" 
            alt="Custos Logo" 
            className="h-7 sm:h-8 w-auto object-contain select-none transition-all duration-200 group-hover:scale-105 group-hover:brightness-110 filter drop-shadow-[0_0_12px_rgba(0,112,243,0.35)]"
            onError={(e) => {
              (e.currentTarget as HTMLImageElement).src = '/assets/custos.png';
            }}
          />
        </div>

        <div className="h-4 w-[1px] bg-surface-border shrink-0 hidden sm:block"></div>

        {/* Toggle Sessions Sidebar Button */}
        <button 
          onClick={onToggleSessions} 
          className={`p-1.5 rounded-lg transition shrink-0 ${
            isSessionsCollapsed 
              ? 'text-brand-blue bg-surface-elevated' 
              : 'text-neutral-400 hover:text-white hover:bg-surface-elevated'
          }`}
          title="Toggle Sessions Sidebar (⌘B)"
        >
          <PanelLeft className="w-4 h-4" />
        </button>

        {/* Project Selector Dropdown */}
        <div className="relative min-w-0" ref={dropdownRef}>
          <button 
            onClick={(e) => {
              e.stopPropagation();
              setIsProjectDropdownOpen(!isProjectDropdownOpen);
            }} 
            className="flex items-center gap-1.5 sm:gap-2 px-2 py-1.5 rounded-lg hover:bg-surface-elevated text-xs font-medium text-neutral-200 transition border border-surface-border/60 hover:border-surface-border min-w-0"
          >
            <div className="w-5 h-5 rounded-md bg-surface-elevated border border-surface-border flex items-center justify-center shrink-0 overflow-hidden">
              <img src="/assets/custos-owl.png" alt="Custos" className="w-3.5 h-3.5 object-contain" />
            </div>
            <span className="font-semibold text-neutral-100 tracking-tight truncate max-w-[110px] sm:max-w-[180px] md:max-w-none">
              {currentProject}
            </span>
            <ChevronDown className="w-3 h-3 text-neutral-500 shrink-0" />
          </button>

          {isProjectDropdownOpen && (
            <div className="absolute left-0 mt-1.5 w-64 bg-surface-card border border-surface-border rounded-xl shadow-2xl p-1.5 text-xs z-50">
              <div className="px-2.5 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-neutral-500 flex items-center justify-between">
                <span>Switch Project</span>
                <span className="text-neutral-600 font-mono">{projectNames.length} active</span>
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
                    className={`flex items-center justify-between px-2.5 py-2 rounded-lg cursor-pointer transition ${
                      isActive 
                        ? 'bg-surface-elevated text-white' 
                        : 'hover:bg-surface-hover text-neutral-400 hover:text-white'
                    }`}
                  >
                    <div className="flex items-center gap-2 truncate">
                      <span className={`w-2 h-2 rounded-full shrink-0 ${isActive ? 'bg-emerald-500' : 'bg-neutral-600'}`}></span>
                      <span className={`truncate ${isActive ? 'font-medium' : ''}`}>{proj}</span>
                    </div>
                  </div>
                );
              })}

              <div className="h-[1px] bg-surface-border my-1"></div>
              
              <button 
                onClick={() => {
                  setIsProjectDropdownOpen(false);
                  onNewProjectPrompt();
                }} 
                className="w-full text-left px-2.5 py-1.5 rounded-lg hover:bg-surface-hover text-brand-blue flex items-center gap-2 transition"
              >
                <Plus className="w-3.5 h-3.5" />
                <span className="font-medium">Create New Project</span>
              </button>
            </div>
          )}
        </div>

        <div className="h-3 w-[1px] bg-surface-border hidden sm:block shrink-0"></div>

        {/* OmniRoute Status Pill */}
        <div className="hidden md:flex items-center gap-2 bg-surface-card px-2.5 py-1 rounded-md border border-surface-border text-[11px] shrink-0">
          <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse shrink-0"></span>
          <span className="text-neutral-400 font-mono">OmniRoute:</span>
          <span className="text-neutral-200 font-medium font-mono truncate max-w-[200px] xl:max-w-none">Claude 3.7 &rarr; Gemini 2.5</span>
        </div>
      </div>

      {/* Center/Right: View Mode Selector & Links */}
      <div className="flex items-center gap-2 shrink-0">
        {/* Studio View Mode Selector */}
        <div className="flex items-center bg-surface-card p-0.5 rounded-lg border border-surface-border">
          <button 
            onClick={() => onSetViewMode('split')} 
            className={`px-2 sm:px-2.5 py-1 rounded-md transition flex items-center gap-1 text-[10.5px] ${
              viewMode === 'split' 
                ? 'bg-surface-elevated text-white font-medium' 
                : 'text-neutral-400 hover:text-white'
            }`}
            title="Split View (⌘\)"
          >
            <Columns2 className="w-3 h-3" />
            <span className="hidden sm:inline">Split</span>
          </button>
          <button 
            onClick={() => onSetViewMode('chat')} 
            className={`px-2 sm:px-2.5 py-1 rounded-md transition flex items-center gap-1 text-[10.5px] ${
              viewMode === 'chat' 
                ? 'bg-surface-elevated text-white font-medium' 
                : 'text-neutral-400 hover:text-white'
            }`}
            title="Chat Only"
          >
            <MessageSquare className="w-3 h-3" />
            <span className="hidden sm:inline">Chat</span>
          </button>
          <button 
            onClick={() => onSetViewMode('diff')} 
            className={`px-2 sm:px-2.5 py-1 rounded-md transition flex items-center gap-1 text-[10.5px] ${
              viewMode === 'diff' 
                ? 'bg-surface-elevated text-white font-medium' 
                : 'text-neutral-400 hover:text-white'
            }`}
            title="Code Diff Only"
          >
            <FileDiff className="w-3 h-3" />
            <span className="hidden sm:inline">Code</span>
          </button>
        </div>

        <div className="h-3 w-[1px] bg-surface-border shrink-0"></div>

        {/* Links & Settings */}
        <div className="flex items-center gap-1 sm:gap-2 shrink-0">
          <button 
            onClick={() => navigate('/dashboard')} 
            className={`text-[11px] font-medium transition px-2 py-1 rounded-md hidden lg:inline-block ${
              location.pathname === '/dashboard'
                ? 'bg-surface-elevated text-white'
                : 'text-neutral-400 hover:text-white hover:bg-surface-elevated'
            }`}
          >
            Dashboard
          </button>
          <button 
            onClick={() => navigate('/docs')} 
            className={`text-[11px] font-medium transition px-2 py-1 rounded-md hidden lg:inline-block ${
              location.pathname === '/docs'
                ? 'bg-surface-elevated text-white'
                : 'text-neutral-400 hover:text-white hover:bg-surface-elevated'
            }`}
          >
            API Docs
          </button>
          
          <button 
            onClick={() => {
              if (onOpenSettings) onOpenSettings();
              else navigate('/settings');
            }} 
            className={`p-1.5 rounded-lg transition flex items-center gap-1.5 ${
              location.pathname === '/settings'
                ? 'bg-surface-elevated text-white'
                : 'text-neutral-400 hover:text-white hover:bg-surface-elevated'
            }`} 
            title="Settings (⌘,)"
          >
            <Settings className="w-4 h-4" />
            <kbd className="text-[10px] bg-surface-card px-1 py-0.5 rounded border border-surface-border font-mono text-neutral-500 hidden sm:inline-block">⌘,</kbd>
          </button>
        </div>
      </div>
    </header>
  );
};
