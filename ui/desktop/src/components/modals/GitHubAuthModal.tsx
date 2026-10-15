import React, { useState } from 'react';
import { X, Key, Check, Loader2, LogOut, ExternalLink } from 'lucide-react';
import { useAppContext } from '@/context/AppContext';

export interface GitHubAuthModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const GitHubAuthModal: React.FC<GitHubAuthModalProps> = ({ isOpen, onClose }) => {
  const { userProfile, handleGitHubLogin, handleGitHubLogout, showToast } = useAppContext();
  const [authMethod, setAuthMethod] = useState<'username' | 'token'>('username');
  const [usernameInput, setUsernameInput] = useState(userProfile.githubUsername || '');
  const [tokenInput, setTokenInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  if (!isOpen) return null;

  const handleUsernameSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const username = usernameInput.trim();
    if (!username) {
      showToast('Please enter a GitHub username');
      return;
    }
    setIsLoading(true);
    try {
      const success = await handleGitHubLogin({
        username,
        avatar_url: `https://github.com/${encodeURIComponent(username)}.png`,
      });
      if (success) {
        onClose();
      }
    } finally {
      setIsLoading(false);
    }
  };

  const handleTokenSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const token = tokenInput.trim();
    if (!token) {
      showToast('Please enter a GitHub Personal Access Token');
      return;
    }
    setIsLoading(true);
    try {
      const success = await handleGitHubLogin({
        username: usernameInput.trim() || 'GitHub User',
        token,
      });
      if (success) {
        onClose();
      }
    } finally {
      setIsLoading(false);
    }
  };

  const handleLogout = () => {
    handleGitHubLogout();
    onClose();
  };

  return (
    <div
      className="fixed inset-0 z-[120] flex items-center justify-center bg-black/75 p-4 backdrop-blur-md animate-in fade-in duration-150 select-none"
      onClick={onClose}
    >
      <div
        className="w-full max-w-md rounded-2xl border border-white/10 bg-[#1e1e22] text-editor-fg shadow-2xl overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-white/10 px-5 py-4 bg-[#232328]">
          <div className="flex items-center gap-3">
            <div className="flex size-9 items-center justify-center rounded-xl bg-white/10 text-white">
              <svg className="size-5 fill-current" viewBox="0 0 24 24">
                <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
              </svg>
            </div>
            <div>
              <h2 className="text-sm font-semibold text-white">GitHub Account</h2>
              <p className="text-[11px] text-white/50">Authenticate Custos with your GitHub profile</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="flex size-7 items-center justify-center rounded-lg text-white/50 hover:bg-white/10 hover:text-white transition-colors"
          >
            <X className="size-4" />
          </button>
        </div>

        {/* Content */}
        <div className="p-5 space-y-4">
          {userProfile.isAuthenticated ? (
            /* Logged In View */
            <div className="space-y-4">
              <div className="flex items-center gap-3.5 rounded-xl border border-white/10 bg-white/5 p-3.5">
                {userProfile.avatarUrl ? (
                  <img
                    src={userProfile.avatarUrl}
                    alt={userProfile.name}
                    className="size-12 rounded-full border border-white/20 object-cover"
                  />
                ) : (
                  <div className="flex size-12 items-center justify-center rounded-full bg-gradient-to-tr from-pink-500 to-rose-400 text-sm font-bold text-white">
                    {userProfile.initials}
                  </div>
                )}
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-1.5">
                    <span className="font-semibold text-white truncate text-[14px]">{userProfile.name}</span>
                    <span className="flex items-center gap-0.5 rounded-full bg-emerald-500/15 px-2 py-0.5 text-[10px] font-medium text-emerald-400 border border-emerald-500/30">
                      <Check className="size-3" /> Connected
                    </span>
                  </div>
                  {userProfile.githubUsername && (
                    <div className="text-xs text-white/50 font-mono mt-0.5">
                      @{userProfile.githubUsername}
                    </div>
                  )}
                  <div className="text-[11px] text-white/40 mt-1 font-medium">
                    Tier: <span className="text-white/80">{userProfile.tier}</span>
                  </div>
                </div>
              </div>

              <div className="rounded-xl border border-white/5 bg-white/[0.02] p-3 text-xs text-white/60 space-y-1">
                <div className="font-medium text-white/80">Account Permissions</div>
                <p className="text-[11.5px] leading-relaxed text-white/50">
                  Custos uses your GitHub identity to sign git commits, identify your sovereign workbench sessions, and access GitHub Copilot models.
                </p>
              </div>

              <div className="pt-2 flex gap-2">
                <button
                  type="button"
                  onClick={handleLogout}
                  className="flex-1 flex items-center justify-center gap-2 rounded-xl border border-rose-500/30 bg-rose-500/10 py-2.5 text-xs font-semibold text-rose-300 hover:bg-rose-500/20 transition cursor-pointer"
                >
                  <LogOut className="size-3.5" />
                  Sign Out of GitHub
                </button>
                <button
                  type="button"
                  onClick={onClose}
                  className="flex-1 rounded-xl border border-white/10 bg-white/5 py-2.5 text-xs font-medium text-white hover:bg-white/10 transition cursor-pointer"
                >
                  Done
                </button>
              </div>
            </div>
          ) : (
            /* Logged Out / Sign In View */
            <div className="space-y-4">
              {/* Tab Selector */}
              <div className="flex rounded-lg border border-white/10 bg-white/5 p-0.5">
                <button
                  type="button"
                  onClick={() => setAuthMethod('username')}
                  className={`flex-1 py-1.5 text-xs font-medium rounded-md transition ${
                    authMethod === 'username'
                      ? 'bg-white/15 text-white shadow-xs'
                      : 'text-white/50 hover:text-white'
                  }`}
                >
                  GitHub Username
                </button>
                <button
                  type="button"
                  onClick={() => setAuthMethod('token')}
                  className={`flex-1 py-1.5 text-xs font-medium rounded-md transition ${
                    authMethod === 'token'
                      ? 'bg-white/15 text-white shadow-xs'
                      : 'text-white/50 hover:text-white'
                  }`}
                >
                  Personal Access Token
                </button>
              </div>

              {authMethod === 'username' ? (
                <form onSubmit={handleUsernameSubmit} className="space-y-3">
                  <div>
                    <label className="block text-xs font-medium text-white/70 mb-1.5">
                      GitHub Username / Handle
                    </label>
                    <div className="relative">
                      <span className="absolute left-3 top-2.5 text-xs text-white/40 font-mono">@</span>
                      <input
                        type="text"
                        value={usernameInput}
                        onChange={(e) => setUsernameInput(e.target.value)}
                        placeholder="e.g. tranchivi"
                        className="w-full rounded-xl border border-white/10 bg-black/30 pl-7 pr-3 py-2 text-xs text-white placeholder:text-white/30 focus:border-workbench-accent focus:outline-none"
                        autoFocus
                      />
                    </div>
                    <p className="mt-1 text-[11px] text-white/40">
                      Syncs your public GitHub avatar, display name, and sets your workbench identity.
                    </p>
                  </div>

                  <button
                    type="submit"
                    disabled={isLoading}
                    className="w-full flex items-center justify-center gap-2 rounded-xl bg-white text-black py-2.5 text-xs font-semibold hover:bg-white/90 disabled:opacity-50 transition shadow-sm cursor-pointer"
                  >
                    {isLoading ? <Loader2 className="size-3.5 animate-spin" /> : null}
                    Connect GitHub Profile
                  </button>
                </form>
              ) : (
                <form onSubmit={handleTokenSubmit} className="space-y-3">
                  <div>
                    <div className="flex items-center justify-between mb-1.5">
                      <label className="text-xs font-medium text-white/70">
                        GitHub Personal Access Token (classic or fine-grained)
                      </label>
                      <a
                        href="https://github.com/settings/tokens"
                        target="_blank"
                        rel="noreferrer"
                        className="flex items-center gap-1 text-[11px] text-sky-400 hover:underline"
                      >
                        Generate <ExternalLink className="size-2.5" />
                      </a>
                    </div>
                    <input
                      type="password"
                      value={tokenInput}
                      onChange={(e) => setTokenInput(e.target.value)}
                      placeholder="ghp_xxxxxxxxxxxx"
                      className="w-full rounded-xl border border-white/10 bg-black/30 px-3 py-2 text-xs font-mono text-white placeholder:text-white/30 focus:border-workbench-accent focus:outline-none"
                    />
                    <p className="mt-1 text-[11px] text-white/40">
                      Required for private repository indexing and authenticated Copilot routing.
                    </p>
                  </div>

                  <button
                    type="submit"
                    disabled={isLoading}
                    className="w-full flex items-center justify-center gap-2 rounded-xl bg-white text-black py-2.5 text-xs font-semibold hover:bg-white/90 disabled:opacity-50 transition shadow-sm cursor-pointer"
                  >
                    {isLoading ? <Loader2 className="size-3.5 animate-spin" /> : <Key className="size-3.5" />}
                    Authorize with Token
                  </button>
                </form>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
