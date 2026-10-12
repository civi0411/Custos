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
} from 'lucide-react';
import { ProviderBrandIcon } from '@/components/common/AgentIcons';
import { CustomSelect, CustomSelectOption } from '../CustomSelect';
import { daemonClient } from '@/api/daemon_client';
import { ProbedModel, ModelCatalogOption, ProviderServiceId } from '@/types';
import { useAppContext } from '@/context/AppContext';

interface AddProviderModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSaveProvider: (providerPayload: {
    service: string;
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
    value: 'local',
    label: 'Local Inference (Ollama / vLLM)',
    sublabel: 'Ollama, LM Studio, llama.cpp',
    icon: <Server className="w-4 h-4 text-emerald-400" />,
  },
];

const priorityOptions: CustomSelectOption[] = [
  { value: 'primary', label: 'Primary Provider (Route 1)', sublabel: 'Default target for agent queries' },
  { value: 'fallback', label: 'Fallback on HTTP 429 (Route 2)', sublabel: 'Automatic failover on rate limits' },
  { value: 'cache', label: 'Offline Local Cache (Route 3)', sublabel: 'Local model fallback on network cut' },
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

  // Auth Mode: API Key or OAuth
  const [authMode, setAuthMode] = useState<'api_key' | 'oauth' | 'local'>('api_key');

  // API Key Form State
  const [providerService, setProviderService] = useState('anthropic');
  const [routingPriority, setRoutingPriority] = useState('primary');
  const [apiKey, setApiKey] = useState('');
  const closeAndClearSecret = () => { setApiKey(''); setCallbackInput(''); setAuthUrlResult(null); onClose(); };
  useEffect(() => { if (!isOpen) setApiKey(''); }, [isOpen]);
  const [endpointUrl, setEndpointUrl] = useState('');
  const [selectedModel, setSelectedModel] = useState('');
  const [availableModels, setAvailableModels] = useState<CustomSelectOption[]>([]);
  const [rawCatalog, setRawCatalog] = useState<ModelCatalogOption[]>([]);
  const [rawProbed, setRawProbed] = useState<ProbedModel[]>([]);
  const [isProbing, setIsProbing] = useState(false);
  const [probeStatus, setProbeStatus] = useState<{ type: 'idle' | 'success' | 'error'; message?: string }>({ type: 'idle' });
  const [localUrl, setLocalUrl] = useState('http://localhost:11434');
  const [localModels, setLocalModels] = useState<ProbedModel[]>([]);
  const [localModel, setLocalModel] = useState('');
  const [localError, setLocalError] = useState('');
  const [localConnected, setLocalConnected] = useState(false);
  const [localBusy, setLocalBusy] = useState(false);
  const localGeneration = useRef(0);
  const connectLocal = async () => {
    const generation = ++localGeneration.current;
    setLocalBusy(true); setLocalError(''); setLocalConnected(false);
    try {
      const result = await daemonClient.probeModels(localUrl.trim(), 'ollama');
      if (generation !== localGeneration.current) return;
      const models = result?.models || [];
      setLocalModels(models);
      setLocalModel(models.some(m => m.id === localModel) ? localModel : models[0]?.id || '');
      setLocalConnected(true);
      if (!models.length) setLocalError('No installed models. Run ollama pull <model>, then refresh.');
      else if (localModel && !models.some(m => m.id === localModel)) setLocalError('The previously selected model was removed. Select an installed model.');
    } catch (err: any) {
      if (generation !== localGeneration.current) return;
      setLocalModels([]); setLocalModel('');
      setLocalError(`Check the URL and ensure Ollama is running: ${err?.message || err}`);
    } finally { if (generation === localGeneration.current) setLocalBusy(false); }
  };
  const saveLocal = async (event: React.FormEvent) => {
    event.preventDefault();
    const model = localModels.find(m => m.id === localModel);
    if (!localConnected || !model || localBusy) return;
    setLocalBusy(true); setLocalError('');
    try {
      await onSaveProvider({ service: 'local', apiKey: '', endpointUrl: localUrl.trim(), model: model.id, contextWindow: model.context_window || undefined });
      onClose();
    } catch (err: any) { setLocalError(`Save failed: ${err?.message || err}`); }
    finally { setLocalBusy(false); }
  };
  const [fastMode, setFastMode] = useState(true);
  const [reasoningEffort, setReasoningEffort] = useState('medium');

  useEffect(() => {
    if (!isOpen) { localGeneration.current++; setLocalConnected(false); setLocalModels([]); setLocalModel(''); setLocalError(''); setLocalBusy(false); }
  }, [isOpen]);

  // OAuth Form State
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

  const startPolling = useCallback((targetProvider: ProviderServiceId, accountName: string, email?: string) => {
    stopPolling();
    setCallbackStatus('listening');
    pollTimerRef.current = setInterval(async () => {
      try {
        const statusRes = await daemonClient.getOAuthStatus();
        if (statusRes && statusRes.status) {
          setCallbackStatus(statusRes.status);
          if (statusRes.status === 'completed' && statusRes.provider_id === targetProvider) {
            stopPolling();
            setExchangeSuccess(true);
            await handleConnectOAuth(targetProvider, accountName, email);
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

      } catch {
        // Silently continue polling
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
      const res = await daemonClient.getOAuthAuthorizeUrl({ provider_id: oauthProvider, service_type: oauthProvider });
      if (!res?.authorization_url) throw new Error('Daemon did not return an authorization URL');
      setAuthUrlResult(res);
      startPolling(oauthProvider, oauthAccountName, oauthEmail);
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
        provider_id: oauthProvider,
        service_type: oauthProvider,
        redirect_uri: authUrlResult.redirect_uri,
      });
      setExchangeSuccess(true);
      await handleConnectOAuth(oauthProvider, oauthAccountName, oauthEmail);
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
    const targetUrl = endpointUrl.trim() || (providerService === 'local' ? 'http://localhost:11434' : '');
    if (!targetUrl) {
      setProbeStatus({ type: 'error', message: 'Vui lòng nhập Endpoint URL để quét (vd: http://localhost:11434).' });
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
    if (!apiKey.trim() && providerService !== 'local') {
      alert('Vui lòng nhập API Key cho cloud provider.');
      return;
    }

    const probedMatch = rawProbed.find((m) => m.id === selectedModel);
    const catalogMatch = rawCatalog.find((m) => m.id === selectedModel);
    const contextWindow = probedMatch?.context_window || catalogMatch?.context_window || 128000;

    try {
    await onSaveProvider({
      service: providerService,
      apiKey: apiKey.trim(),
      endpointUrl: endpointUrl.trim() || undefined,
      model: selectedModel || undefined,
      contextWindow,
      fastMode,
    });

    setApiKey('');
    onClose();
    } catch (err: any) { setProbeStatus({ type: 'error', message: `Save failed: ${err?.message || err}` }); }
  };


  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-3 select-none">
      <div className="w-full max-w-xl bg-surface-1 border border-border-default rounded-2xl shadow-2xl flex flex-col max-h-[92vh] overflow-hidden animate-in fade-in zoom-in-95 duration-100">
        {/* Header */}
        <div className="h-12 border-b border-border-muted px-5 flex items-center justify-between shrink-0 bg-surface-1 rounded-t-2xl">
          <div className="flex items-center gap-2">
            <Key className="w-4 h-4 text-workbench-accent" />
            <span className="text-sm font-semibold text-fg-editor">Connect Provider or Account</span>
          </div>
          <button
            onClick={closeAndClearSecret}
            className="p-1 hover:bg-surface-2 rounded-md text-fg-muted hover:text-fg-editor transition cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Tab Switcher: API Key vs OAuth */}
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
            <span>API Key & Custom Endpoint</span>
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
            <span>OAuth 2.0 / SSO Connect</span>
          </button>
          <button type="button" onClick={() => setAuthMode('local')} className={`pb-2.5 px-3 font-medium border-b-2 flex items-center gap-1.5 ${authMode === 'local' ? 'border-workbench-accent text-fg-editor' : 'border-transparent text-fg-muted'}`}><Server className="w-3.5 h-3.5" />Local</button>
        </div>

        {/* TAB 1: API KEY */}
        {authMode === 'local' ? (
          <form onSubmit={saveLocal} className="flex flex-col min-h-0">
            <div className="p-5 space-y-4 text-xs overflow-y-auto">
              <p className="text-fg-editor font-semibold flex gap-2"><ProviderBrandIcon provider="ollama" size={16} />Ollama local models</p>
              <label className="block space-y-2 text-fg-editor"><span>Endpoint Base URL</span>
                <input type="url" required value={localUrl} disabled={localBusy} onChange={e => { setLocalUrl(e.target.value); localGeneration.current++; setLocalConnected(false); setLocalModels([]); setLocalModel(''); setLocalError(''); }} className="w-full bg-surface-0 border border-border-default rounded-lg px-3 py-2 text-fg-editor" />
              </label>
              <button type="button" onClick={connectLocal} disabled={localBusy || !localUrl.trim()} className="provider-oauth-action rounded-lg px-3 py-2 flex gap-2 items-center disabled:opacity-50"><RefreshCw className={`w-4 h-4 ${localBusy ? 'animate-spin' : ''}`} />{localBusy ? 'Connecting...' : 'Connect / Refresh models'}</button>
              {localError && <p role="alert" className="text-red-400 break-words">{localError}</p>}
              {localConnected && localModels.length > 0 && <p role="status" className="text-emerald-400">Connected · {localModels.length} installed models</p>}
              <label className="block text-fg-editor">Model</label>
              <CustomSelect value={localModel} onChange={setLocalModel} options={localModels.map(m => ({ value: m.id, label: m.name || m.id, sublabel: m.context_window ? `${m.context_window} tokens` : 'Context unknown' }))} headerTitle="Installed Ollama models" />
            </div>
            <div className="border-t border-border-muted p-4 flex justify-end gap-2">
              <button type="button" onClick={closeAndClearSecret} disabled={localBusy} className="text-fg-editor px-3 py-2">Cancel</button>
              <button type="submit" disabled={localBusy || !localConnected || !localModels.some(m => m.id === localModel)} className="provider-oauth-action rounded-lg px-3 py-2 disabled:opacity-50">{localBusy ? 'Please wait...' : 'Save & Activate Provider'}</button>
            </div>
          </form>
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
                  <label className="font-medium text-fg-editor">
                    {providerService === 'local' ? 'API Key / Bearer Token (Optional)' : 'API Key Secret'}
                  </label>
                  <span className="text-[10px] text-fg-subtle font-mono">Fail-closed & Masked</span>
                </div>
                <input
                  type="password"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  placeholder={providerService === 'local' ? 'none (or token if reverse proxy auth)' : 'sk-ant-... / sk-proj-...'}
                  className="w-full bg-surface-0 border border-border-default focus:border-border-emphasis rounded-lg px-3 py-2 text-fg-editor placeholder-fg-subtle focus:outline-none font-mono text-xs transition"
                  autoFocus={providerService !== 'local'}
                />
              </div>

              {/* Custom / Local Endpoint URL */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <label className="font-medium text-fg-editor">Endpoint Base URL</label>
                  <span className="text-[10px] text-fg-subtle">
                    {providerService === 'local' ? 'Ollama / vLLM / LM Studio' : 'Custom Gateway (Optional)'}
                  </span>
                </div>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={endpointUrl}
                    onChange={(e) => setEndpointUrl(e.target.value)}
                    placeholder={providerService === 'local' ? 'http://localhost:11434' : 'Default cloud endpoint'}
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

              {/* Routing Priority */}
              <div className="space-y-1.5">
                <label className="font-medium text-fg-editor">OmniRoute Priority Tier</label>
                <CustomSelect
                  value={routingPriority}
                  onChange={setRoutingPriority}
                  options={priorityOptions}
                  headerTitle="Priority Tier"
                />
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
                className="provider-oauth-action px-4 py-1.5 rounded-lg font-medium transition text-xs shadow-xs cursor-pointer"
              >
                Save & Activate Provider
              </button>
            </div>
          </form>
        ) : (
          /* TAB 2: OAUTH SSO */
          <div className="flex flex-col flex-1 min-h-0 overflow-hidden">
            <div className="p-5 space-y-4 text-xs overflow-y-auto min-h-0 flex-1">
              <div className="p-3 bg-indigo-500/10 border border-indigo-500/20 rounded-xl flex items-start gap-3">
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
                <div className="grid grid-cols-2 gap-2.5">
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
                      className={`p-3 rounded-xl border text-left transition cursor-pointer flex items-center gap-2.5 ${
                        oauthProvider === p.id
                          ? 'bg-surface-2 border-workbench-accent text-fg-editor shadow-xs'
                          : 'bg-surface-0 border-border-default text-fg-muted hover:border-border-emphasis'
                      }`}
                    >
                      <div className="w-6 h-6 rounded-md overflow-hidden shrink-0 border border-border-muted">
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

              <p className="text-fg-muted text-xs">Your account identity is retrieved after authorization. No name or email input is required.</p>
              {/* OpenAI PKCE Interactive Box */}
              {(
                <div className="p-3.5 bg-surface-0 border border-border-default rounded-xl space-y-3 font-mono text-xs">
                  <div className="flex items-center justify-between text-fg-subtle text-[11px]">
                    <span className="font-semibold text-fg-editor">{oauthProvider === 'copilot' ? 'GitHub Device Authorization' : `${oauthProviderLabel} PKCE Flow (RFC 7636)`}</span>
                    <span className="text-[10px] text-emerald-400 bg-emerald-500/10 px-2 py-0.5 rounded border border-emerald-500/20">
                      {oauthProvider === 'copilot' ? 'Device Flow' : 'PKCE S256'}
                    </span>
                  </div>

                  {!authUrlResult ? (
                    <div className="flex flex-col items-center justify-center p-4 bg-surface-2/60 rounded-xl border border-dashed border-border-default space-y-3">
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
            <div className="h-14 border-t border-border-muted px-5 flex items-center justify-end gap-2 bg-surface-1 shrink-0">
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
                className="provider-oauth-action px-4 py-1.5 rounded-lg font-medium transition text-xs shadow-xs cursor-pointer flex items-center gap-1.5 disabled:opacity-50"
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
                    <ShieldCheck className="w-3.5 h-3.5" />
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
