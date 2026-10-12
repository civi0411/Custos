// Launch a test-profile desktop with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=
// "--remote-debugging-port=9223", then: node tools/browser-native-smoke.mjs
import assert from 'node:assert/strict';
import http from 'node:http';
import { setTimeout as delay } from 'node:timers/promises';

const endpoint = process.env.CUSTOS_BROWSER_CDP ?? 'http://127.0.0.1:9223';
const targets = async () => (await fetch(`${endpoint}/json/list`)).json();
async function waitFor(read, predicate, message) {
  for (let attempt = 0; attempt < 100; attempt++) {
    const value = await read();
    if (predicate(value)) return value;
    await delay(100);
  }
  throw new Error(message);
}
async function connect(target) {
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
  let sequence = 0;
  const pending = new Map();
  socket.onmessage = ({ data }) => {
    const response = JSON.parse(data);
    const callback = pending.get(response.id);
    if (callback) { pending.delete(response.id); response.error ? callback.reject(response.error) : callback.resolve(response.result); }
  };
  return {
    async eval(expression) {
      const id = ++sequence;
      const response = await new Promise((resolve, reject) => {
        const timer = setTimeout(() => { pending.delete(id); reject(new Error('CDP evaluation timed out')); }, 15000);
        pending.set(id, { resolve: (value) => { clearTimeout(timer); resolve(value); }, reject: (error) => { clearTimeout(timer); reject(error); } });
        socket.send(JSON.stringify({ id, method: 'Runtime.evaluate', params: { expression, awaitPromise: true, returnByValue: true } }));
      });
      if (response.exceptionDetails) throw new Error(JSON.stringify(response.exceptionDetails));
      return response.result.value;
    },
    close() { socket.close(); },
  };
}
const fixture = http.createServer((req, res) => {
  if (req.url === '/redirect') { res.writeHead(302, { Location: '/second' }); res.end(); return; }
  res.setHeader('Content-Type', 'text/html');
  res.end(`<title>Browser fixture</title><input id="draft"><a href="/second">Next</a><button onclick="window.open('/popup')">Popup</button><p>${req.url}</p>`);
});
await new Promise((resolve) => fixture.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${fixture.address().port}`;
const mainTarget = (await targets()).find((target) => target.url.startsWith('http://localhost:1420') || target.url.startsWith('http://tauri.localhost') || target.url.startsWith('tauri://'));
assert.ok(mainTarget, 'Main application target must be available');
const main = await connect(mainTarget);
const ids = ['smoke-native-a', 'smoke-native-b'];
const command = (id, action, fields = {}) => main.eval(`window.__TAURI_INTERNALS__.invoke('browser_command', ${JSON.stringify({ request: { id, action, ...fields } })})`);
const fixtureTargets = async () => (await targets()).filter((target) => target.url.startsWith(origin));
try {
  assert.equal((await fixtureTargets()).length, 0, 'No eager browser allocation');
  await command(ids[0], 'create', { url: `${origin}/first` });
  await waitFor(fixtureTargets, (list) => list.length === 1, 'First webview was not created');
  await command(ids[0], 'create', { url: `${origin}/first` });
  assert.equal((await fixtureTargets()).length, 1, 'Create must be idempotent');
  await command(ids[1], 'create', { url: `${origin}/second` });
  const views = await waitFor(fixtureTargets, (list) => list.length === 2, 'Second webview was not created');
  const page = await connect(views.find((target) => target.url.endsWith('/first')));
  try {
    await page.eval("document.querySelector('#draft').value = 'retained draft'");
    await command(ids[0], 'layout', { visible: false });
    await command(ids[0], 'layout', { visible: true, x: 100, y: 120, width: 375, height: 400 });
    assert.equal(await page.eval("document.querySelector('#draft').value"), 'retained draft', 'Hide/show preserves page state');
    for (const action of ['browser_command', 'custos_http_endpoint', 'custos_request', 'custos_dispatch', 'greet']) {
      const result = await page.eval(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(action)}, ${JSON.stringify({ request: { id: ids[0], action: 'state' }, method: 'v1.sessions.list', params: {}, rawJson: '{}', name: 'test' })}).then(() => 'ALLOWED', error => String(error))`);
      assert.match(result, /main application webview|not allowed|denied/i, `${action} must be denied to browser content`);
    }
    await page.eval("document.querySelector('button').click()");
    await delay(300);
    assert.equal((await fixtureTargets()).length, 2, 'Popup must not allocate a new webview');
  } finally { page.close(); }
  for (const url of ['file:///secret', 'javascript:alert(1)', 'tauri://localhost', 'data:text/html,test']) {
    await assert.rejects(() => command(ids[0], 'navigate', { url }), /Only HTTP|Invalid browser/);
  }
  await command(ids[0], 'navigate', { url: `${origin}/redirect` });
  await waitFor(() => command(ids[0], 'state'), (url) => url === `${origin}/second`, 'Redirect URL did not synchronize');
  await command(ids[0], 'back');
  await waitFor(() => command(ids[0], 'state'), (url) => url === `${origin}/first`, 'Native Back failed');
  await command(ids[0], 'forward');
  await waitFor(() => command(ids[0], 'state'), (url) => url === `${origin}/second`, 'Native Forward failed');
  await command(ids[0], 'reload');
  await command(ids[0], 'zoom', { zoom: 1.25 });
  for (let index = 0; index < 5; index++) {
    await command(ids[0], 'close');
    await command(ids[0], 'close');
    await command(ids[0], 'create', { url: `${origin}/first` });
    await waitFor(fixtureTargets, (list) => list.length === 2, 'Close/create leaked or lost a webview');
  }
  // Concurrent create/close tests the host lock independently of frontend queues.
  await command(ids[0], 'close');
  await Promise.all([command(ids[0], 'create', { url: `${origin}/first` }), command(ids[0], 'close')]);
  await command(ids[0], 'close');
  await command(ids[1], 'close');
  await waitFor(fixtureTargets, (list) => list.length === 0, 'Closed webviews remain alive');
  console.log('PASS: native lifecycle, retained state, history, redirects, popup, URL schemes and IPC isolation');
} finally {
  for (const id of ids) await command(id, 'close').catch(() => {});
  main.close();
  fixture.closeAllConnections();
  fixture.close();
}
