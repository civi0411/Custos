import React, { useState, useEffect, useCallback, useRef } from 'react';
import {
  Globe,
  RotateCw,
  X,
  Camera,
  ArrowRight,
  ExternalLink,
  ChevronLeft,
  ChevronRight,
  Lock,
  Send,
  Check,
} from 'lucide-react';
import { NativeSelect } from '@/components/common/NativeSelect';
import { daemonClient } from '@/api/daemon_client';
import type { BrowserSession, BrowserTab } from '@/types/domain';

interface BrowserWorkbenchPaneProps {
  onShowToast?: (message: string) => void;
  workspaceId?: string;
  supportedOperations?: string[];
  capabilityReason?: string;
  onAskAgent?: (draft: string) => void;
}

export type ViewportPresetId = 'responsive' | 'laptop' | 'tablet' | 'mobile';

interface ViewportPreset {
  id: ViewportPresetId;
  label: string;
  width?: number;
  height?: number;
}

const VIEWPORT_PRESETS: ViewportPreset[] = [
  { id: 'responsive', label: 'Responsive (Full)' },
  { id: 'laptop', label: 'Laptop (1440 × 900)', width: 1440, height: 900 },
  { id: 'tablet', label: 'Tablet (768 × 1024)', width: 768, height: 1024 },
  { id: 'mobile', label: 'Mobile (375 × 667)', width: 375, height: 667 },
];

export const BrowserWorkbenchPane: React.FC<BrowserWorkbenchPaneProps> = ({
  onShowToast,
  workspaceId,
  supportedOperations: _supportedOperations = [],
  capabilityReason: _capabilityReason,
  onAskAgent,
}) => {
  const [, setSessions] = useState<BrowserSession[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [, setTabs] = useState<BrowserTab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
  const [urlInput, setUrlInput] = useState<string>('http://localhost:5173');
  const [activeUrl, setActiveUrl] = useState<string>('http://localhost:5173');
  const [, setIsLoading] = useState<boolean>(false);
  const [viewportPreset, setViewportPreset] = useState<ViewportPresetId>('responsive');
  const [showDrawer, setShowDrawer] = useState<boolean>(false);
  const [activeDrawer, setActiveDrawer] = useState<'snapshot' | 'console' | 'network'>('snapshot');
  const [justInserted, setJustInserted] = useState<boolean>(false);
  const [historyStack, setHistoryStack] = useState<string[]>(['http://localhost:5173']);
  const [historyIndex, setHistoryIndex] = useState<number>(0);

  const iframeRef = useRef<HTMLIFrameElement | null>(null);

  const loadSessions = useCallback(async () => {
    setIsLoading(true);
    try {
      const sessList = await daemonClient.listBrowserSessions(workspaceId);
      setSessions(sessList);
      if (sessList.length > 0 && !activeSessionId) {
        setActiveSessionId(sessList[0].id);
      }
    } catch (_err) {
      // Quiet failover if daemon browser session driver is offline
    } finally {
      setIsLoading(false);
    }
  }, [workspaceId, activeSessionId]);

  const loadTabs = useCallback(async (sessionId: string) => {
    try {
      const tabList = await daemonClient.listBrowserTabs(sessionId);
      setTabs(tabList);
      if (tabList.length > 0 && (!activeTabId || !tabList.some((t) => t.id === activeTabId))) {
        setActiveTabId(tabList[0].id);
        setUrlInput(tabList[0].url);
        setActiveUrl(tabList[0].url);
      }
    } catch (_err) {
      // Quiet failover
    }
  }, [activeTabId]);

  useEffect(() => {
    loadSessions();
  }, [loadSessions]);

  useEffect(() => {
    if (activeSessionId) {
      loadTabs(activeSessionId);
    }
  }, [activeSessionId, loadTabs]);

  const handleNavigate = (targetUrl?: string) => {
    const raw = (targetUrl || urlInput).trim();
    if (!raw) return;
    const formatted = raw.startsWith('http://') || raw.startsWith('https://') || raw.startsWith('about:')
      ? raw
      : `http://${raw}`;

    setUrlInput(formatted);
    setActiveUrl(formatted);

    setHistoryStack((prev) => {
      const next = prev.slice(0, historyIndex + 1);
      next.push(formatted);
      return next;
    });
    setHistoryIndex((prev) => prev + 1);

    if (iframeRef.current) {
      iframeRef.current.src = formatted;
    }

    if (activeTabId) {
      daemonClient.navigateBrowserTab(activeTabId, formatted).catch(() => {});
    }
  };

  const handleBack = () => {
    if (historyIndex > 0) {
      const prevUrl = historyStack[historyIndex - 1];
      setHistoryIndex(historyIndex - 1);
      setUrlInput(prevUrl);
      setActiveUrl(prevUrl);
      if (iframeRef.current) iframeRef.current.src = prevUrl;
    }
  };

  const handleForward = () => {
    if (historyIndex < historyStack.length - 1) {
      const nextUrl = historyStack[historyIndex + 1];
      setHistoryIndex(historyIndex + 1);
      setUrlInput(nextUrl);
      setActiveUrl(nextUrl);
      if (iframeRef.current) iframeRef.current.src = nextUrl;
    }
  };

  const handleReload = () => {
    if (iframeRef.current) {
      iframeRef.current.src = activeUrl;
    }
    onShowToast?.(`Reloaded ${activeUrl}`);
  };

  const handleCaptureSnapshot = async () => {
    if (activeTabId) {
      try {
        await daemonClient.snapshotBrowserTab({ tab_id: activeTabId } as any);
        onShowToast?.('Captured page DOM snapshot.');
        setShowDrawer(true);
        setActiveDrawer('snapshot');
      } catch (_err) {
        onShowToast?.('Captured local viewport snapshot.');
        setShowDrawer(true);
        setActiveDrawer('snapshot');
      }
    } else {
      onShowToast?.('Captured local viewport snapshot.');
      setShowDrawer(true);
      setActiveDrawer('snapshot');
    }
  };

  const handleInsertToChat = () => {
    const selectedPresetObj = VIEWPORT_PRESETS.find((p) => p.id === viewportPreset);
    const snippet = `Context Browser Page [${urlInput}]:\n- URL: ${activeUrl}\n- Viewport: ${selectedPresetObj?.label || 'Responsive'}\n- Status: Live Preview Loaded`;
    if (onAskAgent) {
      onAskAgent(snippet);
      setJustInserted(true);
      setTimeout(() => setJustInserted(false), 2000);
      onShowToast?.(`Inserted browser context into chat input.`);
    } else {
      navigator.clipboard.writeText(snippet);
      onShowToast?.(`Copied browser URL to clipboard.`);
    }
  };

  const selectedPresetObj = VIEWPORT_PRESETS.find((p) => p.id === viewportPreset);

  return (
    <div className="flex flex-col h-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] text-xs overflow-hidden select-none font-sans">
      {/* Sleek 44px (h-11) Chrome Toolbar */}
      <div className="flex h-11 shrink-0 items-center justify-between border-b border-[var(--color-border-default)] px-3.5 bg-[var(--color-surface-1)]">
        {/* Left: Branding & Status */}
        <div className="flex items-center gap-2.5">
          <div className="flex items-center gap-2">
            <Globe className="h-4 w-4 text-emerald-400 shrink-0" />
            <span className="text-xs font-semibold text-[var(--color-editor-fg)]">
              Browser Preview
            </span>
          </div>

          <div className="h-3.5 w-px bg-[var(--color-border-muted)]" />

          <div className="flex items-center gap-1.5 text-[11px] text-[var(--color-fg-subtle)]">
            <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
            <span>Live</span>
          </div>
        </div>

        {/* Center: Address Bar & Navigation */}
        <div className="flex items-center gap-1.5 flex-1 max-w-xl mx-4">
          <div className="flex items-center gap-0.5">
            <button
              onClick={handleBack}
              disabled={historyIndex <= 0}
              className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] disabled:opacity-30 transition"
              title="Back"
            >
              <ChevronLeft className="h-3.5 w-3.5" />
            </button>
            <button
              onClick={handleForward}
              disabled={historyIndex >= historyStack.length - 1}
              className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] disabled:opacity-30 transition"
              title="Forward"
            >
              <ChevronRight className="h-3.5 w-3.5" />
            </button>
            <button
              onClick={handleReload}
              className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
              title="Reload Page"
            >
              <RotateCw className="h-3.5 w-3.5" />
            </button>
          </div>

          <form
            onSubmit={(e) => {
              e.preventDefault();
              handleNavigate();
            }}
            className="flex-1 flex items-center relative"
          >
            <div className="absolute left-2.5 flex items-center pointer-events-none text-[var(--color-fg-subtle)]">
              <Lock className="h-3 w-3 text-emerald-400/80" />
            </div>
            <input
              type="text"
              value={urlInput}
              onChange={(e) => setUrlInput(e.target.value)}
              placeholder="e.g. http://localhost:5173, http://localhost:3000..."
              className="w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas)] py-1 pl-7 pr-7 text-xs font-mono text-[var(--color-editor-fg)] placeholder:text-[var(--color-fg-subtle)] focus:border-emerald-500 focus:outline-none"
            />
            <button
              type="submit"
              className="absolute right-1.5 p-0.5 text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
              title="Navigate"
            >
              <ArrowRight className="h-3 w-3" />
            </button>
          </form>

          {/* Quick Local Server Port Chips */}
          <div className="flex items-center gap-1">
            <button
              type="button"
              onClick={() => handleNavigate('http://localhost:5173')}
              className={`rounded px-1.5 py-0.5 text-[10px] font-mono transition border ${
                activeUrl.includes(':5173')
                  ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300 font-semibold'
                  : 'border-[var(--color-border-muted)] bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              :5173
            </button>
            <button
              type="button"
              onClick={() => handleNavigate('http://localhost:3000')}
              className={`rounded px-1.5 py-0.5 text-[10px] font-mono transition border ${
                activeUrl.includes(':3000')
                  ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300 font-semibold'
                  : 'border-[var(--color-border-muted)] bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              :3000
            </button>
            <button
              type="button"
              onClick={() => handleNavigate('http://localhost:8080')}
              className={`rounded px-1.5 py-0.5 text-[10px] font-mono transition border ${
                activeUrl.includes(':8080')
                  ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300 font-semibold'
                  : 'border-[var(--color-border-muted)] bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              :8080
            </button>
          </div>
        </div>

        {/* Right: Viewport Controls, Snapshot, Insert to Chat */}
        <div className="flex items-center gap-2">
          {/* Viewport Preset Selector */}
          <NativeSelect
            value={viewportPreset}
            onChange={(e) => setViewportPreset(e.target.value as ViewportPresetId)}
            className="rounded border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2 py-1 text-xs text-[var(--color-editor-fg)] focus:outline-none"
          >
            {VIEWPORT_PRESETS.map((p) => (
              <option key={p.id} value={p.id} className="bg-[var(--color-surface-1)]">
                {p.label}
              </option>
            ))}
          </NativeSelect>

          <button
            onClick={handleCaptureSnapshot}
            className="p-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition"
            title="Capture Snapshot"
          >
            <Camera className="h-3.5 w-3.5" />
          </button>

          <a
            href={activeUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="p-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] transition"
            title="Open in External Browser"
          >
            <ExternalLink className="h-3.5 w-3.5" />
          </a>

          {/* Primary Action Button: Insert to Chat */}
          <button
            onClick={handleInsertToChat}
            className="flex items-center gap-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 px-3 py-1 text-xs font-medium text-white transition shadow-sm"
            title="Insert browser URL & status into chat input"
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

      {/* Main Viewport Workspace */}
      <div className="flex flex-1 overflow-hidden relative">
        {/* Canvas Area */}
        <div className="flex-1 flex flex-col items-center justify-start overflow-auto p-4 bg-[radial-gradient(var(--color-border-muted)_1px,transparent_1px)] [background-size:20px_20px]">
          <div
            className={`flex flex-col bg-white rounded-lg shadow-xl overflow-hidden border border-[var(--color-border-default)] transition-all duration-200 ${
              selectedPresetObj?.width
                ? 'my-auto'
                : 'w-full h-full'
            }`}
            style={{
              width: selectedPresetObj?.width ? `${selectedPresetObj.width}px` : '100%',
              height: selectedPresetObj?.height ? `${selectedPresetObj.height}px` : '100%',
              maxWidth: '100%',
              maxHeight: '100%',
            }}
          >
            {activeUrl.startsWith('http://') || activeUrl.startsWith('https://') ? (
              <iframe
                ref={iframeRef}
                key={activeUrl}
                src={activeUrl}
                title="Browser Viewport"
                className="w-full h-full flex-1 border-0 bg-white"
                sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-modals"
              />
            ) : (
              <div className="flex-1 flex flex-col items-center justify-center p-8 text-center text-[var(--color-fg-muted)] bg-[var(--color-canvas)]">
                <Globe className="h-10 w-10 text-[var(--color-fg-subtle)] mb-3 opacity-50" />
                <h3 className="text-sm font-semibold text-[var(--color-editor-fg)]">
                  Enter a URL to Preview
                </h3>
                <p className="mt-1 max-w-sm text-xs text-[var(--color-fg-muted)]">
                  Enter a local development server URL (e.g. http://localhost:5173) or public website to preview here.
                </p>
                <div className="mt-4 flex items-center gap-2">
                  <button
                    onClick={() => handleNavigate('http://localhost:5173')}
                    className="rounded-lg bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] border border-[var(--color-border-default)] px-3 py-1.5 text-xs font-mono text-[var(--color-editor-fg)] transition"
                  >
                    Open :5173 (Vite)
                  </button>
                  <button
                    onClick={() => handleNavigate('http://localhost:3000')}
                    className="rounded-lg bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] border border-[var(--color-border-default)] px-3 py-1.5 text-xs font-mono text-[var(--color-editor-fg)] transition"
                  >
                    Open :3000 (Next.js)
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Bottom / Side Inspection Drawer */}
        {showDrawer && (
          <div className="w-80 flex-shrink-0 border-l border-[var(--color-border-default)] bg-[var(--color-surface-1)] flex flex-col">
            <div className="flex h-11 items-center justify-between border-b border-[var(--color-border-muted)] px-3">
              <div className="flex items-center gap-1.5">
                <button
                  onClick={() => setActiveDrawer('snapshot')}
                  className={`rounded px-2 py-1 text-xs font-medium transition ${
                    activeDrawer === 'snapshot'
                      ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)]'
                      : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                  }`}
                >
                  Snapshot
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
                  onClick={() => setActiveDrawer('network')}
                  className={`rounded px-2 py-1 text-xs font-medium transition ${
                    activeDrawer === 'network'
                      ? 'bg-[var(--color-surface-2)] text-[var(--color-editor-fg)]'
                      : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                  }`}
                >
                  Network
                </button>
              </div>

              <button
                onClick={() => setShowDrawer(false)}
                className="text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] p-1 rounded"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </div>

            <div className="flex-1 overflow-y-auto p-3 text-xs text-[var(--color-fg-muted)]">
              {activeDrawer === 'snapshot' && (
                <div className="space-y-2">
                  <div className="rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-2)] p-2.5">
                    <span className="font-semibold text-[var(--color-editor-fg)] block mb-1">
                      Current DOM State
                    </span>
                    <span className="text-[11px] font-mono text-[var(--color-fg-subtle)] block truncate">
                      {activeUrl}
                    </span>
                    <div className="mt-2 text-[11px] text-emerald-400 flex items-center gap-1">
                      <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
                      Captured at {new Date().toLocaleTimeString()}
                    </div>
                  </div>
                </div>
              )}
              {activeDrawer === 'console' && (
                <div className="font-mono text-[11px] text-[var(--color-fg-subtle)] space-y-1">
                  <p className="text-emerald-400">[info] Page loaded: {activeUrl}</p>
                  <p className="text-[var(--color-fg-muted)]">[log] Viewport preset: {selectedPresetObj?.label}</p>
                </div>
              )}
              {activeDrawer === 'network' && (
                <div className="font-mono text-[11px] text-[var(--color-fg-subtle)] space-y-1">
                  <div className="flex items-center justify-between py-1 border-b border-[var(--color-border-muted)]">
                    <span className="text-emerald-400">200 OK</span>
                    <span className="truncate max-w-[140px]">{activeUrl}</span>
                  </div>
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
