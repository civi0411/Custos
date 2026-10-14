import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Smartphone, RotateCw, Power, RefreshCw, Home, Globe, ArrowLeft, ArrowRight, MessageSquarePlus, Loader2 } from 'lucide-react';
import { NativeSelect } from '@/components/common/NativeSelect';
import { normalizeBrowserUrl } from '@/api/native_browser';
import { useNativeBrowser } from '@/hooks/useNativeBrowser';

const DEVICES = [
  { id: 'iphone-15-pro', name: 'iPhone 15 Pro', width: 393, height: 852, radius: 48 },
  { id: 'pixel-8', name: 'Google Pixel 8', width: 412, height: 915, radius: 42 },
  { id: 'iphone-se', name: 'iPhone SE (3rd Gen)', width: 375, height: 667, radius: 32 },
  { id: 'ipad-mini', name: 'iPad Mini (6th Gen)', width: 744, height: 1133, radius: 28 },
];

// Request mobile layouts while preserving the installed Chromium media engine/version.
const mobileUserAgent = navigator.userAgent.includes('Chrome/')
  ? navigator.userAgent.replace(/\([^)]*\)/, '(Linux; Android 10; K)').replace(' Safari/', ' Mobile Safari/').replace(' Edg/', ' EdgA/')
  : undefined;

interface MobileSimulatorWorkbenchPaneProps {
  tabId: string;
  url: string;
  active: boolean;
  onUrlChange: (url: string) => void;
  onAskAgent?: (draft: string) => void;
}

export const MobileSimulatorWorkbenchPane: React.FC<MobileSimulatorWorkbenchPaneProps> = ({ tabId, url, active, onUrlChange, onAskAgent }) => {
  const [deviceId, setDeviceId] = useState(DEVICES[0].id);
  const [landscape, setLandscape] = useState(false);
  const [powered, setPowered] = useState(true);
  const [requestedScale, setRequestedScale] = useState(0.75);
  const [pageUrl, setPageUrl] = useState(url);
  const [address, setAddress] = useState(url === 'about:blank' ? '' : url);
  const [canvasSize, setCanvasSize] = useState({ width: 0, height: 0 });
  const canvas = useRef<HTMLDivElement>(null);
  const viewport = useRef<HTMLDivElement>(null);
  const device = DEVICES.find(item => item.id === deviceId) ?? DEVICES[0];
  const width = landscape ? device.height : device.width;
  const height = landscape ? device.width : device.height;
  const pageHeight = height - 88;
  const scale = Math.max(0.1, Math.min(requestedScale, (canvasSize.width - 32) / (width + 24), (canvasSize.height - 32) / (height + 24)));
  const home = pageUrl === 'about:blank';
  const urlChanged = useCallback((current: string) => {
    setPageUrl(current);
    setAddress(current === 'about:blank' ? '' : current);
    onUrlChange(current);
  }, [onUrlChange]);
  const browser = useNativeBrowser({ tabId, url: pageUrl, active, enabled: powered && !home,
    viewport, zoom: scale, layoutKey: `${deviceId}:${landscape}:${scale}`, userAgent: mobileUserAgent, containFullscreen: true, onUrlChange: urlChanged });

  useEffect(() => {
    const el = canvas.current;
    if (!el) return;
    const update = () => {
      const rect = el.getBoundingClientRect();
      setCanvasSize(previous => previous.width === rect.width && previous.height === rect.height ? previous : { width: rect.width, height: rect.height });
    };
    const observer = new ResizeObserver(update);
    observer.observe(el);
    update();
    return () => observer.disconnect();
  }, []);

  const navigate = (raw = address) => {
    const target = normalizeBrowserUrl(raw);
    try {
      const parsed = new URL(target);
      if (!['http:', 'https:'].includes(parsed.protocol) && target !== 'about:blank') throw new Error('Only HTTP, HTTPS and about:blank are supported');
      browser.setError(null);
      if (target === 'about:blank') {
        setPageUrl(target); setAddress(''); onUrlChange(target);
      } else {
        setPageUrl(target); setAddress(target); onUrlChange(target);
        if (browser.ready) void browser.command('navigate', { url: target });
      }
    } catch (reason) { browser.setError(String(reason)); }
  };
  const control = 'h-7 w-7 shrink-0 rounded flex items-center justify-center hover:bg-surface-2 disabled:opacity-30';
  return <div className="h-full flex flex-col bg-canvas text-editor-fg text-xs overflow-hidden">
    <div className="shrink-0 flex flex-wrap items-center gap-2 border-b border-border-muted bg-surface-1 px-3 py-2">
      <Smartphone size={15} className="text-[var(--color-accent)]" /><span className="font-semibold">Phone Simulator</span>
      <div className="ml-auto flex items-center gap-2">
        <NativeSelect aria-label="Preview device" value={deviceId} onChange={event => setDeviceId(event.target.value)}>{DEVICES.map(item => <option key={item.id} value={item.id}>{item.name}</option>)}</NativeSelect>
        <button className={control} aria-label="Rotate device" title="Rotate device" onClick={() => setLandscape(value => !value)}><RotateCw size={15} /></button>
        <button className={control} aria-label={powered ? 'Turn screen off' : 'Turn screen on'} title={powered ? 'Turn screen off' : 'Turn screen on'} onClick={() => setPowered(value => !value)}><Power size={15} className={powered ? 'text-[var(--color-accent)]' : 'text-fg-muted'} /></button>
      </div>
    </div>
    <form className="shrink-0 flex items-center gap-2 border-b border-border-muted px-3 py-2" onSubmit={event => { event.preventDefault(); navigate(); }}>
      <button type="button" className={control} aria-label="Phone home" disabled={!powered} onClick={() => navigate('about:blank')}><Home size={14} /></button>
      <button type="button" className={control} aria-label="Reload phone preview" disabled={!browser.ready || !powered} onClick={() => void browser.command('reload')}>{browser.loading ? <Loader2 size={14} className="animate-spin" /> : <RefreshCw size={14} />}</button>
      <input aria-label="Phone browser address" className="min-w-0 flex-1 rounded border border-border-default bg-surface-1 px-2 py-1.5 font-mono focus:outline-none focus:border-[var(--color-accent)]" value={address} onChange={event => setAddress(event.target.value)} placeholder="Enter URL, :5173 or search" />
      <button type="submit" disabled={!powered || !browser.native} className="rounded px-2 py-1.5 hover:bg-surface-2 disabled:opacity-30">Go</button>
      <button type="button" className={control} aria-label="Insert phone context into chat" disabled={home || !powered || !browser.ready} onClick={() => onAskAgent?.(`Phone web preview: ${device.name}\nURL: ${browser.currentUrl}\nViewport: ${width} x ${pageHeight} CSS pixels\nOrientation: ${landscape ? 'landscape' : 'portrait'}`)}><MessageSquarePlus size={15} /></button>
    </form>
    <div className="h-0.5 shrink-0 bg-border-muted" role="progressbar" aria-label="Loading phone page" aria-hidden={!browser.loading}>{browser.loading && <div className="h-full bg-[var(--color-accent)] animate-pulse" />}</div>
    <div ref={canvas} className="relative min-h-0 flex-1 flex items-center justify-center overflow-hidden bg-canvas-inset p-4">
      <div className="relative shrink-0" style={{ width: (width + 24) * scale, height: (height + 24) * scale }}>
        <div className="absolute left-0 top-0 border-2 border-zinc-600 bg-zinc-900 p-[10px] shadow-xl" style={{ width: width + 24, height: height + 24, borderRadius: device.radius, transform: `scale(${scale})`, transformOrigin: 'top left' }}>
          <div className="h-full flex flex-col overflow-hidden bg-canvas" style={{ borderRadius: device.radius - 12 }}>
            <div className="h-11 shrink-0 flex items-center justify-between px-5 bg-surface-1 text-editor-fg"><span className="font-semibold text-[11px]">{device.name}</span><span className="text-[10px] text-fg-muted">{powered ? browser.loading ? 'Loading' : home ? 'Home' : 'Web preview' : 'Off'}</span></div>
            <div className="relative min-h-0 flex-1">
              <div ref={viewport} className="absolute inset-0 bg-canvas" />
              {!powered ? <div className="absolute inset-0 flex items-center justify-center bg-black text-zinc-400"><Power size={26} /></div>
                : !browser.native ? <div className="absolute inset-0 flex items-center justify-center bg-canvas p-8 text-center text-fg-muted">Open Custos desktop to use the native phone preview.</div>
                : browser.error ? <div role="alert" className="absolute inset-0 flex flex-col items-center justify-center gap-4 bg-canvas p-8 text-center"><p>{browser.error}</p><button className="rounded border border-border-default bg-surface-2 px-4 py-2" onClick={browser.retry}>Retry</button></div>
                : home ? <div className="absolute inset-0 bg-canvas flex flex-col items-center justify-center gap-6 p-8 text-center">
                  <div className="rounded-2xl bg-[var(--color-accent-subtle)] p-5 text-[var(--color-accent)]"><Globe size={34} /></div><div><h2 className="text-lg font-semibold">Preview a mobile page</h2><p className="mt-2 text-sm text-fg-muted">Enter a web address above or open a local dev server.</p></div>
                  <div className="flex flex-wrap justify-center gap-3"><button className="rounded-lg border border-border-default bg-surface-1 px-4 py-2" onClick={() => navigate(':5173')}>Dev :5173</button><button className="rounded-lg border border-border-default bg-surface-1 px-4 py-2" onClick={() => navigate('https://duckduckgo.com')}>Search the web</button></div>
                </div>
                : !browser.ready ? <div role="status" aria-label="Preparing phone preview" className="absolute inset-0 bg-canvas p-8 space-y-5 motion-safe:animate-pulse"><span className="sr-only">Preparing phone preview...</span><div className="h-6 w-3/4 rounded bg-surface-2" /><div className="h-40 rounded bg-surface-1" /><div className="h-3 rounded bg-surface-2" /><div className="h-3 w-2/3 rounded bg-surface-2" /></div> : null}
            </div>
            <div className="h-11 shrink-0 flex items-center justify-center gap-6 border-t border-border-muted bg-surface-1">
              <button className={control} aria-label="Phone back" disabled={!browser.ready} onClick={() => void browser.command('back')}><ArrowLeft size={16} /></button>
              <button className={control} aria-label="Phone home screen" disabled={!powered} onClick={() => navigate('about:blank')}><Home size={16} /></button>
              <button className={control} aria-label="Phone forward" disabled={!browser.ready} onClick={() => void browser.command('forward')}><ArrowRight size={16} /></button>
            </div>
          </div>
        </div>
      </div>
    </div>
    <div className="shrink-0 flex items-center justify-between gap-3 border-t border-border-muted bg-surface-1 px-3 py-2 text-[10px] text-fg-muted">
      <span title="Responsive preview using the installed browser engine. Chromium requests mobile layouts with its actual engine version. Device presets change dimensions; OS, touch and DPR are not emulated.">{width} x {pageHeight} CSS viewport</span>
      <label className="flex items-center gap-2" title="Maximum preview scale; the device also fits the available canvas.">Scale<input style={{ '--range-progress': `${(requestedScale * 100 - 30) / 90 * 100}%` } as React.CSSProperties} aria-label="Phone preview scale" type="range" min="30" max="120" step="10" value={requestedScale * 100} onChange={event => setRequestedScale(Number(event.target.value) / 100)} /><span className="w-8 font-mono">{Math.round(scale * 100)}%</span></label>
    </div>
  </div>;
};
