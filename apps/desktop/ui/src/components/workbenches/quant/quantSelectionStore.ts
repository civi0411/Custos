import { useSyncExternalStore } from 'react';

export interface QuantSelectionState {
  strategyId: string;
  symbol: string;
  market: string;
  timeframe: string;
  formula: string;
  revision: number;
}

let state: QuantSelectionState = {
  strategyId: '',
  symbol: 'SPY',
  market: 'USStock',
  timeframe: '1d',
  formula: 'close / delay(close, 5) - 1.0',
  revision: 0,
};

const listeners = new Set<() => void>();

function emit() {
  for (const listener of listeners) {
    listener();
  }
}

export const quantSelectionStore = {
  getSnapshot(): QuantSelectionState {
    return state;
  },
  subscribe(listener: () => void): () => void {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
  update(patch: Partial<Omit<QuantSelectionState, 'revision'>>, bumpRevision = false) {
    state = {
      ...state,
      ...patch,
      revision: bumpRevision ? state.revision + 1 : state.revision,
    };
    emit();
  },
  notifyMutation() {
    state = { ...state, revision: state.revision + 1 };
    emit();
  },
};

export function useQuantSelection(): QuantSelectionState {
  return useSyncExternalStore(quantSelectionStore.subscribe, quantSelectionStore.getSnapshot);
}

