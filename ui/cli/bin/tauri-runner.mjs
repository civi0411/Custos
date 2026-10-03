#!/usr/bin/env node
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const targetDir = path.resolve(__dirname, '../../../crates/custos-app/cli');
const tauriJs = path.resolve(__dirname, '../node_modules/@tauri-apps/cli/tauri.js');

// Change working directory to the Tauri backend crate
process.chdir(targetDir);

// Execute Tauri CLI directly
await import(pathToFileURL(tauriJs).href);
