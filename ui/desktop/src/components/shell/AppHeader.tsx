import React from 'react';
import {
  MessageSquare,
  Code2,
  FlaskConical,
  ArrowLeft,
  ArrowRight,
  PanelLeft,
  PanelRight,
  PlusSquare,
  Maximize2
} from 'lucide-react';

import { formatKeyCombo } from '@/lib/utils';

// 3 primary workspace modes
export type AppWorkspaceMode = 'chat' | 'code' | 'research';

interface AppHeaderProps {
  mode: AppWorkspaceMode;
  workspaceTitle?: string;
  onSwitchMode: (mode: AppWorkspaceMode) => void;

  isSidebarCollapsed: boolean;
  onToggleSidebar: () => void;
  onShowToast?: (msg: string) => void;
  onNewTab?: () => void;
  onNewTabFullView?: () => void;
  isWebTabOpen?: boolean;
  onToggleWebTab?: () => void;
  webTabCount?: number;
}

export const AppHeader: React.FC<AppHeaderProps> = ({
  mode,
  workspaceTitle,
  onSwitchMode,
  isSidebarCollapsed: _isSidebarCollapsed,
  onToggleSidebar,
  onShowToast,
  onNewTab,
  onNewTabFullView,
  isWebTabOpen,
  onToggleWebTab,
  webTabCount = 0
}) => {

  const modes: { id: AppWorkspaceMode; label: string; icon: React.ElementType; color: string; shortcut: string; description: string }[] = [
    {
      id: 'chat',
      label: 'Chat',
      icon: MessageSquare,
      color: '#e6edf3',
      shortcut: `${formatKeyCombo({ ctrlOrCmd: true, key: "1" })}`,
      description: 'Custos Copilot workspace'
    },
    {
      id: 'code',
      label: 'Code',
      icon: Code2,
      color: '#e6edf3',
      shortcut: `${formatKeyCombo({ ctrlOrCmd: true, key: "2" })}`,
      description: 'Custos Coding workspace'
    },
    {
      id: 'research',
      label: 'Research',
      icon: FlaskConical,
      color: '#e6edf3',
      shortcut: `${formatKeyCombo({ ctrlOrCmd: true, key: "3" })}`,
      description: 'Custos Research workspace'
    }
  ];

  return (
    <header className="h-10 bg-[#090d13] flex items-center select-none z-30 shrink-0 font-sans text-xs transition-all border-b border-[#262c36]/60">
      {/* ── LEFT: Nav Controls + Sidebar + Mode Switcher ── */}
      <div className={`flex items-center justify-between shrink-0 h-full transition-all duration-300 ${!_isSidebarCollapsed ? 'w-64 pr-3 border-r border-[#262c36] bg-[#161b22]' : 'w-auto pr-3 bg-transparent border-transparent'}`}>
        <div className="flex items-center gap-1.5 pl-3">
          {/* Nav arrows */}
          <div className="flex items-center gap-0.5 text-[#8b949e]">
            <button
              onClick={() => { if (onShowToast) onShowToast('Back'); }}
              className="p-1 rounded hover:text-white hover:bg-[#22272e] transition"
              title="Back"
            >
              <ArrowLeft className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => { if (onShowToast) onShowToast('Forward'); }}
              className="p-1 rounded hover:text-white hover:bg-[#22272e] transition"
              title="Forward"
            >
              <ArrowRight className="w-3.5 h-3.5" />
            </button>
          </div>

          <button
            onClick={onToggleSidebar}
            className={`p-1 rounded transition ${!_isSidebarCollapsed ? 'text-[#8b949e] hover:text-white hover:bg-[#22272e]' : 'text-white bg-[#22272e]'}`}
            title={`Toggle Sidebar (${formatKeyCombo({ ctrlOrCmd: true, key: "B" })})`}
          >
            <PanelLeft className="w-3.5 h-3.5" />
          </button>
        </div>

        {/* ── THE 3 MODE ICONS PILL (Chat | Code | Research) ── */}
        <div className="flex items-center p-0.5 rounded-lg bg-[#1c2128] border border-[#262c36]/60 gap-0.5">
          {modes.map((m) => {
            const Icon = m.icon;
            const isActive = mode === m.id;
            return (
              <button
                key={m.id}
                onClick={() => {
                  onSwitchMode(m.id);
                  if (onShowToast) onShowToast(`Switched to ${m.description} (${m.shortcut})`);
                }}
                title={`${m.label} (${m.shortcut})`}
                className={`flex items-center gap-1 p-1 rounded-md transition ${
                  isActive
                    ? 'bg-[#2d333b] text-white shadow-sm'
                    : 'text-[#8b949e] hover:text-white hover:bg-[#22272e]'
                }`}
              >
                <Icon
                  className="w-4 h-4"
                  style={{ color: isActive ? m.color : undefined }}
                />
              </button>
            );
          })}
        </div>
      </div>

      {/* ── CENTER & RIGHT: Spans the exact rest of workspace ── */}
      <div className={`flex-1 flex items-center min-w-0 h-full overflow-hidden border-b border-[#262c36]/60 ${isWebTabOpen ? '' : 'hidden'}`}>
        <div id="app-header-tabs-portal" className="w-full h-full flex items-center" />
      </div>

      {!isWebTabOpen && (
        <div className="flex-1 flex items-center justify-between min-w-0 h-full px-4 border-b border-[#262c36]/60">
          <div className="flex items-center gap-2 text-[12px] text-[#8b949e] min-w-0">
            <MessageSquare className="w-3.5 h-3.5 text-[#8b949e] shrink-0" />
            <span className="truncate font-medium text-white/90">
              {workspaceTitle || (mode === 'code' ? 'Engineering Copilot' : mode === 'research' ? 'Research Assistant' : 'Claude Conversation')}
            </span>
          </div>

          <div className="flex items-center gap-2 shrink-0 relative group">
            <button
              onClick={onToggleWebTab}
              className="flex items-center gap-1.5 px-2.5 py-1 rounded-md text-[11px] text-[#8b949e] hover:text-white hover:bg-[#1c2128] border border-[#262c36]/60 transition"
              title="Open resource pane"
            >
              {webTabCount > 0 ? (
                <div className="w-3.5 h-3.5 flex items-center justify-center border border-[#8b949e] rounded-[3px] text-[9px] font-bold">
                  {webTabCount}
                </div>
              ) : (
                <PanelRight className="w-3.5 h-3.5" />
              )}
              <span>Resources</span>
            </button>

            {/* Dropdown Menu on Hover */}
            <div className="absolute top-full right-0 mt-1 w-56 bg-[#161b22] border border-[#262c36] rounded-xl shadow-2xl opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-200 z-50 p-1 flex flex-col">
              <button
                onClick={onNewTab}
                className="flex items-center justify-between px-2.5 py-1.5 rounded-lg text-[12px] text-[#e6edf3] hover:bg-[#21262d] transition group/btn"
              >
                <div className="flex items-center gap-2">
                  <PlusSquare className="w-3.5 h-3.5 text-[#8b949e] group-hover/btn:text-white transition" />
                  <span>New tab</span>
                </div>
                <span className="text-[10px] text-[#8b949e] font-sans tracking-widest">{`${formatKeyCombo({ ctrlOrCmd: true, shift: true, key: "B" })}`}</span>
              </button>
              
              <button
                onClick={onNewTabFullView}
                className="flex items-center justify-between px-2.5 py-1.5 rounded-lg text-[12px] text-[#e6edf3] hover:bg-[#21262d] transition group/btn"
              >
                <div className="flex items-center gap-2">
                  <Maximize2 className="w-3.5 h-3.5 text-[#8b949e] group-hover/btn:text-white transition" />
                  <span>New tab in full view</span>
                </div>
                <span className="text-[10px] text-[#8b949e] font-sans tracking-widest">{`${formatKeyCombo({ ctrlOrCmd: true, shift: true, key: "F" })}`}</span>
              </button>
            </div>
          </div>
        </div>
      )}
    </header>
  );
};
