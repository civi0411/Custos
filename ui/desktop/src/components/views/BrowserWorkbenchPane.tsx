import React, { useRef, useState } from 'react';
import { ArrowLeft, ArrowRight, RefreshCw, Globe, Loader2, ExternalLink, MessageSquarePlus } from 'lucide-react';
import { useNativeBrowser } from '@/hooks/useNativeBrowser';
import { normalizeBrowserUrl } from '@/api/native_browser';
export { normalizeBrowserUrl } from '@/api/native_browser';

interface Props {
  tabId: string;
  url: string;
  active: boolean;
  onUrlChange: (url: string) => void;
  onAskAgent?: (draft: string) => void;
}


const presets = [
  { label: 'Responsive', width: 0, height: 0 },
  { label: 'Mobile S (320 x 568)', width: 320, height: 568 },
  { label: 'Mobile M (375 x 667)', width: 375, height: 667 },
  { label: 'Mobile L (425 x 812)', width: 425, height: 812 },
  { label: 'Tablet (768 x 1024)', width: 768, height: 1024 },
  { label: 'Laptop (1024 x 768)', width: 1024, height: 768 },
  { label: 'Laptop L (1440 x 900)', width: 1440, height: 900 },
];

export const BrowserWorkbenchPane: React.FC<Props> = ({ tabId, url, active, onUrlChange, onAskAgent }) => {
  const [address, setAddress] = useState(url === 'about:blank' ? '' : url);
  const [preset, setPreset] = useState(0);
  const [zoom, setZoom] = useState(100);
  const viewport = useRef<HTMLDivElement>(null);
  const browser = useNativeBrowser({ tabId, url, active, viewport, zoom: zoom / 100, layoutKey: preset,
    onUrlChange: (current) => { setAddress(current === 'about:blank' ? '' : current); onUrlChange(current); },
  });
  const { native, ready, loading, error, command: act, setError } = browser;
  const navigate = () => {
    const target = normalizeBrowserUrl(address);
    try {
      const parsed = new URL(target);
      if (!['http:', 'https:'].includes(parsed.protocol) && target !== 'about:blank') throw new Error('Only HTTP, HTTPS and about:blank are supported');
      void act('navigate', { url: target });
    } catch (reason) { setError(String(reason)); }
  };
  const selected = presets[preset];
  const button = 'h-7 w-7 shrink-0 flex items-center justify-center rounded hover:bg-surface-2 disabled:opacity-30';
  return (
    <div className="h-full flex flex-col bg-canvas text-editor-fg text-xs">
      <form className="h-11 shrink-0 flex items-center gap-1 border-b border-border-muted bg-surface-1 px-2" onSubmit={(event) => { event.preventDefault(); navigate(); }}>
        <button type="button" className={button} aria-label="Back" disabled={!ready} onClick={() => void act('back')}><ArrowLeft size={15} /></button>
        <button type="button" className={button} aria-label="Forward" disabled={!ready} onClick={() => void act('forward')}><ArrowRight size={15} /></button>
        <button type="button" className={button} aria-label="Reload" disabled={!ready} onClick={() => void act('reload')}>{loading ? <Loader2 size={15} className="animate-spin" /> : <RefreshCw size={15} />}</button>
        <Globe size={14} className="shrink-0 text-fg-muted" />
        <input aria-label="Browser address" className="min-w-0 flex-1 h-7 px-2 bg-canvas border border-border-muted rounded font-mono focus:outline-none focus:border-[var(--color-accent)]" value={address} onChange={(event) => setAddress(event.target.value)} placeholder="Enter URL, :5173 or search" />
        <button type="submit" className="px-2 h-7 rounded hover:bg-surface-2 disabled:opacity-30" disabled={!ready}>Go</button>
        <button type="button" className={button} aria-label="Insert page context into chat" disabled={!ready} onClick={() => onAskAgent?.(`Browser URL: ${browser.currentUrl}\nViewport: ${selected.label}`)}><MessageSquarePlus size={15} /></button>
        <button type="button" className={button} aria-label="Open in external browser" disabled={!ready || browser.currentUrl === 'about:blank'} onClick={() => { void import('@tauri-apps/plugin-opener').then(({ openUrl }) => openUrl(browser.currentUrl)).catch((reason) => setError(String(reason))); }}><ExternalLink size={15} /></button>
      </form>
      <div className="h-8 shrink-0 flex items-center gap-3 px-3 bg-surface-1 border-b border-border-muted">
        <select aria-label="Browser viewport" className="bg-surface-1 text-editor-fg" value={preset} onChange={(event) => setPreset(Number(event.target.value))}>{presets.map((item, index) => <option key={item.label} value={index}>{item.label}</option>)}</select>
        <select aria-label="Browser zoom" className="bg-surface-1 text-editor-fg" value={zoom} disabled={!ready} onChange={(event) => { const value = Number(event.target.value); setZoom(value); }}>{[50,75,100,125,150,200].map((value) => <option key={value} value={value}>{value}%</option>)}</select>
        <span className="ml-auto text-fg-muted">Manual browser</span>
      </div>
      <div className="h-0.5 shrink-0 bg-border-muted" role="progressbar" aria-label="Loading page" aria-hidden={!loading}>{loading && <div className="h-full bg-[var(--color-accent)] animate-pulse" />}</div>
      <div className="relative min-h-0 flex-1 overflow-auto flex justify-center bg-surface-0">
        <div ref={viewport} className="shrink-0 bg-canvas" style={{ width: selected.width ? `min(100%, ${selected.width}px)` : '100%', height: selected.height ? `min(100%, ${selected.height}px)` : '100%' }} />
        {!native ? <div className="absolute inset-0 flex items-center justify-center p-6 text-fg-muted">Open Custos desktop to use the native Browser.</div> : error ? (
          <div role="alert" className="absolute inset-0 flex flex-col items-center justify-center gap-3 p-6 bg-canvas"><p className="max-w-md break-words">{error}</p><button className="px-4 py-2 rounded bg-surface-2 border border-border-muted" onClick={browser.retry}>Retry</button></div>
        ) : !ready ? (
          <div role="status" aria-label="Preparing browser" className="absolute inset-0 p-8 bg-canvas space-y-5 motion-safe:animate-pulse">
            <span className="sr-only">Preparing browser...</span><div className="h-6 w-2/5 rounded bg-surface-2" /><div className="h-32 rounded bg-surface-1" /><div className="h-3 w-4/5 rounded bg-surface-2" /><div className="h-3 w-3/5 rounded bg-surface-2" />
          </div>
        ) : null}
      </div>
    </div>
  );
};
