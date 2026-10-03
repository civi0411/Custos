import React, { useState } from 'react';
import { Database, Search, Trash2, RefreshCw } from 'lucide-react';
import { useAppContext } from '../../context/AppContext';

interface CacheItem {
  key: string;
  category: 'JWT' | 'AST' | 'INDEX' | 'EVAL';
  ttl: string;
  size: string;
  lastAccess: string;
  valuePreview: string;
}

const mockCacheItems: CacheItem[] = [
  { key: 'jwt:alex_session_01', category: 'JWT', ttl: '240s', size: '1.2 KB', lastAccess: '12s ago', valuePreview: '{"sub":"usr_9918","role":"architect","exp":1772658400,"valid":true}' },
  { key: 'ast:compiler:parser.ts', category: 'AST', ttl: '1800s', size: '48.6 KB', lastAccess: '2m ago', valuePreview: '{"nodes":182,"identifiers":["StatementBlock","CallSignature"],"hash":"sha256:7f9"}' },
  { key: 'workspace:index:main', category: 'INDEX', ttl: '3600s', size: '184.2 KB', lastAccess: '14m ago', valuePreview: '{"files":46,"symbols":388,"crates":["custos-kernel","custos-domain"]}' },
  { key: 'eval:recipe:auth_flow', category: 'EVAL', ttl: '7200s', size: '8.4 KB', lastAccess: '1h ago', valuePreview: '{"recipeId":"auth-middleware-v2","assertions":5,"verdict":"PASSED"}' },
  { key: 'jwt:bot_worker_04', category: 'JWT', ttl: '52s', size: '1.1 KB', lastAccess: '30s ago', valuePreview: '{"sub":"bot_worker_04","capabilities":["read_repo","create_patch"]}' },
  { key: 'ast:middleware:gateway.ts', category: 'AST', ttl: '1200s', size: '32.1 KB', lastAccess: '45m ago', valuePreview: '{"functions":["verifySession","withFailover"],"imports":["localKvStore"]}' }
];

export const CachePage: React.FC = () => {
  const { showToast } = useAppContext();
  const [items, setItems] = useState<CacheItem[]>(mockCacheItems);
  const [search, setSearch] = useState('');
  const [selectedKey, setSelectedKey] = useState<CacheItem | null>(mockCacheItems[0]);

  const filtered = items.filter((i) =>
    i.key.toLowerCase().includes(search.toLowerCase()) ||
    i.category.toLowerCase().includes(search.toLowerCase())
  );

  const handleDeleteKey = (key: string) => {
    setItems((prev) => prev.filter((i) => i.key !== key));
    if (selectedKey?.key === key) setSelectedKey(null);
    showToast(`Key purged from SQLite: ${key}`);
  };

  const handleFlushAll = () => {
    setItems([]);
    setSelectedKey(null);
    showToast('SQLite KV Cache flushed');
  };

  return (
    <div className="flex-1 flex flex-col h-full bg-surface overflow-hidden p-4 sm:p-6 space-y-4">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-3 border-b border-surface-border shrink-0">
        <div className="flex items-center gap-2.5">
          <div className="p-2 rounded-xl bg-cyan-500/10 border border-cyan-500/30 text-cyan-400">
            <Database className="w-5 h-5" />
          </div>
          <div>
            <h1 className="text-lg font-bold text-white tracking-tight">Local KV Cache</h1>
            <p className="text-xs text-neutral-400">Embedded SQLite Key-Value storage, fast AST lookups & session caches</p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => showToast('Compacting SQLite database... 0 pages reclaimed')}
            className="px-3 py-1.5 bg-surface-elevated hover:bg-surface-hover border border-surface-border rounded-lg text-xs font-medium text-neutral-300 transition flex items-center gap-1.5"
          >
            <RefreshCw className="w-3.5 h-3.5" />
            <span>Vacuum</span>
          </button>
          <button
            onClick={handleFlushAll}
            className="px-3 py-1.5 bg-red-500/10 hover:bg-red-500/20 text-red-400 border border-red-500/30 rounded-lg text-xs font-medium transition flex items-center gap-1.5"
          >
            <Trash2 className="w-3.5 h-3.5" />
            <span>Flush Cache</span>
          </button>
        </div>
      </div>

      {/* Metrics Row */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 shrink-0">
        <div className="p-3 bg-surface-card border border-surface-border rounded-xl">
          <div className="text-[11px] text-neutral-400">Total Entries</div>
          <div className="text-lg font-mono font-bold text-white mt-0.5">{items.length} cached</div>
        </div>
        <div className="p-3 bg-surface-card border border-surface-border rounded-xl">
          <div className="text-[11px] text-neutral-400">Hit Ratio</div>
          <div className="text-lg font-mono font-bold text-emerald-400 mt-0.5">94.2%</div>
        </div>
        <div className="p-3 bg-surface-card border border-surface-border rounded-xl">
          <div className="text-[11px] text-neutral-400">Disk Footprint</div>
          <div className="text-lg font-mono font-bold text-cyan-400 mt-0.5">4.8 MB</div>
        </div>
        <div className="p-3 bg-surface-card border border-surface-border rounded-xl">
          <div className="text-[11px] text-neutral-400">Storage Backend</div>
          <div className="text-lg font-mono font-bold text-neutral-200 mt-0.5">SQLite WAL</div>
        </div>
      </div>

      {/* Search Bar */}
      <div className="bg-surface-card p-2 rounded-xl border border-surface-border flex items-center gap-2 shrink-0">
        <Search className="w-4 h-4 text-neutral-500 ml-2 shrink-0" />
        <input
          type="text"
          placeholder="Filter cache keys by name or namespace..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="w-full bg-transparent border-none text-neutral-200 placeholder-neutral-500 focus:outline-none text-xs"
        />
      </div>

      {/* Main Split: Table + Key Details */}
      <div className="flex-1 grid grid-cols-1 lg:grid-cols-3 gap-4 min-h-0 overflow-hidden">
        {/* Left Column: Keys Table */}
        <div className="lg:col-span-2 bg-surface-card border border-surface-border rounded-xl overflow-hidden flex flex-col">
          <div className="overflow-y-auto flex-1 divide-y divide-surface-border/40 text-xs">
            {filtered.length === 0 ? (
              <div className="p-8 text-center text-neutral-500">No cached items found.</div>
            ) : (
              filtered.map((item) => (
                <div
                  key={item.key}
                  onClick={() => setSelectedKey(item)}
                  className={`p-3 cursor-pointer flex items-center justify-between gap-3 transition ${
                    selectedKey?.key === item.key ? 'bg-surface-elevated border-l-2 border-brand-blue' : 'hover:bg-surface-elevated/50'
                  }`}
                >
                  <div className="flex items-center gap-2.5 min-w-0">
                    <span className="text-[10px] px-1.5 py-0.5 rounded font-mono uppercase bg-neutral-800 text-neutral-400 border border-neutral-700 shrink-0">
                      {item.category}
                    </span>
                    <span className="font-mono text-neutral-200 truncate">{item.key}</span>
                  </div>

                  <div className="flex items-center gap-4 shrink-0 text-[11px] text-neutral-400 font-mono">
                    <span>{item.size}</span>
                    <span className="text-emerald-400">TTL: {item.ttl}</span>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDeleteKey(item.key);
                      }}
                      className="text-neutral-500 hover:text-red-400 transition"
                      title="Purge key"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Right Column: Value Inspector */}
        <div className="bg-surface-card border border-surface-border rounded-xl p-4 flex flex-col overflow-hidden">
          <h3 className="text-xs font-semibold text-neutral-400 uppercase tracking-wider mb-2">Key Inspector</h3>
          {selectedKey ? (
            <div className="flex-1 flex flex-col overflow-hidden space-y-3">
              <div>
                <span className="text-[11px] text-neutral-500">Key Name:</span>
                <div className="text-xs font-mono font-medium text-white break-all">{selectedKey.key}</div>
              </div>
              <div className="flex items-center justify-between text-xs text-neutral-400 pt-2 border-t border-surface-border">
                <span>Size: <strong className="text-white font-mono">{selectedKey.size}</strong></span>
                <span>TTL: <strong className="text-emerald-400 font-mono">{selectedKey.ttl}</strong></span>
              </div>
              <div className="flex-1 flex flex-col min-h-0 pt-2 border-t border-surface-border">
                <span className="text-[11px] text-neutral-500 mb-1">Raw Stored Value:</span>
                <pre className="flex-1 bg-canvas/60 p-3 rounded-lg border border-surface-border text-[11px] font-mono text-neutral-300 overflow-auto whitespace-pre-wrap">
                  {selectedKey.valuePreview}
                </pre>
              </div>
            </div>
          ) : (
            <div className="flex-1 flex items-center justify-center text-xs text-neutral-500">
              Select a cache key to inspect its value
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default CachePage;
