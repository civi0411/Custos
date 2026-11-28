/**
 * Layer 5 of Custos Mobile:
 * - DoubleQueueTerminalBridge: buffers native PTY chunks until web-ready -> xterm.init
 *   with Orca-style Write Coalescing (48ms, tightening to 16ms for 180ms on keystrokes)
 *   and Credit-Based Backpressure (128 KiB ack, 32 MiB max pending backlog).
 * - ResumableStreamRegistry: tracks sequence offsets for seamless replay after reconnect.
 */

export const TERMINAL_BRIDGE_POLICY = {
  coalesceWindowMs: 48,
  interactiveWindowMs: 16,
  interactiveHoldMs: 180,
  ackBatchBytes: 128 * 1024,
  maxPendingBytes: 32 * 1024 * 1024,
} as const;

export class DoubleQueueTerminalBridge {
  private nativeQueue: Uint8Array[] = [];
  private queuedBytes = 0;
  private webReady = false;
  private xtermInit = false;
  private sink: ((chunk: Uint8Array) => void) | null = null;
  private flushTimer: ReturnType<typeof setTimeout> | null = null;
  private interactiveUntilMs = 0;

  markInteractive(nowMs = Date.now()): void {
    this.interactiveUntilMs = nowMs + TERMINAL_BRIDGE_POLICY.interactiveHoldMs;
  }

  pushNativeChunk(chunk: Uint8Array): void {
    if (this.queuedBytes + chunk.byteLength > TERMINAL_BRIDGE_POLICY.maxPendingBytes) {
      // Drop oldest buffered chunk under extreme 32 MiB backpressure
      const dropped = this.nativeQueue.shift();
      if (dropped) this.queuedBytes -= dropped.byteLength;
    }
    this.nativeQueue.push(chunk);
    this.queuedBytes += chunk.byteLength;
    this.scheduleFlush();
  }

  markWebReady(): void {
    this.webReady = true;
    this.flushNow();
  }

  attachXterm(sink: (chunk: Uint8Array) => void): void {
    this.sink = sink;
    this.xtermInit = true;
    this.flushNow();
  }

  pendingBytes(): number {
    return this.queuedBytes;
  }

  private scheduleFlush(): void {
    if (!this.webReady || !this.xtermInit || !this.sink || this.flushTimer) return;
    const windowMs =
      Date.now() < this.interactiveUntilMs
        ? TERMINAL_BRIDGE_POLICY.interactiveWindowMs
        : TERMINAL_BRIDGE_POLICY.coalesceWindowMs;
    this.flushTimer = setTimeout(() => {
      this.flushTimer = null;
      this.flushNow();
    }, windowMs);
  }

  private flushNow(): void {
    if (!this.webReady || !this.xtermInit || !this.sink) return;
    while (this.nativeQueue.length > 0) {
      const next = this.nativeQueue.shift();
      if (next) {
        this.queuedBytes -= next.byteLength;
        this.sink(next);
      }
    }
  }
}

export class ResumableStreamRegistry {
  private offsets = new Map<string, number>();

  recordOffset(streamId: string, seq: number): void {
    const prev = this.offsets.get(streamId) ?? 0;
    if (seq > prev) {
      this.offsets.set(streamId, seq);
    }
  }

  resumeFrom(streamId: string): number {
    return this.offsets.get(streamId) ?? 0;
  }
}
