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
} from 'lucide-react';
import { NativeSelect } from '@/components/common/NativeSelect';

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
  onAskAgent?: (draft: string) => void;
}

export const MobileSimulatorWorkbenchPane: React.FC<MobileSimulatorWorkbenchPaneProps> = ({
  onShowToast,
  defaultUrl = 'http://localhost:5173',
  onAskAgent,
}) => {
  const [selectedDeviceId, setSelectedDeviceId] = useState<string>('iphone-15-pro');
  const [orientation, setOrientation] = useState<'portrait' | 'landscape'>('portrait');
  const [scale, setScale] = useState<number>(0.75);
  const [urlInput, setUrlInput] = useState<string>(defaultUrl);
  const [activeUrl, setActiveUrl] = useState<string>(defaultUrl);
  const [isPowerOn, setIsPowerOn] = useState<boolean>(true);
  const [activeDrawer, setActiveDrawer] = useState<'axtree' | 'console' | 'evidence'>('axtree');
  const [justInserted, setJustInserted] = useState<boolean>(false);
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
    const formatted = trimmed.startsWith('http') ? trimmed : `http://${trimmed}`;
    setActiveUrl(formatted);
    setUrlInput(formatted);
  };

  const handleReload = () => {
    if (iframeRef.current) {
      iframeRef.current.src = activeUrl;
    }
    onShowToast?.(`Reloaded ${activeUrl}`);
  };

  const toggleOrientation = () => {
    setOrientation((prev) => (prev === 'portrait' ? 'landscape' : 'portrait'));
  };

  // Accessibility Tree (AX Tree) representation for Agent ingestion & responsive verification
  const mockAxTree = useMemo(() => {
    return [
      { id: '1', role: 'banner', name: 'App Header', bounds: { x: 0, y: 0, w: screenWidth, h: 56 } },
      { id: '2', role: 'heading', level: 1, name: 'Active Project Viewport', bounds: { x: 16, y: 12, w: 240, h: 32 } },
      { id: '3', role: 'button', name: 'Navigation Menu', bounds: { x: screenWidth - 48, y: 12, w: 32, h: 32 } },
      { id: '4', role: 'main', name: 'Primary Mobile Viewport', bounds: { x: 0, y: 56, w: screenWidth, h: screenHeight - 120 } },
      { id: '5', role: 'textbox', name: 'Search Query / Input', bounds: { x: 16, y: 72, w: screenWidth - 32, h: 44 } },
      { id: '6', role: 'button', name: 'Action Button', bounds: { x: 16, y: 128, w: screenWidth - 32, h: 48 } },
      { id: '7', role: 'navigation', name: 'Bottom Tab Bar', bounds: { x: 0, y: screenHeight - 64, w: screenWidth, h: 64 } },
    ];
  }, [screenWidth, screenHeight]);

  const handleInsertToChat = () => {
    const axSummary = mockAxTree
      .map((n) => `- [${n.role}] "${n.name}" (${n.bounds.w}×${n.bounds.h}px)`)
      .join('\n');
    const snippet = `Context Mobile Simulator [${device.name} - ${orientation}]:\n- Preview URL: ${activeUrl}\n- Resolution: ${screenWidth}×${screenHeight} (dpr: ${device.dpr})\n- Accessibility Tree:\n${axSummary}`;
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
            <span>Live Preview</span>
          </div>
        </div>

        {/* Device Switcher & Hardware Controls */}
        <div className="flex items-center gap-2">
          <NativeSelect
            value={selectedDeviceId}
            onChange={(e) => setSelectedDeviceId(e.target.value)}
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
            onClick={handleInsertToChat}
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
          onClick={handleReload}
          className="p-1.5 rounded bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-3)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition border border-[var(--color-border-muted)]"
          title="Reload Preview Frame"
        >
          <RefreshCw className="h-3 w-3" />
        </button>

        <form onSubmit={handleNavigate} className="flex-1 flex items-center relative">
          <input
            type="text"
            value={urlInput}
            onChange={(e) => setUrlInput(e.target.value)}
            placeholder="e.g. http://localhost:5173, http://localhost:3000..."
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
            onClick={() => {
              setUrlInput('http://localhost:5173');
              setActiveUrl('http://localhost:5173');
            }}
            className="px-1.5 py-0.5 rounded bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-3)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] border border-[var(--color-border-muted)] transition"
          >
            :5173
          </button>
          <button
            onClick={() => {
              setUrlInput('http://localhost:3000');
              setActiveUrl('http://localhost:3000');
            }}
            className="px-1.5 py-0.5 rounded bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-3)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] border border-[var(--color-border-muted)] transition"
          >
            :3000
          </button>
          <button
            onClick={() => {
              setUrlInput('http://localhost:8080');
              setActiveUrl('http://localhost:8080');
            }}
            className="px-1.5 py-0.5 rounded bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-3)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] border border-[var(--color-border-muted)] transition"
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

                {/* Display Content */}
                {isPowerOn ? (
                  <iframe
                    ref={iframeRef}
                    key={activeUrl}
                    src={activeUrl}
                    title={device.name}
                    className="w-full h-full flex-1 border-0 bg-white"
                    sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
                  />
                ) : (
                  <div className="flex-1 flex flex-col items-center justify-center bg-black text-zinc-600 text-[11px]">
                    <Power className="h-8 w-8 mb-2 opacity-30" />
                    <span>Screen Powered Off</span>
                  </div>
                )}

                {/* Home Indicator Bar */}
                {isPowerOn && orientation === 'portrait' && (
                  <div className="absolute bottom-1.5 left-1/2 -translate-x-1/2 w-32 h-1 bg-white/60 rounded-full z-30 pointer-events-none" />
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
                <div className="text-[11px] text-[var(--color-fg-subtle)] mb-2">
                  Semantic Accessibility Tree for agent UI review and testing:
                </div>

                <div className="space-y-1.5">
                  {mockAxTree.map((node) => (
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
                  ))}
                </div>
              </div>
            )}

            {activeDrawer === 'console' && (
              <div className="space-y-2 font-mono text-[11px] text-[var(--color-fg-subtle)]">
                <div className="p-2.5 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] space-y-1">
                  <p className="text-emerald-400">[info] Simulator viewport: {screenWidth}×{screenHeight} @{device.dpr}x</p>
                  <p className="text-[var(--color-fg-muted)]">[log] Active orientation: {orientation}</p>
                  <p className="text-[var(--color-fg-muted)]">[log] Frame src: {activeUrl}</p>
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
