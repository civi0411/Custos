#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const os = require('os');
const fs = require('fs');
const https = require('https');
const http = require('http');

let pkg = { version: '0.1.1', name: 'custos-cli' };
try {
  pkg = require('../package.json');
} catch {}

const currentVersion = pkg.version;
const packageName = pkg.name || 'custos-cli';
const userArgs = process.argv.slice(2);
const isInstallOnly = userArgs.includes('--install-only');

function parseSemver(v) {
  if (typeof v !== 'string') return null;
  const clean = v.replace(/^v/, '').trim();
  const [verPart, prePart] = clean.split('-');
  const parts = verPart.split('.').map(x => parseInt(x, 10) || 0);
  while (parts.length < 3) parts.push(0);
  return {
    major: parts[0],
    minor: parts[1],
    patch: parts[2],
    prerelease: prePart || null
  };
}

function isNewerVersion(latest, current) {
  const l = parseSemver(latest);
  const c = parseSemver(current);
  if (!l || !c) return false;
  if (l.major > c.major) return true;
  if (l.major < c.major) return false;
  if (l.minor > c.minor) return true;
  if (l.minor < c.minor) return false;
  if (l.patch > c.patch) return true;
  if (l.patch < c.patch) return false;
  if (c.prerelease && !l.prerelease) return true;
  if (c.prerelease && l.prerelease && c.prerelease !== l.prerelease) {
    return l.prerelease.localeCompare(c.prerelease) > 0;
  }
  return false;
}

function stripAnsi(str) {
  return str.replace(/\x1b\[[0-9;]*m/g, '');
}

function padLine(str, width) {
  const visibleLen = stripAnsi(str).length;
  const padding = Math.max(0, width - visibleLen);
  return str + ' '.repeat(padding);
}

function printUpdateBanner(current, latest) {
  const c = {
    reset: '\x1b[0m',
    bold: '\x1b[1m',
    dim: '\x1b[2m',
    yellow: '\x1b[33m',
    green: '\x1b[32m',
    cyan: '\x1b[36m',
    gray: '\x1b[90m'
  };

  const innerWidth = 67;
  const line1 = '   Update available ' + c.dim + current + c.reset + ' ' + c.gray + '→' + c.reset + ' ' + c.green + c.bold + latest + c.reset;
  const line2 = '   Run ' + c.cyan + c.bold + `npm i -g ${packageName}` + c.reset + ' to update to the latest version';
  const line3 = '   Changelog: ' + c.cyan + 'https://github.com/civi0411/Custos/releases' + c.reset;

  const top = c.yellow + '╭' + '─'.repeat(innerWidth) + '╮' + c.reset;
  const empty = c.yellow + '│' + c.reset + ' '.repeat(innerWidth) + c.yellow + '│' + c.reset;
  const bot = c.yellow + '╰' + '─'.repeat(innerWidth) + '╯' + c.reset;

  console.log('');
  console.log(top);
  console.log(empty);
  console.log(c.yellow + '│' + c.reset + padLine(line1, innerWidth) + c.yellow + '│' + c.reset);
  console.log(c.yellow + '│' + c.reset + padLine(line2, innerWidth) + c.yellow + '│' + c.reset);
  console.log(empty);
  console.log(c.yellow + '│' + c.reset + padLine(line3, innerWidth) + c.yellow + '│' + c.reset);
  console.log(empty);
  console.log(bot);
  console.log('');
}

const cacheDir = path.join(os.homedir(), '.custos');
const cacheFile = path.join(cacheDir, 'update-check.json');
const CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;

let pendingLatestVersion = null;

if (process.env.CUSTOS_SIMULATE_UPDATE) {
  pendingLatestVersion = process.env.CUSTOS_SIMULATE_UPDATE.trim();
} else {
  try {
    if (fs.existsSync(cacheFile)) {
      const cache = JSON.parse(fs.readFileSync(cacheFile, 'utf8'));
      if (cache && cache.latestVersion) {
        pendingLatestVersion = cache.latestVersion;
      }
      const isExpired = !cache.lastChecked || (Date.now() - cache.lastChecked > CHECK_INTERVAL_MS);
      if (isExpired) {
        checkRemoteVersion();
      }
    } else {
      checkRemoteVersion();
    }
  } catch {
    checkRemoteVersion();
  }
}

function checkRemoteVersion() {
  const checkUrl = process.env.CUSTOS_UPDATE_CHECK_URL || `https://registry.npmjs.org/-/package/${packageName}/dist-tags`;
  try {
    const req = https.get(checkUrl, {
      headers: { 'User-Agent': 'custos-cli-updater' },
      timeout: 1500
    }, (res) => {
      if (res.statusCode !== 200) {
        res.resume();
        return;
      }
      let rawData = '';
      res.on('data', chunk => rawData += chunk);
      res.on('end', () => {
        try {
          const json = JSON.parse(rawData);
          const latest = json.latest || json.version;
          if (latest) {
            pendingLatestVersion = latest;
            try {
              if (!fs.existsSync(cacheDir)) {
                fs.mkdirSync(cacheDir, { recursive: true });
              }
              fs.writeFileSync(cacheFile, JSON.stringify({
                lastChecked: Date.now(),
                latestVersion: latest
              }, null, 2), 'utf8');
            } catch {}
          }
        } catch {}
      });
    });

    req.on('timeout', () => req.destroy());
    req.on('error', () => {});
  } catch {}
}

function getBinaryInfo() {
  const platform = os.platform();
  const arch = os.arch();
  let target = '';
  let ext = '';

  if (platform === 'win32') {
    target = 'windows-x64';
    ext = '.exe';
  } else if (platform === 'darwin') {
    target = arch === 'arm64' ? 'darwin-arm64' : 'darwin-x64';
  } else if (platform === 'linux') {
    target = arch === 'arm64' ? 'linux-arm64' : 'linux-x64';
  } else {
    target = `${platform}-${arch}`;
  }

  const remoteName = `custos-${target}${ext}`;
  const localName = platform === 'win32' ? 'custos.exe' : 'custos';
  return { platform, arch, target, remoteName, localName, isWindows: platform === 'win32' };
}

function downloadFile(url, destPath, callback, maxRedirects = 5) {
  if (maxRedirects <= 0) return callback(new Error('Too many redirects'));
  const client = url.startsWith('https') ? https : http;
  const tempPath = `${destPath}.tmp-${Date.now()}`;

  const req = client.get(url, { headers: { 'User-Agent': 'custos-cli-installer' } }, (res) => {
    if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
      res.resume();
      const nextUrl = new URL(res.headers.location, url).toString();
      return downloadFile(nextUrl, destPath, callback, maxRedirects - 1);
    }
    if (res.statusCode !== 200) {
      res.resume();
      return callback(new Error(`HTTP ${res.statusCode} ${res.statusMessage}`));
    }

    const totalBytes = parseInt(res.headers['content-length'], 10) || 0;
    let receivedBytes = 0;
    const file = fs.createWriteStream(tempPath);

    res.on('data', (chunk) => {
      receivedBytes += chunk.length;
      if (totalBytes && process.stdout.isTTY && !isInstallOnly) {
        const pct = Math.floor((receivedBytes / totalBytes) * 100);
        const mbRec = (receivedBytes / (1024 * 1024)).toFixed(1);
        const mbTot = (totalBytes / (1024 * 1024)).toFixed(1);
        process.stdout.write(`\r[Custos] Downloading binary: ${pct}% (${mbRec}/${mbTot} MB)`);
      }
    });

    res.pipe(file);

    file.on('finish', () => {
      if (totalBytes && process.stdout.isTTY && !isInstallOnly) {
        process.stdout.write('\r[Custos] Downloading binary: 100% Done!          \n');
      }
      file.close(() => {
        try {
          fs.renameSync(tempPath, destPath);
          callback(null);
        } catch (renameErr) {
          callback(renameErr);
        }
      });
    });

    file.on('error', (err) => {
      try { fs.unlinkSync(tempPath); } catch {}
      callback(err);
    });
  });

  req.on('error', (err) => {
    try { fs.unlinkSync(tempPath); } catch {}
    callback(err);
  });
}

function downloadBinaryWithFallback(url, fallbackUrl, destPath, callback) {
  downloadFile(url, destPath, (err) => {
    if (!err) return callback(null);
    if (!fallbackUrl || fallbackUrl === url) return callback(err);
    downloadFile(fallbackUrl, destPath, callback);
  });
}

function resolveBinary(callback) {
  const info = getBinaryInfo();
  const candidates = [
    path.join(__dirname, info.localName),
    path.join(__dirname, info.remoteName),
    path.join(cacheDir, 'bin', info.remoteName),
    path.join(cacheDir, 'bin', info.localName)
  ];

  for (const p of candidates) {
    if (fs.existsSync(p)) {
      if (!info.isWindows) {
        try { fs.chmodSync(p, 0o755); } catch {}
      }
      return callback(null, p);
    }
  }

  const binDir = path.join(cacheDir, 'bin');
  try {
    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }
  } catch {}

  const destPath = path.join(binDir, info.remoteName);
  const downloadUrl = `https://github.com/civi0411/Custos/releases/download/v${currentVersion}/${info.remoteName}`;
  const fallbackUrl = `https://github.com/civi0411/Custos/releases/latest/download/${info.remoteName}`;

  if (!isInstallOnly) {
    console.log(`[Custos] Native binary for ${info.target} not found locally.`);
    console.log(`[Custos] Downloading precompiled binary from GitHub Releases...`);
  }

  downloadBinaryWithFallback(downloadUrl, fallbackUrl, destPath, (err) => {
    if (err) {
      if (!isInstallOnly) {
        console.error(`[Custos Error] Could not automatically download binary:`, err.message);
        console.error(`[Custos] Please download '${info.remoteName}' manually from:`);
        console.error(`        https://github.com/civi0411/Custos/releases`);
        console.error(`        and place it at: ${destPath}`);
      }
      return callback(err);
    }
    if (!info.isWindows) {
      try { fs.chmodSync(destPath, 0o755); } catch {}
    }
    callback(null, destPath);
  });
}

function launch(binaryPath) {
  const childEnv = { ...process.env };
  childEnv.CUSTOS_PKG_NAME = packageName;
  if (pendingLatestVersion) {
    childEnv.CUSTOS_LATEST_VERSION = pendingLatestVersion;
  }

  const child = spawn(binaryPath, userArgs, {
    stdio: 'inherit',
    windowsHide: false,
    env: childEnv
  });

  child.on('error', (err) => {
    console.error('[Custos Error] Unable to launch binary:', err.message);
    console.error('Binary path:', binaryPath);
    process.exit(1);
  });

  child.on('exit', (code, signal) => {
    const isInteractiveVibe = userArgs.length === 0 || userArgs[0] === 'vibe';
    const isJson = userArgs.includes('--json');
    const isCI = !!process.env.CI;
    const isSuppressed = !!process.env.CUSTOS_NO_UPDATE_NOTIFIER;
    const isTTY = !!process.stdout.isTTY || !!process.env.CUSTOS_SIMULATE_UPDATE || !!process.env.CUSTOS_FORCE_UPDATE_NOTIFIER;

    if (!isInteractiveVibe && isTTY && !isJson && !isCI && !isSuppressed && pendingLatestVersion) {
      if (isNewerVersion(pendingLatestVersion, currentVersion)) {
        printUpdateBanner(currentVersion, pendingLatestVersion);
      }
    }

    if (signal) {
      try {
        process.kill(process.pid, signal);
      } catch {
        process.exit(1);
      }
    } else {
      process.exit(code ?? 0);
    }
  });
}

resolveBinary((err, binPath) => {
  if (err) {
    if (isInstallOnly) {
      console.warn(`[Custos] Notice: Could not download native binary during install (${err.message}).`);
      console.warn(`[Custos] It will be downloaded automatically on first run.`);
      process.exit(0);
    }
    process.exit(1);
  }
  if (isInstallOnly) {
    process.exit(0);
  }
  launch(binPath);
});
