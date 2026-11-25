import { useCallback, useEffect, useRef, useState } from 'react';
import { daemonClient } from '@/api/daemon_client';
import type { TerminalSession } from '@/types/domain';

const pendingStarts = new Map<string, Promise<TerminalSession>>();
const memoryBindings = new Map<string, string>();

export function terminalBindingKey(tabId: string, workspaceId: string, command?: string): string {
  return `custos.terminal.binding:${JSON.stringify([tabId, workspaceId, command || null])}`;
}

export function terminalMatchesWorkspace(session: TerminalSession, workspaceId: string): boolean {
  return !!workspaceId && session.workspace_id === workspaceId;
}

function readBinding(key: string): string | null {
  try { return localStorage.getItem(key) || memoryBindings.get(key) || null; }
  catch { return memoryBindings.get(key) || null; }
}

function bind(key: string, id: string | null) {
  if (id) memoryBindings.set(key, id); else memoryBindings.delete(key);
  try { if (id) localStorage.setItem(key, id); else localStorage.removeItem(key); } catch { /* The daemon still owns this session when browser storage is unavailable. */ }
}

export function useTerminalSession(tabId: string, workspaceId: string, command?: string) {
  const key = terminalBindingKey(tabId, workspaceId, command);
  const currentKey = useRef(key);
  currentKey.current = key;
  const [state, setState] = useState<{ key: string; session: TerminalSession | null; busy: boolean; error: string | null }>({ key, session: null, busy: true, error: null });
  const publish = useCallback((next: Omit<typeof state, 'key'>) => {
    if (currentKey.current === key) setState({ ...next, key });
  }, [key]);
  const session = state.key === key ? state.session : null;
  const busy = state.key !== key || state.busy;
  const error = state.key === key ? state.error : null;

  const refresh = useCallback(async () => {
    if (!workspaceId) { publish({ session: null, busy: false, error: null }); return; }
    const id = readBinding(key);
    if (!id) { publish({ session: null, busy: false, error: null }); return; }
    if (currentKey.current === key) setState(previous => ({ key, session: previous.key === key ? previous.session : null, busy: true, error: null }));
    try {
      const found = await daemonClient.getTerminal(id);
      if (!terminalMatchesWorkspace(found, workspaceId)) throw new Error('The terminal belongs to a different workspace.');
      publish({ session: found, busy: false, error: null });
    } catch (reason) {
      const message = reason instanceof Error ? reason.message : String(reason);
      if (/not.?found|unknown terminal|no longer exists/i.test(message)) {
        bind(key, null);
        publish({ session: null, busy: false, error: null });
      } else publish({ session: null, busy: false, error: `Could not reconnect to this terminal: ${message}` });
    }
  }, [key, workspaceId, publish]);
  useEffect(() => { currentKey.current = key; void refresh(); return () => { if (currentKey.current === key) currentKey.current = ''; }; }, [refresh, key]);

  const start = useCallback(async () => {
    if (!workspaceId || (session?.status.type === 'active')) return;
    publish({ session, busy: true, error: null });
    try {
      let attempt = pendingStarts.get(key);
      if (!attempt) {
        const commandKey = `${key}:launch`;
        const commandId = readBinding(commandKey) || crypto.randomUUID();
        bind(commandKey, commandId);
        attempt = daemonClient.spawnTerminal({ workspace_id: workspaceId, command, cols: 100, rows: 30, command_id: commandId });
        pendingStarts.set(key, attempt);
      }
      const created = await attempt;
      bind(key, created.id);
      bind(`${key}:launch`, null);
      publish({ session: created, busy: false, error: null });
    } catch (reason) {
      publish({ session, busy: false, error: `Terminal start could not be confirmed: ${reason instanceof Error ? reason.message : String(reason)}. Retry reconnect before starting again.` });
    } finally { pendingStarts.delete(key); }
  }, [key, workspaceId, command, session, error, publish]);

  const attach = useCallback(async (id: string) => {
    publish({ session, busy: true, error: null });
    try {
      const found = await daemonClient.getTerminal(id);
      if (!terminalMatchesWorkspace(found, workspaceId)) throw new Error('The terminal belongs to a different workspace.');
      bind(key, id);
      publish({ session: found, busy: false, error: null });
    } catch (reason) { publish({ session, busy: false, error: String(reason) }); }
  }, [key, workspaceId, session, publish]);

  const stop = useCallback(async () => {
    if (!session) return;
    publish({ session, busy: true, error: null });
    try {
      await daemonClient.terminateTerminal(session.id);
      publish({ session: { ...session, status: { type: 'terminated' } }, busy: false, error: null });
    } catch (reason) { publish({ session, busy: false, error: `Could not confirm process termination: ${String(reason)}` }); }
  }, [session, publish]);

  return { session, busy, error, start, stop, refresh, attach };
}
