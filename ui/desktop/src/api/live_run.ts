import type { LiveRunSnapshot } from '../types/domain';
import type { DaemonTransport } from './workbenches/quant_client';

const TERMINAL_RUN_STATUSES = ['completed', 'failed', 'cancelled', 'interrupted'];

export async function streamLiveRun(
  transport: DaemonTransport,
  resolveHttpEndpoint: () => Promise<string>,
  session: string,
  run: string,
  update: (snapshot: LiveRunSnapshot) => void,
  signal: AbortSignal
): Promise<void> {
  let snapshot: LiveRunSnapshot | null = null;
  let backoff = 1000;

  while (!signal.aborted) {
    try {
      const base = await resolveHttpEndpoint();
      const replay: { snapshot: LiveRunSnapshot } = await transport.request<{ snapshot: LiveRunSnapshot }>(
        'v1.workflow.live_run',
        { session_id: session, run_id: run, cursor: snapshot?.cursor || 0 }
      );
      snapshot = { ...replay.snapshot, reconnecting: false };
      update(snapshot);
      if (TERMINAL_RUN_STATUSES.includes(snapshot.status)) return;

      const response = await fetch(
        `${base}/api/events?session_id=${encodeURIComponent(session)}&run_id=${encodeURIComponent(run)}&cursor=${snapshot.cursor}`,
        { signal, headers: { Accept: 'text/event-stream' } }
      );
      if (!response.ok || !response.body) throw new Error(`Live connection HTTP ${response.status}`);

      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let pending = '';
      const consume = (frame: string) => {
        let event = '';
        const data: string[] = [];
        for (const line of frame.split('\n')) {
          if (line.startsWith('event:')) event = line.slice(6).trim();
          if (line.startsWith('data:')) data.push(line.slice(5).trimStart());
        }
        if (!data.length || !snapshot) return;
        const value = JSON.parse(data.join('\n'));
        if (event === 'snapshot') {
          if (value.run_id !== run || value.session_id !== session) throw new Error('Mismatched live snapshot');
          snapshot = { ...value, reconnecting: false };
        } else if (event === 'run') {
          if (value.run_id !== run || value.session_id !== session || value.id <= snapshot.cursor) return;
          if (value.id !== snapshot.cursor + 1) throw new Error('Live cursor gap');
          const payload = value.payload || {};
          snapshot = { ...snapshot, cursor: value.id, reconnecting: false };
          if (value.kind === 'text_delta') snapshot.text += payload.text || '';
          if (value.kind === 'status') snapshot.status = payload.status;
          if (['tool_input', 'tool_output', 'reasoning_summary'].includes(value.kind)) {
            snapshot.activities = [...snapshot.activities, { kind: value.kind, payload }].slice(-100);
          }
          if (value.kind === 'usage') snapshot.usage = payload;
          if (value.kind === 'result') {
            snapshot.status = payload.status;
            snapshot.text = payload.text || snapshot.text;
            snapshot.error = payload.error;
          }
        }
        if (snapshot) update(snapshot);
      };

      try {
        while (!signal.aborted) {
          const { value, done } = await reader.read();
          if (done) break;
          pending += decoder.decode(value, { stream: true }).replace(/\r\n/g, '\n');
          if (pending.length > 3 * 1024 * 1024) throw new Error('Live frame exceeded limit');
          let end: number;
          while ((end = pending.indexOf('\n\n')) >= 0) {
            consume(pending.slice(0, end));
            pending = pending.slice(end + 2);
          }
          if (snapshot && TERMINAL_RUN_STATUSES.includes(snapshot.status)) return;
          backoff = 1000;
        }
      } finally {
        await reader.cancel().catch(() => undefined);
        reader.releaseLock();
      }
      if (!signal.aborted) throw new Error('Live stream disconnected');
    } catch {
      if (signal.aborted) return;
      if (snapshot) update({ ...snapshot, reconnecting: true });
      await new Promise<void>((resolve) => {
        const timer = window.setTimeout(() => {
          signal.removeEventListener('abort', abort);
          resolve();
        }, backoff);
        const abort = () => {
          window.clearTimeout(timer);
          resolve();
        };
        signal.addEventListener('abort', abort, { once: true });
      });
      backoff = Math.min(5000, backoff * 2);
    }
  }
}

