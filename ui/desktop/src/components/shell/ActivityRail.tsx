import React, { useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import {
  Home,
  Layers,
  Clock,
  BookOpen,
  Compass,
  MoreHorizontal,
  Download,
  Cpu,
  Settings,
} from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { UserProfileMenu } from './UserProfileMenu';

export const ActivityRail: React.FC = () => {
  const location = useLocation();
  const navigate = useNavigate();

  const {
    openPlugins,
    openSettings,
    isSettingsOpen,
    openAiSetup,
    isAiSetupOpen,
    userProfile,
    showToast,
  } = useAppContext();

  const [isUserMenuOpen, setIsUserMenuOpen] = useState(false);
  const [hoveredItem, setHoveredItem] = useState<string | null>(null);

  const isStudioActive = location.pathname === '/' || location.pathname.startsWith('/studio');
  const isDashboardActive = location.pathname.startsWith('/dashboard');

  const handleHomeClick = () => {
    navigate('/studio');
  };

  const handleWorkspacesClick = () => {
    try {
      window.dispatchEvent(new CustomEvent('custos:switch-sidebar-view', { detail: 'chats' }));
    } catch {}
    if (!isStudioActive) navigate('/studio');
  };

  const handleHistoryClick = () => {
    try {
      window.dispatchEvent(new CustomEvent('custos:switch-sidebar-view', { detail: 'tasks' }));
    } catch {}
    if (!isStudioActive) navigate('/studio');
  };

  const handleDocsClick = () => {
    openSettings('usage');
  };

  const handleCompassClick = () => {
    navigate('/dashboard');
  };

  const handleMoreClick = () => {
    openPlugins();
  };

  const handleUpdateCheck = () => {
    showToast('Custos Sovereign Engine v0.3.0 · Local inference sidecar online');
  };

  return (
    <aside
      className="relative flex h-full w-11 shrink-0 flex-col items-center justify-between bg-[#212124] border-r border-white/[0.08] select-none z-40 py-2.5"
      aria-label="Activity Rail"
    >
      {/* ── TOP SECTION: Navigation Action Stack ── */}
      <div className="flex w-full flex-col items-center">
        {/* Navigation Action Stack */}
        <div className="flex flex-col items-center gap-2 w-full px-1">
          {/* 1. Home Icon */}
          <div className="relative flex items-center justify-center">
            <button
              type="button"
              onClick={handleHomeClick}
              onMouseEnter={() => setHoveredItem('home')}
              onMouseLeave={() => setHoveredItem(null)}
              aria-label="Home Studio"
              className={`flex size-7 items-center justify-center rounded-md transition-all duration-150 cursor-pointer ${
                isStudioActive
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

          {/* 2. Workspaces / Layers Icon */}
          <div className="relative flex items-center justify-center">
            <button
              type="button"
              onClick={handleWorkspacesClick}
              onMouseEnter={() => setHoveredItem('workspaces')}
              onMouseLeave={() => setHoveredItem(null)}
              aria-label="Workspaces & Canvas"
              className="flex size-7 items-center justify-center rounded-md text-[#a1a1a6] hover:text-white hover:bg-white/8 hover:scale-105 transition-all duration-150 cursor-pointer"
            >
              <Layers className="size-[15px] shrink-0" strokeWidth={1.8} />
            </button>
            {hoveredItem === 'workspaces' && (
              <div className="pointer-events-none fixed left-[48px] z-50 flex items-center gap-1.5 rounded-md border border-white/10 bg-[#222226] px-2 py-0.5 text-[11px] font-medium text-white shadow-xl backdrop-blur-md whitespace-nowrap">
                <span>Workspaces & Canvas</span>
              </div>
            )}
          </div>

          {/* 3. Recent History / Clock Icon */}
          <div className="relative flex items-center justify-center">
            <button
              type="button"
              onClick={handleHistoryClick}
              onMouseEnter={() => setHoveredItem('history')}
              onMouseLeave={() => setHoveredItem(null)}
              aria-label="Recent History"
              className="flex size-7 items-center justify-center rounded-md text-[#a1a1a6] hover:text-white hover:bg-white/8 hover:scale-105 transition-all duration-150 cursor-pointer"
            >
              <Clock className="size-[15px] shrink-0" strokeWidth={1.8} />
            </button>
            {hoveredItem === 'history' && (
              <div className="pointer-events-none fixed left-[48px] z-50 flex items-center gap-1.5 rounded-md border border-white/10 bg-[#222226] px-2 py-0.5 text-[11px] font-medium text-white shadow-xl backdrop-blur-md whitespace-nowrap">
                <span>Recent Tasks & History</span>
              </div>
            )}
          </div>

          {/* 4. Documentation / Books Icon */}
          <div className="relative flex items-center justify-center">
            <button
              type="button"
              onClick={handleDocsClick}
              onMouseEnter={() => setHoveredItem('docs')}
              onMouseLeave={() => setHoveredItem(null)}
              aria-label="Documentation & Knowledge"
              className="flex size-7 items-center justify-center rounded-md text-[#a1a1a6] hover:text-white hover:bg-white/8 hover:scale-105 transition-all duration-150 cursor-pointer"
            >
              <BookOpen className="size-[15px] shrink-0" strokeWidth={1.8} />
            </button>
            {hoveredItem === 'docs' && (
              <div className="pointer-events-none fixed left-[48px] z-50 flex items-center gap-1.5 rounded-md border border-white/10 bg-[#222226] px-2 py-0.5 text-[11px] font-medium text-white shadow-xl backdrop-blur-md whitespace-nowrap">
                <span>Documentation & Usage</span>
              </div>
            )}
          </div>

          {/* 5. Compass / Agent Fleet Icon */}
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

          {/* 6. More Options (...) Icon */}
          <div className="relative flex items-center justify-center">
            <button
              type="button"
              onClick={handleMoreClick}
              onMouseEnter={() => setHoveredItem('more')}
              onMouseLeave={() => setHoveredItem(null)}
              aria-label="More Extensions"
              className="flex size-7 items-center justify-center rounded-md text-[#a1a1a6] hover:text-white hover:bg-white/8 hover:scale-105 transition-all duration-150 cursor-pointer"
            >
              <MoreHorizontal className="size-[15px] shrink-0" strokeWidth={1.8} />
            </button>
            {hoveredItem === 'more' && (
              <div className="pointer-events-none fixed left-[48px] z-50 flex items-center gap-1.5 rounded-md border border-white/10 bg-[#222226] px-2 py-0.5 text-[11px] font-medium text-white shadow-xl backdrop-blur-md whitespace-nowrap">
                <span>Plugins, Skills & MCP</span>
              </div>
            )}
          </div>

          {/* Subtle Horizontal Divider */}
          <div className="my-1 h-[1px] w-4.5 bg-white/10 shrink-0" />

          {/* 7. Dedicated AI & Model Setup Button */}
          <div className="relative flex items-center justify-center">
            <button
              type="button"
              onClick={() => openAiSetup('local')}
              onMouseEnter={() => setHoveredItem('ai-setup')}
              onMouseLeave={() => setHoveredItem(null)}
              aria-label="AI & Model Setup"
              className={`flex size-7.5 items-center justify-center rounded-lg transition-all duration-150 cursor-pointer shadow-xs ${
                isAiSetupOpen
                  ? 'bg-teal-600 text-white ring-2 ring-teal-400/40 shadow-teal-500/20 scale-105'
                  : 'bg-[#2a2a2e] hover:bg-[#36363a] text-teal-400 hover:text-teal-300 border border-teal-500/25 hover:border-teal-500/50 hover:scale-105 active:scale-95'
              }`}
            >
              <Cpu className="size-4 shrink-0" strokeWidth={1.8} />
            </button>
            {hoveredItem === 'ai-setup' && (
              <div className="pointer-events-none fixed left-[48px] z-50 flex items-center gap-1.5 rounded-md border border-white/10 bg-[#222226] px-2 py-0.5 text-[11px] font-medium text-white shadow-xl backdrop-blur-md whitespace-nowrap">
                <span>AI & Model Setup</span>
                <span className="font-mono text-[9px] text-teal-300">⌘3</span>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* ── BOTTOM SECTION: Blue Sync Button + Settings Gear + User Profile Avatar ── */}
      <div className="flex w-full flex-col items-center gap-2 pb-1.5 px-1">
        {/* Blue Sync / Download Circular Pill */}
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

        {/* Settings Gear Button */}
        <div className="relative flex items-center justify-center">
          <button
            type="button"
            onClick={() => openSettings('general')}
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

        {/* User Profile Avatar Circle (triggers UserProfileMenu) */}
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

