import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

const scripts = path.dirname(fileURLToPath(import.meta.url));
function fixture(t, script) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'custos-boundaries-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  fs.mkdirSync(path.join(root, 'xtask/scripts'), { recursive: true });
  const target = path.join(root, 'xtask/scripts', script);
  fs.copyFileSync(path.join(scripts, script), target);
  return { root, run: () => spawnSync('bash', [target], { encoding: 'utf8', timeout: 30000 }) };
}

test('layer guard fails on missing roots and forbidden source instead of reporting success', t => {
  const { root, run } = fixture(t, 'check_layering.sh');
  assert.equal(run().status, 1);
  for (const folder of ['domain/src', 'core/src', 'runtime/src/oi']) {
    fs.mkdirSync(path.join(root, 'platform', folder), { recursive: true });
  }
  assert.equal(run().status, 0);
  fs.writeFileSync(path.join(root, 'platform/domain/src/lib.rs'), 'use std::fs;\n');
  assert.equal(run().status, 1);
  fs.writeFileSync(path.join(root, 'platform/domain/src/lib.rs'), '');
  fs.writeFileSync(path.join(root, 'platform/core/src/lib.rs'), 'use custos_adapters::documents;\n');
  assert.equal(run().status, 1);
});

test('dependency guard detects adapter to application imports in the new layout', t => {
  const { root, run } = fixture(t, 'check_deps.sh');
  const members = ['platform/domain', 'platform/adapters', 'apps/daemon'];
  fs.writeFileSync(path.join(root, 'Cargo.toml'), `[workspace]\nresolver = "2"\nmembers = ${JSON.stringify(members)}\n`);
  for (const [index, member] of members.entries()) {
    const folder = path.join(root, member);
    fs.mkdirSync(path.join(folder, 'src'), { recursive: true });
    fs.writeFileSync(path.join(folder, 'src/lib.rs'), '');
    fs.writeFileSync(path.join(folder, 'Cargo.toml'), `[package]\nname = "${['custos-domain', 'custos-adapters', 'custos-daemon'][index]}"\nversion = "0.0.0"\nedition = "2021"\n`);
  }
  let result = run();
  assert.equal(result.status, 0, result.stderr);
  fs.appendFileSync(path.join(root, 'platform/adapters/Cargo.toml'), '\n[dependencies]\ncustos-daemon = { path = "../../apps/daemon" }\n');
  result = run();
  assert.equal(result.status, 1, result.stderr);
  assert.match(result.stderr, /custos-adapters -> custos-daemon/);
});
