#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const targetDir = path.resolve(__dirname, '../../../crates/custos-app/desktop');

let tauriJs = path.resolve(__dirname, '../node_modules/@tauri-apps/cli/tauri.js');
if (!fs.existsSync(tauriJs)) {
  tauriJs = path.resolve(__dirname, '../../cli/node_modules/@tauri-apps/cli/tauri.js');
}

// Change working directory to the Tauri backend crate
process.chdir(targetDir);

// Execute Tauri CLI directly
await import(pathToFileURL(tauriJs).href);
