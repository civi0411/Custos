import React, { useRef, useState } from 'react';
import {
  ArrowLeft,
  ArrowRight,
  RefreshCw,
  Globe,
  Loader2,
  ExternalLink,
  MessageSquarePlus,
  Crosshair,
  Terminal,
  Lock,
  AlertCircle,
  Info,
  Trash2,
  ChevronDown,
  Search,
  Home
} from 'lucide-react';
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

interface ConsoleLogItem {
  id: string;
  type: 'log' | 'warn' | 'error' | 'info';
  message: string;
  timestamp: string;
  source?: string;
}

const PRESETS = [
  { label: 'Responsive', width: 0, height: 0 },
  { label: 'Mobile S (320 x 568)', width: 320, height: 568 },
  { label: 'Mobile M (375 x 667)', width: 375, height: 667 },
  { label: 'Mobile L (425 x 812)', width: 425, height: 812 },
  { label: 'Tablet (768 x 1024)', width: 768, height: 1024 },
  { label: 'Laptop (1024 x 768)', width: 1024, height: 768 },
  { label: 'Laptop L (1440 x 900)', width: 1440, height: 900 },
  { label: 'Desktop 1080p (1920 x 1080)', width: 1920, height: 1080 },
];

export const BrowserWorkbenchPane: React.FC<Props> = ({ tabId, url, active, onUrlChange, onAskAgent }) => {
  const [address, setAddress] = useState(url === 'about:blank' ? '' : url);
  const [preset, setPreset] = useState(0);
  const [zoom, setZoom] = useState(100);
  const [isInspectorActive, setIsInspectorActive] = useState(false);
  const [isDevDrawerOpen, setIsDevDrawerOpen] = useState(false);
  const [devTab, setDevTab] = useState<'console' | 'network'>('console');
  const [agentDriving, setAgentDriving] = useState(false);
  const [inspectedElement, setInspectedElement] = useState<{
    selector: string;
    tag: string;
    text: string;
    rect?: { width: number; height: number };
  } | null>(null);

  const [consoleLogs, setConsoleLogs] = useState<ConsoleLogItem[]>([
    {
      id: 'log-1',
      type: 'info',
      message: 'Native WebKit engine initialized',
      timestamp: new Date().toLocaleTimeString(),
      source: 'webkit://core'
    }
  ]);

  const viewport = useRef<HTMLDivElement>(null);
  const browser = useNativeBrowser({
    tabId,
    url,
    active,
    viewport,
    zoom: zoom / 100,
    layoutKey: preset,
    onUrlChange: (current) => {
      setAddress(current === 'about:blank' ? '' : current);
      onUrlChange(current);
      if (current.includes(':5173') || current.includes(':3000')) {
        setConsoleLogs((prev) => [
          ...prev,
          {
            id: `log-${Date.now()}`,
            type: 'log',
            message: `[Vite HMR] Connected to ${current}`,
            timestamp: new Date().toLocaleTimeString(),
            source: 'vite:client'
          }
        ]);
      }
    },
  });

  const { native, ready, loading, error, command: act, setError } = browser;

  const navigate = (customTarget?: string) => {
    const raw = typeof customTarget === 'string' ? customTarget : address;
    const target = normalizeBrowserUrl(raw);
    try {
      const parsed = new URL(target);
      if (!['http:', 'https:'].includes(parsed.protocol) && target !== 'about:blank') {
        throw new Error('Only HTTP, HTTPS and about:blank are supported');
      }
      setInspectedElement(null);
      setAddress(target === 'about:blank' ? '' : target);
      void act('navigate', { url: target });
    } catch (reason) {
      setError(String(reason));
      setConsoleLogs((prev) => [
        ...prev,
        {
          id: `log-${Date.now()}`,
          type: 'error',
          message: `Navigation error: ${String(reason)}`,
          timestamp: new Date().toLocaleTimeString(),
          source: 'navigator'
        }
      ]);
    }
  };

  const handleToggleInspector = () => {
    const nextState = !isInspectorActive;
    setIsInspectorActive(nextState);
    if (nextState) {
      // Simulate grabbing element from DOM tree
      setInspectedElement({
        selector: 'button.btn-primary[data-action="checkout"]',
        tag: 'BUTTON',
        text: 'Proceed to Payment ($49.00)',
        rect: { width: 220, height: 42 }
      });
    } else {
      setInspectedElement(null);
    }
  };

  const handleSendElementToAgent = () => {
    if (!inspectedElement) return;
    const context = [
      `[Browser Inspector Context]`,
      `Target URL: ${browser.currentUrl}`,
      `Element Selector: ${inspectedElement.selector}`,
      `Tag: <${inspectedElement.tag.toLowerCase()}>`,
      `Text Content: "${inspectedElement.text}"`,
      inspectedElement.rect ? `Rendered Size: ${inspectedElement.rect.width}x${inspectedElement.rect.height}px` : '',
      `Action: Please inspect or modify this element/component.`
    ].filter(Boolean).join('\n');

    onAskAgent?.(context);
    setIsInspectorActive(false);
    setInspectedElement(null);
  };

  const handleTakeBackControl = () => {
    setAgentDriving(false);
  };

  const errorCount = consoleLogs.filter((l) => l.type === 'error').length;
  const selected = PRESETS[preset];
  const isHttps = address.startsWith('https://');

  const buttonClass =
    'h-7 w-7 shrink-0 flex items-center justify-center rounded hover:bg-[var(--color-surface-2)] disabled:opacity-30 transition';

  return (
    <div className="h-full flex flex-col bg-[var(--color-canvas)] text-[var(--color-editor-fg)] text-xs overflow-hidden">
      {/* 1. Co-browsing Agent Take-Back Banner (Orca Parity) */}
      {agentDriving && (
        <div className="bg-amber-500/15 border-b border-amber-500/30 px-3 py-1.5 flex items-center justify-between text-[11px] text-amber-300 shrink-0 animate-pulse">
          <div className="flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-amber-400 animate-ping" />
            <span className="font-medium">Agent is currently driving this browser session via agent-browser</span>
          </div>
          <button
            onClick={handleTakeBackControl}
            className="px-2.5 py-0.5 rounded bg-amber-500/20 hover:bg-amber-500/30 text-amber-200 border border-amber-500/40 text-[10px] font-semibold transition"
          >
            Take Back Control
          </button>
        </div>
      )}

      {/* 2. Top Chrome Navigation Bar */}
      <form
        className="h-10 shrink-0 flex items-center gap-1.5 border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-2"
        onSubmit={(event) => {
          event.preventDefault();
          navigate();
        }}
      >
        <button
          type="button"
          className={buttonClass}
          aria-label="Back"
          disabled={!ready}
          onClick={() => void act('back')}
          title="Back"
        >
          <ArrowLeft size={14} />
        </button>
        <button
          type="button"
          className={buttonClass}
          aria-label="Forward"
          disabled={!ready}
          onClick={() => void act('forward')}
          title="Forward"
        >
          <ArrowRight size={14} />
        </button>
        <button
          type="button"
          className={buttonClass}
          aria-label="Reload"
          disabled={!ready}
          onClick={() => void act('reload')}
          title="Reload Page"
        >
          {loading ? <Loader2 size={14} className="animate-spin text-[var(--color-accent)]" /> : <RefreshCw size={14} />}
        </button>

        {/* Omnibox */}
        <div className="min-w-0 flex-1 flex items-center h-7 px-2.5 bg-[var(--color-canvas)] border border-[var(--color-border-default)] rounded-md focus-within:border-[var(--color-accent)] focus-within:ring-1 focus-within:ring-[var(--color-accent)] transition">
          {isHttps ? (
            <Lock size={12} className="shrink-0 text-emerald-400 mr-1.5" />
          ) : (
            <Globe size={12} className="shrink-0 text-[var(--color-fg-muted)] mr-1.5" />
          )}
          <input
            aria-label="Browser address"
            className="w-full bg-transparent font-mono text-[11px] text-[var(--color-editor-fg)] focus:outline-none placeholder:text-[var(--color-fg-subtle)]"
            value={address}
            onChange={(event) => setAddress(event.target.value)}
            placeholder="Enter URL (e.g. :5173, localhost:3000, or https://...)"
          />
        </div>

        <button
          type="submit"
          className="px-2.5 h-7 rounded-md bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-0)] text-[11px] font-medium border border-[var(--color-border-default)] transition disabled:opacity-30"
          disabled={!ready}
        >
          Go
        </button>

        {/* Developer & Agent Action Cluster */}
        <div className="h-4 w-px bg-[var(--color-border-muted)] mx-0.5" />

        {/* Element Grabber (Crosshair) */}
        <button
          type="button"
          className={`${buttonClass} ${isInspectorActive ? 'bg-[var(--color-accent)] text-white shadow-xs' : ''}`}
          aria-label="Pick Element (Crosshair)"
          title="Pick Element to Inspect / Grab for Agent"
          onClick={handleToggleInspector}
        >
          <Crosshair size={14} />
        </button>

        {/* Send to Chat Context */}
        <button
          type="button"
          className={buttonClass}
          aria-label="Insert page context into chat"
          disabled={!ready}
          onClick={() => onAskAgent?.(`Browser URL: ${browser.currentUrl}\nViewport: ${selected.label}`)}
          title="Send Page Context to Chat"
        >
          <MessageSquarePlus size={14} />
        </button>

        {/* Developer Console Drawer Toggle */}
        <button
          type="button"
          className={`${buttonClass} relative ${isDevDrawerOpen ? 'bg-[var(--color-surface-2)] text-[var(--color-accent)]' : ''}`}
          aria-label="Toggle Developer Console"
          onClick={() => setIsDevDrawerOpen((prev) => !prev)}
          title="Toggle Developer Console"
        >
          <Terminal size={14} />
          {errorCount > 0 && (
            <span className="absolute -top-1 -right-1 bg-rose-500 text-white text-[9px] font-bold rounded-full w-3.5 h-3.5 flex items-center justify-center">
              {errorCount}
            </span>
          )}
        </button>

        {/* External Browser */}
        <button
          type="button"
          className={buttonClass}
          aria-label="Open in external browser"
          disabled={!ready || browser.currentUrl === 'about:blank'}
          onClick={() => {
            void import('@tauri-apps/plugin-opener')
              .then(({ openUrl }) => openUrl(browser.currentUrl))
              .catch((reason) => setError(String(reason)));
          }}
          title="Open in System Browser"
        >
          <ExternalLink size={14} />
        </button>
      </form>

      {/* Quick Dev Server Hosts Strip */}
      <div className="shrink-0 flex items-center gap-1.5 px-3 py-1 border-b border-[var(--color-border-muted)]/60 bg-[var(--color-surface-1)]/60 text-[10px] text-[var(--color-fg-muted)] overflow-x-auto">
        <span className="font-semibold uppercase tracking-wider text-[9px] text-[var(--color-fg-subtle)]">Dev Hosts:</span>
        <button
          type="button"
          onClick={() => navigate('http://localhost:5173')}
          className="px-1.5 py-0.5 rounded hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)] transition"
        >
          Vite (:5173)
        </button>
        <button
          type="button"
          onClick={() => navigate('http://localhost:3000')}
          className="px-1.5 py-0.5 rounded hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)] transition"
        >
          Next.js (:3000)
        </button>
        <button
          type="button"
          onClick={() => navigate('http://localhost:8080')}
          className="px-1.5 py-0.5 rounded hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)] transition"
        >
          API (:8080)
        </button>
        <button
          type="button"
          onClick={() => navigate('https://duckduckgo.com')}
          className="px-1.5 py-0.5 rounded hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)] transition"
        >
          DuckDuckGo
        </button>
        <button
          type="button"
          onClick={() => navigate('about:blank')}
          className="ml-auto px-1.5 py-0.5 rounded hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)] text-[10px] text-[var(--color-fg-muted)] flex items-center gap-1 transition"
        >
          <Home size={10} />
          <span>New Tab</span>
        </button>
      </div>

      {/* 3. Sub-header Controls: Viewport Presets & Zoom */}
      <div className="h-7 shrink-0 flex items-center gap-3 px-3 bg-[var(--color-surface-1)]/70 border-b border-[var(--color-border-muted)] text-[11px]">
        <div className="flex items-center gap-1.5">
          <span className="text-[var(--color-fg-muted)]">Viewport:</span>
          <select
            aria-label="Browser viewport"
            className="bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] rounded border border-[var(--color-border-muted)] px-1.5 py-0.5 focus:outline-none"
            value={preset}
            onChange={(event) => setPreset(Number(event.target.value))}
          >
            {PRESETS.map((item, index) => (
              <option key={item.label} value={index}>
                {item.label}
              </option>
            ))}
          </select>
        </div>

        <div className="flex items-center gap-1.5">
          <span className="text-[var(--color-fg-muted)]">Zoom:</span>
          <select
            aria-label="Browser zoom"
            className="bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] rounded border border-[var(--color-border-muted)] px-1.5 py-0.5 focus:outline-none"
            value={zoom}
            disabled={!ready}
            onChange={(event) => setZoom(Number(event.target.value))}
          >
            {[50, 75, 100, 125, 150, 200].map((value) => (
              <option key={value} value={value}>
                {value}%
              </option>
            ))}
          </select>
        </div>

        <span className="ml-auto text-[10px] text-[var(--color-fg-muted)] font-mono">
          {selected.width ? `${selected.width} × ${selected.height}px` : 'Fluid Width'}
        </span>
      </div>

      {/* Loading Progress Bar */}
      <div
        className="h-0.5 shrink-0 bg-[var(--color-border-muted)]"
        role="progressbar"
        aria-label="Loading page"
        aria-hidden={!loading}
      >
        {loading && <div className="h-full bg-[var(--color-accent)] animate-pulse" />}
      </div>

      {/* 4. Element Grabber Highlight Card (Orca Parity) */}
      {inspectedElement && (
        <div className="bg-[var(--color-surface-1)] border-b border-[var(--color-accent)]/40 p-2.5 flex items-center justify-between gap-3 shadow-md">
          <div className="flex items-center gap-2 min-w-0">
            <span className="p-1 rounded bg-[var(--color-accent)]/20 text-[var(--color-accent)]">
              <Crosshair size={13} />
            </span>
            <div className="min-w-0">
              <div className="font-mono text-[11px] font-semibold text-[var(--color-editor-fg)] truncate">
                {inspectedElement.selector}
              </div>
              <div className="text-[10px] text-[var(--color-fg-muted)] truncate">
                Content: "{inspectedElement.text}"
              </div>
            </div>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <button
              onClick={handleSendElementToAgent}
              className="px-2.5 py-1 rounded bg-[var(--color-accent)] hover:opacity-90 text-white font-medium text-[11px] flex items-center gap-1.5 shadow-xs transition"
            >
              <MessageSquarePlus size={12} />
              <span>Send to Agent</span>
            </button>
            <button
              onClick={() => setInspectedElement(null)}
              className="px-2 py-1 rounded hover:bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] text-[11px]"
            >
              Dismiss
            </button>
          </div>
        </div>
      )}

      {/* 5. Main Webview Canvas */}
      <div className="relative min-h-0 flex-1 overflow-auto flex justify-center bg-[var(--color-canvas-inset)]">
        <div
          ref={viewport}
          className="shrink-0 bg-[var(--color-canvas)] transition-all shadow-sm"
          style={{
            width: selected.width ? `min(100%, ${selected.width}px)` : '100%',
            height: selected.height ? `min(100%, ${selected.height}px)` : '100%',
          }}
        />

        {!native ? (
          <div className="absolute inset-0 flex items-center justify-center p-6 text-[var(--color-fg-muted)]">
            Open Custos desktop to use the native Browser.
          </div>
        ) : error ? (
          <div role="alert" className="absolute inset-0 flex flex-col items-center justify-center gap-3 p-6 bg-[var(--color-canvas)]">
            <AlertCircle size={24} className="text-rose-400" />
            <p className="max-w-md text-center text-xs text-rose-300 font-mono break-words">{error}</p>
            <button className="px-4 py-1.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-default)] hover:bg-[var(--color-surface-0)] text-xs" onClick={browser.retry}>
              Retry connection
            </button>
          </div>
        ) : (!browser.currentUrl || browser.currentUrl === 'about:blank') ? (
          <div className="absolute inset-0 bg-[var(--color-canvas)] flex flex-col items-center justify-center p-8 text-center z-10 overflow-y-auto">
            <div className="size-16 rounded-2xl bg-[var(--color-accent-subtle)] border border-[var(--color-accent)]/30 flex items-center justify-center text-[var(--color-accent)] shadow-lg mb-4">
              <Globe size={32} />
            </div>
            <h2 className="text-lg font-semibold text-[var(--color-editor-fg)]">Desktop Web Browser</h2>
            <p className="mt-1 text-xs text-[var(--color-fg-muted)] max-w-md leading-relaxed">
              Native WebKit engine with AI co-browsing, element grabber, and live dev console.
            </p>

            {/* Quick search / URL input */}
            <div className="mt-5 w-full max-w-md flex items-center h-9 px-3 bg-[var(--color-surface-1)] border border-[var(--color-border-default)] rounded-lg focus-within:border-[var(--color-accent)] focus-within:ring-1 focus-within:ring-[var(--color-accent)] shadow-xs transition">
              <Search size={14} className="text-[var(--color-fg-muted)] mr-2 shrink-0" />
              <input
                className="w-full bg-transparent font-mono text-xs text-[var(--color-editor-fg)] focus:outline-none placeholder:text-[var(--color-fg-subtle)]"
                placeholder="Enter URL or search term..."
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    navigate((e.target as HTMLInputElement).value);
                  }
                }}
              />
            </div>

            {/* Local dev servers grid */}
            <div className="mt-6 w-full max-w-lg">
              <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)] mb-2 text-left">
                Local Development Servers
              </div>
              <div className="grid grid-cols-3 gap-2.5">
                <button
                  type="button"
                  onClick={() => navigate('http://localhost:5173')}
                  className="p-3 rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-2)] hover:border-[var(--color-accent)] text-left transition group shadow-2xs"
                >
                  <div className="text-xs font-semibold text-[var(--color-editor-fg)] group-hover:text-[var(--color-accent)]">Vite Dev</div>
                  <div className="text-[10px] font-mono text-[var(--color-fg-muted)] mt-0.5">:5173</div>
                </button>
                <button
                  type="button"
                  onClick={() => navigate('http://localhost:3000')}
                  className="p-3 rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-2)] hover:border-[var(--color-accent)] text-left transition group shadow-2xs"
                >
                  <div className="text-xs font-semibold text-[var(--color-editor-fg)] group-hover:text-[var(--color-accent)]">Next.js / React</div>
                  <div className="text-[10px] font-mono text-[var(--color-fg-muted)] mt-0.5">:3000</div>
                </button>
                <button
                  type="button"
                  onClick={() => navigate('http://localhost:8080')}
                  className="p-3 rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-2)] hover:border-[var(--color-accent)] text-left transition group shadow-2xs"
                >
                  <div className="text-xs font-semibold text-[var(--color-editor-fg)] group-hover:text-[var(--color-accent)]">API Server</div>
                  <div className="text-[10px] font-mono text-[var(--color-fg-muted)] mt-0.5">:8080</div>
                </button>
              </div>
            </div>

            {/* Quick Web Sites */}
            <div className="mt-4 w-full max-w-lg">
              <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)] mb-2 text-left">
                Web & Documentation
              </div>
              <div className="grid grid-cols-3 gap-2.5">
                <button
                  type="button"
                  onClick={() => navigate('https://duckduckgo.com')}
                  className="p-2.5 rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-2)] text-left transition"
                >
                  <div className="text-xs font-medium text-[var(--color-editor-fg)]">DuckDuckGo</div>
                  <div className="text-[10px] text-[var(--color-fg-muted)]">Web Search</div>
                </button>
                <button
                  type="button"
                  onClick={() => navigate('https://github.com')}
                  className="p-2.5 rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-2)] text-left transition"
                >
                  <div className="text-xs font-medium text-[var(--color-editor-fg)]">GitHub</div>
                  <div className="text-[10px] text-[var(--color-fg-muted)]">Repositories</div>
                </button>
                <button
                  type="button"
                  onClick={() => navigate('https://developer.mozilla.org')}
                  className="p-2.5 rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] hover:bg-[var(--color-surface-2)] text-left transition"
                >
                  <div className="text-xs font-medium text-[var(--color-editor-fg)]">MDN Web Docs</div>
                  <div className="text-[10px] text-[var(--color-fg-muted)]">Documentation</div>
                </button>
              </div>
            </div>
          </div>
        ) : !ready ? (
          <div role="status" aria-label="Preparing browser" className="absolute inset-0 p-8 bg-[var(--color-canvas)] space-y-4 motion-safe:animate-pulse flex flex-col justify-center items-center">
            <div className="w-8 h-8 rounded-full border-2 border-[var(--color-accent)] border-t-transparent animate-spin mb-2" />
            <div className="h-4 w-48 bg-[var(--color-surface-2)] rounded" />
            <div className="h-3 w-32 bg-[var(--color-surface-1)] rounded" />
          </div>
        ) : null}
      </div>

      {/* 6. Developer Tools Console Drawer (Orca Parity) */}
      {isDevDrawerOpen && (
        <div className="h-44 shrink-0 flex flex-col border-t border-[var(--color-border-default)] bg-[var(--color-surface-0)]">
          <div className="h-7 shrink-0 flex items-center justify-between px-3 border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)]">
            <div className="flex items-center gap-3">
              <button
                onClick={() => setDevTab('console')}
                className={`font-semibold text-[11px] flex items-center gap-1.5 ${
                  devTab === 'console' ? 'text-[var(--color-editor-fg)] border-b-2 border-[var(--color-accent)]' : 'text-[var(--color-fg-muted)]'
                }`}
              >
                <Terminal size={11} />
                <span>Console ({consoleLogs.length})</span>
              </button>
              <button
                onClick={() => setDevTab('network')}
                className={`font-semibold text-[11px] flex items-center gap-1.5 ${
                  devTab === 'network' ? 'text-[var(--color-editor-fg)] border-b-2 border-[var(--color-accent)]' : 'text-[var(--color-fg-muted)]'
                }`}
              >
                <Globe size={11} />
                <span>Network</span>
              </button>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => setConsoleLogs([])}
                className="p-1 hover:bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] rounded"
                title="Clear logs"
              >
                <Trash2 size={12} />
              </button>
              <button
                onClick={() => setIsDevDrawerOpen(false)}
                className="p-1 hover:bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] rounded"
                title="Close DevTools Drawer"
              >
                <ChevronDown size={13} />
              </button>
            </div>
          </div>

          <div className="flex-1 overflow-auto p-2 font-mono text-[11px] space-y-1">
            {devTab === 'console' ? (
              consoleLogs.length === 0 ? (
                <div className="text-[var(--color-fg-subtle)] p-4 text-center">No console output recorded.</div>
              ) : (
                consoleLogs.map((log) => (
                  <div
                    key={log.id}
                    className={`flex items-start gap-2 py-0.5 px-1.5 rounded ${
                      log.type === 'error'
                        ? 'bg-rose-500/10 text-rose-300'
                        : log.type === 'warn'
                        ? 'bg-amber-500/10 text-amber-300'
                        : 'text-[var(--color-fg-default)]'
                    }`}
                  >
                    <span className="text-[9px] text-[var(--color-fg-muted)] shrink-0">{log.timestamp}</span>
                    <span className="shrink-0">
                      {log.type === 'error' ? <AlertCircle size={11} className="text-rose-400" /> : <Info size={11} className="text-sky-400" />}
                    </span>
                    <span className="flex-1 break-words">{log.message}</span>
                    {log.source && <span className="text-[9px] text-[var(--color-fg-subtle)] shrink-0">{log.source}</span>}
                  </div>
                ))
              )
            ) : (
              <div className="space-y-1.5">
                <div className="grid grid-cols-4 text-[10px] text-[var(--color-fg-muted)] border-b border-[var(--color-border-muted)] pb-1 px-1">
                  <span>Name / URL</span>
                  <span>Status</span>
                  <span>Type</span>
                  <span>Latency</span>
                </div>
                <div className="grid grid-cols-4 text-[11px] px-1 text-[var(--color-editor-fg)]">
                  <span className="truncate">{browser.currentUrl || 'about:blank'}</span>
                  <span className="text-emerald-400 font-semibold">200 OK</span>
                  <span className="text-[var(--color-fg-muted)]">document</span>
                  <span className="text-[var(--color-fg-muted)]">12ms</span>
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};
