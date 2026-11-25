import React from 'react';
import { Ban, Cpu, Settings } from 'lucide-react';
import { ProviderIcon } from './ProviderIcon';

const PROVIDER_FILTER_LABELS: Record<string, string> = {
  antigravity: 'Antigravity',
  copilot: 'GitHub Copilot',
  openai: 'OpenAI',
  anthropic: 'Anthropic',
  gemini: 'Gemini',
  local: 'Local',
  deepseek: 'DeepSeek',
};

export const ModelSelectorProviderPills: React.FC<{
  availableProviders: string[];
  selectedProviderFilter: string;
  onSelectFilter: (provider: string) => void;
}> = ({ availableProviders, selectedProviderFilter, onSelectFilter }) => {
  if (availableProviders.length <= 1) return null;

  return (
    <div className="flex items-center gap-1 mb-2 overflow-x-auto no-scrollbar pb-0.5">
      <button
        type="button"
        onClick={() => onSelectFilter('all')}
        className={`px-2 py-0.5 rounded text-[10px] font-medium transition shrink-0 ${
          selectedProviderFilter === 'all'
            ? 'bg-surface-3 text-fg-editor border border-border-default'
            : 'text-fg-subtle hover:text-fg-editor hover:bg-surface-2'
        }`}
      >
        All
      </button>
      {availableProviders.map((prov) => (
        <button
          key={prov}
          type="button"
          onClick={() => onSelectFilter(prov)}
          className={`flex items-center gap-1.5 px-2 py-0.5 rounded text-[10px] font-medium transition shrink-0 ${
            selectedProviderFilter === prov
              ? 'bg-surface-3 text-fg-editor border border-border-default'
              : 'text-fg-subtle hover:text-fg-editor hover:bg-surface-2'
          }`}
        >
          <ProviderIcon provider={prov} size={11} />
          <span>{PROVIDER_FILTER_LABELS[prov] ?? prov}</span>
        </button>
      ))}
    </div>
  );
};

export const ModelSelectorDropdownFooter: React.FC<{
  onUnsetModel: () => void;
  onOpenLocalModels: () => void;
  onManageProviders: () => void;
}> = ({ onUnsetModel, onOpenLocalModels, onManageProviders }) => (
  <div className="shrink-0 rounded-b-[inherit] p-2 border-t border-border-muted bg-surface-0/60 flex items-center justify-between text-[10px]">
    <button
      type="button"
      onClick={onUnsetModel}
      className="text-zinc-400 hover:text-amber-400 flex items-center gap-1 px-1.5 py-0.5 rounded hover:bg-surface-2 transition"
    >
      <Ban size={11} />
      Unset Model
    </button>

    <button
      type="button"
      onClick={onOpenLocalModels}
      className="text-teal-400 hover:text-teal-300 flex items-center gap-1 px-2 py-0.5 rounded hover:bg-surface-2 transition text-xs font-medium"
    >
      <Cpu size={11} />
      Local Models
    </button>

    <button
      type="button"
      onClick={onManageProviders}
      className="text-fg-muted hover:text-fg-editor flex items-center gap-1 px-1.5 py-0.5 rounded hover:bg-surface-2 transition"
    >
      <Settings size={11} />
      Manage Providers...
    </button>
  </div>
);

