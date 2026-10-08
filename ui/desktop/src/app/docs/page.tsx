import React, { useState } from 'react';
import { BookOpen, Copy, Check } from 'lucide-react';
import { useAppContext } from '../../context/AppContext';

interface ApiEndpoint {
  method: 'GET' | 'POST' | 'DELETE';
  path: string;
  desc: string;
  requestBody?: string;
  response: string;
  curl: string;
}

const endpoints: ApiEndpoint[] = [
  {
    method: 'GET',
    path: '/health',
    desc: 'Returns daemon health, uptime, and loaded kernel state.',
    response: '{\n  "status": "healthy",\n  "version": "0.1.0",\n  "uptime_secs": 1820,\n  "kernel": "active"\n}',
    curl: 'curl -X GET http://127.0.0.1:4140/health'
  },
  {
    method: 'POST',
    path: '/api/v1/tasks',
    desc: 'Submits a new task transition request to Custos Task Kernel with ActionIntent.',
    requestBody: '{\n  "project": "Default project",\n  "intent": {\n    "title": "Refactor Auth Middleware",\n    "prompt": "Cache JWT in SQLite"\n  }\n}',
    response: '{\n  "task_id": "task-auth-09",\n  "status": "Draft",\n  "created_at": "2026-10-02T15:09:41Z"\n}',
    curl: 'curl -X POST http://127.0.0.1:4140/api/v1/tasks \\\n  -H "Content-Type: application/json" \\\n  -d \'{"project":"Default project","intent":{"title":"Refactor Auth Middleware"}}\''
  },
  {
    method: 'GET',
    path: '/api/v1/sessions',
    desc: 'Lists all active workspace sessions and their latest diff hunk state.',
    response: '{\n  "sessions": [\n    {\n      "id": "auth",\n      "title": "Refactor Auth Middleware",\n      "model": "Claude 3.7 Sonnet",\n      "file": "src/middleware/gateway.ts"\n    }\n  ]\n}',
    curl: 'curl -X GET http://127.0.0.1:4140/api/v1/sessions'
  },
  {
    method: 'POST',
    path: '/api/v1/permits',
    desc: 'Requests a sandboxed ExecutionPermit before mutating repository files.',
    requestBody: '{\n  "task_id": "task-auth-09",\n  "file_path": "src/middleware/gateway.ts",\n  "action": "write"\n}',
    response: '{\n  "permit_id": "pmt_8810",\n  "granted": true,\n  "sandbox": "bwrap_isolated"\n}',
    curl: 'curl -X POST http://127.0.0.1:4140/api/v1/permits \\\n  -H "Content-Type: application/json" \\\n  -d \'{"task_id":"task-auth-09","action":"write"}\''
  }
];

export const DocsPage: React.FC = () => {
  const { showToast } = useAppContext();
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);

  const handleCopy = (text: string, index: number) => {
    navigator.clipboard.writeText(text);
    setCopiedIndex(index);
    showToast('cURL command copied to clipboard');
    setTimeout(() => setCopiedIndex(null), 2000);
  };

  return (
    <div className="flex-1 flex flex-col h-full bg-canvas overflow-y-auto p-4 sm:p-6 lg:p-8 space-y-6 text-fg-editor font-sans">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-border-muted">
        <div>
          <div className="flex items-center gap-2.5">
            <div className="w-9 h-9 rounded-xl bg-surface-2 border border-border-default flex items-center justify-center workbench-accent">
              <BookOpen className="w-5 h-5" />
            </div>
            <div>
              <h1 className="text-base sm:text-lg font-semibold tracking-[-0.02em] text-fg-editor">Custos Local API Documentation</h1>
              <p className="text-xs text-fg-muted mt-0.5">REST & IPC specification for local IDE agents, CLI, and desktop clients</p>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2 text-xs font-mono text-fg-muted bg-surface-1 px-3 py-1.5 rounded-lg border border-border-default">
          <span>Base URL:</span>
          <span className="workbench-accent font-semibold">http://127.0.0.1:4140</span>
        </div>
      </div>

      {/* Endpoints List */}
      <div className="space-y-4">
        {endpoints.map((ep, idx) => (
          <div key={ep.path} className="p-5 bg-surface-1 border border-border-default rounded-xl space-y-3 shadow-xs">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="flex items-center gap-2.5">
                <span
                  className={`text-[11px] font-mono font-semibold px-2 py-0.5 rounded border ${
                    ep.method === 'GET'
                      ? 'bg-surface-2 text-fg-editor border-border-default'
                      : 'bg-emerald-500/10 text-emerald-500 border-emerald-500/30'
                  }`}
                >
                  {ep.method}
                </span>
                <span className="font-mono text-sm font-semibold text-fg-editor">{ep.path}</span>
              </div>

              <button
                onClick={() => handleCopy(ep.curl, idx)}
                className="px-2.5 py-1 bg-surface-2 hover:bg-surface-3 border border-border-default rounded-md text-xs text-fg-editor transition flex items-center gap-1.5"
              >
                {copiedIndex === idx ? <Check className="w-3.5 h-3.5 text-emerald-500" /> : <Copy className="w-3.5 h-3.5 text-fg-muted" />}
                <span>{copiedIndex === idx ? 'Copied' : 'Copy cURL'}</span>
              </button>
            </div>

            <p className="text-xs text-fg-muted">{ep.desc}</p>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-2">
              {ep.requestBody && (
                <div>
                  <span className="text-[11px] font-mono text-fg-subtle block mb-1">Request Payload</span>
                  <pre className="bg-surface-0 p-3 rounded-lg border border-border-muted text-[11px] font-mono text-fg-editor overflow-x-auto">
                    {ep.requestBody}
                  </pre>
                </div>
              )}
              <div className={ep.requestBody ? '' : 'md:col-span-2'}>
                <span className="text-[11px] font-mono text-fg-subtle block mb-1">Response (200 OK)</span>
                <pre className="bg-surface-0 p-3 rounded-lg border border-border-muted text-[11px] font-mono text-emerald-500/90 overflow-x-auto">
                  {ep.response}
                </pre>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default DocsPage;
