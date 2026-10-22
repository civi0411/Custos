import React, { useEffect, useRef, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { FolderTree, Activity, Plug, Settings, UserRound } from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { UserProfileMenu } from './UserProfileMenu';

/** Shared application navigation; drives primary sidebar and core studio workspace. */
export const ActivityRail: React.FC = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const app = useAppContext();
  const [accountOpen, setAccountOpen] = useState(false);
  const studio = location.pathname === '/' || location.pathname === '/studio' || location.pathname.startsWith('/studio/');
  const lastWorkingRoute = useRef('/studio');

  useEffect(() => {
    if (studio) lastWorkingRoute.current = `${location.pathname === '/' ? '/studio' : location.pathname}${location.search}`;
  }, [studio, location.pathname, location.search]);

  const closeManagement = () => {
    app.setIsSettingsOpen(false);
    app.setIsAiSetupOpen(false);
    app.setIsPluginsOpen(false);
    app.setIsSkillsOpen(false);
    app.setIsHelpOpen(false);
    setAccountOpen(false);
  };

  const handleOpenExplorer = () => {
    closeManagement();
    if (!studio) {
      navigate('/studio');
    }
    app.setIsRecentActivityOpen(false);
    if (app.isSidebarCollapsed) {
      app.setIsSidebarCollapsed(false);
    }
  };

  const handleOpenSupervision = () => {
    closeManagement();
    navigate('/dashboard');
  };

  const managementOpen = app.isSettingsOpen || app.isAiSetupOpen;

  const button = (label: string, Icon: React.ElementType, action: () => void, active = false) => (
    <button
      key={label}
      type="button"
      aria-label={label}
      title={label}
      aria-pressed={active}
      onClick={action}
      className={`flex size-9 items-center justify-center rounded-lg transition focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--color-border-focus)] ${
        active
          ? 'bg-surface-2 text-fg-editor'
          : 'text-fg-muted hover:bg-surface-2 hover:text-fg-editor'
      }`}
    >
      <Icon size={18} strokeWidth={1.8} />
    </button>
  );

  return (
    <aside
      aria-label="Application navigation"
      className="flex w-12 shrink-0 flex-col items-center justify-between border-r border-border-muted bg-surface-0 py-3 z-30 select-none"
    >
      <nav aria-label="Work" className="flex flex-col items-center gap-2">
        {button('Workspaces', FolderTree, handleOpenExplorer, studio && !managementOpen && !app.isSidebarCollapsed)}
        {button('Supervision', Activity, handleOpenSupervision, location.pathname.startsWith('/dashboard') && !managementOpen)}
      </nav>
      <nav aria-label="Application preferences and account" className="flex flex-col items-center gap-2">
        {button('Connections & Tools', Plug, () => app.openAiSetup('cloud'), app.isAiSetupOpen)}
        {button('Settings', Settings, () => app.openSettings('general'), app.isSettingsOpen)}
        {button('Account', UserRound, () => setAccountOpen((prev) => !prev), accountOpen)}
      </nav>
      <UserProfileMenu isOpen={accountOpen} onClose={() => setAccountOpen(false)} />
    </aside>
  );
};
