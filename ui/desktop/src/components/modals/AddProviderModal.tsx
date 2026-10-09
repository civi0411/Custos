import React, { useState, useEffect } from 'react';
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
  UserCheck
} from 'lucide-react';
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
  }) => void;
}

const providerOptions: CustomSelectOption[] = [
  {
    value: 'anthropic',
    label: 'Anthropic Claude',
    sublabel: 'Claude 3.7 Sonnet, 3.5 Haiku',
    icon: <img src="/assets/provider-logo/anthropic.jpg" alt="Anthropic" className="w-4 h-4 rounded object-cover" />,
  },
  {
    value: 'openai',
    label: 'OpenAI / Codex',
    sublabel: 'GPT-4o, o3-mini, o1',
    icon: <img src="/assets/provider-logo/openai.jpg" alt="OpenAI" className="w-4 h-4 rounded object-cover" />,
  },
  {
    value: 'gemini',
    label: 'Google Cloud / Gemini',
    sublabel: 'Gemini 2.5 Pro, Flash (1M tokens)',
    icon: <img src="/assets/provider-logo/gemini.jpg" alt="Google" className="w-4 h-4 rounded object-cover" />,
  },
  {
    value: 'deepseek',
    label: 'DeepSeek',
    sublabel: 'DeepSeek V3, R1 Reasoner',
    icon: <img src="/assets/provider-logo/deepseek.jpg" alt="DeepSeek" className="w-4 h-4 rounded object-cover" />,
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
  const [authMode, setAuthMode] = useState<'api_key' | 'oauth'>('api_key');

  // API Key Form State
  const [providerService, setProviderService] = useState('anthropic');
  const [routingPriority, setRoutingPriority] = useState('primary');
  const [apiKey, setApiKey] = useState('');
  const [endpointUrl, setEndpointUrl] = useState('');
  const [selectedModel, setSelectedModel] = useState('');
  const [availableModels, setAvailableModels] = useState<CustomSelectOption[]>([]);
  const [rawCatalog, setRawCatalog] = useState<ModelCatalogOption[]>([]);
  const [rawProbed, setRawProbed] = useState<ProbedModel[]>([]);
  const [isProbing, setIsProbing] = useState(false);
  const [probeStatus, setProbeStatus] = useState<{ type: 'idle' | 'success' | 'error'; message?: string }>({ type: 'idle' });
  const [fastMode, setFastMode] = useState(true);
  const [reasoningEffort, setReasoningEffort] = useState('medium');

  // OAuth Form State
  const [oauthProvider, setOauthProvider] = useState<ProviderServiceId>('copilot');
  const [oauthAccountName, setOauthAccountName] = useState('GitHub Copilot (Enterprise)');
  const [oauthEmail, setOauthEmail] = useState('developer@custos.local');
  const [oauthStep, setOauthStep] = useState<'idle' | 'authorizing' | 'success'>('idle');
  const [deviceCode] = useState('CUST-8492');

  // Reset & update OAuth defaults on provider switch
  const handleSelectOauthProvider = (prov: ProviderServiceId) => {
    setOauthProvider(prov);
    if (prov === 'copilot') {
      setOauthAccountName('GitHub Copilot Account');
      setOauthEmail('developer@custos.local');
    } else if (prov === 'gemini') {
      setOauthAccountName('Google AI Studio Account');
      setOauthEmail('developer@gmail.com');
    } else if (prov === 'openai') {
      setOauthAccountName('OpenAI Team Org');
      setOauthEmail('team@openai-org.local');
    } else {
      setOauthAccountName('Anthropic Console Account');
      setOauthEmail('claude-user@anthropic.local');
    }
    setOauthStep('idle');
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

  const handleSubmitApiKey = (e: React.FormEvent) => {
    e.preventDefault();
    if (!apiKey.trim() && providerService !== 'local') {
      alert('Vui lòng nhập API Key cho cloud provider.');
      return;
    }

    const probedMatch = rawProbed.find((m) => m.id === selectedModel);
    const catalogMatch = rawCatalog.find((m) => m.id === selectedModel);
    const contextWindow = probedMatch?.context_window || catalogMatch?.context_window || 128000;

    onSaveProvider({
      service: providerService,
      apiKey: apiKey.trim(),
      endpointUrl: endpointUrl.trim() || undefined,
      model: selectedModel || (providerService === 'anthropic' ? 'claude-3-7-sonnet' : 'gpt-4o'),
      contextWindow,
      fastMode,
    });

    setApiKey('');
    onClose();
  };

  const handleStartOAuthConnect = async () => {
    setOauthStep('authorizing');
    setTimeout(async () => {
      await handleConnectOAuth(oauthProvider, oauthAccountName, oauthEmail);
      setOauthStep('success');
      setTimeout(() => {
        onClose();
      }, 700);
    }, 1200);
  };

  const selectedOption = providerOptions.find((opt) => opt.value === providerService);

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
            onClick={onClose}
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
            <ShieldCheck className="w-3.5 h-3.5 text-indigo-400" />
            <span>OAuth 2.0 / SSO Connect</span>
          </button>
        </div>

        {/* TAB 1: API KEY */}
        {authMode === 'api_key' ? (
          <form onSubmit={handleSubmitApiKey} className="flex flex-col flex-1 overflow-y-auto">
            <div className="p-5 space-y-4 text-xs">
              {/* Provider Service Picker */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <label className="font-medium text-fg-editor">Provider Service</label>
                  {selectedOption?.icon && (
                    <div className="w-5 h-5 rounded-md overflow-hidden shrink-0 shadow-xs flex items-center justify-center">
                      {selectedOption.icon}
                    </div>
                  )}
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
                onClick={onClose}
                className="px-3.5 py-1.5 rounded-lg hover:bg-surface-2 text-fg-muted hover:text-fg-editor transition text-xs cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="submit"
                className="workbench-primary-action px-4 py-1.5 rounded-lg font-medium transition text-xs shadow-xs cursor-pointer"
              >
                Save & Activate Provider
              </button>
            </div>
          </form>
        ) : (
          /* TAB 2: OAUTH SSO */
          <div className="flex flex-col flex-1 overflow-y-auto">
            <div className="p-5 space-y-4 text-xs">
              <div className="p-3 bg-indigo-500/10 border border-indigo-500/20 rounded-xl flex items-start gap-3">
                <ShieldCheck className="w-5 h-5 text-indigo-400 shrink-0 mt-0.5" />
                <div className="space-y-0.5">
                  <div className="font-semibold text-fg-editor">OAuth 2.0 Sovereign Authorization</div>
                  <div className="text-[11px] text-fg-muted leading-relaxed">
                    Connect seamlessly via Device Code flow or Browser OAuth. Tokens are encrypted locally and automatically refreshed without exposing raw credentials.
                  </div>
                </div>
              </div>

              {/* OAuth Providers */}
              <div className="space-y-1.5">
                <label className="font-medium text-fg-editor">Select OAuth Identity Provider</label>
                <div className="grid grid-cols-2 gap-2.5">
                  {[
                    { id: 'copilot', name: 'GitHub Copilot', sub: 'Device Code / Enterprise SSO', icon: '/assets/provider-logo/openai.jpg' },
                    { id: 'gemini', name: 'Google Cloud Gemini', sub: 'Google OAuth 2.0', icon: '/assets/provider-logo/gemini.jpg' },
                    { id: 'openai', name: 'OpenAI Platform', sub: 'OAuth Team / Enterprise', icon: '/assets/provider-logo/openai.jpg' },
                    { id: 'anthropic', name: 'Anthropic Claude', sub: 'Console OAuth Login', icon: '/assets/provider-logo/anthropic.jpg' },
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
                        <img src={p.icon} alt={p.name} className="w-full h-full object-cover" />
                      </div>
                      <div className="min-w-0">
                        <div className="font-semibold text-xs text-fg-editor truncate">{p.name}</div>
                        <div className="text-[10px] text-fg-subtle truncate">{p.sub}</div>
                      </div>
                    </button>
                  ))}
                </div>
              </div>

              {/* Account Alias & Email */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div className="space-y-1">
                  <label className="font-medium text-fg-editor">Account Alias / Label</label>
                  <input
                    type="text"
                    value={oauthAccountName}
                    onChange={(e) => setOauthAccountName(e.target.value)}
                    placeholder="e.g. Work Copilot"
                    className="w-full bg-surface-0 border border-border-default focus:border-border-emphasis rounded-lg px-3 py-2 text-fg-editor focus:outline-none text-xs transition"
                  />
                </div>

                <div className="space-y-1">
                  <label className="font-medium text-fg-editor">Authorized Account / Email</label>
                  <input
                    type="text"
                    value={oauthEmail}
                    onChange={(e) => setOauthEmail(e.target.value)}
                    placeholder="e.g. dev@company.com"
                    className="w-full bg-surface-0 border border-border-default focus:border-border-emphasis rounded-lg px-3 py-2 text-fg-editor focus:outline-none text-xs transition"
                  />
                </div>
              </div>

              {/* Device Flow Simulation Box */}
              {oauthProvider === 'copilot' && (
                <div className="p-3.5 bg-surface-0 border border-border-default rounded-xl space-y-2 font-mono text-[11px]">
                  <div className="text-fg-subtle flex items-center justify-between">
                    <span>GitHub Device Code Flow</span>
                    <a
                      href="https://github.com/login/device"
                      target="_blank"
                      rel="noreferrer"
                      className="text-workbench-accent hover:underline flex items-center gap-1 text-[10px]"
                    >
                      <span>github.com/login/device</span>
                      <ExternalLink className="w-3 h-3" />
                    </a>
                  </div>
                  <div className="flex items-center justify-between p-2 bg-surface-2 rounded-lg border border-border-muted">
                    <span className="text-fg-editor font-bold text-sm tracking-wider">{deviceCode}</span>
                    <span className="text-[10px] text-emerald-400 bg-emerald-500/10 px-2 py-0.5 rounded border border-emerald-500/20">
                      Code Ready
                    </span>
                  </div>
                </div>
              )}
            </div>

            {/* Footer */}
            <div className="h-14 border-t border-border-muted px-5 flex items-center justify-end gap-2 bg-surface-1 shrink-0">
              <button
                type="button"
                onClick={onClose}
                className="px-3.5 py-1.5 rounded-lg hover:bg-surface-2 text-fg-muted hover:text-fg-editor transition text-xs cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleStartOAuthConnect}
                disabled={oauthStep === 'authorizing'}
                className="workbench-primary-action px-4 py-1.5 rounded-lg font-medium transition text-xs shadow-xs cursor-pointer flex items-center gap-1.5 disabled:opacity-50"
              >
                {oauthStep === 'authorizing' ? (
                  <>
                    <Loader2 className="w-3.5 h-3.5 animate-spin" />
                    <span>Connecting OAuth...</span>
                  </>
                ) : oauthStep === 'success' ? (
                  <>
                    <UserCheck className="w-3.5 h-3.5 text-emerald-400" />
                    <span>Connected!</span>
                  </>
                ) : (
                  <>
                    <ShieldCheck className="w-3.5 h-3.5" />
                    <span>Authorize & Connect Account</span>
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
