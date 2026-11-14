import React, { useState, useEffect, useCallback } from 'react';
import { daemonClient } from '@/api/daemon_client';
import { useAppContext } from '@/context/AppContext';
import type {
  LocalModelFolder,
  LocalModelDescriptor,
  LocalHardwareProfile,
  LoadedModelInstance,
  LocalModelLoadProfile,
  LocalModelPreflightResponse,
} from '@/types/domain';
import {
  type LocalAiUnifiedPaneProps,
  type LMStudioModelItem,
} from './types';
import { HardwareSpecsBar } from './HardwareSpecsBar';
import { LiveGenerationProbe } from './LiveGenerationProbe';
import { LocalModelsListSection } from './LocalModelsListSection';
import { ModelFoldersManager } from './ModelFoldersManager';

export const LocalAiUnifiedView: React.FC<LocalAiUnifiedPaneProps> = ({
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

  const handleSetActiveModel = (modelId: string, modelName: string) => {
    handleSelectModel(modelId, undefined, existingAccount?.id);
    onShowToast?.(`Active model set to "${modelName}".`);
  };

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
      {/* Top Header & Hardware Specs Bar */}
      <HardwareSpecsBar
        serverStatus={serverStatus}
        probeLatencyMs={probeLatencyMs}
        instancesCount={instances.length}
        isProbing={isProbing}
        onProbe={() => void probeLMStudio()}
        hasExistingAccount={Boolean(existingAccount)}
        onConnectLMStudio={() => void handleConnectLMStudio()}
        onDisconnectLMStudio={() => void handleDisconnectLMStudio()}
        isSavingProvider={isSavingProvider}
        endpoint={endpoint}
        tempEndpoint={tempEndpoint}
        setTempEndpoint={setTempEndpoint}
        isEditingEndpoint={isEditingEndpoint}
        setIsEditingEndpoint={setIsEditingEndpoint}
        onApplyEndpoint={handleApplyEndpoint}
        hardware={hardware}
      />

      {errorMessage && (
        <div className="rounded-xl border border-rose-500/30 bg-rose-500/10 px-3.5 py-2.5 text-xs text-rose-300 font-mono">
          {errorMessage}
        </div>
      )}

      {/* Single Unified Models List */}
      <div className="rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-4 sm:p-5 space-y-4 shadow-xs">
        <ModelFoldersManager
          filterTab={filterTab}
          setFilterTab={setFilterTab}
          counts={{
            all: ggufModels.length + lmsModels.length,
            loaded: instances.length,
            gguf: ggufModels.length,
            lmstudio: lmsModels.length,
          }}
          scanningFolders={scanningFolders}
          onScanFolders={handleScanFolders}
          showAddFolder={showAddFolder}
          setShowAddFolder={setShowAddFolder}
          newFolderPath={newFolderPath}
          setNewFolderPath={setNewFolderPath}
          onPickFolder={handlePickFolder}
          onAddFolder={handleAddFolder}
          folders={folders}
          onRemoveFolder={handleRemoveFolder}
        />

        {/* Model List */}
        <LocalModelsListSection
          filterTab={filterTab}
          ggufModels={ggufModels}
          lmsModels={lmsModels}
          instances={instances}
          hardware={hardware}
          operatingModelPath={operatingModelPath}
          activeConfigPath={activeConfigPath}
          modelLoadErrors={modelLoadErrors}
          verifyingPath={verifyingPath}
          isModelActive={isModelActive}
          onSetActive={handleSetActiveModel}
          onToggleConfig={handleOpenModelConfig}
          onDirectLoad={handleDirectLoadModel}
          onUnload={handleUnloadModel}
          onVerify={handleVerifyGeneration}
          editingProfile={editingProfile}
          setEditingProfile={setEditingProfile}
          profileEstimate={profileEstimate}
          estimatingProfile={estimatingProfile}
          onEstimateProfile={handleEstimateProfile}
          onSaveProfileOnly={handleSaveProfileOnly}
          onSaveAndLoadProfile={handleSaveAndLoadProfile}
          savingProfile={savingProfile}
        />
      </div>

      {/* Live Generation Probe Console */}
      <LiveGenerationProbe
        testModelId={testModelId}
        setTestModelId={setTestModelId}
        testPrompt={testPrompt}
        setTestPrompt={setTestPrompt}
        instances={instances}
        lmsModels={lmsModels}
        onRunTest={handleRunLiveTest}
        isGenerating={isGenerating}
        testError={testError}
        testResult={testResult}
        testLatencyMs={testLatencyMs}
        testTokens={testTokens}
      />
    </div>
  );
};
