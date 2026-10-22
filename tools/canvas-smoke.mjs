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
const { createCanvas, createCanvasFromPreset, canvasGroups, canvasReducer: reduce, reconcileCanvas, restoreCanvas } = exports;
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
  let source = fs.readFileSync(new URL(`../ui/desktop/src/${path}`, import.meta.url), 'utf8');
  if (source.includes("from './normalizers'")) {
    const normSource = fs.readFileSync(new URL('../ui/desktop/src/api/normalizers.ts', import.meta.url), 'utf8');
    source = normSource + '\n' + source;
  }
  if (source.includes("from './transport'")) {
    const transSource = fs.readFileSync(new URL('../ui/desktop/src/api/transport.ts', import.meta.url), 'utf8');
    source = transSource + '\n' + source;
  }
  const parsed = ts.createSourceFile(path, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  const selected = parsed.statements.filter(statement => {
    if (ts.isClassDeclaration(statement)) return names.includes(statement.name?.text) || true;
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
  const { findScopedResourceTab } = helpers('components/workbenches/workbenchResources.tsx', ['getTabType', 'findScopedResourceTab'], {
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
  const { synthesisClaimFromUrl } = helpers('components/workbenches/workbenchResources.tsx', ['synthesisClaimFromUrl'], { URL });
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
  const { workbenchResourceIds } = helpers('components/workbenches/workbenchResources.tsx', ['workbenchResourceIds']);
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
test('screen navigation never borrows a layout from another workbench', () => {
  const { screensForWorkbench } = helpers('types/screen.ts', ['screensForWorkbench']);
  const screens = [
    { id: 'code', homeWorkbench: 'engineering' },
    { id: 'study', homeWorkbench: 'research' },
    { id: 'assistant', homeWorkbench: 'copilot' },
    { id: 'legacy' },
  ];
  assert.deepEqual(plain(screensForWorkbench(screens, 'research').map((screen) => screen.id)), ['study']);
  assert.deepEqual(plain(screensForWorkbench(screens, 'engineering').map((screen) => screen.id)), ['code', 'legacy']);
  assert.deepEqual(plain(screensForWorkbench(screens, 'copilot').map((screen) => screen.id)), ['assistant']);
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

test('Research notebook, runs and training are peer resources with independent tab identities', () => {
  const { workbenchResourceIds } = helpers('components/workbenches/workbenchResources.tsx', ['workbenchResourceIds']);
  const resources = workbenchResourceIds('research');
  for (const id of ['lab', 'runs', 'training', 'terminal', 'files']) assert.ok(resources.includes(id));
  let state = createCanvas(['notebook', 'runs', 'training']);
  state = reduce(state, { type: 'split', groupId: 'root', newGroupId: 'runs-pane', axis: 'horizontal', tabId: 'runs' });
  state = reduce(state, { type: 'split', groupId: 'runs-pane', newGroupId: 'training-pane', axis: 'vertical', tabId: 'training' });
  invariant(state, ['notebook', 'runs', 'training']);
  assert.equal(canvasGroups(state.root).length, 3);
});

test('terminal wire normalization retains workspace identity through camelCase transport', () => {
  const { normalizeTerminalSession } = helpers('api/daemon_client.ts', ['normalizeTerminalSession']);
  const normalized = normalizeTerminalSession({ id: 'term-1', workspaceId: 'worktree-a', workingDir: '/repo/a', createdAt: 123, closedAt: null });
  assert.equal(normalized.workspace_id, 'worktree-a');
  assert.equal(normalized.working_dir, '/repo/a');
  assert.equal(normalized.created_at, 123);
  assert.equal(normalized.closed_at, null);
});

test('terminal attachment requires an exact workspace and tab binding never aliases another command', () => {
  const { terminalBindingKey, terminalMatchesWorkspace } = helpers('hooks/useTerminalSession.ts', ['terminalBindingKey', 'terminalMatchesWorkspace']);
  assert.notEqual(terminalBindingKey('a', 'workspace', 'claude'), terminalBindingKey('b', 'workspace', 'claude'));
  assert.notEqual(terminalBindingKey('a', 'workspace', 'claude'), terminalBindingKey('a', 'other', 'claude'));
  assert.notEqual(terminalBindingKey('a', 'workspace', 'claude'), terminalBindingKey('a', 'workspace', 'codex'));
  assert.equal(terminalMatchesWorkspace({ workspace_id: 'a' }, 'a'), true);
  assert.equal(terminalMatchesWorkspace({ workspace_id: 'a' }, 'b'), false);
  assert.equal(terminalMatchesWorkspace({ workspace_id: '' }, ''), false);
});

test('harness wire normalization retains availability, cancellation and assurance', () => {
  const { normalizeHarness } = helpers('api/daemon_client.ts', ['normalizeHarness']);
  const normalized = normalizeHarness({ id: 'agent', binaryPath: '/bin/agent', isAvailable: true, launchCmd: 'agent', profile: {
    toolMediation: 'provider_governed', worktreeOwnership: 'shared_live', supportsCancel: true, supportsSteer: false, costVisibility: 'none',
  } });
  assert.equal(normalized.binary_path, '/bin/agent');
  assert.equal(normalized.is_available, true);
  assert.equal(normalized.profile.supports_cancel, true);
  assert.equal(normalized.profile.tool_mediation, 'provider_governed');
});

test('Outbox never dispatches a pending, uncertain or already-dispatched action', () => {
  const { outboxActions } = helpers('components/workbenches/copilot/OutboxPane.tsx', ['outboxActions']);
  const operations = ['outbox.approve', 'outbox.dispatch'];
  assert.equal(outboxActions('pending_approval', operations).approve, true);
  assert.equal(outboxActions('pending_approval', operations).dispatch, false);
  assert.equal(outboxActions('approved', operations).dispatch, true);
  assert.equal(outboxActions('approved', []).dispatch, false);
  for (const status of ['uncertain', 'failed', 'dispatching', 'receipted', 'succeeded']) {
    assert.deepEqual(plain(outboxActions(status, operations)), { approve: false, dispatch: false });
  }
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

test('createCanvasFromPreset creates split canvas matching preset splitConfig', () => {
  const horizontal = createCanvasFromPreset(['conversation', 'files'], { axis: 'horizontal', ratio: 45 });
  invariant(horizontal, ['conversation', 'files']);
  assert.equal(horizontal.root.kind, 'split');
  assert.equal(horizontal.root.axis, 'horizontal');
  assert.equal(horizontal.root.ratio, 45);
  assert.equal(canvasGroups(horizontal.root).length, 2);

  const vertical = createCanvasFromPreset(['files', 'terminal'], { axis: 'vertical', ratio: 65 });
  invariant(vertical, ['files', 'terminal']);
  assert.equal(vertical.root.kind, 'split');
  assert.equal(vertical.root.axis, 'vertical');
  assert.equal(vertical.root.ratio, 65);

  const fallback = createCanvasFromPreset(['single']);
  invariant(fallback, ['single']);
  assert.equal(fallback.root.kind, 'group');
});

test('scoped resource tabs allow distinct papers, files and terminals in same session without deduplication', () => {
  const { findScopedResourceTab } = helpers('components/workbenches/workbenchResources.tsx', ['findScopedResourceTab', 'getTabType'], {
    DEFAULT_RESOURCES: [{ id: 'conversation' }, { id: 'literature' }, { id: 'files' }, { id: 'terminal' }],
    MessageSquare: () => null,
  });
  const tabs = [
    { id: 'lit-1', type: 'literature', title: 'Paper A', url: 'custos://resource/literature?doc=paperA', workspaceId: 'ws-1', sessionId: 's-1', sourceId: 'src-paperA' },
    { id: 'lit-2', type: 'literature', title: 'Paper B', url: 'custos://resource/literature?doc=paperB', workspaceId: 'ws-1', sessionId: 's-1', sourceId: 'src-paperB' },
    { id: 'file-1', type: 'files', title: 'main.rs', url: 'custos://file?path=main.rs', workspaceId: 'ws-1', sessionId: 's-1' },
    { id: 'file-2', type: 'files', title: 'lib.rs', url: 'custos://file?path=lib.rs', workspaceId: 'ws-1', sessionId: 's-1' },
  ];
  const foundPaperA = findScopedResourceTab(tabs, 'literature', { workspaceId: 'ws-1', sessionId: 's-1', sourceId: 'src-paperA' });
  const foundPaperB = findScopedResourceTab(tabs, 'literature', { workspaceId: 'ws-1', sessionId: 's-1', sourceId: 'src-paperB' });
  assert.equal(foundPaperA?.id, 'lit-1');
  assert.equal(foundPaperB?.id, 'lit-2');
  assert.notEqual(foundPaperA?.id, foundPaperB?.id);

  const foundFile1 = findScopedResourceTab(tabs, 'files', { workspaceId: 'ws-1', sessionId: 's-1', url: 'custos://file?path=main.rs' });
  const foundFile2 = findScopedResourceTab(tabs, 'files', { workspaceId: 'ws-1', sessionId: 's-1', url: 'custos://file?path=lib.rs' });
  assert.equal(foundFile1?.id, 'file-1');
  assert.equal(foundFile2?.id, 'file-2');
  assert.notEqual(foundFile1?.id, foundFile2?.id);
});

test('ScreenLayoutSnapshotV2 migration reads legacy screen and canvas keys and validates write-back', () => {
  const store = new Map();
  const mockLocalStorage = {
    getItem: key => store.get(key) || null,
    setItem: (key, val) => store.set(key, String(val)),
  };
  const { loadAndMigrateScreens } = helpers('context/ScreenContext.tsx', ['loadAndMigrateScreens'], { localStorage: mockLocalStorage });
  const legacyScreens = [{ id: 's-1', name: 'Legacy Screen', homeWorkbench: 'engineering', canvasState: null }];
  mockLocalStorage.setItem('custos.screens.default', JSON.stringify(legacyScreens));
  const canvasKey = `custos.canvas.v1:${JSON.stringify(['default', 's-1'])}`;
  mockLocalStorage.setItem(canvasKey, JSON.stringify({ version: 1, root: { kind: 'group', id: 'root', tabIds: ['a'], activeTabId: 'a' } }));

  const defaultScreens = [{ id: 'screen-default', name: 'Default', homeWorkbench: 'engineering' }];
  const migrated = loadAndMigrateScreens('default', 'default', defaultScreens);
  assert.equal(migrated.screens.length, 1);
  assert.equal(migrated.screens[0].id, 's-1');
  assert.equal(migrated.screens[0].canvasState?.root?.kind, 'group');
  assert.ok(mockLocalStorage.getItem('custos.screen_layout.v2.default'));
});

test('scoped live run observer buffers active tokens and notifies run subscribers without global store churn', () => {
  const { scopedLiveRunStore } = helpers('context/app/scopedLiveRun.ts', ['scopedLiveRunStore', 'ScopedLiveRunRegistry'], { console });
  const runId = 'run-test-123';
  const received = [];

  const unsubscribe = scopedLiveRunStore.subscribe(runId, (snapshot) => {
    received.push(snapshot.text);
  });

  scopedLiveRunStore.emit(runId, { run_id: runId, status: 'running', text: 'Hello' });
  scopedLiveRunStore.emit(runId, { run_id: runId, status: 'running', text: 'Hello world' });

  assert.equal(received.length, 2);
  assert.equal(received[0], 'Hello');
  assert.equal(received[1], 'Hello world');
  assert.equal(scopedLiveRunStore.get(runId)?.text, 'Hello world');

  unsubscribe();
  scopedLiveRunStore.emit(runId, { run_id: runId, status: 'completed', text: 'Hello world finished' });
  assert.equal(received.length, 2); // Unsubscribed, does not receive further updates

  scopedLiveRunStore.clear(runId);
  assert.equal(scopedLiveRunStore.get(runId), undefined);
});

test('Screen restore preserves saved selection between multiple screens on same workbench without default overwrite', () => {
  const store = new Map();
  const mockLocalStorage = {
    getItem: key => store.get(key) || null,
    setItem: (key, val) => store.set(key, String(val)),
  };
  const { loadAndMigrateScreens } = helpers('context/ScreenContext.tsx', ['loadAndMigrateScreens'], { localStorage: mockLocalStorage });
  const screens = [
    { id: 'screen-default', name: 'Main Dev', homeWorkbench: 'engineering', canvasState: null },
    { id: 'screen-second', name: 'Second Dev Screen', homeWorkbench: 'engineering', canvasState: null },
  ];
  const snapshot = {
    version: 2,
    scope: 'default',
    screens,
    activeScreenId: 'screen-second',
    timestamp: Date.now(),
  };
  mockLocalStorage.setItem('custos.screen_layout.v2.default', JSON.stringify(snapshot));

  const defaultScreens = [{ id: 'screen-default', name: 'Default', homeWorkbench: 'engineering' }];
  const restored = loadAndMigrateScreens('default', 'default', defaultScreens);
  assert.equal(restored.screens.length, 2);
  assert.equal(restored.activeScreenId, 'screen-second');
  assert.notEqual(restored.activeScreenId, 'screen-default');
});

test('Screen tab isolation ensures new and existing screens do not leak tabs across workbenches or screens', () => {
  // Screen A: Engineering Dev Screen
  const tabsA = [
    { id: 'conv-a', type: 'conversation', title: 'Chat A' },
    { id: 'term-a', type: 'terminal', title: 'Terminal A' },
  ];
  let canvasA = createCanvas(tabsA.map(t => t.id));

  // Screen B: Research Lab Screen
  const tabsB = [
    { id: 'conv-b', type: 'conversation', title: 'Chat B' },
    { id: 'lab-b', type: 'lab', title: 'Lab B' },
  ];
  let canvasB = createCanvas(tabsB.map(t => t.id));

  // Invariant 1: Canvas A groups contain ONLY tabs from Screen A
  const groupsA = canvasGroups(canvasA.root);
  const tabIdsA = groupsA.flatMap(g => g.tabIds);
  assert.deepEqual(plain(tabIdsA).sort(), ['conv-a', 'term-a'].sort());
  assert.ok(!tabIdsA.includes('conv-b'));
  assert.ok(!tabIdsA.includes('lab-b'));

  // Invariant 2: Canvas B groups contain ONLY tabs from Screen B
  const groupsB = canvasGroups(canvasB.root);
  const tabIdsB = groupsB.flatMap(g => g.tabIds);
  assert.deepEqual(plain(tabIdsB).sort(), ['conv-b', 'lab-b'].sort());
  assert.ok(!tabIdsB.includes('conv-a'));
  assert.ok(!tabIdsB.includes('term-a'));

  // Invariant 3: Reconcile Canvas B with its own tabs MUST NOT inject tabs from Screen A
  const reconciledB = reconcileCanvas(canvasB, tabsB.map(t => t.id), 'conv-b');
  const reconciledTabIdsB = canvasGroups(reconciledB.root).flatMap(g => g.tabIds);
  assert.deepEqual(plain(reconciledTabIdsB).sort(), ['conv-b', 'lab-b'].sort());
  assert.ok(!reconciledTabIdsB.includes('conv-a'));
  assert.ok(!reconciledTabIdsB.includes('term-a'));

  // Invariant 4: Modifying Screen B (opening new tab 'lit-b') never pollutes Screen A
  const updatedTabsB = [...tabsB, { id: 'lit-b', type: 'literature', title: 'Paper B' }];
  const updatedCanvasB = reconcileCanvas(canvasB, updatedTabsB.map(t => t.id), 'lit-b');
  const finalIdsB = canvasGroups(updatedCanvasB.root).flatMap(g => g.tabIds);
  assert.deepEqual(plain(finalIdsB).sort(), ['conv-b', 'lab-b', 'lit-b'].sort());

  // Screen A remains strictly isolated
  const finalIdsA = canvasGroups(canvasA.root).flatMap(g => g.tabIds);
  assert.deepEqual(plain(finalIdsA).sort(), ['conv-a', 'term-a'].sort());
  assert.ok(!finalIdsA.includes('lit-b'));
});

test('Transport semantics: command ID preservation on retry, conservative effectful rejection, and normalizer parity', () => {
  const transport = helpers('api/transport.ts', [
    'isReadMethod',
    'isRetrySafeMethod',
    'isTransportError',
    'generateCommandId',
    'prepareRequestParams',
    'evaluateRetry',
  ]);
  const normalizers = helpers('api/normalizers.ts', [
    'normalizeWorkspace',
  ]);

  // Invariant 1: Pure read method gets no command_id and is retry safe
  const readParams = transport.prepareRequestParams('v1.tasks.list', { session_id: 's-1' });
  assert.equal(readParams.command_id, undefined);
  const readRetry = transport.evaluateRetry('v1.tasks.list', readParams, new Error('channel closed'));
  assert.equal(readRetry.shouldRetry, true);
  assert.equal(readRetry.reason, 'idempotent_read');

  // Invariant 2: Mutation method generates command_id and preserves exact command_id across retries
  const initialParams = { title: 'Do work' };
  const preparedParams = transport.prepareRequestParams('v1.tasks.create', initialParams);
  assert.ok(typeof preparedParams.command_id === 'string' && preparedParams.command_id.startsWith('cmd_'));

  // Replaying with already assigned command_id MUST preserve it
  const replayParams = transport.prepareRequestParams('v1.tasks.create', preparedParams);
  assert.equal(replayParams.command_id, preparedParams.command_id);

  // Invariant 3: Mutation retry evaluation on transport error allows retry with preserved command_id
  const mutationRetry = transport.evaluateRetry('v1.tasks.create', preparedParams, new Error('IPC timeout'));
  assert.equal(mutationRetry.shouldRetry, true);
  assert.equal(mutationRetry.preserveCommandId, true);
  assert.equal(mutationRetry.reason, 'idempotent_command_id');

  // Invariant 4: Business rejection (e.g. Validation error, Conflict) is NEVER retried
  const businessError = new Error('Task title cannot be empty');
  const businessRetry = transport.evaluateRetry('v1.tasks.create', preparedParams, businessError);
  assert.equal(businessRetry.shouldRetry, false);
  assert.equal(businessRetry.reason, 'business_rejection');

  // Invariant 5: Effectful or interactive methods (e.g. pick_folder, probe) without command ID are not blindly retried
  const effectfulRetry = transport.evaluateRetry('v1.workspaces.pick_folder', {}, new Error('IPC timeout'));
  assert.equal(effectfulRetry.shouldRetry, false);
  assert.equal(effectfulRetry.reason, 'effectful_no_retry');

  // Invariant 6: normalizeWorkspace correctly handles setup_failed wire format without crashing
  const failedWs = normalizers.normalizeWorkspace({
    id: 'ws-failed',
    status: { setup_failed: { reason: 'git clone failed' } },
  });
  assert.equal(failedWs.status, 'setup_failed');
  assert.equal(failedWs.status_reason, 'git clone failed');
});
