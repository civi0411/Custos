import React from 'react';
import { Columns2, X, Plus, MessageSquare, Maximize2, Minimize2, MoreHorizontal } from 'lucide-react';
import { canvasGroups, canvasReducer, createCanvas, reconcileCanvas, restoreCanvas, type CanvasAction, type CanvasNode, type CanvasState, type CanvasGroup } from '@/types/canvas';
import { useScreenContext } from '@/context/ScreenContext';
import { ActionIcon } from '@/components/common';
import { WorkbenchEmptyState } from '@/components/common/WorkbenchPane';
import { ResourceContent, type OrcaTabbedContainerProps } from './ResourceContent';
import { DEFAULT_RESOURCES, findScopedResourceTab, getTabIcon, getTabType, useResourceCatalog, urlFor, type OrcaTab, type OrcaTabId } from './workbenchResources';
export * from './workbenchResources';
export { ResourceContent } from './ResourceContent';

export const OrcaTabbedContainer = React.memo(function OrcaTabbedContainer(props: OrcaTabbedContainerProps) {
  const { resourceTabs } = props;
  const {
    activeScreen,
    updateActiveScreenLayout,
    registerCanvasActions,
  } = useScreenContext();
  const catalog = useResourceCatalog();
  const scope = resourceTabs.scopeKey || 'default';
  const storageKey = `custos.canvas.v1:${scope}`;
  const tabIds = resourceTabs.tabs.map(tab => tab.id);
  const tabSignature = JSON.stringify(tabIds);
  const loadFallback = React.useCallback(() => {
    try { return restoreCanvas(JSON.parse(localStorage.getItem(storageKey) || 'null'), tabIds, resourceTabs.activeTabId); }
    catch { return createCanvas(tabIds, resourceTabs.activeTabId); }
  }, [storageKey, tabSignature, resourceTabs.activeTabId]);
  const [layouts, setLayouts] = React.useState<Record<string, CanvasState>>({});
  const activeCanvas = activeScreen?.canvasState || layouts[scope] || loadFallback();
  const state = reconcileCanvas(activeCanvas, tabIds, resourceTabs.activeTabId);
  const stateRef = React.useRef(state);
  stateRef.current = state;
  const dispatch = React.useCallback((action: CanvasAction) => {
    const next = canvasReducer(stateRef.current, action);
    if (activeScreen) {
      updateActiveScreenLayout(next);
    } else {
      setLayouts(previous => ({ ...previous, [scope]: next }));
    }
  }, [activeScreen, updateActiveScreenLayout, scope]);
  const resize = React.useRef<{ id: string; axis: 'horizontal' | 'vertical'; rect: DOMRect } | null>(null);
  const [dragTarget, setDragTarget] = React.useState<string | null>(null);
  const rootRef = React.useRef<HTMLDivElement>(null);
  const [visited, setVisited] = React.useState<Set<string>>(new Set());
  React.useEffect(() => {
    const visibleIds = canvasGroups(state.root).map(group => group.activeTabId).filter(Boolean);
    setVisited(previous => new Set([...previous, ...visibleIds].filter(id => tabIds.includes(id))));
  }, [scope, tabSignature, JSON.stringify(state.root)]);
  React.useEffect(() => {
    const move = (event: PointerEvent) => {
      const current = resize.current;
      if (!current) return;
      const ratio = current.axis === 'horizontal'
        ? (event.clientX - current.rect.left) / current.rect.width * 100
        : (event.clientY - current.rect.top) / current.rect.height * 100;
      dispatch({ type: 'resize', splitId: current.id, ratio });
    };
    const stop = () => { resize.current = null; document.body.classList.remove('canvas-resizing'); };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', stop);
    window.addEventListener('pointercancel', stop);
    return () => { window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', stop); window.removeEventListener('pointercancel', stop); stop(); };
  }, [dispatch]);
  const focus = (group: CanvasGroup, tabId = group.activeTabId) => {
    dispatch({ type: 'focus', groupId: group.id, tabId });
    resourceTabs.setActiveTabId(tabId);
    const targetTab = resourceTabs.tabs.find((t) => t.id === tabId);
    if (targetTab?.sessionId && targetTab.sessionId !== props.session?.id && props.onOpenSession) {
      props.onOpenSession(targetTab.sessionId);
    }
    if (targetTab?.taskId && targetTab.taskId !== props.selectedTaskId && props.onSelectTask) {
      props.onSelectTask(targetTab.taskId);
    }
  };
  const split = (group: CanvasGroup, axis: 'horizontal' | 'vertical', tabId?: string) => {
    let targetTabId = tabId;
    if (!targetTabId && group.tabIds.length > 1) {
      targetTabId = group.tabIds.find(id => id !== group.activeTabId) || group.tabIds[1];
    }
    const newGroupId = crypto.randomUUID();
    dispatch({ type: 'split', groupId: group.id, newGroupId, axis, tabId: targetTabId });
    if (targetTabId) {
      resourceTabs.setActiveTabId(targetTabId);
    }
  };


  type Rect = { x: number; y: number; w: number; h: number };
  const leaves: { group: CanvasGroup; rect: Rect }[] = [];
  const separators: { node: Extract<CanvasNode, { kind: 'split' }>; rect: Rect }[] = [];
  const walk = (node: CanvasNode, rect: Rect) => {
    if (node.kind === 'group') { leaves.push({ group: node, rect }); return; }
    separators.push({ node, rect });
    const ratio = node.ratio / 100;
    walk(node.first, node.axis === 'horizontal' ? { ...rect, w: rect.w * ratio } : { ...rect, h: rect.h * ratio });
    walk(node.second, node.axis === 'horizontal' ? { ...rect, x: rect.x + rect.w * ratio, w: rect.w * (1 - ratio) } : { ...rect, y: rect.y + rect.h * ratio, h: rect.h * (1 - ratio) });
  };
  const groups = canvasGroups(state.root);
  walk(state.root, { x: 0, y: 0, w: 100, h: 100 });
  const singleGroup = state.zoomedGroupId;
  if (singleGroup) {
    const displayed = groups.find(group => group.id === singleGroup) || groups[0];
    leaves.length = 0; separators.length = 0;
    walk(displayed, { x: 0, y: 0, w: 100, h: 100 });
  }
  const position = (rect: Rect) => ({ left: `${rect.x}%`, top: `${rect.y}%`, width: `${rect.w}%`, height: `${rect.h}%` });
  const isEngineeringMode = props.mode === 'code' || props.mode === 'engineering';
  const deskKinds: OrcaTabId[] = props.mode === 'research' ? ['lab', 'literature', 'findings', 'artifacts']
    : isEngineeringMode ? ['terminal', 'files'] : ['tasks', 'drafts'];
  const deskUnavailable = deskKinds.map(kind => catalog.resources.find(resource => resource.id === kind)).find(resource => !resource || resource.status === 'unavailable');
  const deskCapabilitiesReady = deskKinds.every(kind => catalog.resources.some(resource => resource.id === kind && resource.status !== 'unavailable'));
  const deskScopeReady = !!props.session?.id && !!props.activeWorkspaceId && !!props.workspaces?.some(workspace => workspace.id === props.activeWorkspaceId && workspace.status !== 'archived');
  const openDesk = () => {
    if (!deskScopeReady || !deskCapabilitiesReady) return;
    const nextTabs = [...resourceTabs.tabs];
    const selected = deskKinds.map(kind => {
      const scopeRef = { workspaceId: props.activeWorkspaceId, sessionId: props.session?.id, taskId: props.selectedTaskId || props.session?.taskId };
      const existing = findScopedResourceTab(nextTabs, kind, scopeRef);
      if (existing) return existing.id;
      const resource = DEFAULT_RESOURCES.find(item => item.id === kind);
      const tab: OrcaTab = { ...scopeRef, id: `${kind}-${crypto.randomUUID()}`, type: kind, title: resource?.title || kind, url: urlFor(kind) };
      nextTabs.push(tab); return tab.id;
    });
    resourceTabs.setTabs(nextTabs);
    const arranged = canvasReducer(
      reconcileCanvas(activeScreen?.canvasState || layouts[scope] || loadFallback(), nextTabs.map(tab => tab.id)),
      { type: 'arrange', tabIds: selected, layout: 'review' }
    );
    if (activeScreen) {
      updateActiveScreenLayout(arranged);
    } else {
      setLayouts(previous => ({ ...previous, [scope]: arranged }));
    }
    resourceTabs.setActiveTabId(selected[0]);
  };
  const close = (tabId: string) => {
    const owner = groups.find(group => group.tabIds.includes(tabId));
    // If this pane only has this 1 tab and there are multiple split panes, close the split pane!
    if (owner && owner.tabIds.length <= 1 && groups.length > 1) {
      dispatch({ type: 'removeGroup', groupId: owner.id });
    } else {
      dispatch({ type: 'close', tabId });
    }
    resourceTabs.setTabs(previous => previous.filter(tab => tab.id !== tabId));
    if (resourceTabs.activeTabId === tabId) {
      const remaining = owner?.tabIds.filter(id => id !== tabId) || [];
      const fallbackTab = remaining[0] || tabIds.find(id => id !== tabId) || '';
      resourceTabs.setActiveTabId(fallbackTab);
    }
  };

  React.useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && stateRef.current.zoomedGroupId) {
        event.preventDefault();
        dispatch({ type: 'zoom', groupId: stateRef.current.zoomedGroupId });
        return;
      }
      if (!(event.metaKey || event.ctrlKey)) return;
      const currentGroups = canvasGroups(stateRef.current.root);
      const focused = currentGroups.find(group => group.id === stateRef.current.focusedGroupId) || currentGroups[0];
      if (event.key === '\\') {
        event.preventDefault();
        split(focused, event.shiftKey ? 'vertical' : 'horizontal');
      } else if (event.key.toLowerCase() === 't' && !event.shiftKey) {
        event.preventDefault();
        if (focused) focus(focused);
        const btn = rootRef.current?.querySelector<HTMLButtonElement>(`[data-group-id="${focused.id}"] .canvas-tab-new`) || rootRef.current?.querySelector<HTMLButtonElement>('.canvas-tab-new');
        const rect = btn?.getBoundingClientRect();
        props.onOpenResourcePicker?.('primary', rect ? {
          top: rect.bottom + 4,
          left: Math.max(12, Math.min(rect.left, window.innerWidth - 316)),
        } : undefined);
      } else if (event.key.toLowerCase() === 'w' && !event.shiftKey) {
        event.preventDefault();
        if (focused) {
          if (focused.activeTabId) {
            close(focused.activeTabId);
          } else if (currentGroups.length > 1) {
            dispatch({ type: 'removeGroup', groupId: focused.id });
          }
        }
      } else if (event.altKey && ['ArrowLeft', 'ArrowRight'].includes(event.key)) {
        event.preventDefault();
        const index = currentGroups.findIndex(group => group.id === focused.id);
        const next = currentGroups[(index + (event.key === 'ArrowRight' ? 1 : currentGroups.length - 1)) % currentGroups.length];
        focus(next);
        rootRef.current?.querySelector<HTMLElement>(`[data-group-id="${next.id}"] [role="tab"]`)?.focus();
      }
    };
    const root = rootRef.current;
    root?.addEventListener('keydown', onKey);
    return () => root?.removeEventListener('keydown', onKey);
  }, [dispatch, resourceTabs.setActiveTabId, props.onOpenResourcePicker]);

  // Expose focused pane operations to AppTopBar ScreenSwitcherBar
  React.useEffect(() => {
    registerCanvasActions({
      splitRight: () => {
        const focusedGroup = groups.find((g) => g.id === state.focusedGroupId) || groups[0];
        if (focusedGroup) split(focusedGroup, 'horizontal');
      },
      splitDown: () => {
        const focusedGroup = groups.find((g) => g.id === state.focusedGroupId) || groups[0];
        if (focusedGroup) split(focusedGroup, 'vertical');
      },
      toggleZoom: () => {
        if (state.focusedGroupId) {
          dispatch({ type: 'zoom', groupId: state.focusedGroupId });
        }
      },
      isZoomed: Boolean(state.zoomedGroupId),
    });
    return () => registerCanvasActions(null);
  }, [registerCanvasActions, dispatch, scope, tabSignature, JSON.stringify(state.root), state.focusedGroupId, state.zoomedGroupId]);

  return (
    <div className="h-full min-h-0 flex flex-col bg-canvas">
      <div ref={rootRef} className="workspace-canvas flex-1 min-h-0 relative overflow-hidden bg-canvas" aria-label="Resource workspace">
        {leaves.map(({ group, rect }) => (
          <section key={group.id} data-group-id={group.id} className={`canvas-group absolute flex flex-col overflow-hidden ${state.focusedGroupId === group.id ? 'is-focused' : ''}`}
            style={position(rect)} aria-label={`Resource group ${groups.indexOf(group) + 1}`}
            onPointerDown={() => focus(group)}
            onDragOver={event => { if (event.dataTransfer.types.includes('application/custos-tab-id')) { event.preventDefault(); setDragTarget(group.id); } }}
            onDragLeave={event => { if (!event.currentTarget.contains(event.relatedTarget as Node)) setDragTarget(null); }}
            onDrop={event => {
              const id = event.dataTransfer.getData('application/custos-tab-id');
              setDragTarget(null); if (!tabIds.includes(id)) return;
              event.preventDefault();
              const bounds = event.currentTarget.getBoundingClientRect();
              if (event.clientY > bounds.bottom - bounds.height * .22) split(group, 'vertical', id);
              else if (event.clientX > bounds.right - bounds.width * .22) split(group, 'horizontal', id);
              else dispatch({ type: 'move', tabId: id, groupId: group.id });
              resourceTabs.setActiveTabId(id);
            }}>
            <header className="canvas-tabbar flex shrink-0 items-center gap-1 px-1">
              <div role="tablist" aria-label="Resources" className="flex flex-1 min-w-0 items-center overflow-x-auto no-scrollbar py-0.5">
                {group.tabIds.map(id => {
                  const tab = resourceTabs.tabs.find(item => item.id === id);
                  if (!tab) return null;
                  const isChatTab = tab.type === 'conversation' || tab.type === 'chat';
                  const tabSession = isChatTab
                    ? (tab.sessionId ? props.allSessions?.find((s) => s.id === tab.sessionId) : (tab.id === 'conversation' ? props.session : null))
                    : null;
                  const tabTitle = tabSession?.title || tab.title;
                  const tabWs = tab.workspaceId ? props.workspaces?.find((w: import('@/types/domain').ExecutionWorkspace) => w.id === tab.workspaceId) : null;
                  const wsBadge = tabWs?.kind?.type === 'git' ? tabWs.kind.branch : (tabWs?.lineage?.target_branch || tabWs?.name);
                  const fullTabTitle = wsBadge ? `[${wsBadge}] ${tabTitle}` : tabTitle;
                  const isActive = group.activeTabId === id;
                  return <div key={id} className={`canvas-tab group flex items-center shrink-0 ${isActive ? 'is-active' : ''}`} draggable
                    onDragStart={event => { event.dataTransfer.setData('application/custos-tab-id', id); event.dataTransfer.effectAllowed = 'move'; }}
                    onDragEnd={() => setDragTarget(null)}
                    onDragOver={event => { if (event.dataTransfer.types.includes('application/custos-tab-id')) event.preventDefault(); }}
                    onDrop={event => { const moved = event.dataTransfer.getData('application/custos-tab-id'); if (!tabIds.includes(moved)) return; event.stopPropagation(); event.preventDefault(); setDragTarget(null); dispatch({ type: 'move', tabId: moved, groupId: group.id, beforeTabId: id }); focus(group, moved); }}>
                    <button type="button" role="tab" id={`tab-${id}`} aria-selected={isActive} aria-controls={`resource-${id}`} tabIndex={isActive ? 0 : -1}
                      className="flex items-center gap-1.5 px-2.5 h-full min-w-0 select-none text-left" title={fullTabTitle} onClick={() => focus(group, id)}
                      onKeyDown={event => {
                        if (event.altKey || event.metaKey || event.ctrlKey) return;
                        if (!['ArrowRight', 'ArrowLeft', 'Home', 'End', 'Delete'].includes(event.key)) return;
                        event.preventDefault(); event.stopPropagation();
                        const index = group.tabIds.indexOf(id);
                        if (event.key === 'Delete') {
                          const next = group.tabIds[index + 1] || group.tabIds[index - 1];
                          close(id);
                          if (next) { focus(group, next); requestAnimationFrame(() => document.getElementById(`tab-${next}`)?.focus()); }
                          else requestAnimationFrame(() => rootRef.current?.querySelector<HTMLButtonElement>('[data-group-id] button[aria-label="Open resource in group"]')?.focus());
                          return;
                        }
                        const next = group.tabIds[event.key === 'Home' ? 0 : event.key === 'End' ? group.tabIds.length - 1
                          : (index + (event.key === 'ArrowRight' ? 1 : group.tabIds.length - 1)) % group.tabIds.length];
                        focus(group, next); document.getElementById(`tab-${next}`)?.focus();
                      }}>
                      {getTabIcon(getTabType(tab), isActive, tab)}
                      <span className={`truncate max-w-40 text-[11px] ${isActive ? 'font-semibold text-white' : 'font-normal text-white/60'}`}>{tabTitle}</span>
                      {wsBadge && (
                        <span className={`font-mono text-[9px] px-1 py-0.2 rounded shrink-0 ${isActive ? 'bg-white/10 text-white/80 border border-white/15' : 'bg-white/5 text-white/40 border border-white/5'}`}>
                          {wsBadge}
                        </span>
                      )}
                    </button>
                    <button
                      type="button"
                      className="canvas-tab-close mr-1"
                      aria-label={`Close ${tabTitle}`}
                      onClick={event => { event.stopPropagation(); close(id); }}
                    >
                      <X size={11} />
                    </button>
                  </div>;
                })}

                {/* Browser-style '+' button directly next to the displayed tabs */}
                <button
                  type="button"
                  className="canvas-tab-new ml-1 cursor-pointer"
                  title="New Tab (⌘T)"
                  aria-label="Open resource in group"
                  onClick={(e) => {
                    e.stopPropagation();
                    focus(group);
                    const rect = e.currentTarget.getBoundingClientRect();
                    props.onOpenResourcePicker?.('primary', {
                      top: rect.bottom + 4,
                      left: Math.max(12, Math.min(rect.left, window.innerWidth - 316)),
                    });
                  }}
                >
                  <Plus size={14} />
                </button>
              </div>

              {/* Pane window controls */}
              <div className="flex items-center gap-0.5 shrink-0 pl-1 border-l border-white/5">
                <ActionIcon
                  icon={Columns2}
                  tooltip="Chia ô sang phải (⌘\)"
                  size="xs"
                  onClick={() => split(group, 'horizontal')}
                  ariaLabel="Split group right"
                />
                <ActionIcon
                  icon={state.zoomedGroupId ? Minimize2 : Maximize2}
                  tooltip={state.zoomedGroupId ? "Thu nhỏ về kích thước cũ (⇧⌘⏎)" : "Phóng to khung này (⇧⌘⏎)"}
                  size="xs"
                  active={Boolean(state.zoomedGroupId)}
                  onClick={() => dispatch({ type: 'zoom', groupId: group.id })}
                  ariaLabel="Zoom or restore group"
                />
                {groups.length > 1 && (
                  <ActionIcon
                    icon={X}
                    tooltip="Đóng ô chia này (Close pane)"
                    size="xs"
                    onClick={() => dispatch({ type: 'removeGroup', groupId: group.id })}
                    ariaLabel="Close pane"
                  />
                )}
                <details
                  className="canvas-layout-menu"
                  onKeyDown={event => {
                    if (event.key === 'Escape') {
                      event.preventDefault();
                      event.currentTarget.removeAttribute('open');
                      event.currentTarget.querySelector<HTMLElement>('summary')?.focus();
                    }
                  }}
                  onBlur={event => {
                    if (!event.currentTarget.contains(event.relatedTarget as Node)) {
                      event.currentTarget.removeAttribute('open');
                    }
                  }}
                >
                  <summary className="canvas-icon" aria-label="More layout actions" title="More layout actions">
                    <MoreHorizontal size={14} />
                  </summary>
                  <div className="canvas-layout-popover">
                    <p className="text-[10px] uppercase tracking-wider text-fg-muted px-2 py-1">Layout</p>
                    <button
                      type="button"
                      onClick={(event) => {
                        split(group, 'vertical');
                        event.currentTarget.closest('details')?.removeAttribute('open');
                      }}
                    >
                      Chia ô xuống dưới (Split Down)
                    </button>
                    {props.onToggleExpand && (
                      <button
                        type="button"
                        onClick={(event) => {
                          props.onToggleExpand?.();
                          event.currentTarget.closest('details')?.removeAttribute('open');
                        }}
                      >
                        {props.isExpanded ? 'Hiện hội thoại' : 'Mở rộng không gian'}
                      </button>
                    )}
                    {groups.length > 1 && (
                      <>
                        <div className="h-px bg-border-default my-1" />
                        {group.activeTabId && groups.filter(item => item.id !== group.id).map(item => (
                          <button
                            key={item.id}
                            type="button"
                            onClick={(event) => {
                              dispatch({ type: 'move', tabId: group.activeTabId, groupId: item.id });
                              resourceTabs.setActiveTabId(group.activeTabId);
                              event.currentTarget.closest('details')?.removeAttribute('open');
                            }}
                          >
                            Chuyển tab sang ô bên cạnh
                          </button>
                        ))}
                        <button
                          type="button"
                          onClick={(event) => {
                            dispatch({ type: 'merge' });
                            event.currentTarget.closest('details')?.removeAttribute('open');
                          }}
                        >
                          Gộp tất cả về 1 khung (Merge)
                        </button>
                        <button
                          type="button"
                          className="text-red-400 hover:text-red-300"
                          onClick={(event) => {
                            dispatch({ type: 'removeGroup', groupId: group.id });
                            event.currentTarget.closest('details')?.removeAttribute('open');
                          }}
                        >
                          Đóng ô chia này (Close Pane)
                        </button>
                      </>
                    )}
                    {!isEngineeringMode && (
                      <>
                        <div className="h-px bg-border-default my-1" />
                        <button
                          type="button"
                          disabled={!deskScopeReady || !deskCapabilitiesReady}
                          title={!deskScopeReady ? 'Select a conversation and workspace first' : !deskCapabilitiesReady ? deskUnavailable?.reason || 'Some desk resources are unavailable' : 'Arrange existing resources without starting execution'}
                          onClick={event => {
                            openDesk();
                            event.currentTarget.closest('details')?.removeAttribute('open');
                          }}
                        >
                          Arrange full {props.mode === 'research' ? 'Research' : 'Copilot'} desk
                        </button>
                      </>
                    )}
                  </div>
                </details>
              </div>
            </header>
            {!group.activeTabId && <WorkbenchEmptyState
              title={props.mode === 'research' ? 'Research workspace' : isEngineeringMode ? 'Coding workspace' : 'Copilot workspace'}
              description="Open a conversation, document or working resource here. You can split this space when you need another view."
              action={<>
                {props.onNewConversation && <button type="button" className="canvas-open-button" onClick={props.onNewConversation}><MessageSquare size={13} />New conversation</button>}
                <button type="button" className="canvas-open-button" onClick={() => { focus(group); props.onOpenResourcePicker?.('primary'); }}><Plus size={13} />Open resource</button>
              </>}
            />}
            {dragTarget === group.id && <div className="absolute inset-x-1 bottom-1 top-9 z-40 pointer-events-none rounded border-2 border-dashed border-accent bg-accent/10 flex items-center justify-center text-xs text-fg-default">Drop to move · right edge to split · bottom edge to stack</div>}
          </section>
        ))}
        {resourceTabs.tabs.filter(tab => visited.has(tab.id) || leaves.some(({ group }) => group.activeTabId === tab.id)).map(tab => {
          const leaf = leaves.find(({ group }) => group.activeTabId === tab.id);
          return <div key={`${scope}:${tab.id}`} id={`resource-${tab.id}`} role="tabpanel" aria-labelledby={`tab-${tab.id}`}
            className="workbench-resource absolute overflow-hidden" hidden={!leaf}
            style={leaf ? { left: `${leaf.rect.x}%`, top: `calc(${leaf.rect.y}% + 34px)`, width: `${leaf.rect.w}%`, height: `calc(${leaf.rect.h}% - 34px)` } : { display: 'none' }}
            onPointerDown={() => { if (leaf) focus(leaf.group); }}
            onDragOver={event => { if (leaf && event.dataTransfer.types.includes('application/custos-tab-id')) { event.preventDefault(); setDragTarget(leaf.group.id); } }}
            onDrop={event => {
              if (!leaf) return;
              const id = event.dataTransfer.getData('application/custos-tab-id');
              if (!tabIds.includes(id)) return;
              event.preventDefault(); setDragTarget(null);
              const bounds = event.currentTarget.getBoundingClientRect();
              if (event.clientY > bounds.bottom - bounds.height * .22) split(leaf.group, 'vertical', id);
              else if (event.clientX > bounds.right - bounds.width * .22) split(leaf.group, 'horizontal', id);
              else dispatch({ type: 'move', tabId: id, groupId: leaf.group.id });
              resourceTabs.setActiveTabId(id);
            }}>
            <ResourceContent {...props} catalog={catalog} session={tab.sessionId ? props.allSessions?.find(session => session.id === tab.sessionId) || (props.session?.id === tab.sessionId ? props.session : null) : props.session}
              onAskAgent={props.onAskAgent ? draft => props.onAskAgent?.(draft, tab.sessionId || props.session?.id) : undefined}
              onSendMessage={props.onSendMessage ? (text, sId) => props.onSendMessage?.(text, sId || tab.sessionId || props.session?.id) : undefined}
              activeWorkspaceId={tab.workspaceId} isResourceActive={!!leaf && props.isPaneOpen}
              resourceTabs={{ ...resourceTabs, activeTabId: tab.id, splitTabId: null }} />
          </div>;
        })}
        {separators.map(({ node, rect }) => <div key={node.id} role="separator" tabIndex={0} aria-label="Resize resource groups"
          aria-orientation={node.axis === 'horizontal' ? 'vertical' : 'horizontal'} aria-valuenow={Math.round(node.ratio)} aria-valuemin={20} aria-valuemax={80}
          className={`canvas-separator absolute z-30 ${node.axis === 'horizontal' ? 'is-horizontal' : 'is-vertical'}`}
          style={node.axis === 'horizontal' ? { left: `calc(${rect.x + rect.w * node.ratio / 100}% - 2px)`, top: `${rect.y}%`, width: 5, height: `${rect.h}%` } : { top: `calc(${rect.y + rect.h * node.ratio / 100}% - 2px)`, left: `${rect.x}%`, height: 5, width: `${rect.w}%` }}
          onDoubleClick={() => dispatch({ type: 'resize', splitId: node.id, ratio: 50 })}
          onKeyDown={event => { if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home'].includes(event.key)) return; event.preventDefault(); dispatch({ type: 'resize', splitId: node.id, ratio: event.key === 'Home' ? 50 : node.ratio + (['ArrowLeft', 'ArrowUp'].includes(event.key) ? -5 : 5) }); }}
          onPointerDown={event => {
            event.preventDefault(); const bounds = rootRef.current?.getBoundingClientRect(); if (!bounds) return;
            resize.current = { id: node.id, axis: node.axis, rect: new DOMRect(bounds.left + bounds.width * rect.x / 100, bounds.top + bounds.height * rect.y / 100, bounds.width * rect.w / 100, bounds.height * rect.h / 100) };
            document.body.classList.add('canvas-resizing');
          }} />)}
      </div>
    </div>
  );
});
