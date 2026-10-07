import React from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { useAppContext } from '@/context/AppContext';
import { Header, StatusBar, Toast } from '@/components/shell';
import { UnifiedSidebar } from '@/components/sidebar';
import { SettingsModal, NewSessionModal, AddProviderModal } from '@/components/modals';

export const RootLayout: React.FC = () => {
  const {
    currentProject,
    projectData,
    setCurrentProject,
    activeSessionId,
    setActiveSessionId,
    activeSession,
    currentSessions,
    isSessionsCollapsed,
    setIsSessionsCollapsed,
    isSettingsOpen,
    setIsSettingsOpen,
    settingsTab,
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
    <div className="bg-canvas text-editor-fg font-sans antialiased h-full w-full min-w-[960px] min-h-[640px] overflow-hidden flex flex-col select-none">
      {/* Top Header - Render only on secondary standalone pages; Studio has its own single unified titlebar */}
      {getCurrentTab() !== 'studio' && (
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
          onOpenSettings={() => setIsSettingsOpen(true)}
        />
      )}

      {/* Main Body */}
      <div className="flex-1 flex overflow-hidden min-w-0 relative">
        {/* Left Unified Master Sidebar (Orca style: compact rail when collapsed, full tree when expanded) */}
        {getCurrentTab() !== 'studio' && (
          <UnifiedSidebar
            currentTab={getCurrentTab()}
            onSwitchTab={handleSwitchTab}
            currentProject={currentProject}
            projectNames={Object.keys(projectData)}
            onSelectProject={(p) => {
              setCurrentProject(p);
              const first = (projectData[p] || [])[0];
              if (first) setActiveSessionId(first.id);
            }}
            onNewProjectPrompt={handleNewProjectPrompt}
            sessions={currentSessions}
            activeSessionId={activeSessionId}
            onSelectSession={(id) => {
              setActiveSessionId(id);
              navigate(`/studio/${id}`);
            }}
            onOpenNewSessionModal={() => setIsNewSessionOpen(true)}
            isCollapsed={isSessionsCollapsed}
            onToggleCollapse={() => setIsSessionsCollapsed((prev) => !prev)}
            onOpenSettings={() => setIsSettingsOpen(true)}
            onShowToast={showToast}
          />
        )}

        {/* Dynamic Route Content (Studio / Providers / Chains / Telemetry / Cache / Settings) */}
        <div className="flex-1 flex overflow-hidden min-w-0 relative bg-canvas select-text">
          <Outlet />
        </div>
      </div>

        {/* Bottom Status Bar */}
        <StatusBar
          activeSessionTitle={activeSession?.title || 'No active session'}
          activeModel={activeSession?.model || 'Model not reported'}
          uiScale={uiScale}
          onSetUiScale={handleSetUiScale}
          onStepUiScale={handleStepUiScale}
        />

      {/* Modals */}
      <SettingsModal
        isOpen={isSettingsOpen}
        initialTab={settingsTab}
        onClose={() => setIsSettingsOpen(false)}
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
