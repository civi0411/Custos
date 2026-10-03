import React from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { useAppContext } from '@/context/AppContext';
import { Header } from '@/components/Header';
import { IconSidebar } from '@/components/IconSidebar';
import { StatusBar } from '@/components/StatusBar';
import { SettingsModal } from '@/components/SettingsModal';
import { NewSessionModal } from '@/components/NewSessionModal';
import { AddProviderModal } from '@/components/AddProviderModal';
import { Toast } from '@/components/Toast';

export const RootLayout: React.FC = () => {
  const {
    currentProject,
    projectData,
    setCurrentProject,
    setActiveSessionId,
    activeSession,
    isSessionsCollapsed,
    setIsSessionsCollapsed,
    viewMode,
    setViewMode,
    isSettingsOpen,
    setIsSettingsOpen,
    isNewSessionOpen,
    setIsNewSessionOpen,
    isAddProviderOpen,
    setIsAddProviderOpen,
    uiScale,
    handleSetUiScale,
    handleStepUiScale,
    toastMessage,
    showToast,
    handleNewProjectPrompt,
    handleCreateNewSession,
    handleSaveProvider
  } = useAppContext();

  const navigate = useNavigate();
  const location = useLocation();

  // Map route pathname to currentTab for IconSidebar compatibility
  const getCurrentTab = () => {
    const path = location.pathname;
    if (path.startsWith('/providers')) return 'providers';
    if (path.startsWith('/chains')) return 'chains';
    if (path.startsWith('/telemetry')) return 'telemetry';
    if (path.startsWith('/cache')) return 'cache';
    if (path.startsWith('/dashboard')) return 'dashboard';
    if (path.startsWith('/docs')) return 'docs';
    if (path.startsWith('/settings')) return 'settings';
    return 'studio';
  };

  const handleSwitchTab = (tab: string) => {
    switch (tab) {
      case 'studio':
        navigate('/studio');
        break;
      case 'providers':
        navigate('/providers');
        break;
      case 'chains':
        navigate('/chains');
        break;
      case 'telemetry':
        navigate('/telemetry');
        break;
      case 'cache':
        navigate('/cache');
        break;
      case 'dashboard':
        navigate('/dashboard');
        break;
      case 'docs':
        navigate('/docs');
        break;
      case 'settings':
        navigate('/settings');
        break;
      default:
        navigate('/studio');
    }
  };

  return (
    <div className="bg-canvas text-[#e2e4ea] font-sans antialiased h-full w-full overflow-hidden flex flex-col">
      {/* Top Header */}
      <Header
        currentProject={currentProject}
        projectNames={Object.keys(projectData)}
        onSelectProject={(p) => {
          setCurrentProject(p);
          const first = (projectData[p] || [])[0];
          if (first) setActiveSessionId(first.id);
        }}
        onNewProjectPrompt={handleNewProjectPrompt}
        isSessionsCollapsed={isSessionsCollapsed}
        onToggleSessions={() => setIsSessionsCollapsed((prev) => !prev)}
        viewMode={viewMode}
        onSetViewMode={setViewMode}
        onOpenSettings={() => setIsSettingsOpen(true)}
        onShowToast={showToast}
      />

      {/* Main Body */}
      <div className="flex-1 flex overflow-hidden min-w-0 relative">
        {/* Left Compact Icon Sidebar */}
        <IconSidebar
          currentTab={getCurrentTab()}
          onSwitchTab={handleSwitchTab}
          onOpenSettings={() => setIsSettingsOpen(true)}
          onShowToast={showToast}
        />

        {/* Dynamic Route Content (Next.js App Router Page Outlet) */}
        <div className="flex-1 flex overflow-hidden min-w-0 relative bg-canvas select-text">
          <Outlet />
        </div>
      </div>

      {/* Bottom Status Bar */}
      <StatusBar
        activeSessionTitle={activeSession?.title || 'No active session'}
        activeModel={activeSession?.model || 'Claude 3.7 Sonnet'}
        uiScale={uiScale}
        onSetUiScale={handleSetUiScale}
        onStepUiScale={handleStepUiScale}
      />

      {/* Modals */}
      <SettingsModal
        isOpen={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
        onOpenProviders={() => navigate('/providers')}
        onShowToast={showToast}
      />

      <NewSessionModal
        isOpen={isNewSessionOpen}
        onClose={() => setIsNewSessionOpen(false)}
        onCreateSession={handleCreateNewSession}
      />

      <AddProviderModal
        isOpen={isAddProviderOpen}
        onClose={() => setIsAddProviderOpen(false)}
        onSaveProvider={handleSaveProvider}
      />

      {/* Floating Toast */}
      <Toast message={toastMessage} />
    </div>
  );
};

export default RootLayout;
