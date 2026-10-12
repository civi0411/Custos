import { NativeSelect } from '@/components/common/NativeSelect';
import React, { useState, useEffect, useCallback } from 'react';
import {
  Globe,
  RotateCw,
  Plus,
  X,
  Camera,
  Terminal,
  Activity,
  Layers,
  ArrowRight,
} from 'lucide-react';
import {
  BrowserSession,
  BrowserTab,
} from '@/types/domain';
import { daemonClient } from '@/api/daemon_client';

interface BrowserWorkbenchPaneProps {
  onShowToast?: (message: string) => void;
  workspaceId?: string;
  supportedOperations: string[];
  capabilityReason?: string;
}

export const BrowserWorkbenchPane: React.FC<BrowserWorkbenchPaneProps> = ({
  onShowToast,
  workspaceId,
  supportedOperations,
  capabilityReason,
}) => {
  const can = useCallback(
    (operation: string) => supportedOperations.includes(operation),
    [supportedOperations]
  );
  const [sessions, setSessions] = useState<BrowserSession[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [tabs, setTabs] = useState<BrowserTab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
  const [urlInput, setUrlInput] = useState<string>('');
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [activeDrawer, setActiveDrawer] = useState<'snapshot' | 'console' | 'network'>('snapshot');
  const [newSessionName, setNewSessionName] = useState<string>('');
  const [isCreatingSession, setIsCreatingSession] = useState<boolean>(false);
  const [actionRef, setActionRef] = useState<string>('@e1');
  const [actionText, setActionText] = useState<string>('');
  const [isActionRunning, setIsActionRunning] = useState<boolean>(false);

  const activeTab = tabs.find((t) => t.id === activeTabId) ?? tabs[0] ?? null;

  const loadSessions = useCallback(async () => {
    setIsLoading(true);
    try {
      const sessList = await daemonClient.listBrowserSessions(workspaceId);
      setSessions(sessList);
      if (sessList.length > 0 && !activeSessionId) {
        setActiveSessionId(sessList[0].id);
      }
    } catch (err) {
      onShowToast?.(`Failed to load browser sessions: ${String(err)}`);
    } finally {
      setIsLoading(false);
    }
  }, [workspaceId, activeSessionId, onShowToast]);

  const loadTabs = useCallback(async (sessionId: string) => {
    try {
      const tabList = await daemonClient.listBrowserTabs(sessionId);
      setTabs(tabList);
      if (tabList.length > 0 && (!activeTabId || !tabList.some((t) => t.id === activeTabId))) {
        setActiveTabId(tabList[0].id);
        setUrlInput(tabList[0].url);
      }
    } catch (err) {
      onShowToast?.(`Failed to load browser tabs: ${String(err)}`);
    }
  }, [activeTabId, onShowToast]);

  useEffect(() => {
    loadSessions();
  }, [loadSessions]);

  useEffect(() => {
    if (activeSessionId) {
      loadTabs(activeSessionId);
    } else {
      setTabs([]);
      setActiveTabId(null);
    }
  }, [activeSessionId, loadTabs]);

  const handleCreateSession = async () => {
    const name = newSessionName.trim() || `Session ${sessions.length + 1}`;
    try {
      const sess = await daemonClient.createBrowserSession(name, workspaceId);
      setSessions((prev) => [sess, ...prev]);
      setActiveSessionId(sess.id);
      setNewSessionName('');
      setIsCreatingSession(false);
      onShowToast?.(`Browser session '${name}' created.`);
    } catch (err) {
      onShowToast?.(`Error creating session: ${String(err)}`);
    }
  };

  const handleCreateTab = async (url: string = 'about:blank') => {
    if (!activeSessionId) {
      onShowToast?.('Create or select a browser session first.');
      return;
    }
    try {
      const tab = await daemonClient.createBrowserTab(activeSessionId, url);
      setTabs((prev) => [...prev, tab]);
      setActiveTabId(tab.id);
      setUrlInput(tab.url);
      onShowToast?.(`Opened new tab: ${tab.url}`);
    } catch (err) {
      onShowToast?.(`Error creating tab: ${String(err)}`);
    }
  };

  const handleNavigate = async (e?: React.FormEvent) => {
    e?.preventDefault();
    if (!activeTabId || !urlInput.trim()) return;
    try {
      const updatedTab = await daemonClient.navigateBrowserTab(activeTabId, urlInput.trim());
      setTabs((prev) => prev.map((t) => (t.id === activeTabId ? updatedTab : t)));
      setUrlInput(updatedTab.url);
      onShowToast?.(`Navigated to: ${updatedTab.url}`);
    } catch (err) {
      onShowToast?.(`Navigation failed: ${String(err)}`);
    }
  };

  const handleCaptureSnapshot = async () => {
    if (!activeTabId) return;
    try {
      await daemonClient.snapshotBrowserTab({ tab_id: activeTabId } as any);
      if (activeSessionId) {
        await loadTabs(activeSessionId);
      }
      onShowToast?.('Page snapshot captured.');
    } catch (err) {
      onShowToast?.(`Snapshot failed: ${String(err)}`);
    }
  };

  const handleElementClick = async (refToClick: string) => {
    if (!activeTabId || !refToClick.trim()) return;
    setIsActionRunning(true);
    try {
      const updated = await daemonClient.clickBrowserTab(activeTabId, refToClick.trim());
      setTabs((prev) => prev.map((t) => (t.id === activeTabId ? updated : t)));
      onShowToast?.(`Clicked element ${refToClick}.`);
      if (activeSessionId) {
        await loadTabs(activeSessionId);
      }
    } catch (err) {
      onShowToast?.(`Click failed: ${String(err)}`);
    } finally {
      setIsActionRunning(false);
    }
  };

  const handleElementFill = async () => {
    if (!activeTabId || !actionRef.trim() || !actionText.trim()) return;
    setIsActionRunning(true);
    try {
      await daemonClient.fillBrowserTab(activeTabId, actionRef.trim(), actionText.trim());
      onShowToast?.(`Filled ${actionRef} with "${actionText}".`);
      setActionText('');
      if (activeSessionId) {
        await loadTabs(activeSessionId);
      }
    } catch (err) {
      onShowToast?.(`Fill failed: ${String(err)}`);
    } finally {
      setIsActionRunning(false);
    }
  };

  const handleCloseTab = async (tabId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await daemonClient.closeBrowserTab(tabId);
      const remaining = tabs.filter((t) => t.id !== tabId);
      setTabs(remaining);
      if (activeTabId === tabId) {
        if (remaining.length > 0) {
          setActiveTabId(remaining[0].id);
          setUrlInput(remaining[0].url);
        } else {
          setActiveTabId(null);
        }
      }
      onShowToast?.('Browser tab closed.');
    } catch (err) {
      onShowToast?.(`Error closing tab: ${String(err)}`);
    }
  };

  return (
    <div className="flex flex-col h-full bg-[#121214] text-zinc-200 text-xs overflow-hidden select-none font-sans">
      {/* Session Header / Controls Bar */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-zinc-800 bg-[#18181b]">
        <div className="flex items-center gap-2">
          <Globe size={14} className="text-emerald-400 shrink-0" />
          <span className="font-semibold text-zinc-100 tracking-wide text-[11px] uppercase">
            Scoped Browser Workbench
          </span>
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-400">{can('tabs.navigate') ? 'Navigation ready' : 'Saved tabs only'}</span>
        </div>

        {/* Sessions Selector & New Session */}
        <div className="flex items-center gap-2">
          {sessions.length > 0 ? (
            <NativeSelect
              value={activeSessionId || ''}
              onChange={(e) => setActiveSessionId(e.target.value)}
              className="bg-zinc-900 border border-zinc-700 text-zinc-200 rounded px-2 py-1 text-xs focus:outline-none focus:border-zinc-500"
            >
              {sessions.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name} ({s.tabs?.length ?? 0} tabs)
                </option>
              ))}
            </NativeSelect>
          ) : (
            <span className="text-zinc-500 text-[11px]">No active sessions</span>
          )}

          {isCreatingSession ? (
            <div className="flex items-center gap-1">
              <input
                type="text"
                placeholder="Session name"
                value={newSessionName}
                onChange={(e) => setNewSessionName(e.target.value)}
                className="bg-zinc-900 border border-zinc-700 text-zinc-200 rounded px-2 py-0.5 text-xs w-28 focus:outline-none"
                autoFocus
              />
              <button
                onClick={handleCreateSession}
                className="px-2 py-0.5 bg-zinc-700 hover:bg-zinc-600 rounded text-zinc-100 text-[11px]"
              >
                Save
              </button>
              <button
                onClick={() => setIsCreatingSession(false)}
                className="px-1 py-0.5 text-zinc-400 hover:text-zinc-200"
              >
                <X size={12} />
              </button>
            </div>
          ) : (
            <button
              disabled={!can('sessions.create')}
              onClick={() => setIsCreatingSession(true)}
              title={can('sessions.create') ? 'Create browser session' : 'Session creation is not available'}
              className="flex items-center gap-1 px-2 py-1 bg-zinc-800 hover:bg-zinc-700 disabled:text-zinc-500 disabled:cursor-not-allowed border border-zinc-700 rounded text-zinc-300 text-[11px] transition"
            >
              <Plus size={11} />
              <span>New Session</span>
            </button>
          )}
        </div>
      </div>

      {/* Fail-closed Degraded Status Notification */}
      <div className="bg-amber-500/10 border-b border-amber-500/20 px-3 py-1.5 flex items-center justify-between text-[11px] text-amber-400 shrink-0">
        <div className="flex items-center gap-1.5">
          <span className="h-1.5 w-1.5 rounded-full bg-amber-400 animate-pulse" />
          <span>{capabilityReason || 'Browser execution is degraded; only advertised operations are enabled.'}</span>
        </div>
        <span className="font-mono text-[10px] text-amber-400/80">Fail-Closed</span>
      </div>

      {/* Tabs Strip */}
      <div className="flex items-center gap-1 px-2 pt-1 border-b border-zinc-800 bg-[#151518] overflow-x-auto no-scrollbar">
        {tabs.map((tab) => {
          const isActive = tab.id === activeTabId;
          return (
            <div
              key={tab.id}
              onClick={() => {
                setActiveTabId(tab.id);
                setUrlInput(tab.url);
              }}
              className={`flex items-center gap-2 px-3 py-1.5 rounded-t-md text-xs cursor-pointer border-t border-x transition max-w-[200px] truncate ${
                isActive
                  ? 'bg-[#18181b] border-zinc-700 text-zinc-100 font-medium'
                  : 'bg-zinc-900/40 border-transparent text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40'
              }`}
            >
              <span className="truncate">{tab.title || tab.url}</span>
              <button
                disabled={!can('tabs.close')}
                onClick={(e) => handleCloseTab(tab.id, e)}
                title={can('tabs.close') ? 'Close tab' : 'Tab closing is not available'}
                className="hover:text-red-400 disabled:hover:text-zinc-500 disabled:cursor-not-allowed text-zinc-500 rounded p-0.5"
              >
                <X size={11} />
              </button>
            </div>
          );
        })}
        {activeSessionId && (
          <button
            disabled={!can('tabs.create')}
            onClick={() => handleCreateTab('about:blank')}
            className="p-1 text-zinc-500 hover:text-zinc-200 disabled:hover:text-zinc-500 disabled:cursor-not-allowed rounded hover:bg-zinc-800 transition mb-0.5"
            title="Open New Tab"
          >
            <Plus size={13} />
          </button>
        )}
      </div>

      {/* Address & Navigation Bar */}
      <div className="flex items-center gap-2 px-3 py-2 bg-[#18181b] border-b border-zinc-800">
        <button
          onClick={() => handleNavigate()}
          disabled={!can('tabs.navigate')}
          className="p-1.5 rounded bg-zinc-800/50 text-zinc-400 enabled:hover:text-zinc-100 disabled:cursor-not-allowed transition"
          title={can('tabs.navigate') ? 'Reload page' : 'Browser navigation is not connected'}
        >
          <RotateCw size={12} />
        </button>

        <form onSubmit={handleNavigate} className="flex-1 flex items-center relative">
          <input
            type="text"
            value={urlInput}
            onChange={(e) => setUrlInput(e.target.value)}
            disabled={!can('tabs.navigate')}
            placeholder={can('tabs.navigate') ? 'Enter a URL' : 'Navigation is not connected'}
            className="w-full bg-zinc-950/60 border border-zinc-800 rounded-md px-3 py-1 text-xs text-zinc-200 font-mono tracking-tight focus:outline-none disabled:text-zinc-500 disabled:cursor-not-allowed"
          />
          <button
            type="submit"
            disabled={!can('tabs.navigate')}
            title={can('tabs.navigate') ? 'Navigate' : 'Browser navigation is not connected'}
            className="absolute right-1 px-2 py-0.5 bg-zinc-800 border border-zinc-700 text-zinc-400 rounded text-[11px] enabled:hover:text-zinc-100 disabled:cursor-not-allowed"
          >
            <ArrowRight size={11} />
          </button>
        </form>

        <button
          onClick={handleCaptureSnapshot}
          disabled={!can('tabs.snapshot')}
          title={can('tabs.snapshot') ? 'Capture DOM snapshot' : 'DOM capture is not connected'}
          className="flex items-center gap-1.5 px-2.5 py-1 bg-zinc-800/50 border border-zinc-800 rounded text-zinc-400 font-medium text-[11px] enabled:hover:text-zinc-100 disabled:cursor-not-allowed"
        >
          <Camera size={12} className="text-zinc-500" />
          <span>DOM Snapshot</span>
        </button>
      </div>

      {/* Main Viewport & Drawer Container */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left: Webpage Canvas Viewport */}
        <div className="flex-1 flex flex-col bg-zinc-950 overflow-y-auto p-4 border-r border-zinc-800">
          {isLoading ? (
            <div className="flex-1 flex items-center justify-center text-zinc-500">
              <span className="animate-pulse">Loading browser sessions...</span>
            </div>
          ) : !activeTab ? (
            <div className="flex-1 flex flex-col items-center justify-center text-center p-8 space-y-3">
              <Globe size={32} className="text-zinc-700" />
              <p className="text-zinc-400 font-medium text-xs">No active browser tab open.</p>
              <p className="text-zinc-600 text-[11px] max-w-sm">
                {can('tabs.navigate') ? 'Open a tab to browse scoped documentation and inspect the page.' : 'Saved browser tabs are available. Navigation and page capture require a browser adapter.'}
              </p>
              {activeSessionId ? (
                <button
                  disabled={!can('tabs.create')}
                  onClick={() => handleCreateTab('about:blank')}
                  className="px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 disabled:text-zinc-500 disabled:cursor-not-allowed border border-zinc-700 rounded text-zinc-200 text-xs font-medium transition"
                >
                  Open First Tab
                </button>
              ) : (
                <button
                  disabled={!can('sessions.create')}
                  onClick={handleCreateSession}
                  className="px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 disabled:text-zinc-500 disabled:cursor-not-allowed border border-zinc-700 rounded text-zinc-200 text-xs font-medium transition"
                >
                  Create Initial Session
                </button>
              )}
            </div>
          ) : (
            <div className="space-y-4">
              {/* Page Identity Header */}
              <div className="flex items-center justify-between p-3 rounded-lg bg-zinc-900/60 border border-zinc-800">
                <div className="space-y-1">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-zinc-200 text-xs">{activeTab.title}</span>
                    <span className="px-1.5 py-0.5 rounded text-[10px] bg-emerald-950/60 text-emerald-400 border border-emerald-800/40">
                      {activeTab.status}
                    </span>
                  </div>
                  <p className="text-[11px] text-zinc-500 font-mono truncate">{activeTab.url}</p>
                </div>
                <div className="flex items-center gap-1.5 text-zinc-400 text-[11px]">
                  <span>{can('tabs.navigate') ? 'Browser transport connected' : 'Page content unavailable'}</span>
                </div>
              </div>

              {/* Interactive Web Viewport */}
              <div className="rounded-lg bg-zinc-950 border border-zinc-800 overflow-hidden flex flex-col h-[480px]">
                <div className="flex items-center justify-between px-3 py-1.5 bg-zinc-900 border-b border-zinc-800 text-zinc-400 text-[10px]">
                  <div className="flex items-center gap-2">
                    <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
                    <span className="font-sans font-medium text-zinc-300">Live Viewport</span>
                  </div>
                  <div className="flex items-center gap-3">
                    <span className="font-mono text-zinc-400">1280 × 800</span>
                    {activeTab.url && activeTab.url !== 'about:blank' && (
                      <a
                        href={activeTab.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-zinc-400 hover:text-amber-400 underline font-sans"
                      >
                        External Tab
                      </a>
                    )}
                  </div>
                </div>
                {activeTab.url.startsWith('http://') || activeTab.url.startsWith('https://') ? (
                  <iframe
                    key={activeTab.url}
                    src={activeTab.url}
                    title={activeTab.title}
                    className="w-full flex-1 bg-white border-0"
                    sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-modals"
                  />
                ) : (
                  <div className="flex-1 flex flex-col items-center justify-center p-6 text-center text-zinc-500 space-y-2">
                    <Globe size={32} className="text-zinc-600" />
                    <p className="text-xs font-medium text-zinc-400">Enter a URL in the address bar above</p>
                    <p className="text-[11px] text-zinc-500">
                      Supports local development servers (e.g., http://localhost:5173, http://localhost:3000) and external websites.
                    </p>
                  </div>
                )}
              </div>

              {/* Rendered Snapshot Summary */}
              <div className="rounded-lg bg-zinc-900/40 border border-zinc-800 p-4 space-y-2 font-mono text-[11px]">
                <div className="flex items-center justify-between text-zinc-400 border-b border-zinc-800 pb-1.5">
                  <span className="text-[10px] uppercase font-sans tracking-wider text-zinc-500">
                    DOM Tree &amp; Semantic Summary
                  </span>
                  <span className="text-zinc-500 text-[10px]">
                    Links: {activeTab.last_snapshot?.links?.length ?? 0}
                  </span>
                </div>
                <div className="p-3 bg-zinc-950 rounded border border-zinc-900 text-zinc-300 whitespace-pre-wrap leading-relaxed text-xs max-h-36 overflow-y-auto">
                  {activeTab.last_snapshot?.dom_tree_summary || (
                    <span className="text-zinc-600 font-sans">
                      No DOM snapshot recorded yet. Click &apos;DOM Snapshot&apos; above to record page elements into persistence.
                    </span>
                  )}
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Right Drawer: Inspection Tools (Snapshot / Console / Network) */}
        <div className="w-80 flex flex-col bg-[#161619] border-l border-zinc-800">
          {/* Drawer Selector Tabs */}
          <div className="flex items-center border-b border-zinc-800 bg-[#18181b]">
            <button
              onClick={() => setActiveDrawer('snapshot')}
              className={`flex-1 flex items-center justify-center gap-1.5 py-2 text-[11px] font-medium border-b-2 transition ${
                activeDrawer === 'snapshot'
                  ? 'border-amber-400 text-zinc-100 bg-zinc-800/30'
                  : 'border-transparent text-zinc-400 hover:text-zinc-200'
              }`}
            >
              <Layers size={12} />
              <span>Snapshot</span>
            </button>
            <button
              onClick={() => setActiveDrawer('console')}
              className={`flex-1 flex items-center justify-center gap-1.5 py-2 text-[11px] font-medium border-b-2 transition ${
                activeDrawer === 'console'
                  ? 'border-amber-400 text-zinc-100 bg-zinc-800/30'
                  : 'border-transparent text-zinc-400 hover:text-zinc-200'
              }`}
            >
              <Terminal size={12} />
              <span>Console</span>
            </button>
            <button
              onClick={() => setActiveDrawer('network')}
              className={`flex-1 flex items-center justify-center gap-1.5 py-2 text-[11px] font-medium border-b-2 transition ${
                activeDrawer === 'network'
                  ? 'border-amber-400 text-zinc-100 bg-zinc-800/30'
                  : 'border-transparent text-zinc-400 hover:text-zinc-200'
              }`}
            >
              <Activity size={12} />
              <span>Network</span>
            </button>
          </div>

          {/* Drawer Content Area */}
          <div className="flex-1 overflow-y-auto p-3 space-y-3 font-mono text-[11px]">
            {activeDrawer === 'snapshot' && (
              <div className="space-y-3 font-sans">
                <span className="text-zinc-400 font-semibold text-[11px] block">
                  Latest Captured Snapshot
                </span>
                {activeTab?.last_snapshot ? (
                  <div className="space-y-2">
                    {/* Element Interaction Tool */}
                    <div className="p-2.5 rounded bg-zinc-900 border border-zinc-800 space-y-2">
                      <span className="text-amber-400 font-semibold text-[10px] uppercase tracking-wider block">
                        Agent AX Actions
                      </span>
                      <div className="flex items-center gap-1.5">
                        <input
                          type="text"
                          value={actionRef}
                          onChange={(e) => setActionRef(e.target.value)}
                          disabled={!can('tabs.click')}
                          placeholder="@e1"
                          className="w-16 bg-zinc-950 border border-zinc-800 rounded px-1.5 py-1 text-center font-mono text-[11px] text-zinc-500 cursor-not-allowed"
                        />
                        <button
                          disabled={!can('tabs.click') || isActionRunning}
                          onClick={() => handleElementClick(actionRef)}
                          title="Click unavailable until the governed browser adapter is connected"
                          className="flex-1 py-1 px-2 bg-zinc-800 text-zinc-500 cursor-not-allowed rounded text-[10px] font-medium transition"
                        >
                          {isActionRunning ? 'Running...' : 'Click Ref'}
                        </button>
                      </div>
                      <div className="flex items-center gap-1.5">
                        <input
                          type="text"
                          value={actionText}
                          onChange={(e) => setActionText(e.target.value)}
                          disabled={!can('tabs.fill')}
                          placeholder="Text to fill..."
                          className="flex-1 bg-zinc-950 border border-zinc-800 rounded px-2 py-1 font-mono text-[11px] text-zinc-500 cursor-not-allowed"
                        />
                        <button
                          disabled={!can('tabs.fill') || isActionRunning || !actionText.trim()}
                          onClick={handleElementFill}
                          title="Fill unavailable until the governed browser adapter is connected"
                          className="py-1 px-2 bg-zinc-800 text-zinc-500 cursor-not-allowed rounded text-[10px] font-medium transition"
                        >
                          Fill
                        </button>
                      </div>
                    </div>

                    {activeTab.last_snapshot.screenshot_uri && (
                      <div className="p-2.5 rounded bg-zinc-900 border border-zinc-800 space-y-1">
                        <span className="text-zinc-500 text-[10px] block">Live Screenshot</span>
                        <p className="text-zinc-400 font-mono text-[10px] truncate">
                          {activeTab.last_snapshot.screenshot_uri}
                        </p>
                      </div>
                    )}

                    <div className="p-2.5 rounded bg-zinc-900 border border-zinc-800 space-y-1">
                      <span className="text-zinc-500 text-[10px] block">Interactive Elements (@refs)</span>
                      <div className="space-y-1 max-h-48 overflow-y-auto">
                        {activeTab.last_snapshot.links.map((link, idx) => (
                          <div
                            key={idx}
                            onClick={() => {
                              const match = link.match(/^(@\w+)/);
                              if (match) setActionRef(match[1]);
                            }}
                            className="p-1 rounded bg-zinc-950/60 hover:bg-zinc-800 border border-zinc-850 hover:border-amber-500/40 text-emerald-400 font-mono text-[10px] cursor-pointer transition truncate"
                            title="Click to select this ref"
                          >
                            {link}
                          </div>
                        ))}
                      </div>
                    </div>
                  </div>
                ) : (
                  <div className="text-center p-6 text-zinc-600 text-xs">
                    No snapshot recorded for this tab.
                  </div>
                )}
              </div>
            )}

            {activeDrawer === 'console' && (
              <div className="space-y-2 font-mono">
                <span className="text-zinc-400 font-semibold text-[11px] block font-sans">
                  Console Output
                </span>
                {activeTab && activeTab.console_logs && activeTab.console_logs.length > 0 ? (
                  activeTab.console_logs.map((log, idx) => (
                    <div
                      key={idx}
                      className="p-1.5 rounded bg-zinc-900 border border-zinc-800 text-[10px] flex items-start gap-1"
                    >
                      <span className="text-zinc-500">[{log.level}]</span>
                      <span className="text-zinc-300">{log.message}</span>
                    </div>
                  ))
                ) : (
                  <div className="text-zinc-600 text-[11px] font-sans text-center p-4">
                    Console stream clean. Zero errors detected.
                  </div>
                )}
              </div>
            )}

            {activeDrawer === 'network' && (
              <div className="space-y-2 font-mono">
                <span className="text-zinc-400 font-semibold text-[11px] block font-sans">
                  Network Requests Ledger
                </span>
                {activeTab && activeTab.network_requests && activeTab.network_requests.length > 0 ? (
                  activeTab.network_requests.map((req, idx) => (
                    <div
                      key={idx}
                      className="p-1.5 rounded bg-zinc-900 border border-zinc-800 text-[10px] space-y-0.5"
                    >
                      <div className="flex justify-between items-center text-zinc-400">
                        <span className="text-emerald-400 font-bold">{req.method}</span>
                        <span>{req.status ?? 200} OK</span>
                      </div>
                      <div className="text-zinc-300 truncate">{req.url}</div>
                      <div className="text-zinc-500 text-[9px]">{req.duration_ms}ms</div>
                    </div>
                  ))
                ) : (
                  <div className="text-zinc-600 text-[11px] font-sans text-center p-4">
                    No network activity recorded.
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
