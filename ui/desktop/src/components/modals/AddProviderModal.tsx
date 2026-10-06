import React, { useState } from 'react';
import { Key, X } from 'lucide-react';
import { CustomSelect, CustomSelectOption } from './CustomSelect';

interface AddProviderModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSaveProvider: (provider: string, apiKey: string) => void;
}

const providerOptions: CustomSelectOption[] = [
  { 
    value: 'anthropic', 
    label: 'Anthropic (Claude 3.7 / 3.5)', 
    icon: <img src="/assets/provider-logo/anthropic.jpg" alt="Anthropic" className="w-4 h-4 rounded object-cover" /> 
  },
  { 
    value: 'openai', 
    label: 'OpenAI (GPT-4o, o3-mini)', 
    icon: <img src="/assets/provider-logo/openai.jpg" alt="OpenAI" className="w-4 h-4 rounded object-cover" /> 
  },
  { 
    value: 'google', 
    label: 'Google Cloud / Gemini', 
    icon: <img src="/assets/provider-logo/gemini.jpg" alt="Google" className="w-4 h-4 rounded object-cover" /> 
  },
  { 
    value: 'deepseek', 
    label: 'DeepSeek (Local / Cloud)', 
    icon: <img src="/assets/provider-logo/deepseek.jpg" alt="DeepSeek" className="w-4 h-4 rounded object-cover" /> 
  },
];

const priorityOptions: CustomSelectOption[] = [
  { value: 'primary', label: 'Primary Provider (Route 1)' },
  { value: 'fallback', label: 'Fallback on HTTP 429 (Route 2)' },
  { value: 'cache', label: 'Offline Local Cache (Route 3)' },
];

export const AddProviderModal: React.FC<AddProviderModalProps> = ({
  isOpen,
  onClose,
  onSaveProvider
}) => {
  const [providerService, setProviderService] = useState('anthropic');
  const [routingPriority, setRoutingPriority] = useState('primary');
  const [apiKey, setApiKey] = useState('');

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!apiKey.trim()) return;
    onSaveProvider(providerService, apiKey.trim());
    setApiKey('');
  };

  const selectedOption = providerOptions.find((opt) => opt.value === providerService);

  return (
    <div className="fixed inset-0 bg-canvas/80 backdrop-blur-sm z-50 flex items-center justify-center p-3 select-none">
      <div className="w-full max-w-lg bg-surface-card border border-surface-border rounded-2xl shadow-2xl flex flex-col max-h-[90vh]">
        <div className="h-12 border-b border-surface-border px-5 flex items-center justify-between shrink-0 bg-surface rounded-t-2xl">
          <div className="flex items-center gap-2">
            <Key className="w-4 h-4 text-brand-blue" />
            <span className="text-sm font-semibold text-white">Add Provider Key</span>
          </div>
          <button 
            onClick={onClose} 
            className="p-1 hover:bg-surface-elevated rounded-md text-neutral-400 hover:text-white transition cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="flex flex-col flex-1 overflow-visible">
          <div className="p-5 space-y-4 text-xs overflow-visible">
            <div className="space-y-1.5">
              <div className="flex items-center justify-between">
                <label className="font-medium text-neutral-300">Provider Service</label>
                {selectedOption?.icon && (
                  <div className="w-5 h-5 rounded-md overflow-hidden shrink-0 shadow-sm">
                    {selectedOption.icon}
                  </div>
                )}
              </div>
              <CustomSelect 
                value={providerService}
                onChange={setProviderService}
                options={providerOptions}
                headerTitle="Available Providers"
                headerBadge="4 Active"
              />
            </div>

            <div className="space-y-1.5">
              <label className="font-medium text-neutral-300">API Key Secret</label>
              <input 
                type="password" 
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder="sk-..." 
                className="w-full bg-surface border border-surface-border rounded-lg px-3 py-2 text-white placeholder-neutral-500 focus:outline-none focus:border-brand-blue font-mono"
                autoFocus
              />
            </div>

            <div className="space-y-1.5">
              <label className="font-medium text-neutral-300">Routing Priority</label>
              <CustomSelect
                value={routingPriority}
                onChange={setRoutingPriority}
                options={priorityOptions}
                headerTitle="Priority Tier"
              />
            </div>
          </div>

          <div className="h-12 border-t border-surface-border px-5 flex items-center justify-end gap-2 bg-surface shrink-0">
            <button 
              type="button"
              onClick={onClose} 
              className="px-3 py-1.5 rounded-lg hover:bg-surface-elevated text-neutral-400 transition text-xs"
            >
              Cancel
            </button>
            <button 
              type="submit" 
              className="px-3.5 py-1.5 rounded-lg bg-brand-blue hover:bg-blue-600 text-white font-medium transition text-xs"
            >
              Save & Validate
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
