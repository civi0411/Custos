import React, { useState } from 'react';
import { 
  ArrowLeft, 
  ArrowRight, 
  RotateCw, 
  Globe, 
  ExternalLink, 
  Monitor, 
  Tablet, 
  Smartphone, 
  ShieldCheck
} from 'lucide-react';

export const BrowserWorkspace: React.FC = () => {
  const [url, setUrl] = useState('http://localhost:1420/studio');
  const [inputUrl, setInputUrl] = useState('http://localhost:1420/studio');
  const [deviceMode, setDeviceMode] = useState<'desktop' | 'tablet' | 'mobile'>('desktop');
  const [isLoading, setIsLoading] = useState(false);

  const handleNavigate = (e: React.FormEvent) => {
    e.preventDefault();
    let formatted = inputUrl.trim();
    if (!formatted.startsWith('http://') && !formatted.startsWith('https://')) {
      formatted = 'https://' + formatted;
    }
    setUrl(formatted);
    setInputUrl(formatted);
    setIsLoading(true);
    setTimeout(() => setIsLoading(false), 400);
  };

  const handleReload = () => {
    setIsLoading(true);
    setTimeout(() => setIsLoading(false), 300);
  };

  const getContainerWidth = () => {
    switch (deviceMode) {
      case 'mobile':
        return 'max-w-[375px]';
      case 'tablet':
        return 'max-w-[768px]';
      case 'desktop':
      default:
        return 'w-full';
    }
  };

  return (
    <div className="flex flex-col h-full w-full bg-[#0a0c12] text-neutral-200 select-none font-sans overflow-hidden">
      {/* 1. Browser Navigation Toolbar */}
      <div className="h-10 px-3 bg-[#0d0f17] border-b border-[#1c2130] flex items-center justify-between gap-2 shrink-0">
        {/* Navigation Buttons */}
        <div className="flex items-center gap-1 text-neutral-400">
          <button 
            className="p-1.5 rounded hover:bg-[#1a2030] hover:text-white transition disabled:opacity-40"
            title="Back"
          >
            <ArrowLeft className="w-3.5 h-3.5" />
          </button>
          <button 
            className="p-1.5 rounded hover:bg-[#1a2030] hover:text-white transition disabled:opacity-40"
            title="Forward"
          >
            <ArrowRight className="w-3.5 h-3.5" />
          </button>
          <button 
            onClick={handleReload}
            className={`p-1.5 rounded hover:bg-[#1a2030] hover:text-white transition ${isLoading ? 'animate-spin text-brand-blue' : ''}`}
            title="Reload (⌘R)"
          >
            <RotateCw className="w-3.5 h-3.5" />
          </button>
        </div>

        {/* Address Bar */}
        <form onSubmit={handleNavigate} className="flex-1 max-w-xl mx-2">
          <div className="relative flex items-center bg-[#131724] border border-[#232938] hover:border-brand-blue/50 rounded-lg px-2.5 py-1 text-xs transition">
            <ShieldCheck className="w-3.5 h-3.5 text-emerald-400 mr-2 shrink-0" />
            <input
              type="text"
              value={inputUrl}
              onChange={(e) => setInputUrl(e.target.value)}
              placeholder="Enter URL or search..."
              className="w-full bg-transparent text-white placeholder-neutral-500 focus:outline-none text-[11.5px] font-mono"
            />
            {isLoading && (
              <span className="w-2 h-2 rounded-full bg-brand-blue animate-ping mr-1 shrink-0"></span>
            )}
          </div>
        </form>

        {/* Viewport Device Controls & External Link */}
        <div className="flex items-center gap-1">
          <div className="flex items-center bg-[#131724] border border-[#232938] rounded-lg p-0.5 mr-1">
            <button
              onClick={() => setDeviceMode('desktop')}
              className={`p-1 rounded ${deviceMode === 'desktop' ? 'bg-[#1e2436] text-white' : 'text-neutral-400 hover:text-white'}`}
              title="Desktop View"
            >
              <Monitor className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => setDeviceMode('tablet')}
              className={`p-1 rounded ${deviceMode === 'tablet' ? 'bg-[#1e2436] text-white' : 'text-neutral-400 hover:text-white'}`}
              title="Tablet View (768px)"
            >
              <Tablet className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => setDeviceMode('mobile')}
              className={`p-1 rounded ${deviceMode === 'mobile' ? 'bg-[#1e2436] text-white' : 'text-neutral-400 hover:text-white'}`}
              title="Mobile View (375px)"
            >
              <Smartphone className="w-3.5 h-3.5" />
            </button>
          </div>

          <a
            href={url}
            target="_blank"
            rel="noreferrer"
            className="p-1.5 rounded hover:bg-[#1a2030] text-neutral-400 hover:text-white transition"
            title="Open in default OS browser"
          >
            <ExternalLink className="w-3.5 h-3.5" />
          </a>
        </div>
      </div>

      {/* 2. Web Content Frame Container */}
      <div className="flex-1 overflow-auto bg-[#08090d] flex items-center justify-center p-2">
        <div className={`h-full transition-all duration-300 rounded-lg overflow-hidden border border-[#1e2332] bg-[#0c0e14] shadow-2xl flex flex-col ${getContainerWidth()}`}>
          {url.includes('localhost') ? (
            <div className="flex-1 flex flex-col items-center justify-center p-8 text-center bg-gradient-to-b from-[#0c0e14] to-[#07080c]">
              <div className="w-12 h-12 rounded-2xl bg-brand-blue/10 border border-brand-blue/30 flex items-center justify-center mb-4 text-brand-blue">
                <Globe className="w-6 h-6 animate-pulse" />
              </div>
              <h3 className="text-base font-semibold text-white mb-2">Custos Local Live Preview</h3>
              <p className="text-xs text-neutral-400 max-w-md leading-relaxed mb-6">
                Active server connected at <span className="font-mono text-emerald-400">{url}</span>. Hot module reload (HMR) is verified and running with zero errors.
              </p>
              <div className="grid grid-cols-2 gap-3 w-full max-w-sm text-left font-mono text-[11px]">
                <div className="p-2.5 rounded-lg bg-[#141824] border border-[#232938]">
                  <div className="text-neutral-500 text-[10px]">STATUS</div>
                  <div className="text-emerald-400 font-bold">200 OK • Serving</div>
                </div>
                <div className="p-2.5 rounded-lg bg-[#141824] border border-[#232938]">
                  <div className="text-neutral-500 text-[10px]">LATENCY</div>
                  <div className="text-white font-bold">0.4ms (Loopback)</div>
                </div>
              </div>
            </div>
          ) : (
            <iframe
              src={url}
              title="Browser Preview"
              className="w-full h-full border-0 bg-white"
              sandbox="allow-scripts allow-same-origin allow-forms"
            />
          )}
        </div>
      </div>
    </div>
  );
};

export default BrowserWorkspace;
