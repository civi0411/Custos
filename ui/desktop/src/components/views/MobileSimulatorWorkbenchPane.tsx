import React, { useState, useMemo, useRef } from 'react';
import {
  Smartphone,
  RotateCw,
  Power,
  RefreshCw,
  Copy,
  Check,
  Terminal,
  ShieldCheck,
  Camera,
  ZoomIn,
  ZoomOut,
  ArrowRight,
  Sparkles,
} from 'lucide-react';

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

interface MobileSimulatorWorkbenchPaneProps {
  onShowToast?: (message: string) => void;
  defaultUrl?: string;
}

export const MobileSimulatorWorkbenchPane: React.FC<MobileSimulatorWorkbenchPaneProps> = ({
  onShowToast,
  defaultUrl = 'http://localhost:5173',
}) => {
  const [selectedDeviceId, setSelectedDeviceId] = useState<string>('iphone-15-pro');
  const [orientation, setOrientation] = useState<'portrait' | 'landscape'>('portrait');
  const [scale, setScale] = useState<number>(0.75);
  const [urlInput, setUrlInput] = useState<string>(defaultUrl);
  const [activeUrl, setActiveUrl] = useState<string>(defaultUrl);
  const [isPowerOn, setIsPowerOn] = useState<boolean>(true);
  const [activeDrawer, setActiveDrawer] = useState<'axtree' | 'console' | 'evidence'>('axtree');
  const [copiedAx, setCopiedAx] = useState<boolean>(false);
  const iframeRef = useRef<HTMLIFrameElement | null>(null);

  const device = useMemo(
    () => DEVICE_PRESETS.find((d) => d.id === selectedDeviceId) ?? DEVICE_PRESETS[0],
    [selectedDeviceId]
  );

  const screenWidth = orientation === 'portrait' ? device.width : device.height;
  const screenHeight = orientation === 'portrait' ? device.height : device.width;

  const handleNavigate = (e?: React.FormEvent) => {
    e?.preventDefault();
    const trimmed = urlInput.trim();
    if (!trimmed) return;
    setActiveUrl(trimmed.startsWith('http') ? trimmed : `http://${trimmed}`);
    onShowToast?.(`Simulator loaded: ${trimmed}`);
  };

  const handleReload = () => {
    if (iframeRef.current) {
      iframeRef.current.src = activeUrl;
      onShowToast?.('Simulator reloaded.');
    }
  };

  const toggleOrientation = () => {
    setOrientation((prev) => (prev === 'portrait' ? 'landscape' : 'portrait'));
    onShowToast?.(`Orientation: ${orientation === 'portrait' ? 'Landscape' : 'Portrait'}`);
  };

  // Mock Accessibility Tree (AX Tree) representation for Coding Agent ingestion
  const mockAxTree = useMemo(() => {
    return [
      { id: '1', role: 'banner', name: 'App Header', bounds: { x: 0, y: 0, w: screenWidth, h: 56 } },
      { id: '2', role: 'heading', level: 1, name: 'Active Project Title', bounds: { x: 16, y: 12, w: 240, h: 32 } },
      { id: '3', role: 'button', name: 'Navigation Menu', bounds: { x: screenWidth - 48, y: 12, w: 32, h: 32 } },
      { id: '4', role: 'main', name: 'Primary Viewport Canvas', bounds: { x: 0, y: 56, w: screenWidth, h: screenHeight - 120 } },
      { id: '5', role: 'textbox', name: 'Search Query / Input', bounds: { x: 16, y: 72, w: screenWidth - 32, h: 44 } },
      { id: '6', role: 'button', name: 'Submit / Action Trigger', bounds: { x: 16, y: 128, w: screenWidth - 32, h: 48 } },
      { id: '7', role: 'navigation', name: 'Bottom Tab Bar', bounds: { x: 0, y: screenHeight - 64, w: screenWidth, h: 64 } },
    ];
  }, [screenWidth, screenHeight]);

  const handleCopyAxTree = () => {
    const formatted = JSON.stringify(
      {
        device: device.name,
        orientation,
        viewport: { width: screenWidth, height: screenHeight, dpr: device.dpr },
        url: activeUrl,
        accessibility_tree: mockAxTree,
      },
      null,
      2
    );
    navigator.clipboard.writeText(formatted);
    setCopiedAx(true);
    setTimeout(() => setCopiedAx(false), 2000);
    onShowToast?.('Accessibility tree copied for Agent context.');
  };

  const handleCaptureEvidence = () => {
    onShowToast?.(`Visual evidence receipt recorded for ${device.name} (${screenWidth}x${screenHeight}).`);
  };

  return (
    <div className="flex flex-col h-full bg-[#101012] text-zinc-200 text-xs overflow-hidden select-none font-sans">
      {/* Top Controls Toolbar */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-zinc-800 bg-[#161619] shrink-0">
        <div className="flex items-center gap-2">
          <Smartphone size={14} className="text-cyan-400 shrink-0" />
          <span className="font-semibold text-zinc-100 tracking-wide text-[11px] uppercase">
            Mobile Device Simulator
          </span>
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-cyan-950/60 text-cyan-400 border border-cyan-800/40 font-mono">
            {device.platform.toUpperCase()}
          </span>
        </div>

        {/* Device Switcher & Orientation Controls */}
        <div className="flex items-center gap-2">
          <select
            value={selectedDeviceId}
            onChange={(e) => setSelectedDeviceId(e.target.value)}
            className="bg-zinc-900 border border-zinc-700 text-zinc-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-zinc-500"
          >
            {DEVICE_PRESETS.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name} ({d.width}×{d.height})
              </option>
            ))}
          </select>

          <button
            onClick={toggleOrientation}
            className="flex items-center gap-1 px-2 py-1 bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 rounded text-zinc-300 text-[11px] transition"
            title="Rotate Device"
          >
            <RotateCw size={12} className={orientation === 'landscape' ? 'text-cyan-400' : ''} />
            <span className="capitalize">{orientation}</span>
          </button>

          {/* Scale Presets */}
          <div className="flex items-center bg-zinc-900 border border-zinc-800 rounded px-1">
            <button
              onClick={() => setScale((s) => Math.max(0.4, Number((s - 0.1).toFixed(2))))}
              className="p-1 text-zinc-400 hover:text-zinc-200"
              title="Zoom Out"
            >
              <ZoomOut size={12} />
            </button>
            <span className="font-mono text-[10px] text-zinc-400 px-1.5">{Math.round(scale * 100)}%</span>
            <button
              onClick={() => setScale((s) => Math.min(1.2, Number((s + 0.1).toFixed(2))))}
              className="p-1 text-zinc-400 hover:text-zinc-200"
              title="Zoom In"
            >
              <ZoomIn size={12} />
            </button>
          </div>

          {/* Power Toggle */}
          <button
            onClick={() => setIsPowerOn(!isPowerOn)}
            className={`p-1.5 rounded border transition ${
              isPowerOn
                ? 'bg-zinc-800 hover:bg-zinc-700 text-emerald-400 border-zinc-700'
                : 'bg-red-950/40 text-red-400 border-red-800/40'
            }`}
            title={isPowerOn ? 'Turn Screen Off' : 'Turn Screen On'}
          >
            <Power size={12} />
          </button>
        </div>
      </div>

      {/* Address & Quick Destination Bar */}
      <div className="flex items-center gap-2 px-3 py-1.5 bg-[#141416] border-b border-zinc-800 shrink-0">
        <button
          onClick={handleReload}
          className="p-1.5 rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300 transition"
          title="Reload Frame"
        >
          <RefreshCw size={12} />
        </button>

        <form onSubmit={handleNavigate} className="flex-1 flex items-center relative">
          <input
            type="text"
            value={urlInput}
            onChange={(e) => setUrlInput(e.target.value)}
            placeholder="e.g. http://localhost:5173, http://localhost:3000..."
            className="w-full bg-zinc-950 border border-zinc-800 focus:border-cyan-700 rounded px-3 py-1 text-xs text-zinc-200 font-mono tracking-tight focus:outline-none transition"
          />
          <button
            type="submit"
            className="absolute right-1 px-2 py-0.5 bg-cyan-900/60 hover:bg-cyan-800/80 border border-cyan-700/60 text-cyan-200 rounded text-[11px]"
          >
            <ArrowRight size={11} />
          </button>
        </form>

        {/* Quick Local Server Presets */}
        <div className="flex items-center gap-1 text-[10px] font-mono">
          <button
            onClick={() => {
              setUrlInput('http://localhost:5173');
              setActiveUrl('http://localhost:5173');
            }}
            className="px-1.5 py-0.5 rounded bg-zinc-800/60 hover:bg-zinc-700 text-zinc-400 hover:text-zinc-200 transition"
          >
            :5173
          </button>
          <button
            onClick={() => {
              setUrlInput('http://localhost:3000');
              setActiveUrl('http://localhost:3000');
            }}
            className="px-1.5 py-0.5 rounded bg-zinc-800/60 hover:bg-zinc-700 text-zinc-400 hover:text-zinc-200 transition"
          >
            :3000
          </button>
          <button
            onClick={() => {
              setUrlInput('http://localhost:8080');
              setActiveUrl('http://localhost:8080');
            }}
            className="px-1.5 py-0.5 rounded bg-zinc-800/60 hover:bg-zinc-700 text-zinc-400 hover:text-zinc-200 transition"
          >
            :8080
          </button>
        </div>
      </div>

      {/* Main Workbench Body: Canvas + Inspection Drawer */}
      <div className="flex-1 flex overflow-hidden">
        {/* Center: Device Simulation Canvas */}
        <div className="flex-1 flex flex-col items-center justify-start overflow-auto p-8 bg-[radial-gradient(#1f2024_1px,transparent_1px)] [background-size:16px_16px]">
          <div
            className="relative transition-all duration-300 ease-out origin-top flex flex-col items-center"
            style={{
              transform: `scale(${scale})`,
              marginBottom: `${Math.max(40, screenHeight * scale * 0.1)}px`,
            }}
          >
            {/* Realistic Physical Device Frame */}
            <div
              className="relative p-[10px] bg-[#232428] rounded-[52px] shadow-[0_25px_60px_-15px_rgba(0,0,0,0.9),0_0_0_1px_rgba(255,255,255,0.08)] border border-zinc-700/60 flex flex-col"
              style={{
                width: `${screenWidth + 20}px`,
                height: `${screenHeight + 20}px`,
                borderRadius: device.bezelRadius,
              }}
            >
              {/* Outer Edge Hardware Accent / Antennas */}
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

                {/* Live Screen Content / Iframe */}
                {!isPowerOn ? (
                  <div className="flex-1 flex flex-col items-center justify-center bg-black text-zinc-700 select-none">
                    <Power size={32} className="mb-2 opacity-40" />
                    <span className="text-[11px] font-mono">Device Screen Suspended</span>
                  </div>
                ) : (
                  <div className="relative flex-1 w-full h-full bg-zinc-950 overflow-hidden flex flex-col">
                    <iframe
                      ref={iframeRef}
                      src={activeUrl}
                      title="Mobile Device Simulator Viewport"
                      sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
                      className="w-full h-full border-0 bg-white"
                      style={{
                        width: '100%',
                        height: '100%',
                      }}
                    />

                    {/* iOS Home Indicator Bar */}
                    {device.platform === 'ios' && (
                      <div className="absolute bottom-1.5 left-1/2 -translate-x-1/2 w-32 h-1 bg-zinc-400/60 rounded-full z-20 pointer-events-none" />
                    )}
                  </div>
                )}
              </div>
            </div>

            {/* Device Info Badge under frame */}
            <div className="mt-3 flex items-center gap-2 font-mono text-[11px] text-zinc-500">
              <span className="text-zinc-400">{device.name}</span>
              <span>•</span>
              <span>{screenWidth} × {screenHeight} pt</span>
              <span>•</span>
              <span>DPR: {device.dpr}x</span>
            </div>
          </div>
        </div>

        {/* Right Drawer: Inspection & Coding Agent Intelligence */}
        <div className="w-80 flex flex-col bg-[#161619] border-l border-zinc-800 shrink-0">
          {/* Drawer Tabs */}
          <div className="flex items-center border-b border-zinc-800 bg-[#121214] text-[11px]">
            <button
              onClick={() => setActiveDrawer('axtree')}
              className={`flex-1 py-2 text-center border-b-2 font-medium transition ${
                activeDrawer === 'axtree'
                  ? 'border-cyan-500 text-cyan-400 bg-zinc-900/40'
                  : 'border-transparent text-zinc-400 hover:text-zinc-200'
              }`}
            >
              Accessibility Tree
            </button>
            <button
              onClick={() => setActiveDrawer('console')}
              className={`flex-1 py-2 text-center border-b-2 font-medium transition ${
                activeDrawer === 'console'
                  ? 'border-cyan-500 text-cyan-400 bg-zinc-900/40'
                  : 'border-transparent text-zinc-400 hover:text-zinc-200'
              }`}
            >
              Console
            </button>
            <button
              onClick={() => setActiveDrawer('evidence')}
              className={`flex-1 py-2 text-center border-b-2 font-medium transition ${
                activeDrawer === 'evidence'
                  ? 'border-cyan-500 text-cyan-400 bg-zinc-900/40'
                  : 'border-transparent text-zinc-400 hover:text-zinc-200'
              }`}
            >
              Receipts
            </button>
          </div>

          {/* Drawer Content */}
          <div className="flex-1 overflow-y-auto p-3 space-y-3 font-mono text-[11px]">
            {activeDrawer === 'axtree' && (
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-1.5 text-zinc-400 text-xs font-sans">
                    <Sparkles size={13} className="text-cyan-400" />
                    <span>Agent Screen Ingestion</span>
                  </div>
                  <button
                    onClick={handleCopyAxTree}
                    className="flex items-center gap-1 px-2 py-0.5 rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300 text-[10px] transition"
                  >
                    {copiedAx ? <Check size={11} className="text-emerald-400" /> : <Copy size={11} />}
                    <span>{copiedAx ? 'Copied' : 'Copy for Agent'}</span>
                  </button>
                </div>

                <p className="text-[10px] font-sans text-zinc-500 leading-normal">
                  Hierarchical element snapshot for Coding LLMs to inspect UI responsiveness, button reachability, and touch target sizes.
                </p>

                <div className="space-y-1.5">
                  {mockAxTree.map((node) => (
                    <div
                      key={node.id}
                      className="p-2 rounded bg-zinc-900/60 border border-zinc-800 hover:border-zinc-700 transition"
                    >
                      <div className="flex items-center justify-between text-[10px]">
                        <span className="text-cyan-400 font-semibold uppercase">{node.role}</span>
                        <span className="text-zinc-500">{node.bounds.w}×{node.bounds.h}px</span>
                      </div>
                      <div className="text-zinc-200 text-xs mt-0.5 truncate">{node.name}</div>
                      <div className="text-[10px] text-zinc-600 mt-0.5">
                        Pos: ({node.bounds.x}, {node.bounds.y})
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {activeDrawer === 'console' && (
              <div className="space-y-2">
                <div className="flex items-center justify-between text-xs font-sans text-zinc-400">
                  <span className="flex items-center gap-1.5">
                    <Terminal size={13} className="text-emerald-400" />
                    <span>Mobile Logcat / Web Console</span>
                  </span>
                  <span className="text-[10px] text-emerald-400 font-mono">Live</span>
                </div>
                <div className="p-3 bg-zinc-950 rounded border border-zinc-850 text-zinc-400 space-y-1.5 text-[10px] leading-relaxed">
                  <div className="text-zinc-500">[Info] Viewport initialized at {screenWidth}x{screenHeight} @{device.dpr}x</div>
                  <div className="text-zinc-500">[Info] Touch gesture controller attached</div>
                  <div className="text-emerald-400">[Log] React dev server connected: {activeUrl}</div>
                  <div className="text-zinc-400">[Info] DOMContentLoaded in 42ms</div>
                </div>
              </div>
            )}

            {activeDrawer === 'evidence' && (
              <div className="space-y-3">
                <div className="flex items-center justify-between text-xs font-sans text-zinc-400">
                  <span className="flex items-center gap-1.5">
                    <ShieldCheck size={13} className="text-cyan-400" />
                    <span>Invariant Visual Evidence</span>
                  </span>
                </div>
                <p className="text-[10px] font-sans text-zinc-500 leading-normal">
                  Record mobile viewport render proof into the session journal to satisfy verification gates.
                </p>
                <button
                  onClick={handleCaptureEvidence}
                  className="w-full flex items-center justify-center gap-2 py-1.5 bg-cyan-900/60 hover:bg-cyan-800/80 border border-cyan-700/60 rounded text-cyan-200 text-xs font-sans font-medium transition"
                >
                  <Camera size={13} />
                  <span>Record Render Receipt</span>
                </button>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
