import React, { useState, useEffect } from 'react';
import {
  Key,
  RefreshCw,
  Cpu,
  CheckCircle2,
  AlertCircle,
  Plus,
  Trash2,
} from 'lucide-react';
import { ProviderBrandIcon } from '@/components/common/AgentIcons';
import { CustomSelect, type CustomSelectOption } from '@/components/common/CustomSelect';
import { daemonClient } from '@/api/daemon_client';
import type { ProbedModel, ModelCatalogOption, ProviderServiceId, ConnectedAccount } from '@/types';
import { useAppContext, defaultProbeBaseUrl } from '@/context/AppContext';
import {
  providerOptions,
  defaultEndpoints,
  fallbackDefaultModels,
} from './constants';

export interface ApiKeyProvidersSectionProps {
  apiKeyAccounts: ConnectedAccount[];
  onRefreshAccount: (id: string) => Promise<void>;
  onDisconnect: (id: string, name: string) => Promise<void>;
  refreshingId: string | null;
  onShowToast?: (msg: string) => void;
}

export const ApiKeyProvidersSection: React.FC<ApiKeyProvidersSectionProps> = ({
  apiKeyAccounts,
  onRefreshAccount,
  onDisconnect,
  refreshingId,
  onShowToast,
}) => {
  const { handleSaveProvider, refreshModels } = useAppContext();

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

  useEffect(() => {
    setProbeStatus({ type: 'idle' });

    if (defaultEndpoints[providerService]) {
      setEndpointUrl(defaultEndpoints[providerService]);
    } else if (['openai', 'anthropic', 'gemini', 'deepseek'].includes(providerService)) {
      setEndpointUrl('');
    }

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

  return (
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
                    onClick={() => onRefreshAccount(account.id)}
                    disabled={refreshingId === account.id}
                    className="flex items-center gap-1 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1 font-mono text-[11px] text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)] disabled:opacity-50 transition cursor-pointer"
                  >
                    <RefreshCw className={`size-3 ${refreshingId === account.id ? 'animate-spin' : ''}`} />
                    Refresh
                  </button>
                  <button
                    type="button"
                    onClick={() => onDisconnect(account.id, account.accountName || account.providerName)}
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
  );
};
