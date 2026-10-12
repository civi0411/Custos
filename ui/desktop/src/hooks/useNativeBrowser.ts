import { useCallback, useEffect, useRef, useState, type RefObject } from 'react';
import { browserCommand, type BrowserState } from '@/api/native_browser';
import { useAppContext } from '@/context/AppContext';

export interface NativeBrowserOptions {
  tabId: string;
  url: string;
  active: boolean;
  enabled?: boolean;
  viewport: RefObject<HTMLDivElement | null>;
  zoom?: number;
  layoutKey?: string | number;
  userAgent?: string;
  containFullscreen?: boolean;
  onUrlChange?: (url: string) => void;
}

export function useNativeBrowser({ tabId, url, active, enabled = true, viewport, zoom = 1, layoutKey, userAgent, containFullscreen = false, onUrlChange }: NativeBrowserOptions) {
  const { uiScale } = useAppContext();
  const native = '__TAURI_INTERNALS__' in window;
  const [ready, setReady] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [retryVersion, setRetryVersion] = useState(0);
  const [currentUrl, setCurrentUrl] = useState(url);
  const lastUrl = useRef(url);
  lastUrl.current = url;
  const onChange = useRef(onUrlChange);
  onChange.current = onUrlChange;
  const snapshot = useRef({ active, ready, error });
  snapshot.current = { active, ready, error };

  useEffect(() => {
    if (!native || !enabled) { setReady(false); setLoading(false); return; }
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
          setCurrentUrl(payload.url);
          onChange.current?.(payload.url);
        }
      });
      if (disposed) { cleanup(); return; }
      unlisten = cleanup;
      await browserCommand(tabId, 'create', { url: lastUrl.current, instance, user_agent: userAgent, contain_fullscreen: containFullscreen });
      if (!disposed) setReady(true);
    })().catch((reason) => { if (!disposed) setError(String(reason)); });
    return () => {
      disposed = true;
      unlisten?.();
      void browserCommand(tabId, 'close').catch((reason) => console.error('Browser cleanup failed', reason));
    };
  }, [tabId, native, enabled, retryVersion, userAgent, containFullscreen]);

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
  }, [ready, native, tabId, active, error, layoutKey, uiScale, zoom, viewport]);

  useEffect(() => {
    if (!ready || !active || !native) return;
    let disposed = false;
    const timer = window.setInterval(() => {
      void browserCommand(tabId, 'state').then((current) => {
        if (!disposed && current && current !== lastUrl.current) {
          lastUrl.current = current;
          setCurrentUrl(current);
          onChange.current?.(current);
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
    void browserCommand(tabId, 'zoom', { zoom: zoom * uiScale / 100 }).catch((reason) => { if (!disposed) setError(String(reason)); });
    return () => { disposed = true; };
  }, [ready, native, tabId, zoom, uiScale]);

  const command = useCallback(async (action: string, fields: Record<string, unknown> = {}) => {
    setError(null);
    try { await browserCommand(tabId, action, fields); }
    catch (reason) { setLoading(false); setError(String(reason)); }
  }, [tabId]);
  const retry = () => {
    setError(null);
    if (ready) void command('reload');
    else setRetryVersion(value => value + 1);
  };
  return { native, ready, loading, error, currentUrl, command, retry, setError };
}
