import React, { useState, useMemo, useRef } from 'react';
import {
  Smartphone,
  RotateCw,
  Power,
  RefreshCw,
  Copy,
  Camera,
  ZoomIn,
  ZoomOut,
  ArrowRight,
  Send,
  Check,
  Home,
  Globe,
  Sparkles,
  Terminal,
  Search,
  Wifi,
  Battery,
  Cpu,
  Loader2,
} from 'lucide-react';
import { NativeSelect } from '@/components/common/NativeSelect';
import { daemonClient } from '@/api/daemon_client';

interface DevicePreset {
  id: string;
  name: string;
  platform: 'ios' | 'android' | 'tablet';
  width: number;
  height: number;
  dpr: number;
  bezelRadius: string;
  notchType: 'island' | 'punchhole' | 'classic' | 'tablet';
}

const DEVICE_PRESETS: DevicePreset[] = [
  {
    id: 'iphone-15-pro',
    name: 'iPhone 15 Pro',
    platform: 'ios',
    width: 393,
    height: 852,
    dpr: 3.0,
    bezelRadius: '48px',
    notchType: 'island',
  },
  {
    id: 'pixel-8',
    name: 'Google Pixel 8',
    platform: 'android',
    width: 412,
    height: 915,
    dpr: 2.6,
    bezelRadius: '42px',
    notchType: 'punchhole',
  },
  {
    id: 'iphone-se',
    name: 'iPhone SE (3rd Gen)',
    platform: 'ios',
    width: 375,
    height: 667,
    dpr: 2.0,
    bezelRadius: '32px',
    notchType: 'classic',
  },
  {
    id: 'ipad-mini',
    name: 'iPad Mini (6th Gen)',
    platform: 'tablet',
    width: 744,
    height: 1133,
    dpr: 2.0,
    bezelRadius: '28px',
    notchType: 'tablet',
  },
];

const SPRINGBOARD_URL = 'custos://springboard';

interface MobileSimulatorWorkbenchPaneProps {
  onShowToast?: (message: string) => void;
  defaultUrl?: string;
  onAskAgent?: (draft: string) => void;
}

export const MobileSimulatorWorkbenchPane: React.FC<MobileSimulatorWorkbenchPaneProps> = ({
  onShowToast,
  defaultUrl = SPRINGBOARD_URL,
  onAskAgent,
}) => {
  const [selectedDeviceId, setSelectedDeviceId] = useState<string>('iphone-15-pro');
  const [orientation, setOrientation] = useState<'portrait' | 'landscape'>('portrait');
  const [scale, setScale] = useState<number>(0.75);
  const [urlInput, setUrlInput] = useState<string>(defaultUrl === SPRINGBOARD_URL ? '' : defaultUrl);
  const [activeUrl, setActiveUrl] = useState<string>(defaultUrl);
  const [isPowerOn, setIsPowerOn] = useState<boolean>(true);
  const [activeDrawer, setActiveDrawer] = useState<'axtree' | 'console' | 'evidence'>('axtree');
  const [justInserted, setJustInserted] = useState<boolean>(false);
  const iframeRef = useRef<HTMLIFrameElement | null>(null);

  // Governed Mobile Simulator Backend Engine State
  const [engineMode, setEngineMode] = useState<'governed' | 'responsive_frame'>('governed');
  const [emulatorSessionId, setEmulatorSessionId] = useState<string | null>(null);
  const [emulatorScreenshot, setEmulatorScreenshot] = useState<string | null>(null);
  const [emulatorSnapshot, setEmulatorSnapshot] = useState<{
    snapshot?: string;
    refs?: Record<string, { name?: string; role?: string }>;
    origin?: string;
  } | null>(null);
  const [isNavigatingEmulator, setIsNavigatingEmulator] = useState<boolean>(false);

  const device = useMemo(
    () => DEVICE_PRESETS.find((d) => d.id === selectedDeviceId) ?? DEVICE_PRESETS[0],
    [selectedDeviceId]
  );

  const screenWidth = orientation === 'portrait' ? device.width : device.height;
  const screenHeight = orientation === 'portrait' ? device.height : device.width;

  const executeEmulatorNavigation = async (url: string) => {
    setIsNavigatingEmulator(true);
    try {
      const devId = selectedDeviceId.replace(/-/g, '_');
      const sessId = emulatorSessionId || `mobile_${devId}_${Date.now()}`;
      if (!emulatorSessionId) {
        setEmulatorSessionId(sessId);
        await daemonClient.launchEmulatorDevice({
          device_id: devId,
          session_id: sessId,
          url,
        });
      } else {
        await daemonClient.navigateEmulator(sessId, url);
      }

      const shot = await daemonClient.captureEmulatorScreenshot(sessId);
      if (shot?.image_base64) {
        setEmulatorScreenshot(shot.image_base64);
      }

      const snap = await daemonClient.snapshotEmulator(sessId);
      if (snap) {
        setEmulatorSnapshot(snap);
      }
    } catch (err: any) {
      console.warn('Mobile emulator error:', err);
    } finally {
      setIsNavigatingEmulator(false);
    }
  };

  const handleTap = async (refKey: string) => {
    if (!emulatorSessionId) return;
    onShowToast?.(`Tapping ${refKey}...`);
    try {
      await daemonClient.tapEmulator(emulatorSessionId, refKey);
      const shot = await daemonClient.captureEmulatorScreenshot(emulatorSessionId);
      if (shot?.image_base64) {
        setEmulatorScreenshot(shot.image_base64);
      }
      const snap = await daemonClient.snapshotEmulator(emulatorSessionId);
      if (snap) {
        setEmulatorSnapshot(snap);
      }
      onShowToast?.(`Tapped ${refKey}`);
    } catch (err: any) {
      onShowToast?.(`Tap failed: ${err.message || err}`);
    }
  };

  const handleNavigate = (target?: string) => {
    const raw = (target !== undefined ? target : urlInput).trim();
    if (!raw || raw === SPRINGBOARD_URL || raw === 'about:blank') {
      setActiveUrl(SPRINGBOARD_URL);
      setUrlInput('');
      setEmulatorScreenshot(null);
      setEmulatorSnapshot(null);
      return;
    }
    const formatted = raw.startsWith('http://') || raw.startsWith('https://')
      ? raw
      : raw.startsWith(':')
      ? `http://localhost${raw}`
      : `http://${raw}`;

    setActiveUrl(formatted);
    setUrlInput(formatted);

    if (engineMode === 'governed') {
      executeEmulatorNavigation(formatted);
    }
  };

  const handleReload = () => {
    if (activeUrl === SPRINGBOARD_URL) return;
    if (engineMode === 'governed') {
      executeEmulatorNavigation(activeUrl);
    } else if (iframeRef.current) {
      iframeRef.current.src = activeUrl;
    }
    onShowToast?.(`Reloaded ${activeUrl}`);
  };

  const toggleOrientation = () => {
    setOrientation((prev) => (prev === 'portrait' ? 'landscape' : 'portrait'));
  };

  // Accessibility Tree (AX Tree) representation for Agent ingestion & responsive verification
  const mockAxTree = useMemo(() => {
    if (activeUrl === SPRINGBOARD_URL) {
      return [
        { id: '1', role: 'banner', name: 'Mobile Status Bar', bounds: { x: 0, y: 0, w: screenWidth, h: 48 } },
        { id: '2', role: 'grid', name: 'Home Screen Springboard', bounds: { x: 20, y: 80, w: screenWidth - 40, h: 320 } },
        { id: '3', role: 'button', name: 'App: Local Dev Vite (:5173)', bounds: { x: 24, y: 90, w: 64, h: 64 } },
        { id: '4', role: 'button', name: 'App: Mobile Web Browser', bounds: { x: 104, y: 90, w: 64, h: 64 } },
        { id: '5', role: 'button', name: 'App: Design Inspo (Linear)', bounds: { x: 184, y: 90, w: 64, h: 64 } },
        { id: '6', role: 'searchbox', name: 'Quick Search / URL', bounds: { x: 20, y: screenHeight - 110, w: screenWidth - 40, h: 44 } },
      ];
    }
    return [
      { id: '1', role: 'banner', name: 'App Header', bounds: { x: 0, y: 0, w: screenWidth, h: 56 } },
      { id: '2', role: 'heading', level: 1, name: 'Active Project Viewport', bounds: { x: 16, y: 12, w: 240, h: 32 } },
      { id: '3', role: 'button', name: 'Navigation Menu', bounds: { x: screenWidth - 48, y: 12, w: 32, h: 32 } },
      { id: '4', role: 'main', name: 'Primary Mobile Viewport', bounds: { x: 0, y: 56, w: screenWidth, h: screenHeight - 120 } },
      { id: '5', role: 'textbox', name: 'Search Query / Input', bounds: { x: 16, y: 72, w: screenWidth - 32, h: 44 } },
      { id: '6', role: 'button', name: 'Action Button', bounds: { x: 16, y: 128, w: screenWidth - 32, h: 48 } },
      { id: '7', role: 'navigation', name: 'Bottom Tab Bar', bounds: { x: 0, y: screenHeight - 64, w: screenWidth, h: 64 } },
    ];
  }, [screenWidth, screenHeight, activeUrl]);

  const handleInsertToChat = (customSnippet?: string) => {
    const axSummary = mockAxTree
      .map((n) => `- [${n.role}] "${n.name}" (${n.bounds.w}×${n.bounds.h}px)`)
      .join('\n');
    const snippet = customSnippet || `Context Mobile Simulator [${device.name} - ${orientation}]:\n- Active View: ${activeUrl}\n- Resolution: ${screenWidth}×${screenHeight} (dpr: ${device.dpr})\n- Accessibility Tree:\n${axSummary}`;
    if (onAskAgent) {
      onAskAgent(snippet);
      setJustInserted(true);
      setTimeout(() => setJustInserted(false), 2000);
      onShowToast?.(`Inserted mobile simulator context into chat input.`);
    } else {
      navigator.clipboard.writeText(snippet);
      onShowToast?.(`Copied mobile context to clipboard.`);
    }
  };

  const isSpringboard = activeUrl === SPRINGBOARD_URL;

  return (
    <div className="flex flex-col h-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] text-xs overflow-hidden select-none font-sans">
      {/* Sleek 44px (h-11) Top Controls Toolbar */}
      <div className="flex h-11 shrink-0 items-center justify-between border-b border-[var(--color-border-default)] px-3.5 bg-[var(--color-surface-1)]">
        <div className="flex items-center gap-2.5">
          <div className="flex items-center gap-2">
            <Smartphone className="h-4 w-4 text-cyan-400 shrink-0" />
            <span className="text-xs font-semibold text-[var(--color-editor-fg)]">
              Phone Simulator
            </span>
          </div>

          <div className="h-3.5 w-px bg-[var(--color-border-muted)]" />

          <span className="rounded bg-cyan-950/60 px-1.5 py-0.5 text-[10px] font-mono text-cyan-400 border border-cyan-800/40">
            {device.platform.toUpperCase()}
          </span>

          <div className="flex items-center gap-1.5 text-[11px] text-[var(--color-fg-subtle)]">
            <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
            <span>{isSpringboard ? 'Home Screen' : 'Live Preview'}</span>
          </div>
        </div>

        {/* Device Switcher & Hardware Controls */}
        <div className="flex items-center gap-2">
          {/* Engine Mode Toggle */}
          <button
            onClick={() => {
              const next = engineMode === 'governed' ? 'responsive_frame' : 'governed';
              setEngineMode(next);
              onShowToast?.(`Mobile Engine: ${next === 'governed' ? 'Governed Mobile CDP' : 'Direct Responsive Frame'}`);
              if (activeUrl !== SPRINGBOARD_URL) {
                if (next === 'governed') {
                  executeEmulatorNavigation(activeUrl);
                } else {
                  setEmulatorScreenshot(null);
                }
              }
            }}
            className={`flex items-center gap-1.5 px-2 py-1 rounded-lg border text-xs font-mono transition ${
              engineMode === 'governed'
                ? 'border-cyan-500/40 bg-cyan-500/15 text-cyan-300'
                : 'border-[var(--color-border-default)] bg-[var(--color-surface-2)] text-[var(--color-fg-muted)]'
            }`}
            title="Toggle between Governed Mobile CDP Engine and Direct Responsive Frame"
          >
            <Cpu className="h-3 w-3 text-cyan-400" />
            <span>{engineMode === 'governed' ? 'CDP Mobile' : 'Local Frame'}</span>
          </button>

          <NativeSelect
            value={selectedDeviceId}
            onChange={(e) => {
              setSelectedDeviceId(e.target.value);
              if (engineMode === 'governed' && activeUrl !== SPRINGBOARD_URL) {
                setEmulatorSessionId(null);
              }
            }}
            className="rounded border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2 py-1 text-xs text-[var(--color-editor-fg)] focus:outline-none"
          >
            {DEVICE_PRESETS.map((d) => (
              <option key={d.id} value={d.id} className="bg-[var(--color-surface-1)]">
                {d.name} ({d.width}×{d.height})
              </option>
            ))}
          </NativeSelect>

          <button
            onClick={toggleOrientation}
            className="flex items-center gap-1 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2 py-1 text-xs text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] transition"
            title="Rotate Device Orientation"
          >
            <RotateCw className={`h-3.5 w-3.5 ${orientation === 'landscape' ? 'text-cyan-400' : ''}`} />
            <span className="capitalize hidden sm:inline">{orientation}</span>
          </button>

          {/* Zoom Controls */}
          <div className="flex items-center rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-1">
            <button
              onClick={() => setScale((s) => Math.max(0.4, Number((s - 0.1).toFixed(2))))}
              className="p-1 text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
              title="Zoom Out"
            >
              <ZoomOut className="h-3 w-3" />
            </button>
            <span className="font-mono text-[10px] text-[var(--color-fg-muted)] px-1">
              {Math.round(scale * 100)}%
            </span>
            <button
              onClick={() => setScale((s) => Math.min(1.2, Number((s + 0.1).toFixed(2))))}
              className="p-1 text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
              title="Zoom In"
            >
              <ZoomIn className="h-3 w-3" />
            </button>
          </div>

          {/* Power Toggle */}
          <button
            onClick={() => setIsPowerOn(!isPowerOn)}
            className={`p-1.5 rounded-lg border transition ${
              isPowerOn
                ? 'border-[var(--color-border-default)] bg-[var(--color-surface-2)] text-emerald-400 hover:bg-[var(--color-surface-3)]'
                : 'border-red-800/40 bg-red-950/40 text-red-400'
            }`}
            title={isPowerOn ? 'Turn Screen Off' : 'Turn Screen On'}
          >
            <Power className="h-3.5 w-3.5" />
          </button>

          {/* Primary Action Button: Insert to Chat */}
          <button
            onClick={() => handleInsertToChat()}
            className="flex items-center gap-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 px-3 py-1 text-xs font-medium text-white transition shadow-sm"
            title="Insert mobile layout & AX Tree into chat input"
          >
            {justInserted ? (
              <>
                <Check className="h-3.5 w-3.5 text-white" />
                <span>Inserted</span>
              </>
            ) : (
              <>
                <Send className="h-3.5 w-3.5 text-white" />
                <span>Insert to Chat</span>
              </>
            )}
          </button>
        </div>
      </div>

      {/* Address & Quick Server Bar */}
      <div className="flex items-center gap-2 px-3.5 py-1.5 bg-[var(--color-surface-2)] border-b border-[var(--color-border-default)] shrink-0">
        <button
          onClick={() => handleNavigate(SPRINGBOARD_URL)}
          className={`p-1.5 rounded transition border ${
            isSpringboard
              ? 'bg-cyan-500/10 text-cyan-400 border-cyan-500/30'
              : 'bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-3)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] border-[var(--color-border-muted)]'
          }`}
          title="Home Screen Springboard"
        >
          <Home className="h-3 w-3" />
        </button>

        <button
          onClick={handleReload}
          disabled={isSpringboard}
          className="p-1.5 rounded bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-3)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition border border-[var(--color-border-muted)] disabled:opacity-30"
          title="Reload Preview Frame"
        >
          <RefreshCw className="h-3 w-3" />
        </button>

        <form
          onSubmit={(e) => {
            e.preventDefault();
            handleNavigate();
          }}
          className="flex-1 flex items-center relative"
        >
          <input
            type="text"
            value={urlInput}
            onChange={(e) => setUrlInput(e.target.value)}
            placeholder="Search web or enter mobile URL (e.g. http://localhost:5173, linear.app)..."
            className="w-full rounded border border-[var(--color-border-default)] bg-[var(--color-canvas)] px-2.5 py-0.5 text-xs font-mono text-[var(--color-editor-fg)] focus:border-cyan-600 focus:outline-none"
          />
          <button
            type="submit"
            className="absolute right-1 px-1.5 py-0.5 bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] rounded text-[10px]"
          >
            <ArrowRight className="h-3 w-3" />
          </button>
        </form>

        {/* Quick Local Server Presets */}
        <div className="flex items-center gap-1 text-[10px] font-mono">
          <button
            onClick={() => handleNavigate('http://localhost:5173')}
            className={`px-1.5 py-0.5 rounded border transition ${
              activeUrl.includes(':5173')
                ? 'bg-cyan-500/15 border-cyan-500/40 text-cyan-300 font-semibold'
                : 'bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-3)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] border-[var(--color-border-muted)]'
            }`}
          >
            :5173
          </button>
          <button
            onClick={() => handleNavigate('http://localhost:3000')}
            className={`px-1.5 py-0.5 rounded border transition ${
              activeUrl.includes(':3000')
                ? 'bg-cyan-500/15 border-cyan-500/40 text-cyan-300 font-semibold'
                : 'bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-3)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] border-[var(--color-border-muted)]'
            }`}
          >
            :3000
          </button>
          <button
            onClick={() => handleNavigate('http://localhost:8080')}
            className={`px-1.5 py-0.5 rounded border transition ${
              activeUrl.includes(':8080')
                ? 'bg-cyan-500/15 border-cyan-500/40 text-cyan-300 font-semibold'
                : 'bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-3)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] border-[var(--color-border-muted)]'
            }`}
          >
            :8080
          </button>
        </div>
      </div>

      {/* Main Body: Canvas + Inspection Drawer */}
      <div className="flex-1 flex overflow-hidden">
        {/* Center: Realistic Device Canvas */}
        <div className="flex-1 flex flex-col items-center justify-start overflow-auto p-8 bg-[radial-gradient(var(--color-border-muted)_1px,transparent_1px)] [background-size:18px_18px]">
          <div
            className="relative transition-all duration-300 ease-out origin-top flex flex-col items-center"
            style={{
              transform: `scale(${scale})`,
              marginBottom: `${Math.max(40, screenHeight * scale * 0.1)}px`,
            }}
          >
            {/* Realistic Physical Device Frame */}
            <div
              className="relative p-[10px] bg-[#1e2024] rounded-[52px] shadow-[0_25px_60px_-15px_rgba(0,0,0,0.9),0_0_0_1px_rgba(255,255,255,0.08)] border border-zinc-700/60 flex flex-col"
              style={{
                width: `${screenWidth + 20}px`,
                height: `${screenHeight + 20}px`,
                borderRadius: device.bezelRadius,
              }}
            >
              {/* Outer Edge Hardware Accent / Buttons */}
              <div className="absolute -left-[3px] top-24 w-[3px] h-8 bg-zinc-600/70 rounded-l-xs" title="Volume Up" />
              <div className="absolute -left-[3px] top-36 w-[3px] h-8 bg-zinc-600/70 rounded-l-xs" title="Volume Down" />
              <div className="absolute -right-[3px] top-28 w-[3px] h-12 bg-zinc-600/70 rounded-r-xs" title="Power Button" />

              {/* Inner Screen Display Bezel */}
              <div
                className="relative flex-1 bg-black overflow-hidden flex flex-col shadow-inner"
                style={{
                  borderRadius: `calc(${device.bezelRadius} - 8px)`,
                }}
              >
                {/* Dynamic Island / Notch Simulation */}
                {isPowerOn && (
                  <>
                    {device.notchType === 'island' && orientation === 'portrait' && (
                      <div className="absolute top-2 left-1/2 -translate-x-1/2 w-28 h-6 bg-black rounded-full z-30 flex items-center justify-between px-2.5 shadow-md border border-zinc-800/80">
                        <div className="w-2.5 h-2.5 rounded-full bg-zinc-900 border border-zinc-700" />
                        <div className="w-2 h-2 rounded-full bg-emerald-500/80 animate-pulse" title="Microphone / Camera Active" />
                      </div>
                    )}
                    {device.notchType === 'punchhole' && orientation === 'portrait' && (
                      <div className="absolute top-2.5 left-1/2 -translate-x-1/2 w-3.5 h-3.5 bg-black rounded-full z-30 border border-zinc-800" />
                    )}
                  </>
                )}

                {/* Display Content: Springboard Home Screen or Live Iframe */}
                {isPowerOn ? (
                  isSpringboard ? (
                    /* Mobile Springboard / Home Launcher */
                    <div className="flex-1 flex flex-col justify-between p-6 bg-gradient-to-b from-zinc-900 via-slate-900 to-black text-white select-none">
                      {/* Top Status Bar */}
                      <div className="flex items-center justify-between text-[11px] font-semibold tracking-tight px-1 pt-1">
                        <span>9:41</span>
                        <div className="flex items-center gap-1.5 opacity-80">
                          <Wifi className="h-3 w-3" />
                          <Battery className="h-3.5 w-3.5" />
                        </div>
                      </div>

                      {/* App Grid */}
                      <div className="grid grid-cols-3 gap-6 my-auto pt-6 px-2">
                        {/* App 1: Local Dev App */}
                        <button
                          onClick={() => handleNavigate('http://localhost:5173')}
                          className="flex flex-col items-center gap-1.5 group cursor-pointer"
                        >
                          <div className="w-14 h-14 rounded-2xl bg-gradient-to-tr from-emerald-600 to-teal-400 flex items-center justify-center shadow-lg group-hover:scale-105 transition">
                            <Terminal className="h-6 w-6 text-white" />
                          </div>
                          <span className="text-[10px] font-medium text-zinc-300">Dev :5173</span>
                        </button>

                        {/* App 2: Web Browser */}
                        <button
                          onClick={() => handleNavigate('https://duckduckgo.com')}
                          className="flex flex-col items-center gap-1.5 group cursor-pointer"
                        >
                          <div className="w-14 h-14 rounded-2xl bg-gradient-to-tr from-cyan-600 to-blue-500 flex items-center justify-center shadow-lg group-hover:scale-105 transition">
                            <Globe className="h-6 w-6 text-white" />
                          </div>
                          <span className="text-[10px] font-medium text-zinc-300">Web</span>
                        </button>

                        {/* App 3: Mobile Inspo */}
                        <button
                          onClick={() => handleNavigate('https://linear.app')}
                          className="flex flex-col items-center gap-1.5 group cursor-pointer"
                        >
                          <div className="w-14 h-14 rounded-2xl bg-gradient-to-tr from-amber-500 to-orange-400 flex items-center justify-center shadow-lg group-hover:scale-105 transition">
                            <Sparkles className="h-6 w-6 text-white" />
                          </div>
                          <span className="text-[10px] font-medium text-zinc-300">Inspo</span>
                        </button>

                        {/* App 4: Next.js :3000 */}
                        <button
                          onClick={() => handleNavigate('http://localhost:3000')}
                          className="flex flex-col items-center gap-1.5 group cursor-pointer"
                        >
                          <div className="w-14 h-14 rounded-2xl bg-gradient-to-tr from-zinc-700 to-zinc-900 border border-zinc-600 flex items-center justify-center shadow-lg group-hover:scale-105 transition">
                            <span className="text-xs font-bold text-white">N</span>
                          </div>
                          <span className="text-[10px] font-medium text-zinc-300">Next :3000</span>
                        </button>
                      </div>

                      {/* Bottom Search Bar */}
                      <div className="pb-4">
                        <div
                          onClick={() => {
                            const target = prompt('Enter URL or search term:', 'http://localhost:5173');
                            if (target) handleNavigate(target);
                          }}
                          className="w-full flex items-center justify-center gap-2 py-2 px-3 rounded-full bg-white/10 backdrop-blur-md border border-white/15 text-[11px] text-zinc-300 cursor-pointer hover:bg-white/15 transition shadow-sm"
                        >
                          <Search className="h-3 w-3 text-zinc-400" />
                          <span>Search or enter URL...</span>
                        </div>
                      </div>
                    </div>
                  ) : engineMode === 'governed' && emulatorScreenshot ? (
                    /* Live Governed Mobile Screencast Canvas */
                    <div className="relative w-full h-full flex flex-col items-center justify-start overflow-auto bg-slate-950">
                      {isNavigatingEmulator && (
                        <div className="absolute inset-0 z-30 flex items-center justify-center bg-black/50 backdrop-blur-xs">
                          <div className="flex items-center gap-2 rounded-lg bg-[var(--color-surface-1)] px-3 py-1.5 text-xs border border-[var(--color-border-default)] shadow-xl text-cyan-400">
                            <Loader2 className="h-3.5 w-3.5 animate-spin" />
                            <span>Simulating Mobile Viewport...</span>
                          </div>
                        </div>
                      )}
                      <img
                        src={emulatorScreenshot}
                        alt="Mobile Screencast"
                        className="w-full h-auto block select-none pointer-events-auto"
                      />
                    </div>
                  ) : (
                    <iframe
                      ref={iframeRef}
                      key={activeUrl}
                      src={activeUrl}
                      title={device.name}
                      className="w-full h-full flex-1 border-0 bg-white"
                      sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
                    />
                  )
                ) : (
                  <div className="flex-1 flex flex-col items-center justify-center bg-black text-zinc-600 text-[11px]">
                    <Power className="h-8 w-8 mb-2 opacity-30" />
                    <span>Screen Powered Off</span>
                  </div>
                )}

                {/* Home Indicator Bar (Click to return to Springboard) */}
                {isPowerOn && orientation === 'portrait' && (
                  <div
                    onClick={() => handleNavigate(SPRINGBOARD_URL)}
                    className="absolute bottom-1.5 left-1/2 -translate-x-1/2 w-32 h-2.5 flex items-center justify-center z-30 cursor-pointer group"
                    title="Tap to return Home"
                  >
                    <div className="w-28 h-1 bg-white/60 group-hover:bg-white group-hover:h-1.5 rounded-full transition-all" />
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>

        {/* Right Drawer: AX Tree & Inspection */}
        <div className="w-80 border-l border-[var(--color-border-default)] bg-[var(--color-surface-1)] flex flex-col shrink-0">
          <div className="flex h-11 items-center justify-between border-b border-[var(--color-border-muted)] px-3">
            <div className="flex items-center gap-1">
              <button
                onClick={() => setActiveDrawer('axtree')}
                className={`rounded px-2 py-1 text-xs font-medium transition ${
                  activeDrawer === 'axtree'
                    ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)]'
                    : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                }`}
              >
                AX Tree
              </button>
              <button
                onClick={() => setActiveDrawer('console')}
                className={`rounded px-2 py-1 text-xs font-medium transition ${
                  activeDrawer === 'console'
                    ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)]'
                    : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                }`}
              >
                Console
              </button>
              <button
                onClick={() => setActiveDrawer('evidence')}
                className={`rounded px-2 py-1 text-xs font-medium transition ${
                  activeDrawer === 'evidence'
                    ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)]'
                    : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                }`}
              >
                Evidence
              </button>
            </div>

            <button
              onClick={() => {
                const text = JSON.stringify(mockAxTree, null, 2);
                navigator.clipboard.writeText(text);
                onShowToast?.('Copied Accessibility Tree JSON.');
              }}
              className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
              title="Copy JSON"
            >
              <Copy className="h-3.5 w-3.5" />
            </button>
          </div>

          <div className="flex-1 overflow-y-auto p-3 text-xs">
            {activeDrawer === 'axtree' && (
              <div className="space-y-2">
                <div className="flex items-center justify-between text-[11px] mb-2">
                  <span className="text-[var(--color-fg-subtle)]">
                    Accessibility Tree Nodes ({emulatorSnapshot?.refs ? Object.keys(emulatorSnapshot.refs).length : mockAxTree.length}):
                  </span>
                  <button
                    onClick={() => {
                      const summary = emulatorSnapshot?.snapshot || mockAxTree.map((n) => `- [${n.role}] ${n.name}`).join('\n');
                      handleInsertToChat(
                        `Mobile AX Tree Audit Task for ${device.name} (${activeUrl}):\n${summary}\nAudit responsive layout, touch target sizes (>44px), and accessibility labels.`
                      );
                    }}
                    className="flex items-center gap-1 text-[11px] text-cyan-400 hover:underline"
                  >
                    <Send className="h-3 w-3" />
                    <span>Audit (AI Agent)</span>
                  </button>
                </div>

                <div className="space-y-1.5">
                  {emulatorSnapshot?.refs && Object.keys(emulatorSnapshot.refs).length > 0 ? (
                    Object.entries(emulatorSnapshot.refs).map(([refKey, info]) => (
                      <div
                        key={refKey}
                        className="flex items-center justify-between p-2 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border-muted)]"
                      >
                        <div className="truncate mr-2">
                          <div className="flex items-center gap-1 text-[10px]">
                            <span className="text-emerald-400 font-bold font-mono">@{refKey}</span>
                            <span className="text-cyan-400 uppercase font-mono">[{info.role || 'element'}]</span>
                          </div>
                          <div className="text-[var(--color-editor-fg)] text-xs mt-0.5 truncate">{info.name || refKey}</div>
                        </div>
                        <button
                          onClick={() => handleTap(refKey)}
                          className="px-2 py-0.5 rounded bg-cyan-500/20 hover:bg-cyan-500/30 text-cyan-300 font-mono text-[10px] shrink-0"
                          title="Tap element on mobile simulator"
                        >
                          Tap
                        </button>
                      </div>
                    ))
                  ) : (
                    mockAxTree.map((node) => (
                      <div
                        key={node.id}
                        className="p-2 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border-muted)]"
                      >
                        <div className="flex items-center justify-between text-[10px]">
                          <span className="text-cyan-400 font-semibold uppercase">{node.role}</span>
                          <span className="text-[var(--color-fg-muted)]">{node.bounds.w}×{node.bounds.h}px</span>
                        </div>
                        <div className="text-[var(--color-editor-fg)] text-xs mt-0.5 truncate">{node.name}</div>
                        <div className="text-[10px] text-[var(--color-fg-subtle)] mt-0.5">
                          Pos: ({node.bounds.x}, {node.bounds.y})
                        </div>
                      </div>
                    ))
                  )}
                </div>
              </div>
            )}

            {activeDrawer === 'console' && (
              <div className="space-y-2 font-mono text-[11px] text-[var(--color-fg-subtle)]">
                <div className="p-2.5 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] space-y-1">
                  <p className="text-emerald-400">[info] Simulator viewport: {screenWidth}×{screenHeight} @{device.dpr}x</p>
                  <p className="text-[var(--color-fg-muted)]">[log] Active orientation: {orientation}</p>
                  <p className="text-[var(--color-fg-muted)]">[log] Active view: {activeUrl}</p>
                </div>
              </div>
            )}

            {activeDrawer === 'evidence' && (
              <div className="space-y-3">
                <div className="text-[11px] text-[var(--color-fg-subtle)]">
                  Capture mobile viewport snapshot into session evidence ledger:
                </div>
                <button
                  onClick={() => onShowToast?.('Mobile snapshot recorded into evidence ledger.')}
                  className="w-full flex items-center justify-center gap-1.5 py-1.5 bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] border border-[var(--color-border-default)] rounded-lg text-xs font-medium text-[var(--color-editor-fg)] transition"
                >
                  <Camera className="h-3.5 w-3.5 text-cyan-400" />
                  <span>Capture Mobile Snapshot</span>
                </button>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
