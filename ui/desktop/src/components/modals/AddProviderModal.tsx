import React, { useState, useEffect, useRef, useCallback } from 'react';
import {
  Key,
  X,
  RefreshCw,
  Cpu,
  Server,
  Zap,
  CheckCircle2,
  AlertCircle,
  ShieldCheck,
  ExternalLink,
  Loader2,
  UserCheck,
  Copy,
  Check,
  FolderPlus,
} from 'lucide-react';
import { ProviderBrandIcon } from '@/components/common/AgentIcons';
import { CustomSelect, CustomSelectOption } from '../common/CustomSelect';
import { daemonClient } from '@/api/daemon_client';
import { ProbedModel, ModelCatalogOption, ProviderServiceId } from '@/types';
import type { LocalHardwareProfile } from '@/types/domain';
import { useAppContext } from '@/context/AppContext';

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
  const { handleConnectOAuth, showToast } = useAppContext();

  // Auth Mode: API Key, OAuth SSO, or Local AI
  const [authMode, setAuthMode] = useState<'api_key' | 'oauth' | 'local'>('api_key');
  const [localSubTab, setLocalSubTab] = useState<'native' | 'server'>('native');

  // API Key Form State
  const [providerService, setProviderService] = useState('anthropic');
  const [accountName, setAccountName] = useState('');
  const [apiKey, setApiKey] = useState('');
  const closeAndClearSecret = () => {
    setApiKey('');
    setLocalApiKey('');
    setCallbackInput('');
    setAuthUrlResult(null);
    onClose();
  };
  useEffect(() => {
    if (!isOpen) {
      setApiKey('');
      setLocalApiKey('');
      setAccountName('');
    }
  }, [isOpen]);
  const [endpointUrl, setEndpointUrl] = useState('');
  const [selectedModel, setSelectedModel] = useState('');
  const [availableModels, setAvailableModels] = useState<CustomSelectOption[]>([]);
  const [rawCatalog, setRawCatalog] = useState<ModelCatalogOption[]>([]);
  const [rawProbed, setRawProbed] = useState<ProbedModel[]>([]);
  const [isProbing, setIsProbing] = useState(false);
  const [probeStatus, setProbeStatus] = useState<{ type: 'idle' | 'success' | 'error'; message?: string }>({ type: 'idle' });

  // Local Inference Server State
  type LocalPreset = 'ollama' | 'lm_studio' | 'vllm' | 'custom';
  const [localPreset, setLocalPreset] = useState<LocalPreset>('ollama');
  const [localUrl, setLocalUrl] = useState('http://localhost:11434');
  const [localAccountName, setLocalAccountName] = useState('Ollama Local');
  const [localApiKey, setLocalApiKey] = useState('');
  const [localModels, setLocalModels] = useState<ProbedModel[]>([]);
  const [localModel, setLocalModel] = useState('');
  const [localError, setLocalError] = useState('');
  const [localConnected, setLocalConnected] = useState(false);
  const [localBusy, setLocalBusy] = useState(false);
  const localGeneration = useRef(0);

  // Native GGUF Registration State
  const [nativeFolderPath, setNativeFolderPath] = useState('');
  const [nativeFolderLabel, setNativeFolderLabel] = useState('');
  const [isRegisteringFolder, setIsRegisteringFolder] = useState(false);
  const [nativeError, setNativeError] = useState<string | null>(null);
  const [nativeHw, setNativeHw] = useState<LocalHardwareProfile | null>(null);

  useEffect(() => {
    if (isOpen) {
      daemonClient.getLocalHardware().then(setNativeHw).catch(() => {});
    }
  }, [isOpen]);

  const handleRegisterNativeFolder = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!nativeFolderPath.trim()) {
      setNativeError('Please specify a directory path containing .gguf models.');
      return;
    }
    setNativeError(null);
    setIsRegisteringFolder(true);
    try {
      await daemonClient.addLocalFolder(
        nativeFolderPath.trim(),
        nativeFolderLabel.trim() || undefined
      );
      await daemonClient.scanLocalModels();
      showToast('Local GGUF models registered and scanned.');
      onClose();
    } catch (err: any) {
      setNativeError(err.message || 'Failed to register model directory');
    } finally {
      setIsRegisteringFolder(false);
    }
  };

  const handleSelectPreset = (preset: LocalPreset) => {
    setLocalPreset(preset);
    localGeneration.current++;
    setLocalConnected(false);
    setLocalModels([]);
    setLocalModel('');
    setLocalError('');
    switch (preset) {
      case 'ollama':
        setLocalUrl('http://localhost:11434');
        setLocalAccountName('Ollama Local');
        break;
      case 'lm_studio':
        setLocalUrl('http://localhost:1234/v1');
        setLocalAccountName('LM Studio Local');
        break;
      case 'vllm':
        setLocalUrl('http://localhost:8000/v1');
        setLocalAccountName('vLLM Local');
        break;
      case 'custom':
        setLocalAccountName('Local Server');
        break;
    }
  };

  const connectLocal = async () => {
    const generation = ++localGeneration.current;
    setLocalBusy(true);
    setLocalError('');
    setLocalConnected(false);
    try {
      const probeType = localPreset === 'ollama' || localUrl.includes(':11434') ? 'ollama' : 'local';
      const result = await daemonClient.probeModels(
        localUrl.trim(),
        probeType,
        localApiKey.trim() || undefined
      );
      if (generation !== localGeneration.current) return;
      const models = result?.models || [];
      setLocalModels(models);
      setLocalModel(models.some((m) => m.id === localModel) ? localModel : models[0]?.id || '');
      setLocalConnected(true);
      if (!models.length) {
        setLocalError(
          localPreset === 'ollama'
            ? 'No installed models detected. Run `ollama pull <model>` in terminal, then scan again.'
            : 'Server responded, but no models were discovered. Verify that at least one model is active.'
        );
      } else if (localModel && !models.some((m) => m.id === localModel)) {
        setLocalError('The previously selected model was not found on this server.');
      }
    } catch (err: any) {
      if (generation !== localGeneration.current) return;
      setLocalModels([]);
      setLocalModel('');
      setLocalError(
        `Connection failed: ${err?.message || err}. Verify ${
          localPreset === 'ollama' ? 'Ollama is running (`ollama serve`)' : 'the local model server is started'
        } at ${localUrl.trim()}.`
      );
    } finally {
      if (generation === localGeneration.current) setLocalBusy(false);
    }
  };

  const saveLocal = async (event: React.FormEvent) => {
    event.preventDefault();
    const model = localModels.find((m) => m.id === localModel);
    if (!localConnected || !model || localBusy) return;
    setLocalBusy(true);
    setLocalError('');
    try {
      await onSaveProvider({
        service: 'local',
        accountName: localAccountName.trim() || `${localPreset.toUpperCase()} Local`,
        apiKey: localApiKey.trim(),
        endpointUrl: localUrl.trim(),
        model: model.id,
        contextWindow: model.context_window || undefined,
        fastMode: true,
      });
      onClose();
    } catch (err: any) {
      setLocalError(`Save failed: ${err?.message || err}`);
    } finally {
      setLocalBusy(false);
    }
  };
  const [fastMode, setFastMode] = useState(true);
  const [reasoningEffort, setReasoningEffort] = useState('medium');

  useEffect(() => {
    if (!isOpen) {
      localGeneration.current++;
      setLocalConnected(false);
      setLocalModels([]);
      setLocalModel('');
      setLocalError('');
      setLocalBusy(false);
    }
  }, [isOpen]);

  // OAuth Form State
  const oauthAccountId = useRef('');
  const [oauthProvider, setOauthProvider] = useState<ProviderServiceId>('openai');
  const oauthProviderLabel = ({ openai: 'OpenAI', antigravity: 'Antigravity', copilot: 'GitHub Copilot', gemini: 'Gemini', anthropic: 'Claude' } as Partial<Record<ProviderServiceId, string>>)[oauthProvider] || oauthProvider;
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
    } catch {
      window.open(url, '_blank');
      return true;
    }
  };

  const handleGenerateAuthorizeUrl = async (autoOpen = true) => {
    setIsGeneratingUrl(true);
    setExchangeError(null);
    try {
      oauthAccountId.current = `${oauthProvider}-${crypto.randomUUID()}`;
      const res = await daemonClient.getOAuthAuthorizeUrl({ provider_id: oauthAccountId.current, service_type: oauthProvider });
      if (!res?.authorization_url) throw new Error('Daemon did not return an authorization URL');
      setAuthUrlResult(res);
      startPolling(oauthAccountId.current, oauthAccountName, oauthEmail);
      if (autoOpen && res.authorization_url) {
        await openExternalBrowser(res.authorization_url);
      }
    } catch (err: any) {
      console.error('[AddProviderModal] Generate URL error:', err);
      setExchangeError(err?.message || 'Failed to generate authorization URL');
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
      setExchangeError(err?.message || 'Token exchange failed. Start authorization again.');
    } finally {
      setIsExchangingCode(false);
    }
  };

  // Load canonical catalog whenever provider changes
  useEffect(() => {
    if (!isOpen) return;

    setProbeStatus({ type: 'idle' });

    if (providerService === 'local' && !endpointUrl) {
      setEndpointUrl('http://localhost:11434');
    }

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
        }
      } catch (err) {
        console.warn('[AddProviderModal] Catalog fetch warning:', err);
      }
    }

    loadCatalog();
  }, [providerService, isOpen]);

  if (!isOpen) return null;

  const handleProbeEndpoint = async () => {
    const targetUrl = endpointUrl.trim();
    if (!targetUrl) {
      setProbeStatus({ type: 'error', message: 'Vui lòng nhập Endpoint URL để quét (ví dụ: custom gateway).' });
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
        setProbeStatus({
          type: 'success',
          message: `✓ Đã kết nối thành công! Quét được ${res.models.length} model từ ${targetUrl}.`,
        });
      } else {
        setProbeStatus({ type: 'error', message: 'Endpoint phản hồi nhưng không tìm thấy danh sách model.' });
      }
    } catch (err: any) {
      setProbeStatus({ type: 'error', message: `Lỗi quét endpoint: ${err?.message || err}` });
    } finally {
      setIsProbing(false);
    }
  };

  const handleSubmitApiKey = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!apiKey.trim()) {
      alert('Vui lòng nhập API Key cho cloud provider.');
      return;
    }

    const probedMatch = rawProbed.find((m) => m.id === selectedModel);
    const catalogMatch = rawCatalog.find((m) => m.id === selectedModel);
    const contextWindow = probedMatch?.context_window || catalogMatch?.context_window || 128000;

    try {
      await onSaveProvider({
        accountName: accountName.trim() || undefined,
        service: providerService,
        apiKey: apiKey.trim(),
        endpointUrl: endpointUrl.trim() || undefined,
        model: selectedModel || undefined,
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
      <div className={`w-full ${authMode === 'local' ? 'max-w-3xl' : 'max-w-xl'} bg-surface-1 border border-border-default rounded-2xl shadow-2xl flex flex-col max-h-[92vh] overflow-hidden transition-all duration-150`}>
        {/* Header */}
        <div className="h-12 border-b border-border-muted px-5 flex items-center justify-between shrink-0 bg-surface-1 rounded-t-2xl">
          <div className="flex items-center gap-2">
            <Key className="w-4 h-4 text-workbench-accent" />
            <span className="text-sm font-semibold text-fg-editor">
              {authMode === 'local' ? 'Add Local Models' : 'Connect Provider or Account'}
            </span>
          </div>
          <button
            onClick={closeAndClearSecret}
            className="p-1 hover:bg-surface-2 rounded-md text-fg-muted hover:text-fg-editor transition cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Tab Switcher: Cloud API vs OAuth vs Local */}
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
          <button
            type="button"
            onClick={() => setAuthMode('local')}
            className={`pb-2.5 px-3 font-medium transition border-b-2 flex items-center gap-1.5 cursor-pointer ${
              authMode === 'local'
                ? 'border-workbench-accent text-fg-editor'
                : 'border-transparent text-fg-muted hover:text-fg-editor'
            }`}
          >
            <Cpu className="w-3.5 h-3.5 text-teal-400" />
            <span>Local Models (on Mac)</span>
          </button>
        </div>

        {/* TAB: LOCAL AI (NATIVE GGUF & LOCAL SERVERS) */}
        {authMode === 'local' ? (
          <div className="flex flex-col flex-1 min-h-0 overflow-hidden">
            {/* Sub-tab Switcher: Model Files vs Ollama */}
            <div className="flex items-center justify-between px-5 py-2.5 bg-surface-2/30 border-b border-border-muted shrink-0">
              <div className="flex items-center gap-1.5 p-0.5 bg-surface-0 rounded-lg border border-border-default/60">
                <button
                  type="button"
                  onClick={() => setLocalSubTab('native')}
                  className={`px-3 py-1 rounded-md text-xs font-medium transition cursor-pointer flex items-center gap-1.5 ${
                    localSubTab === 'native'
                      ? 'bg-surface-2 text-teal-300 shadow-xs'
                      : 'text-fg-muted hover:text-fg-editor'
                  }`}
                >
                  <Cpu className="w-3.5 h-3.5 text-teal-400" />
                  <span>Model Files (.gguf)</span>
                </button>
                <button
                  type="button"
                  onClick={() => setLocalSubTab('server')}
                  className={`px-3 py-1 rounded-md text-xs font-medium transition cursor-pointer flex items-center gap-1.5 ${
                    localSubTab === 'server'
                      ? 'bg-surface-2 text-workbench-accent shadow-xs'
                      : 'text-fg-muted hover:text-fg-editor'
                  }`}
                >
                  <Server className="w-3.5 h-3.5 text-workbench-accent" />
                  <span>Ollama / Local Service</span>
                </button>
              </div>
              <div className="text-[11px] text-fg-subtle hidden sm:block">
                {localSubTab === 'native' ? 'Runs in Custos; no model server required' : 'Connects to localhost HTTP service'}
              </div>
            </div>

            {localSubTab === 'native' ? (
              <form onSubmit={handleRegisterNativeFolder} className="flex flex-col flex-1 min-h-0 overflow-hidden">
                <div className="p-5 space-y-4 text-xs overflow-y-auto min-h-0 flex-1">
                  {/* Hardware Readiness Badge */}
                  <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-3.5">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <Cpu className="h-4 w-4 text-teal-400" />
                        <span className="font-semibold text-xs text-[var(--color-editor-fg)]">
                          Native Apple Silicon Metal Engine
                        </span>
                      </div>
                      <span className="rounded-full bg-teal-500/10 border border-teal-500/30 px-2 py-0.5 text-[10px] font-medium text-teal-400">
                        {nativeHw?.is_apple_silicon ? 'Hardware Ready' : 'CPU Fallback'}
                      </span>
                    </div>
                    <div className="mt-2 grid grid-cols-2 gap-2 text-[11px] text-[var(--color-fg-muted)]">
                      <div>
                        Processor: <span className="font-mono text-[var(--color-editor-fg)]">{nativeHw?.chip_name || 'Apple M2 Pro'}</span>
                      </div>
                      <div>
                        Unified RAM: <span className="font-mono text-cyan-400">{nativeHw?.total_ram_bytes ? `${(nativeHw.total_ram_bytes / (1024**3)).toFixed(1)} GB` : '32.0 GB'}</span>
                      </div>
                    </div>
                  </div>

                  {/* Directory Input */}
                  <div className="space-y-1.5">
                    <label className="font-medium text-[var(--color-editor-fg)] flex items-center justify-between">
                      <span>Model Directory on Disk</span>
                      <span className="text-[10px] text-[var(--color-fg-subtle)]">Scans for .gguf files recursively</span>
                    </label>
                    <div className="relative">
                      <input
                        type="text"
                        value={nativeFolderPath}
                        onChange={(e) => setNativeFolderPath(e.target.value)}
                        placeholder="/Users/mac/models or ~/.custos/models"
                        className="w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-3 py-2 text-xs text-[var(--color-editor-fg)] font-mono placeholder:text-[var(--color-fg-subtle)] focus:border-workbench-accent focus:outline-hidden"
                      />
                    </div>
                    {/* Quick suggestion pills */}
                    <div className="flex items-center gap-2 pt-1">
                      <span className="text-[10px] text-[var(--color-fg-subtle)]">Common paths:</span>
                      <button
                        type="button"
                        onClick={() => setNativeFolderPath('~/.custos/models')}
                        className="rounded border border-[var(--color-border-muted)] px-2 py-0.5 text-[10px] text-[var(--color-fg-muted)] hover:border-[var(--color-border-default)] hover:text-[var(--color-editor-fg)] font-mono transition"
                      >
                        ~/.custos/models
                      </button>
                      <button
                        type="button"
                        onClick={() => setNativeFolderPath('~/models')}
                        className="rounded border border-[var(--color-border-muted)] px-2 py-0.5 text-[10px] text-[var(--color-fg-muted)] hover:border-[var(--color-border-default)] hover:text-[var(--color-editor-fg)] font-mono transition"
                      >
                        ~/models
                      </button>
                    </div>
                  </div>

                  {/* Label Input */}
                  <div className="space-y-1.5">
                    <label className="font-medium text-[var(--color-editor-fg)] flex items-center justify-between">
                      <span>Folder Label / Category (Optional)</span>
                      <span className="text-[10px] text-[var(--color-fg-subtle)]">How this group appears in model selector</span>
                    </label>
                    <input
                      type="text"
                      value={nativeFolderLabel}
                      onChange={(e) => setNativeFolderLabel(e.target.value)}
                      placeholder="e.g. Local GGUF LLMs"
                      className="w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-3 py-2 text-xs text-[var(--color-editor-fg)] placeholder:text-[var(--color-fg-subtle)] focus:border-workbench-accent focus:outline-hidden"
                    />
                  </div>

                  {nativeError && (
                    <div className="flex items-center gap-2 rounded-md bg-rose-500/10 border border-rose-500/20 p-2.5 text-xs text-rose-400">
                      <AlertCircle className="w-4 h-4 shrink-0" />
                      <span>{nativeError}</span>
                    </div>
                  )}

                  {/* Direct Hardware Inference Callout */}
                  <div className="rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-1)]/60 p-3 text-[11px] text-[var(--color-fg-muted)] space-y-1">
                    <div className="font-semibold text-[var(--color-editor-fg)]">Direct On-Device Inference</div>
                    <p>
                      Custos loads .gguf models directly into Apple Silicon Unified RAM using Metal GPU acceleration, running completely offline with no external apps required.
                    </p>
                  </div>
                </div>

                {/* Modal Action Footer */}
                <div className="flex items-center justify-between border-t border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4">
                  <button
                    type="button"
                    onClick={onClose}
                    className="rounded-md px-3 py-1.5 text-xs text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)] transition cursor-pointer"
                  >
                    Cancel
                  </button>
                  <button
                    type="submit"
                    disabled={isRegisteringFolder || !nativeFolderPath.trim()}
                    className="flex items-center gap-1.5 rounded-md bg-[var(--color-editor-fg)] px-4 py-1.5 text-xs font-semibold text-[var(--color-canvas)] hover:opacity-90 transition disabled:opacity-50 cursor-pointer"
                  >
                    {isRegisteringFolder ? (
                      <>
                        <Loader2 className="w-3.5 h-3.5 animate-spin" />
                        <span>Registering & Scanning...</span>
                      </>
                    ) : (
                      <>
                        <FolderPlus className="w-3.5 h-3.5" />
                        <span>Add & Scan GGUF Models</span>
                      </>
                    )}
                  </button>
                </div>
              </form>
            ) : (
          <form onSubmit={saveLocal} className="flex flex-col flex-1 min-h-0 overflow-hidden">
            <div className="p-5 space-y-4 text-xs overflow-y-auto min-h-0 flex-1">
              {/* Presets Header */}
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <label className="font-medium text-fg-editor">Local Ollama or HTTP Endpoint</label>
                  <span className="text-[10px] text-fg-subtle">Runs in background</span>
                </div>
                <div className="flex gap-2">
                  <button
                    type="button"
                    onClick={() => handleSelectPreset('ollama')}
                    className={`px-3 py-2 rounded-lg border text-left transition flex-1 flex items-center justify-between cursor-pointer ${
                      localPreset === 'ollama'
                        ? 'border-workbench-accent bg-workbench-accent/10 text-fg-editor'
                        : 'border-border-default bg-surface-0 hover:bg-surface-2 text-fg-muted'
                    }`}
                  >
                    <div className="flex items-center gap-2 font-semibold text-xs text-fg-editor">
                      <ProviderBrandIcon provider="ollama" size={16} />
                      <span>Ollama Service (Default)</span>
                    </div>
                    <span className="text-[10px] text-fg-subtle font-mono">http://localhost:11434</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => handleSelectPreset('custom')}
                    className={`px-3 py-2 rounded-lg border text-left transition flex items-center gap-1.5 cursor-pointer ${
                      localPreset === 'custom'
                        ? 'border-workbench-accent bg-workbench-accent/10 text-fg-editor'
                        : 'border-border-default bg-surface-0 hover:bg-surface-2 text-fg-muted'
                    }`}
                  >
                    <Server className="w-3.5 h-3.5" />
                    <span>Custom URL</span>
                  </button>
                </div>
              </div>

              {/* Endpoint URL + Probe */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <label className="font-medium text-fg-editor">Endpoint Base URL</label>
                  <span className="text-[10px] text-fg-subtle">
                    {localPreset === 'ollama' ? 'Standard Ollama REST endpoint' : 'OpenAI-compatible /v1 endpoint'}
                  </span>
                </div>
                <div className="flex gap-2">
                  <input
                    type="url"
                    required
                    value={localUrl}
                    disabled={localBusy}
                    onChange={(e) => {
                      setLocalUrl(e.target.value);
                      localGeneration.current++;
                      setLocalConnected(false);
                      setLocalModels([]);
                      setLocalModel('');
                      setLocalError('');
                    }}
                    placeholder={
                      localPreset === 'ollama' ? 'http://localhost:11434' : 'http://localhost:1234/v1'
                    }
                    className="flex-1 bg-surface-0 border border-border-default focus:border-border-emphasis rounded-lg px-3 py-2 text-fg-editor placeholder-fg-subtle focus:outline-none font-mono text-xs transition"
                  />
                  <button
                    type="button"
                    onClick={connectLocal}
                    disabled={localBusy || !localUrl.trim()}
                    className="px-3.5 py-2 rounded-lg bg-surface-2 hover:bg-surface-3 border border-border-default hover:border-border-emphasis text-fg-editor font-medium flex items-center gap-1.5 transition text-xs shrink-0 cursor-pointer disabled:opacity-50"
                  >
                    <RefreshCw className={`w-3.5 h-3.5 ${localBusy ? 'animate-spin' : ''}`} />
                    <span>{localBusy ? 'Scanning...' : 'Scan & Discover Models'}</span>
                  </button>
                </div>

                {/* Status Messages */}
                {localError && (
                  <div className="flex items-start gap-2 text-[11px] text-rose-400 bg-rose-500/10 border border-rose-500/20 p-2.5 rounded-lg">
                    <AlertCircle className="w-4 h-4 shrink-0 mt-0.5" />
                    <span className="break-words leading-relaxed">{localError}</span>
                  </div>
                )}
                {localConnected && localModels.length > 0 && (
                  <div className="flex items-center gap-2 text-[11px] text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 px-2.5 py-1.5 rounded-lg">
                    <CheckCircle2 className="w-3.5 h-3.5 shrink-0" />
                    <span>Connected successfully · {localModels.length} models discovered on {localUrl}</span>
                  </div>
                )}
              </div>

              {/* Optional API Key / Bearer Token */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <label className="font-medium text-fg-editor">
                    API Key / Bearer Token <span className="text-fg-subtle font-normal">(Optional)</span>
                  </label>
                  <span className="text-[10px] text-fg-subtle font-mono">For secured local servers</span>
                </div>
                <input
                  type="password"
                  value={localApiKey}
                  onChange={(e) => setLocalApiKey(e.target.value)}
                  placeholder="none (leave blank unless your local proxy requires authentication)"
                  className="w-full bg-surface-0 border border-border-default focus:border-border-emphasis rounded-lg px-3 py-2 text-fg-editor placeholder-fg-subtle focus:outline-none font-mono text-xs transition"
                />
              </div>

              {/* Account Label */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <label className="font-medium text-fg-editor">Account Display Name</label>
                  <span className="text-[10px] text-fg-subtle">How this server appears in Custos</span>
                </div>
                <input
                  type="text"
                  value={localAccountName}
                  onChange={(e) => setLocalAccountName(e.target.value)}
                  placeholder="e.g. Ollama Workstation, LM Studio Local"
                  className="w-full bg-surface-0 border border-border-default focus:border-border-emphasis rounded-lg px-3 py-2 text-fg-editor placeholder-fg-subtle focus:outline-none text-xs transition"
                />
              </div>

              {/* Discovered Model Selection */}
              {localModels.length > 0 && (
                <div className="space-y-1.5">
                  <div className="flex items-center justify-between">
                    <label className="font-medium text-fg-editor">Active Model</label>
                    <span className="text-[10px] text-emerald-400 font-mono">
                      {localModels.find((m) => m.id === localModel)?.context_window
                        ? `${Math.round((localModels.find((m) => m.id === localModel)?.context_window || 0) / 1000)}k context`
                        : 'Ready'}
                    </span>
                  </div>
                  <CustomSelect
                    value={localModel}
                    onChange={setLocalModel}
                    options={localModels.map((m) => ({
                      value: m.id,
                      label: m.name || m.id,
                      sublabel: m.context_window ? `${Math.round(m.context_window / 1000)}k tokens context window` : 'Context window unknown',
                      icon: <Cpu className="w-3.5 h-3.5 text-emerald-400" />,
                    }))}
                    headerTitle="Discovered Local Models"
                    headerBadge={`${localModels.length} Available`}
                  />
                </div>
              )}

                  {/* Switch to Native GGUF Bridge Card */}
                  <div className="rounded-xl border border-border-default/80 bg-surface-2/30 p-3.5 flex items-start gap-3">
                    <div className="p-2 rounded-lg bg-teal-500/10 text-teal-400 shrink-0 mt-0.5">
                      <Cpu className="w-4 h-4" />
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="text-xs font-semibold text-fg-editor">
                        Have GGUF model files on disk?
                      </div>
                      <p className="text-[11px] text-fg-muted mt-0.5 leading-relaxed">
                        You can load and execute GGUF files directly into Apple Silicon Metal GPU without running Ollama or external servers.
                      </p>
                      <button
                        type="button"
                        onClick={() => setLocalSubTab('native')}
                        className="mt-2 text-xs font-medium text-teal-400 hover:underline flex items-center gap-1 cursor-pointer"
                      >
                        <span>Switch to Native GGUF Runtime →</span>
                      </button>
                    </div>
                  </div>
                </div>

                {/* Footer */}
                <div className="border-t border-border-muted p-4 flex justify-end gap-2 bg-surface-1 rounded-b-2xl shrink-0">
                  <button
                    type="button"
                    onClick={closeAndClearSecret}
                    disabled={localBusy}
                    className="px-4 py-2 rounded-lg hover:bg-surface-2 text-fg-muted hover:text-fg-editor transition text-xs font-medium cursor-pointer"
                  >
                    Cancel
                  </button>
                  <button
                    type="submit"
                    disabled={localBusy || !localConnected || !localModels.some((m) => m.id === localModel)}
                    className="btn-primary rounded-lg px-4 py-2 text-xs font-medium transition cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed flex items-center gap-1.5"
                  >
                    {localBusy ? (
                      <>
                        <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                        <span>Connecting...</span>
                      </>
                    ) : (
                      <span>Save & Activate Provider</span>
                    )}
                  </button>
                </div>
              </form>
            )}
          </div>
        ) : authMode === 'api_key' ? (
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
                  <span className="text-[10px] text-fg-subtle">Custom Gateway (Optional)</span>
                </div>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={endpointUrl}
                    onChange={(e) => setEndpointUrl(e.target.value)}
                    placeholder="Default cloud endpoint"
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

              {/* Model Selection Dropdown */}
              {availableModels.length > 0 && (
                <div className="space-y-1.5">
                  <div className="flex items-center justify-between">
                    <label className="font-medium text-fg-editor">Default Model</label>
                    <span className="text-[10px] text-emerald-400 font-mono">
                      {rawProbed.length > 0 ? `${rawProbed.length} Live Probed` : 'Catalog Models'}
                    </span>
                  </div>
                  <CustomSelect
                    value={selectedModel}
                    onChange={setSelectedModel}
                    options={availableModels}
                    headerTitle="Select Model"
                    headerBadge={rawProbed.length > 0 ? 'Live Endpoint' : 'Curated Catalog'}
                  />
                </div>
              )}

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
                  <div className="font-semibold text-fg-editor">OAuth 2.0 Sovereign Authorization</div>
                  <div className="text-[11px] text-fg-muted leading-relaxed">
                    Authorize in your browser using PKCE, or GitHub Device Flow for Copilot. Custos stores credentials in the daemon and retrieves your account identity.
                  </div>
                </div>
              </div>

              {/* OAuth Providers */}
              <div className="space-y-1.5">
                <label className="font-medium text-fg-editor">Select OAuth Identity Provider</label>
                <div className="grid grid-cols-2 gap-2.5 pt-2">
                  {[
                    { id: 'antigravity', name: 'Antigravity', sub: 'Google OAuth / PKCE' },
                    { id: 'copilot', name: 'GitHub Copilot', sub: 'Device Authorization' },
                    { id: 'gemini', name: 'Google Cloud Gemini', sub: 'Google OAuth 2.0' },
                    { id: 'openai', name: 'OpenAI Platform', sub: 'OAuth Team / Enterprise' },
                    { id: 'anthropic', name: 'Anthropic Claude', sub: 'Console OAuth Login' },
                  ].map((p) => (
                    <button
                      key={p.id}
                      type="button"
                      onClick={() => handleSelectOauthProvider(p.id as ProviderServiceId)}
                      className={`p-3 rounded-xl text-left transition cursor-pointer flex items-center gap-2.5 ${
                        oauthProvider === p.id
                          ? 'bg-surface-2 text-fg-editor shadow-xs'
                          : 'bg-surface-0 text-fg-muted'
                      }`}
                    >
                      <div className="w-6 h-6 shrink-0">
                        <ProviderBrandIcon provider={p.id} size={24} />
                      </div>
                      <div className="min-w-0">
                        <div className="font-semibold text-xs text-fg-editor truncate">{p.name}</div>
                        <div className="text-[10px] text-fg-subtle truncate">{p.sub}</div>
                      </div>
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
