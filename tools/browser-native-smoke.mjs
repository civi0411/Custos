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
  res.end(`<title>Browser fixture</title><input id="draft"><a href="/second">Next</a><button onclick="window.open('/popup')">Popup</button><div id="player" style="width:120px;height:80px;background:black"><video></video></div><p>${req.url}</p>`);
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
  await command(ids[0], 'create', { url: `${origin}/first`, contain_fullscreen: true });
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
    const windowFullscreen = () => main.eval("window.__TAURI_INTERNALS__.invoke('plugin:window|is_fullscreen', { label: 'main' })");
    const originalFullscreen = await windowFullscreen();
    const originalStyle = await page.eval("document.querySelector('#player').style.cssText");
    await page.eval("window.__fullscreenChanges=[];window.__fullscreenResolved=false;document.addEventListener('fullscreenchange',()=>window.__fullscreenChanges.push(window.__fullscreenResolved));document.querySelector('#player').requestFullscreen().then(()=>{window.__fullscreenResolved=true;})");
    await waitFor(() => page.eval('window.__fullscreenChanges[0]'), value => value === true, 'Fullscreen change must follow promise resolution');
    assert.equal(await page.eval("document.fullscreenElement?.id"), 'player', 'Player enters scoped fullscreen');
    assert.deepEqual(await page.eval("({width:document.fullscreenElement.getBoundingClientRect().width,height:document.fullscreenElement.getBoundingClientRect().height})"), { width: 375, height: 400 }, 'Player fills only the native child viewport');
    assert.equal(await windowFullscreen(), originalFullscreen, 'Player must not fullscreen the host window');
    await page.eval("document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}))");
    assert.equal(await page.eval('document.fullscreenElement'), null, 'Escape exits scoped fullscreen');
    assert.equal(await page.eval("document.querySelector('#player').style.cssText"), originalStyle, 'Exit restores page layout');
    await page.eval("document.querySelector('#player').requestFullscreen().then(()=>document.exitFullscreen())");
    assert.equal(await page.eval('document.fullscreenElement'), null, 'Fullscreen API exit restores state');
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
  await assert.rejects(() => command('smoke-invalid-ua', 'create', { url: `${origin}/first`, user_agent: 'bad\nagent' }), /Invalid browser user agent/);
  await command(ids[0], 'close');
  const mobileAgent = 'Custos Mobile Preview fixture';
  await command(ids[0], 'create', { url: `${origin}/first`, user_agent: mobileAgent });
  const mobileTargets = await waitFor(fixtureTargets, list => list.length === 2, 'User-agent webview was not created');
  const mobile = await connect(mobileTargets.find(target => target.url.endsWith('/first')));
  try {
    await waitFor(() => mobile.eval('document.readyState'), state => state === 'complete', 'Mobile fixture did not load');
    assert.equal(await mobile.eval('navigator.userAgent'), mobileAgent, 'Create must apply the requested user agent');
  } finally { mobile.close(); }
  // Concurrent create/close tests the host lock independently of frontend queues.
  await command(ids[0], 'close');
  await Promise.all([command(ids[0], 'create', { url: `${origin}/first` }), command(ids[0], 'close')]);
  await command(ids[0], 'close');
  await command(ids[1], 'close');
  await waitFor(fixtureTargets, (list) => list.length === 0, 'Closed webviews remain alive');
  console.log('PASS: native lifecycle, scoped fullscreen/exit, retained state, history, redirects, popup, URL schemes, user agents and IPC isolation');
} finally {
  for (const id of ids) await command(id, 'close').catch(() => {});
  main.close();
  fixture.closeAllConnections();
  fixture.close();
}
