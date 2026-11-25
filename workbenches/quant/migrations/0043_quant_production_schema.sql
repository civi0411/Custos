-- On-Demand Quant Workbench Production Schema (Wave 1)
-- Loaded strictly when the 'quant' workbench package is installed and activated.

CREATE TABLE IF NOT EXISTS quant_strategies (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    market TEXT NOT NULL DEFAULT 'USStock',
    venue TEXT NOT NULL DEFAULT 'local',
    symbol TEXT NOT NULL,
    timeframe TEXT NOT NULL DEFAULT '1d',
    formula TEXT NOT NULL,
    direction_mode TEXT NOT NULL DEFAULT 'long_only',
    status TEXT NOT NULL DEFAULT 'backtest',
    latest_sharpe REAL,
    latest_ic_mean REAL,
    source_hash TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_quant_strategies_symbol ON quant_strategies(symbol);
CREATE INDEX IF NOT EXISTS idx_quant_strategies_status ON quant_strategies(status);

CREATE TABLE IF NOT EXISTS quant_positions (
    id TEXT PRIMARY KEY,
    strategy_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    market TEXT NOT NULL DEFAULT 'USStock',
    direction TEXT NOT NULL,
    quantity REAL NOT NULL,
    avg_entry_price REAL NOT NULL,
    realized_pnl REAL NOT NULL DEFAULT 0.0,
    updated_at INTEGER NOT NULL,
    UNIQUE(strategy_id, symbol, direction)
);

CREATE INDEX IF NOT EXISTS idx_quant_positions_strategy ON quant_positions(strategy_id);

CREATE TABLE IF NOT EXISTS quant_fills (
    fill_id TEXT PRIMARY KEY,
    order_id TEXT NOT NULL,
    strategy_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    direction TEXT NOT NULL,
    price REAL NOT NULL,
    quantity REAL NOT NULL,
    commission REAL NOT NULL,
    close_reason TEXT,
    execution_mode TEXT NOT NULL DEFAULT 'paper',
    timestamp INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_quant_fills_strategy ON quant_fills(strategy_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_quant_fills_symbol ON quant_fills(symbol, timestamp);

CREATE TABLE IF NOT EXISTS quant_evidence_snapshots (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    market TEXT NOT NULL,
    quality_score REAL NOT NULL,
    confidence_cap REAL NOT NULL,
    payload_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_quant_evidence_symbol ON quant_evidence_snapshots(symbol, created_at);

CREATE TABLE IF NOT EXISTS quant_market_bar_meta (
    symbol TEXT NOT NULL,
    timeframe TEXT NOT NULL,
    market TEXT NOT NULL,
    venue TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (symbol, timeframe)
);

