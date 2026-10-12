import React, { useEffect, useRef, useState } from 'react';
import { ArrowLeft, ArrowRight, RefreshCw, Globe, Loader2, ExternalLink, MessageSquarePlus } from 'lucide-react';
import { useAppContext } from '@/context/AppContext';
import { browserCommand, type BrowserState } from '@/api/native_browser';

interface Props {
  tabId: string;
  url: string;
  active: boolean;
  onUrlChange: (url: string) => void;
  onAskAgent?: (draft: string) => void;
}

export function normalizeBrowserUrl(raw: string): string {
  const value = raw.trim();
  if (!value || value === 'about:blank') return 'about:blank';
  if (/^:\d+(\/.*)?$/.test(value)) return `http://localhost${value}`;
  if (/^(localhost|127\.0\.0\.1|0\.0\.0\.0)(:\d+)?(\/.*)?$/i.test(value)) return `http://${value}`;
  if (/^[a-z][a-z\d+.-]*:/i.test(value)) return value;
  if (/^[^\s]+\.[a-z]{2,}(:\d+)?(\/.*)?$/i.test(value)) return `https://${value}`;
  return `https://duckduckgo.com/?q=${encodeURIComponent(value)}`;
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
  const { uiScale } = useAppContext();
  const native = '__TAURI_INTERNALS__' in window;
  const [ready, setReady] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [address, setAddress] = useState(url === 'about:blank' ? '' : url);
  const [retry, setRetry] = useState(0);
  const [preset, setPreset] = useState(0);
  const [zoom, setZoom] = useState(100);
  const viewport = useRef<HTMLDivElement>(null);
  const lastUrl = useRef(url);
  const onChange = useRef(onUrlChange);
  onChange.current = onUrlChange;
  const snapshot = useRef({ active, ready, error });
  snapshot.current = { active, ready, error };

  useEffect(() => {
    if (!native) return;
    let disposed = false;
    const instance = crypto.randomUUID();
    let unlisten: (() => void) | undefined;
    setReady(false);
    setError(null);
    void (async () => {
      const { listen } = await import('@tauri-apps/api/event');
      const cleanup = await listen<BrowserState>('custos-browser-state', ({ payload }) => {
        if (disposed || payload.id !== tabId || payload.instance !== instance) return;
        setLoading(payload.loading);
        setError(payload.error);
        if (!payload.error) {
          lastUrl.current = payload.url;
          setAddress(payload.url === 'about:blank' ? '' : payload.url);
          onChange.current(payload.url);
        }
      });
      if (disposed) { cleanup(); return; }
      unlisten = cleanup;
      await browserCommand(tabId, 'create', { url: lastUrl.current, instance });
      if (!disposed) setReady(true);
    })().catch((reason) => { if (!disposed) setError(String(reason)); });
    return () => {
      disposed = true;
      unlisten?.();
      void browserCommand(tabId, 'close').catch((reason) => console.error('Browser cleanup failed', reason));
    };
  }, [tabId, native, retry]);

  useEffect(() => {
    if (!ready || !native) return;
    let stopped = false;
    let frame = 0;
    let signature = '';
    // Native surfaces cannot participate in CSS stacking. Watch application
    // overlays and layout/zoom changes, including animated split resizing.
    const update = () => {
      if (stopped) return;
      const el = viewport.current;
      if (el) {
        const rect = el.getBoundingClientRect();
        const overlay = Array.from(document.querySelectorAll('[aria-modal="true"], [role="dialog"], [role="menu"], [role="listbox"], .fixed, .absolute, [data-browser-overlay]')).some((node) => {
          const style = getComputedStyle(node);
          const bounds = node.getBoundingClientRect();
          if (style.display === 'none' || style.visibility === 'hidden' || bounds.width <= 0 || bounds.height <= 0) return false;
          const explicit = node.matches('[aria-modal="true"], [role="dialog"], [role="menu"], [role="listbox"], [data-browser-overlay]');
          const elevated = Number(style.zIndex) >= 40;
          return (explicit || elevated) && bounds.left < rect.right && bounds.right > rect.left && bounds.top < rect.bottom && bounds.bottom > rect.top;
        });
        const state = snapshot.current;
        const visible = state.active && state.ready && !state.error && !overlay && !document.hidden && rect.width > 0 && rect.height > 0;
        // CSS pixels become native logical pixels after the main WebView zoom.
        const scale = uiScale / 100;
        const fields = { x: rect.x * scale, y: rect.y * scale, width: rect.width * scale, height: rect.height * scale, visible };
        const next = JSON.stringify(fields);
        if (next !== signature) {
          signature = next;
          void browserCommand(tabId, 'layout', fields).catch((reason) => { if (!stopped) setError(String(reason)); });
        }
      }

    };
    if (!active) { update(); return () => { stopped = true; }; }
    const schedule = () => { cancelAnimationFrame(frame); if (document.hidden) update(); else frame = requestAnimationFrame(update); };
    const observer = new ResizeObserver(schedule);
    const overlays = new MutationObserver(schedule);
    overlays.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ['class', 'style', 'role', 'hidden', 'aria-modal', 'data-browser-overlay'] });
    window.addEventListener('resize', schedule);
    document.addEventListener('visibilitychange', schedule);
    document.addEventListener('scroll', schedule, true);
    if (viewport.current) observer.observe(viewport.current);
    update();
    return () => { stopped = true; cancelAnimationFrame(frame); observer.disconnect(); overlays.disconnect(); window.removeEventListener('resize', schedule); document.removeEventListener('visibilitychange', schedule); document.removeEventListener('scroll', schedule, true); };
  }, [ready, native, tabId, active, error, preset, uiScale]);

  useEffect(() => {
    if (!ready || !active || !native) return;
    let disposed = false;
    const timer = window.setInterval(() => {
      void browserCommand(tabId, 'state').then((current) => {
        if (!disposed && current && current !== lastUrl.current) {
          lastUrl.current = current;
          setAddress(current === 'about:blank' ? '' : current);
          onChange.current(current);
        }
      }).catch((reason) => { if (!disposed) setError(String(reason)); });
    }, 1000);
    return () => { disposed = true; clearInterval(timer); };
  }, [ready, active, native, tabId]);

  useEffect(() => {
    if (!loading) return;
    const timer = window.setTimeout(() => { setLoading(false); setError('Page loading timed out. Check the address or retry.'); }, 30000);
    return () => clearTimeout(timer);
  }, [loading]);

  useEffect(() => {
    if (!ready || !native) return;
    let disposed = false;
    void browserCommand(tabId, 'zoom', { zoom: zoom / 100 * uiScale / 100 }).catch((reason) => { if (!disposed) setError(String(reason)); });
    return () => { disposed = true; };
  }, [ready, native, tabId, zoom, uiScale]);

  const act = async (action: string, fields: Record<string, unknown> = {}) => {
    setError(null);
    try { await browserCommand(tabId, action, fields); }
    catch (reason) { setLoading(false); setError(String(reason)); }
  };
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
        <button type="button" className={button} aria-label="Insert page context into chat" disabled={!ready} onClick={() => onAskAgent?.(`Browser URL: ${lastUrl.current}\nViewport: ${selected.label}`)}><MessageSquarePlus size={15} /></button>
        <button type="button" className={button} aria-label="Open in external browser" disabled={!ready || lastUrl.current === 'about:blank'} onClick={() => { void import('@tauri-apps/plugin-opener').then(({ openUrl }) => openUrl(lastUrl.current)).catch((reason) => setError(String(reason))); }}><ExternalLink size={15} /></button>
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
          <div role="alert" className="absolute inset-0 flex flex-col items-center justify-center gap-3 p-6 bg-canvas"><p className="max-w-md break-words">{error}</p><button className="px-4 py-2 rounded bg-surface-2 border border-border-muted" onClick={() => { setError(null); if (ready) void act('reload'); else setRetry((value) => value + 1); }}>Retry</button></div>
        ) : !ready ? (
          <div role="status" aria-label="Preparing browser" className="absolute inset-0 p-8 bg-canvas space-y-5 motion-safe:animate-pulse">
            <span className="sr-only">Preparing browser...</span><div className="h-6 w-2/5 rounded bg-surface-2" /><div className="h-32 rounded bg-surface-1" /><div className="h-3 w-4/5 rounded bg-surface-2" /><div className="h-3 w-3/5 rounded bg-surface-2" />
          </div>
        ) : null}
      </div>
    </div>
  );
};
