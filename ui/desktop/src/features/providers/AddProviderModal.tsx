import React, { useState, useEffect, useRef, useCallback } from 'react';
import {
  Key,
  X,
  RefreshCw,
  Cpu,
  Zap,
  CheckCircle2,
  AlertCircle,
  ShieldCheck,
  ExternalLink,
  Loader2,
  UserCheck,
  Copy,
  Check,
  Sparkles,
} from 'lucide-react';
import { ProviderBrandIcon } from '@/components/common/AgentIcons';
import { CustomSelect, CustomSelectOption } from '@/components/common/CustomSelect';
import { daemonClient } from '@/api/daemon_client';
import { ProbedModel, ModelCatalogOption, ProviderServiceId } from '@/types';
import { useAppContext, defaultProbeBaseUrl } from '@/context/AppContext';

interface AddProviderModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSaveProvider: (providerPayload: {
    service: string;
    accountName?: string;
    apiKey: string;
    endpointUrl?: string;
    model?: string;
    contextWindow?: number;
    fastMode?: boolean;
  }) => Promise<void>;
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
    value: 'lmstudio',
    label: 'LM Studio',
    sublabel: 'Local LLM server (http://127.0.0.1:1234/v1)',
    icon: <ProviderBrandIcon provider="lmstudio" size={16} />,
  },
  {
    value: 'custom',
    label: 'Custom (OpenAI-Compatible)',
    sublabel: 'Local/Remote Gateway (vLLM, 9Router, TGI)',
    icon: <ProviderBrandIcon provider="server" size={16} />,
  },
];

const effortOptions: CustomSelectOption[] = [
  { value: 'low', label: 'Low Effort', sublabel: 'Faster response, lower cost' },
  { value: 'medium', label: 'Medium Effort', sublabel: 'Balanced reasoning (default)' },
  { value: 'high', label: 'High Effort', sublabel: 'Deep systematic reasoning' },
];

export const AddProviderModal: React.FC<AddProviderModalProps> = ({
  isOpen,
  onClose,
  onSaveProvider,
}) => {
  const { handleConnectOAuth } = useAppContext();

  // Auth Mode: Cloud API Key or OAuth SSO
  const [authMode, setAuthMode] = useState<'api_key' | 'oauth'>('api_key');

  // API Key Form State
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
  const [reasoningEffort, setReasoningEffort] = useState('medium');

  const closeAndClearSecret = () => {
    setApiKey('');
    setCallbackInput('');
    setAuthUrlResult(null);
    onClose();
  };

  useEffect(() => {
    if (!isOpen) {
      setApiKey('');
      setAccountName('');
      setIsCustomModel(false);
      setCustomModelId('');
      setProbeStatus({ type: 'idle' });
    }
  }, [isOpen]);

  // OAuth Form State
  const oauthAccountId = useRef('');
  const [oauthProvider, setOauthProvider] = useState<ProviderServiceId>('antigravity');
  const oauthProviderLabel = ({
    antigravity: 'Google Account (Unified)',
    google: 'Google Account (Unified)',
    openai: 'OpenAI Codex',
    copilot: 'GitHub Copilot',
    gemini: 'Google Gemini',
    anthropic: 'Anthropic Claude Code',
  } as Partial<Record<ProviderServiceId, string>>)[oauthProvider] || oauthProvider;
  const oauthAccountName = oauthProviderLabel;
  const oauthEmail = undefined;



  // OAuth PKCE Flow State
  const [authUrlResult, setAuthUrlResult] = useState<import('@/types/domain').OAuthAuthorizeResult | null>(null);
  const [isGeneratingUrl, setIsGeneratingUrl] = useState(false);
  const [callbackInput, setCallbackInput] = useState('');
  useEffect(() => { if (!isOpen) { setCallbackInput(''); setAuthUrlResult(null); } }, [isOpen]);
  const [isExchangingCode, setIsExchangingCode] = useState(false);
  const [exchangeError, setExchangeError] = useState<string | null>(null);
  const [exchangeSuccess, setExchangeSuccess] = useState(false);
  const [copiedLink, setCopiedLink] = useState(false);

  const [callbackStatus, setCallbackStatus] = useState<'idle' | 'listening' | 'exchanging' | 'completed' | 'failed'>('idle');
  const pollTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const stopPolling = useCallback(() => {
    if (pollTimerRef.current) {
      clearInterval(pollTimerRef.current);
      pollTimerRef.current = null;
    }
  }, []);

  const startPolling = useCallback((targetProvider: string, accountName: string, email?: string) => {
    stopPolling();
    setCallbackStatus('listening');
    pollTimerRef.current = setInterval(async () => {
      let completing = false;
      try {
        const statusRes = await daemonClient.getOAuthStatus();
        if (statusRes && statusRes.status) {
          setCallbackStatus(statusRes.status);
          if (statusRes.status === 'completed' && statusRes.provider_id === targetProvider) {
            completing = true;
            stopPolling();
            await handleConnectOAuth(targetProvider, accountName, email);
            setExchangeSuccess(true);
            setTimeout(() => {
              onClose();
            }, 900);
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
          setExchangeError(error?.message || 'Could not load the connected account. Retry.');
        }
      }
    }, 1000);
  }, [stopPolling, handleConnectOAuth, onClose]);

  // Clean up timer on unmount
  useEffect(() => {
    return () => {
      stopPolling();
      void daemonClient.cancelOAuth().catch(() => {});
    };
  }, [stopPolling]);

  // Reset & update OAuth defaults on provider switch
  const handleSelectOauthProvider = async (prov: ProviderServiceId) => {
    stopPolling();
    if (authUrlResult) {
      try { await daemonClient.cancelOAuth(); } catch { /* A disconnected daemon has no active browser flow. */ }
    }
    setCallbackStatus('idle');
    setOauthProvider(prov);
    setAuthUrlResult(null);
    setCallbackInput('');
    setExchangeError(null);
    setExchangeSuccess(false);
  };

  // Helper to open URL in default external browser (Tauri opener or window.open)
  const openExternalBrowser = async (url: string) => {
    try {
      const { openUrl } = await import('@tauri-apps/plugin-opener');
      await openUrl(url);
      return true;
    } catch (err) {
      console.warn('[AddProviderModal] plugin-opener failed, trying window.open:', err);
      try {
        window.open(url, '_blank');
      } catch (wErr) {
        console.warn('[AddProviderModal] window.open failed:', wErr);
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
      startPolling(oauthAccountId.current, oauthAccountName, oauthEmail);
      if (autoOpen && res.authorization_url) {
        try {
          await openExternalBrowser(res.authorization_url);
        } catch (browserErr) {
          console.warn('[AddProviderModal] Auto-opening browser failed, user can click open manually:', browserErr);
        }
      }
    } catch (err: any) {
      console.error('[AddProviderModal] Generate URL error:', err);
      const msg = typeof err === 'string' ? err : err?.message || (typeof err?.error === 'string' ? err.error : String(err || 'Failed to generate authorization URL'));
      setExchangeError(msg);
    } finally {
      setIsGeneratingUrl(false);
    }
  };

  const handleOpenInBrowser = async () => {
    if (!authUrlResult?.authorization_url) return;
    await openExternalBrowser(authUrlResult.authorization_url);
  };

  const handleCopyLink = () => {
    if (authUrlResult?.authorization_url) {
      navigator.clipboard.writeText(authUrlResult.authorization_url);
      setCopiedLink(true);
      setTimeout(() => setCopiedLink(false), 2000);
    }
  };

  const handleExchangeCode = async () => {
    const rawInput = callbackInput.trim();
    if (!rawInput || !authUrlResult?.code_verifier) return;
    setIsExchangingCode(true);
    setExchangeError(null);
    try {
      const returnedState = oauthProvider === 'anthropic' && !rawInput.startsWith('http') ? rawInput.split('#')[1] : new URL(rawInput).searchParams.get('state');
      if (returnedState !== authUrlResult.state) throw new Error('OAuth callback state is missing or does not match');
      stopPolling();
      await daemonClient.exchangeOAuthCode({
        code_or_url: rawInput,
        code_verifier: authUrlResult.code_verifier,
        provider_id: oauthAccountId.current,
        service_type: oauthProvider,
        redirect_uri: authUrlResult.redirect_uri,
      });
      await handleConnectOAuth(oauthAccountId.current, oauthAccountName, oauthEmail);
      setExchangeSuccess(true);
      setTimeout(() => {
        onClose();
      }, 900);
    } catch (err: any) {
      console.error('[AddProviderModal] Token exchange error:', err);
      setAuthUrlResult(null);
      const msg = typeof err === 'string' ? err : err?.message || (typeof err?.error === 'string' ? err.error : String(err || 'Token exchange failed. Start authorization again.'));
      setExchangeError(msg);
    } finally {
      setIsExchangingCode(false);
    }
  };

  // Load canonical catalog whenever provider changes
  useEffect(() => {
    if (!isOpen) return;

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
      local: 'http://localhost:11434',
    };

    if (defaultEndpoints[providerService]) {
      setEndpointUrl(defaultEndpoints[providerService]);
    } else if (providerService === 'openai' || providerService === 'anthropic' || providerService === 'gemini' || providerService === 'deepseek') {
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
        { value: 'qwen-qwq-32b', label: 'Qwen QwQ 32B' },
      ],
      cerebras: [
        { value: 'llama-3.3-70b', label: 'Llama 3.3 70B (Fast)' },
        { value: 'llama3.1-8b', label: 'Llama 3.1 8B (Fast)' },
      ],
      nvidia: [
        { value: 'meta/llama-3.3-70b-instruct', label: 'Llama 3.3 70B Instruct' },
        { value: 'nvidia/llama-3.1-nemotron-70b-instruct', label: 'Nemotron 70B Instruct' },
        { value: 'deepseek-ai/deepseek-r1', label: 'DeepSeek R1' },
      ],
      together: [
        { value: 'meta-llama/Llama-3.3-70B-Instruct-Turbo', label: 'Llama 3.3 70B Turbo' },
        { value: 'Qwen/Qwen2.5-Coder-32B-Instruct', label: 'Qwen 2.5 Coder 32B' },
      ],
      mistral: [
        { value: 'codestral-latest', label: 'Codestral Latest' },
        { value: 'mistral-large-latest', label: 'Mistral Large' },
      ],
      moonshot: [
        { value: 'kimi-k3', label: 'Kimi K3 (1M Context)' },
        { value: 'kimi-latest', label: 'Kimi Latest' },
      ],
      zhipu: [
        { value: 'glm-4-plus', label: 'GLM-4 Plus' },
        { value: 'glm-4-flash', label: 'GLM-4 Flash (Free)' },
      ],
      minimax: [
        { value: 'MiniMax-Text-01', label: 'MiniMax Text 01' },
      ],
      alibaba: [
        { value: 'qwen2.5-coder-32b-instruct', label: 'Qwen 2.5 Coder 32B' },
        { value: 'qwen-max', label: 'Qwen Max' },
      ],
      perplexity: [
        { value: 'sonar-pro', label: 'Sonar Pro' },
        { value: 'sonar', label: 'Sonar' },
      ],
      xai: [
        { value: 'grok-2-latest', label: 'Grok 2' },
        { value: 'grok-beta', label: 'Grok Beta' },
      ],
      sambanova: [
        { value: 'Meta-Llama-3.3-70B-Instruct', label: 'Llama 3.3 70B Instruct' },
        { value: 'Qwen2.5-Coder-32B-Instruct', label: 'Qwen 2.5 Coder 32B' },
        { value: 'DeepSeek-R1', label: 'DeepSeek R1' },
      ],
      fireworks: [
        { value: 'accounts/fireworks/models/deepseek-v3', label: 'DeepSeek V3' },
        { value: 'accounts/fireworks/models/deepseek-r1', label: 'DeepSeek R1' },
        { value: 'accounts/fireworks/models/llama-v3p3-70b-instruct', label: 'Llama 3.3 70B' },
      ],
      siliconflow: [
        { value: 'deepseek-ai/DeepSeek-V3', label: 'DeepSeek V3' },
        { value: 'deepseek-ai/DeepSeek-R1', label: 'DeepSeek R1' },
        { value: 'Qwen/Qwen2.5-72B-Instruct', label: 'Qwen 2.5 72B' },
      ],
      nebius: [
        { value: 'deepseek-ai/DeepSeek-V3', label: 'DeepSeek V3' },
        { value: 'deepseek-ai/DeepSeek-R1', label: 'DeepSeek R1' },
      ],
      hyperbolic: [
        { value: 'deepseek-ai/DeepSeek-V3', label: 'DeepSeek V3' },
        { value: 'meta-llama/Llama-3.3-70B-Instruct', label: 'Llama 3.3 70B' },
      ],
      cohere: [
        { value: 'command-r-plus', label: 'Command R+' },
        { value: 'command-r', label: 'Command R' },
      ],
      custom: [
        { value: 'default', label: 'Default Model' },
      ],
    };

    async function loadCatalog() {
      try {
        const cat = await daemonClient.getModelCatalog(providerService);
        if (cat && cat.models && cat.models.length > 0) {
          setRawCatalog(cat.models);
          const opts: CustomSelectOption[] = cat.models.map((m) => {
            const ctxBadge = m.context_window ? `${Math.round(m.context_window / 1000)}k ctx` : '';
            const priceBadge = m.pricing ? `$${m.pricing.input_cost_per_m}/$${m.pricing.output_cost_per_m}` : '';
            const sub = [ctxBadge, priceBadge].filter(Boolean).join(' · ');
            return {
              value: m.id,
              label: m.label || m.id,
              sublabel: sub || m.description || undefined,
            };
          });
          setAvailableModels(opts);
          const def = cat.models.find((m) => m.is_default) || cat.models[0];
          setSelectedModel(def.id);
          if (def.default_effort) {
            setReasoningEffort(def.default_effort);
          }
        } else if (fallbackDefaultModels[providerService]) {
          const fallbackOpts = fallbackDefaultModels[providerService];
          setAvailableModels(fallbackOpts);
          setSelectedModel(String(fallbackOpts[0].value));
        }
      } catch (err) {
        console.warn('[AddProviderModal] Catalog fetch warning:', err);
        if (fallbackDefaultModels[providerService]) {
          const fallbackOpts = fallbackDefaultModels[providerService];
          setAvailableModels(fallbackOpts);
          setSelectedModel(String(fallbackOpts[0].value));
        }
      }
    }

    loadCatalog();
  }, [providerService, isOpen]);

  if (!isOpen) return null;

  const handleProbeEndpoint = async () => {
    const defaultUrl = defaultProbeBaseUrl(providerService as ProviderServiceId);
    const targetUrl = endpointUrl.trim() || defaultUrl;
    if (!targetUrl) {
      setProbeStatus({ type: 'error', message: 'Please enter an Endpoint Base URL to probe (e.g. custom gateway).' });
      return;
    }

    setIsProbing(true);
    setProbeStatus({ type: 'idle' });
    try {
      const res = await daemonClient.probeModels(targetUrl, providerService, apiKey.trim() || undefined);
      if (res && res.models && res.models.length > 0) {
        setRawProbed(res.models);
        const probedOpts: CustomSelectOption[] = res.models.map((m) => {
          const ctxText = m.context_window ? `${Math.round(m.context_window / 1000)}k tokens` : 'Discovered';
          return {
            value: m.id,
            label: m.name || m.id,
            sublabel: `${ctxText} · Live probed`,
            icon: <Cpu className="w-3.5 h-3.5 text-emerald-400" />,
          };
        });
        setAvailableModels(probedOpts);
        setSelectedModel(res.models[0].id);
        setIsCustomModel(false);
        setProbeStatus({
          type: 'success',
          message: `✓ Connected! Discovered ${res.models.length} model(s) from ${targetUrl}.`,
        });
      } else {
        setProbeStatus({ type: 'error', message: 'Endpoint responded but returned no models.' });
      }
    } catch (err: any) {
      setProbeStatus({ type: 'error', message: `Endpoint probe error: ${err?.message || err}` });
    } finally {
      setIsProbing(false);
    }
  };

  const handleSubmitApiKey = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!apiKey.trim()) {
      alert('Please provide an API Key for this cloud provider.');
      return;
    }

    const effectiveModel = isCustomModel && customModelId.trim() ? customModelId.trim() : selectedModel;
    if (!effectiveModel) {
      alert('Please select a model or enter a custom Model ID.');
      return;
    }

    const probedMatch = rawProbed.find((m) => m.id === effectiveModel);
    const catalogMatch = rawCatalog.find((m) => m.id === effectiveModel);
    const contextWindow = probedMatch?.context_window || catalogMatch?.context_window || 128000;

    try {
      await onSaveProvider({
        accountName: accountName.trim() || undefined,
        service: providerService,
        apiKey: apiKey.trim(),
        endpointUrl: endpointUrl.trim() || undefined,
        model: effectiveModel,
        contextWindow,
        fastMode,
      });

      setApiKey('');
      onClose();
    } catch (err: any) {
      setProbeStatus({ type: 'error', message: `Save failed: ${err?.message || err}` });
    }
  };


  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-3 select-none">
      <div className="w-full max-w-xl bg-surface-1 border border-border-default rounded-2xl shadow-2xl flex flex-col max-h-[92vh] overflow-hidden transition-all duration-150">
        {/* Header */}
        <div className="h-12 border-b border-border-muted px-5 flex items-center justify-between shrink-0 bg-surface-1 rounded-t-2xl">
          <div className="flex items-center gap-2">
            <Key className="w-4 h-4 text-workbench-accent" />
            <span className="text-sm font-semibold text-fg-editor">
              Connect Provider or Account
            </span>
          </div>
          <button
            onClick={closeAndClearSecret}
            className="p-1 hover:bg-surface-2 rounded-md text-fg-muted hover:text-fg-editor transition cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Tab Switcher: Cloud API vs OAuth */}
        <div className="flex border-b border-border-muted bg-surface-2/40 px-5 pt-2 gap-2 text-xs shrink-0">
          <button
            type="button"
            onClick={() => setAuthMode('api_key')}
            className={`pb-2.5 px-3 font-medium transition border-b-2 flex items-center gap-1.5 cursor-pointer ${
              authMode === 'api_key'
                ? 'border-workbench-accent text-fg-editor'
                : 'border-transparent text-fg-muted hover:text-fg-editor'
            }`}
          >
            <Key className="w-3.5 h-3.5" />
            <span>Cloud API Key</span>
          </button>
          <button
            type="button"
            onClick={() => setAuthMode('oauth')}
            className={`pb-2.5 px-3 font-medium transition border-b-2 flex items-center gap-1.5 cursor-pointer ${
              authMode === 'oauth'
                ? 'border-workbench-accent text-fg-editor'
                : 'border-transparent text-fg-muted hover:text-fg-editor'
            }`}
          >
            <ShieldCheck className="w-3.5 h-3.5" />
            <span>OAuth Sign-In</span>
          </button>
        </div>

        {/* TAB 1: CLOUD API KEY */}
        {authMode === 'api_key' ? (
          <form onSubmit={handleSubmitApiKey} className="flex flex-col flex-1 min-h-0 overflow-hidden">
            <div className="p-5 space-y-4 text-xs overflow-y-auto min-h-0 flex-1">
              {/* Provider Service Picker */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <label className="font-medium text-fg-editor">Provider Service</label>
                </div>
                <CustomSelect
                  value={providerService}
                  onChange={setProviderService}
                  options={providerOptions}
                  headerTitle="Available Providers"
                  headerBadge={`${providerOptions.length} Supported`}
                />
              </div>

              {/* API Key Secret */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <label className="font-medium text-fg-editor">API Key Secret</label>
                  <span className="text-[10px] text-fg-subtle font-mono">Fail-closed & Masked</span>
                </div>
                <input
                  type="password"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  placeholder="sk-ant-... / sk-proj-..."
                  className="w-full bg-surface-0 border border-border-default focus:border-border-emphasis rounded-lg px-3 py-2 text-fg-editor placeholder-fg-subtle focus:outline-none font-mono text-xs transition"
                  autoFocus
                />
              </div>

              {/* Custom Endpoint URL */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <label className="font-medium text-fg-editor">Endpoint Base URL</label>
                  <span className="text-[10px] text-fg-subtle">Custom Gateway / Live Probe</span>
                </div>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={endpointUrl}
                    onChange={(e) => setEndpointUrl(e.target.value)}
                    placeholder={defaultProbeBaseUrl(providerService as ProviderServiceId) || 'Default cloud endpoint'}
                    className="flex-1 bg-surface-0 border border-border-default focus:border-border-emphasis rounded-lg px-3 py-2 text-fg-editor placeholder-fg-subtle focus:outline-none font-mono text-xs transition"
                  />
                  <button
                    type="button"
                    onClick={handleProbeEndpoint}
                    disabled={isProbing}
                    className="px-3 py-2 rounded-lg bg-surface-2 hover:bg-surface-3 border border-border-default hover:border-border-emphasis text-fg-editor font-medium flex items-center gap-1.5 transition text-xs shrink-0 cursor-pointer disabled:opacity-50"
                    title="Probe live models and context windows from endpoint"
                  >
                    <RefreshCw className={`w-3.5 h-3.5 ${isProbing ? 'animate-spin' : ''}`} />
                    <span>{isProbing ? 'Scanning...' : 'Probe Models'}</span>
                  </button>
                </div>

                {probeStatus.type === 'success' && (
                  <div className="flex items-center gap-1.5 text-[11px] text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 px-2.5 py-1.5 rounded-lg">
                    <CheckCircle2 className="w-3.5 h-3.5 shrink-0" />
                    <span className="truncate">{probeStatus.message}</span>
                  </div>
                )}
                {probeStatus.type === 'error' && (
                  <div className="flex items-center gap-1.5 text-[11px] text-rose-400 bg-rose-500/10 border border-rose-500/20 px-2.5 py-1.5 rounded-lg">
                    <AlertCircle className="w-3.5 h-3.5 shrink-0" />
                    <span className="truncate">{probeStatus.message}</span>
                  </div>
                )}
              </div>

              {/* Model Selection Dropdown & Custom Model Direct Entry */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <label className="font-medium text-fg-editor">Default Model</label>
                  <div className="flex items-center gap-2">
                    {rawProbed.length > 0 && !isCustomModel && (
                      <span className="text-[10px] text-emerald-400 font-mono">
                        {rawProbed.length} Live Probed
                      </span>
                    )}
                    <button
                      type="button"
                      onClick={() => {
                        setIsCustomModel(!isCustomModel);
                        if (!isCustomModel && !customModelId && selectedModel) {
                          setCustomModelId(selectedModel);
                        }
                      }}
                      className="text-[11px] text-workbench-accent hover:underline cursor-pointer flex items-center gap-1 font-medium"
                    >
                      <Sparkles className="w-3 h-3 text-amber-400" />
                      <span>{isCustomModel ? 'Choose from list' : '+ Enter new / custom model'}</span>
                    </button>
                  </div>
                </div>

                {isCustomModel ? (
                  <div className="space-y-1.5 p-3 rounded-xl border border-teal-500/30 bg-teal-500/5">
                    <div className="flex items-center justify-between">
                      <span className="text-[11px] font-medium text-fg-editor">Custom / New Model ID</span>
                      <span className="text-[10px] text-teal-400 font-mono">Instant Support</span>
                    </div>
                    <input
                      type="text"
                      value={customModelId}
                      onChange={(e) => setCustomModelId(e.target.value)}
                      placeholder="e.g. claude-3-7-sonnet-20250219, gpt-4.5-turbo, deepseek-ai/DeepSeek-V3"
                      className="w-full bg-surface-0 border border-border-default focus:border-teal-500 rounded-lg px-3 py-2 text-fg-editor placeholder-fg-subtle focus:outline-none font-mono text-xs transition"
                      autoFocus
                    />
                    <p className="text-[10px] text-fg-muted">
                      Use newly released models or custom fine-tunes immediately without waiting for catalog updates.
                    </p>
                  </div>
                ) : availableModels.length > 0 ? (
                  <CustomSelect
                    value={selectedModel}
                    onChange={setSelectedModel}
                    options={availableModels}
                    headerTitle="Select Model"
                    headerBadge={rawProbed.length > 0 ? 'Live Endpoint' : 'Curated Catalog'}
                  />
                ) : (
                  <div className="p-3 bg-surface-0 border border-border-muted rounded-xl text-center space-y-1.5">
                    <p className="text-[11px] text-fg-muted">No models loaded yet for this provider.</p>
                    <button
                      type="button"
                      onClick={handleProbeEndpoint}
                      disabled={isProbing}
                      className="text-xs text-workbench-accent hover:underline cursor-pointer inline-flex items-center gap-1.5 font-medium"
                    >
                      <RefreshCw className={`w-3 h-3 ${isProbing ? 'animate-spin' : ''}`} />
                      <span>Probe models live from endpoint</span>
                    </button>
                  </div>
                )}
              </div>

              {/* Fast Mode & Reasoning Effort */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-1">
                <div className="p-3 bg-surface-0 border border-border-muted rounded-xl space-y-2">
                  <div className="flex items-center justify-between">
                    <label className="font-medium text-fg-editor flex items-center gap-1.5">
                      <Zap className="w-3.5 h-3.5 text-amber-400" />
                      <span>Fast Mode</span>
                    </label>
                    <input
                      type="checkbox"
                      checked={fastMode}
                      onChange={(e) => setFastMode(e.target.checked)}
                      className="accent-workbench-accent cursor-pointer"
                    />
                  </div>
                  <p className="text-[10px] text-fg-muted">
                    OrCa fast tier route acceleration with speculative speed mode.
                  </p>
                </div>

                <div className="p-3 bg-surface-0 border border-border-muted rounded-xl space-y-1.5">
                  <label className="font-medium text-fg-editor block">Reasoning Effort</label>
                  <CustomSelect
                    value={reasoningEffort}
                    onChange={setReasoningEffort}
                    options={effortOptions}
                    headerTitle="Reasoning Effort"
                  />
                </div>
              </div>

              <div className="space-y-1.5">
                <label htmlFor="provider-account-name" className="font-medium text-fg-editor">Account name (optional)</label>
                <input id="provider-account-name" value={accountName} onChange={e => setAccountName(e.target.value)} placeholder="Personal, Work…" className="w-full rounded-lg border border-border-muted bg-surface-0 px-3 py-2 text-xs" />
                <p className="text-[10px] text-fg-muted">Set account priority and round-robin in Settings after connecting.</p>
              </div>
            </div>

            <div className="h-14 border-t border-border-muted px-5 flex items-center justify-end gap-2 bg-surface-1 shrink-0">
              <button
                type="button"
                onClick={closeAndClearSecret}
                className="px-3.5 py-1.5 rounded-lg hover:bg-surface-2 text-fg-muted hover:text-fg-editor transition text-xs cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="submit"
                className="btn-primary px-4 py-1.5 rounded-lg font-medium transition text-xs shadow-xs cursor-pointer"
              >
                Save & Activate Provider
              </button>
            </div>
          </form>
        ) : (
          /* TAB 2: OAUTH SSO */
          <div className="flex flex-col flex-1 min-h-0 overflow-hidden">
            <div className="p-5 space-y-4 text-xs overflow-y-auto min-h-0 flex-1">
              <div className="p-3 bg-indigo-500/10 rounded-xl flex items-start gap-3">
                <ShieldCheck className="w-5 h-5 text-indigo-400 shrink-0 mt-0.5" />
                <div className="space-y-0.5">
                  <div className="font-semibold text-fg-editor">OAuth 2.0 Sovereign Authorization & Multi-Account Pooling</div>
                  <div className="text-[11px] text-fg-muted leading-relaxed">
                    Authorize securely via PKCE or Device Flow. Custos supports connecting multiple accounts per provider (e.g. 2 Google accounts) with automatic round-robin pooling and in-flight 429 rate limit failover.
                  </div>
                </div>
              </div>

              {/* OAuth Providers */}
              <div className="space-y-1.5">
                <label className="font-medium text-fg-editor">Select OAuth Identity Provider</label>
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5 pt-2">
                  {[
                    {
                      id: 'antigravity',
                      name: 'Google Account (Unified)',
                      sub: '1 Login unlocks Antigravity (Claude 5.5, Gemini 3.8), Gemini-CLI & Gemini API · Multi-Account Pooling',
                      tag: 'Unified Umbrella',
                      tagColor: 'text-emerald-400 bg-emerald-500/10 border-emerald-500/20',
                    },
                    {
                      id: 'anthropic',
                      name: 'Anthropic Claude Code',
                      sub: 'Official Claude Code OAuth PKCE · Claude 3.7 Sonnet & 3.5 Haiku',
                      tag: 'Claude Pro/Team',
                      tagColor: 'text-amber-400 bg-amber-500/10 border-amber-500/20',
                    },
                    {
                      id: 'openai',
                      name: 'OpenAI Codex',
                      sub: 'ChatGPT Plus/Team OAuth PKCE · GPT-4o, o3-mini, o1',
                      tag: 'ChatGPT Plus',
                      tagColor: 'text-indigo-400 bg-indigo-500/10 border-indigo-500/20',
                    },
                    {
                      id: 'copilot',
                      name: 'GitHub Copilot',
                      sub: 'Device Authorization Code · Copilot Chat & Claude 3.5 Sonnet',
                      tag: 'Device Flow',
                      tagColor: 'text-sky-400 bg-sky-500/10 border-sky-500/20',
                    },
                  ].map((p) => (
                    <button
                      key={p.id}
                      type="button"
                      onClick={() => handleSelectOauthProvider(p.id as ProviderServiceId)}
                      className={`p-3 rounded-xl text-left transition cursor-pointer flex flex-col justify-between border ${
                        oauthProvider === p.id
                          ? 'bg-surface-2 border-workbench-accent text-fg-editor shadow-xs'
                          : 'bg-surface-0 border-border-default hover:border-border-emphasis text-fg-muted'
                      }`}
                    >
                      <div className="flex items-start justify-between gap-2 w-full">
                        <div className="flex items-center gap-2">
                          <div className="w-5 h-5 shrink-0 flex items-center justify-center">
                            <ProviderBrandIcon provider={p.id === 'antigravity' ? 'gemini' : p.id} size={20} />
                          </div>
                          <div className="font-semibold text-xs text-fg-editor">{p.name}</div>
                        </div>
                        {p.tag && (
                          <span className={`text-[9px] font-mono px-1.5 py-0.2 rounded border shrink-0 ${p.tagColor}`}>
                            {p.tag}
                          </span>
                        )}
                      </div>
                      <div className="text-[10px] text-fg-subtle mt-2 line-clamp-2 leading-relaxed">{p.sub}</div>
                    </button>
                  ))}
                </div>
              </div>

              {/* OpenAI PKCE Interactive Box */}
              {(
                <div className="p-3.5 bg-surface-0 rounded-xl space-y-3 font-mono text-xs">
                  <div className="flex items-center justify-between text-fg-subtle text-[11px]">
                    <span className="font-semibold text-fg-editor">{oauthProvider === 'copilot' ? 'GitHub Device Authorization' : `${oauthProviderLabel} PKCE Flow (RFC 7636)`}</span>
                    <span className="text-[10px] text-emerald-400 bg-emerald-500/10 px-2 py-0.5 rounded">
                      {oauthProvider === 'copilot' ? 'Device Flow' : 'PKCE S256'}
                    </span>
                  </div>

                  {!authUrlResult ? (
                    <div className="flex flex-col items-center justify-center p-4 bg-surface-2/60 rounded-xl space-y-3">
                      <p className="text-[11px] text-fg-muted text-center font-sans">
                        Click below to authorize {oauthProviderLabel} in your browser. {oauthProvider === 'copilot' ? 'Enter the device code shown here on GitHub.' : oauthProvider === 'anthropic' ? 'Paste the authorization code#state from Claude below.' : 'Custos captures the callback automatically; manual paste is available below.'}
                      </p>
                      <button
                        type="button"
                        onClick={() => handleGenerateAuthorizeUrl(true)}
                        disabled={isGeneratingUrl}
                        className="provider-oauth-action px-4 py-2 rounded-lg text-xs font-sans font-medium flex items-center gap-2 cursor-pointer disabled:opacity-50 shadow-sm"
                      >
                        {isGeneratingUrl ? (
                          <>
                            <Loader2 className="w-3.5 h-3.5 animate-spin" />
                            <span>Opening authorization portal...</span>
                          </>
                        ) : (
                          <>
                            <ExternalLink className="w-3.5 h-3.5" />
                            <span>Log in with {oauthProviderLabel} (Opens Browser)</span>
                          </>
                        )}
                      </button>

                      {exchangeError && (
                        <div className="p-2.5 bg-red-500/10 border border-red-500/20 rounded-lg text-red-400 text-[11px] flex items-center gap-2 w-full font-sans">
                          <AlertCircle className="w-3.5 h-3.5 shrink-0" />
                          <span className="truncate">{exchangeError}</span>
                        </div>
                      )}
                    </div>
                  ) : (
                    <div className="space-y-3 font-sans">
                      {authUrlResult.user_code && <div className="p-4 bg-surface-2 border border-border-emphasis rounded-lg text-center space-y-2"><p className="text-xs text-fg-muted">Enter this code on GitHub to authorize Copilot:</p><code className="text-xl tracking-widest text-fg-editor select-all">{authUrlResult.user_code}</code></div>}
                      {/* Auto Callback Listener Status Banner */}
                      <div className="p-3 bg-surface-2 border border-border-default rounded-xl text-xs space-y-1.5 font-sans">
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-2">
                            {callbackStatus === 'listening' ? (
                              <span className="relative flex h-2.5 w-2.5">
                                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
                                <span className="relative inline-flex rounded-full h-2.5 w-2.5 bg-emerald-500"></span>
                              </span>
                            ) : callbackStatus === 'exchanging' ? (
                              <Loader2 className="w-3.5 h-3.5 text-indigo-400 animate-spin" />
                            ) : (
                              <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
                            )}
                            <span className="font-medium text-fg-editor">
                              {callbackStatus === 'listening'
                                ? 'Waiting for browser authorization...'
                                : callbackStatus === 'exchanging'
                                ? 'Exchanging provider tokens...'
                                : callbackStatus === 'failed' ? 'Authorization failed' : callbackStatus === 'completed' ? 'Authorization complete' : 'Authorization pending'}
                            </span>
                          </div>
                          <span className="text-[10px] text-fg-muted font-mono bg-surface-1 px-1.5 py-0.5 rounded border border-border-muted">
                            {oauthProvider === 'copilot' ? 'GitHub Device Flow' : authUrlResult.redirect_uri}
                          </span>
                        </div>
                        <p className="text-[11px] text-fg-muted">
                          {oauthProvider === 'anthropic' ? 'After authorizing, paste the code#state shown by Claude below.' : 'Complete authorization in your browser; Custos waits for the provider response.'}
                        </p>
                      </div>

                      {/* URL Display & Action Buttons */}
                      <div className="space-y-1.5">
                        <label className="text-[11px] font-medium text-fg-editor">1. Provider Authorization URL</label>
                        <div className="flex items-center gap-2">
                          <input
                            type="text"
                            readOnly
                            value={authUrlResult.authorization_url}
                            className="flex-1 bg-surface-2 border border-border-muted rounded-lg px-2.5 py-1.5 text-[11px] text-fg-subtle font-mono truncate select-all focus:outline-none"
                          />
                          <button
                            type="button"
                            onClick={handleOpenInBrowser}
                            className="provider-oauth-action px-2.5 py-1.5 rounded-lg hover:brightness-110 text-xs font-medium flex items-center gap-1 shrink-0 cursor-pointer"
                            title="Re-open in Browser"
                          >
                            <ExternalLink className="w-3.5 h-3.5" />
                            <span>Re-open</span>
                          </button>
                          <button
                            type="button"
                            onClick={handleCopyLink}
                            className="px-2.5 py-1.5 bg-surface-2 hover:bg-surface-3 text-fg-editor rounded-lg border border-border-default text-xs font-medium flex items-center gap-1 shrink-0 cursor-pointer"
                            title="Copy to clipboard"
                          >
                            {copiedLink ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                            <span>{copiedLink ? 'Copied' : 'Copy'}</span>
                          </button>
                        </div>
                      </div>

                      {/* Callback Input & Exchange */}
                      {oauthProvider !== 'copilot' && <div className="space-y-1.5">
                        <label className="text-[11px] font-medium text-fg-editor">
                          2. {oauthProvider === 'anthropic' ? 'Paste authorization code#state' : 'Manual Fallback: Paste Callback URL'}
                        </label>
                        <p className="text-[11px] text-fg-muted">
                          {oauthProvider === "anthropic" ? "Paste the complete authorization code and state shown by Claude." : "If automatic capture is blocked, copy the redirect URL from your browser address bar (including code and state) and paste it below."}
                        </p>
                        <input
                          type="text"
                          value={callbackInput}
                          onChange={(e) => setCallbackInput(e.target.value)}
                          placeholder={oauthProvider === "anthropic" ? "code#state" : `${authUrlResult?.redirect_uri || "Callback URL"}?code=...&state=...`}
                          className="w-full bg-surface-0 border border-border-default focus:border-border-emphasis rounded-lg px-3 py-2 text-fg-editor focus:outline-none text-xs font-mono transition"
                        />
                      </div>}

                      {exchangeError && (
                        <div className="p-2.5 bg-red-500/10 border border-red-500/20 rounded-lg text-red-400 text-[11px] flex items-center gap-2 font-sans">
                          <AlertCircle className="w-3.5 h-3.5 shrink-0" />
                          <span className="truncate">{exchangeError}</span>
                        </div>
                      )}

                      {exchangeSuccess && (
                        <div className="p-2.5 bg-emerald-500/10 border border-emerald-500/20 rounded-lg text-emerald-400 text-[11px] flex items-center gap-2 font-sans">
                          <CheckCircle2 className="w-3.5 h-3.5 shrink-0" />
                          <span>Connected successfully! Stored tokens & activated live models.</span>
                        </div>
                      )}

                      {oauthProvider !== 'copilot' && <button
                        type="button"
                        onClick={handleExchangeCode}
                        disabled={!callbackInput.trim() || isExchangingCode || exchangeSuccess}
                        className="w-full provider-oauth-action py-2 rounded-lg text-xs font-medium flex items-center justify-center gap-2 cursor-pointer disabled:opacity-50"
                      >
                        {isExchangingCode ? (
                          <>
                            <Loader2 className="w-3.5 h-3.5 animate-spin" />
                            <span>Exchanging OAuth Token...</span>
                          </>
                        ) : exchangeSuccess ? (
                          <>
                            <CheckCircle2 className="w-3.5 h-3.5" />
                            <span>Verified & Connected!</span>
                          </>
                        ) : (
                          <>
                            <Zap className="w-3.5 h-3.5" />
                            <span>Verify & Connect Provider</span>
                          </>
                        )}
                      </button>}
                    </div>
                  )}
                </div>
              )}
            </div>

            {/* Footer */}
            <div className="h-14 px-5 flex items-center justify-end gap-2 bg-surface-1 shrink-0">
              <button
                type="button"
                onClick={closeAndClearSecret}
                className="px-3.5 py-1.5 rounded-lg hover:bg-surface-2 text-fg-muted hover:text-fg-editor transition text-xs cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => {
                  if (!exchangeSuccess) {
                    if (!authUrlResult) {
                      handleGenerateAuthorizeUrl(true);
                    } else if (callbackInput.trim()) {
                      handleExchangeCode();
                    } else {
                      handleOpenInBrowser();
                    }
                  }
                }}
                disabled={exchangeSuccess || isGeneratingUrl || isExchangingCode}
                className="btn-primary px-4 py-1.5 rounded-lg font-medium transition text-xs shadow-xs cursor-pointer flex items-center gap-1.5 disabled:opacity-50"
              >
                {isExchangingCode ? (
                  <>
                    <Loader2 className="w-3.5 h-3.5 animate-spin" />
                    <span>Connecting OAuth...</span>
                  </>
                ) : exchangeSuccess ? (
                  <>
                    <UserCheck className="w-3.5 h-3.5 text-emerald-400" />
                    <span>Connected!</span>
                  </>
                ) : (
                  <>
                    <span>
                      {!authUrlResult
                        ? `Log in with ${oauthProviderLabel}`
                        : callbackInput.trim()
                        ? 'Verify & Connect'
                        : 'Authorize & Connect Account'}
                    </span>
                  </>
                )}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
