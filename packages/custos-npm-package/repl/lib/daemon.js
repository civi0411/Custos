import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { spawn } from 'node:child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export const VERSION = '0.1.1';
let daemonPortPromise = null;

export function getProfileDir() {
  if (process.env.CUSTOS_PROFILE_DIR) return process.env.CUSTOS_PROFILE_DIR;
  const profile = process.env.CUSTOS_PROFILE || 'default';
  const root = process.env.CUSTOS_HOME || path.join(os.homedir(), '.custos');
  return path.join(root, 'profiles', profile);
}

export function readDaemonHttpPort() {
  try {
    return Number.parseInt(fs.readFileSync(path.join(getProfileDir(), 'daemon.http_port'), 'utf8').trim(), 10) || null;
  } catch {
    return null;
  }
}

export async function daemonHealth(port, timeoutMs = 350) {
  if (!port) return false;
  try {
    const response = await fetch(`http://127.0.0.1:${port}/api/health`, {
      signal: AbortSignal.timeout(timeoutMs),
    });
    return response.ok;
  } catch {
    return false;
  }
}

async function waitForDaemonHttp(timeoutMs = 900) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const port = readDaemonHttpPort();
    if (port && await daemonHealth(port)) return port;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  return null;
}

function daemonCandidates() {
  const executable = process.platform === 'win32' ? 'custos-daemon.exe' : 'custos-daemon';
  const workspaceRoot = path.resolve(__dirname, '..', '..', '..', '..');
  return [
    process.env.CUSTOS_DAEMON_BIN,
    path.join(workspaceRoot, 'target', 'debug', executable),
    path.join(workspaceRoot, 'target', 'release', executable),
    'custos-daemon',
  ].filter(Boolean);
}

async function ensureDaemonHttpPort() {
  const existing = readDaemonHttpPort();
  if (existing && await daemonHealth(existing)) return existing;

  const candidates = daemonCandidates();
  const daemonBinary = candidates.find((candidate) => candidate === 'custos-daemon' || fs.existsSync(candidate));
  if (!daemonBinary) {
    throw new Error('Backend chưa được cài. Hãy build custos-daemon hoặc đặt CUSTOS_DAEMON_BIN. Chat thường vẫn dùng được.');
  }

  let launchFailed = false;
  try {
    const child = spawn(daemonBinary, [], { detached: true, stdio: 'ignore', windowsHide: true });
    child.once('error', () => { launchFailed = true; });
    child.unref();
  } catch {
    launchFailed = true;
  }

  const workspaceBinary = path.isAbsolute(daemonBinary) && fs.existsSync(daemonBinary);
  const port = await waitForDaemonHttp(launchFailed ? 250 : workspaceBinary ? 5000 : 900);
  if (port) return port;
  throw new Error(`Backend chưa sẵn sàng (${daemonBinary}). Hãy build/chạy custos-daemon hoặc đặt CUSTOS_DAEMON_BIN. Chat thường vẫn dùng được.`);
}

export async function daemonRequest(method, params = {}) {
  if (!daemonPortPromise) {
    daemonPortPromise = ensureDaemonHttpPort().catch((error) => {
      daemonPortPromise = null;
      throw error;
    });
  }
  const port = await daemonPortPromise;
  let response;
  try {
    response = await fetch(`http://127.0.0.1:${port}/api/request`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ id: `cli_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`, method, params }),
    });
  } catch {
    daemonPortPromise = null;
    throw new Error('Mất kết nối với custos-daemon. Chat thường vẫn dùng được; hãy khởi động lại backend để dùng provider.');
  }
  if (!response.ok) throw new Error(`Daemon HTTP ${response.status}`);
  const envelope = await response.json();
  if (envelope.error) throw new Error(envelope.error);
  return envelope.result;
}

export function openExternalUrl(url) {
  const command = process.platform === 'win32' ? 'rundll32' : process.platform === 'darwin' ? 'open' : 'xdg-open';
  const args = process.platform === 'win32' ? ['url.dll,FileProtocolHandler', url] : [url];
  const child = spawn(command, args, { detached: true, stdio: 'ignore', windowsHide: true });
  child.on('error', () => {});
  child.unref();
}

