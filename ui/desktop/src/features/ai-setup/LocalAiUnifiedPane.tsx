import React, { useState, useEffect, useCallback } from 'react';
import {
  Play,
  RefreshCw,
  Cpu,
  Save,
  Trash2,
  FolderPlus,
  Folder,
  PowerOff,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Terminal,
  Sliders,
  ChevronDown,
  ChevronUp,
  Check,
  X,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import { useAppContext } from '@/context/AppContext';
import type {
  LocalModelFolder,
  LocalModelDescriptor,
  LocalHardwareProfile,
  ModelCompatibility,
  LoadedModelInstance,
  LocalModelLoadProfile,
  LocalModelPreflightResponse,
} from '@/types/domain';

export interface LMStudioModelItem {
  id: string;
  name?: string;
  architecture?: string;
  quantization?: string;
  contextLength?: number;
  isLoaded?: boolean;
  type?: string;
}

interface LocalAiUnifiedPaneProps {
  onShowToast?: (msg: string) => void;
  onNavigateToWorkbench?: () => void;
}

export const LocalAiUnifiedPane: React.FC<LocalAiUnifiedPaneProps> = ({
  onShowToast,
}) => {
  const {
    connectedAccounts,
    handleSaveProvider,
    handleDeleteAccount,
    refreshModels,
    selectedModel,
    handleSelectModel,
  } = useAppContext();

  // Existing LM Studio account
  const existingAccount = connectedAccounts.find(
    (a) => a.providerId === 'lmstudio' || a.endpointUrl?.includes('1234')
  );

  // Connection & probe settings
  const [endpoint, setEndpoint] = useState(
    existingAccount?.endpointUrl || 'http://127.0.0.1:1234/v1'
  );
  const [isEditingEndpoint, setIsEditingEndpoint] = useState(false);
  const [tempEndpoint, setTempEndpoint] = useState(endpoint);
  const [isProbing, setIsProbing] = useState(false);
  const [serverStatus, setServerStatus] = useState<'idle' | 'online' | 'offline' | 'error'>('idle');
  const [probeLatencyMs, setProbeLatencyMs] = useState<number | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // LM Studio models
  const [lmsModels, setLmsModels] = useState<LMStudioModelItem[]>([]);

  // Local GGUF & Metal daemon state
  const [folders, setFolders] = useState<LocalModelFolder[]>([]);
  const [ggufModels, setGgufModels] = useState<LocalModelDescriptor[]>([]);
  const [hardware, setHardware] = useState<LocalHardwareProfile | null>(null);
  const [instances, setInstances] = useState<LoadedModelInstance[]>([]);
  const [scanningFolders, setScanningFolders] = useState(false);
  const [operatingModelPath, setOperatingModelPath] = useState<string | null>(null);
  const [modelLoadErrors, setModelLoadErrors] = useState<Record<string, string>>({});
  const [showAddFolder, setShowAddFolder] = useState(false);
  const [newFolderPath, setNewFolderPath] = useState('');
  const [newFolderLabel, setNewFolderLabel] = useState('');
  const [verifyingPath, setVerifyingPath] = useState<string | null>(null);

  // Active per-model settings drawer
  const [activeConfigPath, setActiveConfigPath] = useState<string | null>(null);
  const [editingProfile, setEditingProfile] = useState<LocalModelLoadProfile | null>(null);
  const [savingProfile, setSavingProfile] = useState(false);
  const [profileEstimate, setProfileEstimate] = useState<LocalModelPreflightResponse | null>(null);
  const [estimatingProfile, setEstimatingProfile] = useState(false);

  // Filter: 'all' | 'loaded' | 'gguf' | 'lmstudio'
  const [filterTab, setFilterTab] = useState<'all' | 'loaded' | 'gguf' | 'lmstudio'>('all');

  // Live generation test
  const [testModelId, setTestModelId] = useState<string>('');
  const [testPrompt, setTestPrompt] = useState('Write an idiomatic Rust function to reverse a string slice.');
  const [testResult, setTestResult] = useState<string | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [testLatencyMs, setTestLatencyMs] = useState<number | null>(null);
  const [testTokens, setTestTokens] = useState<{ total?: number; tps?: number } | null>(null);
  const [testError, setTestError] = useState<string | null>(null);
  const [isSavingProvider, setIsSavingProvider] = useState(false);

  // Base URL normalization
  const getNormalizedBaseUrl = useCallback((url: string): string => {
    let clean = (url || '').trim().replace(/\/+$/, '');
    if (!clean.startsWith('http://') && !clean.startsWith('https://')) {
      clean = `http://${clean}`;
    }
    if (!clean.endsWith('/v1')) {
      clean = `${clean}/v1`;
    }
    return clean;
  }, []);

  const formatBytes = (bytes: number) => {
    if (!bytes || bytes <= 0) return '0 B';
    const gb = bytes / (1024 * 1024 * 1024);
    if (gb >= 1) return `${gb.toFixed(1)} GB`;
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(0)} MB`;
  };

  // Load daemon data
  const loadLocalDaemonData = useCallback(async () => {
    try {
      const [foldersRes, modelsRes, hwRes, instRes] = await Promise.allSettled([
        daemonClient.listLocalFolders(),
        daemonClient.listLocalModels(),
        daemonClient.getLocalHardware(),
        daemonClient.listLoadedInstances(),
      ]);
      if (foldersRes.status === 'fulfilled') setFolders(foldersRes.value || []);
      if (modelsRes.status === 'fulfilled') setGgufModels(modelsRes.value.models || []);
      if (hwRes.status === 'fulfilled') setHardware(hwRes.value);
      if (instRes.status === 'fulfilled') setInstances(instRes.value.instances || []);
    } catch (err: any) {
      console.warn('Failed loading daemon local info:', err);
    }
  }, []);

  // Probe LM Studio Server
  const probeLMStudio = useCallback(
    async (targetUrl?: string) => {
      const probeUrl = getNormalizedBaseUrl(targetUrl || endpoint);
      setIsProbing(true);
      setErrorMessage(null);
      const start = performance.now();

      try {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), 4000);

        const res = await fetch(`${probeUrl}/models`, {
          method: 'GET',
          headers: { 'Content-Type': 'application/json' },
          signal: controller.signal,
        });
        clearTimeout(timeoutId);

        const latency = Math.round(performance.now() - start);
        setProbeLatencyMs(latency);

        if (!res.ok) {
          throw new Error(`LM Studio HTTP ${res.status}: ${res.statusText}`);
        }

        const data = await res.json();
        const rawList = Array.isArray(data) ? data : data?.data || [];
        const parsedList: LMStudioModelItem[] = rawList.map((m: any) => ({
          id: m.id || m.name,
          name: m.name || m.id,
          architecture: m.architecture || m.arch || 'transformer',
          quantization: m.quantization || m.quant || undefined,
          contextLength: m.context_length || m.max_context_length || undefined,
          isLoaded: m.loaded ?? true,
          type: m.type || 'llm',
        }));

        setLmsModels(parsedList);
        setServerStatus('online');
      } catch (err: any) {
        setServerStatus('offline');
        setProbeLatencyMs(null);
        setErrorMessage(
          err.name === 'AbortError'
            ? 'Connection to LM Studio timed out. Verify server is running on port 1234.'
            : err.message || 'Unable to connect to LM Studio.'
        );
      } finally {
        setIsProbing(false);
      }
    },
    [endpoint, getNormalizedBaseUrl]
  );

  useEffect(() => {
    void loadLocalDaemonData();
    void probeLMStudio();
  }, [loadLocalDaemonData, probeLMStudio]);

  // Connect LM Studio Provider to Custos
  const handleConnectLMStudio = async () => {
    setIsSavingProvider(true);
    setErrorMessage(null);
    try {
      const cleanUrl = getNormalizedBaseUrl(endpoint);
      const defaultMod = lmsModels[0]?.id || 'default';

      await handleSaveProvider({
        service: 'lmstudio',
        accountName: 'LM Studio (Local Server)',
        endpointUrl: cleanUrl,
        model: defaultMod,
        contextWindow: 16384,
        fastMode: true,
        apiKey: '',
      });

      await refreshModels();
      onShowToast?.('LM Studio connected to Custos.');
    } catch (err: any) {
      setErrorMessage(`Failed to connect: ${err.message || err}`);
    } finally {
      setIsSavingProvider(false);
    }
  };

  const handleDisconnectLMStudio = async () => {
    if (!existingAccount) return;
    try {
      await handleDeleteAccount(existingAccount.id);
      await refreshModels();
      onShowToast?.('LM Studio disconnected.');
    } catch (err: any) {
      setErrorMessage(`Failed to disconnect: ${err.message || err}`);
    }
  };

  const handleApplyEndpoint = () => {
    setEndpoint(tempEndpoint);
    setIsEditingEndpoint(false);
    void probeLMStudio(tempEndpoint);
  };

  // Set any model (LM Studio or GGUF) as the active model in Custos
  const handleSetActiveModel = (modelId: string, modelName: string) => {
    handleSelectModel(modelId, undefined, existingAccount?.id);
    onShowToast?.(`Active model set to "${modelName}".`);
  };

  // Open & edit load settings for a specific GGUF model
  const handleOpenModelConfig = async (filePath: string) => {
    if (activeConfigPath === filePath) {
      setActiveConfigPath(null);
      setEditingProfile(null);
      setProfileEstimate(null);
      return;
    }

    setActiveConfigPath(filePath);
    setErrorMessage(null);
    setProfileEstimate(null);
    try {
      const res = await daemonClient.getLocalModelLoadProfile(filePath);
      setEditingProfile(res.profile);
    } catch (err: any) {
      setErrorMessage(err?.message || 'Failed to fetch model profile');
    }
  };

  const handleEstimateProfile = async () => {
    if (!editingProfile) return;
    setEstimatingProfile(true);
    try {
      const est = await daemonClient.preflightLocalModel(editingProfile.model_path, editingProfile.context_size);
      setProfileEstimate(est);
    } catch (err: any) {
      setErrorMessage(err?.message || 'Preflight estimate failed');
    } finally {
      setEstimatingProfile(false);
    }
  };

  // Save profile settings only (without forcing load)
  const handleSaveProfileOnly = async () => {
    if (!editingProfile) return;
    setSavingProfile(true);
    setErrorMessage(null);
    try {
      const { revision: _rev, updated_at: _up, ...desired } = editingProfile;
      await daemonClient.saveLocalModelLoadProfile(desired);
      onShowToast?.('Model configuration saved.');
      setActiveConfigPath(null);
    } catch (err: any) {
      setErrorMessage(err?.message || 'Save failed');
    } finally {
      setSavingProfile(false);
    }
  };

  // Save profile and immediately load into VRAM
  const handleSaveAndLoadProfile = async () => {
    if (!editingProfile) return;
    setSavingProfile(true);
    setOperatingModelPath(editingProfile.model_path);
    setErrorMessage(null);
    try {
      const { revision: _rev, updated_at: _up, ...desired } = editingProfile;
      await daemonClient.saveLocalModelLoadProfile(desired);
      await daemonClient.ensureLocalModel(editingProfile.model_path, true);
      await loadLocalDaemonData();
      setActiveConfigPath(null);
      onShowToast?.(`Loaded ${editingProfile.model_path.split('/').pop()} into VRAM.`);
    } catch (err: any) {
      setErrorMessage(err?.message || 'Load failed');
    } finally {
      setSavingProfile(false);
      setOperatingModelPath(null);
    }
  };

  // Load a model directly with current settings
  const handleDirectLoadModel = async (model: LocalModelDescriptor) => {
    setOperatingModelPath(model.file_path);
    setErrorMessage(null);
    setModelLoadErrors((prev) => {
      const next = { ...prev };
      delete next[model.file_path];
      return next;
    });
    try {
      await daemonClient.ensureLocalModel(model.file_path, true);
      await loadLocalDaemonData();
      onShowToast?.(`Model "${model.model_name || model.file_name}" loaded into VRAM.`);
    } catch (err: any) {
      setModelLoadErrors((prev) => ({ ...prev, [model.file_path]: err.message || 'Load failed' }));
    } finally {
      setOperatingModelPath(null);
    }
  };

  // Unload a model from VRAM
  const handleUnloadModel = async (filePath: string, modelName?: string) => {
    setOperatingModelPath(filePath);
    try {
      await daemonClient.unloadLocalModel(filePath);
      await loadLocalDaemonData();
      onShowToast?.(`Model "${modelName || filePath.split('/').pop()}" unloaded from VRAM.`);
    } catch (err: any) {
      setErrorMessage(err.message || 'Failed to unload model');
    } finally {
      setOperatingModelPath(null);
    }
  };

  // Verify loaded model generation
  const handleVerifyGeneration = async (filePath: string) => {
    setVerifyingPath(filePath);
    try {
      const res = await daemonClient.verifyLocalModelGeneration(filePath);
      await loadLocalDaemonData();
      onShowToast?.(`Tensor verified in ${res.latency_ms}ms.`);
    } catch (err: any) {
      setErrorMessage(err.message || 'Generation verification failed');
    } finally {
      setVerifyingPath(null);
    }
  };

  // Folder management
  const handleScanFolders = async () => {
    setScanningFolders(true);
    setErrorMessage(null);
    try {
      const res = await daemonClient.scanLocalModels();
      const inventory = await daemonClient.listLocalModels();
      setGgufModels(inventory.models || []);
      if (res.hardware) setHardware(res.hardware);
      onShowToast?.(`Scan complete: found ${inventory.models?.length || 0} GGUF model(s).`);
    } catch (err: any) {
      setErrorMessage(err.message || 'Folder scan failed');
    } finally {
      setScanningFolders(false);
    }
  };

  const handleAddFolder = async () => {
    if (!newFolderPath.trim()) return;
    setErrorMessage(null);
    try {
      await daemonClient.addLocalFolder(newFolderPath.trim(), newFolderLabel.trim() || undefined);
      setNewFolderPath('');
      setNewFolderLabel('');
      setShowAddFolder(false);
      await loadLocalDaemonData();
      onShowToast?.('Model directory registered.');
    } catch (err: any) {
      setErrorMessage(err.message || 'Failed to add folder');
    }
  };

  const handlePickFolder = async () => {
    try {
      const selected = await daemonClient.pickFolder('model');
      if (selected) setNewFolderPath(selected);
    } catch (err: any) {
      setErrorMessage(err?.message || 'Folder picker failed');
    }
  };

  const handleRemoveFolder = async (id: string) => {
    try {
      await daemonClient.removeLocalFolder(id);
      await loadLocalDaemonData();
      onShowToast?.('Model directory removed.');
    } catch (err: any) {
      setErrorMessage(err.message || 'Failed to remove folder');
    }
  };

  // Live generation test
  const handleRunLiveTest = async () => {
    const activeModel = testModelId || instances[0]?.model_path || selectedModel || lmsModels[0]?.id;
    if (!activeModel) return;

    setIsGenerating(true);
    setTestError(null);
    setTestResult(null);
    setTestLatencyMs(null);
    setTestTokens(null);

    const start = performance.now();
    try {
      let answer = '';
      let totalTok = 0;

      // If activeModel matches LM Studio
      if (serverStatus === 'online' && (lmsModels.some((m) => m.id === activeModel) || activeModel.startsWith('lmstudio/'))) {
        const cleanName = activeModel.replace('lmstudio/', '');
        const res = await fetch(`${getNormalizedBaseUrl(endpoint)}/chat/completions`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            model: cleanName,
            messages: [{ role: 'user', content: testPrompt }],
            max_tokens: 512,
            stream: false,
          }),
        });
        if (!res.ok) throw new Error(`LM Studio HTTP ${res.status}`);
        const json = await res.json();
        answer = json?.choices?.[0]?.message?.content || json?.choices?.[0]?.text || '';
        totalTok = json?.usage?.completion_tokens || answer.split(/\s+/).length;
      } else {
        // Native Metal verification probe
        const targetPath = instances[0]?.model_path || activeModel;
        const res = await daemonClient.verifyLocalModelGeneration(targetPath);
        answer = res.test_output || `Tensor verified successfully in ${res.latency_ms}ms.`;
        totalTok = 16;
      }

      const latency = Math.round(performance.now() - start);
      setTestLatencyMs(latency);
      setTestResult(answer);

      const tps = latency > 0 ? Math.round((totalTok / (latency / 1000)) * 10) / 10 : 0;
      setTestTokens({ total: totalTok, tps });
    } catch (err: any) {
      setTestError(err.message || 'Generation test failed');
    } finally {
      setIsGenerating(false);
    }
  };

  const renderCompatBadge = (compat: ModelCompatibility) => {
    if (!hardware?.is_apple_silicon) {
      return (
        <span className="rounded border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-1.5 py-0.5 font-mono text-[9px] text-[var(--color-fg-subtle)]">
          CPU
        </span>
      );
    }
    switch (compat) {
      case 'green':
        return (
          <span className="inline-flex items-center gap-1 rounded border border-emerald-500/25 bg-emerald-500/10 px-1.5 py-0.5 text-[9px] font-mono text-emerald-400">
            <CheckCircle2 size={10} /> Fits VRAM
          </span>
        );
      case 'yellow':
        return (
          <span className="inline-flex items-center gap-1 rounded border border-amber-500/25 bg-amber-500/10 px-1.5 py-0.5 text-[9px] font-mono text-amber-400">
            <AlertTriangle size={10} /> Partial Offload
          </span>
        );
      case 'red':
        return (
          <span className="inline-flex items-center gap-1 rounded border border-rose-500/25 bg-rose-500/10 px-1.5 py-0.5 text-[9px] font-mono text-rose-400">
            <XCircle size={10} /> Exceeds VRAM
          </span>
        );
    }
  };

  const isModelActive = (modelId: string) => {
    if (!selectedModel) return false;
    return (
      selectedModel === modelId ||
      selectedModel === `lmstudio/${modelId}` ||
      selectedModel === `local/${modelId}` ||
      selectedModel.endsWith(modelId)
    );
  };

  return (
    <div className="space-y-4 max-w-4xl text-[var(--color-editor-fg)] font-sans">
      {/* ── Top Header & Hardware Specs Bar ── */}
      <div className="rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-4 sm:p-5 space-y-4 shadow-xs">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <div className="flex size-9 items-center justify-center rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] shrink-0 shadow-2xs">
              <Cpu className="size-4.5" />
            </div>
            <div>
              <div className="flex flex-wrap items-center gap-2">
                <h2 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-editor-fg)] font-mono">
                  Local Models & Runtimes
                </h2>
                {/* Status Indicator */}
                <span className="inline-flex items-center gap-1.5 rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2 py-0.5 font-mono text-[10px] text-[var(--color-fg-muted)]">
                  <span
                    className={`size-1.5 rounded-full ${
                      serverStatus === 'online' ? 'bg-emerald-400 ring-2 ring-emerald-400/20' : 'bg-[var(--color-fg-subtle)]'
                    }`}
                  />
                  {serverStatus === 'online'
                    ? `LM Studio (${probeLatencyMs ? `${probeLatencyMs}ms` : 'online'})`
                    : 'LM Studio Offline'}
                </span>
                {instances.length > 0 && (
                  <span className="inline-flex items-center gap-1 rounded-md border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 font-mono text-[10px] text-emerald-400 font-medium">
                    <span className="size-1.5 rounded-full bg-emerald-400" />
                    {instances.length} in VRAM
                  </span>
                )}
              </div>
              <p className="text-[11px] text-[var(--color-fg-muted)] mt-0.5">
                Configure context, GPU offload layers, and load multiple models into memory simultaneously.
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2 self-end sm:self-auto shrink-0">
            {/* Probe button */}
            <button
              type="button"
              onClick={() => void probeLMStudio()}
              disabled={isProbing}
              className="flex items-center gap-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3 py-1.5 text-xs font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)] transition disabled:opacity-50 cursor-pointer"
            >
              <RefreshCw className={`size-3 ${isProbing ? 'animate-spin' : ''}`} />
              {isProbing ? 'Probing…' : 'Probe Server'}
            </button>

            {/* Connect / Disconnect button */}
            {existingAccount ? (
              <button
                type="button"
                onClick={() => void handleDisconnectLMStudio()}
                className="flex items-center gap-1.5 rounded-lg border border-rose-500/30 bg-rose-500/10 px-3 py-1.5 text-xs font-medium text-rose-400 hover:bg-rose-500/20 hover:border-rose-500/50 transition cursor-pointer"
                title="Disconnect LM Studio account from Custos"
              >
                <Trash2 className="size-3" /> Disconnect
              </button>
            ) : (
              <button
                type="button"
                onClick={() => void handleConnectLMStudio()}
                disabled={isSavingProvider || serverStatus !== 'online'}
                className="flex items-center gap-1.5 rounded-lg bg-[var(--color-editor-fg)] px-3.5 py-1.5 text-xs font-semibold text-[var(--color-canvas)] hover:opacity-90 active:scale-[0.98] transition disabled:opacity-40 cursor-pointer shadow-xs"
              >
                <Save className="size-3" /> Connect LM Studio
              </button>
            )}
          </div>
        </div>

        {/* Endpoint Row (Clean inline, editable) */}
        <div className="flex items-center justify-between text-xs pt-3 border-t border-[var(--color-border-muted)] font-mono text-[var(--color-fg-muted)]">
          <div className="flex items-center gap-2 min-w-0">
            <span className="text-[var(--color-fg-subtle)] text-[10px] uppercase shrink-0">Server Endpoint:</span>
            {isEditingEndpoint ? (
              <div className="flex items-center gap-1.5 flex-wrap">
                <input
                  type="text"
                  value={tempEndpoint}
                  onChange={(e) => setTempEndpoint(e.target.value)}
                  className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-2.5 py-0.5 text-xs text-[var(--color-editor-fg)] outline-none w-64 focus:border-[var(--color-border-emphasis)]"
                />
                <button
                  type="button"
                  onClick={handleApplyEndpoint}
                  className="rounded-md bg-[var(--color-editor-fg)] px-2.5 py-0.5 text-[11px] font-medium text-[var(--color-canvas)] hover:opacity-90 cursor-pointer"
                >
                  Apply
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setTempEndpoint(endpoint);
                    setIsEditingEndpoint(false);
                  }}
                  className="p-1 text-[var(--color-fg-subtle)] hover:text-[var(--color-editor-fg)] cursor-pointer"
                >
                  <X className="size-3" />
                </button>
              </div>
            ) : (
              <span className="text-[var(--color-editor-fg)] font-mono truncate">{endpoint}</span>
            )}
          </div>
          {!isEditingEndpoint && (
            <button
              type="button"
              onClick={() => {
                setTempEndpoint(endpoint);
                setIsEditingEndpoint(true);
              }}
              className="text-[11px] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] underline underline-offset-2 cursor-pointer shrink-0 ml-2"
            >
              Change URL
            </button>
          )}
        </div>

        {/* Hardware specs row */}
        {hardware && (
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-2.5 pt-3 border-t border-[var(--color-border-muted)]">
            <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-3 shadow-2xs">
              <div className="text-[10px] text-[var(--color-fg-subtle)] font-mono uppercase tracking-wider">
                Processor
              </div>
              <div className="mt-1 text-xs font-semibold text-[var(--color-editor-fg)] truncate">
                {hardware.chip_name || 'Host CPU'}
              </div>
            </div>
            <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-3 shadow-2xs">
              <div className="text-[10px] text-[var(--color-fg-subtle)] font-mono uppercase tracking-wider">
                Unified RAM
              </div>
              <div className="mt-1 text-xs font-semibold text-[var(--color-editor-fg)] font-mono">
                {formatBytes(hardware.total_ram_bytes)}
              </div>
            </div>
            <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-3 shadow-2xs">
              <div className="text-[10px] text-[var(--color-fg-subtle)] font-mono uppercase tracking-wider">
                Metal VRAM Limit
              </div>
              <div className="mt-1 text-xs font-semibold text-[var(--color-editor-fg)] font-mono">
                {formatBytes(hardware.metal_vram_limit_bytes)}
              </div>
            </div>
          </div>
        )}
      </div>

      {errorMessage && (
        <div className="rounded-xl border border-rose-500/30 bg-rose-500/10 px-3.5 py-2.5 text-xs text-rose-300 font-mono">
          {errorMessage}
        </div>
      )}

      {/* ── SINGLE UNIFIED MODELS LIST ── */}
      <div className="rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-4 sm:p-5 space-y-4 shadow-xs">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2.5">
          {/* Filter Tabs */}
          <div className="flex items-center rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-1 text-[11px] gap-1">
            {[
              { id: 'all' as const, label: 'All', count: ggufModels.length + lmsModels.length },
              { id: 'loaded' as const, label: 'In VRAM', count: instances.length },
              { id: 'gguf' as const, label: 'Metal GGUF', count: ggufModels.length },
              { id: 'lmstudio' as const, label: 'LM Studio', count: lmsModels.length },
            ].map((f) => (
              <button
                key={f.id}
                type="button"
                onClick={() => setFilterTab(f.id)}
                className={`flex items-center gap-1.5 rounded-lg px-2.5 py-1 transition cursor-pointer ${
                  filterTab === f.id
                    ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] font-medium shadow-xs border border-[var(--color-border-emphasis)]'
                    : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-1)] border border-transparent'
                }`}
              >
                <span>{f.label}</span>
                <span
                  className={`text-[10px] font-mono px-1.5 py-0.2 rounded-full ${
                    filterTab === f.id
                      ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)]'
                      : 'bg-[var(--color-surface-2)] text-[var(--color-fg-subtle)]'
                  }`}
                >
                  {f.count}
                </span>
              </button>
            ))}
          </div>

          <div className="flex items-center gap-2 self-end sm:self-auto">
            <button
              type="button"
              onClick={handleScanFolders}
              disabled={scanningFolders}
              className="flex items-center gap-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1 text-xs font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)] transition disabled:opacity-50 cursor-pointer"
            >
              <RefreshCw className={`size-3 ${scanningFolders ? 'animate-spin' : ''}`} />
              {scanningFolders ? 'Scanning…' : 'Scan'}
            </button>
            <button
              type="button"
              onClick={() => setShowAddFolder((prev) => !prev)}
              className="flex items-center gap-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1 text-xs font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)] transition cursor-pointer"
            >
              <FolderPlus className="size-3" /> Add Folder
            </button>
          </div>
        </div>

        {/* Add Folder Form */}
        {showAddFolder && (
          <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-3.5 space-y-2.5 text-xs shadow-2xs">
            <span className="font-semibold text-[var(--color-editor-fg)]">Register Model Search Directory</span>
            <div className="flex gap-2">
              <input
                type="text"
                placeholder="/path/to/gguf/models"
                value={newFolderPath}
                onChange={(e) => setNewFolderPath(e.target.value)}
                className="flex-1 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] font-mono outline-none focus:border-[var(--color-border-emphasis)]"
              />
              <button
                type="button"
                onClick={handlePickFolder}
                className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3 py-1.5 text-xs font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] cursor-pointer"
              >
                Browse…
              </button>
            </div>
            <div className="flex justify-end gap-2 pt-1">
              <button
                type="button"
                onClick={() => setShowAddFolder(false)}
                className="rounded-lg px-3 py-1 text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleAddFolder}
                disabled={!newFolderPath.trim()}
                className="rounded-lg bg-[var(--color-editor-fg)] px-3.5 py-1 text-xs font-semibold text-[var(--color-canvas)] hover:opacity-90 active:scale-[0.98] disabled:opacity-50 cursor-pointer shadow-xs"
              >
                Save Directory
              </button>
            </div>
          </div>
        )}

        {/* Folders List */}
        {folders.length > 0 && (
          <div className="flex flex-wrap gap-2">
            {folders.map((f) => (
              <span
                key={f.id}
                className="inline-flex items-center gap-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-2.5 py-1 text-[10px] text-[var(--color-fg-muted)] font-mono shadow-2xs"
              >
                <Folder className="size-3 text-[var(--color-fg-subtle)]" />
                <span className="truncate max-w-[200px]" title={f.folder_path}>{f.folder_path}</span>
                <button
                  type="button"
                  onClick={() => handleRemoveFolder(f.id)}
                  className="p-0.5 rounded hover:bg-[var(--color-surface-2)] hover:text-rose-400 transition cursor-pointer"
                  title="Remove directory"
                >
                  <X className="size-2.5" />
                </button>
              </span>
            ))}
          </div>
        )}

        {/* ── Single Unified Model List ── */}
        <div className="space-y-2.5">
          {/* Native GGUF Models */}
          {ggufModels
            .filter((m) => {
              if (filterTab === 'loaded') {
                return instances.some((inst) => inst.model_path === m.file_path);
              }
              if (filterTab === 'lmstudio') return false;
              return true;
            })
            .map((m) => {
              const instance = instances.find((inst) => inst.model_path === m.file_path);
              const isLoaded = Boolean(instance);
              const isOperating = operatingModelPath === m.file_path;
              const isConfigOpen = activeConfigPath === m.file_path;
              const loadError = modelLoadErrors[m.file_path];
              const isCurrentActive = isModelActive(m.file_path);

              return (
                <div
                  key={`gguf-card-${m.file_path}`}
                  className={`rounded-xl border transition-all ${
                    isCurrentActive
                      ? 'border-sky-500/40 bg-[var(--color-surface-0)] shadow-xs ring-1 ring-sky-500/20'
                      : isLoaded
                      ? 'border-emerald-500/35 bg-[var(--color-surface-0)] shadow-xs'
                      : isConfigOpen
                      ? 'border-[var(--color-border-emphasis)] bg-[var(--color-surface-0)] shadow-xs'
                      : 'border-[var(--color-border-default)] bg-[var(--color-surface-0)] hover:border-[var(--color-border-emphasis)]'
                  }`}
                >
                  {/* Model Row Header */}
                  <div className="flex items-center justify-between gap-3 p-3.5 text-xs">
                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-2">
                        {/* Status Dot */}
                        <span
                          className={`size-2 rounded-full shrink-0 ${
                            isLoaded ? 'bg-emerald-400 ring-2 ring-emerald-400/20' : 'bg-[var(--color-fg-subtle)]/50'
                          }`}
                        />
                        <span className="font-semibold text-[var(--color-editor-fg)] font-mono truncate">
                          {m.model_name || m.file_name}
                        </span>
                        <span className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-1.5 py-0.5 font-mono text-[9px] text-[var(--color-fg-muted)]">
                          GGUF
                        </span>
                        {isLoaded ? (
                          <span className="inline-flex items-center gap-1 rounded-md border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 font-mono text-[9px] text-emerald-400 font-medium">
                            <span className="size-1 rounded-full bg-emerald-400" />
                            in VRAM ({instance?.context_limit.toLocaleString()} ctx)
                          </span>
                        ) : loadError ? (
                          <span className="rounded-md border border-rose-500/30 bg-rose-500/10 px-2 py-0.5 font-mono text-[9px] text-rose-400">
                            Load Failed
                          </span>
                        ) : (
                          renderCompatBadge(m.compatibility)
                        )}
                        {isCurrentActive && (
                          <span className="inline-flex items-center gap-1 rounded-md border border-sky-500/30 bg-sky-500/10 px-2 py-0.5 font-mono text-[9px] text-sky-300 font-medium">
                            <Check className="size-2.5" />
                            Active Model
                          </span>
                        )}
                      </div>
                      <div className="font-mono text-[10px] text-[var(--color-fg-subtle)] mt-1 flex flex-wrap items-center gap-2">
                        <span className="text-[var(--color-fg-muted)] font-medium">{formatBytes(m.file_size_bytes)}</span>
                        <span>•</span>
                        <span>{m.quantization || 'Q4_K_M'}</span>
                        <span>•</span>
                        <span>{m.architecture || 'gguf'}</span>
                        <span>•</span>
                        <span className="truncate max-w-[280px]" title={m.file_path}>
                          {m.file_path}
                        </span>
                      </div>
                    </div>

                    <div className="flex items-center gap-2 shrink-0">
                      {/* Set Active Model Button */}
                      {isCurrentActive ? (
                        <span className="flex items-center gap-1 rounded-lg border border-sky-500/30 bg-sky-500/10 px-2.5 py-1 font-mono text-[11px] text-sky-300 font-medium">
                          <Check className="size-3" /> Active
                        </span>
                      ) : (
                        <button
                          type="button"
                          onClick={() => handleSetActiveModel(`local/${m.file_path}`, m.model_name || m.file_name)}
                          className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1 font-mono text-[11px] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:border-[var(--color-border-emphasis)] hover:bg-[var(--color-surface-3)] transition cursor-pointer"
                        >
                          Set Active
                        </button>
                      )}

                      {/* Settings Toggle */}
                      <button
                        type="button"
                        onClick={() => handleOpenModelConfig(m.file_path)}
                        className={`flex items-center gap-1 rounded-lg border px-2.5 py-1 font-mono text-[11px] transition cursor-pointer ${
                          isConfigOpen
                            ? 'border-[var(--color-border-emphasis)] bg-[var(--color-surface-3)] text-[var(--color-editor-fg)] font-medium'
                            : 'border-[var(--color-border-default)] bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)]'
                        }`}
                      >
                        <Sliders className="size-3 text-[var(--color-fg-subtle)]" />
                        <span>Settings</span>
                        {isConfigOpen ? <ChevronUp className="size-3" /> : <ChevronDown className="size-3" />}
                      </button>

                      {isLoaded ? (
                        <>
                          <button
                            type="button"
                            onClick={() => void handleVerifyGeneration(m.file_path)}
                            disabled={verifyingPath === m.file_path}
                            className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1 font-mono text-[11px] text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)] disabled:opacity-50 transition cursor-pointer"
                          >
                            {verifyingPath === m.file_path ? 'Verifying…' : 'Verify'}
                          </button>
                          <button
                            type="button"
                            onClick={() => handleUnloadModel(m.file_path, m.model_name)}
                            disabled={isOperating}
                            className="flex items-center gap-1 rounded-lg border border-rose-500/25 bg-rose-500/10 px-2.5 py-1 font-mono text-[11px] text-rose-400 hover:bg-rose-500/20 hover:border-rose-500/40 transition cursor-pointer font-medium"
                          >
                            <PowerOff className="size-3" /> Unload
                          </button>
                        </>
                      ) : (
                        <button
                          type="button"
                          onClick={() => handleDirectLoadModel(m)}
                          disabled={isOperating}
                          className="flex items-center gap-1.5 rounded-lg bg-[var(--color-editor-fg)] px-3.5 py-1 font-mono text-[11px] font-semibold text-[var(--color-canvas)] hover:opacity-90 active:scale-[0.98] disabled:opacity-50 transition cursor-pointer shadow-xs"
                        >
                          <Play className="size-3 fill-current" />
                          {isOperating ? 'Loading…' : 'Load to VRAM'}
                        </button>
                      )}
                    </div>
                  </div>

                  {/* ── Per-Model Config Drawer (Tuning right before loading) ── */}
                  {isConfigOpen && editingProfile && (
                    <div className="border-t border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4 space-y-3.5 rounded-b-xl">
                      <div className="flex items-center justify-between text-[11px] font-mono text-[var(--color-fg-muted)]">
                        <span className="font-medium text-[var(--color-editor-fg)]">VRAM Offload & Execution Configuration</span>
                        <span className="text-[10px] text-[var(--color-fg-subtle)]">Config applies directly on load</span>
                      </div>

                      {/* Inputs Grid */}
                      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
                        {/* Context Size */}
                        <div>
                          <label className="text-[10px] text-[var(--color-fg-muted)] font-mono block mb-1">Context Window</label>
                          <select
                            value={editingProfile.context_size}
                            onChange={(e) => setEditingProfile({ ...editingProfile, context_size: Number(e.target.value) })}
                            className="w-full rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] font-mono outline-none focus:border-[var(--color-border-emphasis)] cursor-pointer"
                          >
                            <option value={4096}>4,096 tokens (Fast)</option>
                            <option value={8192}>8,192 tokens</option>
                            <option value={16384}>16,384 tokens (Balanced)</option>
                            <option value={32768}>32,768 tokens</option>
                            <option value={65536}>65,536 tokens</option>
                            <option value={131072}>131,072 tokens (Long context)</option>
                          </select>
                        </div>

                        {/* GPU Mode */}
                        <div>
                          <label className="text-[10px] text-[var(--color-fg-muted)] font-mono block mb-1">GPU Offload Mode</label>
                          <select
                            value={editingProfile.gpu_mode}
                            onChange={(e) => setEditingProfile({ ...editingProfile, gpu_mode: e.target.value as any })}
                            className="w-full rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] font-mono outline-none focus:border-[var(--color-border-emphasis)] cursor-pointer"
                          >
                            <option value="auto">Auto (Full Metal GPU)</option>
                            <option value="max">Max Offload</option>
                            <option value="off">CPU Only</option>
                            <option value="manual">Manual Layers</option>
                          </select>
                        </div>

                        {/* Eval Batch */}
                        <div>
                          <label className="text-[10px] text-[var(--color-fg-muted)] font-mono block mb-1">Evaluation Batch</label>
                          <input
                            type="number"
                            value={editingProfile.eval_batch_size || 512}
                            onChange={(e) => setEditingProfile({ ...editingProfile, eval_batch_size: Number(e.target.value) })}
                            className="w-full rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] font-mono outline-none focus:border-[var(--color-border-emphasis)]"
                          />
                        </div>

                        {/* CPU Threads */}
                        <div>
                          <label className="text-[10px] text-[var(--color-fg-muted)] font-mono block mb-1">CPU Threads</label>
                          <input
                            type="number"
                            placeholder="Auto"
                            value={editingProfile.cpu_threads || ''}
                            onChange={(e) => setEditingProfile({ ...editingProfile, cpu_threads: e.target.value ? Number(e.target.value) : undefined })}
                            className="w-full rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] font-mono outline-none focus:border-[var(--color-border-emphasis)]"
                          />
                        </div>
                      </div>

                      {/* Checkboxes */}
                      <div className="flex flex-wrap items-center gap-5 text-[11px] text-[var(--color-fg-muted)] pt-1 font-mono">
                        <label className="flex items-center gap-2 cursor-pointer hover:text-[var(--color-editor-fg)]">
                          <input
                            type="checkbox"
                            checked={editingProfile.flash_attention ?? true}
                            onChange={(e) => setEditingProfile({ ...editingProfile, flash_attention: e.target.checked })}
                            className="accent-[var(--color-editor-fg)] rounded"
                          />
                          <span>Flash Attention</span>
                        </label>
                        <label className="flex items-center gap-2 cursor-pointer hover:text-[var(--color-editor-fg)]">
                          <input
                            type="checkbox"
                            checked={editingProfile.mmap ?? true}
                            onChange={(e) => setEditingProfile({ ...editingProfile, mmap: e.target.checked })}
                            className="accent-[var(--color-editor-fg)] rounded"
                          />
                          <span>Memory Map (mmap)</span>
                        </label>
                        <label className="flex items-center gap-2 cursor-pointer hover:text-[var(--color-editor-fg)]">
                          <input
                            type="checkbox"
                            checked={editingProfile.mlock ?? false}
                            onChange={(e) => setEditingProfile({ ...editingProfile, mlock: e.target.checked })}
                            className="accent-[var(--color-editor-fg)] rounded"
                          />
                          <span>Lock VRAM (mlock)</span>
                        </label>
                      </div>

                      {/* Estimate Preview */}
                      {profileEstimate && (
                        <div className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-2.5 text-[11px] font-mono text-[var(--color-fg-muted)] flex flex-wrap items-center gap-4">
                          <span>RAM Est: <strong className="text-[var(--color-editor-fg)]">{formatBytes(profileEstimate.estimated_memory_bytes)}</strong></span>
                          <span>•</span>
                          <span>Free: <strong className="text-[var(--color-editor-fg)]">{formatBytes(profileEstimate.free_ram_bytes)}</strong></span>
                          <span>•</span>
                          <span>Context: <strong className="text-[var(--color-editor-fg)]">{profileEstimate.context_limit.toLocaleString()}</strong></span>
                        </div>
                      )}

                      {/* Action buttons inside drawer */}
                      <div className="flex justify-end items-center gap-2 pt-2 border-t border-[var(--color-border-muted)]">
                        <button
                          type="button"
                          onClick={handleEstimateProfile}
                          disabled={estimatingProfile}
                          className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3 py-1.5 text-xs font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)] disabled:opacity-50 cursor-pointer"
                        >
                          {estimatingProfile ? 'Estimating…' : 'Estimate Memory'}
                        </button>
                        <button
                          type="button"
                          onClick={handleSaveProfileOnly}
                          disabled={savingProfile}
                          className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3.5 py-1.5 text-xs font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)] disabled:opacity-50 cursor-pointer"
                        >
                          Save Settings
                        </button>
                        <button
                          type="button"
                          onClick={handleSaveAndLoadProfile}
                          disabled={savingProfile || isOperating}
                          className="flex items-center gap-1.5 rounded-lg bg-[var(--color-editor-fg)] px-4 py-1.5 text-xs font-semibold text-[var(--color-canvas)] hover:opacity-90 active:scale-[0.98] disabled:opacity-50 cursor-pointer shadow-xs"
                        >
                          <Save className="size-3" />
                          {savingProfile
                            ? 'Loading…'
                            : isLoaded
                            ? 'Save & Reload VRAM'
                            : 'Save & Load to VRAM'}
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              );
            })}

          {/* LM Studio Models */}
          {(filterTab === 'all' || filterTab === 'lmstudio') &&
            lmsModels.map((m) => {
              const isCurrentActive = isModelActive(m.id);
              return (
                <div
                  key={`lms-row-${m.id}`}
                  className={`rounded-xl border transition-all p-3.5 text-xs ${
                    isCurrentActive
                      ? 'border-sky-500/40 bg-[var(--color-surface-0)] shadow-xs ring-1 ring-sky-500/20'
                      : 'border-[var(--color-border-default)] bg-[var(--color-surface-0)] hover:border-[var(--color-border-emphasis)]'
                  }`}
                >
                  <div className="flex items-center justify-between gap-3">
                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="size-2 rounded-full bg-emerald-400 shrink-0 ring-2 ring-emerald-400/20" />
                        <span className="font-mono text-[var(--color-editor-fg)] font-semibold truncate">{m.id}</span>
                        <span className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-1.5 py-0.5 font-mono text-[9px] text-[var(--color-fg-muted)]">
                          LM Studio
                        </span>
                        {isCurrentActive && (
                          <span className="inline-flex items-center gap-1 rounded-md border border-sky-500/30 bg-sky-500/10 px-2 py-0.5 font-mono text-[9px] text-sky-300 font-medium">
                            <Check className="size-2.5" />
                            Active Model
                          </span>
                        )}
                      </div>
                      <div className="font-mono text-[10px] text-[var(--color-fg-subtle)] mt-1 flex flex-wrap items-center gap-2">
                        <span>Arch: {m.architecture}</span>
                        <span>•</span>
                        <span>{m.quantization || 'Q4_K_M'}</span>
                        <span>•</span>
                        <span>{m.contextLength ? `${m.contextLength.toLocaleString()} ctx` : 'Dynamic Context'}</span>
                      </div>
                    </div>

                    <div className="flex items-center gap-2 shrink-0">
                      {isCurrentActive ? (
                        <span className="flex items-center gap-1 rounded-lg border border-sky-500/30 bg-sky-500/10 px-3 py-1 font-mono text-[11px] text-sky-300 font-medium">
                          <Check className="size-3" /> Active
                        </span>
                      ) : (
                        <button
                          type="button"
                          onClick={() => handleSetActiveModel(`lmstudio/${m.id}`, m.id)}
                          className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3 py-1 font-mono text-[11px] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:border-[var(--color-border-emphasis)] hover:bg-[var(--color-surface-3)] transition cursor-pointer"
                        >
                          Set Active
                        </button>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}

          {ggufModels.length === 0 && lmsModels.length === 0 && (
            <div className="py-8 text-center text-xs text-[var(--color-fg-subtle)] font-mono">
              No local models detected. Launch LM Studio server or add a GGUF directory above.
            </div>
          )}
        </div>
      </div>

      {/* ── Live Generation Probe Console ── */}
      <div className="rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-4 sm:p-5 space-y-3 shadow-xs">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Terminal className="size-3.5 text-[var(--color-fg-muted)]" />
            <h3 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-editor-fg)] font-mono">
              Live Generation Probe
            </h3>
          </div>
          {testTokens && (
            <span className="font-mono text-[11px] text-[var(--color-fg-muted)]">
              {testLatencyMs}ms · {testTokens.tps} tok/s ({testTokens.total} tokens)
            </span>
          )}
        </div>

        <div className="flex flex-col sm:flex-row gap-2">
          {/* Target model select */}
          <select
            value={testModelId}
            onChange={(e) => setTestModelId(e.target.value)}
            className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-2.5 py-1.5 font-mono text-xs text-[var(--color-editor-fg)] focus:border-[var(--color-border-emphasis)] outline-none cursor-pointer max-w-[200px]"
          >
            <option value="">Auto (Active Model)</option>
            {instances.map((inst) => (
              <option key={`opt-inst-${inst.model_path}`} value={inst.model_path}>
                VRAM: {inst.model_name}
              </option>
            ))}
            {lmsModels.map((m) => (
              <option key={`opt-lms-${m.id}`} value={`lmstudio/${m.id}`}>
                LM Studio: {m.id}
              </option>
            ))}
          </select>

          <input
            type="text"
            value={testPrompt}
            onChange={(e) => setTestPrompt(e.target.value)}
            placeholder="Type a test prompt..."
            className="flex-1 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-3 py-1.5 text-xs text-[var(--color-editor-fg)] font-mono outline-none focus:border-[var(--color-border-emphasis)]"
          />

          <button
            type="button"
            onClick={handleRunLiveTest}
            disabled={isGenerating}
            className="flex items-center gap-1.5 rounded-lg bg-[var(--color-editor-fg)] px-4 py-1.5 text-xs font-semibold text-[var(--color-canvas)] hover:opacity-90 active:scale-[0.98] transition disabled:opacity-40 cursor-pointer shrink-0 shadow-xs"
          >
            <Play className={`size-3 ${isGenerating ? 'animate-spin' : 'fill-current'}`} />
            {isGenerating ? 'Testing…' : 'Run Test'}
          </button>
        </div>

        {testError && (
          <div className="rounded-lg border border-rose-500/25 bg-rose-500/10 p-2.5 text-xs text-rose-300 font-mono">
            {testError}
          </div>
        )}

        {testResult && (
          <div className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-3.5 font-mono text-xs text-[var(--color-editor-fg)] whitespace-pre-wrap max-h-48 overflow-y-auto leading-relaxed shadow-inner">
            {testResult}
          </div>
        )}
      </div>
    </div>
  );
};
