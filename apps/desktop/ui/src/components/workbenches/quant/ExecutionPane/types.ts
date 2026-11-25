import type { OrcaTabId } from '../../workbenchResources';

export interface ExecutionPaneProps {
  sessionId?: string;
  onShowToast?: (message: string) => void;
  onOpenTab?: (id: OrcaTabId) => void;
}

export interface OrderTicketItem {
  id: string;
  strategy_id: string;
  symbol: string;
  direction: string;
  quantity: number;
  target_weight: number;
  status: string;
  permit_id?: string;
}

export interface PositionItem {
  symbol: string;
  direction: 'long' | 'short';
  quantity: number;
  entryPrice: number;
  markPrice: number;
  unrealizedPnl: number;
}

export interface MarketBar {
  timestamp: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

export interface OrderBookLevel {
  price: number;
  size: number;
  depth: number;
}
