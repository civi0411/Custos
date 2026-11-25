import React from 'react';
import type {
  LoadedModelInstance,
  LocalHardwareProfile,
  LocalModelDescriptor,
  LocalModelLoadProfile,
  LocalModelPreflightResponse,
} from '@/types/domain';
import type { LMStudioModelItem } from './types';
import { GgufModelCard } from './GgufModelCard';
import { LMStudioModelCard } from './LMStudioModelCard';

interface LocalModelsListSectionProps {
  filterTab: 'all' | 'loaded' | 'gguf' | 'lmstudio';
  ggufModels: LocalModelDescriptor[];
  lmsModels: LMStudioModelItem[];
  instances: LoadedModelInstance[];
  hardware: LocalHardwareProfile | null;
  operatingModelPath: string | null;
  activeConfigPath: string | null;
  modelLoadErrors: Record<string, string>;
  verifyingPath: string | null;
  isModelActive: (modelId: string) => boolean;
  onSetActive: (modelId: string, modelName: string) => void;
  onToggleConfig: (filePath: string) => void;
  onDirectLoad: (model: LocalModelDescriptor) => void;
  onUnload: (filePath: string, modelName?: string) => void;
  onVerify: (filePath: string) => void;
  editingProfile: LocalModelLoadProfile | null;
  setEditingProfile: React.Dispatch<React.SetStateAction<LocalModelLoadProfile | null>>;
  profileEstimate: LocalModelPreflightResponse | null;
  estimatingProfile: boolean;
  onEstimateProfile: () => void;
  onSaveProfileOnly: () => void;
  onSaveAndLoadProfile: () => void;
  savingProfile: boolean;
}

export const LocalModelsListSection: React.FC<LocalModelsListSectionProps> = ({
  filterTab,
  ggufModels,
  lmsModels,
  instances,
  hardware,
  operatingModelPath,
  activeConfigPath,
  modelLoadErrors,
  verifyingPath,
  isModelActive,
  onSetActive,
  onToggleConfig,
  onDirectLoad,
  onUnload,
  onVerify,
  editingProfile,
  setEditingProfile,
  profileEstimate,
  estimatingProfile,
  onEstimateProfile,
  onSaveProfileOnly,
  onSaveAndLoadProfile,
  savingProfile,
}) => (
  <div className="space-y-2.5">
    {ggufModels
      .filter((m) => {
        if (filterTab === 'loaded') {
          return instances.some((inst) => inst.model_path === m.file_path);
        }
        return filterTab !== 'lmstudio';
      })
      .map((m) => {
        const instance = instances.find((inst) => inst.model_path === m.file_path);
        return (
          <GgufModelCard
            key={`gguf-card-${m.file_path}`}
            model={m}
            instance={instance}
            isAppleSilicon={Boolean(hardware?.is_apple_silicon)}
            isOperating={operatingModelPath === m.file_path}
            isConfigOpen={activeConfigPath === m.file_path}
            loadError={modelLoadErrors[m.file_path]}
            isCurrentActive={isModelActive(m.file_path)}
            verifyingPath={verifyingPath}
            onSetActive={onSetActive}
            onToggleConfig={onToggleConfig}
            onDirectLoad={onDirectLoad}
            onUnload={onUnload}
            onVerify={onVerify}
            editingProfile={editingProfile}
            setEditingProfile={setEditingProfile}
            profileEstimate={profileEstimate}
            estimatingProfile={estimatingProfile}
            onEstimateProfile={onEstimateProfile}
            onSaveProfileOnly={onSaveProfileOnly}
            onSaveAndLoadProfile={onSaveAndLoadProfile}
            savingProfile={savingProfile}
          />
        );
      })}

    {(filterTab === 'all' || filterTab === 'lmstudio') &&
      lmsModels.map((m) => (
        <LMStudioModelCard
          key={`lms-row-${m.id}`}
          model={m}
          isCurrentActive={isModelActive(m.id)}
          onSetActive={onSetActive}
        />
      ))}

    {ggufModels.length === 0 && lmsModels.length === 0 && (
      <div className="py-8 text-center text-xs text-[var(--color-fg-subtle)] font-mono">
        No local models detected. Launch LM Studio server or add a GGUF directory above.
      </div>
    )}
  </div>
);
