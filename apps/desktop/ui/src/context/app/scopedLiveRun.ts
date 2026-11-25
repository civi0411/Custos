import { useState, useEffect } from 'react';
import type { LiveRunSnapshot } from '@/types/domain';

type LiveRunListener = (snapshot: LiveRunSnapshot) => void;

class ScopedLiveRunRegistry {
  private snapshots = new Map<string, LiveRunSnapshot>();
  private listeners = new Map<string, Set<LiveRunListener>>();

  public emit(runId: string, snapshot: LiveRunSnapshot): void {
    this.snapshots.set(runId, snapshot);
    const runListeners = this.listeners.get(runId);
    if (runListeners) {
      runListeners.forEach((listener) => {
        try {
          listener(snapshot);
        } catch (err) {
          console.error('[ScopedLiveRun] listener error:', err);
        }
      });
    }
  }

  public get(runId: string): LiveRunSnapshot | undefined {
    return this.snapshots.get(runId);
  }

  public clear(runId: string): void {
    this.snapshots.delete(runId);
    this.listeners.delete(runId);
  }

  public subscribe(runId: string, listener: LiveRunListener): () => void {
    let set = this.listeners.get(runId);
    if (!set) {
      set = new Set();
      this.listeners.set(runId, set);
    }
    set.add(listener);

    // Initial notification if snapshot exists
    const current = this.snapshots.get(runId);
    if (current) {
      listener(current);
    }

    return () => {
      const existing = this.listeners.get(runId);
      if (existing) {
        existing.delete(listener);
        if (existing.size === 0) {
          this.listeners.delete(runId);
        }
      }
    };
  }
}

export const scopedLiveRunStore = new ScopedLiveRunRegistry();

export function useLiveRunSnapshot(runId: string | undefined): LiveRunSnapshot | undefined {
  const [snapshot, setSnapshot] = useState<LiveRunSnapshot | undefined>(() =>
    runId ? scopedLiveRunStore.get(runId) : undefined
  );

  useEffect(() => {
    if (!runId) {
      setSnapshot(undefined);
      return;
    }

    setSnapshot(scopedLiveRunStore.get(runId));
    const unsubscribe = scopedLiveRunStore.subscribe(runId, (next) => {
      setSnapshot({ ...next });
    });

    return unsubscribe;
  }, [runId]);

  return snapshot;
}
