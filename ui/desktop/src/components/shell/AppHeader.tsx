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

// 3 primary workspace modes
export type AppWorkspaceMode = 'chat' | 'code' | 'research';

interface AppHeaderProps {
  mode: AppWorkspaceMode;
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
      color: '#cc785c',
      shortcut: '⌘1',
      description: 'Claude Desktop — Chat & Cowork assistant'
    },
    {
      id: 'code',
      label: 'Code',
      icon: Code2,
      color: '#58a6ff',
      shortcut: '⌘2',
      description: 'Codex 3-Column — Engineering ADE + Orca tools'
    },
    {
      id: 'research',
      label: 'Research',
      icon: FlaskConical,
      color: '#a371f7',
      shortcut: '⌘3',
      description: 'Claude Science — Literature, Claims & Experiments'
    }
  ];

  return (
    <header className="h-10 bg-[#090d13] flex items-center select-none z-30 shrink-0 font-sans text-xs transition-all">
      {/* ── LEFT: Nav Controls + Sidebar + Mode Switcher ── */}
      <div className={`flex items-center justify-between shrink-0 h-full transition-all duration-300 ${!_isSidebarCollapsed ? 'w-64 pr-3 border-r border-[#2d2d2d] bg-[#181818]' : 'w-auto pr-3 bg-transparent border-transparent'}`}>
        <div className="flex items-center gap-1.5 pl-3">
          {/* Nav arrows */}
          <div className="flex items-center gap-0.5 text-[#8b949e]">
            <button
              onClick={() => { if (onShowToast) onShowToast('Back'); }}
              className="p-1 rounded hover:text-white hover:bg-[#2b2b2b] transition"
              title="Back"
            >
              <ArrowLeft className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => { if (onShowToast) onShowToast('Forward'); }}
              className="p-1 rounded hover:text-white hover:bg-[#2b2b2b] transition"
              title="Forward"
            >
              <ArrowRight className="w-3.5 h-3.5" />
            </button>
          </div>

          <button
            onClick={onToggleSidebar}
            className={`p-1 rounded transition ${!_isSidebarCollapsed ? 'text-[#8b949e] hover:text-white hover:bg-[#2b2b2b]' : 'text-white bg-[#2b2b2b]'}`}
            title="Toggle Sidebar (⌘B)"
          >
            <PanelLeft className="w-3.5 h-3.5" />
          </button>
        </div>

        

        {/* ── THE 3 MODE ICONS PILL (Chat | Code | Research) ── */}
        <div className="flex items-center p-0.5 rounded-lg bg-[#222222] border border-[#383838] gap-0.5">
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
                    ? 'bg-[#333333] text-white shadow-sm'
                    : 'text-[#8b949e] hover:text-white hover:bg-[#333333]/60'
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

      {/* ── CENTER: Tabs (Children) ── */}
      <div className="flex-1 flex items-center min-w-0 h-full overflow-hidden border-b border-[#21262d]">
        <div id="app-header-tabs-portal" className="w-full h-full flex items-center" />
      </div>

      {/* ── RIGHT: New Tab Dropdown / Toggle Right Sidebar ── */}
      <div className="flex items-center gap-2 shrink-0 px-3 h-full border-b border-[#21262d] relative group">
        {isWebTabOpen ? (
          <button
            onClick={onToggleWebTab}
            className="p-1 rounded-lg text-[#8b949e] hover:text-white hover:bg-[#161b22] transition"
            title="Toggle Right Sidebar"
          >
            <PanelRight className="w-4 h-4" />
          </button>
        ) : (
          <>
            <button
              className="p-1 rounded-lg text-[#8b949e] hover:text-white hover:bg-[#161b22] transition"
            >
              {webTabCount > 0 ? (
                <div className="w-4 h-4 flex items-center justify-center border border-[#8b949e] rounded-[3px] text-[10px] font-bold group-hover:text-white group-hover:border-white transition-colors">
                  {webTabCount}
                </div>
              ) : (
                <PlusSquare className="w-4 h-4" />
              )}
            </button>

            {/* Dropdown Menu on Hover */}
            <div className="absolute top-full right-2 mt-1 w-56 bg-[#161b22] border border-[#30363d] rounded-xl shadow-2xl opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-200 z-50 p-1 flex flex-col">
              <button
                onClick={onNewTab}
                className="flex items-center justify-between px-2.5 py-1.5 rounded-lg text-[13px] text-[#e6edf3] hover:bg-[#21262d] transition group/btn"
              >
                <div className="flex items-center gap-2">
                  <PlusSquare className="w-3.5 h-3.5 text-[#8b949e] group-hover/btn:text-white transition" />
                  <span>New tab</span>
                </div>
                <span className="text-[10px] text-[#8b949e] font-sans tracking-widest">⇧⌘B</span>
              </button>
              
              <button
                onClick={onNewTabFullView}
                className="flex items-center justify-between px-2.5 py-1.5 rounded-lg text-[13px] text-[#e6edf3] hover:bg-[#21262d] transition group/btn"
              >
                <div className="flex items-center gap-2">
                  <Maximize2 className="w-3.5 h-3.5 text-[#8b949e] group-hover/btn:text-white transition" />
                  <span>New tab in full view</span>
                </div>
                <span className="text-[10px] text-[#8b949e] font-sans tracking-widest">⇧⌘F</span>
              </button>
            </div>
          </>
        )}
      </div>
    </header>
  );
};
