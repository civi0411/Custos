CREATE TABLE IF NOT EXISTS quant_backtest_tearsheets (
    id TEXT PRIMARY KEY,
    strategy_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    annualized_sharpe REAL NOT NULL,
    deflated_sharpe_ratio REAL NOT NULL,
    max_drawdown REAL NOT NULL,
    calmar_ratio REAL NOT NULL,
    sortino_ratio REAL NOT NULL,
    var_95 REAL NOT NULL,
    lookahead_clean INTEGER NOT NULL DEFAULT 1,
    tearsheet_payload TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS quant_order_tickets (
    id TEXT PRIMARY KEY,
    strategy_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    direction TEXT NOT NULL,
    order_type TEXT NOT NULL,
    quantity REAL NOT NULL,
    limit_price REAL,
    target_weight REAL NOT NULL,
    estimated_slippage_bps REAL NOT NULL,
    status TEXT NOT NULL,
    permit_id TEXT,
    payload_digest TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_quant_tearsheets_strategy ON quant_backtest_tearsheets(strategy_id);
CREATE INDEX IF NOT EXISTS idx_quant_orders_strategy ON quant_order_tickets(strategy_id);
CREATE INDEX IF NOT EXISTS idx_quant_orders_status ON quant_order_tickets(status);

CREATE TABLE IF NOT EXISTS quant_market_bars (
    symbol TEXT NOT NULL,
    timeframe TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    open REAL NOT NULL,
    high REAL NOT NULL,
    low REAL NOT NULL,
    close REAL NOT NULL,
    volume REAL NOT NULL,
    PRIMARY KEY (symbol, timeframe, timestamp)
);
CREATE INDEX IF NOT EXISTS idx_quant_bars_lookup ON quant_market_bars(symbol, timeframe, timestamp);
