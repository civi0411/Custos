import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  Smartphone,
  RotateCw,
  RefreshCw,
  Home,
  Globe,
  ArrowLeft,
  ArrowRight,
  Loader2,
  Hand,
  MousePointer,
  Bot,
  ShieldCheck,
} from 'lucide-react';
import { NativeSelect } from '@/components/common/NativeSelect';
import { normalizeBrowserUrl } from '@/api/native_browser';
import { useNativeBrowser } from '@/hooks/useNativeBrowser';

export interface DeviceSpec {
  id: string;
  name: string;
  kind: 'phone' | 'tablet';
  width: number;
  height: number;
  dpr: number;
}

const DEVICES: DeviceSpec[] = [
  { id: 'iphone-15-pro', name: 'iPhone 15 Pro', kind: 'phone', width: 393, height: 852, dpr: 3 },
  { id: 'iphone-15-pro-max', name: 'iPhone 15 Pro Max', kind: 'phone', width: 430, height: 932, dpr: 3 },
  { id: 'pixel-8', name: 'Google Pixel 8', kind: 'phone', width: 412, height: 915, dpr: 2.625 },
  { id: 'galaxy-s24', name: 'Samsung Galaxy S24', kind: 'phone', width: 412, height: 919, dpr: 3 },
  { id: 'iphone-se', name: 'iPhone SE', kind: 'phone', width: 375, height: 667, dpr: 2 },
  { id: 'ipad-mini', name: 'iPad Mini', kind: 'tablet', width: 744, height: 1133, dpr: 2 },
  { id: 'ipad-pro', name: 'iPad Pro 11"', kind: 'tablet', width: 834, height: 1194, dpr: 2 },
];

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

export const MobileSimulatorWorkbenchPane: React.FC<MobileSimulatorWorkbenchPaneProps> = ({
  tabId,
  url,
  active,
  onUrlChange,
  onAskAgent,
}) => {
  const [deviceId, setDeviceId] = useState(DEVICES[0].id);
  const [landscape, setLandscape] = useState(false);
  const [requestedScale, setRequestedScale] = useState(0.85);
  const [pageUrl, setPageUrl] = useState(url);
  const [address, setAddress] = useState(url === 'about:blank' ? '' : url);
  const [canvasSize, setCanvasSize] = useState({ width: 0, height: 0 });
  const [touchMode, setTouchMode] = useState(true);
  const [touchCursor, setTouchCursor] = useState<{ x: number; y: number; active: boolean } | null>(null);

  const canvas = useRef<HTMLDivElement>(null);
  const viewport = useRef<HTMLDivElement>(null);

  const device = DEVICES.find((item) => item.id === deviceId) ?? DEVICES[0];
  const width = landscape ? device.height : device.width;
  const height = landscape ? device.width : device.height;

  const scale = Math.max(
    0.1,
    Math.min(
      requestedScale,
      (canvasSize.width - 32) / width,
      (canvasSize.height - 32) / height
    )
  );
  const home = pageUrl === 'about:blank';

  const urlChanged = useCallback(
    (current: string) => {
      setPageUrl(current);
      setAddress(current === 'about:blank' ? '' : current);
      onUrlChange(current);
    },
    [onUrlChange]
  );

  const browser = useNativeBrowser({
    tabId,
    url: pageUrl,
    active,
    enabled: !home,
    viewport,
    zoom: scale,
    layoutKey: `${deviceId}:${landscape}:${scale}`,
    userAgent: mobileUserAgent,
    containFullscreen: true,
    onUrlChange: urlChanged,
  });

  useEffect(() => {
    const el = canvas.current;
    if (!el) return;
    const update = () => {
      const rect = el.getBoundingClientRect();
      setCanvasSize((previous) =>
        previous.width === rect.width && previous.height === rect.height
          ? previous
          : { width: rect.width, height: rect.height }
      );
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
      if (!['http:', 'https:'].includes(parsed.protocol) && target !== 'about:blank') {
        throw new Error('Only HTTP, HTTPS and about:blank are supported');
      }
      browser.setError(null);
      if (target === 'about:blank') {
        setPageUrl(target);
        setAddress('');
        onUrlChange(target);
      } else {
        setPageUrl(target);
        setAddress(target);
        onUrlChange(target);
        if (browser.ready) void browser.command('navigate', { url: target });
      }
    } catch (reason) {
      browser.setError(String(reason));
    }
  };

  const handleScreenPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!touchMode) return;
    const rect = e.currentTarget.getBoundingClientRect();
    setTouchCursor({
      x: e.clientX - rect.left,
      y: e.clientY - rect.top,
      active: (e.buttons & 1) === 1,
    });
  };

  const handleScreenPointerLeave = () => {
    setTouchCursor(null);
  };

  const handleSendToAgent = () => {
    if (!onAskAgent) return;
    const snippet = `[Mobile Simulator Context]
Device: ${device.name} (${device.kind})
Viewport: ${width} × ${height} CSS px (DPR: ${device.dpr}x)
Orientation: ${landscape ? 'Landscape' : 'Portrait'}
Active URL: ${browser.currentUrl || pageUrl || 'about:blank'}
Engine: Chromium Mobile Emulation
User Agent: ${mobileUserAgent || 'Default Mobile'}`;
    onAskAgent(snippet);
  };

  const control =
    'h-7 w-7 shrink-0 rounded flex items-center justify-center hover:bg-surface-2 transition-colors disabled:opacity-30 disabled:pointer-events-none text-fg-muted hover:text-editor-fg';

  return (
    <div className="h-full flex flex-col bg-canvas text-editor-fg text-xs overflow-hidden select-none">
      {/* Top Toolbar */}
      <div className="shrink-0 flex flex-wrap items-center gap-2 border-b border-border-muted bg-surface-1 px-3 py-1.5">
        <div className="flex items-center gap-1.5 font-medium">
          <Smartphone size={14} className="text-[var(--color-accent)] shrink-0" />
          <span className="font-semibold text-editor-fg">Mobile Simulator</span>
          <span className="text-[10px] text-fg-muted px-1.5 py-0.5 rounded bg-surface-2 font-mono">
            {width}×{height}
          </span>
        </div>

        <div className="ml-auto flex items-center gap-1.5">
          <NativeSelect
            aria-label="Preview device"
            value={deviceId}
            onChange={(event) => setDeviceId(event.target.value)}
          >
            {DEVICES.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name} ({item.width}×{item.height})
              </option>
            ))}
          </NativeSelect>

          <button
            className={control}
            aria-label="Rotate orientation"
            title={`Rotate to ${landscape ? 'Portrait' : 'Landscape'}`}
            onClick={() => setLandscape((value) => !value)}
          >
            <RotateCw size={13} className={landscape ? 'text-[var(--color-accent)]' : ''} />
          </button>

          <button
            className={`${control} ${touchMode ? 'text-[var(--color-accent)] bg-surface-2' : ''}`}
            aria-label="Toggle touch pointer cursor"
            title={touchMode ? 'Touch cursor active' : 'Standard mouse cursor'}
            onClick={() => setTouchMode((v) => !v)}
          >
            {touchMode ? <Hand size={13} /> : <MousePointer size={13} />}
          </button>

          <div className="h-4 w-px bg-border-muted mx-0.5" />

          <button
            className="flex items-center gap-1.5 px-2 py-1 rounded bg-[var(--color-accent)] hover:opacity-90 text-white font-medium text-[11px] shadow-sm transition-all"
            title="Send device specs and mobile viewport to Agent Chat"
            onClick={handleSendToAgent}
          >
            <Bot size={13} />
            <span>Ask Agent</span>
          </button>
        </div>
      </div>

      {/* Address Bar & Navigation */}
      <form
        className="shrink-0 flex items-center gap-1.5 border-b border-border-muted px-3 py-1.5 bg-canvas"
        onSubmit={(event) => {
          event.preventDefault();
          navigate();
        }}
      >
        <button
          type="button"
          className={control}
          aria-label="Phone back"
          disabled={!browser.ready}
          onClick={() => void browser.command('back')}
          title="Back"
        >
          <ArrowLeft size={13} />
        </button>
        <button
          type="button"
          className={control}
          aria-label="Phone forward"
          disabled={!browser.ready}
          onClick={() => void browser.command('forward')}
          title="Forward"
        >
          <ArrowRight size={13} />
        </button>
        <button
          type="button"
          className={control}
          aria-label="Phone home"
          onClick={() => navigate('about:blank')}
          title="Home"
        >
          <Home size={13} />
        </button>
        <button
          type="button"
          className={control}
          aria-label="Reload phone preview"
          disabled={!browser.ready}
          onClick={() => void browser.command('reload')}
          title="Reload"
        >
          {browser.loading ? <Loader2 size={13} className="animate-spin" /> : <RefreshCw size={13} />}
        </button>

        <div className="relative min-w-0 flex-1 flex items-center">
          <div className="absolute left-2.5 pointer-events-none text-fg-muted">
            {address.startsWith('https://') ? (
              <ShieldCheck size={13} className="text-emerald-400" />
            ) : (
              <Globe size={13} />
            )}
          </div>
          <input
            aria-label="Phone browser address"
            className="w-full rounded border border-border-default bg-surface-1 pl-8 pr-2 py-1 font-mono text-xs text-editor-fg focus:outline-none focus:border-[var(--color-accent)] transition-colors"
            value={address}
            onChange={(event) => setAddress(event.target.value)}
            placeholder="Enter URL, :5173, :3000 or search"
          />
        </div>

        <button
          type="submit"
          disabled={!browser.native}
          className="rounded px-2.5 py-1 bg-surface-2 hover:bg-surface-3 text-editor-fg font-medium disabled:opacity-30 transition-colors text-xs"
        >
          Go
        </button>
      </form>

      {/* Dev Hosts Quick Links */}
      <div className="shrink-0 flex items-center gap-1.5 px-3 py-1 border-b border-border-muted/50 bg-surface-1/40 text-[10px] text-fg-muted overflow-x-auto">
        <span className="font-semibold uppercase tracking-wider text-[9px] text-fg-subtle">Hosts:</span>
        <button
          type="button"
          onClick={() => navigate('http://localhost:5173')}
          className="px-1.5 py-0.5 rounded hover:bg-surface-2 hover:text-editor-fg transition-colors font-mono"
        >
          :5173
        </button>
        <button
          type="button"
          onClick={() => navigate('http://localhost:3000')}
          className="px-1.5 py-0.5 rounded hover:bg-surface-2 hover:text-editor-fg transition-colors font-mono"
        >
          :3000
        </button>
        <button
          type="button"
          onClick={() => navigate('http://localhost:8080')}
          className="px-1.5 py-0.5 rounded hover:bg-surface-2 hover:text-editor-fg transition-colors font-mono"
        >
          :8080
        </button>
        <button
          type="button"
          onClick={() => navigate('https://duckduckgo.com')}
          className="px-1.5 py-0.5 rounded hover:bg-surface-2 hover:text-editor-fg transition-colors"
        >
          DuckDuckGo
        </button>
      </div>

      {/* Progress Line */}
      <div
        className="h-0.5 shrink-0 bg-border-muted"
        role="progressbar"
        aria-label="Loading phone page"
        aria-hidden={!browser.loading}
      >
        {browser.loading && <div className="h-full bg-[var(--color-accent)] animate-pulse" />}
      </div>

      {/* Responsive Viewport Canvas */}
      <div
        ref={canvas}
        className="relative min-h-0 flex-1 flex items-center justify-center overflow-hidden bg-canvas-inset p-4"
      >
        <div
          className="relative shrink-0 transition-transform duration-150"
          style={{
            width: width * scale,
            height: height * scale,
          }}
        >
          <div
            className="absolute left-0 top-0 origin-top-left rounded-lg border border-border-default/80 bg-canvas shadow-2xl overflow-hidden flex flex-col ring-1 ring-white/10"
            style={{
              width,
              height,
              transform: `scale(${scale})`,
            }}
            onPointerMove={handleScreenPointerMove}
            onPointerLeave={handleScreenPointerLeave}
          >
            {/* Viewport Surface */}
            <div className="relative min-h-0 flex-1 bg-canvas overflow-hidden">
              <div ref={viewport} className="absolute inset-0 bg-canvas" />

              {!browser.native ? (
                <div className="absolute inset-0 flex items-center justify-center bg-canvas p-8 text-center text-fg-muted z-20">
                  Open Custos Desktop to experience native mobile webview.
                </div>
              ) : browser.error ? (
                <div
                  role="alert"
                  className="absolute inset-0 flex flex-col items-center justify-center gap-3 bg-canvas p-6 text-center z-20"
                >
                  <p className="text-rose-400 font-medium text-xs">{browser.error}</p>
                  <button
                    className="rounded border border-border-default bg-surface-2 px-3 py-1.5 text-xs hover:bg-surface-3 transition-colors"
                    onClick={browser.retry}
                  >
                    Retry Connection
                  </button>
                </div>
              ) : home ? (
                <div className="absolute inset-0 bg-canvas flex flex-col items-center justify-center gap-4 p-6 text-center z-20">
                  <div className="rounded-xl bg-[var(--color-accent-subtle)] p-3 text-[var(--color-accent)]">
                    <Globe size={28} />
                  </div>
                  <div>
                    <h2 className="text-sm font-semibold text-editor-fg">{device.name}</h2>
                    <p className="mt-1 text-xs text-fg-muted font-mono">
                      {width} × {height} CSS px • {device.dpr}x DPR
                    </p>
                  </div>
                  <div className="flex flex-wrap justify-center gap-1.5">
                    <button
                      className="rounded border border-border-default bg-surface-1 px-2.5 py-1 hover:bg-surface-2 text-xs font-mono transition-colors"
                      onClick={() => navigate('http://localhost:5173')}
                    >
                      :5173
                    </button>
                    <button
                      className="rounded border border-border-default bg-surface-1 px-2.5 py-1 hover:bg-surface-2 text-xs font-mono transition-colors"
                      onClick={() => navigate('http://localhost:3000')}
                    >
                      :3000
                    </button>
                  </div>
                </div>
              ) : !browser.ready ? (
                <div
                  role="status"
                  aria-label="Preparing phone preview"
                  className="absolute inset-0 bg-canvas p-6 space-y-3 motion-safe:animate-pulse z-20"
                >
                  <div className="h-4 w-3/4 rounded bg-surface-2" />
                  <div className="h-32 rounded bg-surface-1" />
                  <div className="h-3 w-1/2 rounded bg-surface-2" />
                </div>
              ) : null}

              {/* Touch Pointer Simulation Overlay (Cursor Dot) */}
              {touchMode && touchCursor && (
                <div
                  className="pointer-events-none absolute z-50 -translate-x-1/2 -translate-y-1/2 transition-transform duration-75"
                  style={{ left: touchCursor.x, top: touchCursor.y }}
                >
                  <div
                    className={`size-6 rounded-full border border-white/60 bg-white/20 backdrop-blur-xs shadow-md transition-transform ${
                      touchCursor.active ? 'scale-90 bg-white/40 ring-2 ring-[var(--color-accent)]' : ''
                    }`}
                  >
                    <div className="absolute inset-0 m-auto size-1.5 rounded-full bg-white/80" />
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Footer Metrics & Responsive Scale Slider */}
      <div className="shrink-0 flex items-center justify-between gap-3 border-t border-border-muted bg-surface-1 px-3 py-1.5 text-[10px] text-fg-muted">
        <div className="flex items-center gap-2">
          <span className="font-mono text-editor-fg">
            {width} × {height} CSS px
          </span>
          <span>•</span>
          <span>{device.dpr}x DPR</span>
          <span>•</span>
          <span className="capitalize">{landscape ? 'Landscape' : 'Portrait'}</span>
        </div>

        <label
          className="flex items-center gap-2"
          title="Zoom and scale preview to fit canvas."
        >
          <span>Scale</span>
          <input
            style={
              {
                '--range-progress': `${((requestedScale * 100 - 30) / 90) * 100}%`
              } as React.CSSProperties
            }
            aria-label="Phone preview scale"
            type="range"
            min="30"
            max="120"
            step="5"
            value={requestedScale * 100}
            onChange={(event) => setRequestedScale(Number(event.target.value) / 100)}
            className="w-24 cursor-pointer accent-[var(--color-accent)]"
          />
          <span className="w-8 font-mono text-editor-fg">{Math.round(scale * 100)}%</span>
        </label>
      </div>
    </div>
  );
};
