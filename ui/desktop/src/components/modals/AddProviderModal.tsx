import React, { useState, useEffect } from 'react';
import { Key, X, RefreshCw, Cpu, Server, Zap, CheckCircle2, AlertCircle } from 'lucide-react';
import { CustomSelect, CustomSelectOption } from '../CustomSelect';
import { daemonClient } from '@/api/daemon_client';
import { ProbedModel, ModelCatalogOption } from '@/types';

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
    icon: <img src="/assets/provider-logo/anthropic.jpg" alt="Anthropic" className="w-4 h-4 rounded object-cover" /> 
  },
  { 
    value: 'openai', 
    label: 'OpenAI / Codex', 
    sublabel: 'GPT-4o, o3-mini, o1',
    icon: <img src="/assets/provider-logo/openai.jpg" alt="OpenAI" className="w-4 h-4 rounded object-cover" /> 
  },
  { 
    value: 'gemini', 
    label: 'Google Cloud / Gemini', 
    sublabel: 'Gemini 2.5 Pro, Flash (1M tokens)',
    icon: <img src="/assets/provider-logo/gemini.jpg" alt="Google" className="w-4 h-4 rounded object-cover" /> 
  },
  { 
    value: 'deepseek', 
    label: 'DeepSeek', 
    sublabel: 'DeepSeek V3, R1 Reasoner',
    icon: <img src="/assets/provider-logo/deepseek.jpg" alt="DeepSeek" className="w-4 h-4 rounded object-cover" /> 
  },
  { 
    value: 'local', 
    label: 'Local Inference (Ollama / vLLM)', 
    sublabel: 'Ollama, LM Studio, llama.cpp',
    icon: <Server className="w-4 h-4 text-emerald-400" /> 
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
  onSaveProvider
}) => {
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

  // Load canonical catalog whenever provider changes
  useEffect(() => {
    if (!isOpen) return;

    // Reset probe status
    setProbeStatus({ type: 'idle' });

    // Set default endpoint for local inference
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

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!apiKey.trim() && providerService !== 'local') {
      alert('Vui lòng nhập API Key cho cloud provider.');
      return;
    }

    // Determine context window from selected model
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

  const selectedOption = providerOptions.find((opt) => opt.value === providerService);

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-3 select-none">
      <div className="w-full max-w-xl bg-surface-1 border border-border-default rounded-2xl shadow-2xl flex flex-col max-h-[92vh] overflow-hidden animate-in fade-in zoom-in-95 duration-100">
        <div className="h-12 border-b border-border-muted px-5 flex items-center justify-between shrink-0 bg-surface-1 rounded-t-2xl">
          <div className="flex items-center gap-2">
            <Key className="w-4 h-4 workbench-accent" />
            <span className="text-sm font-semibold text-fg-editor">Configure Model Provider</span>
          </div>
          <button 
            onClick={onClose} 
            className="p-1 hover:bg-surface-2 rounded-md text-fg-muted hover:text-fg-editor transition cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="flex flex-col flex-1 overflow-y-auto">
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

            {/* API Key Secret (Optional for local) */}
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

              {/* Probe Status Feedback */}
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

            {/* OrCa Feature: Fast Mode & Reasoning Effort */}
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
      </div>
    </div>
  );
};
