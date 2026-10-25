import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  Smartphone,
  RotateCw,
  Power,
  RefreshCw,
  Home,
  Globe,
  ArrowLeft,
  ArrowRight,
  Loader2,
  Volume2,
  VolumeX,
  Volume1,
  Hand,
  MousePointer,
  Bot,
  ShieldCheck
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
  radius: number;
  cutout: 'island' | 'punch-hole' | 'notch' | 'tablet-bezel';
  dpr: number;
  hasActionButton?: boolean;
}

const DEVICES: DeviceSpec[] = [
  { id: 'iphone-15-pro', name: 'iPhone 15 Pro', kind: 'phone', width: 393, height: 852, radius: 50, cutout: 'island', dpr: 3, hasActionButton: true },
  { id: 'iphone-15-pro-max', name: 'iPhone 15 Pro Max', kind: 'phone', width: 430, height: 932, radius: 54, cutout: 'island', dpr: 3, hasActionButton: true },
  { id: 'pixel-8', name: 'Google Pixel 8', kind: 'phone', width: 412, height: 915, radius: 44, cutout: 'punch-hole', dpr: 2.625 },
  { id: 'galaxy-s24', name: 'Samsung Galaxy S24 Ultra', kind: 'phone', width: 412, height: 919, radius: 24, cutout: 'punch-hole', dpr: 3 },
  { id: 'iphone-se', name: 'iPhone SE (3rd Gen)', kind: 'phone', width: 375, height: 667, radius: 36, cutout: 'notch', dpr: 2 },
  { id: 'ipad-mini', name: 'iPad Mini (6th Gen)', kind: 'tablet', width: 744, height: 1133, radius: 30, cutout: 'tablet-bezel', dpr: 2 },
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

export const MobileSimulatorWorkbenchPane: React.FC<MobileSimulatorWorkbenchPaneProps> = ({
  tabId,
  url,
  active,
  onUrlChange,
  onAskAgent
}) => {
  const [deviceId, setDeviceId] = useState(DEVICES[0].id);
  const [landscape, setLandscape] = useState(false);
  const [powered, setPowered] = useState(true);
  const [requestedScale, setRequestedScale] = useState(0.75);
  const [pageUrl, setPageUrl] = useState(url);
  const [address, setAddress] = useState(url === 'about:blank' ? '' : url);
  const [canvasSize, setCanvasSize] = useState({ width: 0, height: 0 });
  const [touchMode, setTouchMode] = useState(true);
  const [touchCursor, setTouchCursor] = useState<{ x: number; y: number; active: boolean } | null>(null);
  const [agentDriving, setAgentDriving] = useState(false);

  // Hardware interaction state
  const [volumeLevel, setVolumeLevel] = useState(65);
  const [isMuted, setIsMuted] = useState(false);
  const [volumeHudTimer, setVolumeHudTimer] = useState<ReturnType<typeof setTimeout> | null>(null);
  const [showVolumeHud, setShowVolumeHud] = useState(false);
  const [hudMessage, setHudMessage] = useState<string | null>(null);

  const canvas = useRef<HTMLDivElement>(null);
  const viewport = useRef<HTMLDivElement>(null);

  const device = DEVICES.find((item) => item.id === deviceId) ?? DEVICES[0];
  const width = landscape ? device.height : device.width;
  const height = landscape ? device.width : device.height;
  const pageHeight = height - (landscape ? 54 : 88);
  const scale = Math.max(
    0.1,
    Math.min(
      requestedScale,
      (canvasSize.width - 48) / (width + 36),
      (canvasSize.height - 48) / (height + 36)
    )
  );
  const home = pageUrl === 'about:blank';

  const triggerHud = (message: string) => {
    setHudMessage(message);
    setShowVolumeHud(true);
    if (volumeHudTimer) clearTimeout(volumeHudTimer);
    const timer = setTimeout(() => {
      setShowVolumeHud(false);
      setHudMessage(null);
    }, 1600);
    setVolumeHudTimer(timer);
  };

  const handleVolumeUp = () => {
    const next = Math.min(100, volumeLevel + 10);
    setVolumeLevel(next);
    setIsMuted(false);
    triggerHud(`Volume ${next}%`);
  };

  const handleVolumeDown = () => {
    const next = Math.max(0, volumeLevel - 10);
    setVolumeLevel(next);
    if (next === 0) setIsMuted(true);
    triggerHud(next === 0 ? 'Mute' : `Volume ${next}%`);
  };

  const handleActionToggle = () => {
    setIsMuted((prev) => {
      const next = !prev;
      triggerHud(next ? 'Silent Mode On' : 'Silent Mode Off');
      return next;
    });
  };

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
    enabled: powered && !home,
    viewport,
    zoom: scale,
    layoutKey: `${deviceId}:${landscape}:${scale}`,
    userAgent: mobileUserAgent,
    containFullscreen: true,
    onUrlChange: urlChanged
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

  // Touch pointer simulation handlers
  const handleScreenPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!touchMode) return;
    const rect = e.currentTarget.getBoundingClientRect();
    setTouchCursor({
      x: e.clientX - rect.left,
      y: e.clientY - rect.top,
      active: (e.buttons & 1) === 1
    });
  };

  const handleScreenPointerLeave = () => {
    setTouchCursor(null);
  };

  const handleSendToAgent = () => {
    if (!onAskAgent) return;
    const snippet = `[Mobile Simulator Context]
Device: ${device.name} (${device.kind})
Viewport: ${width} × ${pageHeight} CSS px (DPR: ${device.dpr}x)
Orientation: ${landscape ? 'Landscape' : 'Portrait'}
Active URL: ${browser.currentUrl || pageUrl || 'about:blank'}
Status: ${powered ? (browser.loading ? 'Loading' : 'Rendered') : 'Powered Off'}
Engine: Chromium Mobile Emulation
User Agent: ${mobileUserAgent || 'Default Mobile'}`;
    onAskAgent(snippet);
  };

  const control =
    'h-7 w-7 shrink-0 rounded flex items-center justify-center hover:bg-surface-2 transition-colors disabled:opacity-30 disabled:pointer-events-none';

  return (
    <div className="h-full flex flex-col bg-canvas text-editor-fg text-xs overflow-hidden select-none">
      {/* Top Toolbar */}
      <div className="shrink-0 flex flex-wrap items-center gap-2 border-b border-border-muted bg-surface-1 px-3 py-2">
        <div className="flex items-center gap-1.5 font-medium">
          <Smartphone size={15} className="text-[var(--color-accent)] shrink-0" />
          <span className="font-semibold text-editor-fg">Mobile Simulator</span>
          <span className="text-[10px] text-fg-muted px-1.5 py-0.5 rounded bg-surface-2">
            {device.name}
          </span>
        </div>

        <div className="ml-auto flex items-center gap-1.5">
          {/* Device Selector */}
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

          {/* Orientation Rotate */}
          <button
            className={control}
            aria-label="Rotate orientation"
            title={`Rotate to ${landscape ? 'Portrait' : 'Landscape'}`}
            onClick={() => setLandscape((value) => !value)}
          >
            <RotateCw size={14} className={landscape ? 'text-[var(--color-accent)]' : ''} />
          </button>

          {/* Touch Pointer Simulation Toggle */}
          <button
            className={`${control} ${touchMode ? 'text-[var(--color-accent)] bg-surface-2' : 'text-fg-muted'}`}
            aria-label="Toggle touch pointer cursor"
            title={touchMode ? 'Touch cursor active' : 'Standard mouse cursor'}
            onClick={() => setTouchMode((v) => !v)}
          >
            {touchMode ? <Hand size={14} /> : <MousePointer size={14} />}
          </button>

          {/* Power Toggle */}
          <button
            className={control}
            aria-label={powered ? 'Turn screen off' : 'Turn screen on'}
            title={powered ? 'Lock / Power Off' : 'Wake / Power On'}
            onClick={() => setPowered((value) => !value)}
          >
            <Power size={14} className={powered ? 'text-white' : 'text-fg-muted'} />
          </button>

          <div className="h-4 w-px bg-border-muted mx-0.5" />

          {/* Ask Agent / Insert Mobile Context */}
          <button
            className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-[var(--color-accent)] hover:opacity-90 text-white font-medium text-[11px] shadow-sm transition-all"
            title="Send device specs and mobile viewport to Agent Chat"
            onClick={handleSendToAgent}
          >
            <Bot size={13} />
            <span>Ask Agent</span>
          </button>
        </div>
      </div>

      {/* Co-Browsing Take-Back Banner (Orca Parity) */}
      {agentDriving && (
        <div className="shrink-0 flex items-center justify-between gap-3 px-3 py-1.5 bg-white/5 border-b border-white/10 text-white/80 text-xs">
          <div className="flex items-center gap-2">
            <Bot size={14} className="animate-pulse text-white/80" />
            <span>AI Agent is inspecting and driving this mobile session</span>
          </div>
          <button
            onClick={() => setAgentDriving(false)}
            className="px-2 py-0.5 rounded bg-white/10 hover:bg-white/15 border border-white/20 text-white text-[11px] font-medium transition-colors"
          >
            Take Back Control
          </button>
        </div>
      )}

      {/* Address Bar & Navigation */}
      <form
        className="shrink-0 flex items-center gap-1.5 border-b border-border-muted px-3 py-2 bg-canvas"
        onSubmit={(event) => {
          event.preventDefault();
          navigate();
        }}
      >
        <button
          type="button"
          className={control}
          aria-label="Phone home"
          disabled={!powered}
          onClick={() => navigate('about:blank')}
          title="Home screen"
        >
          <Home size={14} />
        </button>
        <button
          type="button"
          className={control}
          aria-label="Reload phone preview"
          disabled={!browser.ready || !powered}
          onClick={() => void browser.command('reload')}
          title="Reload page"
        >
          {browser.loading ? <Loader2 size={14} className="animate-spin" /> : <RefreshCw size={14} />}
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
            className="w-full rounded border border-border-default bg-surface-1 pl-8 pr-2 py-1.5 font-mono text-xs text-editor-fg focus:outline-none focus:border-[var(--color-accent)] transition-colors"
            value={address}
            onChange={(event) => setAddress(event.target.value)}
            placeholder="Enter URL, :5173, :3000 or search"
          />
        </div>

        <button
          type="submit"
          disabled={!powered || !browser.native}
          className="rounded px-3 py-1.5 bg-surface-2 hover:bg-surface-3 text-editor-fg font-medium disabled:opacity-30 transition-colors"
        >
          Go
        </button>
      </form>

      {/* Quick Local Presets Strip */}
      <div className="shrink-0 flex items-center gap-1.5 px-3 py-1 border-b border-border-muted/50 bg-surface-1/50 text-[10px] text-fg-muted overflow-x-auto">
        <span className="font-semibold uppercase tracking-wider text-[9px]">Dev Hosts:</span>
        <button
          type="button"
          onClick={() => navigate('http://localhost:5173')}
          className="px-1.5 py-0.5 rounded hover:bg-surface-2 hover:text-editor-fg transition-colors"
        >
          Vite (:5173)
        </button>
        <button
          type="button"
          onClick={() => navigate('http://localhost:3000')}
          className="px-1.5 py-0.5 rounded hover:bg-surface-2 hover:text-editor-fg transition-colors"
        >
          Next.js (:3000)
        </button>
        <button
          type="button"
          onClick={() => navigate('http://localhost:8080')}
          className="px-1.5 py-0.5 rounded hover:bg-surface-2 hover:text-editor-fg transition-colors"
        >
          API (:8080)
        </button>
        <button
          type="button"
          onClick={() => navigate('https://duckduckgo.com')}
          className="px-1.5 py-0.5 rounded hover:bg-surface-2 hover:text-editor-fg transition-colors"
        >
          DuckDuckGo
        </button>
      </div>

      {/* Progress Bar */}
      <div
        className="h-0.5 shrink-0 bg-border-muted"
        role="progressbar"
        aria-label="Loading phone page"
        aria-hidden={!browser.loading}
      >
        {browser.loading && <div className="h-full bg-[var(--color-accent)] animate-pulse" />}
      </div>

      {/* Interactive Mobile Simulator Canvas with Hardware Framing */}
      <div
        ref={canvas}
        className="relative min-h-0 flex-1 flex items-center justify-center overflow-hidden bg-canvas-inset p-6"
      >
        {/* Scaled Hardware Frame Outset */}
        <div
          className="relative shrink-0 transition-transform duration-200"
          style={{
            width: (width + 36) * scale,
            height: (height + 36) * scale
          }}
        >
          <div
            className="absolute left-0 top-0 origin-top-left"
            style={{
              width: width + 36,
              height: height + 36,
              transform: `scale(${scale})`
            }}
          >
            {/* Physical Hardware Buttons (Left Side: Action Button, Volume Up, Volume Down) */}
            {!landscape && (
              <>
                {device.hasActionButton && (
                  <button
                    type="button"
                    title="Action Button (Mute / Unmute)"
                    onClick={handleActionToggle}
                    className="absolute -left-[5px] top-[14%] w-[5px] h-[30px] rounded-l-sm bg-zinc-600 hover:bg-zinc-400 active:w-[3px] transition-all cursor-pointer shadow-sm z-30"
                  />
                )}
                <button
                  type="button"
                  title="Volume Up"
                  onClick={handleVolumeUp}
                  className="absolute -left-[5px] top-[24%] w-[5px] h-[52px] rounded-l-sm bg-zinc-600 hover:bg-zinc-400 active:w-[3px] transition-all cursor-pointer shadow-sm z-30"
                />
                <button
                  type="button"
                  title="Volume Down"
                  onClick={handleVolumeDown}
                  className="absolute -left-[5px] top-[34%] w-[5px] h-[52px] rounded-l-sm bg-zinc-600 hover:bg-zinc-400 active:w-[3px] transition-all cursor-pointer shadow-sm z-30"
                />

                {/* Physical Hardware Button (Right Side: Power / Lock Button) */}
                <button
                  type="button"
                  title="Power / Lock Screen"
                  onClick={() => setPowered((v) => !v)}
                  className="absolute -right-[5px] top-[26%] w-[5px] h-[64px] rounded-r-sm bg-zinc-600 hover:bg-zinc-400 active:w-[3px] transition-all cursor-pointer shadow-sm z-30"
                />
              </>
            )}

            {/* Main Device Outer Chassis (Titanium/Aluminum Finish with Chamfer Ring) */}
            <div
              className="relative w-full h-full bg-zinc-950 p-[12px] shadow-[0_25px_60px_-15px_rgba(0,0,0,0.85),0_0_0_1px_rgba(255,255,255,0.12)] border-2 border-zinc-700/80"
              style={{ borderRadius: device.radius }}
            >
              {/* Screen Bezel & Container */}
              <div
                className="relative h-full flex flex-col overflow-hidden bg-black ring-1 ring-white/10"
                style={{ borderRadius: device.radius - 10 }}
                onPointerMove={handleScreenPointerMove}
                onPointerLeave={handleScreenPointerLeave}
              >
                {/* On-Screen Hardware Volume HUD / Alert Feedback */}
                {showVolumeHud && (
                  <div className="absolute top-4 left-4 z-50 flex items-center gap-2 px-3 py-1.5 rounded-full bg-zinc-900/90 text-white text-xs border border-white/20 shadow-xl backdrop-blur-md animate-in fade-in zoom-in-95 duration-150">
                    {isMuted ? (
                      <VolumeX size={14} className="text-red-400" />
                    ) : volumeLevel > 50 ? (
                      <Volume2 size={14} className="text-white/80" />
                    ) : (
                      <Volume1 size={14} className="text-white/60" />
                    )}
                    <span className="font-medium text-[11px]">{hudMessage}</span>
                  </div>
                )}

                {/* Device Top Status Bar & Notch / Dynamic Island */}
                <div className="h-10 shrink-0 relative flex items-center justify-between px-6 bg-zinc-950 text-white z-40 select-none">
                  {/* Left: Clock */}
                  <span className="font-semibold text-[11px] tracking-tight">
                    {new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                  </span>

                  {/* Center: Dynamic Island or Punch-Hole Camera Cutout */}
                  {device.cutout === 'island' && !landscape ? (
                    <div className="absolute left-1/2 -translate-x-1/2 top-2 h-7 w-28 bg-black rounded-full flex items-center justify-between px-2.5 shadow-inner border border-zinc-800/80">
                      <div className="size-2.5 rounded-full bg-zinc-900 border border-zinc-800" />
                      <div className="size-2 rounded-full bg-zinc-950 border border-zinc-800/50" />
                    </div>
                  ) : device.cutout === 'punch-hole' ? (
                    <div className="absolute left-1/2 -translate-x-1/2 top-2.5 size-3.5 bg-black rounded-full border border-zinc-800/80 shadow-inner" />
                  ) : null}

                  {/* Right: Battery and Network Icons */}
                  <div className="flex items-center gap-1.5 text-[10px] text-zinc-400">
                    <span>5G</span>
                    <div className="w-5 h-2.5 rounded-sm border border-zinc-400 p-0.5 flex items-center">
                      <div className="h-full w-3/4 bg-white/80 rounded-2xs" />
                    </div>
                  </div>
                </div>

                {/* Web Viewport Surface */}
                <div className="relative min-h-0 flex-1 bg-canvas overflow-hidden">
                  <div ref={viewport} className="absolute inset-0 bg-canvas" />

                  {/* Screen Off State */}
                  {!powered ? (
                    <div className="absolute inset-0 flex flex-col items-center justify-center bg-black text-zinc-600 gap-3 z-30">
                      <Power size={32} className="opacity-40 animate-pulse" />
                      <span className="text-[11px] text-zinc-500 font-medium">Screen Locked</span>
                      <button
                        onClick={() => setPowered(true)}
                        className="mt-2 px-3 py-1 rounded-full border border-zinc-700 bg-zinc-900 text-zinc-300 text-[10px] hover:bg-zinc-800 transition-colors"
                      >
                        Click to Wake
                      </button>
                    </div>
                  ) : !browser.native ? (
                    <div className="absolute inset-0 flex items-center justify-center bg-canvas p-8 text-center text-fg-muted z-20">
                      Open Custos Desktop to experience native mobile webview.
                    </div>
                  ) : browser.error ? (
                    <div
                      role="alert"
                      className="absolute inset-0 flex flex-col items-center justify-center gap-4 bg-canvas p-8 text-center z-20"
                    >
                      <p className="text-red-400 font-medium">{browser.error}</p>
                      <button
                        className="rounded border border-border-default bg-surface-2 px-4 py-2 hover:bg-surface-3 transition-colors"
                        onClick={browser.retry}
                      >
                        Retry Connection
                      </button>
                    </div>
                  ) : home ? (
                    <div className="absolute inset-0 bg-canvas flex flex-col items-center justify-center gap-6 p-8 text-center z-20">
                      <div className="rounded-2xl bg-[var(--color-accent-subtle)] p-5 text-[var(--color-accent)]">
                        <Globe size={36} />
                      </div>
                      <div>
                        <h2 className="text-base font-semibold text-editor-fg">Mobile Simulator Ready</h2>
                        <p className="mt-1.5 text-xs text-fg-muted max-w-[240px]">
                          Enter an address or click a local server below to start testing.
                        </p>
                      </div>
                      <div className="flex flex-wrap justify-center gap-2">
                        <button
                          className="rounded-lg border border-border-default bg-surface-1 px-3 py-1.5 hover:bg-surface-2 text-xs font-medium transition-colors"
                          onClick={() => navigate('http://localhost:5173')}
                        >
                          Dev :5173
                        </button>
                        <button
                          className="rounded-lg border border-border-default bg-surface-1 px-3 py-1.5 hover:bg-surface-2 text-xs font-medium transition-colors"
                          onClick={() => navigate('http://localhost:3000')}
                        >
                          Dev :3000
                        </button>
                        <button
                          className="rounded-lg border border-border-default bg-surface-1 px-3 py-1.5 hover:bg-surface-2 text-xs font-medium transition-colors"
                          onClick={() => navigate('https://duckduckgo.com')}
                        >
                          DuckDuckGo
                        </button>
                      </div>
                    </div>
                  ) : !browser.ready ? (
                    <div
                      role="status"
                      aria-label="Preparing phone preview"
                      className="absolute inset-0 bg-canvas p-8 space-y-4 motion-safe:animate-pulse z-20"
                    >
                      <span className="sr-only">Preparing phone preview...</span>
                      <div className="h-6 w-3/4 rounded bg-surface-2" />
                      <div className="h-44 rounded bg-surface-1" />
                      <div className="h-3 rounded bg-surface-2" />
                      <div className="h-3 w-2/3 rounded bg-surface-2" />
                    </div>
                  ) : null}

                  {/* Touch Pointer Simulation Overlay (Cursor Dot) */}
                  {touchMode && touchCursor && powered && (
                    <div
                      className="pointer-events-none absolute z-50 -translate-x-1/2 -translate-y-1/2 transition-transform duration-75"
                      style={{ left: touchCursor.x, top: touchCursor.y }}
                    >
                      <div
                        className={`size-8 rounded-full border border-white/60 bg-white/20 backdrop-blur-xs shadow-lg transition-transform ${
                          touchCursor.active ? 'scale-90 bg-white/40 ring-2 ring-[var(--color-accent)]' : ''
                        }`}
                      >
                        <div className="absolute inset-0 m-auto size-1.5 rounded-full bg-white/80" />
                      </div>
                    </div>
                  )}
                </div>

                {/* Bottom Navigation & iOS Home Indicator Bar */}
                <div className="h-10 shrink-0 flex flex-col items-center justify-between bg-zinc-950 border-t border-zinc-800/60 text-zinc-400 z-40 select-none pb-1.5">
                  <div className="w-full flex items-center justify-around pt-1">
                    <button
                      className="p-1 hover:text-white disabled:opacity-30 transition-colors"
                      aria-label="Phone back"
                      disabled={!browser.ready}
                      onClick={() => void browser.command('back')}
                    >
                      <ArrowLeft size={15} />
                    </button>
                    <button
                      className="p-1 hover:text-white disabled:opacity-30 transition-colors"
                      aria-label="Phone home screen"
                      disabled={!powered}
                      onClick={() => navigate('about:blank')}
                    >
                      <Home size={15} />
                    </button>
                    <button
                      className="p-1 hover:text-white disabled:opacity-30 transition-colors"
                      aria-label="Phone forward"
                      disabled={!browser.ready}
                      onClick={() => void browser.command('forward')}
                    >
                      <ArrowRight size={15} />
                    </button>
                  </div>

                  {/* Authentic iOS Home Indicator Bar (Clickable to go Home) */}
                  <button
                    type="button"
                    title="Home Swipe Bar"
                    onClick={() => navigate('about:blank')}
                    className="w-32 h-1 rounded-full bg-white/40 hover:bg-white/80 transition-all cursor-pointer"
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Footer Metrics & Responsive Scale Slider */}
      <div className="shrink-0 flex items-center justify-between gap-3 border-t border-border-muted bg-surface-1 px-3 py-2 text-[10px] text-fg-muted">
        <div className="flex items-center gap-2">
          <span className="font-mono text-editor-fg">
            {width} × {pageHeight} CSS px
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
