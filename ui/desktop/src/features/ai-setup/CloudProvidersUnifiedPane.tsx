import React, { useState, useEffect, useRef, useCallback } from 'react';
import {
  Key,
  ShieldCheck,
  RefreshCw,
  Cpu,
  CheckCircle2,
  AlertCircle,
  Plus,
  Trash2,
  ExternalLink,
  Copy,
  Check,
} from 'lucide-react';
import { ProviderBrandIcon } from '@/components/common/AgentIcons';
import { CustomSelect, CustomSelectOption } from '@/components/common/CustomSelect';
import { daemonClient } from '@/api/daemon_client';
import { ProbedModel, ModelCatalogOption, ProviderServiceId } from '@/types';
import { useAppContext, defaultProbeBaseUrl } from '@/context/AppContext';

interface CloudProvidersUnifiedPaneProps {
  onShowToast?: (msg: string) => void;
  initialSubTab?: 'api_keys' | 'oauth';
}

const providerOptions: CustomSelectOption[] = [
  {
    value: 'anthropic',
    label: 'Anthropic Claude',
    sublabel: 'Claude 3.7 Sonnet, 3.5 Haiku',
    icon: <ProviderBrandIcon provider="anthropic" size={16} />,
  },
  {
    value: 'openai',
    label: 'OpenAI / Codex',
    sublabel: 'GPT-4o, o3-mini, o1',
    icon: <ProviderBrandIcon provider="openai" size={16} />,
  },
  {
    value: 'gemini',
    label: 'Google Cloud / Gemini',
    sublabel: 'Gemini 2.5 Pro, Flash (1M tokens)',
    icon: <ProviderBrandIcon provider="gemini" size={16} />,
  },
  {
    value: 'deepseek',
    label: 'DeepSeek',
    sublabel: 'DeepSeek V3, R1 Reasoner',
    icon: <ProviderBrandIcon provider="deepseek" size={16} />,
  },
  {
    value: 'openrouter',
    label: 'OpenRouter',
    sublabel: '200+ AI models unified gateway',
    icon: <ProviderBrandIcon provider="openrouter" size={16} />,
  },
  {
    value: 'groq',
    label: 'Groq LPU',
    sublabel: 'Ultra-fast inference (Llama, Qwen)',
    icon: <ProviderBrandIcon provider="groq" size={16} />,
  },
  {
    value: 'cerebras',
    label: 'Cerebras',
    sublabel: 'World-record fast token generation',
    icon: <ProviderBrandIcon provider="cerebras" size={16} />,
  },
  {
    value: 'nvidia',
    label: 'NVIDIA NIM',
    sublabel: 'Free tier Nemotron, Llama 3, DeepSeek',
    icon: <ProviderBrandIcon provider="nvidia" size={16} />,
  },
  {
    value: 'mistral',
    label: 'Mistral AI',
    sublabel: 'Codestral, Mistral Large',
    icon: <ProviderBrandIcon provider="mistral" size={16} />,
  },
  {
    value: 'together',
    label: 'Together AI',
    sublabel: 'Open-source frontier models',
    icon: <ProviderBrandIcon provider="together" size={16} />,
  },
  {
    value: 'moonshot',
    label: 'Moonshot Kimi',
    sublabel: 'Kimi K3 (1M context window)',
    icon: <ProviderBrandIcon provider="moonshot" size={16} />,
  },
  {
    value: 'zhipu',
    label: 'Zhipu GLM / Z.ai',
    sublabel: 'GLM-5.3, GLM-4 coding models',
    icon: <ProviderBrandIcon provider="zhipu" size={16} />,
  },
  {
    value: 'minimax',
    label: 'MiniMax',
    sublabel: 'MiniMax M3.1 Flash, M3',
    icon: <ProviderBrandIcon provider="minimax" size={16} />,
  },
  {
    value: 'alibaba',
    label: 'Alibaba Qwen (DashScope)',
    sublabel: 'Qwen 2.5 Coder 32B, Qwen 3.8',
    icon: <ProviderBrandIcon provider="alibaba" size={16} />,
  },
  {
    value: 'perplexity',
    label: 'Perplexity AI',
    sublabel: 'Sonar search-grounded models',
    icon: <ProviderBrandIcon provider="perplexity" size={16} />,
  },
  {
    value: 'xai',
    label: 'xAI (Grok)',
    sublabel: 'Grok 4.6, Grok Code Fast',
    icon: <ProviderBrandIcon provider="xai" size={16} />,
  },
  {
    value: 'sambanova',
    label: 'SambaNova Cloud',
    sublabel: 'Ultra-fast Llama 3.3 70B & DeepSeek R1',
    icon: <ProviderBrandIcon provider="sambanova" size={16} />,
  },
  {
    value: 'fireworks',
    label: 'Fireworks AI',
    sublabel: 'High-speed serverless inference gateway',
    icon: <ProviderBrandIcon provider="fireworks" size={16} />,
  },
  {
    value: 'siliconflow',
    label: 'SiliconFlow',
    sublabel: 'Frontier open models (DeepSeek, Qwen)',
    icon: <ProviderBrandIcon provider="siliconflow" size={16} />,
  },
  {
    value: 'nebius',
    label: 'Nebius AI Studio',
    sublabel: 'Enterprise high-performance GPU serving',
    icon: <ProviderBrandIcon provider="nebius" size={16} />,
  },
  {
    value: 'hyperbolic',
    label: 'Hyperbolic',
    sublabel: 'Decentralized fast compute & open LLMs',
    icon: <ProviderBrandIcon provider="hyperbolic" size={16} />,
  },
  {
    value: 'cohere',
    label: 'Cohere',
    sublabel: 'Command R+, Command R Enterprise',
    icon: <ProviderBrandIcon provider="cohere" size={16} />,
  },
  {
    value: 'custom',
    label: 'Custom (OpenAI-Compatible)',
    sublabel: 'Local/Remote Gateway (vLLM, 9Router, TGI)',
    icon: <ProviderBrandIcon provider="server" size={16} />,
  },
];

export const CloudProvidersUnifiedPane: React.FC<CloudProvidersUnifiedPaneProps> = ({
  onShowToast,
  initialSubTab = 'api_keys',
}) => {
  const {
    connectedAccounts,
    handleSaveProvider,
    handleConnectOAuth,
    handleDeleteAccount,
    refreshModels,
  } = useAppContext();

  // Unified Sub-tab State
  const [subTab, setSubTab] = useState<'api_keys' | 'oauth'>(initialSubTab);

  useEffect(() => {
    if (initialSubTab) {
      setSubTab(initialSubTab);
    }
  }, [initialSubTab]);

  // Accounts separation
  const apiKeyAccounts = connectedAccounts.filter((a) => a.authType !== 'oauth' && a.providerId !== 'lmstudio');
  const oauthAccounts = connectedAccounts.filter((a) => a.authType === 'oauth');

  const [refreshingId, setRefreshingId] = useState<string | null>(null);

  // ── API Key Form State ──
  const [providerService, setProviderService] = useState('anthropic');
  const [accountName, setAccountName] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [endpointUrl, setEndpointUrl] = useState('');
  const [selectedModel, setSelectedModel] = useState('');
  const [isCustomModel, setIsCustomModel] = useState(false);
  const [customModelId, setCustomModelId] = useState('');
  const [availableModels, setAvailableModels] = useState<CustomSelectOption[]>([]);
  const [rawCatalog, setRawCatalog] = useState<ModelCatalogOption[]>([]);
  const [rawProbed, setRawProbed] = useState<ProbedModel[]>([]);
  const [isProbing, setIsProbing] = useState(false);
  const [probeStatus, setProbeStatus] = useState<{ type: 'idle' | 'success' | 'error'; message?: string }>({ type: 'idle' });
  const [fastMode, setFastMode] = useState(true);
  const [isSaving, setIsSaving] = useState(false);

  // ── OAuth State ──
  const oauthAccountId = useRef('');
  const [oauthProvider, setOauthProvider] = useState<ProviderServiceId>('antigravity');
  const [authUrlResult, setAuthUrlResult] = useState<import('@/types/domain').OAuthAuthorizeResult | null>(null);
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

  // Update default endpoint & catalog on provider change
  useEffect(() => {
    setProbeStatus({ type: 'idle' });

    const defaultEndpoints: Record<string, string> = {
      openrouter: 'https://openrouter.ai/api/v1',
      groq: 'https://api.groq.com/openai/v1',
      cerebras: 'https://api.cerebras.ai/v1',
      nvidia: 'https://integrate.api.nvidia.com/v1',
      together: 'https://api.together.xyz/v1',
      mistral: 'https://api.mistral.ai/v1',
      moonshot: 'https://api.moonshot.cn/v1',
      zhipu: 'https://open.bigmodel.cn/api/paas/v4',
      minimax: 'https://api.minimax.chat/v1',
      alibaba: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
      perplexity: 'https://api.perplexity.ai',
      xai: 'https://api.x.ai/v1',
      sambanova: 'https://api.sambanova.ai/v1',
      fireworks: 'https://api.fireworks.ai/inference/v1',
      siliconflow: 'https://api.siliconflow.cn/v1',
      nebius: 'https://api.studio.nebius.ai/v1',
      hyperbolic: 'https://api.hyperbolic.xyz/v1',
      cohere: 'https://api.cohere.com/v2',
      custom: 'http://localhost:20128/v1',
    };

    if (defaultEndpoints[providerService]) {
      setEndpointUrl(defaultEndpoints[providerService]);
    } else if (['openai', 'anthropic', 'gemini', 'deepseek'].includes(providerService)) {
      setEndpointUrl('');
    }

    const fallbackDefaultModels: Record<string, CustomSelectOption[]> = {
      openrouter: [
        { value: 'openrouter/auto', label: 'Auto (Best for Prompt)' },
        { value: 'anthropic/claude-3.7-sonnet', label: 'Claude 3.7 Sonnet' },
        { value: 'google/gemini-2.5-pro', label: 'Gemini 2.5 Pro' },
        { value: 'openai/gpt-4o', label: 'GPT-4o' },
      ],
      groq: [
        { value: 'llama-3.3-70b-versatile', label: 'Llama 3.3 70B Versatile' },
        { value: 'llama-3.1-8b-instant', label: 'Llama 3.1 8B Instant' },
      ],
      cerebras: [
        { value: 'llama-3.3-70b', label: 'Llama 3.3 70B (Fast)' },
      ],
      deepseek: [
        { value: 'deepseek-chat', label: 'DeepSeek V3' },
        { value: 'deepseek-reasoner', label: 'DeepSeek R1 Reasoner' },
      ],
      anthropic: [
        { value: 'claude-3-7-sonnet-20250219', label: 'Claude 3.7 Sonnet' },
        { value: 'claude-3-5-haiku-20241022', label: 'Claude 3.5 Haiku' },
      ],
      openai: [
        { value: 'gpt-4o', label: 'GPT-4o' },
        { value: 'o3-mini', label: 'o3-mini' },
      ],
      gemini: [
        { value: 'gemini-2.5-pro', label: 'Gemini 2.5 Pro' },
        { value: 'gemini-2.5-flash', label: 'Gemini 2.5 Flash' },
      ],
      custom: [
        { value: 'default', label: 'Default Model' },
      ],
    };

    async function loadCatalog() {
      try {
        const cat = await daemonClient.getModelCatalog(providerService);
        if (cat?.models && cat.models.length > 0) {
          setRawCatalog(cat.models);
          const opts: CustomSelectOption[] = cat.models.map((m) => {
            const ctxBadge = m.context_window ? `${Math.round(m.context_window / 1000)}k ctx` : '';
            return {
              value: m.id,
              label: m.label || m.id,
              sublabel: ctxBadge || m.description || undefined,
            };
          });
          setAvailableModels(opts);
          const def = cat.models.find((m) => m.is_default) || cat.models[0];
          setSelectedModel(def.id);
        } else if (fallbackDefaultModels[providerService]) {
          const fallbackOpts = fallbackDefaultModels[providerService];
          setAvailableModels(fallbackOpts);
          setSelectedModel(String(fallbackOpts[0].value));
        }
      } catch (_err) {
        if (fallbackDefaultModels[providerService]) {
          const fallbackOpts = fallbackDefaultModels[providerService];
          setAvailableModels(fallbackOpts);
          setSelectedModel(String(fallbackOpts[0].value));
        }
      }
    }

    void loadCatalog();
  }, [providerService]);

  // Probe Endpoint
  const handleProbeEndpoint = async () => {
    const defaultUrl = defaultProbeBaseUrl(providerService as ProviderServiceId);
    const targetUrl = endpointUrl.trim() || defaultUrl;
    if (!targetUrl) {
      setProbeStatus({ type: 'error', message: 'Enter an Endpoint URL to probe.' });
      return;
    }

    setIsProbing(true);
    setProbeStatus({ type: 'idle' });
    try {
      const res = await daemonClient.probeModels(targetUrl, providerService, apiKey.trim() || undefined);
      if (res?.models && res.models.length > 0) {
        setRawProbed(res.models);
        const probedOpts: CustomSelectOption[] = res.models.map((m) => ({
          value: m.id,
          label: m.name || m.id,
          sublabel: m.context_window ? `${Math.round(m.context_window / 1000)}k ctx · live probed` : 'Live probed',
          icon: <Cpu className="w-3.5 h-3.5 text-emerald-400" />,
        }));
        setAvailableModels(probedOpts);
        setSelectedModel(res.models[0].id);
        setIsCustomModel(false);
        setProbeStatus({
          type: 'success',
          message: `Endpoint verified! Found ${res.models.length} model(s).`,
        });
      } else {
        setProbeStatus({ type: 'error', message: 'Endpoint reachable but no model list returned.' });
      }
    } catch (err: any) {
      setProbeStatus({ type: 'error', message: `Probe failed: ${err?.message || err}` });
    } finally {
      setIsProbing(false);
    }
  };

  // Submit API Key
  const handleSubmitApiKey = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!apiKey.trim()) {
      alert('Please enter an API Key.');
      return;
    }

    const effectiveModel = isCustomModel && customModelId.trim() ? customModelId.trim() : selectedModel;
    if (!effectiveModel) {
      alert('Please select or specify a model ID.');
      return;
    }

    setIsSaving(true);
    try {
      const probedMatch = rawProbed.find((m) => m.id === effectiveModel);
      const catalogMatch = rawCatalog.find((m) => m.id === effectiveModel);
      const contextWindow = probedMatch?.context_window || catalogMatch?.context_window || 128000;

      await handleSaveProvider({
        accountName: accountName.trim() || undefined,
        service: providerService,
        apiKey: apiKey.trim(),
        endpointUrl: endpointUrl.trim() || undefined,
        model: effectiveModel,
        contextWindow,
        fastMode,
      });

      await refreshModels();
      setApiKey('');
      setAccountName('');
      setProbeStatus({ type: 'idle' });
      onShowToast?.(`API Key connected for ${providerService}.`);
    } catch (err: any) {
      setProbeStatus({ type: 'error', message: `Save failed: ${err?.message || err}` });
    } finally {
      setIsSaving(false);
    }
  };

  // ── OAuth Handlers ──
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

  const handleRefreshAccount = async (id: string) => {
    setRefreshingId(id);
    try {
      await refreshModels();
      onShowToast?.('Account credentials refreshed.');
    } finally {
      setRefreshingId(null);
    }
  };

  const handleDisconnect = async (id: string, name: string) => {
    if (window.confirm(`Disconnect account "${name}"?`)) {
      await handleDeleteAccount(id);
      await refreshModels();
      onShowToast?.(`Disconnected "${name}".`);
    }
  };

  return (
    <div className="space-y-4 max-w-4xl text-[var(--color-editor-fg)] font-sans">
      {/* ── Top Pane Header with Integrated Mode Switcher ── */}
      <div className="rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-4 sm:p-5 space-y-4 shadow-xs">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-editor-fg)] font-mono">
                Cloud Providers & Accounts
              </h2>
              <span className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2 py-0.5 font-mono text-[10px] text-[var(--color-fg-muted)]">
                {connectedAccounts.length} Connected
              </span>
            </div>
            <p className="text-[11px] text-[var(--color-fg-muted)] mt-0.5">
              Configure private API keys or authorize official accounts via OAuth 2.0 PKCE with auto-pooling.
            </p>
          </div>

          {/* Integrated Mode Switcher */}
          <div className="flex items-center rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-1 text-[11px] gap-1 shrink-0">
            <button
              type="button"
              onClick={() => setSubTab('api_keys')}
              className={`flex items-center gap-1.5 rounded-lg px-3 py-1 transition cursor-pointer ${
                subTab === 'api_keys'
                  ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] font-medium shadow-xs border border-[var(--color-border-emphasis)]'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-1)] border border-transparent'
              }`}
            >
              <Key className={`size-3 ${subTab === 'api_keys' ? 'text-emerald-400' : 'text-[var(--color-fg-subtle)]'}`} />
              <span>Cloud API Keys</span>
              <span
                className={`text-[10px] font-mono px-1.5 py-0.2 rounded-full ${
                  subTab === 'api_keys'
                    ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)]'
                    : 'bg-[var(--color-surface-2)] text-[var(--color-fg-subtle)]'
                }`}
              >
                {apiKeyAccounts.length}
              </span>
            </button>

            <button
              type="button"
              onClick={() => setSubTab('oauth')}
              className={`flex items-center gap-1.5 rounded-lg px-3 py-1 transition cursor-pointer ${
                subTab === 'oauth'
                  ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] font-medium shadow-xs border border-[var(--color-border-emphasis)]'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-1)] border border-transparent'
              }`}
            >
              <ShieldCheck className={`size-3 ${subTab === 'oauth' ? 'text-sky-400' : 'text-[var(--color-fg-subtle)]'}`} />
              <span>OAuth Sign-In</span>
              <span
                className={`text-[10px] font-mono px-1.5 py-0.2 rounded-full ${
                  subTab === 'oauth'
                    ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)]'
                    : 'bg-[var(--color-surface-2)] text-[var(--color-fg-subtle)]'
                }`}
              >
                {oauthAccounts.length}
              </span>
            </button>
          </div>
        </div>
      </div>

      {/* ── MODE 1: CLOUD API KEYS ── */}
      {subTab === 'api_keys' && (
        <div className="space-y-4">
          {/* Active Connected API Key Credentials */}
          <div className="space-y-2.5">
            <div className="text-[11px] font-semibold uppercase font-mono tracking-wider text-[var(--color-fg-subtle)] flex items-center justify-between">
              <span>Configured API Key Accounts</span>
              <span className="text-[10px] lowercase text-[var(--color-fg-subtle)]">
                {apiKeyAccounts.length} active
              </span>
            </div>

            {apiKeyAccounts.length === 0 ? (
              <div className="rounded-xl border border-dashed border-[var(--color-border-muted)] bg-[var(--color-surface-0)] p-6 text-center text-xs text-[var(--color-fg-muted)] space-y-1">
                <p>No private API keys connected yet.</p>
                <p className="text-[11px] text-[var(--color-fg-subtle)]">
                  Fill out the form below to connect your first API key.
                </p>
              </div>
            ) : (
              <div className="grid grid-cols-1 gap-2.5">
                {apiKeyAccounts.map((account) => (
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
                            {account.accountName || account.providerName}
                          </span>
                          <span className="rounded border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-1.5 py-0.2 font-mono text-[9px] text-[var(--color-fg-muted)]">
                            API Key
                          </span>
                        </div>
                        <div className="mt-0.5 font-mono text-[10px] text-[var(--color-fg-subtle)] truncate">
                          {account.providerName} · {account.defaultModel || 'Default Model'}
                          {account.endpointUrl && ` · ${account.endpointUrl}`}
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center gap-2 shrink-0">
                      <button
                        type="button"
                        onClick={() => handleRefreshAccount(account.id)}
                        disabled={refreshingId === account.id}
                        className="flex items-center gap-1 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1 font-mono text-[11px] text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)] disabled:opacity-50 transition cursor-pointer"
                      >
                        <RefreshCw className={`size-3 ${refreshingId === account.id ? 'animate-spin' : ''}`} />
                        Refresh
                      </button>
                      <button
                        type="button"
                        onClick={() => handleDisconnect(account.id, account.accountName || account.providerName)}
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

          {/* Connect New API Key Form */}
          <form
            onSubmit={handleSubmitApiKey}
            className="rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-4 sm:p-5 space-y-4 shadow-xs"
          >
            <div className="flex items-center gap-2 border-b border-[var(--color-border-muted)] pb-3">
              <Plus className="size-4 text-[var(--color-editor-fg)]" />
              <h3 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-editor-fg)] font-mono">
                Connect New Cloud Provider API Key
              </h3>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              {/* Provider Service */}
              <div className="space-y-1.5">
                <label className="text-[11px] font-mono font-medium text-[var(--color-fg-muted)]">
                  AI Provider Service
                </label>
                <CustomSelect
                  value={providerService}
                  onChange={setProviderService}
                  options={providerOptions}
                  headerTitle="Supported AI Providers"
                  headerBadge={`${providerOptions.length} Services`}
                />
              </div>

              {/* Account Label (Optional) */}
              <div className="space-y-1.5">
                <label className="text-[11px] font-mono font-medium text-[var(--color-fg-muted)]">
                  Account Label <span className="text-[10px] text-[var(--color-fg-subtle)]">(Optional)</span>
                </label>
                <input
                  type="text"
                  placeholder="e.g. Work Claude, DeepSeek Team"
                  value={accountName}
                  onChange={(e) => setAccountName(e.target.value)}
                  className="w-full rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] font-mono outline-none focus:border-[var(--color-border-emphasis)] shadow-2xs"
                />
              </div>
            </div>

            {/* API Key Secret */}
            <div className="space-y-1.5">
              <div className="flex items-center justify-between">
                <label className="text-[11px] font-mono font-medium text-[var(--color-editor-fg)]">
                  API Key Secret
                </label>
                <span className="text-[10px] text-[var(--color-fg-subtle)] font-mono">
                  Stored securely on daemon
                </span>
              </div>
              <input
                type="password"
                placeholder="sk-ant-... / sk-proj-... / deepseek-..."
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                className="w-full rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-3 py-2 text-xs text-[var(--color-editor-fg)] font-mono outline-none focus:border-[var(--color-border-emphasis)] shadow-2xs"
                required
              />
            </div>

            {/* Custom Endpoint URL & Live Probe */}
            <div className="space-y-1.5">
              <div className="flex items-center justify-between">
                <label className="text-[11px] font-mono font-medium text-[var(--color-fg-muted)]">
                  Endpoint Base URL
                </label>
                <span className="text-[10px] text-[var(--color-fg-subtle)]">Custom Gateway / Proxy</span>
              </div>
              <div className="flex gap-2">
                <input
                  type="text"
                  placeholder={defaultProbeBaseUrl(providerService as ProviderServiceId) || 'Default official endpoint'}
                  value={endpointUrl}
                  onChange={(e) => setEndpointUrl(e.target.value)}
                  className="flex-1 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] font-mono outline-none focus:border-[var(--color-border-emphasis)] shadow-2xs"
                />
                <button
                  type="button"
                  onClick={handleProbeEndpoint}
                  disabled={isProbing}
                  className="flex items-center gap-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3 py-1.5 text-xs font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)] transition disabled:opacity-50 cursor-pointer shrink-0"
                >
                  <RefreshCw className={`size-3.5 ${isProbing ? 'animate-spin' : ''}`} />
                  <span>{isProbing ? 'Scanning…' : 'Probe Models'}</span>
                </button>
              </div>

              {probeStatus.type === 'success' && (
                <div className="flex items-center gap-1.5 text-[11px] text-emerald-400 bg-emerald-500/10 border border-emerald-500/25 px-2.5 py-1.5 rounded-lg font-mono">
                  <CheckCircle2 className="size-3.5 shrink-0" />
                  <span>{probeStatus.message}</span>
                </div>
              )}
              {probeStatus.type === 'error' && (
                <div className="flex items-center gap-1.5 text-[11px] text-rose-400 bg-rose-500/10 border border-rose-500/25 px-2.5 py-1.5 rounded-lg font-mono">
                  <AlertCircle className="size-3.5 shrink-0" />
                  <span>{probeStatus.message}</span>
                </div>
              )}
            </div>

            {/* Model Selection & Custom Model Toggle */}
            <div className="space-y-1.5">
              <div className="flex items-center justify-between">
                <label className="text-[11px] font-mono font-medium text-[var(--color-fg-muted)]">
                  Default Model
                </label>
                <button
                  type="button"
                  onClick={() => {
                    setIsCustomModel(!isCustomModel);
                    if (!isCustomModel && !customModelId && selectedModel) {
                      setCustomModelId(selectedModel);
                    }
                  }}
                  className="text-[10px] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] underline cursor-pointer"
                >
                  {isCustomModel ? 'Select from list' : 'Enter custom Model ID'}
                </button>
              </div>

              {isCustomModel ? (
                <input
                  type="text"
                  placeholder="e.g. meta-llama/Llama-3.3-70B-Instruct"
                  value={customModelId}
                  onChange={(e) => setCustomModelId(e.target.value)}
                  className="w-full rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] font-mono outline-none focus:border-[var(--color-border-emphasis)]"
                />
              ) : (
                <CustomSelect
                  value={selectedModel}
                  onChange={setSelectedModel}
                  options={availableModels}
                  placeholder="Select default model"
                />
              )}
            </div>

            {/* Fast Mode Toggle */}
            <div className="flex items-center gap-2 pt-1 font-mono text-xs text-[var(--color-fg-muted)]">
              <label className="flex items-center gap-2 cursor-pointer hover:text-[var(--color-editor-fg)]">
                <input
                  type="checkbox"
                  checked={fastMode}
                  onChange={(e) => setFastMode(e.target.checked)}
                  className="accent-[var(--color-editor-fg)] rounded"
                />
                <span>Enable Fast Mode (Prioritize low-latency inference)</span>
              </label>
            </div>

            {/* Submit Actions */}
            <div className="flex justify-end pt-2 border-t border-[var(--color-border-muted)]">
              <button
                type="submit"
                disabled={isSaving || !apiKey.trim()}
                className="flex items-center gap-1.5 rounded-lg bg-[var(--color-editor-fg)] px-4 py-2 text-xs font-semibold text-[var(--color-canvas)] hover:opacity-90 active:scale-[0.98] transition disabled:opacity-50 cursor-pointer shadow-xs"
              >
                <Key className="size-3.5" />
                {isSaving ? 'Connecting…' : 'Save & Connect Provider'}
              </button>
            </div>
          </form>
        </div>
      )}

      {/* ── MODE 2: OAUTH SSO ── */}
      {subTab === 'oauth' && (
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
                        onClick={() => handleRefreshAccount(account.id)}
                        disabled={refreshingId === account.id}
                        className="flex items-center gap-1 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1 font-mono text-[11px] text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)] disabled:opacity-50 transition cursor-pointer"
                      >
                        <RefreshCw className={`size-3 ${refreshingId === account.id ? 'animate-spin' : ''}`} />
                        Refresh
                      </button>
                      <button
                        type="button"
                        onClick={() => handleDisconnect(account.id, account.accountName)}
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
              {[
                {
                  id: 'antigravity' as const,
                  name: 'Google Account (Unified)',
                  sub: '1 Login unlocks Antigravity (Claude 5.5, Gemini 3.8), Gemini-CLI & Gemini API · Multi-Account Pooling',
                  tag: 'Unified Umbrella',
                  tagColor: 'text-emerald-400 bg-emerald-500/10 border-emerald-500/25',
                },
                {
                  id: 'anthropic' as const,
                  name: 'Anthropic Claude Code',
                  sub: 'Official Claude Code OAuth PKCE · Claude 3.7 Sonnet & 3.5 Haiku',
                  tag: 'Claude Pro/Team',
                  tagColor: 'text-amber-400 bg-amber-500/10 border-amber-500/25',
                },
                {
                  id: 'openai' as const,
                  name: 'OpenAI Codex',
                  sub: 'ChatGPT Plus/Team OAuth PKCE · GPT-4o, o3-mini, o1',
                  tag: 'ChatGPT Plus',
                  tagColor: 'text-indigo-400 bg-indigo-500/10 border-indigo-500/25',
                },
                {
                  id: 'copilot' as const,
                  name: 'GitHub Copilot',
                  sub: 'Device Authorization Code · Copilot Chat & Claude 3.5 Sonnet',
                  tag: 'Device Flow',
                  tagColor: 'text-sky-400 bg-sky-500/10 border-sky-500/25',
                },
              ].map((p) => (
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
                    <button
                      type="button"
                      onClick={handleCopyLink}
                      className="flex items-center gap-1 text-[11px] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] cursor-pointer"
                    >
                      {copiedLink ? <Check className="size-3 text-emerald-400" /> : <Copy className="size-3" />}
                      <span>{copiedLink ? 'Copied Link' : 'Copy Auth Link'}</span>
                    </button>
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
                <div className="flex items-center gap-1.5 text-xs text-rose-400 bg-rose-500/10 border border-rose-500/25 p-2.5 rounded-lg font-mono">
                  <AlertCircle className="size-3.5 shrink-0" />
                  <span>{exchangeError}</span>
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
