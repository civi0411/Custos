export interface BrowserState {
  id: string;
  instance: string | null;
  url: string;
  loading: boolean;
  error: string | null;
}

// One queue per tab includes close/create, including StrictMode remounts.
const queues = new Map<string, Promise<unknown>>();
export function browserCommand(id: string, action: string, fields: Record<string, unknown> = {}): Promise<string | null> {
  const previous = queues.get(id) ?? Promise.resolve();
  const next = previous.catch(() => {}).then(async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<string | null>('browser_command', { request: { id, action, ...fields } });
  });
  queues.set(id, next);
  void next.finally(() => { if (queues.get(id) === next) queues.delete(id); }).catch(() => {});
  return next;
}

export function normalizeBrowserUrl(raw: string): string {
  const value = raw.trim();
  if (!value || value === 'about:blank') return 'about:blank';
  if (/^:\d+(\/.*)?$/.test(value)) return `http://localhost${value}`;
  if (/^(localhost|127\.0\.0\.1|0\.0\.0\.0)(:\d+)?(\/.*)?$/i.test(value)) return `http://${value}`;
  if (/^[a-z][a-z\d+.-]*:/i.test(value)) return value;
  if (/^[^\s]+\.[a-z]{2,}(:\d+)?(\/.*)?$/i.test(value)) return `https://${value}`;
  return `https://duckduckgo.com/?q=${encodeURIComponent(value)}`;
}
