/**
 * Layers 1–3 of Custos Mobile:
 * - StableLogicalRpcClient: decouples UI state from PhysicalChannel swaps (Wi-Fi <-> 4G)
 * - CalmConnectionState: suppresses transient (< 2s) network drop banner flicker
 * - MicrotaskCandidateRacer: deterministic LAN-direct > Relay race via 1 microtask wait
 */

export interface PhysicalChannel {
  readonly kind: 'lan_direct' | 'e2ee_relay';
  sendFrame(payload: string): Promise<string>;
  close(): void;
}

export type CalmBannerState = 'connected' | 'reconnecting_silent' | 'degraded';

export class CalmConnectionState {
  private droppedAtMs: number | null = null;
  private readonly graceMs: number;

  constructor(graceMs = 2000) {
    this.graceMs = graceMs;
  }

  onConnected(): CalmBannerState {
    this.droppedAtMs = null;
    return 'connected';
  }

  onDisconnected(nowMs = Date.now()): CalmBannerState {
    if (this.droppedAtMs === null) {
      this.droppedAtMs = nowMs;
    }
    return nowMs - this.droppedAtMs < this.graceMs ? 'reconnecting_silent' : 'degraded';
  }
}

export class MicrotaskCandidateRacer {
  private lastWinnerByHost = new Map<string, 'lan_direct' | 'e2ee_relay'>();

  async race(
    hostId: string,
    lanDial: () => Promise<PhysicalChannel>,
    relayDial: () => Promise<PhysicalChannel>
  ): Promise<PhysicalChannel> {
    const lanPromise = lanDial().catch(() => null);
    const relayPromise = relayDial().catch(() => null);

    // Wait 1 microtask so LAN direct wins deterministically when on the same network
    await Promise.resolve();
    const lan = await Promise.race([lanPromise, Promise.resolve(null)]);
    if (lan) {
      this.lastWinnerByHost.set(hostId, 'lan_direct');
      return lan;
    }

    const first = await Promise.race([
      lanPromise.then((c) => c ?? relayPromise),
      relayPromise.then((c) => c ?? lanPromise),
    ]);
    if (!first) {
      throw new Error(`All physical channels failed for host ${hostId}`);
    }
    this.lastWinnerByHost.set(hostId, first.kind);
    return first;
  }

  preferredPath(hostId: string): 'lan_direct' | 'e2ee_relay' | undefined {
    return this.lastWinnerByHost.get(hostId);
  }
}

export class StableLogicalRpcClient {
  private channel: PhysicalChannel | null = null;
  private seq = 0;
  readonly calm = new CalmConnectionState();

  attachChannel(channel: PhysicalChannel): CalmBannerState {
    this.channel?.close();
    this.channel = channel;
    return this.calm.onConnected();
  }

  async request<T = unknown>(method: string, params: Record<string, unknown> = {}): Promise<T> {
    if (!this.channel) {
      throw new Error(`No active physical channel for RPC ${method}`);
    }
    const id = `mob-${++this.seq}`;
    const raw = await this.channel.sendFrame(JSON.stringify({ id, method, params }));
    const parsed = JSON.parse(raw) as { result?: T; error?: string };
    if (parsed.error) {
      throw new Error(parsed.error);
    }
    return parsed.result as T;
  }
}

