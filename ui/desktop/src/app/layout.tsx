import React from 'react';
import { Outlet } from 'react-router-dom';
import { useAppContext } from '@/context/AppContext';
import { StatusBar, Toast, ActivityRail, AppTopBar } from '@/components/shell';
import {
  SettingsModal,
  AiSetupModal,
  NewSessionModal,
  AddProviderModal,
  OpenProjectModal,
  PluginsModal,
  SkillsModal,
  HelpModal,
  GitHubAuthModal,
} from '@/components/modals';

/**
 * RootLayout
 * Unified sovereign application shell with fixed left Activity Rail, adaptive workspace, and sovereign titlebar.
 */
export const RootLayout: React.FC = () => {
  const {
    activeSession,
    isSettingsOpen,
    setIsSettingsOpen,
    settingsTab,
    isAiSetupOpen,
    setIsAiSetupOpen,
    aiSetupTab,
    isNewSessionOpen,
    setIsNewSessionOpen,
    isOpenProjectOpen,
    setIsOpenProjectOpen,
    handleProjectOpened,
    isAddProviderOpen,
    setIsAddProviderOpen,
    isPluginsOpen,
    setIsPluginsOpen,
    isSkillsOpen,
    setIsSkillsOpen,
    isHelpOpen,
    setIsHelpOpen,
    isGitHubAuthOpen,
    setIsGitHubAuthOpen,
    uiScale,
    handleSetUiScale,
    handleStepUiScale,
    toastMessage,
    showToast,
    selectedModel,
    activeSessionId,
    activeWorkbench,
    activeDetection,
    handleCreateNewSession,
    handleCreateChatSession,
    handleSaveProvider,
  } = useAppContext();

  return (
    <div className="bg-[#212124] text-editor-fg font-sans antialiased app-viewport overflow-hidden flex flex-col select-none">
      {/* ── 1. Sovereign Fixed Top Bar Across Top ── */}
      <AppTopBar />

      {/* ── 2. Body: Left Activity Rail + Inner Workspace Island ── */}
      <div className="flex-1 flex flex-row overflow-hidden min-h-0 w-full relative">
        {/* Fixed Left Activity Rail (Matching #212124 background) */}
        <ActivityRail />

        {/* Inner Workspace Container (Flat, seamless, sovereign layout) */}
        <main className="flex-1 flex flex-col overflow-hidden min-w-0 min-h-0 relative bg-[#121214]">
          {/* Dynamic Route Content */}
          <div className="flex-1 flex flex-col overflow-hidden min-w-0 min-h-0 relative bg-transparent">
            <Outlet />
          </div>

          {/* Bottom Status Bar */}
          <StatusBar
            activeSessionTitle={activeSession?.title || 'Custos Sovereign Workspace'}
            activeSessionId={activeSession?.id || activeSessionId}
            activeModel={activeSession?.model || selectedModel || 'No model selected'}
            activeWorkbench={activeWorkbench}
            activeBranch={activeDetection?.branch}
            isDirty={activeDetection?.is_dirty}
            uiScale={uiScale}
            onSetUiScale={handleSetUiScale}
            onStepUiScale={handleStepUiScale}
          />
        </main>
      </div>

      {/* ── Global Modals (Preserves session context without full-page navigation) ── */}
      <AiSetupModal
        isOpen={isAiSetupOpen}
        initialTab={aiSetupTab}
        onClose={() => setIsAiSetupOpen(false)}
        onShowToast={showToast}
      />

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
        onCreateChat={handleCreateChatSession}
        activeWorkbench={activeWorkbench}
      />

      <OpenProjectModal
        isOpen={isOpenProjectOpen}
        onClose={() => setIsOpenProjectOpen(false)}
        onProjectOpened={handleProjectOpened}
        onShowToast={showToast}
      />

      <AddProviderModal
        isOpen={isAddProviderOpen}
        onClose={() => setIsAddProviderOpen(false)}
        onSaveProvider={handleSaveProvider}
      />

      <PluginsModal
        isOpen={isPluginsOpen}
        onClose={() => setIsPluginsOpen(false)}
      />

      <SkillsModal
        isOpen={isSkillsOpen}
        onClose={() => setIsSkillsOpen(false)}
      />

      <HelpModal
        isOpen={isHelpOpen}
        onClose={() => setIsHelpOpen(false)}
      />

      <GitHubAuthModal
        isOpen={isGitHubAuthOpen}
        onClose={() => setIsGitHubAuthOpen(false)}
      />

      {/* Floating System Toast */}
      <Toast message={toastMessage} />
    </div>
  );
};

export default RootLayout;
