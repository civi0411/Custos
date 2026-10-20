import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  AlertCircle,
  BookOpen,
  CheckSquare,
  GitBranch,
  HardDrive,
  ListChecks,
  ShieldCheck,
  SlidersHorizontal,
  X,
  Zap,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import type { OrcaTab, OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';
import type { ExecutionWorkspace, Task } from '@/types/domain';
import type { Session } from '@/types';

const DEFAULT_WIDTH = 300;
const MIN_WIDTH = 260;
const MAX_WIDTH = 480;
export type RightSidebarTab = 'inspector' | 'checks' | 'governance';

interface TabConfig {
  id: RightSidebarTab;
  label: string;
  icon: React.ElementType;
  title: string;
}

export const getTabsForMode = (workbenchMode: string): TabConfig[] => {
  if (workbenchMode === 'research') {
    return [
      { id: 'inspector', label: 'Context', icon: SlidersHorizontal, title: 'Selected task and research resource' },
      { id: 'checks', label: 'Evidence', icon: ListChecks, title: 'Evidence availability and Findings' },
    ];
  }
  if (workbenchMode === 'copilot') {
    return [
      { id: 'inspector', label: 'Context', icon: SlidersHorizontal, title: 'Selected task and conversation' },
      { id: 'governance', label: 'Outbox', icon: ShieldCheck, title: 'Open delivery and approval records' },
    ];
  }
  return [{ id: 'inspector', label: 'Context', icon: SlidersHorizontal, title: 'Selected worktree, task and resource' }];
};

export interface RightSidebarProps {
  isOpen: boolean;
  onClose: () => void;
  activeWorkspace?: ExecutionWorkspace | null;
  activeSession?: Session;
  activeTask?: Task | null;
  activeTab?: OrcaTab;
  onOpenTab?: (id: OrcaTabId, customUrl?: string, customTitle?: string) => void;
  mode?: string;
}

export const RightSidebar: React.FC<RightSidebarProps> = ({
  isOpen,
  onClose,
  activeWorkspace,
  activeSession,
  activeTask,
  activeTab,
  onOpenTab,
  mode = 'engineering',
}) => {
  const tabs = useMemo(() => getTabsForMode(mode), [mode]);
  const [activeTabId, setActiveTabId] = useState<RightSidebarTab>(() => {
    try {
      const saved = localStorage.getItem('custos.right-sidebar-tab') as RightSidebarTab | null;
      return saved && tabs.some((tab) => tab.id === saved) ? saved : tabs[0]?.id || 'inspector';
    } catch {
      return tabs[0]?.id || 'inspector';
    }
  });
  const [width, setWidth] = useState(() => {
    try {
      const saved = Number(localStorage.getItem('custos.right-sidebar-width'));
      return saved >= MIN_WIDTH && saved <= MAX_WIDTH ? saved : DEFAULT_WIDTH;
    } catch {
      return DEFAULT_WIDTH;
    }
  });
  const widthRef = useRef(width);
  const dragging = useRef(false);
  const [sessionMetrics, setSessionMetrics] = useState<{
    total_tokens?: number;
    estimated_cost_usd?: number;
    active_runs?: number;
  } | null>(null);

  useEffect(() => {
    if (tabs.some((tab) => tab.id === activeTabId)) return;
    const next = tabs[0]?.id || 'inspector';
    setActiveTabId(next);
    try { localStorage.setItem('custos.right-sidebar-tab', next); } catch {}
  }, [tabs, activeTabId]);

  const selectTab = useCallback((tab: RightSidebarTab) => {
    setActiveTabId(tab);
    try { localStorage.setItem('custos.right-sidebar-tab', tab); } catch {}
  }, []);

  const resizeTo = useCallback((nextWidth: number) => {
    const bounded = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, nextWidth));
    widthRef.current = bounded;
    setWidth(bounded);
    window.dispatchEvent(new CustomEvent('custos:right-sidebar-resized', { detail: bounded }));
  }, []);

  useEffect(() => {
    const onPointerMove = (event: PointerEvent) => {
      if (!dragging.current) return;
      const panel = document.getElementById('custos-context-inspector');
      if (!panel) return;
      resizeTo(panel.getBoundingClientRect().right - event.clientX);
    };
    const onPointerUp = () => {
      if (!dragging.current) return;
      dragging.current = false;
      try { localStorage.setItem('custos.right-sidebar-width', String(widthRef.current)); } catch {}
    };
    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', onPointerUp);
    window.addEventListener('pointercancel', onPointerUp);
    return () => {
      window.removeEventListener('pointermove', onPointerMove);
      window.removeEventListener('pointerup', onPointerUp);
      window.removeEventListener('pointercancel', onPointerUp);
    };
  }, [resizeTo]);

  useEffect(() => {
    let current = true;
    setSessionMetrics(null);
    if (!isOpen || activeTabId !== 'inspector' || !activeSession?.id) return;
    daemonClient.getSessionMetrics(activeSession.id)
      .then((metrics) => { if (current) setSessionMetrics(metrics); })
      .catch(() => { if (current) setSessionMetrics(null); });
    return () => { current = false; };
  }, [isOpen, activeTabId, activeSession?.id]);

  const openResource = useCallback((id: OrcaTabId, title?: string) => {
    onOpenTab?.(id, undefined, title);
  }, [onOpenTab]);

  if (!isOpen) return null;

  const criteria = Array.isArray(activeTask?.contract?.criteria) ? activeTask.contract.criteria : [];
  const activeTabType = activeTab?.type || activeTab?.id;

  return (
    <aside
      id="custos-context-inspector"
      style={{ width }}
      className="relative flex h-full shrink-0 flex-col overflow-hidden border-l border-white/8 bg-[#101114] text-white/80"
      aria-label="Context Inspector"
    >
      <div
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize context inspector"
        aria-valuemin={MIN_WIDTH}
        aria-valuemax={MAX_WIDTH}
        aria-valuenow={width}
        tabIndex={0}
        className="absolute inset-y-0 left-0 z-40 w-1 cursor-col-resize outline-none hover:bg-workbench-accent focus-visible:bg-workbench-accent"
        title="Drag to resize; use Left/Right Arrow to adjust; Home/End for limits"
        onDoubleClick={() => {
          resizeTo(DEFAULT_WIDTH);
          try { localStorage.setItem('custos.right-sidebar-width', String(DEFAULT_WIDTH)); } catch {}
        }}
        onPointerDown={(event) => {
          event.preventDefault();
          dragging.current = true;
        }}
        onKeyDown={(event) => {
          if (event.key === 'ArrowLeft') { event.preventDefault(); resizeTo(width + 16); }
          else if (event.key === 'ArrowRight') { event.preventDefault(); resizeTo(width - 16); }
          else if (event.key === 'Home') { event.preventDefault(); resizeTo(MIN_WIDTH); }
          else if (event.key === 'End') { event.preventDefault(); resizeTo(MAX_WIDTH); }
          else return;
          try { localStorage.setItem('custos.right-sidebar-width', String(widthRef.current)); } catch {}
        }}
      />

      <header className="flex h-10 shrink-0 items-center justify-between gap-2 border-b border-white/8 bg-[#141518] px-2.5">
        <div className="min-w-0">
          <h2 id="inspector-title" className="truncate text-[11px] font-semibold text-white/85">Inspector</h2>
          <p className="truncate text-[10px] text-white/40">Support for the focused work</p>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="flex size-7 shrink-0 items-center justify-center rounded-md text-white/45 transition hover:bg-white/8 hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-workbench-accent"
          aria-label="Close Inspector"
          title="Close Inspector (⌥I)"
        >
          <X aria-hidden="true" className="size-3.5" />
        </button>
      </header>

      {tabs.length > 1 && (
        <div className="shrink-0 border-b border-white/8 px-2 py-1.5">
          <div className="flex items-center gap-1" role="tablist" aria-label="Inspector sections">
            {tabs.map((tab) => {
              const Icon = tab.icon;
              const selected = activeTabId === tab.id;
              return (
                <button
                  key={tab.id}
                  type="button"
                  role="tab"
                  id={`inspector-tab-${tab.id}`}
                  aria-controls="inspector-panel-body"
                  aria-selected={selected}
                  title={tab.title}
                  onClick={() => selectTab(tab.id)}
                  className={`flex h-8 min-w-0 flex-1 items-center justify-center gap-1.5 rounded-md px-2 text-[10.5px] transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-workbench-accent ${selected ? 'bg-white/10 font-medium text-white' : 'text-white/50 hover:bg-white/5 hover:text-white/85'}`}
                >
                  <Icon aria-hidden="true" className="size-3.5 shrink-0" />
                  <span className="truncate">{tab.label}</span>
                </button>
              );
            })}
          </div>
        </div>
      )}

      <div
        id="inspector-panel-body"
        role={tabs.length > 1 ? 'tabpanel' : 'region'}
        aria-labelledby={tabs.length > 1 ? `inspector-tab-${activeTabId}` : 'inspector-title'}
        className="min-h-0 flex-1 overflow-y-auto p-3 text-xs"
      >
        {activeTabId === 'inspector' && (
          <section className="space-y-3" aria-label="Focused resource context">
            <div className="rounded-lg border border-white/8 bg-white/[0.035] p-3">
              <div className="mb-2 flex items-center gap-2 text-[10px] font-medium uppercase tracking-wide text-white/45"><HardDrive aria-hidden="true" className="size-3.5" />Workspace</div>
              {activeWorkspace ? (
                <>
                  <p className="truncate text-xs font-medium text-white/85" title={activeWorkspace.name}>{activeWorkspace.name}</p>
                  {activeWorkspace.path && <p className="mt-1 break-all font-mono text-[10px] leading-relaxed text-white/45">{activeWorkspace.path}</p>}
                  {activeWorkspace.active_branch && <p className="mt-2 flex items-center gap-1.5 font-mono text-[10px] text-white/55"><GitBranch aria-hidden="true" className="size-3" />{activeWorkspace.active_branch}</p>}
                </>
              ) : <p className="text-[11px] text-white/50">This resource is not bound to a repository.</p>}
            </div>

            <div className="rounded-lg border border-white/8 bg-white/[0.035] p-3">
              <div className="mb-2 flex items-center gap-2 text-[10px] font-medium uppercase tracking-wide text-white/45"><CheckSquare aria-hidden="true" className="size-3.5" />Task</div>
              {activeTask ? (
                <>
                  <p className="text-xs font-medium text-white/85">{activeTask.title || 'Untitled task'}</p>
                  <p className="mt-1 text-[10px] text-white/45">Status: <span className="capitalize text-white/70">{activeTask.status}</span></p>
                  {activeTask.contract?.description && <p className="mt-2 line-clamp-3 text-[11px] leading-relaxed text-white/55">{activeTask.contract.description}</p>}
                </>
              ) : <p className="text-[11px] text-white/50">No Task is linked to the focused resource.</p>}
            </div>

            <div className="rounded-lg border border-white/8 bg-white/[0.035] p-3">
              <div className="mb-2 flex items-center gap-2 text-[10px] font-medium uppercase tracking-wide text-white/45"><Zap aria-hidden="true" className="size-3.5" />Conversation</div>
              {activeSession ? (
                <>
                  <p className="truncate text-xs font-medium text-white/85" title={activeSession.title}>{activeSession.title || 'Untitled conversation'}</p>
                  <div className="mt-2 grid grid-cols-2 gap-2 text-[10px]">
                    <div className="min-w-0"><span className="block text-white/40">Model</span><span className="block truncate text-white/70" title={activeSession.model || undefined}>{activeSession.model || 'Not reported'}</span></div>
                    <div><span className="block text-white/40">Tokens</span><span className="font-mono text-white/70">{sessionMetrics?.total_tokens == null ? 'Not reported' : sessionMetrics.total_tokens.toLocaleString()}</span></div>
                    {sessionMetrics?.estimated_cost_usd != null && <div className="col-span-2"><span className="block text-white/40">Estimated cost</span><span className="font-mono text-white/70">${sessionMetrics.estimated_cost_usd.toFixed(4)}</span></div>}
                  </div>
                </>
              ) : <p className="text-[11px] text-white/50">No conversation is focused.</p>}
            </div>

            {activeTab && (
              <div className="rounded-lg border border-white/8 bg-white/[0.035] p-3">
                <div className="mb-2 flex items-center gap-2 text-[10px] font-medium uppercase tracking-wide text-white/45"><BookOpen aria-hidden="true" className="size-3.5" />Focused resource</div>
                <p className="truncate text-xs font-medium text-white/85" title={activeTab.title}>{activeTab.title}</p>
                {activeTabType && <p className="mt-1 font-mono text-[10px] text-workbench-accent">{activeTabType}</p>}
                {activeTab.url && <p className="mt-2 break-all font-mono text-[9px] leading-relaxed text-white/40">{activeTab.url}</p>}
              </div>
            )}
          </section>
        )}

        {activeTabId === 'checks' && (
          <section className="space-y-3" aria-label="Research evidence support">
            <div className="rounded-lg border border-white/8 bg-white/[0.035] p-3">
              <div className="mb-2 flex items-center gap-2 text-[10px] font-medium uppercase tracking-wide text-white/45"><BookOpen aria-hidden="true" className="size-3.5" />Research context</div>
              {activeTab ? <p className="truncate text-xs font-medium text-white/85" title={activeTab.title}>{activeTab.title}</p> : <p className="text-[11px] text-white/50">No research resource is focused.</p>}
              {activeTask && <p className="mt-2 text-[10px] text-white/50">Task: <span className="text-white/70">{activeTask.title}</span></p>}
            </div>
            <div className="rounded-lg border border-white/8 bg-white/[0.035] p-3">
              <div className="mb-2 flex items-center gap-2 text-[10px] font-medium uppercase tracking-wide text-white/45"><ListChecks aria-hidden="true" className="size-3.5" />Declared criteria</div>
              {criteria.length > 0 ? <p className="text-[11px] leading-relaxed text-white/65">{criteria.length} criterion {criteria.length === 1 ? 'is' : 'are'} recorded on the active Task. This panel does not infer verification from the criteria list.</p> : <p className="text-[11px] leading-relaxed text-white/50">No Task criteria are available in this view. Open Findings to inspect the research records and their current evidence state.</p>}
            </div>
            <button type="button" onClick={() => openResource('findings', 'Findings')} className="flex min-h-9 w-full items-center justify-center gap-2 rounded-md border border-white/10 bg-white/[0.04] px-3 py-2 text-[11px] text-white/75 transition hover:bg-white/[0.08] hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-workbench-accent">
              <ListChecks aria-hidden="true" className="size-3.5 text-workbench-accent" />Open Findings in the work area
            </button>
          </section>
        )}

        {activeTabId === 'governance' && (
          <section className="space-y-3" aria-label="Copilot delivery support">
            <div className="rounded-lg border border-white/8 bg-white/[0.035] p-3">
              <div className="mb-2 flex items-center gap-2 text-[10px] font-medium uppercase tracking-wide text-white/45"><ShieldCheck aria-hidden="true" className="size-3.5" />Delivery status</div>
              {activeTask ? <><p className="text-xs font-medium text-white/85">{activeTask.title}</p><p className="mt-1 text-[10px] text-white/50">Task status: <span className="capitalize text-white/70">{activeTask.status}</span></p></> : <p className="text-[11px] leading-relaxed text-white/55">No Task is linked to this view. Exact delivery and approval records are inspected in Outbox.</p>}
            </div>
            <div className="flex items-start gap-2 rounded-lg border border-white/8 bg-white/[0.025] p-3 text-[10.5px] leading-relaxed text-white/55">
              <AlertCircle aria-hidden="true" className="mt-0.5 size-3.5 shrink-0 text-white/45" />
              <p>This Inspector cannot approve or send an action. It only links to the daemon-backed Outbox view.</p>
            </div>
            <button type="button" onClick={() => openResource('outbox', 'Outbox')} className="flex min-h-9 w-full items-center justify-center gap-2 rounded-md border border-white/10 bg-white/[0.04] px-3 py-2 text-[11px] text-white/75 transition hover:bg-white/[0.08] hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-workbench-accent">
              <ShieldCheck aria-hidden="true" className="size-3.5 text-workbench-accent" />Open Outbox in the work area
            </button>
          </section>
        )}
      </div>
    </aside>
  );
};
