import assert from 'node:assert/strict';
import fs from 'node:fs';
import { createRequire } from 'node:module';
import test from 'node:test';
import vm from 'node:vm';

// Exercise the exported presentation functions with the existing desktop compiler.
const require = createRequire(new URL('../ui/desktop/package.json', import.meta.url));
const ts = require('typescript');
const source = fs.readFileSync(new URL('../ui/desktop/src/components/chat/ModelSelector.tsx', import.meta.url), 'utf8');
const parsed = ts.createSourceFile('ModelSelector.tsx', source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
const isolated = ts.factory.updateSourceFile(parsed, parsed.statements.filter(statement => !ts.isImportDeclaration(statement)));
const compiled = ts.transpileModule(ts.createPrinter().printFile(isolated), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.React },
}).outputText;
const api = {};
vm.runInNewContext(compiled, { exports: api });
const { getConnectedModelOptions, groupModelOptions, normalizeModelPresentation, resolveModelRoute, compareModelPresentation } = api;

const account = (id, models, overrides = {}) => ({
  id, providerId: 'openai', providerName: 'OpenAI', accountName: id,
  endpointUrl: 'https://api.openai.com/v1', authType: 'oauth', status: 'active',
  supportedModels: models.map(model => model.id), catalogModels: models, ...overrides,
});
const groups = accounts => groupModelOptions(getConnectedModelOptions(accounts));

test('duplicate OpenAI accounts share one row and retain exact routes', () => {
  const model = { id: 'gpt-6.1-sol', label: 'GPT-6.1-Sol', efforts: ['low', 'xhigh', 'max', 'ultra'] };
  const result = groups([account('A', [model, model]), account('B', [{ ...model, label: 'Different account label' }])]);
  assert.equal(result.length, 1);
  assert.equal(result[0].length, 2);
  for (const effort of model.efforts) assert.equal(resolveModelRoute(result[0], 'B', effort).accountId, 'B');
  assert.equal(resolveModelRoute(result[0], 'A', 'invalid'), undefined);
  assert.equal(resolveModelRoute(result[0], 'missing', 'low'), undefined);
});

test('catalog effort labels override misleading Antigravity suffixes', () => {
  const models = [
    { id: 'antigravity/gemini-3-flash-agent', label: 'Gemini 3.5 Flash (High)' },
    { id: 'antigravity/gemini-3.5-flash-extra-low', label: 'Gemini 3.5 Flash (Low)' },
    { id: 'antigravity/gemini-3.5-flash-low', label: 'Gemini 3.5 Flash (Medium)' },
  ];
  const result = groups([account('G', models, { providerId: 'antigravity' })]);
  assert.equal(result.length, 1);
  assert.equal(resolveModelRoute(result[0], 'G', 'medium').id, models[2].id);
  assert.equal(resolveModelRoute(result[0], 'G', 'low').id, models[1].id);
  assert.equal(resolveModelRoute(result[0], 'G', 'high').id, models[0].id);
  assert.equal(resolveModelRoute(result[0], 'G', 'ultra'), undefined);
});

test('catalog aliases merge while preferring the canonical discovered ID', () => {
  const result = groups([account('G', [
    { id: 'antigravity/gemini-2.5-flash', label: 'Gemini 3.5 Flash Lite' },
    { id: 'antigravity/gemini-3.5-flash-lite', label: 'Gemini 3.5 Flash Lite' },
  ], { providerId: 'antigravity' })]);
  assert.equal(result.length, 1);
  assert.equal(resolveModelRoute(result[0], 'G', '').id, 'antigravity/gemini-3.5-flash-lite');
  assert.equal(resolveModelRoute(result[0], 'G', '', 'antigravity/gemini-2.5-flash').id, 'antigravity/gemini-2.5-flash');
});

test('endpoint and auth remain route properties; offline accounts are excluded', () => {
  const models = [{ id: 'gpt-6-sol', label: 'GPT-6-Sol' }];
  const result = groups([
    account('A', models), account('B', models, { authType: 'api_key' }),
    account('C', models, { endpointUrl: 'https://other.example/v1' }),
    account('D', models, { status: 'offline' }),
  ]);
  assert.equal(result.length, 1);
  assert.equal(result[0].length, 3);
  assert.equal(resolveModelRoute(result[0], 'D', ''), undefined);
});

test('versions, snapshots and unknown IDs keep distinct identities', () => {
  const models = ['gpt-6-sol', 'gpt-6.1-sol', 'gpt-6-sol-2026-10-01'].map(id => ({ id, label: 'Shared label' }));
  assert.equal(groups([account('A', models)]).length, 3);
  assert.notEqual(normalizeModelPresentation('claude-sonnet-4-6', undefined, 'anthropic').name,
    normalizeModelPresentation('claude-sonnet-5-5', undefined, 'anthropic').name);
  assert.equal(normalizeModelPresentation('antigravity/new-model-ultra', undefined, 'antigravity').variantEffort, 'ultra');
});

test('tiered display keeps the real ID and Gemini sorts ahead of Claude and GPT-OSS', () => {
  const models = ['Claude Opus 4.6', 'GPT-OSS 120B', 'Gemini 2.5 Pro', 'Gemini 3.8 Flash', 'Claude Sonnet 4.6'].map(name => ({ name }));
  models.sort(compareModelPresentation);
  assert.equal(models[0].name, 'Gemini 3.8 Flash');
  assert.equal(models[1].name, 'Gemini 2.5 Pro');
  const model = normalizeModelPresentation('antigravity/gemini-3.8-flash-tiered', undefined, 'antigravity');
  assert.equal(model.name, 'Gemini 3.8 Flash');
  assert.equal(model.variantEffort, undefined);
});
