import React, { useEffect, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Settings, Cpu, CircleHelp, LogOut, ChevronRight, Download } from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { formatKeyCombo } from '@/lib/utils';

export interface UserProfileMenuProps {
  isOpen: boolean;
  onClose: () => void;
  anchorRef?: React.RefObject<HTMLButtonElement | null>;
}

export const UserProfileMenu: React.FC<UserProfileMenuProps> = ({
  isOpen,
  onClose,
}) => {
  const { openAiSetup, openSettings, openHelp, openGitHubAuth, handleGitHubLogout, userProfile } = useAppContext();
  const menuRef = useRef<HTMLDivElement>(null);

  // Click outside to close
  useEffect(() => {
    if (!isOpen) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [isOpen, onClose]);

  const handleProfileHeaderClick = () => {
    onClose();
    openGitHubAuth();
  };

  const handleAiSetupClick = () => {
    onClose();
    openAiSetup('cloud');
  };

  const handleSettingsClick = () => {
    onClose();
    openSettings();
  };

  const handleHelpClick = () => {
    onClose();
    openHelp();
  };

  const handleLogoutClick = () => {
    onClose();
    handleGitHubLogout();
  };

  return (
    <AnimatePresence>
      {isOpen && (
        <motion.div
          ref={menuRef}
          initial={{ opacity: 0, scale: 0.95, y: 8, x: -4 }}
          animate={{ opacity: 1, scale: 1, y: 0, x: 0 }}
          exit={{ opacity: 0, scale: 0.95, y: 6, x: -4 }}
          transition={{ duration: 0.15, ease: 'easeOut' }}
          className="fixed left-[48px] bottom-3 z-[100] w-64 rounded-2xl border border-border-muted bg-surface-1 p-1.5 text-editor-fg shadow-2xl backdrop-blur-xl select-none"
          role="menu"
          aria-label="User profile options"
        >
          {/* User Profile Header Card */}
          {userProfile?.isAuthenticated ? (
            <div
              onClick={handleProfileHeaderClick}
              className="flex items-center gap-3 px-3 py-2.5 rounded-xl hover:bg-surface-2 cursor-pointer transition"
              title="Manage GitHub Account"
            >
              {userProfile?.avatarUrl ? (
                <img
                  src={userProfile.avatarUrl}
                  alt={userProfile?.name || userProfile?.githubUsername || 'User'}
                  className="size-9 rounded-full border border-white/20 object-cover shrink-0"
                />
              ) : (
                <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-white/20 text-xs font-bold text-white shadow-md">
                  {userProfile?.initials || 'U'}
                </div>
              )}
              <div className="min-w-0 flex-1">
                <div className="truncate text-[13.5px] font-semibold text-white tracking-tight leading-snug">
                  {userProfile?.name || userProfile?.githubUsername}
                </div>
                <div className="text-[11px] font-medium text-emerald-400 leading-none mt-0.5 truncate flex items-center gap-1">
                  <span>@{userProfile?.githubUsername}</span>
                  <span className="text-fg-muted">•</span>
                  <span>{userProfile?.tier || 'GitHub'}</span>
                </div>
              </div>
              <ChevronRight className="size-3.5 text-fg-muted shrink-0" />
            </div>
          ) : (
            <div
              onClick={handleProfileHeaderClick}
              className="flex items-center gap-3 px-3 py-2.5 rounded-xl hover:bg-surface-2 cursor-pointer transition group"
              title="Sign in with GitHub"
            >
              <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-white/10 text-fg-muted group-hover:text-fg-editor border border-border-muted transition-colors">
                <svg className="size-4.5 fill-current text-fg-editor" viewBox="0 0 24 24">
                  <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
                </svg>
              </div>
              <div className="min-w-0 flex-1">
                <div className="truncate text-[13.5px] font-semibold text-white tracking-tight leading-snug">
                  Sign in with GitHub
                </div>
                <div className="text-[11px] font-medium text-fg-muted leading-none mt-0.5 truncate">
                  Local Workspace
                </div>
              </div>
              <ChevronRight className="size-3.5 text-fg-muted shrink-0" />
            </div>
          )}

          {/* Divider */}
          <div className="my-1 h-px bg-white/10 mx-1" />

          {/* Connections & Tools Item with shortcut */}
          <button
            type="button"
            onClick={handleAiSetupClick}
            className="flex w-full items-center justify-between rounded-xl px-3 py-2 text-left text-[13px] font-medium text-fg-editor transition-colors hover:bg-surface-2 hover:text-fg-editor cursor-pointer"
            role="menuitem"
          >
            <div className="flex items-center gap-3">
              <Cpu className="size-4 text-teal-400" strokeWidth={1.8} />
              <span>Connections & Tools</span>
            </div>
            <span className="rounded border border-border-muted bg-white/5 px-1.5 py-0.5 font-mono text-[10px] text-teal-300/80 shadow-none">
              {formatKeyCombo({ ctrlOrCmd: true, key: '3' })}
            </span>
          </button>

          {/* Settings Item with shortcut */}
          <button
            type="button"
            onClick={handleSettingsClick}
            className="flex w-full items-center justify-between rounded-xl px-3 py-2 text-left text-[13px] font-medium text-fg-editor transition-colors hover:bg-surface-2 hover:text-fg-editor cursor-pointer"
            role="menuitem"
          >
            <div className="flex items-center gap-3">
              <Settings className="size-4 text-fg-muted" strokeWidth={1.8} />
              <span>Settings</span>
            </div>
            <span className="rounded border border-border-muted bg-white/5 px-1.5 py-0.5 font-mono text-[10px] text-fg-muted shadow-none">
              {formatKeyCombo({ ctrlOrCmd: true, key: ',' })}
            </span>
          </button>

          {/* Help Item */}
          <button
            type="button"
            onClick={handleHelpClick}
            className="flex w-full items-center justify-between rounded-xl px-3 py-2 text-left text-[13px] font-medium text-fg-editor transition-colors hover:bg-surface-2 hover:text-fg-editor cursor-pointer"
            role="menuitem"
          >
            <div className="flex items-center gap-3">
              <CircleHelp className="size-4 text-fg-muted" strokeWidth={1.8} />
              <span>Help</span>
            </div>
            <ChevronRight className="size-3.5 text-fg-muted" />
          </button>

          {/* About Custos Item */}
          <button
            type="button"
            onClick={() => {
              onClose();
              openSettings('about');
            }}
            className="flex w-full items-center justify-between rounded-xl px-3 py-2 text-left text-[13px] font-medium text-fg-editor transition-colors hover:bg-surface-2 hover:text-fg-editor cursor-pointer"
            role="menuitem"
          >
            <div className="flex items-center gap-3">
              <Download className="size-4 text-fg-muted" strokeWidth={1.8} />
              <span>About Custos</span>
            </div>
            <span className="font-mono text-[10px] text-fg-muted">About</span>
          </button>

          {/* Divider */}
          <div className="my-1 h-px bg-white/10 mx-1" />

          {/* Log Out / Switch Account Item */}
          {userProfile?.isAuthenticated ? (
            <button
              type="button"
              onClick={handleLogoutClick}
              className="flex w-full items-center justify-between rounded-xl px-3 py-2 text-left text-[13px] font-medium text-rose-400 transition-colors hover:bg-rose-500/10 hover:text-rose-300 cursor-pointer"
              role="menuitem"
            >
              <div className="flex items-center gap-3">
                <LogOut className="size-4 text-rose-400" strokeWidth={1.8} />
                <span>Log out</span>
              </div>
            </button>
          ) : (
            <button
              type="button"
              onClick={handleProfileHeaderClick}
              className="flex w-full items-center justify-between rounded-xl px-3 py-2 text-left text-[13px] font-semibold text-emerald-400 transition-colors hover:bg-emerald-500/10 hover:text-emerald-300 cursor-pointer"
              role="menuitem"
            >
              <div className="flex items-center gap-3">
                <svg className="size-4 fill-current text-emerald-400" viewBox="0 0 24 24">
                  <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
                </svg>
                <span>Sign in with GitHub</span>
              </div>
            </button>
          )}
        </motion.div>
      )}
    </AnimatePresence>
  );
};
