export interface DaemonTransport {
  request<T>(method: string, params?: Record<string, any>): Promise<T>;
}

export class QuantClient {
  constructor(private transport: DaemonTransport) {}

  async submitQuantBacktest(params: {
    strategy_id: string;
    task_id: string;
    formula?: string;
    annualized_sharpe: number;
    max_drawdown: number;
    total_return: number;
    trials_count?: number;
    sample_length?: number;
    volatility?: number;
    win_rate?: number;
    profit_factor?: number;
    turnover?: number;
  }): Promise<{
    id: string;
    strategy_id: string;
    annualized_sharpe: number;
    deflated_sharpe: number;
    max_drawdown: number;
    total_return: number;
    calmar_ratio: number;
    annualized_volatility: number;
    win_rate: number;
    profit_factor: number;
    turnover: number;
  }> {
    return this.transport.request('v1.quant.backtest.submit', params);
  }

  async getQuantTearsheet(strategy_id: string): Promise<{
    id: string;
    strategy_id: string;
    annualized_sharpe: number;
    deflated_sharpe: number;
    max_drawdown: number;
    total_return: number;
    calmar_ratio: number;
    annualized_volatility: number;
    win_rate: number;
    profit_factor: number;
    turnover: number;
  }> {
    return this.transport.request('v1.quant.tearsheet.get', { strategy_id });
  }

  async createQuantOrders(params: {
    strategy_id: string;
    task_id: string;
    orders: Array<{
      symbol: string;
      direction: 'buy' | 'sell' | 'sell_short' | 'buy_to_cover';
      quantity: number;
      target_weight: number;
      limit_price?: number;
      estimated_slippage_bps?: number;
    }>;
  }): Promise<{
    created_orders: Array<{
      id: string;
      strategy_id: string;
      symbol: string;
      direction: string;
      quantity: number;
      target_weight: number;
      status: string;
    }>;
  }> {
    return this.transport.request('v1.quant.orders.create', params);
  }

  async listQuantOrders(params?: {
    strategy_id?: string;
    task_id?: string;
  }): Promise<{
    orders: Array<{
      id: string;
      strategy_id: string;
      symbol: string;
      direction: string;
      quantity: number;
      target_weight: number;
      status: string;
      permit_id?: string;
    }>;
  }> {
    return this.transport.request('v1.quant.orders.list', params || {});
  }

  async approveQuantOrder(params: {
    order_id: string;
    permit_id: string;
    task_id?: string;
  }): Promise<{
    status: string;
    permit_id: string;
    outbox_action_id?: string;
  }> {
    return this.transport.request('v1.quant.orders.approve', params);
  }

  async discoverQuantFactors(params?: {
    symbol?: string;
    timeframe?: string;
  }): Promise<{
    symbol: string;
    timeframe: string;
    candidates: Array<{
      id: string;
      name: string;
      hypothesis: string;
      formula: string;
      ic_mean: number;
      ic_std: number;
      ir: number;
      q_value: number;
      classification: string;
      lookahead_clean: boolean;
      status: string;
    }>;
    total_evaluated: number;
    passed_fdr: number;
  }> {
    return this.transport.request('v1.quant.factors.discover', params || {});
  }

  async triggerQuantKillSwitch(params?: {
    reason?: string;
  }): Promise<{
    status: string;
    reason: string;
    cancelled_orders_count: number;
    timestamp: string;
  }> {
    return this.transport.request('v1.quant.kill_switch', params || {});
  }

  async evaluateQuantGrid(params: {
    symbol: string;
    lower_price: number;
    upper_price: number;
    grid_count: number;
    mode?: 'arithmetic' | 'geometric';
    total_capital: number;
    current_price: number;
    stop_loss_price?: number;
    take_profit_price?: number;
  }): Promise<{
    symbol: string;
    levels: number[];
    cells_count: number;
    actions: Array<{
      action: string;
      payload: any;
    }>;
    status: {
      symbol: string;
      active_cells: number;
      filled_cells: number;
      total_grid_cycles: number;
      total_realized_pnl: number;
      current_price: number;
      in_boundary: boolean;
    };
  }> {
    return this.transport.request('v1.quant.grid.evaluate', params);
  }

  async getQuantMarketBars(params: {
    symbol: string;
    timeframe?: string;
    limit?: number;
  }): Promise<{
    symbol: string;
    timeframe: string;
    bars: Array<{
      timestamp: number;
      open: number;
      high: number;
      low: number;
      close: number;
      volume: number;
    }>;
    count: number;
  }> {
    return this.transport.request('v1.quant.market_bars.get', params);
  }

  async ingestQuantMarketBars(params: {
    symbol: string;
    timeframe: string;
    bars: Array<{
      timestamp: number;
      open: number;
      high: number;
      low: number;
      close: number;
      volume: number;
    }>;
  }): Promise<{
    symbol: string;
    timeframe: string;
    stored_bars: number;
  }> {
    return this.transport.request('v1.quant.market_bars.ingest', params);
  }

  async getPipelineStatus(): Promise<{
    status: string;
    timestamp_ms: number;
    stages: {
      bronze: { status: string; invariant_checks: string };
      silver: { status: string; invariant_checks: string };
      gold: { status: string; invariant_checks: string };
    };
    modalities: {
      code: { status: string; ast_validation: string };
      science: { status: string; evidence_grounding: string };
      agent: { status: string; outbox_governance: string };
      market: { status: string; lookahead_prevention: string };
    };
  }> {
    return this.transport.request('v1.pipeline.status', {});
  }

  async runPipeline(params: {
    content: string;
    modality?: 'Market' | 'Science' | 'Code' | 'Agent';
    salt?: string;
  }): Promise<{
    run_id: string;
    status: string;
    stage_reached: string;
    modality: string;
    content_hash: string;
    rows_ingested: number;
    headers: string[];
    anonymized_columns: string[];
    anonymization_salt_applied: boolean;
    invariants_verified: string[];
    quality_score: number;
    completed_at_ms: number;
  }> {
    return this.transport.request('v1.pipeline.run', params);
  }

  async computeQuantIndicators(params: {
    symbol: string;
    timeframe?: string;
    limit?: number;
  }): Promise<{
    symbol: string;
    timeframe: string;
    reference_timestamp: number;
    total_bars_evaluated: number;
    indicators: {
      sma_20?: number;
      sma_50?: number;
      ema_20?: number;
      rsi_14?: number;
      bollinger_bands_20_2?: {
        middle: number;
        upper: number;
        lower: number;
        bandwidth: number;
      };
      macd_12_26_9?: {
        macd: number;
        signal: number;
        histogram: number;
      };
      atr_14?: number;
      vwap?: number;
      [key: string]: any;
    };
  }> {
    return this.transport.request('v1.quant.indicators.compute', params);
  }

  async detectPipelineStructure(content: string): Promise<{
    delimiter: string;
    header_row_index: number;
    headers: string[];
    columns: Array<{
      name: string;
      data_type: 'Integer' | 'Float' | 'Datetime' | 'Boolean' | 'Text';
      null_ratio: number;
    }>;
    total_rows: number;
    quality_score: number;
  }> {
    return this.transport.request('v1.pipeline.structure.detect', { content });
  }

  async evaluateQuantGate(params: {
    symbol: string;
    direction: 'buy' | 'sell' | 'sell_short' | 'buy_to_cover';
    quantity: number;
    estimated_price: number;
    current_drawdown?: number;
    consecutive_losses?: number;
    daily_executed_notional?: number;
    is_risk_reducing?: boolean;
    max_order_notional?: number;
    max_daily_notional?: number;
    max_drawdown_limit?: number;
    max_consecutive_losses?: number;
  }): Promise<{
    is_allowed: boolean;
    outcome: {
      status: 'Pass' | 'Reject' | 'BypassExitImmunity' | 'FailOpen';
      details: {
        reason: string;
        order_notional?: number;
        breach_type?: string;
      };
    };
    order_notional: number;
    symbol: string;
  }> {
    return this.transport.request('v1.quant.gate.evaluate', params);
  }

  async computeQuantReview(params: {
    strategy_id?: string;
    symbol?: string;
    fills: Array<{
      fill_id: string;
      order_id: string;
      symbol: string;
      direction: 'buy' | 'sell' | 'sell_short' | 'buy_to_cover';
      price: number;
      quantity: number;
      commission: number;
      timestamp: number;
      exit_reason?: string | null;
    }>;
  }): Promise<{
    strategy_id: string;
    symbol: string;
    total_trades: number;
    gross_pnl: number;
    total_commission: number;
    net_pnl: number;
    profit_factor: number;
    win_loss_ratio: number;
    holding_cohorts: Array<{
      label: string;
      trade_count: number;
      win_count: number;
      win_rate: number;
      gross_pnl: number;
      net_pnl: number;
      total_commission: number;
    }>;
    exit_cohorts: Array<{
      reason: string;
      count: number;
      net_pnl: number;
    }>;
    diagnostics: Array<{
      code: string;
      severity: string;
      summary: string;
      recommendation: string;
    }>;
  }> {
    return this.transport.request('v1.quant.review.compute', params);
  }

  async getQuantOrderBook(params: {
    symbol?: string;
    depth?: number;
  }): Promise<{
    symbol: string;
    mid_price: number;
    spread: number;
    spread_bps: number;
    microprice: number;
    imbalance_pct: number;
    asks: Array<{ price: number; size: number; total: number }>;
    bids: Array<{ price: number; size: number; total: number }>;
    timestamp: number;
  }> {
    return this.transport.request('v1.quant.orderbook.get', params);
  }

  async computeQuantPbo(params: {
    score_matrix: number[][];
    max_combinations?: number;
  }): Promise<{
    available: boolean;
    probability: number;
    samples: number;
    median_logit: number;
    logits: number[];
    is_overfitted: boolean;
    status: string;
  }> {
    return this.transport.request('v1.quant.pbo.compute', params);
  }
}
