import React, { useState, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  SlidersHorizontal,
  ChevronDown,
  LayoutGrid,
  Settings,
  ShieldCheck,
  MessageSquare,
  Code2,
  FlaskConical,
  X
} from 'lucide-react';
import { Session } from '@/types';

export interface SidebarTool {
  icon: React.ElementType;
  label: string;
  onClick: () => void;
  rightElement?: React.ReactNode;
  iconColor?: string;
}

export interface WorkspaceSidebarProps {
  currentProject: string;
  projectNames: string[];
  onSelectProject: (name: string) => void;
  sessions: Session[];
  activeSessionId: string;
  onSelectSession: (id: string) => void;

  // Header button
  newTaskLabel: string;
  newTaskIcon: React.ElementType;
  newTaskIconColor?: string;
  onNewSession: () => void;

  // Nav Tools
  tools: SidebarTool[];
  
  // Lists
  sessionsTitle: string;
  emptySessionsMessage: string;
  projectActiveColor?: string;

  // Footer Avatar
  avatarColor: string;
  avatarText: string;
  avatarTextColor?: string;

  // Callbacks
  onShowToast?: (msg: string) => void;
  onOpenSettings?: () => void;
  onOpenAppGrid?: () => void;
  onSwitchMode?: (mode: 'chat' | 'code' | 'research') => void;
  isSidebarCollapsed?: boolean;
}

export const WorkspaceSidebar: React.FC<WorkspaceSidebarProps> = ({
  currentProject,
  projectNames,
  onSelectProject,
  sessions,
  activeSessionId,
  onSelectSession,
  newTaskLabel,
  newTaskIcon: NewTaskIcon,
  newTaskIconColor = 'text-[#9e9e9e]',
  onNewSession,
  tools,
  sessionsTitle,
  emptySessionsMessage,
  projectActiveColor = 'bg-[#cc785c]',
  avatarColor,
  avatarText,
  avatarTextColor = 'text-white',
  onShowToast,
  onOpenSettings,
  onOpenAppGrid: _onOpenAppGrid,
  onSwitchMode,
  isSidebarCollapsed
}) => {
  const navigate = useNavigate();
  const [isProfileDropdownOpen, setIsProfileDropdownOpen] = useState(false);
  const [isAppGridOpen, setIsAppGridOpen] = useState(false);

  const profileRef = useRef<HTMLDivElement>(null);
  const appGridRef = useRef<HTMLDivElement>(null);

  // Close popovers on outside click
  useEffect(() => {
    const handleOutsideClick = (e: MouseEvent) => {
      const target = e.target as Node;
      if (profileRef.current && !profileRef.current.contains(target)) {
        setIsProfileDropdownOpen(false);
      }
      if (appGridRef.current && !appGridRef.current.contains(target)) {
        setIsAppGridOpen(false);
      }
    };
    document.addEventListener('mousedown', handleOutsideClick);
    return () => document.removeEventListener('mousedown', handleOutsideClick);
  }, []);

  const apps = [
    {
      title: 'Claude Chat',
      desc: 'Cowork & conversation',
      icon: MessageSquare,
      color: 'text-[#cc785c]',
      bg: 'bg-[#cc785c]/10',
      action: () => {
        onSwitchMode ? onSwitchMode('chat') : navigate('/studio');
        setIsAppGridOpen(false);
      }
    },
    {
      title: 'Codex ADE',
      desc: 'Orca engineering copilot',
      icon: Code2,
      color: 'text-[#58a6ff]',
      bg: 'bg-[#58a6ff]/10',
      action: () => {
        onSwitchMode ? onSwitchMode('code') : navigate('/studio');
        setIsAppGridOpen(false);
      }
    },
    {
      title: 'Science Lab',
      desc: 'Literature & claim evidence',
      icon: FlaskConical,
      color: 'text-[#a371f7]',
      bg: 'bg-[#a371f7]/10',
      action: () => {
        onSwitchMode ? onSwitchMode('research') : navigate('/studio');
        setIsAppGridOpen(false);
      }
    },
    {
      title: 'Preferences & Settings',
      desc: 'API keys, models, security & cache',
      icon: Settings,
      color: 'text-[#8b949e]',
      bg: 'bg-[#30363d]/50',
      action: () => {
        if (onOpenSettings) onOpenSettings();
        else navigate('/settings');
        setIsAppGridOpen(false);
      }
    }
  ];

  if (isSidebarCollapsed) return null;

  return (
    <aside className="w-64 bg-[#161b22] border-r border-[#262c36] flex flex-col shrink-0 z-20 relative">
      <div className="flex flex-col min-h-0 flex-1 p-3">
        {sessions.some((session) => session.source === 'demo') && (
          <div className="mb-3 rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-[10px] leading-snug text-amber-200">
            Demo data · actions are not connected to the live daemon
          </div>
        )}
        {/* + New Button */}
        <button
          onClick={onNewSession}
          className="w-full flex items-center gap-2 px-3 py-2 rounded-xl bg-[#21262d] hover:bg-[#2d333b] text-white text-[13px] font-medium transition shadow-sm border border-[#262c36]/60 mb-3 group"
        >
          <NewTaskIcon className={`w-4 h-4 ${newTaskIconColor} group-hover:text-white transition`} />
          <span>{newTaskLabel}</span>
        </button>

        {/* Tools Nav Links */}
        {tools.length > 0 && (
          <nav className="space-y-0.5 mb-4 border-b border-[#262c36]/60 pb-4">
            {tools.map((tool, idx) => {
              const Icon = tool.icon;
              return (
                <button
                  key={idx}
                  onClick={tool.onClick}
                  className="w-full flex items-center justify-between px-3 py-1.5 rounded-lg text-[13px] text-[#b3b3b3] hover:text-white hover:bg-[#262626] transition"
                >
                  <div className="flex items-center gap-2.5">
                    <Icon className={`w-4 h-4 ${tool.iconColor || 'text-[#8a8a8a]'}`} />
                    <span>{tool.label}</span>
                  </div>
                  {tool.rightElement && tool.rightElement}
                </button>
              );
            })}
          </nav>
        )}

        {/* Section: Sessions */}
        <div className="flex items-center justify-between px-3 py-1 text-[11px] font-medium text-[#7d7d7d] mb-1">
          <span>{sessionsTitle}</span>
          <button
            onClick={() => { if (onShowToast) onShowToast('Filter sessions'); }}
            className="p-1 hover:text-[#b3b3b3] transition rounded"
            title="Filter"
          >
            <SlidersHorizontal className="w-3.5 h-3.5" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto overflow-x-hidden space-y-0.5 pr-1 no-scrollbar min-h-0 text-[12.5px]">
          {sessions.length === 0 ? (
            <div className="px-3 py-4 text-center text-[#5a5a5a] text-[12px]">
              {emptySessionsMessage}
            </div>
          ) : (
            sessions.map((s) => {
              const isActive = s.id === activeSessionId;
              const packColor =
                s.pack === 'engineering' || s.pack === 'coding' ? '#3fb950' :
                s.pack === 'research' ? '#a371f7' :
                s.pack === 'assistant' ? '#cc785c' : '#58a6ff';
              return (
                <div
                  key={s.id}
                  onClick={() => onSelectSession(s.id)}
                  className={`flex items-center gap-2.5 px-3 py-1.5 rounded-lg cursor-pointer transition group ${
                    isActive
                      ? 'bg-[#21262d] text-white font-medium'
                      : 'text-[#8b949e] hover:text-[#e6edf3] hover:bg-[#1c2128]'
                  }`}
                >
                  <span
                    className="w-1.5 h-1.5 rounded-full shrink-0"
                    style={{ background: isActive ? packColor : '#555555' }}
                  />
                  <span className="truncate">{s.title}</span>
                  {s.taskStatus && s.taskStatus !== 'draft' && (
                    <span
                      className="shrink-0 text-[9.5px] px-1 py-0.2 rounded font-mono hidden group-hover:block"
                      style={{ background: '#1c1c1c', color: packColor, border: `1px solid ${packColor}33` }}
                    >
                      {s.taskStatus}
                    </span>
                  )}
                </div>
              );
            })
          )}
        </div>

        {/* Project switcher */}
        {projectNames.length > 1 && (
          <div className="mt-3 pt-3 border-t border-[#262c36]/60">
            <div className="text-[10px] font-semibold text-[#5a5a5a] px-2 mb-1">Switch Project</div>
            {projectNames.map((pName) => (
              <button
                key={pName}
                onClick={() => onSelectProject(pName)}
                className={`w-full flex items-center gap-2 px-2.5 py-1 rounded-lg text-[11.5px] text-left transition ${
                  pName === currentProject
                    ? 'bg-[#21262d] text-white font-medium'
                    : 'text-[#7d7d7d] hover:text-[#e0e0e0] hover:bg-[#1c2128]'
                }`}
              >
                <span className={`w-1.5 h-1.5 rounded-full ${pName === currentProject ? projectActiveColor : 'bg-neutral-600'}`} />
                <span className="truncate">{pName}</span>
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Bottom Profile Footer */}
      <div className="p-3 border-t border-[#262c36]/60 flex items-center justify-between relative shrink-0">
        <div ref={profileRef} className="relative">
          <div
            onClick={() => {
              setIsProfileDropdownOpen(!isProfileDropdownOpen);
              setIsAppGridOpen(false);
            }}
            className="flex items-center gap-2 cursor-pointer hover:opacity-85 transition"
          >
            <div className={`w-7 h-7 rounded-full ${avatarColor} ${avatarTextColor} font-bold text-[11px] flex items-center justify-center`}>
              {avatarText}
            </div>
            <div className="flex items-center gap-1">
              <span className="text-[13px] font-medium text-[#e0e0e0]">Chí Vĩ</span>
              <ChevronDown className={`w-3 h-3 text-[#7d7d7d] transition-transform ${isProfileDropdownOpen ? 'rotate-180' : ''}`} />
            </div>
          </div>

          {/* Profile Popover Menu */}
          {isProfileDropdownOpen && (
            <div className="absolute left-0 bottom-full mb-2 w-64 rounded-2xl bg-[#1c2128] border border-[#30363d] shadow-2xl p-2 z-50 animate-in fade-in zoom-in-95 duration-100">
              {/* User Header */}
              <div className="p-2 border-b border-[#30363d]/70 mb-1.5">
                <div className="flex items-center gap-2.5">
                  <div className={`w-8 h-8 rounded-full ${avatarColor} ${avatarTextColor} font-bold text-xs flex items-center justify-center shrink-0`}>
                    {avatarText}
                  </div>
                  <div className="min-w-0">
                    <div className="text-xs font-semibold text-white truncate">Chí Vĩ</div>
                    <div className="text-[11px] text-[#8b949e] truncate">Lead Architect • Sovereign</div>
                  </div>
                </div>
                <div className="mt-2 flex items-center gap-1.5 px-2 py-1 rounded-lg bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 text-[10.5px]">
                  <ShieldCheck className="w-3.5 h-3.5 shrink-0" />
                  <span className="truncate">Zero-IO Strict Barrier Active</span>
                </div>
              </div>

              {/* Action Links */}
              <div className="space-y-0.5 text-xs">
                <button
                  onClick={() => {
                    setIsProfileDropdownOpen(false);
                    onOpenSettings ? onOpenSettings() : navigate('/settings');
                  }}
                  className="w-full flex items-center gap-2.5 px-2.5 py-1.5 rounded-lg text-[#c9d1d9] hover:text-white hover:bg-[#2d333b] transition"
                >
                  <Settings className="w-3.5 h-3.5 text-[#8b949e]" />
                  <span>Preferences & Settings (⌘,)</span>
                </button>
              </div>

              {/* Footer status */}
              <div className="mt-2 pt-2 border-t border-[#30363d]/70 px-2.5 py-1 flex items-center justify-between text-[10.5px] text-[#8b949e]">
                <span>Daemon 127.0.0.1:4140</span>
                <span className="flex items-center gap-1 text-emerald-400 font-mono">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                  online
                </span>
              </div>
            </div>
          )}
        </div>
        
        {/* App Grid Launcher */}
        <div ref={appGridRef} className="relative">
          <button
            onClick={() => {
              setIsAppGridOpen(!isAppGridOpen);
              setIsProfileDropdownOpen(false);
            }}
            className={`p-1.5 rounded-lg transition ${
              isAppGridOpen
                ? 'bg-[#2b2b2b] text-white shadow-sm'
                : 'text-[#7d7d7d] hover:text-white hover:bg-[#2b2b2b]'
            }`}
            title="Custos App Launcher"
          >
            <LayoutGrid className="w-4 h-4" />
          </button>

          {/* App Grid Popover */}
          {isAppGridOpen && (
            <div className="absolute right-0 bottom-full mb-2 w-72 rounded-2xl bg-[#1c2128] border border-[#30363d] shadow-2xl p-3 z-50 animate-in fade-in zoom-in-95 duration-100">
              <div className="flex items-center justify-between pb-2 mb-2 border-b border-[#30363d]/70">
                <div className="flex items-center gap-1.5 text-xs font-semibold text-white">
                  <LayoutGrid className="w-3.5 h-3.5 text-[#58a6ff]" />
                  <span>Custos SADE Suite</span>
                </div>
                <button
                  onClick={() => setIsAppGridOpen(false)}
                  className="p-1 text-[#8b949e] hover:text-white rounded"
                >
                  <X className="w-3.5 h-3.5" />
                </button>
              </div>

              {/* Apps 3x3 Grid */}
              <div className="grid grid-cols-3 gap-2">
                {apps.map((app) => {
                  const Icon = app.icon;
                  return (
                    <div
                      key={app.title}
                      onClick={app.action}
                      className="p-2 rounded-xl bg-[#22272e] hover:bg-[#2d333b] border border-transparent hover:border-[#444c56] cursor-pointer transition flex flex-col items-center text-center group"
                    >
                      <div className={`w-8 h-8 rounded-lg ${app.bg} flex items-center justify-center mb-1.5 group-hover:scale-105 transition-transform`}>
                        <Icon className={`w-4 h-4 ${app.color}`} />
                      </div>
                      <span className="text-[11px] font-medium text-white truncate w-full">{app.title}</span>
                      <span className="text-[9.5px] text-[#8b949e] truncate w-full mt-0.5">{app.desc}</span>
                    </div>
                  );
                })}
              </div>

              <div className="mt-2.5 pt-2 border-t border-[#30363d]/70 text-center">
                <span className="text-[10px] text-[#8b949e]">Custos Sovereign Workbenches</span>
              </div>
            </div>
          )}
        </div>
      </div>
    </aside>
  );
};
