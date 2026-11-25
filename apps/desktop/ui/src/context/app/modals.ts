import { useState, useCallback, useEffect } from 'react';
import { daemonClient } from '@/api/daemon_client';

export interface UserProfile {
  name: string;
  initials: string;
  tier: string;
  githubUsername?: string;
  avatarUrl?: string;
  isAuthenticated?: boolean;
}

export interface ModalsDeps {
  showToast: (msg: string) => void;
}

export function useModalsSlice({ showToast }: ModalsDeps) {
  const [isSettingsOpen, setIsSettingsOpen] = useState<boolean>(false);
  const [settingsTab, setSettingsTab] = useState<string>('general');
  const [isAiSetupOpen, setIsAiSetupOpen] = useState<boolean>(false);
  const [aiSetupTab, setAiSetupTab] = useState<
    'local' | 'cloud' | 'routing' | 'subagents' | 'quotas' | 'tools' | 'environments' | 'diagnostics'
  >('local');
  const [isNewSessionOpen, setIsNewSessionOpen] = useState<boolean>(false);
  const [isOpenProjectOpen, setIsOpenProjectOpen] = useState<boolean>(false);
  const [isAddProviderOpen, setIsAddProviderOpen] = useState<boolean>(false);
  const [isPluginsOpen, setIsPluginsOpen] = useState<boolean>(false);
  const [isSkillsOpen, setIsSkillsOpen] = useState<boolean>(false);
  const [isHelpOpen, setIsHelpOpen] = useState<boolean>(false);
  const [isSessionsCollapsed, setIsSessionsCollapsed] = useState<boolean>(false);

  const openPlugins = useCallback(() => setIsPluginsOpen(true), []);
  const openSkills = useCallback(() => setIsSkillsOpen(true), []);
  const openHelp = useCallback(() => setIsHelpOpen(true), []);

  const [isGitHubAuthOpen, setIsGitHubAuthOpen] = useState<boolean>(false);
  const openGitHubAuth = useCallback(() => setIsGitHubAuthOpen(true), []);

  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState<boolean>(false);
  const toggleSidebar = useCallback(() => setIsSidebarCollapsed((prev) => !prev), []);
  const [isRecentActivityOpen, setIsRecentActivityOpen] = useState<boolean>(false);

  const [userProfile, setUserProfile] = useState<UserProfile>(() => {
    try {
      const saved = localStorage.getItem('custos_user_profile');
      if (saved) {
        const parsed = JSON.parse(saved);
        if (
          parsed &&
          typeof parsed === 'object' &&
          parsed.isAuthenticated &&
          parsed.githubUsername &&
          parsed.name !== 'Trần Vĩ' &&
          parsed.initials !== 'TV'
        ) {
          return { ...parsed, isAuthenticated: false };
        }
      }
    } catch {}
    return {
      name: '',
      initials: '',
      tier: '',
      githubUsername: undefined,
      avatarUrl: undefined,
      isAuthenticated: false,
    };
  });

  useEffect(() => {
    try {
      localStorage.setItem('custos_user_profile', JSON.stringify(userProfile));
    } catch {}
  }, [userProfile]);

  const handleGitHubLogin = useCallback(
    async (account: { username: string; name?: string; avatar_url?: string }) => {
      try {
        const status = await daemonClient.getGitHubAuthStatus();
        if (!status.authenticated || !status.user?.login || status.user.login !== account.username) {
          throw new Error(status.error || 'GitHub account has not been verified by the daemon.');
        }
        const finalName = status.user.name || status.user.login;
        const finalAvatar = status.user.avatar_url;

        const initials =
          finalName
            .split(' ')
            .filter(Boolean)
            .map((p: string) => p[0])
            .join('')
            .slice(0, 2)
            .toUpperCase() || account.username.slice(0, 2).toUpperCase();

        const updated: UserProfile = {
          name: finalName,
          initials,
          tier: 'GitHub',
          githubUsername: account.username,
          avatarUrl: finalAvatar || `https://github.com/${encodeURIComponent(account.username)}.png`,
          isAuthenticated: true,
        };

        setUserProfile(updated);
        try {
          localStorage.setItem('custos_user_profile', JSON.stringify(updated));
          localStorage.removeItem('custos_github_token');
        } catch {}

        showToast(`Logged in as @${account.username}`);
        return true;
      } catch (err: any) {
        showToast(`GitHub login failed: ${err?.message || err}`);
        return false;
      }
    },
    [showToast]
  );

  const handleGitHubLogout = useCallback(() => {
    const unauthProfile: UserProfile = {
      name: '',
      initials: '',
      tier: '',
      githubUsername: undefined,
      avatarUrl: undefined,
      isAuthenticated: false,
    };
    setUserProfile(unauthProfile);
    try {
      localStorage.setItem('custos_user_profile', JSON.stringify(unauthProfile));
      localStorage.removeItem('custos_github_token');
    } catch {}
    showToast('Account hidden in Custos. GitHub CLI credentials remain on this machine.');
  }, [showToast]);

  const openSettings = useCallback((tab?: string) => {
    const canonicalTab =
      tab === 'providers' || tab === 'cloud' || tab === 'connections'
        ? 'connections'
        : tab === 'runtime' || tab === 'local'
        ? 'connections'
        : tab === 'quotas' || tab === 'execution'
        ? 'execution'
        : tab === 'tools' || tab === 'plugins' || tab === 'skills'
        ? 'tools'
        : tab || 'general';
    setSettingsTab(canonicalTab);
    setIsAiSetupOpen(false);
    setIsSettingsOpen(true);
  }, []);

  const openAiSetup = useCallback(
    (tab?: 'local' | 'cloud' | 'routing' | 'subagents' | 'quotas' | 'tools' | 'environments' | 'diagnostics') => {
      openSettings(tab);
    },
    [openSettings]
  );

  return {
    isSettingsOpen,
    setIsSettingsOpen,
    settingsTab,
    setSettingsTab,
    openSettings,
    isAiSetupOpen,
    setIsAiSetupOpen,
    aiSetupTab,
    setAiSetupTab,
    openAiSetup,
    isNewSessionOpen,
    setIsNewSessionOpen,
    isOpenProjectOpen,
    setIsOpenProjectOpen,
    isAddProviderOpen,
    setIsAddProviderOpen,
    isPluginsOpen,
    setIsPluginsOpen,
    openPlugins,
    isSkillsOpen,
    setIsSkillsOpen,
    openSkills,
    isHelpOpen,
    setIsHelpOpen,
    openHelp,
    isSessionsCollapsed,
    setIsSessionsCollapsed,
    isGitHubAuthOpen,
    setIsGitHubAuthOpen,
    openGitHubAuth,
    handleGitHubLogin,
    handleGitHubLogout,
    userProfile,
    setUserProfile,
    isSidebarCollapsed,
    setIsSidebarCollapsed,
    toggleSidebar,
    isRecentActivityOpen,
    setIsRecentActivityOpen,
  };
}
