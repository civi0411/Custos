/**
 * Canonical ResourceRef and Canvas architecture for Custos SADE Desktop.
 * 
 * In SADE architecture:
 * 1. Opening a pane is idempotent (pure rendering of a ResourceRef).
 * 2. Commands (StartRun, ExecuteCell, IntegrateCandidate, DispatchOutbox) are backend actions.
 * 3. Any resource (Conversation, Task, File, Diff, Run, Source, Claim, Notebook) can be tiled as a peer pane.
 */

export type ResourceKind =
  | 'conversation'    // Peer Chat dialogue pane
  | 'task'            // Task management / Kanban / timeline
  | 'agent-run'       // Dedicated execution stream of a coding/research agent
  | 'diff'            // Code diff between commits or worktree candidates
  | 'file'            // Workspace file inspection / editing
  | 'source'          // Literature PDF / Web research source
  | 'claim'           // Research claims, findings, and evidence graph
  | 'notebook'        // Interactive research experiment notebook / lab runs
  | 'artifact'        // Generated artifact inspection with full provenance
  | 'browser'         // Native/sandboxed web preview scoped to host/workspace
  | 'terminal'        // PTY terminal session scoped to workspace
  | 'draft'           // Communication draft (email, report)
  | 'outbox'          // Dispatch queue and approval gate
  | 'contacts'        // Local contact roster
  | 'calendar'        // Local calendar & schedule
  | 'notes'           // Scratchpad notes
  | 'worktrees'       // Worktree management
  | 'fleet';          // Remote daemon nodes

export interface ResourceRef {
  kind: ResourceKind;
  projectId?: string;
  taskId?: string;
  sessionId?: string;
  runId?: string;
  executionWorkspaceId?: string;
  executionHostId?: string;
  sourceId?: string;
  artifactId?: string;
  revision?: string;
  url?: string;
  title?: string;
}

export interface CanvasPaneDescriptor {
  id: string;
  ref: ResourceRef;
  title: string;
  isPinned?: boolean;
  isDirty?: boolean;
}

export type SplitOrientation = 'horizontal' | 'vertical';

export interface CanvasLayoutState {
  primaryPaneId: string;
  splitPaneId?: string | null;
  splitRatio: number; // percentage 10-90, default 50
  orientation: SplitOrientation;
  isMaximized: boolean;
}

// Presentation only: no operation in this reducer may dispatch a domain command.
export type CanvasNode =
  | { kind: 'group'; id: string; tabIds: string[]; activeTabId: string }
  | { kind: 'split'; id: string; axis: SplitOrientation; ratio: number; first: CanvasNode; second: CanvasNode };
export interface CanvasState {
  version: 1;
  root: CanvasNode;
  focusedGroupId: string;
  zoomedGroupId: string | null;
}
export type CanvasAction =
  | { type: 'arrange'; tabIds: string[]; layout: 'columns' | 'review' }
  | { type: 'focus'; groupId: string; tabId?: string }
  | { type: 'move'; tabId: string; groupId: string; beforeTabId?: string }
  | { type: 'split'; groupId: string; newGroupId: string; axis: SplitOrientation; tabId?: string }
  | { type: 'resize'; splitId: string; ratio: number }
  | { type: 'zoom'; groupId: string }
  | { type: 'merge' }
  | { type: 'removeGroup'; groupId: string }
  | { type: 'close'; tabId: string };
export type CanvasGroup = Extract<CanvasNode, { kind: 'group' }>;
export const canvasGroups = (node: CanvasNode): CanvasGroup[] => node.kind === 'group'
  ? [node] : [...canvasGroups(node.first), ...canvasGroups(node.second)];
const mapCanvas = (node: CanvasNode, fn: (node: CanvasNode) => CanvasNode): CanvasNode => fn(node.kind === 'group'
  ? node : { ...node, first: mapCanvas(node.first, fn), second: mapCanvas(node.second, fn) });
const boundedRatio = (ratio: number) => Number.isFinite(ratio) ? Math.max(20, Math.min(80, ratio)) : 50;
export function createCanvas(tabIds: string[] = [], activeTabId = tabIds[0] || ''): CanvasState {
  return { version: 1, root: { kind: 'group', id: 'root', tabIds, activeTabId }, focusedGroupId: 'root', zoomedGroupId: null };
}
export function createCanvasFromPreset(
  tabIds: string[] = [],
  splitConfig?: { axis: SplitOrientation; ratio: number }
): CanvasState {
  if (!splitConfig || tabIds.length < 2) return createCanvas(tabIds);
  const mid = Math.max(1, Math.floor(tabIds.length / 2));
  const firstTabs = tabIds.slice(0, mid);
  const secondTabs = tabIds.slice(mid);
  const g1: CanvasGroup = { kind: 'group', id: 'group:primary', tabIds: firstTabs, activeTabId: firstTabs[0] || '' };
  const g2: CanvasGroup = { kind: 'group', id: 'group:secondary', tabIds: secondTabs, activeTabId: secondTabs[0] || '' };
  return {
    version: 1,
    root: { kind: 'split', id: 'split:preset', axis: splitConfig.axis, ratio: boundedRatio(splitConfig.ratio), first: g1, second: g2 },
    focusedGroupId: g1.id,
    zoomedGroupId: null,
  };
}
// Reconcile stale/duplicate refs and attach new tabs to the focused group.
export function reconcileCanvas(state: CanvasState, tabIds: string[], activeTabId?: string): CanvasState {
  const valid = new Set(tabIds);
  const seen = new Set<string>();
  let root = mapCanvas(state.root, node => {
    if (node.kind === 'split') return { ...node, ratio: boundedRatio(node.ratio) };
    const ids = node.tabIds.filter(id => valid.has(id) && !seen.has(id) && !!seen.add(id));
    return { ...node, tabIds: ids, activeTabId: ids.includes(node.activeTabId) ? node.activeTabId : ids[0] || '' };
  });
  const groups = canvasGroups(root);
  let focusedGroupId = groups.some(g => g.id === state.focusedGroupId) ? state.focusedGroupId : groups[0].id;
  const added = tabIds.filter(id => !seen.has(id));
  root = mapCanvas(root, node => node.kind === 'group' && node.id === focusedGroupId
    ? { ...node, tabIds: [...node.tabIds, ...added], activeTabId: added[added.length - 1] || node.activeTabId } : node);
  if (activeTabId && valid.has(activeTabId)) {
    const owner = canvasGroups(root).find(g => g.tabIds.includes(activeTabId));
    if (owner) {
      focusedGroupId = owner.id;
      root = mapCanvas(root, node => node.kind === 'group' && node.id === owner.id ? { ...node, activeTabId } : node);
    }
  }
  return { ...state, root, focusedGroupId, zoomedGroupId: state.zoomedGroupId && groups.some(g => g.id === state.zoomedGroupId) ? focusedGroupId : null };
}
export function canvasReducer(state: CanvasState, action: CanvasAction): CanvasState {
  const groups = canvasGroups(state.root);
  let root = state.root;
  if (action.type === 'arrange') {
    const all = groups.flatMap(group => group.tabIds);
    const selected = [...new Set(action.tabIds)].filter(id => all.includes(id)).slice(0, 3);
    if (selected.length < 2) return state;
    const rest = all.filter(id => !selected.includes(id));
    const panes: CanvasGroup[] = selected.map((id, index) => ({ kind: 'group', id: `desk:${index}`,
      tabIds: index === 0 ? [id, ...rest] : [id], activeTabId: id }));
    const trailing: CanvasNode = panes.length === 3 ? { kind: 'split', id: 'desk:secondary',
      axis: action.layout === 'review' ? 'vertical' : 'horizontal', ratio: 50, first: panes[1], second: panes[2] } : panes[1];
    return { ...state, root: { kind: 'split', id: 'desk:primary', axis: 'horizontal',
      ratio: action.layout === 'review' ? 60 : panes.length === 3 ? 34 : 50, first: panes[0], second: trailing },
      focusedGroupId: panes[0].id, zoomedGroupId: null };
  }
  if (action.type === 'focus') {
    const group = groups.find(g => g.id === action.groupId);
    if (!group) return state;
    root = mapCanvas(root, node => node.kind === 'group' && node.id === group.id && action.tabId && node.tabIds.includes(action.tabId)
      ? { ...node, activeTabId: action.tabId } : node);
    return { ...state, root, focusedGroupId: group.id, zoomedGroupId: state.zoomedGroupId ? group.id : null };
  }
  if (action.type === 'zoom') return groups.some(g => g.id === action.groupId)
    ? { ...state, zoomedGroupId: state.zoomedGroupId === action.groupId ? null : action.groupId, focusedGroupId: action.groupId } : state;
  if (action.type === 'resize') return { ...state, root: mapCanvas(root, node => node.kind === 'split' && node.id === action.splitId
    ? { ...node, ratio: boundedRatio(action.ratio) } : node) };
  if (action.type === 'merge') {
    const ids = groups.flatMap(g => g.tabIds);
    const focused = groups.find(g => g.id === state.focusedGroupId);
    return createCanvas(ids, focused?.activeTabId || ids[0]);
  }
  if (action.type === 'removeGroup') {
    const removed = groups.find(group => group.id === action.groupId);
    const target = groups.find(group => group.id !== action.groupId);
    if (!removed || !target) return state;
    const prune = (node: CanvasNode): CanvasNode | null => {
      if (node.kind === 'group') return node.id === removed.id ? null : node;
      const first = prune(node.first), second = prune(node.second);
      return first && second ? { ...node, first, second } : first || second;
    };
    root = mapCanvas(prune(root)!, node => node.kind === 'group' && node.id === target.id
      ? { ...node, tabIds: [...node.tabIds, ...removed.tabIds], activeTabId: removed.activeTabId || node.activeTabId } : node);
    return { ...state, root, focusedGroupId: target.id, zoomedGroupId: null };
  }
  if (action.type === 'split') {
    if (groups.length >= 16 || !groups.some(g => g.id === action.groupId) || groups.some(g => g.id === action.newGroupId)) return state;
    if (action.tabId && !groups.some(g => g.tabIds.includes(action.tabId!))) return state;
    if (action.tabId) root = removeTab(root, action.tabId);
    root = mapCanvas(root, node => node.kind === 'group' && node.id === action.groupId ? {
      kind: 'split', id: `split:${action.newGroupId}`, axis: action.axis, ratio: 50, first: node,
      second: { kind: 'group', id: action.newGroupId, tabIds: action.tabId ? [action.tabId] : [], activeTabId: action.tabId || '' },
    } : node);
    return { ...state, root, focusedGroupId: action.newGroupId, zoomedGroupId: null };
  }
  if (action.type === 'close') return { ...state, root: removeTab(root, action.tabId) };
  if (action.type === 'move') {
    if (!groups.some(g => g.id === action.groupId) || !groups.some(g => g.tabIds.includes(action.tabId))) return state;
    root = removeTab(root, action.tabId);
    root = mapCanvas(root, node => {
      if (node.kind !== 'group' || node.id !== action.groupId) return node;
      const ids = [...node.tabIds];
      const before = action.beforeTabId ? ids.indexOf(action.beforeTabId) : -1;
      ids.splice(before < 0 ? ids.length : before, 0, action.tabId);
      return { ...node, tabIds: ids, activeTabId: action.tabId };
    });
    return { ...state, root, focusedGroupId: action.groupId };
  }
  return state;
}
function removeTab(root: CanvasNode, tabId: string): CanvasNode {
  return mapCanvas(root, node => {
    if (node.kind !== 'group') return node;
    const ids = node.tabIds.filter(id => id !== tabId);
    return { ...node, tabIds: ids, activeTabId: node.activeTabId === tabId ? ids[0] || '' : node.activeTabId };
  });
}
// Treat persisted UI state as untrusted. Bound tree depth and size before rendering.
export function restoreCanvas(value: unknown, tabIds: string[], activeTabId?: string): CanvasState {
  const fallback = () => createCanvas(tabIds, activeTabId);
  if (!value || typeof value !== 'object') return fallback();
  const saved = value as Partial<CanvasState>;
  const ids = new Set<string>();
  let count = 0;
  const valid = (node: unknown, depth: number): node is CanvasNode => {
    if (!node || typeof node !== 'object' || depth > 8 || ++count > 63) return false;
    const n = node as CanvasNode;
    if (typeof n.id !== 'string' || ids.has(n.id)) return false;
    ids.add(n.id);
    if (n.kind === 'group') return Array.isArray(n.tabIds) && n.tabIds.length <= 200 && n.tabIds.every(id => typeof id === 'string') && typeof n.activeTabId === 'string';
    return n.kind === 'split' && ['horizontal', 'vertical'].includes(n.axis) && typeof n.ratio === 'number' && valid(n.first, depth + 1) && valid(n.second, depth + 1);
  };
  if (saved.version !== 1 || !valid(saved.root, 0)) return fallback();
  return reconcileCanvas({ version: 1, root: saved.root, focusedGroupId: saved.focusedGroupId || '', zoomedGroupId: saved.zoomedGroupId || null }, tabIds, activeTabId);
}
