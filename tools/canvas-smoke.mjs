import assert from 'node:assert/strict';
import fs from 'node:fs';
import { createRequire } from 'node:module';
import vm from 'node:vm';
import test from 'node:test';
const require = createRequire(new URL('../ui/desktop/package.json', import.meta.url));
const ts = require('typescript');
const source = fs.readFileSync(new URL('../ui/desktop/src/types/canvas.ts', import.meta.url), 'utf8');
const compiled = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 } }).outputText;
const exports = {};
vm.runInNewContext(compiled, { exports });
const { createCanvas, canvasGroups, canvasReducer: reduce, reconcileCanvas, restoreCanvas } = exports;
const plain = value => JSON.parse(JSON.stringify(value));
function invariant(state, expected) {
  const groups = canvasGroups(state.root);
  const tabs = groups.flatMap(group => group.tabIds);
  assert.equal(new Set(tabs).size, tabs.length, 'each tab has exactly one owner');
  assert.deepEqual([...tabs].sort(), [...expected].sort());
  for (const group of groups) assert.ok(group.tabIds.length ? group.tabIds.includes(group.activeTabId) : group.activeTabId === '');
  assert.ok(groups.some(group => group.id === state.focusedGroupId));
}
test('three nested groups, moving, closing and merging preserve resource ownership', () => {
  let state = createCanvas(['a', 'b', 'c'], 'a');
  const original = JSON.stringify(state);
  state = reduce(state, { type: 'split', groupId: 'root', newGroupId: 'right', axis: 'horizontal', tabId: 'b' });
  state = reduce(state, { type: 'split', groupId: 'right', newGroupId: 'bottom', axis: 'vertical', tabId: 'c' });
  invariant(state, ['a', 'b', 'c']);
  assert.equal(canvasGroups(state.root).length, 3);
  state = reduce(state, { type: 'move', groupId: 'bottom', tabId: 'a', beforeTabId: 'c' });
  assert.deepEqual(plain(canvasGroups(state.root).find(group => group.id === 'bottom').tabIds), ['a', 'c']);
  invariant(state, ['a', 'b', 'c']);
  state = reduce(state, { type: 'close', tabId: 'a' });
  invariant(state, ['b', 'c']);
  state = reduce(state, { type: 'merge' });
  invariant(state, ['b', 'c']);
  assert.equal(canvasGroups(state.root).length, 1);
  assert.equal(original, JSON.stringify(createCanvas(['a', 'b', 'c'], 'a')), 'input remains immutable');
});
test('empty groups accept new resources and final close is recoverable', () => {
  let state = reduce(createCanvas(['a']), { type: 'split', groupId: 'root', newGroupId: 'empty', axis: 'horizontal' });
  state = reconcileCanvas(state, ['a', 'b'], 'b');
  assert.equal(canvasGroups(state.root).find(group => group.id === 'empty').activeTabId, 'b');
  state = reduce(reduce(state, { type: 'close', tabId: 'a' }), { type: 'close', tabId: 'b' });
  invariant(state, []);
  invariant(reconcileCanvas(state, ['new']), ['new']);
});
test('restore rejects malformed, excessive and duplicate-node trees; stale refs are removed', () => {
  for (const bad of [null, {}, { version: 1, root: { kind: 'group', id: 'x', tabIds: 'wrong' } }, { version: 2 }]) {
    invariant(restoreCanvas(bad, ['a']), ['a']);
  }
  const state = reduce(createCanvas(['a', 'stale']), { type: 'split', groupId: 'root', newGroupId: 'right', axis: 'horizontal', tabId: 'stale' });
  const restored = restoreCanvas(JSON.parse(JSON.stringify(state)), ['a', 'new'], 'new');
  invariant(restored, ['a', 'new']);
  const duplicate = plain(state); duplicate.root.second.id = 'root';
  assert.equal(restoreCanvas(duplicate, ['a']).root.kind, 'group');
  const brokenRatio = plain(state); brokenRatio.root.ratio = 900;
  assert.equal(restoreCanvas(brokenRatio, ['a']).root.ratio, 80);
});
test('resize bounds, focus, zoom and invalid moves never change tab identities', () => {
  let state = reduce(createCanvas(['a', 'b']), { type: 'split', groupId: 'root', newGroupId: 'right', axis: 'vertical', tabId: 'b' });
  state = reduce(state, { type: 'resize', splitId: 'split:right', ratio: -20 }); assert.equal(state.root.ratio, 20);
  state = reduce(state, { type: 'resize', splitId: 'split:right', ratio: NaN }); assert.equal(state.root.ratio, 50);
  state = reduce(state, { type: 'zoom', groupId: 'right' }); assert.equal(state.zoomedGroupId, 'right');
  state = reduce(state, { type: 'zoom', groupId: 'right' }); assert.equal(state.zoomedGroupId, null);
  assert.equal(reduce(state, { type: 'move', groupId: 'missing', tabId: 'a' }), state);
  invariant(state, ['a', 'b']);
});
test('layout round-trip preserves nested axis, ratio, selected tab and zoom without execution data', () => {
  let state = reduce(createCanvas(['a', 'b']), { type: 'split', groupId: 'root', newGroupId: 'right', axis: 'vertical', tabId: 'b' });
  state = reduce(state, { type: 'resize', splitId: 'split:right', ratio: 63 });
  state = reduce(state, { type: 'zoom', groupId: 'right' });
  assert.deepEqual(plain(restoreCanvas(plain(state), ['a', 'b'])), plain(state));
});
test('long mixed action sequence keeps every resource in exactly one group', () => {
  let state = createCanvas(['a', 'b', 'c', 'd']);
  for (let index = 0; index < 80; index++) {
    const groups = canvasGroups(state.root);
    const group = groups[index % groups.length];
    if (index % 9 === 0 && groups.length < 5) state = reduce(state, { type: 'split', groupId: group.id, newGroupId: `g${index}`, axis: index % 2 ? 'vertical' : 'horizontal' });
    else state = reduce(state, { type: 'move', groupId: group.id, tabId: ['a', 'b', 'c', 'd'][index % 4] });
    state = restoreCanvas(plain(state), ['a', 'b', 'c', 'd']);
    invariant(state, ['a', 'b', 'c', 'd']);
  }
});

test('closing a group transfers resources and collapses the tree without closing a resource', () => {
  let state = reduce(createCanvas(['a', 'b']), { type: 'split', groupId: 'root', newGroupId: 'right', axis: 'vertical', tabId: 'b' });
  state = reduce(state, { type: 'removeGroup', groupId: 'right' });
  invariant(state, ['a', 'b']);
  assert.equal(state.root.kind, 'group');
  assert.equal(reduce(state, { type: 'removeGroup', groupId: 'root' }), state);
});
test('changing focused resources while zoomed shows the destination group', () => {
  let state = reduce(createCanvas(['a', 'b']), { type: 'split', groupId: 'root', newGroupId: 'right', axis: 'horizontal', tabId: 'b' });
  state = reduce(state, { type: 'zoom', groupId: 'right' });
  state = reduce(state, { type: 'focus', groupId: 'root', tabId: 'a' });
  assert.equal(state.zoomedGroupId, 'root');
  state = reconcileCanvas(state, ['a', 'b'], 'b');
  assert.equal(state.zoomedGroupId, 'right');
});

// Isolate pure presentation/wire helpers with the repository's existing compiler.
function helpers(path, names, globals = {}) {
  const source = fs.readFileSync(new URL(`../ui/desktop/src/${path}`, import.meta.url), 'utf8');
  const parsed = ts.createSourceFile(path, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  const selected = parsed.statements.filter(statement => {
    if (ts.isFunctionDeclaration(statement)) return names.includes(statement.name?.text);
    if (ts.isVariableStatement(statement)) return statement.declarationList.declarations.some(declaration => names.includes(declaration.name.getText(parsed)));
    return false;
  });
  const output = ts.transpileModule(ts.createPrinter().printFile(ts.factory.updateSourceFile(parsed, selected)), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  }).outputText;
  const result = {};
  vm.runInNewContext(output, { exports: result, atob, Uint8Array, Date, ...globals });
  return result;
}
test('annotation wire format preserves exact version, target and RFC3339 timestamp', () => {
  const api = helpers('api/daemon_client.ts', ['camelKey', 'snakeKey', 'mapKeysDeep', 'fromDaemon', 'toDaemon', 'annotationFromWire', 'annotationToWire']);
  const annotation = { id: 'a', artifactPath: 'figure.png', artifactVersion: 2,
    target: { type: 'image_pin', xRatio: .2, yRatio: .7, pinNumber: 1 },
    note: 'Check error bars', actor: 'researcher', status: 'pending', createdAt: 1700000000123 };
  const wire = api.annotationToWire(annotation);
  assert.equal(wire.created_at, '2023-11-14T22:13:20.123Z');
  assert.equal(wire.target.x_ratio, .2);
  assert.deepEqual(plain(api.annotationFromWire(wire)), annotation);
  assert.throws(() => api.annotationToWire({ ...annotation, createdAt: NaN }));
});
test('editing notebook source invalidates old output; unchanged source retains it', () => {
  const { editNotebookCell } = helpers('components/workbenches/research/NotebookWorkbenchPane.tsx', ['editNotebookCell']);
  const cell = { id: 'a', source: 'print(42)', status: 'success', execution_count: 1, stdout: '42', output_image: 'old', wall_ms: 10 };
  assert.equal(editNotebookCell(cell, cell.source), cell);
  const changed = editNotebookCell(cell, 'print(43)');
  assert.equal(changed.status, 'idle'); assert.equal(changed.stdout, null); assert.equal(changed.execution_count, null); assert.equal(changed.output_image, null);
  assert.equal(cell.stdout, '42');
});
test('PDF decoding preserves binary bytes and refuses text or malformed transport', () => {
  const { decodePdfBytes } = helpers('components/viewer/PdfViewer.tsx', ['decodePdfBytes']);
  const bytes = Buffer.from('%PDF-1.7\n\xff\xfe\x00', 'latin1');
  assert.deepEqual(Buffer.from(decodePdfBytes(bytes.toString('base64'))), bytes);
  assert.deepEqual(Buffer.from(decodePdfBytes(`data:application/pdf;base64,${bytes.toString('base64')}`)), bytes);
  for (const invalid of ['%PDF-1.7 text', 'not base64!!!', Buffer.from('not PDF').toString('base64')]) assert.throws(() => decodePdfBytes(invalid));
});

test('workspace file normalization retains binary metadata used by shared viewers', () => {
  const { normalizeWorkspaceFile } = helpers('api/daemon_client.ts', ['normalizeWorkspaceFile']);
  for (const wire of [{ workspace_id: 'w', is_binary: true, size_bytes: 42, line_count: 0 }, { workspaceId: 'w', isBinary: true, sizeBytes: 42, lineCount: 0 }]) {
    const file = normalizeWorkspaceFile({ ...wire, path: 'figure.png', content: 'bytes', truncated: false });
    assert.equal(file.workspace_id, 'w'); assert.equal(file.is_binary, true); assert.equal(file.size_bytes, 42);
  }
});

test('resource opening reuses only the exact workspace, session, Task and agent scope', () => {
  const { findScopedResourceTab } = helpers('components/workbenches/OrcaTabbedContainer.tsx', ['getTabType', 'findScopedResourceTab'], {
    DEFAULT_RESOURCES: [{ id: 'files' }, { id: 'agents' }],
  });
  const tab = { id: 'files-a', type: 'files', title: 'Files', workspaceId: 'w1', sessionId: 's1', taskId: 't1' };
  const scope = { workspaceId: 'w1', sessionId: 's1', taskId: 't1' };
  assert.equal(findScopedResourceTab([tab], 'files', scope), tab);
  for (const field of ['workspaceId', 'sessionId', 'taskId', 'agentId']) {
    assert.equal(findScopedResourceTab([tab], 'files', { ...scope, [field]: 'other' }), undefined, field);
  }
  assert.equal(findScopedResourceTab([tab], 'agents', scope), undefined);
  assert.equal(findScopedResourceTab([{ ...tab, id: 'files-forged', type: 'unsupported' }], 'files', scope), undefined);
});

test('synthesis navigation accepts only a bounded claim locator in its resource URL', () => {
  const { synthesisClaimFromUrl } = helpers('components/workbenches/OrcaTabbedContainer.tsx', ['synthesisClaimFromUrl'], { URL });
  assert.equal(synthesisClaimFromUrl('custos://synthesis?claim=claim-42'), 'claim-42');
  for (const value of ['invalid', 'https://synthesis?claim=a', 'custos://other?claim=a', 'custos://synthesis', 'custos://synthesis?claim=%00', `custos://synthesis?claim=${'a'.repeat(129)}`]) {
    assert.equal(synthesisClaimFromUrl(value), undefined);
  }
});

test('switching a notebook cell to Markdown invalidates executed Python output', () => {
  const { editNotebookCell } = helpers('components/workbenches/research/NotebookWorkbenchPane.tsx', ['editNotebookCell']);
  const cell = { id: 'a', cell_type: 'code', source: 'print(42)', status: 'success', execution_count: 1, stdout: '42', output_image: 'old', wall_ms: 1 };
  const markdown = editNotebookCell(cell, cell.source, 'markdown');
  assert.equal(markdown.cell_type, 'markdown');
  assert.equal(markdown.stdout, null); assert.equal(markdown.execution_count, null); assert.equal(markdown.status, 'idle');
  assert.equal(editNotebookCell(markdown, markdown.source, 'markdown'), markdown);
  assert.equal(cell.stdout, '42');
});

test('desk arrangement preserves every open resource, exact ownership and restoration', () => {
  const original = createCanvas(['files', 'agent', 'terminal', 'notes']);
  const arranged = reduce(original, { type: 'arrange', tabIds: ['files', 'agent', 'terminal'], layout: 'review' });
  invariant(arranged, ['files', 'agent', 'terminal', 'notes']);
  assert.equal(arranged.root.first.activeTabId, 'files');
  assert.equal(arranged.root.second.axis, 'vertical');
  assert.deepEqual(plain(canvasGroups(arranged.root)[0].tabIds), ['files', 'notes']);
  const repeated = reduce(arranged, { type: 'arrange', tabIds: ['files', 'agent', 'terminal'], layout: 'review' });
  assert.deepEqual(plain(repeated), plain(arranged));
  invariant(restoreCanvas(plain(arranged), ['files', 'agent', 'terminal', 'notes']), ['files', 'agent', 'terminal', 'notes']);
  assert.equal(canvasGroups(original.root).length, 1);
});
test('desk arrangement rejects unknown or duplicate resources without losing tabs', () => {
  const state = createCanvas(['a', 'b', 'c']);
  assert.equal(reduce(state, { type: 'arrange', tabIds: ['a', 'a', 'missing'], layout: 'columns' }), state);
  const arranged = reduce(state, { type: 'arrange', tabIds: ['c', 'b', 'c', 'missing'], layout: 'columns' });
  invariant(arranged, ['a', 'b', 'c']);
  assert.equal(canvasGroups(arranged.root).length, 2);
});
test('picker and desk use one resource membership registry per workbench', () => {
  const { workbenchResourceIds } = helpers('components/workbenches/OrcaTabbedContainer.tsx', ['workbenchResourceIds']);
  for (const [mode, expected] of [['engineering', ['files', 'agents', 'terminal']], ['research', ['literature', 'lab', 'artifacts']], ['copilot', ['drafts', 'notes', 'calendar']]]) {
    const ids = workbenchResourceIds(mode);
    assert.equal(ids.length, new Set(ids).size);
    for (const id of [...expected, 'conversation', 'notes', 'browser']) assert.ok(ids.includes(id));
  }
  assert.deepEqual(plain(workbenchResourceIds('code')), plain(workbenchResourceIds('engineering')));
  assert.ok(!workbenchResourceIds('copilot').includes('lab'));
  assert.deepEqual(plain(workbenchResourceIds('copilot', false)), ['drafts', 'outbox', 'contacts', 'calendar']);
  assert.ok(!workbenchResourceIds('research', false).includes('conversation'));
});

test('composer drafts stay with their session across navigation and shared renderers', () => {
  const window = new EventTarget();
  let notifications = 0;
  window.addEventListener('custos:composer-updated', () => notifications++);
  const api = helpers('components/chat/ConversationPane.tsx', ['conversationDrafts', 'readConversationDraft', 'writeConversationDraft', 'appendConversationDraft'], { window, Event });
  api.writeConversationDraft('s1', 'Unsent question');
  api.writeConversationDraft('s2', 'Other conversation');
  api.appendConversationDraft('s1', 'Selected figure context');
  assert.equal(api.readConversationDraft('s1'), 'Unsent question\n\nSelected figure context');
  assert.equal(api.readConversationDraft('s2'), 'Other conversation');
  api.writeConversationDraft('s1', '');
  assert.equal(api.readConversationDraft('s1'), '');
  assert.equal(api.readConversationDraft('s2'), 'Other conversation');
  const before = notifications;
  api.appendConversationDraft('', 'must not leak');
  assert.equal(notifications, before);
  assert.equal(api.readConversationDraft(''), '');
});

test('send acknowledgement clears only the accepted, unchanged Session draft', () => {
  const window = new EventTarget();
  const api = helpers('components/chat/ConversationPane.tsx', ['conversationDrafts', 'readConversationDraft', 'writeConversationDraft', 'clearAcceptedConversationDraft'], { window, Event });
  api.writeConversationDraft('s1', 'Original');
  api.writeConversationDraft('s2', 'Other');
  assert.equal(api.clearAcceptedConversationDraft('s1', 'Original', false), false);
  assert.equal(api.readConversationDraft('s1'), 'Original');
  api.writeConversationDraft('s1', 'Edited while pending');
  assert.equal(api.clearAcceptedConversationDraft('s1', 'Original', true), false);
  assert.equal(api.readConversationDraft('s1'), 'Edited while pending');
  assert.equal(api.clearAcceptedConversationDraft('s1', 'Edited while pending', true), true);
  assert.equal(api.readConversationDraft('s1'), '');
  assert.equal(api.readConversationDraft('s2'), 'Other');
});

test('local calendar preserves explicit dates and refuses invalid or ambiguous dates', () => {
  const { rosterDateKey } = helpers('components/workbenches/copilot/ContactsCalendarPane.tsx', ['rosterDateKey']);
  assert.equal(rosterDateKey('2026-10-20T14:00'), '2026-10-20');
  assert.equal(rosterDateKey('2024-02-29'), '2024-02-29');
  assert.equal(rosterDateKey('2026-10-20 14:00 UTC'), '2026-10-20');
  for (const value of ['2026-02-29', '2026-13-01', '2026-04-31', 'tomorrow', '10/20/2026', '', '2026-10-20oops']) assert.equal(rosterDateKey(value), null);
});

const supervision = helpers('app/dashboard/page.tsx', ['supervisionBucket', 'loadSupervisionRuns']);
test('supervision keeps completed runs distinct from acceptance and unknown states in history', () => {
  assert.equal(supervision.supervisionBucket('completed'), 'review');
  assert.equal(supervision.supervisionBucket('suspended'), 'attention');
  assert.equal(supervision.supervisionBucket('active'), 'running');
  for (const status of ['failed', 'cancelled', 'unknown', 'disconnected']) assert.equal(supervision.supervisionBucket(status), 'history');
});
test('supervision bounds concurrency, deduplicates sessions, and reports missing coverage without fabricated runs', async () => {
  let active = 0, peak = 0;
  const calls = [];
  const result = await supervision.loadSupervisionRuns(['a', 'a', 'b', 'bad', 'c', 'd', 'e'], async id => {
    calls.push(id); active++; peak = Math.max(peak, active);
    await new Promise(resolve => setTimeout(resolve, 2));
    active--;
    if (id === 'bad') throw new Error('unavailable');
    return id === 'b' ? [] : [{ run_id: `run-${id}`, status: 'active' }];
  });
  assert.equal(calls.filter(id => id === 'a').length, 1);
  assert.ok(peak <= 4);
  assert.deepEqual(plain(result.failures), ['bad']);
  assert.equal(result.rows.length, 4);
  assert.ok(result.rows.every(row => row.sessionId !== 'bad' && row.sessionId !== 'b'));
  assert.deepEqual(plain(await supervision.loadSupervisionRuns([], () => { throw new Error('must not query'); })), { rows: [], failures: [] });
});

test('Screen layout observations do not trigger feedback writes or overwrite another Screen', () => {
  const { applyScreenCanvas } = helpers('context/ScreenContext.tsx', ['applyScreenCanvas']);
  const original = [{ id: 'a', canvasState: { root: 'first' } }, { id: 'b', canvasState: { root: 'second' } }];
  assert.equal(applyScreenCanvas(original, 'a', { root: 'first' }), original);
  assert.equal(applyScreenCanvas(original, 'missing', { root: 'new' }), original);
  const changed = applyScreenCanvas(original, 'a', { root: 'new' });
  assert.equal(changed[1], original[1]);
  assert.equal(original[0].canvasState.root, 'first');
  assert.equal(changed[0].canvasState.root, 'new');
});

const history = helpers('app/history/page.tsx', ['conversationHistoryRows', 'historyWorkbench', 'userHistoryTasks']);
test('history uses canonical session existence and exact membership, independent of active workspace', () => {
  const rows = history.conversationHistoryRows([{ id: 'a', created_at: '2026-01-01' }, { id: 'a', created_at: '2026-01-01' }, { id: 'unfiled', created_at: '2026-01-02' }], { Project: [{ id: 'a', title: 'Actual', taskId: 't', pack: 'research' }, { id: 'stale', title: 'Deleted' }] });
  assert.equal(rows.length, 2);
  assert.equal(rows[0].project, 'Project');
  assert.equal(rows[0].taskId, 't');
  assert.equal(rows[1].project, undefined);
  assert.equal(history.historyWorkbench('research'), 'research');
  assert.equal(history.historyWorkbench('unknown'), undefined);
  assert.deepEqual(plain(history.userHistoryTasks([{ id: 'goal' }, { id: 'internal', metadata: { kind: 'conversation_chat_turn' } }, { id: 'turn', metadata: { turn_task: true } }])), [{ id: 'goal' }]);
});


test('GitHub login output preserves device code and strips terminal escape sequences', () => {
  const { githubTerminalText } = helpers('features/providers/GitHubAuthModal.tsx', ['githubTerminalText']);
  assert.equal(githubTerminalText('\x1b[32mCode: ABCD-1234\x1b[0m\r\nPress Enter'), 'Code: ABCD-1234\nPress Enter');
  assert.equal(githubTerminalText('\x1b]0;title\x07Device page'), 'Device page');
});

// Execute the actual nested event handlers, without mounting editors or starting runtimes.
function nestedHandler(path, predicate, globals) {
  const source = fs.readFileSync(new URL(`../ui/desktop/src/${path}`, import.meta.url), 'utf8');
  const parsed = ts.createSourceFile(path, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  let expression;
  function visit(node) {
    if (!expression && predicate(node)) expression = ts.isVariableDeclaration(node) ? node.initializer : node.initializer.expression;
    ts.forEachChild(node, visit);
  }
  visit(parsed);
  assert.ok(expression, `Expected handler in ${path}`);
  const output = ts.transpileModule(`const handler = ${expression.getText(parsed)}; exports.handler = handler;`, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  }).outputText;
  const exports = {};
  vm.runInNewContext(output, { exports, ...globals });
  return exports.handler;
}

test('focusing a peer Lab tab does not reopen its already-selected conversation or Task', () => {
  const calls = [];
  const props = { session: { id: 's' }, selectedTaskId: 't',
    onOpenSession: id => calls.push(['session', id]), onSelectTask: id => calls.push(['task', id]) };
  const resourceTabs = { tabs: [{ id: 'lab', sessionId: 's', taskId: 't' }], setActiveTabId: id => calls.push(['tab', id]) };
  const focus = nestedHandler('components/workbenches/OrcaTabbedContainer.tsx', node => ts.isVariableDeclaration(node) && node.name.getText() === 'focus', {
    props, resourceTabs, dispatch: action => calls.push(['canvas', action.tabId]),
  });
  focus({ id: 'root' }, 'lab');
  assert.deepEqual(calls, [['canvas', 'lab'], ['tab', 'lab']]);
  calls.length = 0;
  props.session = { id: 'other' }; props.selectedTaskId = 'other-task';
  focus({ id: 'root' }, 'lab');
  assert.deepEqual(calls, [['canvas', 'lab'], ['tab', 'lab'], ['session', 's'], ['task', 't']]);
});

test('Lab keyboard navigation wraps, retains focus and leaves modified shortcuts alone', () => {
  let selected, focused, prevented = 0;
  const views = ['notebook', 'runs', 'training'];
  const buttons = views.map((_, index) => ({ focus: () => { focused = index; } }));
  const handler = nestedHandler('components/workbenches/research/LabPane.tsx', node => ts.isJsxAttribute(node) && node.name.getText() === 'onKeyDown', {
    views, setView: value => { selected = value; },
  });
  const event = key => ({ key, target: buttons[0], currentTarget: { querySelectorAll: () => buttons }, preventDefault: () => { prevented++; } });
  handler(event('End')); assert.equal(selected, 'training'); assert.equal(focused, 2);
  handler(event('ArrowLeft')); assert.equal(selected, 'training'); assert.equal(focused, 2);
  handler(event('ArrowRight')); assert.equal(selected, 'runs'); assert.equal(focused, 1);
  handler(event('Home')); assert.equal(selected, 'notebook'); assert.equal(focused, 0);
  handler({ ...event('End'), ctrlKey: true });
  handler({ ...event('End'), defaultPrevented: true });
  assert.equal(prevented, 4);
  assert.equal(selected, 'notebook');
});

test('Training search shortcut stays pane-local and does not capture typing or IME', () => {
  let focused = 0, prevented = 0;
  const handler = nestedHandler('components/workbenches/research/TrainingRunsPane.tsx', node => ts.isJsxAttribute(node) && node.name.getText() === 'onKeyDown', {
    searchRef: { current: { focus: () => { focused++; } } },
  });
  const event = editable => ({ key: '/', nativeEvent: { isComposing: false }, target: { closest: () => editable }, preventDefault: () => { prevented++; } });
  handler(event(false));
  handler(event(true));
  handler({ ...event(false), metaKey: true });
  handler({ ...event(false), nativeEvent: { isComposing: true } });
  assert.equal(focused, 1); assert.equal(prevented, 1);
});
