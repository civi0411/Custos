import React, { useState, useRef, useEffect, useCallback } from 'react';
import {
  ShieldCheck,
  RefreshCw,
  CheckCircle2,
  AlertCircle,
  ExternalLink,
  Copy,
  Check,
  X,
  Trash2,
} from 'lucide-react';
import { ProviderBrandIcon } from '@/components/common/AgentIcons';
import { daemonClient } from '@/api/daemon_client';
import type { ProviderServiceId, ConnectedAccount } from '@/types';
import type { OAuthAuthorizeResult } from '@/types/domain';
import { useAppContext } from '@/context/AppContext';
import { oauthProvidersList } from './constants';

export interface OAuthProvidersSectionProps {
  oauthAccounts: ConnectedAccount[];
  onRefreshAccount: (id: string) => Promise<void>;
  onDisconnect: (id: string, name: string) => Promise<void>;
  refreshingId: string | null;
  onShowToast?: (msg: string) => void;
}

export const OAuthProvidersSection: React.FC<OAuthProvidersSectionProps> = ({
  oauthAccounts,
  onRefreshAccount,
  onDisconnect,
  refreshingId,
  onShowToast,
}) => {
  const { handleConnectOAuth, refreshModels } = useAppContext();

  const oauthAccountId = useRef('');
  const [oauthProvider, setOauthProvider] = useState<ProviderServiceId>('antigravity');
  const [authUrlResult, setAuthUrlResult] = useState<OAuthAuthorizeResult | null>(null);
  const [isGeneratingUrl, setIsGeneratingUrl] = useState(false);
  const [callbackInput, setCallbackInput] = useState('');
  const [isExchangingCode, setIsExchangingCode] = useState(false);
  const [exchangeError, setExchangeError] = useState<string | null>(null);
  const [exchangeSuccess, setExchangeSuccess] = useState(false);
  const [copiedLink, setCopiedLink] = useState(false);
  const [callbackStatus, setCallbackStatus] = useState<'idle' | 'listening' | 'exchanging' | 'completed' | 'failed'>('idle');
  const pollTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const oauthProviderLabel = ({
    antigravity: 'Google Account (Unified)',
    google: 'Google Account (Unified)',
    openai: 'OpenAI Codex',
    copilot: 'GitHub Copilot',
    gemini: 'Google Gemini',
    anthropic: 'Anthropic Claude Code',
  } as Partial<Record<ProviderServiceId, string>>)[oauthProvider] || oauthProvider;

  const stopPolling = useCallback(() => {
    if (pollTimerRef.current) {
      clearInterval(pollTimerRef.current);
      pollTimerRef.current = null;
    }
  }, []);

  const startPolling = useCallback(
    (targetProvider: string, accName: string) => {
      stopPolling();
      setCallbackStatus('listening');
      pollTimerRef.current = setInterval(async () => {
        let completing = false;
        try {
          const statusRes = await daemonClient.getOAuthStatus();
          if (statusRes?.status) {
            setCallbackStatus(statusRes.status);
            if (statusRes.status === 'completed' && statusRes.provider_id === targetProvider) {
              completing = true;
              stopPolling();
              await handleConnectOAuth(targetProvider, accName);
              await refreshModels();
              setExchangeSuccess(true);
              onShowToast?.(`Successfully authorized ${accName}.`);
              setTimeout(() => {
                setAuthUrlResult(null);
                setCallbackStatus('idle');
                setExchangeSuccess(false);
              }, 1500);
              return;
            }
            if (statusRes.status === 'failed') {
              stopPolling();
              setAuthUrlResult(null);
              setExchangeError(statusRes.error || 'Token exchange failed');
              return;
            }
          }
        } catch (error: any) {
          if (completing) {
            setCallbackStatus('failed');
            setExchangeError(error?.message || 'Could not load connected account. Retry.');
          }
        }
      }, 1000);
    },
    [stopPolling, handleConnectOAuth, refreshModels, onShowToast]
  );

  useEffect(() => {
    return () => {
      stopPolling();
      void daemonClient.cancelOAuth().catch(() => {});
    };
  }, [stopPolling]);

  const handleSelectOauthProvider = async (prov: ProviderServiceId) => {
    stopPolling();
    if (authUrlResult) {
      try {
        await daemonClient.cancelOAuth();
      } catch {
        /* Ignore */
      }
    }
    setCallbackStatus('idle');
    setOauthProvider(prov);
    setAuthUrlResult(null);
    setCallbackInput('');
    setExchangeError(null);
    setExchangeSuccess(false);
  };

  const openExternalBrowser = async (url: string) => {
    try {
      const { openUrl } = await import('@tauri-apps/plugin-opener');
      await openUrl(url);
      return true;
    } catch (_err) {
      try {
        window.open(url, '_blank');
      } catch (_wErr) {
        /* Ignore */
      }
      return true;
    }
  };

  const handleGenerateAuthorizeUrl = async (autoOpen = true) => {
    setIsGeneratingUrl(true);
    setExchangeError(null);
    try {
      oauthAccountId.current = `${oauthProvider}-${crypto.randomUUID()}`;
      const res = await daemonClient.getOAuthAuthorizeUrl({
        provider_id: oauthAccountId.current,
        service_type: oauthProvider,
      });
      if (!res?.authorization_url) throw new Error('Daemon did not return an authorization URL');
      setAuthUrlResult(res);
      startPolling(oauthAccountId.current, oauthProviderLabel);
      if (autoOpen && res.authorization_url) {
        await openExternalBrowser(res.authorization_url);
      }
    } catch (err: any) {
      setExchangeError(err?.message || String(err || 'Failed to generate authorization URL'));
    } finally {
      setIsGeneratingUrl(false);
    }
  };

  const handleExchangeCode = async () => {
    const rawInput = callbackInput.trim();
    if (!rawInput || !authUrlResult?.code_verifier) return;
    setIsExchangingCode(true);
    setExchangeError(null);
    try {
      const returnedState =
        oauthProvider === 'anthropic' && !rawInput.startsWith('http')
          ? rawInput.split('#')[1]
          : new URL(rawInput).searchParams.get('state');
      if (returnedState !== authUrlResult.state) {
        throw new Error('OAuth callback state does not match or is missing');
      }
      stopPolling();
      await daemonClient.exchangeOAuthCode({
        code_or_url: rawInput,
        code_verifier: authUrlResult.code_verifier,
        provider_id: oauthAccountId.current,
        service_type: oauthProvider,
        redirect_uri: authUrlResult.redirect_uri,
      });
      await handleConnectOAuth(oauthAccountId.current, oauthProviderLabel);
      await refreshModels();
      setExchangeSuccess(true);
      onShowToast?.(`Successfully connected ${oauthProviderLabel}.`);
      setTimeout(() => {
        setAuthUrlResult(null);
        setCallbackInput('');
        setExchangeSuccess(false);
      }, 1500);
    } catch (err: any) {
      setAuthUrlResult(null);
      setExchangeError(err?.message || 'Token exchange failed. Start authorization again.');
    } finally {
      setIsExchangingCode(false);
    }
  };

  const handleCopyLink = () => {
    if (authUrlResult?.authorization_url) {
      navigator.clipboard.writeText(authUrlResult.authorization_url);
      setCopiedLink(true);
      setTimeout(() => setCopiedLink(false), 2000);
    }
  };

  return (
    <div className="space-y-4">
      {/* Active Connected OAuth Credentials */}
      <div className="space-y-2.5">
        <div className="text-[11px] font-semibold uppercase font-mono tracking-wider text-[var(--color-fg-subtle)] flex items-center justify-between">
          <span>Connected Identity & Subscription Accounts</span>
          <span className="text-[10px] lowercase text-[var(--color-fg-subtle)]">
            {oauthAccounts.length} active
          </span>
        </div>

        {oauthAccounts.length === 0 ? (
          <div className="rounded-xl border border-dashed border-[var(--color-border-muted)] bg-[var(--color-surface-0)] p-6 text-center text-xs text-[var(--color-fg-muted)] space-y-1">
            <p>No OAuth subscription accounts connected yet.</p>
            <p className="text-[11px] text-[var(--color-fg-subtle)]">
              Choose an identity provider below to sign in with your browser.
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-2.5">
            {oauthAccounts.map((account) => (
              <div
                key={account.id}
                className="flex items-center justify-between rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-3.5 hover:border-[var(--color-border-emphasis)] transition shadow-2xs"
              >
                <div className="flex items-center gap-3 min-w-0">
                  <div className="size-8 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-1)] flex items-center justify-center shrink-0">
                    <ProviderBrandIcon provider={account.providerId} size={20} />
                  </div>
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-semibold text-[var(--color-editor-fg)] truncate">
                        {account.accountName}
                      </span>
                      <span className="rounded border border-sky-500/30 bg-sky-500/10 px-1.5 py-0.2 font-mono text-[9px] text-sky-300 font-medium">
                        OAuth PKCE
                      </span>
                      <span className="rounded border border-emerald-500/30 bg-emerald-500/10 px-1.5 py-0.2 font-mono text-[9px] text-emerald-400">
                        Pool Eligible
                      </span>
                    </div>
                    <div className="mt-0.5 font-mono text-[10px] text-[var(--color-fg-subtle)] truncate">
                      {account.providerName} · Multi-account rate limit failover active
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-2 shrink-0">
                  <button
                    type="button"
                    onClick={() => onRefreshAccount(account.id)}
                    disabled={refreshingId === account.id}
                    className="flex items-center gap-1 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1 font-mono text-[11px] text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)] disabled:opacity-50 transition cursor-pointer"
                  >
                    <RefreshCw className={`size-3 ${refreshingId === account.id ? 'animate-spin' : ''}`} />
                    Refresh
                  </button>
                  <button
                    type="button"
                    onClick={() => onDisconnect(account.id, account.accountName)}
                    className="flex items-center gap-1 rounded-lg border border-rose-500/25 bg-rose-500/10 px-2.5 py-1 font-mono text-[11px] text-rose-400 hover:bg-rose-500/20 hover:border-rose-500/40 transition cursor-pointer"
                  >
                    <Trash2 className="size-3" /> Disconnect
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* OAuth Identity Provider Selection */}
      <div className="rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-4 sm:p-5 space-y-4 shadow-xs">
        <div className="flex items-center gap-2 border-b border-[var(--color-border-muted)] pb-3">
          <ShieldCheck className="size-4 text-[var(--color-editor-fg)]" />
          <h3 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-editor-fg)] font-mono">
            Authorize Identity & Connect Account
          </h3>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          {oauthProvidersList.map((p) => (
            <button
              key={p.id}
              type="button"
              onClick={() => handleSelectOauthProvider(p.id)}
              className={`p-3.5 rounded-xl text-left transition cursor-pointer flex flex-col justify-between border ${
                oauthProvider === p.id
                  ? 'bg-[var(--color-surface-0)] border-sky-500/40 ring-1 ring-sky-500/20 shadow-xs'
                  : 'bg-[var(--color-surface-0)] border-[var(--color-border-default)] hover:border-[var(--color-border-emphasis)] text-[var(--color-fg-muted)]'
              }`}
            >
              <div className="flex items-start justify-between gap-2 w-full">
                <div className="flex items-center gap-2">
                  <div className="size-6 shrink-0 flex items-center justify-center">
                    <ProviderBrandIcon provider={p.id === 'antigravity' ? 'gemini' : p.id} size={20} />
                  </div>
                  <div className="font-semibold text-xs text-[var(--color-editor-fg)]">{p.name}</div>
                </div>
                <span className={`text-[9px] font-mono px-1.5 py-0.2 rounded border shrink-0 ${p.tagColor}`}>
                  {p.tag}
                </span>
              </div>
              <div className="text-[11px] text-[var(--color-fg-subtle)] mt-2 line-clamp-2 leading-relaxed">
                {p.sub}
              </div>
            </button>
          ))}
        </div>

        {/* Interactive PKCE / Device Flow Box */}
        <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-4 space-y-3 font-mono text-xs shadow-2xs">
          <div className="flex items-center justify-between text-[11px] text-[var(--color-fg-muted)]">
            <span className="font-semibold text-[var(--color-editor-fg)]">
              {oauthProvider === 'copilot'
                ? 'GitHub Device Authorization'
                : `${oauthProviderLabel} PKCE Flow (RFC 7636)`}
            </span>
            <span className="text-[10px] text-emerald-400 bg-emerald-500/10 border border-emerald-500/25 px-2 py-0.5 rounded">
              {oauthProvider === 'copilot' ? 'Device Flow' : 'PKCE S256'}
            </span>
          </div>

          {!authUrlResult ? (
            <div className="flex flex-col items-center justify-center p-5 bg-[var(--color-surface-1)] rounded-xl space-y-3 text-center">
              <p className="text-[11px] text-[var(--color-fg-muted)] max-w-md font-sans">
                Click below to authorize {oauthProviderLabel} in your browser.{' '}
                {oauthProvider === 'copilot'
                  ? 'Enter the device code shown here on GitHub.'
                  : 'Custos automatically captures the callback on completion.'}
              </p>
              <button
                type="button"
                onClick={() => handleGenerateAuthorizeUrl(true)}
                disabled={isGeneratingUrl}
                className="flex items-center gap-2 rounded-lg bg-[var(--color-editor-fg)] px-4 py-2 text-xs font-semibold text-[var(--color-canvas)] hover:opacity-90 active:scale-[0.98] transition cursor-pointer shadow-xs disabled:opacity-50"
              >
                <ExternalLink className="size-3.5" />
                <span>{isGeneratingUrl ? 'Opening Browser…' : `Authorize with ${oauthProviderLabel}`}</span>
              </button>
            </div>
          ) : (
            <div className="space-y-3 p-3 bg-[var(--color-surface-1)] rounded-xl">
              {/* Copilot Device Code display */}
              {oauthProvider === 'copilot' && authUrlResult.user_code && (
                <div className="p-3 bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-lg text-center space-y-1">
                  <div className="text-[10px] text-[var(--color-fg-subtle)] uppercase">GitHub Device Activation Code</div>
                  <div className="text-lg font-bold text-sky-400 tracking-widest">{authUrlResult.user_code}</div>
                  <div className="text-[10px] text-[var(--color-fg-muted)]">
                    Enter this code at <span className="underline text-[var(--color-editor-fg)]">github.com/login/device</span>
                  </div>
                </div>
              )}

              {/* Status Indicator */}
              <div className="flex items-center justify-between text-xs font-sans">
                <span className="flex items-center gap-1.5 text-[var(--color-fg-muted)]">
                  <RefreshCw className="size-3 animate-spin text-sky-400" />
                  {callbackStatus === 'listening'
                    ? 'Listening for browser callback authorization…'
                    : 'Exchanging token with provider…'}
                </span>
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={handleCopyLink}
                    className="flex items-center gap-1 text-[11px] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] cursor-pointer"
                  >
                    {copiedLink ? <Check className="size-3 text-emerald-400" /> : <Copy className="size-3" />}
                    <span>{copiedLink ? 'Copied Link' : 'Copy Auth Link'}</span>
                  </button>
                  <button
                    type="button"
                    onClick={async () => {
                      stopPolling();
                      try { await daemonClient.cancelOAuth(); } catch {}
                      setCallbackStatus('idle');
                      setAuthUrlResult(null);
                      setCallbackInput('');
                      setExchangeError(null);
                    }}
                    className="flex items-center gap-1 rounded px-1.5 py-0.5 text-[11px] text-rose-400 hover:bg-rose-500/10 hover:text-rose-300 transition cursor-pointer"
                  >
                    <X className="size-3" />
                    <span>Cancel</span>
                  </button>
                </div>
              </div>

              {/* Manual callback input */}
              <div className="space-y-1.5 pt-2 border-t border-[var(--color-border-muted)]">
                <label className="text-[10px] text-[var(--color-fg-subtle)] font-mono">
                  Manual Callback / Code Paste (Optional Fallback):
                </label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    placeholder="Paste redirect URL or authorization code here"
                    value={callbackInput}
                    onChange={(e) => setCallbackInput(e.target.value)}
                    className="flex-1 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] font-mono outline-none focus:border-[var(--color-border-emphasis)]"
                  />
                  <button
                    type="button"
                    onClick={handleExchangeCode}
                    disabled={isExchangingCode || !callbackInput.trim()}
                    className="rounded-lg bg-[var(--color-editor-fg)] px-3.5 py-1.5 text-xs font-semibold text-[var(--color-canvas)] hover:opacity-90 active:scale-[0.98] disabled:opacity-50 transition cursor-pointer shadow-xs shrink-0"
                  >
                    {isExchangingCode ? 'Verifying…' : 'Exchange Token'}
                  </button>
                </div>
              </div>
            </div>
          )}

          {exchangeSuccess && (
            <div className="flex items-center gap-1.5 text-xs text-emerald-400 bg-emerald-500/10 border border-emerald-500/25 p-2.5 rounded-lg font-mono">
              <CheckCircle2 className="size-3.5 shrink-0" />
              <span>Token exchange succeeded! Account pooled and ready.</span>
            </div>
          )}

          {exchangeError && (
            <div className="flex items-center justify-between gap-2 text-xs text-rose-400 bg-rose-500/10 border border-rose-500/25 p-2.5 rounded-lg font-mono">
              <div className="flex items-center gap-1.5">
                <AlertCircle className="size-3.5 shrink-0" />
                <span>{exchangeError}</span>
              </div>
              <button
                type="button"
                onClick={() => handleGenerateAuthorizeUrl(true)}
                className="shrink-0 text-[11px] font-semibold text-rose-300 hover:underline cursor-pointer"
              >
                Retry
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
