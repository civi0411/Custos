import React, { useState } from 'react';
import { Key, X } from 'lucide-react';

interface AddProviderModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSaveProvider: (provider: string, apiKey: string) => void;
}

const getProviderLogo = (service: string): string | null => {
  switch (service) {
    case 'anthropic':
      return '/assets/provider-logo/anthropic.jpg';
    case 'openai':
      return '/assets/provider-logo/openai.jpg';
    case 'google':
    case 'gemini':
      return '/assets/provider-logo/gemini.jpg';
    case 'deepseek':
      return '/assets/provider-logo/deepseek.jpg';
    default:
      return null;
  }
};

export const AddProviderModal: React.FC<AddProviderModalProps> = ({
  isOpen,
  onClose,
  onSaveProvider
}) => {
  const [providerService, setProviderService] = useState('anthropic');
  const [apiKey, setApiKey] = useState('');

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!apiKey.trim()) return;
    onSaveProvider(providerService, apiKey.trim());
    setApiKey('');
  };

  const currentLogo = getProviderLogo(providerService);

  return (
    <div className="fixed inset-0 bg-canvas/80 backdrop-blur-sm z-50 flex items-center justify-center p-3 select-none">
      <div className="w-full max-w-lg bg-surface-card border border-surface-border rounded-2xl shadow-2xl flex flex-col overflow-hidden max-h-[90vh]">
        <div className="h-12 border-b border-surface-border px-5 flex items-center justify-between shrink-0 bg-surface">
          <div className="flex items-center gap-2">
            <Key className="w-4 h-4 text-brand-blue" />
            <span className="text-sm font-semibold text-white">Add Provider Key</span>
          </div>
          <button 
            onClick={onClose} 
            className="p-1 hover:bg-surface-elevated rounded-md text-neutral-400 hover:text-white transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="p-5 space-y-4 text-xs overflow-y-auto">
            <div className="space-y-1.5">
              <div className="flex items-center justify-between">
                <label className="font-medium text-neutral-300">Provider Service</label>
                {currentLogo && (
                  <div className="w-5 h-5 rounded-md overflow-hidden shrink-0 shadow-sm">
                    <img src={currentLogo} alt={providerService} className="w-full h-full object-cover" />
                  </div>
                )}
              </div>
              <select 
                value={providerService}
                onChange={(e) => setProviderService(e.target.value)}
                className="w-full bg-surface border border-surface-border rounded-lg px-3 py-2 text-white focus:outline-none focus:border-brand-blue"
              >
                <option value="anthropic">Anthropic (Claude 3.7 / 3.5)</option>
                <option value="openai">OpenAI (GPT-4o, o3-mini)</option>
                <option value="google">Google Cloud / Gemini</option>
                <option value="deepseek">DeepSeek (Local / Cloud)</option>
              </select>
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
              <select className="w-full bg-surface border border-surface-border rounded-lg px-3 py-2 text-white focus:outline-none focus:border-brand-blue">
                <option>Primary Provider (Route 1)</option>
                <option>Fallback on HTTP 429 (Route 2)</option>
                <option>Offline Local Cache (Route 3)</option>
              </select>
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
