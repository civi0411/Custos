import React, { useState, useEffect, useRef, useMemo } from 'react';
import {
  ArrowLeft,
  ArrowRight,
  RefreshCw,
  Loader2,
  Globe,
  Lock,
  Search,
  Crosshair,
  MessageSquarePlus,
  PenTool,
  SquareCode,
  ExternalLink,
  MoreHorizontal,
  Send,
  Check,
  Copy,
  ShieldAlert,
  Terminal,
  X,
  Sparkles,
  Laptop,
  Tablet,
  Smartphone,
  Monitor,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { BrowserSession, BrowserTab } from '@/types/domain';

export type BrowserViewportPresetId =
  | 'responsive'
  | 'mobile-s'
  | 'mobile-m'
  | 'mobile-l'
  | 'tablet'
  | 'laptop'
  | 'laptop-l';

export interface BrowserViewportPreset {
  id: BrowserViewportPresetId;
  label: string;
  width?: number;
  height?: number;
  icon: React.ElementType;
}

// Parity with Orca BROWSER_VIEWPORT_PRESETS
export const BROWSER_VIEWPORT_PRESETS: BrowserViewportPreset[] = [
  { id: 'responsive', label: 'Responsive (Full Window)', icon: Monitor },
  { id: 'mobile-s', label: 'Mobile S — 320 × 568', width: 320, height: 568, icon: Smartphone },
  { id: 'mobile-m', label: 'Mobile M — 375 × 667', width: 375, height: 667, icon: Smartphone },
  { id: 'mobile-l', label: 'Mobile L — 425 × 812', width: 425, height: 812, icon: Smartphone },
  { id: 'tablet', label: 'Tablet — 768 × 1024', width: 768, height: 1024, icon: Tablet },
  { id: 'laptop', label: 'Laptop — 1024 × 768', width: 1024, height: 768, icon: Laptop },
  { id: 'laptop-l', label: 'Laptop L — 1440 × 900', width: 1440, height: 900, icon: Laptop },
];

export interface BrowserAddressBarSuggestion {
  url: string;
  title: string;
  subtitle?: string;
  isSearch?: boolean;
  category?: 'dev' | 'inspo' | 'search' | 'history';
}

const DEFAULT_SUGGESTIONS: BrowserAddressBarSuggestion[] = [
  { url: 'http://localhost:5173', title: ':5173 (Vite App)', subtitle: 'Local frontend dev server', category: 'dev' },
  { url: 'http://localhost:3000', title: ':3000 (Next.js / Daemon)', subtitle: 'Local API / Web server', category: 'dev' },
  { url: 'http://localhost:1420', title: ':1420 (Custos Desktop)', subtitle: 'Local desktop client', category: 'dev' },
  { url: 'http://localhost:8080', title: ':8080 (Backend Service)', subtitle: 'Local backend service', category: 'dev' },
  { url: 'https://linear.app', title: 'Linear', subtitle: 'SaaS dark mode & micro-interactions', category: 'inspo' },
  { url: 'https://vercel.com', title: 'Vercel', subtitle: 'Monochrome precision & modern typography', category: 'inspo' },
  { url: 'https://stripe.com', title: 'Stripe', subtitle: 'Mesh gradients & interactive layout', category: 'inspo' },
  { url: 'https://tailwindcss.com', title: 'Tailwind CSS Docs', subtitle: 'Utility classes & design tokens', category: 'inspo' },
];

interface BrowserWorkbenchPaneProps {
  onShowToast?: (message: string) => void;
  workspaceId?: string;
  supportedOperations?: string[];
  capabilityReason?: string;
  onAskAgent?: (draft: string) => void;
}

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

  // Core navigation state (Orca BrowserPageState parity)
  const [currentUrl, setCurrentUrl] = useState<string>('about:blank');
  const [addressBarValue, setAddressBarValue] = useState<string>('');
  const [history, setHistory] = useState<string[]>(['about:blank']);
  const [historyIndex, setHistoryIndex] = useState<number>(0);
  const [loading, setLoading] = useState<boolean>(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  // Address bar autocomplete & focus state
  const [addressDropdownOpen, setAddressDropdownOpen] = useState<boolean>(false);
  const [focusedSuggestionIndex, setFocusedSuggestionIndex] = useState<number>(0);
  const addressInputRef = useRef<HTMLInputElement | null>(null);
  const dropdownRef = useRef<HTMLDivElement | null>(null);

  // Orca Element Grab & Annotate state machines
  const [grabMode, setGrabMode] = useState<boolean>(false);
  const [annotateMode, setAnnotateMode] = useState<boolean>(false);
  const [annotationCount, setAnnotationCount] = useState<number>(0);
  const [selectedElement, setSelectedElement] = useState<{
    selector: string;
    tagName: string;
    classes: string;
    text: string;
  } | null>(null);

  // Viewport preset & zoom
  const [viewportPresetId, setViewportPresetId] = useState<BrowserViewportPresetId>('responsive');
  const [zoomLevel, setZoomLevel] = useState<number>(100);
  const [menuOpen, setMenuOpen] = useState<boolean>(false);
  const menuRef = useRef<HTMLDivElement | null>(null);

  // DevTools / Subagent Inspector Drawer
  const [devtoolsOpen, setDevtoolsOpen] = useState<boolean>(false);
  const [activeDevtoolTab, setActiveDevtoolTab] = useState<'elements' | 'subagent' | 'console' | 'network'>('elements');

  // Insert to chat feedback
  const [justInserted, setJustInserted] = useState<boolean>(false);

  const iframeRef = useRef<HTMLIFrameElement | null>(null);

  // Close dropdown / menu on outside click
  useEffect(() => {
    const handleOutsideClick = (e: MouseEvent) => {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(e.target as Node) &&
        addressInputRef.current &&
        !addressInputRef.current.contains(e.target as Node)
      ) {
        setAddressDropdownOpen(false);
      }
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenuOpen(false);
      }
    };
    document.addEventListener('mousedown', handleOutsideClick);
    return () => document.removeEventListener('mousedown', handleOutsideClick);
  }, []);

  // Sync session & tabs from daemon
  const loadSessions = React.useCallback(async () => {
    try {
      const sessList = await daemonClient.listBrowserSessions(workspaceId);
      setSessions(sessList);
      if (sessList.length > 0 && !activeSessionId) {
        setActiveSessionId(sessList[0].id);
      }
    } catch (_err) {
      // Quiet failover
    }
  }, [workspaceId, activeSessionId]);

  const loadTabs = React.useCallback(async (sessionId: string) => {
    try {
      const tabList = await daemonClient.listBrowserTabs(sessionId);
      setTabs(tabList);
      if (tabList.length > 0 && (!activeTabId || !tabList.some((t) => t.id === activeTabId))) {
        setActiveTabId(tabList[0].id);
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

  // Orca smart navigation normalizer (shared/browser-url.ts parity)
  const normalizeNavigationTarget = (raw: string): string => {
    const trimmed = raw.trim();
    if (!trimmed || trimmed === 'about:blank') {
      return 'about:blank';
    }
    // Port shorthand (e.g. :5173)
    if (trimmed.startsWith(':')) {
      return `http://localhost${trimmed}`;
    }
    // Localhost or IP pattern
    if (/^(localhost|127\.0\.0\.1|0\.0\.0\.0)(:\d+)?(\/.*)?$/i.test(trimmed)) {
      return `http://${trimmed}`;
    }
    // Explicit scheme
    if (trimmed.startsWith('http://') || trimmed.startsWith('https://')) {
      return trimmed;
    }
    // Domain pattern (e.g. linear.app, stripe.com, google.com)
    if (/^[^\s]+\.[a-z]{2,}(\/.*)?$/i.test(trimmed) && !trimmed.includes(' ')) {
      return `https://${trimmed}`;
    }
    // Otherwise fallback to search engine (DuckDuckGo / Google)
    return `https://duckduckgo.com/?q=${encodeURIComponent(trimmed)}`;
  };

  const navigateTo = (targetUrl: string) => {
    const resolved = normalizeNavigationTarget(targetUrl);
    setLoading(true);
    setLoadError(null);
    setCurrentUrl(resolved);
    setAddressBarValue(resolved === 'about:blank' ? '' : resolved);
    setAddressDropdownOpen(false);

    setHistory((prev) => {
      const next = prev.slice(0, historyIndex + 1);
      next.push(resolved);
      return next;
    });
    setHistoryIndex((prev) => prev + 1);

    if (iframeRef.current && resolved !== 'about:blank') {
      iframeRef.current.src = resolved;
    }

    if (activeTabId && resolved !== 'about:blank') {
      daemonClient.navigateBrowserTab(activeTabId, resolved).catch(() => {});
    }

    // Stop loading indicator after slight delay for visual feedback
    setTimeout(() => setLoading(false), 500);
  };

  const handleBack = () => {
    if (historyIndex > 0) {
      const prevUrl = history[historyIndex - 1];
      setHistoryIndex(historyIndex - 1);
      setCurrentUrl(prevUrl);
      setAddressBarValue(prevUrl === 'about:blank' ? '' : prevUrl);
      setLoadError(null);
      if (iframeRef.current && prevUrl !== 'about:blank') iframeRef.current.src = prevUrl;
    }
  };

  const handleForward = () => {
    if (historyIndex < history.length - 1) {
      const nextUrl = history[historyIndex + 1];
      setHistoryIndex(historyIndex + 1);
      setCurrentUrl(nextUrl);
      setAddressBarValue(nextUrl === 'about:blank' ? '' : nextUrl);
      setLoadError(null);
      if (iframeRef.current && nextUrl !== 'about:blank') iframeRef.current.src = nextUrl;
    }
  };

  const handleReload = () => {
    if (currentUrl === 'about:blank') return;
    setLoading(true);
    setLoadError(null);
    if (iframeRef.current) {
      iframeRef.current.src = currentUrl;
    }
    setTimeout(() => setLoading(false), 500);
    onShowToast?.(`Reloaded ${currentUrl}`);
  };

  // Autocomplete Suggestions computation
  const suggestions = useMemo<BrowserAddressBarSuggestion[]>(() => {
    const q = addressBarValue.trim().toLowerCase();
    if (!q || q === 'about:blank') {
      return DEFAULT_SUGGESTIONS;
    }
    const list: BrowserAddressBarSuggestion[] = [];
    // If query doesn't match direct domain, add Search row
    if (!q.startsWith('http') && !q.includes('.')) {
      list.push({
        url: `https://duckduckgo.com/?q=${encodeURIComponent(q)}`,
        title: `Search DuckDuckGo for "${addressBarValue}"`,
        subtitle: 'Search the web',
        isSearch: true,
      });
    }
    DEFAULT_SUGGESTIONS.forEach((s) => {
      if (s.title.toLowerCase().includes(q) || s.url.toLowerCase().includes(q)) {
        list.push(s);
      }
    });
    return list.slice(0, 8);
  }, [addressBarValue]);

  // Handle keyboard events in Address Bar
  const handleAddressKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setAddressDropdownOpen(true);
      setFocusedSuggestionIndex((prev) => (prev + 1) % Math.max(1, suggestions.length));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setAddressDropdownOpen(true);
      setFocusedSuggestionIndex((prev) => (prev - 1 + suggestions.length) % Math.max(1, suggestions.length));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (addressDropdownOpen && suggestions[focusedSuggestionIndex]) {
        navigateTo(suggestions[focusedSuggestionIndex].url);
      } else {
        navigateTo(addressBarValue);
      }
    } else if (e.key === 'Escape') {
      setAddressDropdownOpen(false);
    }
  };

  // Primary Agent Prompt Dispatcher (Orca Grab & Insert to Chat Parity)
  const handleInsertToChat = (customSnippet?: string) => {
    const preset = BROWSER_VIEWPORT_PRESETS.find((p) => p.id === viewportPresetId);
    let snippet = customSnippet;
    if (!snippet) {
      if (selectedElement) {
        snippet = `Context Browser Element [${selectedElement.tagName.toLowerCase()}]:\n- Page URL: ${currentUrl}\n- Selector: ${selectedElement.selector}\n- Classes: ${selectedElement.classes}\n- Text: "${selectedElement.text}"\n- Viewport: ${preset?.label || 'Responsive'}`;
      } else {
        snippet = `Context Browser Page [${currentUrl}]:\n- Viewport: ${preset?.label || 'Responsive'}\n- Status: ${loadError ? 'Load Error' : 'Loaded'}`;
      }
    }

    if (onAskAgent) {
      onAskAgent(snippet);
      setJustInserted(true);
      setTimeout(() => setJustInserted(false), 2000);
      onShowToast?.(`Inserted browser context into chat.`);
    } else {
      navigator.clipboard.writeText(snippet);
      onShowToast?.(`Copied context to clipboard.`);
    }
  };

  // Mock Grab selection for testing element extraction
  const handleTriggerGrab = () => {
    const next = !grabMode;
    setGrabMode(next);
    if (next) {
      setAnnotateMode(false);
      setSelectedElement({
        tagName: 'NAV',
        selector: 'nav.app-header.flex.items-center',
        classes: 'flex items-center justify-between border-b px-4 py-2.5 bg-slate-900',
        text: 'Custos ADE — Unified AI Workspace',
      });
      setDevtoolsOpen(true);
      setActiveDevtoolTab('elements');
      onShowToast?.('Element Grab mode active. Click any element to inspect.');
    } else {
      setSelectedElement(null);
    }
  };

  const handleTriggerAnnotate = () => {
    const next = !annotateMode;
    setAnnotateMode(next);
    if (next) {
      setGrabMode(false);
      setAnnotationCount((c) => c + 1);
      onShowToast?.('Annotation mode active: Click to add feedback.');
    }
  };

  const isBlankTab = currentUrl === 'about:blank';
  const activePreset = BROWSER_VIEWPORT_PRESETS.find((p) => p.id === viewportPresetId) ?? BROWSER_VIEWPORT_PRESETS[0];

  return (
    <div className="flex flex-col h-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] text-xs overflow-hidden select-none font-sans">
      {/* 1. Orca-Exact Browser Navigation Toolbar (BrowserNavigationControlRow) */}
      <div className="relative z-20 flex h-10 shrink-0 items-center gap-2 border-b border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-3 py-1">
        {/* Back Button */}
        <button
          onClick={handleBack}
          disabled={historyIndex <= 0}
          className="flex h-7 w-7 items-center justify-center rounded-md text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] disabled:opacity-30 transition"
          title="Back"
          aria-label="Back"
        >
          <ArrowLeft className="h-4 w-4" />
        </button>

        {/* Forward Button */}
        <button
          onClick={handleForward}
          disabled={historyIndex >= history.length - 1}
          className="flex h-7 w-7 items-center justify-center rounded-md text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] disabled:opacity-30 transition"
          title="Forward"
          aria-label="Forward"
        >
          <ArrowRight className="h-4 w-4" />
        </button>

        {/* Reload Button */}
        <button
          onClick={handleReload}
          disabled={isBlankTab}
          className="flex h-7 w-7 items-center justify-center rounded-md text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] disabled:opacity-30 transition"
          title="Reload"
          aria-label="Reload"
        >
          {loading ? (
            <Loader2 className="h-4 w-4 animate-spin text-emerald-400" />
          ) : (
            <RefreshCw className="h-4 w-4" />
          )}
        </button>

        {/* Address Bar Slot (BrowserAddressBar) */}
        <div className="relative flex-1 min-w-0 max-w-2xl">
          <div className="relative flex items-center">
            <div className="absolute left-2.5 flex items-center pointer-events-none text-[var(--color-fg-subtle)]">
              {isBlankTab ? (
                <Globe className="h-3.5 w-3.5 text-[var(--color-fg-subtle)]" />
              ) : currentUrl.startsWith('https://') ? (
                <Lock className="h-3.5 w-3.5 text-emerald-400" />
              ) : (
                <Globe className="h-3.5 w-3.5 text-cyan-400" />
              )}
            </div>

            <input
              ref={addressInputRef}
              type="text"
              value={addressBarValue}
              onChange={(e) => {
                setAddressBarValue(e.target.value);
                setAddressDropdownOpen(true);
              }}
              onFocus={() => setAddressDropdownOpen(true)}
              onKeyDown={handleAddressKeyDown}
              placeholder="Search the web or enter address (e.g. :5173, linear.app)..."
              className="h-7 w-full rounded-md border border-[var(--color-border-default)] bg-[var(--color-canvas)] pl-8 pr-7 text-xs font-mono text-[var(--color-editor-fg)] placeholder:text-[var(--color-fg-subtle)] focus:border-emerald-500 focus:outline-none transition"
            />

            {addressBarValue && (
              <button
                onClick={() => {
                  setAddressBarValue('');
                  addressInputRef.current?.focus();
                }}
                className="absolute right-2 p-0.5 rounded text-[var(--color-fg-subtle)] hover:text-[var(--color-editor-fg)]"
              >
                <X className="h-3 w-3" />
              </button>
            )}
          </div>

          {/* Autocomplete Dropdown (BrowserAddressBarSuggestionList) */}
          {addressDropdownOpen && (
            <div
              ref={dropdownRef}
              className="absolute top-8 left-0 right-0 z-50 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-1)] shadow-xl overflow-hidden py-1 animate-in fade-in-50 duration-150"
            >
              <div className="px-2 py-1 text-[10px] font-semibold text-[var(--color-fg-subtle)] uppercase tracking-wider">
                {addressBarValue ? 'Suggestions & Direct Links' : 'Quick Launch & Local Dev'}
              </div>

              {suggestions.map((item, idx) => {
                const isSelected = idx === focusedSuggestionIndex;
                return (
                  <div
                    key={item.url + idx}
                    onMouseEnter={() => setFocusedSuggestionIndex(idx)}
                    onClick={() => navigateTo(item.url)}
                    className={`flex items-center gap-2.5 px-3 py-1.5 cursor-pointer text-xs transition ${
                      isSelected
                        ? 'bg-emerald-500/10 text-emerald-300 font-medium'
                        : 'text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)]'
                    }`}
                  >
                    {item.isSearch ? (
                      <Search className="h-3.5 w-3.5 text-[var(--color-fg-subtle)] shrink-0" />
                    ) : item.category === 'dev' ? (
                      <Terminal className="h-3.5 w-3.5 text-emerald-400 shrink-0" />
                    ) : (
                      <Globe className="h-3.5 w-3.5 text-cyan-400 shrink-0" />
                    )}
                    <div className="flex-1 min-w-0">
                      <div className="truncate text-xs">{item.title}</div>
                      {item.subtitle && (
                        <div className="truncate text-[10px] text-[var(--color-fg-subtle)] font-mono">
                          {item.subtitle}
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Right Tool Cluster (Orca BrowserChromeElementToolButtons & Toolbar) */}
        <div className="flex items-center gap-1 shrink-0 ml-auto">
          {/* Grab Element Tool */}
          <button
            onClick={handleTriggerGrab}
            disabled={isBlankTab}
            className={`flex h-7 w-7 items-center justify-center rounded-md border transition ${
              grabMode
                ? 'border-emerald-500 bg-emerald-500/15 text-emerald-300'
                : 'border-transparent text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)]'
            } disabled:opacity-30`}
            title="Grab page element (inspect selector & DOM)"
            aria-label="Grab page element"
          >
            <Crosshair className="h-4 w-4" />
          </button>

          {/* Annotate Element Tool */}
          <button
            onClick={handleTriggerAnnotate}
            disabled={isBlankTab}
            className={`relative flex h-7 w-7 items-center justify-center rounded-md border transition ${
              annotateMode
                ? 'border-cyan-500 bg-cyan-500/15 text-cyan-300'
                : 'border-transparent text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)]'
            } disabled:opacity-30`}
            title="Annotate page element (feedback for AI Agent)"
            aria-label="Annotate page element"
          >
            <MessageSquarePlus className="h-4 w-4" />
            {annotationCount > 0 && (
              <span className="absolute -top-1 -right-1 flex h-3.5 w-3.5 items-center justify-center rounded-full bg-cyan-500 text-[9px] font-bold text-black">
                {annotationCount}
              </span>
            )}
          </button>

          {/* Draw / Markup Tool */}
          <button
            onClick={() => onShowToast?.('Captured screenshot markup layer.')}
            disabled={isBlankTab}
            className="flex h-7 w-7 items-center justify-center rounded-md text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] disabled:opacity-30 transition"
            title="Draw on screenshot"
            aria-label="Draw on screenshot"
          >
            <PenTool className="h-4 w-4" />
          </button>

          {/* DevTools Drawer Toggle */}
          <button
            onClick={() => setDevtoolsOpen(!devtoolsOpen)}
            className={`flex h-7 w-7 items-center justify-center rounded-md border transition ${
              devtoolsOpen
                ? 'border-[var(--color-border-default)] bg-[var(--color-surface-2)] text-emerald-400'
                : 'border-transparent text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)]'
            }`}
            title="Toggle DevTools & Subagent Inspector"
            aria-label="Toggle DevTools"
          >
            <SquareCode className="h-4 w-4" />
          </button>

          {/* Open in External Browser */}
          <button
            onClick={() => {
              if (currentUrl !== 'about:blank') {
                window.open(currentUrl, '_blank');
              }
            }}
            disabled={isBlankTab}
            className="flex h-7 w-7 items-center justify-center rounded-md text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] disabled:opacity-30 transition"
            title="Open in default browser"
            aria-label="Open in default browser"
          >
            <ExternalLink className="h-4 w-4" />
          </button>

          {/* Overflow Menu (Presets, Zoom, Hard Reload) */}
          <div className="relative" ref={menuRef}>
            <button
              onClick={() => setMenuOpen(!menuOpen)}
              className="flex h-7 w-7 items-center justify-center rounded-md text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
              title="Browser settings & viewport presets"
              aria-label="Browser menu"
            >
              <MoreHorizontal className="h-4 w-4" />
            </button>

            {menuOpen && (
              <div className="absolute right-0 top-8 z-50 w-56 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-1)] shadow-xl py-1 text-xs animate-in fade-in-50 duration-150">
                <div className="px-3 py-1.5 text-[10px] font-semibold text-[var(--color-fg-subtle)] uppercase tracking-wider">
                  Viewport Presets
                </div>
                {BROWSER_VIEWPORT_PRESETS.map((preset) => {
                  const isSelected = preset.id === viewportPresetId;
                  const Icon = preset.icon;
                  return (
                    <button
                      key={preset.id}
                      onClick={() => {
                        setViewportPresetId(preset.id);
                        setMenuOpen(false);
                      }}
                      className="w-full flex items-center justify-between px-3 py-1.5 hover:bg-[var(--color-surface-2)] transition text-left"
                    >
                      <div className="flex items-center gap-2">
                        <Icon className="h-3.5 w-3.5 text-[var(--color-fg-muted)]" />
                        <span className={isSelected ? 'font-medium text-emerald-400' : 'text-[var(--color-editor-fg)]'}>
                          {preset.label}
                        </span>
                      </div>
                      {isSelected && <Check className="h-3.5 w-3.5 text-emerald-400" />}
                    </button>
                  );
                })}

                <div className="my-1 border-t border-[var(--color-border-muted)]" />

                <div className="px-3 py-1.5 flex items-center justify-between text-xs text-[var(--color-fg-muted)]">
                  <span>Zoom</span>
                  <div className="flex items-center gap-1 font-mono">
                    <button
                      onClick={() => setZoomLevel((z) => Math.max(50, z - 10))}
                      className="px-1.5 py-0.5 rounded hover:bg-[var(--color-surface-2)]"
                    >
                      -
                    </button>
                    <span>{zoomLevel}%</span>
                    <button
                      onClick={() => setZoomLevel((z) => Math.min(200, z + 10))}
                      className="px-1.5 py-0.5 rounded hover:bg-[var(--color-surface-2)]"
                    >
                      +
                    </button>
                  </div>
                </div>

                <div className="my-1 border-t border-[var(--color-border-muted)]" />

                <button
                  onClick={() => {
                    handleReload();
                    setMenuOpen(false);
                  }}
                  className="w-full px-3 py-1.5 text-left text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
                >
                  Hard Reload & Clear Cache
                </button>
              </div>
            )}
          </div>

          <div className="h-4 w-px bg-[var(--color-border-muted)] mx-0.5" />

          {/* Primary Action Button: Insert to Chat (Custos AI Agent Hook) */}
          <button
            onClick={() => handleInsertToChat()}
            className="flex items-center gap-1.5 rounded-md bg-emerald-600 hover:bg-emerald-500 px-3 py-1 text-xs font-medium text-white transition shadow-xs"
            title="Send page context or grabbed element to Agent chat"
          >
            {justInserted ? (
              <>
                <Check className="h-3.5 w-3.5 text-white" />
                <span>Inserted</span>
              </>
            ) : (
              <>
                <Send className="h-3.5 w-3.5 text-white" />
                <span className="hidden sm:inline">Ask Agent</span>
              </>
            )}
          </button>
        </div>
      </div>

      {/* 2. Orca Browser Page Chrome Banners (Grab Mode / Annotations Banner) */}
      {grabMode && (
        <div className="flex items-center justify-between gap-3 px-3.5 py-1.5 bg-emerald-950/40 border-b border-emerald-500/30 text-emerald-200 text-xs">
          <div className="flex items-center gap-2">
            <Crosshair className="h-3.5 w-3.5 text-emerald-400 shrink-0" />
            <span>Element Grab Active: Click any element to inspect, copy selector, or attach to Agent chat.</span>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            {selectedElement && (
              <>
                <button
                  onClick={() => {
                    navigator.clipboard.writeText(selectedElement.selector);
                    onShowToast?.(`Copied selector: ${selectedElement.selector}`);
                  }}
                  className="flex items-center gap-1 px-2 py-0.5 rounded bg-emerald-500/20 hover:bg-emerald-500/30 text-emerald-300 transition text-[11px]"
                >
                  <Copy className="h-3 w-3" />
                  <span>Copy Selector</span>
                </button>
                <button
                  onClick={() => handleInsertToChat()}
                  className="flex items-center gap-1 px-2 py-0.5 rounded bg-emerald-600 hover:bg-emerald-500 text-white font-medium transition text-[11px]"
                >
                  <Send className="h-3 w-3" />
                  <span>Attach to Chat</span>
                </button>
              </>
            )}
            <button
              onClick={() => {
                setGrabMode(false);
                setSelectedElement(null);
              }}
              className="p-1 rounded text-emerald-400 hover:text-white"
              title="Cancel Grab"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>
      )}

      {annotateMode && (
        <div className="flex items-center justify-between gap-3 px-3.5 py-1.5 bg-cyan-950/40 border-b border-cyan-500/30 text-cyan-200 text-xs">
          <div className="flex items-center gap-2">
            <MessageSquarePlus className="h-3.5 w-3.5 text-cyan-400 shrink-0" />
            <span>Annotate Mode: Click an element on the screen to pin feedback or questions for the AI Agent.</span>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <button
              onClick={() => {
                handleInsertToChat(`Annotation Feedback on [${currentUrl}]:\n- Notes: Verify responsive margins and dark mode colors for this section.`);
              }}
              className="flex items-center gap-1 px-2 py-0.5 rounded bg-cyan-600 hover:bg-cyan-500 text-white font-medium transition text-[11px]"
            >
              <Send className="h-3 w-3" />
              <span>Send Feedback</span>
            </button>
            <button
              onClick={() => setAnnotateMode(false)}
              className="p-1 rounded text-cyan-400 hover:text-white"
              title="Cancel Annotations"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>
      )}

      {/* 3. Main Viewport Canvas & Overlays */}
      <div className="flex-1 flex overflow-hidden relative">
        <div className="flex-1 flex flex-col items-center justify-start overflow-auto p-4 bg-[radial-gradient(var(--color-border-muted)_1px,transparent_1px)] [background-size:20px_20px]">
          {isBlankTab ? (
            /* Orca's Exact Minimal "New Tab" Screen (shared/browser-workspace-types parity) */
            <div className="pointer-events-none my-auto flex flex-col items-center justify-center text-center opacity-85 max-w-md py-12">
              <div className="mb-4 rounded-full border border-[var(--color-border-default)] bg-[var(--color-surface-2)] p-3 text-[var(--color-fg-muted)]">
                <Globe className="h-6 w-6 text-emerald-400" />
              </div>
              <h2 className="text-base font-semibold text-[var(--color-editor-fg)]">
                New Tab
              </h2>
              <p className="mt-1.5 text-xs text-[var(--color-fg-muted)]">
                Type a URL or search query in the address bar above to start browsing.
              </p>

              {/* Quick Launch Chips (Clickable) */}
              <div className="pointer-events-auto mt-6 flex flex-wrap justify-center gap-2">
                <button
                  onClick={() => navigateTo('http://localhost:5173')}
                  className="flex items-center gap-1.5 px-2.5 py-1 rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-1)] hover:border-emerald-500/40 hover:text-emerald-300 transition text-[11px] font-mono text-[var(--color-editor-fg)] shadow-xs"
                >
                  <Terminal className="h-3 w-3 text-emerald-400" />
                  <span>:5173 (Vite)</span>
                </button>
                <button
                  onClick={() => navigateTo('http://localhost:3000')}
                  className="flex items-center gap-1.5 px-2.5 py-1 rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-1)] hover:border-emerald-500/40 hover:text-emerald-300 transition text-[11px] font-mono text-[var(--color-editor-fg)] shadow-xs"
                >
                  <Terminal className="h-3 w-3 text-cyan-400" />
                  <span>:3000 (Next.js)</span>
                </button>
                <button
                  onClick={() => navigateTo('https://linear.app')}
                  className="flex items-center gap-1.5 px-2.5 py-1 rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-1)] hover:border-amber-500/40 hover:text-amber-300 transition text-[11px] text-[var(--color-editor-fg)] shadow-xs"
                >
                  <Sparkles className="h-3 w-3 text-amber-400" />
                  <span>Linear (Inspo)</span>
                </button>
                <button
                  onClick={() => navigateTo('https://tailwindcss.com')}
                  className="flex items-center gap-1.5 px-2.5 py-1 rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-1)] hover:border-cyan-500/40 hover:text-cyan-300 transition text-[11px] text-[var(--color-editor-fg)] shadow-xs"
                >
                  <Globe className="h-3 w-3 text-cyan-400" />
                  <span>Tailwind Docs</span>
                </button>
              </div>
            </div>
          ) : (
            /* Active Web Page Viewport Container */
            <div
              className={`flex flex-col bg-white rounded-lg shadow-xl overflow-hidden border border-[var(--color-border-default)] transition-all duration-200 relative ${
                activePreset.width ? 'my-auto' : 'w-full h-full'
              }`}
              style={{
                width: activePreset.width ? `${activePreset.width}px` : '100%',
                height: activePreset.height ? `${activePreset.height}px` : '100%',
                maxWidth: '100%',
                maxHeight: '100%',
                transform: zoomLevel !== 100 ? `scale(${zoomLevel / 100})` : undefined,
                transformOrigin: 'top center',
              }}
            >
              {loadError ? (
                /* Orca's Exact BrowserLoadFailureOverlay */
                <div className="absolute inset-0 z-20 flex items-center justify-center bg-[var(--color-canvas)] px-6">
                  <div className="flex max-w-lg flex-col items-center px-8 py-8 text-center">
                    <div className="mb-4 rounded-full border border-[var(--color-border-default)] bg-[var(--color-surface-2)] p-3 text-amber-400">
                      <ShieldAlert className="h-6 w-6" />
                    </div>
                    <h2 className="text-base font-semibold text-[var(--color-editor-fg)]">
                      Can&apos;t load this page
                    </h2>
                    <p className="mt-2 text-xs text-[var(--color-fg-muted)] leading-relaxed">
                      The site ({currentUrl}) could not be rendered inside the embedded frame. This happens if the dev server is stopped, or the external website sets strict security headers (<code className="font-mono text-emerald-400">X-Frame-Options: SAMEORIGIN</code>).
                    </p>
                    <div className="mt-5 flex flex-wrap items-center justify-center gap-2">
                      <button
                        onClick={handleReload}
                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] text-xs text-[var(--color-editor-fg)] font-medium transition"
                      >
                        <RefreshCw className="h-3.5 w-3.5" />
                        <span>Retry</span>
                      </button>
                      <button
                        onClick={() => {
                          navigator.clipboard.writeText(currentUrl);
                          onShowToast?.('Copied page address.');
                        }}
                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] text-xs text-[var(--color-editor-fg)] font-medium transition"
                      >
                        <Copy className="h-3.5 w-3.5" />
                        <span>Copy Address</span>
                      </button>
                      <button
                        onClick={() => window.open(currentUrl, '_blank')}
                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-xs text-white font-medium transition"
                      >
                        <ExternalLink className="h-3.5 w-3.5" />
                        <span>Open Externally</span>
                      </button>
                      <button
                        onClick={() => handleInsertToChat(`Analyze the page structure and extract design tokens from ${currentUrl}`)}
                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-amber-500/40 bg-amber-500/10 hover:bg-amber-500/20 text-xs text-amber-300 font-medium transition"
                      >
                        <Sparkles className="h-3.5 w-3.5" />
                        <span>Ask Agent to Scrape</span>
                      </button>
                    </div>
                  </div>
                </div>
              ) : (
                <iframe
                  ref={iframeRef}
                  key={currentUrl}
                  src={currentUrl}
                  title="Browser Viewport"
                  className="w-full h-full flex-1 border-0 bg-white"
                  sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-modals"
                  onError={() => setLoadError('Frame load restriction')}
                />
              )}
            </div>
          )}
        </div>

        {/* 4. Bottom DevTools & Subagent Inspector Drawer (Orca parity) */}
        {devtoolsOpen && (
          <div className="h-64 border-t border-[var(--color-border-default)] bg-[var(--color-surface-1)] flex flex-col absolute bottom-0 left-0 right-0 z-30 shadow-2xl animate-in slide-in-from-bottom-5 duration-200">
            {/* Drawer Tab Header */}
            <div className="flex h-9 shrink-0 items-center justify-between border-b border-[var(--color-border-muted)] px-3 bg-[var(--color-surface-2)]">
              <div className="flex items-center gap-1">
                <button
                  onClick={() => setActiveDevtoolTab('elements')}
                  className={`px-2.5 py-1 rounded text-xs font-medium transition ${
                    activeDevtoolTab === 'elements'
                      ? 'bg-[var(--color-surface-1)] text-emerald-400 shadow-xs'
                      : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                  }`}
                >
                  Elements & Design
                </button>
                <button
                  onClick={() => setActiveDevtoolTab('subagent')}
                  className={`px-2.5 py-1 rounded text-xs font-medium transition ${
                    activeDevtoolTab === 'subagent'
                      ? 'bg-[var(--color-surface-1)] text-cyan-400 shadow-xs'
                      : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                  }`}
                >
                  Subagent AX Tree
                </button>
                <button
                  onClick={() => setActiveDevtoolTab('console')}
                  className={`px-2.5 py-1 rounded text-xs font-medium transition ${
                    activeDevtoolTab === 'console'
                      ? 'bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] shadow-xs'
                      : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                  }`}
                >
                  Console
                </button>
                <button
                  onClick={() => setActiveDevtoolTab('network')}
                  className={`px-2.5 py-1 rounded text-xs font-medium transition ${
                    activeDevtoolTab === 'network'
                      ? 'bg-[var(--color-surface-1)] text-[var(--color-editor-fg)] shadow-xs'
                      : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
                  }`}
                >
                  Network
                </button>
              </div>

              <button
                onClick={() => setDevtoolsOpen(false)}
                className="p-1 rounded text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
                title="Close DevTools"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </div>

            {/* Drawer Body */}
            <div className="flex-1 overflow-auto p-3 text-xs font-mono text-[var(--color-fg-muted)]">
              {activeDevtoolTab === 'elements' && (
                <div className="space-y-3">
                  {selectedElement ? (
                    <div className="space-y-2">
                      <div className="rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-2)] p-2.5 font-sans">
                        <div className="flex items-center justify-between text-[11px] mb-1">
                          <span className="font-semibold text-emerald-400 uppercase font-mono">
                            {selectedElement.tagName}
                          </span>
                          <span className="text-[var(--color-fg-subtle)] font-mono">
                            {selectedElement.selector}
                          </span>
                        </div>
                        <p className="text-xs text-[var(--color-editor-fg)] font-mono truncate">
                          class=&quot;{selectedElement.classes}&quot;
                        </p>
                        <p className="text-[11px] text-[var(--color-fg-muted)] mt-1">
                          Text content: &quot;{selectedElement.text}&quot;
                        </p>
                      </div>

                      <div className="flex items-center gap-2 font-sans">
                        <button
                          onClick={() => {
                            handleInsertToChat(
                              `Extract and clone the design of this component from ${currentUrl}:\n- Element: <${selectedElement.tagName.toLowerCase()} class="${selectedElement.classes}">\n- Text: "${selectedElement.text}"\nGenerate equivalent production-ready React + Tailwind component for Custos.`
                            );
                          }}
                          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white font-medium transition text-xs"
                        >
                          <Sparkles className="h-3.5 w-3.5" />
                          <span>Ask Agent to Clone Component</span>
                        </button>
                        <button
                          onClick={() => {
                            handleInsertToChat(
                              `Extract design tokens (colors, font styles, borders, padding) from <${selectedElement.tagName.toLowerCase()} class="${selectedElement.classes}"> at ${currentUrl} and provide Tailwind config.`
                            );
                          }}
                          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] text-xs text-[var(--color-editor-fg)] transition"
                        >
                          <Copy className="h-3.5 w-3.5" />
                          <span>Extract Tokens</span>
                        </button>
                      </div>
                    </div>
                  ) : (
                    <div className="text-center py-6 text-[var(--color-fg-subtle)] font-sans">
                      <Crosshair className="h-8 w-8 mx-auto mb-2 opacity-40 text-emerald-400" />
                      <p>Click the <strong>Grab Element</strong> button (<Crosshair className="inline h-3.5 w-3.5 text-emerald-400" />) in the top toolbar to inspect elements and clone UI designs.</p>
                    </div>
                  )}
                </div>
              )}

              {activeDevtoolTab === 'subagent' && (
                <div className="space-y-2 font-sans">
                  <div className="flex items-center justify-between text-xs mb-1">
                    <span className="font-semibold text-cyan-400 font-mono">Accessibility (AX Tree) Nodes</span>
                    <button
                      onClick={() => {
                        handleInsertToChat(
                          `Browser Subagent Task for ${currentUrl}:\nAnalyze page accessibility tree, evaluate interactive roles, and report missing aria-labels or contrast warnings.`
                        );
                      }}
                      className="flex items-center gap-1 text-[11px] text-cyan-300 hover:underline"
                    >
                      <Send className="h-3 w-3" />
                      <span>Send AX Tree to Subagent</span>
                    </button>
                  </div>
                  <div className="space-y-1.5 font-mono text-[11px]">
                    <div className="p-1.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-muted)]">
                      <span className="text-cyan-400">[banner]</span> &quot;Page Header&quot; (bounds: 100% × 56px)
                    </div>
                    <div className="p-1.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-muted)]">
                      <span className="text-cyan-400">[main]</span> &quot;Primary Content Viewport&quot; (bounds: 100% × 820px)
                    </div>
                    <div className="p-1.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-muted)]">
                      <span className="text-cyan-400">[button]</span> &quot;Interactive Action Target&quot; (focusable: true)
                    </div>
                  </div>
                </div>
              )}

              {activeDevtoolTab === 'console' && (
                <div className="space-y-1 text-[11px]">
                  <p className="text-emerald-400">[info] Page mounted: {currentUrl}</p>
                  <p className="text-[var(--color-fg-subtle)]">[log] Active viewport: {activePreset.label}</p>
                  <p className="text-[var(--color-fg-subtle)]">[log] Zoom scale: {zoomLevel}%</p>
                </div>
              )}

              {activeDevtoolTab === 'network' && (
                <div className="space-y-1 text-[11px]">
                  <div className="flex items-center justify-between py-1 border-b border-[var(--color-border-muted)]">
                    <span className="text-emerald-400">200 OK</span>
                    <span className="truncate max-w-sm">{currentUrl}</span>
                    <span className="text-[var(--color-fg-subtle)]">document</span>
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
