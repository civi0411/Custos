import React from 'react';
import { Outlet } from 'react-router-dom';
import { useAppContext } from '@/context/AppContext';
import { Toast, ActivityRail, AppTopBar, AppStatusBar } from '@/components/shell';
import {
  SettingsModal,
  AiSetupModal,
  NewSessionModal,
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
    isPluginsOpen,
    setIsPluginsOpen,
    isSkillsOpen,
    setIsSkillsOpen,
    isHelpOpen,
    setIsHelpOpen,
    isGitHubAuthOpen,
    setIsGitHubAuthOpen,
    toastMessage,
    showToast,
    activeWorkbench,
    handleCreateNewSession,
    handleCreateChatSession,
    isResizingColumn,
  } = useAppContext();

  const managementOpen = isSettingsOpen || isAiSetupOpen;

  return (
    <div className={`bg-[#18181b] text-editor-fg font-sans antialiased app-viewport overflow-hidden flex flex-col h-screen w-screen ${isResizingColumn ? 'select-none cursor-col-resize' : 'select-none'}`}>
      {/* ── 1. Sovereign Fixed Top Bar Across Top ── */}
      <AppTopBar />

      {/* ── Active Dragging Overlay to prevent iframes/canvases/editors from stealing mouse events ── */}
      {isResizingColumn && (
        <div
          className="fixed inset-0 z-[99999] cursor-col-resize select-none bg-transparent"
          style={{ pointerEvents: 'auto' }}
        />
      )}

      {/* ── 2. Body: Left Activity Rail + Main Workspace with curved inner corners ── */}
      <div className="flex-1 flex flex-row overflow-hidden min-h-0 w-full relative bg-[#18181b]">
        {/* Seamless Left Activity Rail */}
        <ActivityRail />

        {/* Inner Workspace Container: curved only at the junction of the left bar with top and bottom bars */}
        <main className="flex-1 flex flex-col overflow-hidden min-w-0 min-h-0 relative bg-canvas rounded-tl-2xl rounded-bl-2xl border-t border-l border-b border-white/10 shadow-2xl">
          {/* Dynamic Route Content */}
          <div hidden={managementOpen} style={{ display: managementOpen ? 'none' : undefined }} className="flex-1 flex flex-col overflow-hidden min-w-0 min-h-0 relative bg-transparent">
            <Outlet />
          </div>
          {isAiSetupOpen && <AiSetupModal embedded isOpen initialTab={aiSetupTab} onClose={() => setIsAiSetupOpen(false)} onShowToast={showToast} />}
          {isSettingsOpen && <SettingsModal embedded isOpen initialTab={settingsTab} onClose={() => setIsSettingsOpen(false)} onShowToast={showToast} />}
        </main>
      </div>

      {/* ── 3. Sovereign Status Bar (RAM, System Used, Quotas, Runs, Status) ── */}
      <AppStatusBar />

      {/* ── Global Modals (Preserves session context without full-page navigation) ── */}
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
