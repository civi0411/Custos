import React, { useState } from 'react';
import { Database, Search, Trash2, RefreshCw } from 'lucide-react';
import { useAppContext } from '../../context/AppContext';
import { Tooltip } from '../../components/Tooltip';

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
    <div className="flex-1 flex flex-col h-full bg-canvas overflow-hidden p-4 sm:p-6 space-y-4 text-fg-editor font-sans">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-3 border-b border-border-muted shrink-0">
        <div className="flex items-center gap-2.5">
          <div className="w-9 h-9 rounded-xl bg-surface-2 border border-border-default flex items-center justify-center workbench-accent">
            <Database className="w-5 h-5" />
          </div>
          <div>
            <h1 className="text-base sm:text-lg font-semibold tracking-[-0.02em] text-fg-editor">Local KV Cache</h1>
            <p className="text-xs text-fg-muted mt-0.5">Embedded SQLite Key-Value storage, fast AST lookups & session caches</p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => showToast('Compacting SQLite database... 0 pages reclaimed')}
            className="px-3 py-1.5 bg-surface-2 hover:bg-surface-3 border border-border-default rounded-lg text-xs font-medium text-fg-editor transition flex items-center gap-1.5"
          >
            <RefreshCw className="w-3.5 h-3.5 text-fg-muted" />
            <span>Vacuum</span>
          </button>
          <button
            onClick={handleFlushAll}
            className="px-3 py-1.5 bg-rose-500/10 hover:bg-rose-500/20 text-rose-500 border border-rose-500/30 rounded-lg text-xs font-medium transition flex items-center gap-1.5"
          >
            <Trash2 className="w-3.5 h-3.5" />
            <span>Flush Cache</span>
          </button>
        </div>
      </div>

      {/* Metrics Row */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 shrink-0">
        <div className="p-3 bg-surface-1 border border-border-muted rounded-xl shadow-xs">
          <div className="text-[11px] text-fg-subtle">Total Entries</div>
          <div className="text-base sm:text-lg font-mono font-semibold text-fg-editor mt-0.5">{items.length} cached</div>
        </div>
        <div className="p-3 bg-surface-1 border border-border-muted rounded-xl shadow-xs">
          <div className="text-[11px] text-fg-subtle">Hit Ratio</div>
          <div className="text-base sm:text-lg font-mono font-semibold text-emerald-500 mt-0.5">94.2%</div>
        </div>
        <div className="p-3 bg-surface-1 border border-border-muted rounded-xl shadow-xs">
          <div className="text-[11px] text-fg-subtle">Disk Footprint</div>
          <div className="text-base sm:text-lg font-mono font-semibold workbench-accent mt-0.5">4.8 MB</div>
        </div>
        <div className="p-3 bg-surface-1 border border-border-muted rounded-xl shadow-xs">
          <div className="text-[11px] text-fg-subtle">Storage Backend</div>
          <div className="text-base sm:text-lg font-mono font-semibold text-fg-editor mt-0.5">SQLite WAL</div>
        </div>
      </div>

      {/* Search Bar */}
      <div className="bg-surface-1 p-2 rounded-xl border border-border-default flex items-center gap-2 shrink-0">
        <Search className="w-4 h-4 text-fg-subtle ml-2 shrink-0" />
        <input
          type="text"
          placeholder="Filter cache keys by name or namespace..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="w-full bg-transparent border-none text-fg-editor placeholder-fg-subtle focus:outline-none text-xs"
        />
      </div>

      {/* Main Split: Table + Key Details */}
      <div className="flex-1 grid grid-cols-1 lg:grid-cols-3 gap-4 min-h-0 overflow-hidden">
        {/* Left Column: Keys Table */}
        <div className="lg:col-span-2 bg-surface-1 border border-border-default rounded-xl overflow-hidden flex flex-col shadow-xs">
          <div className="overflow-y-auto flex-1 divide-y divide-border-muted/50 text-xs">
            {filtered.length === 0 ? (
              <div className="p-8 text-center text-fg-subtle">No cached items found.</div>
            ) : (
              filtered.map((item) => (
                <div
                  key={item.key}
                  onClick={() => setSelectedKey(item)}
                  className={`p-3 cursor-pointer flex items-center justify-between gap-3 transition ${
                    selectedKey?.key === item.key ? 'bg-surface-2 border-l-2 border-[var(--workbench-accent)] text-fg-editor' : 'hover:bg-surface-2/50 text-fg-muted hover:text-fg-editor'
                  }`}
                >
                  <div className="flex items-center gap-2.5 min-w-0">
                    <span className="text-[10px] px-1.5 py-0.5 rounded font-mono uppercase bg-surface-3 text-fg-subtle border border-border-default shrink-0">
                      {item.category}
                    </span>
                    <span className="font-mono text-fg-editor truncate">{item.key}</span>
                  </div>

                  <div className="flex items-center gap-4 shrink-0 text-[11px] text-fg-muted font-mono">
                    <span>{item.size}</span>
                    <span className="text-emerald-500">TTL: {item.ttl}</span>
                    <Tooltip content="Purge key" position="left">
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDeleteKey(item.key);
                        }}
                        className="text-fg-subtle hover:text-rose-500 transition"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </Tooltip>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Right Column: Value Inspector */}
        <div className="bg-surface-1 border border-border-default rounded-xl p-4 flex flex-col overflow-hidden shadow-xs">
          <h3 className="text-xs font-semibold text-fg-subtle uppercase tracking-wider mb-2">Key Inspector</h3>
          {selectedKey ? (
            <div className="flex-1 flex flex-col overflow-hidden space-y-3">
              <div>
                <span className="text-[11px] text-fg-subtle">Key Name:</span>
                <div className="text-xs font-mono font-medium text-fg-editor break-all">{selectedKey.key}</div>
              </div>
              <div className="flex items-center justify-between text-xs text-fg-muted pt-2 border-t border-border-muted">
                <span>Size: <strong className="text-fg-editor font-mono">{selectedKey.size}</strong></span>
                <span>TTL: <strong className="text-emerald-500 font-mono">{selectedKey.ttl}</strong></span>
              </div>
              <div className="flex-1 flex flex-col min-h-0 pt-2 border-t border-border-muted">
                <span className="text-[11px] text-fg-subtle mb-1">Raw Stored Value:</span>
                <pre className="flex-1 bg-surface-0 p-3 rounded-lg border border-border-muted text-[11px] font-mono text-fg-editor overflow-auto whitespace-pre-wrap">
                  {selectedKey.valuePreview}
                </pre>
              </div>
            </div>
          ) : (
            <div className="flex-1 flex items-center justify-center text-xs text-fg-subtle">
              Select a cache key to inspect its value
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default CachePage;
