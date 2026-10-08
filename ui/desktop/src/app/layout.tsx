import React from 'react';
import { Outlet } from 'react-router-dom';
import { useAppContext } from '@/context/AppContext';
import { StatusBar, Toast } from '@/components/shell';
import { SettingsModal, NewSessionModal, AddProviderModal } from '@/components/modals';

/**
 * RootLayout
 * Unified application shell for Custos Sovereign Workspace.
 * Studio renders its own single unified sovereign titlebar and adaptive workspace shell.
 * Eliminates redundant dual-shell architecture (legacy OrCa header & sidebar rail).
 */
export const RootLayout: React.FC = () => {
  const {
    activeSession,
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
    handleCreateNewSession,
    handleSaveProvider
  } = useAppContext();

  return (
    <div className="bg-canvas text-editor-fg font-sans antialiased h-screen w-screen overflow-hidden flex flex-col select-none">
      {/* Dynamic Route Content (Studio Sovereign Workspace or Clean Standalone Views) */}
      <div className="flex-1 flex overflow-hidden min-w-0 relative bg-canvas">
        <Outlet />
      </div>

      {/* Bottom Status Bar */}
      <StatusBar
        activeSessionTitle={activeSession?.title || 'Custos Sovereign Workspace'}
        activeModel={activeSession?.model || 'Custos Runtime · Mediated'}
        uiScale={uiScale}
        onSetUiScale={handleSetUiScale}
        onStepUiScale={handleStepUiScale}
      />

      {/* Global Modals (Preserves session context without full-page navigation) */}
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

      {/* Floating System Toast */}
      <Toast message={toastMessage} />
    </div>
  );
};

export default RootLayout;
