import React, { useCallback, useEffect, useState } from 'react';
import { Plus, RefreshCw, Trash2, PlugZap } from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';

interface McpServerRecord {
  id: string;
  prefix: string;
  display_name: string;
  transport_kind: 'stdio' | 'http';
  command: string | null;
  args: string[];
  url: string | null;
  auth_secret_ref: string | null;
  enabled: boolean;
}

interface McpToolItem {
  name: string;
  description: string;
}

export const McpServersPane: React.FC<{ onShowToast: (msg: string) => void }> = ({
  onShowToast,
}) => {
  const [servers, setServers] = useState<McpServerRecord[]>([]);
  const [activePrefixes, setActivePrefixes] = useState<string[]>([]);
  const [tools, setTools] = useState<McpToolItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [showForm, setShowForm] = useState(false);

  const [prefix, setPrefix] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [transportKind, setTransportKind] = useState<'stdio' | 'http'>('stdio');
  const [command, setCommand] = useState('');
  const [argsText, setArgsText] = useState('');
  const [url, setUrl] = useState('');
  const [secretRef, setSecretRef] = useState('');

  const loadAll = useCallback(async () => {
    setLoading(true);
    try {
      const res = await daemonClient.request<{
        servers: McpServerRecord[];
        active_prefixes: string[];
      }>('v1.mcp.servers.list', {});
      setServers(res.servers ?? []);
      setActivePrefixes(res.active_prefixes ?? []);
    } catch (err) {
      onShowToast(err instanceof Error ? err.message : 'Failed to load MCP servers');
    } finally {
      setLoading(false);
    }
  }, [onShowToast]);

  useEffect(() => {
    void loadAll();
  }, [loadAll]);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      const parsedArgs = argsText
        .split(/\s+/)
        .map((s) => s.trim())
        .filter(Boolean);
      await daemonClient.request('v1.mcp.servers.save', {
        prefix: prefix.trim(),
        display_name: (displayName || prefix).trim(),
        transport_kind: transportKind,
        command: transportKind === 'stdio' ? command.trim() : null,
        args: transportKind === 'stdio' ? parsedArgs : [],
        url: transportKind === 'http' ? url.trim() : null,
        auth_secret_ref: secretRef.trim() || null,
        enabled: true,
      });
      setPrefix('');
      setDisplayName('');
      setCommand('');
      setArgsText('');
      setUrl('');
      setSecretRef('');
      setShowForm(false);
      onShowToast('MCP server saved and registered');
      await loadAll();
    } catch (err) {
      onShowToast(err instanceof Error ? err.message : 'Failed to save MCP server');
    }
  };

  const handleToggle = async (srv: McpServerRecord) => {
    try {
      await daemonClient.request('v1.mcp.servers.toggle', {
        prefix: srv.prefix,
        enabled: !srv.enabled,
      });
      await loadAll();
    } catch (err) {
      onShowToast(err instanceof Error ? err.message : 'Failed to toggle MCP server');
    }
  };

  const handleDelete = async (targetPrefix: string) => {
    try {
      await daemonClient.request('v1.mcp.servers.delete', { prefix: targetPrefix });
      onShowToast(`Removed MCP server "${targetPrefix}"`);
      await loadAll();
    } catch (err) {
      onShowToast(err instanceof Error ? err.message : 'Failed to delete MCP server');
    }
  };

  const handleProbeTools = async () => {
    setLoading(true);
    try {
      const res = await daemonClient.request<{ tools: McpToolItem[] }>('v1.mcp.tools.list', {});
      setTools(res.tools ?? []);
      onShowToast(`Discovered ${(res.tools ?? []).length} federated MCP tools`);
    } catch (err) {
      onShowToast(err instanceof Error ? err.message : 'Failed to probe MCP tools');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-3 rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4">
      <div className="flex items-center justify-between gap-3">
        <div>
          <div className="text-xs font-semibold text-[var(--color-editor-fg)]">MCP Servers</div>
          <div className="mt-0.5 text-[11px] text-[var(--color-fg-muted)]">
            Persisted stdio and HTTP MCP servers federated into the Governed Agent Runtime.
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => void handleProbeTools()}
            disabled={loading}
            className="flex items-center gap-1.5 rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1.5 text-[11px] font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] disabled:opacity-50"
          >
            <RefreshCw className={`size-3 ${loading ? 'animate-spin' : ''}`} />
            <span>Probe Tools</span>
          </button>
          <button
            type="button"
            onClick={() => setShowForm((v) => !v)}
            className="flex items-center gap-1.5 rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1.5 text-[11px] font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)]"
          >
            <Plus className="size-3" />
            <span>Add Server</span>
          </button>
        </div>
      </div>

      {showForm && (
        <form onSubmit={(e) => void handleSave(e)} className="grid grid-cols-2 gap-2.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] p-3">
          <input
            value={prefix}
            onChange={(e) => setPrefix(e.target.value)}
            placeholder="Prefix (e.g. github)"
            required
            className="rounded border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)]"
          />
          <input
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            placeholder="Display name"
            className="rounded border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)]"
          />
          <select
            value={transportKind}
            onChange={(e) => setTransportKind(e.target.value as 'stdio' | 'http')}
            className="rounded border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)]"
          >
            <option value="stdio">stdio (local command)</option>
            <option value="http">http (remote endpoint)</option>
          </select>
          <input
            value={secretRef}
            onChange={(e) => setSecretRef(e.target.value)}
            placeholder="Env secret ref (optional, e.g. GITHUB_TOKEN)"
            className="rounded border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)]"
          />
          {transportKind === 'stdio' ? (
            <>
              <input
                value={command}
                onChange={(e) => setCommand(e.target.value)}
                placeholder="Command (e.g. npx)"
                required
                className="rounded border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)]"
              />
              <input
                value={argsText}
                onChange={(e) => setArgsText(e.target.value)}
                placeholder="Args (space-separated)"
                className="rounded border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)]"
              />
            </>
          ) : (
            <input
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="https://..."
              required
              className="col-span-2 rounded border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)]"
            />
          )}
          <div className="col-span-2 flex justify-end gap-2">
            <button
              type="button"
              onClick={() => setShowForm(false)}
              className="rounded border border-[var(--color-border-default)] px-3 py-1 text-[11px] text-[var(--color-fg-muted)]"
            >
              Cancel
            </button>
            <button
              type="submit"
              className="rounded bg-[var(--color-accent)] px-3 py-1 text-[11px] font-medium text-white"
            >
              Save & Register
            </button>
          </div>
        </form>
      )}

      {servers.length === 0 ? (
        <div className="rounded-lg border border-dashed border-[var(--color-border-default)] p-3 text-center text-[11px] text-[var(--color-fg-muted)]">
          No MCP servers configured yet.
        </div>
      ) : (
        <div className="space-y-2">
          {servers.map((srv) => {
            const isRegistered = activePrefixes.includes(srv.prefix);
            return (
              <div
                key={srv.id}
                className="flex items-center justify-between gap-3 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3 py-2"
              >
                <div className="min-w-0">
                  <div className="flex items-center gap-2 text-xs font-medium text-[var(--color-editor-fg)]">
                    <PlugZap className="size-3.5 shrink-0" />
                    <span className="truncate">{srv.display_name}</span>
                    <span className="rounded bg-[var(--color-surface-3)] px-1.5 py-0.5 font-mono text-[10px] text-[var(--color-fg-muted)]">
                      {srv.prefix}
                    </span>
                    <span className={`text-[10px] ${isRegistered && srv.enabled ? 'text-emerald-400' : 'text-[var(--color-fg-muted)]'}`}>
                      {srv.enabled ? (isRegistered ? 'Registered' : 'Pending') : 'Disabled'}
                    </span>
                  </div>
                  <div className="mt-0.5 truncate font-mono text-[10px] text-[var(--color-fg-muted)]">
                    {srv.transport_kind === 'stdio'
                      ? `${srv.command ?? ''} ${srv.args.join(' ')}`
                      : srv.url ?? ''}
                  </div>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  <button
                    type="button"
                    onClick={() => void handleToggle(srv)}
                    className="rounded border border-[var(--color-border-default)] px-2 py-1 text-[10px] text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)]"
                  >
                    {srv.enabled ? 'Disable' : 'Enable'}
                  </button>
                  <button
                    type="button"
                    onClick={() => void handleDelete(srv.prefix)}
                    aria-label={`Delete ${srv.prefix}`}
                    className="rounded border border-[var(--color-border-default)] p-1 text-[var(--color-fg-muted)] hover:text-rose-400"
                  >
                    <Trash2 className="size-3.5" />
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {tools.length > 0 && (
        <div className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] p-2.5">
          <div className="mb-1.5 text-[10px] font-semibold uppercase tracking-wide text-[var(--color-fg-muted)]">
            Federated Tools ({tools.length})
          </div>
          <div className="max-h-32 space-y-1 overflow-y-auto">
            {tools.map((t) => (
              <div key={t.name} className="font-mono text-[11px] text-[var(--color-editor-fg)]">
                {t.name}
                {t.description ? <span className="ml-2 font-sans text-[10px] text-[var(--color-fg-muted)]">{t.description}</span> : null}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

