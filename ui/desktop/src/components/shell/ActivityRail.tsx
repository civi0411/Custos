import React, { useState, useEffect } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import {
  Home,
  Layers,
  Clock,
  Compass,
  Blocks,
  Cpu,
  BookOpen,
  Download,
  Settings,
} from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { UserProfileMenu } from './UserProfileMenu';

export const ActivityRail: React.FC = () => {
  const location = useLocation();
  const navigate = useNavigate();

  const {
    openPlugins,
    isPluginsOpen,
    isSkillsOpen,
    openHelp,
    isHelpOpen,
    openSettings,
    isSettingsOpen,
    openAiSetup,
    isAiSetupOpen,
    userProfile,
    showToast,
    isSidebarCollapsed,
    setIsSidebarCollapsed,
    activeWorkbench,
  } = useAppContext();

  const [isUserMenuOpen, setIsUserMenuOpen] = useState(false);
  const [hoveredItem, setHoveredItem] = useState<string | null>(null);

  // Synchronize sidebar active view (chats vs tasks)
  const [activeSidebarView, setActiveSidebarView] = useState<'chats' | 'tasks'>(() => {
    try {
      const saved = JSON.parse(localStorage.getItem('custos_sidebar_view') || '{}');
      return saved[activeWorkbench] === 'tasks' ? 'tasks' : 'chats';
    } catch {
      return 'chats';
    }
  });

  useEffect(() => {
    const handleSwitch = (e: Event) => {
      const customEvent = e as CustomEvent<'chats' | 'tasks'>;
      if (customEvent.detail) {
        setActiveSidebarView(customEvent.detail);
      }
    };
    window.addEventListener('custos:switch-sidebar-view', handleSwitch);
    return () => window.removeEventListener('custos:switch-sidebar-view', handleSwitch);
  }, []);

  const isStudioActive = location.pathname === '/' || location.pathname.startsWith('/studio');
  const isDashboardActive = location.pathname.startsWith('/dashboard');

  const handleHomeClick = () => {
    if (!isStudioActive) {
      navigate('/studio');
    }
  };

  const handleWorkspacesClick = () => {
    setActiveSidebarView('chats');
    if (isSidebarCollapsed) {
      setIsSidebarCollapsed(false);
    }
    try {
      window.dispatchEvent(new CustomEvent('custos:switch-sidebar-view', { detail: 'chats' }));
    } catch {}
    if (!isStudioActive) navigate('/studio');
  };

  const handleHistoryClick = () => {
    setActiveSidebarView('tasks');
    if (isSidebarCollapsed) {
      setIsSidebarCollapsed(false);
    }
    try {
      window.dispatchEvent(new CustomEvent('custos:switch-sidebar-view', { detail: 'tasks' }));
    } catch {}
    if (!isStudioActive) navigate('/studio');
  };

  const handleCompassClick = () => {
    navigate('/dashboard');
  };

  const handleAiSetupClick = () => {
    openAiSetup('local');
  };

  const handleExtensionsClick = () => {
    openPlugins();
  };

  const handleHelpDocsClick = () => {
    openHelp();
  };

  const handleSettingsClick = () => {
    openSettings('general');
  };

  const handleUpdateCheck = () => {
    showToast('Custos Sovereign Engine v0.3.0 · Local inference sidecar online');
  };

  return (
    <aside
      className="relative flex h-full w-11 shrink-0 flex-col items-center justify-between bg-[#212124] border-r border-white/[0.08] select-none z-40 py-2.5"
      aria-label="Activity Rail"
    >
      {/* ── TOP SECTION: Primary Navigation & Sovereign AI Hub ── */}
      <div className="flex w-full flex-col items-center">
        <div className="flex flex-col items-center gap-2 w-full px-1">
          {/* 1. Home Studio */}
          <div className="relative flex items-center justify-center">
            <button
              type="button"
              onClick={handleHomeClick}
              onMouseEnter={() => setHoveredItem('home')}
              onMouseLeave={() => setHoveredItem(null)}
              aria-label="Home Studio"
              className={`flex size-7 items-center justify-center rounded-md transition-all duration-150 cursor-pointer ${
                isStudioActive && !isAiSetupOpen && !isPluginsOpen && !isHelpOpen
                  ? 'text-white bg-white/14 shadow-xs ring-1 ring-white/15'
                  : 'text-[#a1a1a6] hover:text-white hover:bg-white/8 hover:scale-105'
              }`}
            >
              <Home className="size-[15px] shrink-0" strokeWidth={1.8} />
            </button>
            {hoveredItem === 'home' && (
              <div className="pointer-events-none fixed left-[48px] z-50 flex items-center gap-1.5 rounded-md border border-white/10 bg-[#222226] px-2 py-0.5 text-[11px] font-medium text-white shadow-xl backdrop-blur-md whitespace-nowrap">
                <span>Home Studio</span>
                <span className="font-mono text-[9px] text-white/50">⌘1</span>
              </div>
            )}
          </div>

          {/* 2. Conversations & Workspaces (Chats view) */}
          <div className="relative flex items-center justify-center">
            <button
              type="button"
              onClick={handleWorkspacesClick}
              onMouseEnter={() => setHoveredItem('workspaces')}
              onMouseLeave={() => setHoveredItem(null)}
              aria-label="Conversations & Workspaces"
              className={`flex size-7 items-center justify-center rounded-md transition-all duration-150 cursor-pointer ${
                isStudioActive && activeSidebarView === 'chats' && !isSidebarCollapsed
                  ? 'text-white bg-white/14 shadow-xs ring-1 ring-white/15'
                  : 'text-[#a1a1a6] hover:text-white hover:bg-white/8 hover:scale-105'
              }`}
            >
              <Layers className="size-[15px] shrink-0" strokeWidth={1.8} />
            </button>
            {hoveredItem === 'workspaces' && (
              <div className="pointer-events-none fixed left-[48px] z-50 flex items-center gap-1.5 rounded-md border border-white/10 bg-[#222226] px-2 py-0.5 text-[11px] font-medium text-white shadow-xl backdrop-blur-md whitespace-nowrap">
                <span>Conversations & Workspaces</span>
              </div>
            )}
          </div>

          {/* 3. Tasks, Kanban & Run History */}
          <div className="relative flex items-center justify-center">
            <button
              type="button"
              onClick={handleHistoryClick}
              onMouseEnter={() => setHoveredItem('history')}
              onMouseLeave={() => setHoveredItem(null)}
              aria-label="Tasks & Runs"
              className={`flex size-7 items-center justify-center rounded-md transition-all duration-150 cursor-pointer ${
                isStudioActive && activeSidebarView === 'tasks' && !isSidebarCollapsed
                  ? 'text-white bg-white/14 shadow-xs ring-1 ring-white/15'
                  : 'text-[#a1a1a6] hover:text-white hover:bg-white/8 hover:scale-105'
              }`}
            >
              <Clock className="size-[15px] shrink-0" strokeWidth={1.8} />
            </button>
            {hoveredItem === 'history' && (
              <div className="pointer-events-none fixed left-[48px] z-50 flex items-center gap-1.5 rounded-md border border-white/10 bg-[#222226] px-2 py-0.5 text-[11px] font-medium text-white shadow-xl backdrop-blur-md whitespace-nowrap">
                <span>Tasks & Execution History</span>
              </div>
            )}
          </div>

          {/* 4. Agent Fleet Dashboard */}
          <div className="relative flex items-center justify-center">
            <button
              type="button"
              onClick={handleCompassClick}
              onMouseEnter={() => setHoveredItem('compass')}
              onMouseLeave={() => setHoveredItem(null)}
              aria-label="Agent Fleet Dashboard"
              className={`flex size-7 items-center justify-center rounded-md transition-all duration-150 cursor-pointer ${
                isDashboardActive
                  ? 'text-white bg-white/14 shadow-xs ring-1 ring-white/15'
                  : 'text-[#a1a1a6] hover:text-white hover:bg-white/8 hover:scale-105'
              }`}
            >
              <Compass className="size-[15px] shrink-0" strokeWidth={1.8} />
            </button>
            {hoveredItem === 'compass' && (
              <div className="pointer-events-none fixed left-[48px] z-50 flex items-center gap-1.5 rounded-md border border-white/10 bg-[#222226] px-2 py-0.5 text-[11px] font-medium text-white shadow-xl backdrop-blur-md whitespace-nowrap">
                <span>Agent Fleet Dashboard</span>
                <span className="font-mono text-[9px] text-white/50">⌘2</span>
              </div>
            )}
          </div>

          {/* Subtle Horizontal Divider */}
          <div className="my-1 h-[1px] w-4.5 bg-white/10 shrink-0" />

          {/* 5. Sovereign AI & Model Setup */}
          <div className="relative flex items-center justify-center">
            <button
              type="button"
              onClick={handleAiSetupClick}
              onMouseEnter={() => setHoveredItem('ai-setup')}
              onMouseLeave={() => setHoveredItem(null)}
              aria-label="AI & Model Setup"
              className={`flex size-7 items-center justify-center rounded-md transition-all duration-150 cursor-pointer ${
                isAiSetupOpen
                  ? 'text-teal-300 bg-teal-500/20 shadow-xs ring-1 ring-teal-400/40 scale-105'
                  : 'text-teal-400 hover:text-teal-300 hover:bg-teal-500/10 hover:scale-105 active:scale-95'
              }`}
            >
              <Cpu className="size-[15px] shrink-0" strokeWidth={1.8} />
            </button>
            {hoveredItem === 'ai-setup' && (
              <div className="pointer-events-none fixed left-[48px] z-50 flex items-center gap-1.5 rounded-md border border-white/10 bg-[#222226] px-2 py-0.5 text-[11px] font-medium text-white shadow-xl backdrop-blur-md whitespace-nowrap">
                <span>AI & Model Setup</span>
                <span className="font-mono text-[9px] text-teal-300">⌘3</span>
              </div>
            )}
          </div>

          {/* 6. Modular Extensions & MCP (Blocks) */}
          <div className="relative flex items-center justify-center">
            <button
              type="button"
              onClick={handleExtensionsClick}
              onMouseEnter={() => setHoveredItem('extensions')}
              onMouseLeave={() => setHoveredItem(null)}
              aria-label="Plugins, Skills & MCP"
              className={`flex size-7 items-center justify-center rounded-md transition-all duration-150 cursor-pointer ${
                isPluginsOpen || isSkillsOpen
                  ? 'text-white bg-white/14 shadow-xs ring-1 ring-white/15'
                  : 'text-[#a1a1a6] hover:text-white hover:bg-white/8 hover:scale-105'
              }`}
            >
              <Blocks className="size-[15px] shrink-0" strokeWidth={1.8} />
            </button>
            {hoveredItem === 'extensions' && (
              <div className="pointer-events-none fixed left-[48px] z-50 flex items-center gap-1.5 rounded-md border border-white/10 bg-[#222226] px-2 py-0.5 text-[11px] font-medium text-white shadow-xl backdrop-blur-md whitespace-nowrap">
                <span>Plugins, Skills & MCP</span>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* ── BOTTOM SECTION: Engine Sync + Help/Docs + Settings + Profile Avatar ── */}
      <div className="flex w-full flex-col items-center gap-2 pb-1.5 px-1">
        {/* 7. Local Engine Synchronized Status */}
        <div className="relative flex items-center justify-center">
          <button
            type="button"
            onClick={handleUpdateCheck}
            onMouseEnter={() => setHoveredItem('update')}
            onMouseLeave={() => setHoveredItem(null)}
            aria-label="Check engine status"
            className="flex size-6.5 items-center justify-center rounded-full bg-[#3b82f6] text-white shadow-xs ring-1 ring-blue-400/25 transition-all duration-150 hover:scale-108 hover:bg-[#2563eb] active:scale-95 cursor-pointer"
          >
            <Download className="size-3" strokeWidth={2.4} />
          </button>
          {hoveredItem === 'update' && (
            <div className="pointer-events-none fixed left-[48px] z-50 rounded-md border border-white/10 bg-[#222226] px-2 py-0.5 text-[11px] font-medium text-white shadow-xl backdrop-blur-md whitespace-nowrap">
              <span>Local Engine Synchronized</span>
            </div>
          )}
        </div>

        {/* 8. Help & Sovereign Documentation */}
        <div className="relative flex items-center justify-center">
          <button
            type="button"
            onClick={handleHelpDocsClick}
            onMouseEnter={() => setHoveredItem('docs')}
            onMouseLeave={() => setHoveredItem(null)}
            aria-label="Help & Documentation"
            className={`flex size-6.5 items-center justify-center rounded-md transition-all duration-150 cursor-pointer ${
              isHelpOpen
                ? 'text-white bg-white/14 shadow-xs ring-1 ring-white/15 scale-105'
                : 'text-[#a1a1a6] hover:text-white hover:bg-white/8 hover:scale-105'
            }`}
          >
            <BookOpen className="size-3.5 shrink-0" strokeWidth={1.8} />
          </button>
          {hoveredItem === 'docs' && (
            <div className="pointer-events-none fixed left-[48px] z-50 flex items-center gap-1.5 rounded-md border border-white/10 bg-[#222226] px-2 py-0.5 text-[11px] font-medium text-white shadow-xl backdrop-blur-md whitespace-nowrap">
              <span>Help & Documentation</span>
            </div>
          )}
        </div>

        {/* 9. Application Settings Gear */}
        <div className="relative flex items-center justify-center">
          <button
            type="button"
            onClick={handleSettingsClick}
            onMouseEnter={() => setHoveredItem('settings')}
            onMouseLeave={() => setHoveredItem(null)}
            aria-label="Settings"
            className={`flex size-6.5 items-center justify-center rounded-md transition-all duration-150 cursor-pointer ${
              isSettingsOpen
                ? 'text-white bg-white/14 shadow-xs ring-1 ring-white/15 scale-105'
                : 'text-[#a1a1a6] hover:text-white hover:bg-white/8 hover:scale-105'
            }`}
          >
            <Settings className="size-3.5 shrink-0" strokeWidth={1.8} />
          </button>
          {hoveredItem === 'settings' && (
            <div className="pointer-events-none fixed left-[48px] z-50 flex items-center gap-1.5 rounded-md border border-white/10 bg-[#222226] px-2 py-0.5 text-[11px] font-medium text-white shadow-xl backdrop-blur-md whitespace-nowrap">
              <span>Settings</span>
              <span className="font-mono text-[9px] text-white/50">⌘,</span>
            </div>
          )}
        </div>

        {/* 10. User Profile Avatar */}
        <div className="relative flex items-center justify-center">
          <button
            type="button"
            onClick={() => setIsUserMenuOpen((prev) => !prev)}
            aria-label="User Profile and Settings"
            aria-haspopup="menu"
            aria-expanded={isUserMenuOpen}
            className={`flex size-6.5 items-center justify-center rounded-full text-[10px] font-bold text-white shadow-xs transition-all duration-150 cursor-pointer ${
              isUserMenuOpen ? 'ring-2 ring-white/60 scale-105' : 'hover:scale-108 hover:ring-1.5 hover:ring-white/30'
            }`}
          >
            {userProfile?.avatarUrl ? (
              <img
                src={userProfile.avatarUrl}
                alt={userProfile?.name}
                className="size-6.5 rounded-full object-cover border border-white/20"
              />
            ) : (
              <div className="flex size-6.5 items-center justify-center rounded-full bg-[#e066a3] text-white font-bold tracking-tight shadow-xs">
                {userProfile?.initials || 'TV'}
              </div>
            )}
          </button>
        </div>
      </div>

      {/* Anchored User Profile Menu Dropdown */}
      <UserProfileMenu
        isOpen={isUserMenuOpen}
        onClose={() => setIsUserMenuOpen(false)}
      />
    </aside>
  );
};
