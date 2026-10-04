import React, { useState, useRef, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { 
  PanelLeft, 
  ChevronDown, 
  Plus, 
  Columns2, 
  MessageSquare, 
  FileDiff, 
  Settings,
  Check
} from 'lucide-react';
import { ViewMode } from '../types';
import { Tooltip } from './Tooltip';

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
  const isChatSessionTab = location.pathname === '/' || location.pathname.startsWith('/studio');

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
    <header className="h-12 bg-surface border-b border-surface-border flex items-center justify-between px-3 shrink-0 z-50 min-w-0 select-none">
      {/* Left Section: Brand Logo + Sidebar Toggle + Project Selector */}
      <div className="flex items-center gap-2 sm:gap-2.5 min-w-0">
        {/* Brand Logo */}
        <Tooltip content="Custos Studio" position="bottom" align="start">
          <div 
            onClick={() => navigate('/studio')}
            className="flex items-center gap-2 pr-1 shrink-0 group cursor-pointer" 
          >
            <img 
              src="/assets/custos-logo.png" 
              alt="Custos Logo" 
              className="h-7 sm:h-8 w-auto object-contain select-none"
              onError={(e) => {
                (e.currentTarget as HTMLImageElement).src = '/assets/custos.png';
              }}
            />
          </div>
        </Tooltip>

        {/* Toggle Sessions Sidebar Button (Only on Chat & Session tab) */}
        {isChatSessionTab && (
          <>
            <div className="h-4 w-[1px] bg-surface-border shrink-0 hidden sm:block"></div>
            <Tooltip content="Toggle Sessions Sidebar (Ctrl+B)" position="bottom" align="start">
              <button 
                onClick={onToggleSessions} 
                className={`p-1.5 rounded-lg transition shrink-0 ${
                  isSessionsCollapsed 
                    ? 'text-brand-blue bg-surface-elevated' 
                    : 'text-neutral-400 hover:text-white hover:bg-surface-elevated'
                }`}
              >
                <PanelLeft className="w-4 h-4" />
              </button>
            </Tooltip>
          </>
        )}

        {/* Project Selector Dropdown */}
        <div className="relative min-w-0" ref={dropdownRef}>
          <button 
            onClick={(e) => {
              e.stopPropagation();
              setIsProjectDropdownOpen(!isProjectDropdownOpen);
            }} 
            className={`flex items-center gap-1.5 sm:gap-2 px-2.5 py-1.5 rounded-lg text-xs font-medium transition border min-w-0 ${
              isProjectDropdownOpen 
                ? 'bg-surface-elevated border-surface-border text-white shadow-sm' 
                : 'hover:bg-surface-elevated text-neutral-200 border-surface-border/60 hover:border-surface-border'
            }`}
          >
            <div className="w-5 h-5 rounded-md bg-surface-card border border-surface-border flex items-center justify-center shrink-0 overflow-hidden shadow-inner">
              <img src="/assets/custos-owl.png" alt="Custos" className="w-3.5 h-3.5 object-contain" />
            </div>
            <span className="font-semibold text-neutral-100 tracking-tight truncate max-w-[110px] sm:max-w-[180px] md:max-w-none">
              {currentProject}
            </span>
            <ChevronDown className={`w-3.5 h-3.5 text-neutral-400 shrink-0 transition-transform duration-200 ${isProjectDropdownOpen ? 'rotate-180 text-brand-blue' : ''}`} />
          </button>

          {isProjectDropdownOpen && (
            <div className="absolute left-0 mt-2 w-64 bg-surface-card/95 backdrop-blur-md border border-surface-border rounded-xl shadow-2xl p-1.5 text-xs z-50 animate-in fade-in zoom-in-95 duration-100">
              <div className="px-2.5 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-neutral-400 flex items-center justify-between border-b border-surface-border/40 pb-1.5 mb-1">
                <span>Switch Project</span>
                <span className="text-neutral-500 font-mono bg-surface-elevated px-1.5 py-0.5 rounded text-[10px]">{projectNames.length} active</span>
              </div>

              <div className="space-y-0.5 max-h-56 overflow-y-auto">
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
                          ? 'bg-surface-elevated text-white font-medium shadow-sm' 
                          : 'hover:bg-surface-hover text-neutral-400 hover:text-white'
                      }`}
                    >
                      <div className="flex items-center gap-2 truncate">
                        <span className={`w-2 h-2 rounded-full shrink-0 ${isActive ? 'bg-emerald-400 shadow-sm shadow-emerald-400/50' : 'bg-neutral-600'}`}></span>
                        <span className="truncate">{proj}</span>
                      </div>
                      {isActive && <Check className="w-3.5 h-3.5 text-brand-blue shrink-0" />}
                    </div>
                  );
                })}
              </div>

              <div className="h-[1px] bg-surface-border/80 my-1"></div>
              
              <button 
                onClick={() => {
                  setIsProjectDropdownOpen(false);
                  onNewProjectPrompt();
                }} 
                className="w-full text-left px-2.5 py-2 rounded-lg hover:bg-brand-blue/10 text-brand-blue flex items-center gap-2 transition font-medium"
              >
                <Plus className="w-3.5 h-3.5" />
                <span>Create New Project</span>
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
        {/* Studio View Mode Selector (Only on Chat & Session tab) */}
        {isChatSessionTab && (
          <>
            <div className="flex items-center bg-surface-card p-0.5 rounded-lg border border-surface-border">
              <Tooltip content="Split View (Ctrl+\)" position="bottom">
                <button 
                  onClick={() => onSetViewMode('split')} 
                  className={`px-2 sm:px-2.5 py-1 rounded-md transition flex items-center gap-1 text-[10.5px] ${
                    viewMode === 'split' 
                      ? 'bg-surface-elevated text-white font-medium' 
                      : 'text-neutral-400 hover:text-white'
                  }`}
                >
                  <Columns2 className="w-3 h-3" />
                  <span className="hidden sm:inline">Split</span>
                </button>
              </Tooltip>
              <Tooltip content="Chat Only" position="bottom">
                <button 
                  onClick={() => onSetViewMode('chat')} 
                  className={`px-2 sm:px-2.5 py-1 rounded-md transition flex items-center gap-1 text-[10.5px] ${
                    viewMode === 'chat' 
                      ? 'bg-surface-elevated text-white font-medium' 
                      : 'text-neutral-400 hover:text-white'
                  }`}
                >
                  <MessageSquare className="w-3 h-3" />
                  <span className="hidden sm:inline">Chat</span>
                </button>
              </Tooltip>
              <Tooltip content="Code Diff Only" position="bottom">
                <button 
                  onClick={() => onSetViewMode('diff')} 
                  className={`px-2 sm:px-2.5 py-1 rounded-md transition flex items-center gap-1 text-[10.5px] ${
                    viewMode === 'diff' 
                      ? 'bg-surface-elevated text-white font-medium' 
                      : 'text-neutral-400 hover:text-white'
                  }`}
                >
                  <FileDiff className="w-3 h-3" />
                  <span className="hidden sm:inline">Code</span>
                </button>
              </Tooltip>
            </div>

            <div className="h-3 w-[1px] bg-surface-border shrink-0"></div>
          </>
        )}

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
          
          <Tooltip content="Settings (Ctrl+,)" position="bottom" align="end">
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
            >
              <Settings className="w-4 h-4" />
              <kbd className="text-[10px] bg-surface-card px-1.5 py-0.5 rounded border border-surface-border font-mono text-neutral-400 hidden sm:inline-block">Ctrl+,</kbd>
            </button>
          </Tooltip>
        </div>
      </div>
    </header>
  );
};
