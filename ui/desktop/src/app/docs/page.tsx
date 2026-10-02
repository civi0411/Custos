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
    <div className="flex-1 flex flex-col h-full bg-canvas overflow-y-auto p-4 sm:p-6 lg:p-8 space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-surface-border">
        <div>
          <div className="flex items-center gap-2.5">
            <div className="p-2 rounded-xl bg-brand-blue/10 border border-brand-blue/30 text-brand-blue">
              <BookOpen className="w-5 h-5" />
            </div>
            <div>
              <h1 className="text-lg font-bold text-white tracking-tight">Custos Local API Documentation</h1>
              <p className="text-xs text-neutral-400">REST & IPC specification for local IDE agents, CLI, and desktop clients</p>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2 text-xs font-mono text-neutral-400 bg-surface-card px-3 py-1.5 rounded-lg border border-surface-border">
          <span>Base URL:</span>
          <span className="text-brand-cyan">http://127.0.0.1:4140</span>
        </div>
      </div>

      {/* Endpoints List */}
      <div className="space-y-4">
        {endpoints.map((ep, idx) => (
          <div key={ep.path} className="p-5 bg-surface-card border border-surface-border rounded-xl space-y-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="flex items-center gap-2.5">
                <span
                  className={`text-[11px] font-mono font-bold px-2 py-0.5 rounded ${
                    ep.method === 'GET'
                      ? 'bg-blue-500/10 text-blue-400 border border-blue-500/30'
                      : 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/30'
                  }`}
                >
                  {ep.method}
                </span>
                <span className="font-mono text-sm font-semibold text-white">{ep.path}</span>
              </div>

              <button
                onClick={() => handleCopy(ep.curl, idx)}
                className="px-2.5 py-1 bg-surface-elevated hover:bg-surface-hover border border-surface-border rounded-md text-xs text-neutral-300 transition flex items-center gap-1.5"
              >
                {copiedIndex === idx ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                <span>{copiedIndex === idx ? 'Copied' : 'Copy cURL'}</span>
              </button>
            </div>

            <p className="text-xs text-neutral-300">{ep.desc}</p>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-2">
              {ep.requestBody && (
                <div>
                  <span className="text-[11px] font-mono text-neutral-500 block mb-1">Request Payload</span>
                  <pre className="bg-canvas/60 p-3 rounded-lg border border-surface-border text-[11px] font-mono text-neutral-300 overflow-x-auto">
                    {ep.requestBody}
                  </pre>
                </div>
              )}
              <div className={ep.requestBody ? '' : 'md:col-span-2'}>
                <span className="text-[11px] font-mono text-neutral-500 block mb-1">Response (200 OK)</span>
                <pre className="bg-canvas/60 p-3 rounded-lg border border-surface-border text-[11px] font-mono text-emerald-400/90 overflow-x-auto">
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
