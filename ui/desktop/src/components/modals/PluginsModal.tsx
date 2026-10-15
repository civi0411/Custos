import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Blocks,
  X,
  Search,
  Globe,
  Terminal,
  GitBranch,
  Cpu,
  Database,
  ExternalLink,
} from 'lucide-react';
import { useAppContext } from '@/context/AppContext';

export interface PluginsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

interface PluginItem {
  id: string;
  name: string;
  category: 'core' | 'tools' | 'mcp' | 'storage';
  description: string;
  version: string;
  icon: React.ComponentType<{ className?: string }>;
  enabled: boolean;
  author: string;
}

const INITIAL_PLUGINS: PluginItem[] = [
  {
    id: 'web-browser',
    name: 'Autonomous Web Browser',
    category: 'tools',
    description: 'Playwright headless browser engine for automated navigation, DOM extraction, and web research.',
    version: '1.4.2',
    icon: Globe,
    enabled: true,
    author: 'Custos Core',
  },
  {
    id: 'code-sandbox',
    name: 'Sandboxed Code Runner',
    category: 'core',
    description: 'Isolated Python & Bash execution environment with resource bounds and security policy enforcement.',
    version: '2.1.0',
    icon: Terminal,
    enabled: true,
    author: 'Custos Core',
  },
  {
    id: 'git-worktree',
    name: 'Git Worktree Engine',
    category: 'core',
    description: 'Automated worktree lifecycle manager allowing parallel feature branches without workspace collisions.',
    version: '1.0.8',
    icon: GitBranch,
    enabled: true,
    author: 'Custos Core',
  },
  {
    id: 'mcp-host',
    name: 'Model Context Protocol (MCP) Host',
    category: 'mcp',
    description: 'Connect standard Model Context Protocol servers to dynamically load external tools and sidecars.',
    version: '1.2.0',
    icon: Cpu,
    enabled: true,
    author: 'Anthropic / MCP Org',
  },
  {
    id: 'vector-memory',
    name: 'Local Vector Store & Long-Term Memory',
    category: 'storage',
    description: 'Embedded ChromaDB semantic memory for cross-session recall and code context indexing.',
    version: '0.9.5',
    icon: Database,
    enabled: true,
    author: 'Custos Core',
  },
];

export const PluginsModal: React.FC<PluginsModalProps> = ({ isOpen, onClose }) => {
  const { showToast } = useAppContext();
  const [plugins, setPlugins] = useState<PluginItem[]>(INITIAL_PLUGINS);
  const [searchQuery, setSearchQuery] = useState('');
  const [activeCategory, setActiveCategory] = useState<'all' | 'core' | 'tools' | 'mcp' | 'storage'>('all');

  const togglePlugin = (id: string) => {
    setPlugins((prev) =>
      prev.map((p) => {
        if (p.id === id) {
          const next = !p.enabled;
          showToast(`${p.name} ${next ? 'enabled' : 'disabled'}`);
          return { ...p, enabled: next };
        }
        return p;
      })
    );
  };

  const filtered = plugins.filter((p) => {
    const matchesSearch =
      p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.description.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesCat = activeCategory === 'all' || p.category === activeCategory;
    return matchesSearch && matchesCat;
  });

  return (
    <AnimatePresence>
      {isOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={onClose}
            className="absolute inset-0 bg-black/60 backdrop-blur-sm"
          />

          <motion.div
            initial={{ scale: 0.95, opacity: 0, y: 10 }}
            animate={{ scale: 1, opacity: 1, y: 0 }}
            exit={{ scale: 0.95, opacity: 0, y: 10 }}
            transition={{ type: 'spring', damping: 25, stiffness: 300 }}
            className="relative flex flex-col h-[640px] w-full max-w-2xl overflow-hidden rounded-2xl border border-border-default/60 bg-[#19191c] text-editor-fg shadow-2xl z-10"
          >
            {/* Modal Header */}
            <div className="flex items-center justify-between border-b border-border-default/40 px-6 py-4">
              <div className="flex items-center gap-3">
                <div className="flex size-9 items-center justify-center rounded-xl bg-workbench-accent/10 text-workbench-accent">
                  <Blocks className="size-5" />
                </div>
                <div>
                  <h2 className="text-base font-semibold text-white tracking-tight">Plugins & Extensions</h2>
                  <p className="text-xs text-fg-muted">Modular capabilities and external tool integrations</p>
                </div>
              </div>
              <button
                type="button"
                onClick={onClose}
                className="rounded-lg p-1.5 text-fg-muted hover:bg-white/5 hover:text-white transition"
              >
                <X className="size-4" />
              </button>
            </div>

            {/* Filter & Search Bar */}
            <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border-default/30 px-6 py-3 bg-[#161619]">
              <div className="flex items-center gap-1.5 overflow-x-auto text-xs">
                {(['all', 'core', 'tools', 'mcp', 'storage'] as const).map((cat) => (
                  <button
                    key={cat}
                    type="button"
                    onClick={() => setActiveCategory(cat)}
                    className={`rounded-lg px-2.5 py-1 font-medium capitalize transition ${
                      activeCategory === cat
                        ? 'bg-white/10 text-white shadow-sm'
                        : 'text-fg-muted hover:bg-white/5 hover:text-white'
                    }`}
                  >
                    {cat}
                  </button>
                ))}
              </div>

              <div className="relative flex-1 max-w-xs">
                <Search className="absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-fg-muted" />
                <input
                  type="text"
                  placeholder="Search plugins…"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="w-full rounded-lg border border-border-default/40 bg-surface-1 py-1.5 pl-8 pr-3 text-xs text-white placeholder-fg-muted/60 focus:border-workbench-accent focus:outline-none"
                />
              </div>
            </div>

            {/* Plugin Cards List */}
            <div className="flex-1 overflow-y-auto p-6 space-y-3">
              {filtered.map((plugin) => {
                const Icon = plugin.icon;
                return (
                  <div
                    key={plugin.id}
                    className="flex items-start justify-between gap-4 rounded-xl border border-border-default/40 bg-surface-1/60 p-4 transition hover:border-border-default hover:bg-surface-1"
                  >
                    <div className="flex items-start gap-3.5">
                      <div className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-white/5 text-workbench-accent border border-white/5 mt-0.5">
                        <Icon className="size-5" />
                      </div>
                      <div className="space-y-1">
                        <div className="flex items-center gap-2">
                          <span className="text-sm font-semibold text-white">{plugin.name}</span>
                          <span className="rounded bg-white/5 px-1.5 py-0.5 font-mono text-[10px] text-fg-muted border border-white/5">
                            v{plugin.version}
                          </span>
                        </div>
                        <p className="text-xs text-fg-muted leading-relaxed line-clamp-2">
                          {plugin.description}
                        </p>
                        <div className="flex items-center gap-2 pt-1 text-[11px] text-fg-muted/70">
                          <span>By {plugin.author}</span>
                          <span>•</span>
                          <span className="capitalize">{plugin.category}</span>
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center gap-2 shrink-0 pt-1">
                      <button
                        type="button"
                        onClick={() => togglePlugin(plugin.id)}
                        className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none ${
                          plugin.enabled ? 'bg-workbench-accent' : 'bg-surface-3'
                        }`}
                      >
                        <span
                          className={`pointer-events-none inline-block size-4 transform rounded-full bg-white shadow-lg ring-0 transition duration-200 ease-in-out ${
                            plugin.enabled ? 'translate-x-4' : 'translate-x-0'
                          }`}
                        />
                      </button>
                    </div>
                  </div>
                );
              })}

              {filtered.length === 0 && (
                <div className="flex flex-col items-center justify-center py-16 text-center text-fg-muted">
                  <Blocks className="size-8 stroke-1 text-fg-muted/40 mb-2" />
                  <p className="text-sm font-medium">No plugins found</p>
                  <p className="text-xs text-fg-muted/70 mt-0.5">Try searching for a different keyword</p>
                </div>
              )}
            </div>

            {/* Modal Footer */}
            <div className="flex items-center justify-between border-t border-border-default/40 px-6 py-3 bg-[#161619] text-xs text-fg-muted">
              <span>5 active modular extensions</span>
              <a
                href="https://modelcontextprotocol.io"
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-1 hover:text-white transition"
              >
                <span>Browse MCP Catalog</span>
                <ExternalLink className="size-3" />
              </a>
            </div>
          </motion.div>
        </div>
      )}
    </AnimatePresence>
  );
};
