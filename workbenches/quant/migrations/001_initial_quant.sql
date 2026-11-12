-- Initial scoped schema for Quant Workbench
-- Rule: Workbench owns its domain tables prefixed with mod_<name>_*

CREATE TABLE IF NOT EXISTS mod_quant_tearsheets (
    id TEXT PRIMARY KEY,
    strategy_id TEXT NOT NULL,
    task_id TEXT,
    sharpe_ratio REAL,
    max_drawdown REAL,
    annual_return REAL,
    metrics_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE IF NOT EXISTS mod_quant_orders (
    id TEXT PRIMARY KEY,
    strategy_id TEXT,
    task_id TEXT,
    symbol TEXT NOT NULL,
    side TEXT NOT NULL,
    order_type TEXT NOT NULL,
    quantity REAL NOT NULL,
    price REAL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE IF NOT EXISTS mod_quant_market_bars (
    symbol TEXT NOT NULL,
    timeframe TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    open REAL NOT NULL,
    high REAL NOT NULL,
    low REAL NOT NULL,
    close REAL NOT NULL,
    volume REAL NOT NULL,
    source TEXT NOT NULL,
    PRIMARY KEY(symbol, timeframe, timestamp)
);
