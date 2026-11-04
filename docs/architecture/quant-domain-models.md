# Quantitative Domain Models & Financial Governance

> **Status:** Canonical SSOT  
> **Layer:** Layer 0 (`custos-domain`) — Pure Zero-I/O Invariant  
> **Crates:** `custos-domain`, `custos-packs`, `custos-daemon`

---

## 1. Architectural Role & Boundary Invariants

The Quantitative Trading Workbench introduces financial primitives into Custos without violating the inviolable **Layer 0 Zero-I/O Invariant**. All structs in `custos-domain::quant` are:
1. **Pure & Deterministic:** No socket, disk, or clock-mutating dependencies.
2. **Memory-Safe:** Bounded vectors, float validations, and explicit error domains via `DomainError`.
3. **Cross-Workbench Compatible:** Seamlessly bridges with `custos-domain::session::assistant::OutboxActionItem` and `custos-domain::authority::ExecutionPermit`.

```
+-----------------------------------------------------------------------------------+
|                              CUSTOS WORKSPACE                                      |
|                                                                                   |
|  [Coding Workbench]       [Research Workbench]      [Assistant / Copilot]         |
|         │                          │                         ▲                    |
|         │                          │                         │ OutboxActionItem   |
|         ▼                          ▼                         │                    |
|  ┌───────────────────────────────────────────────────────────┴─────────────────┐  |
|  │                        QUANTITATIVE WORKBENCH                                │  |
|  │                                                                             │  |
|  │  1. AlphaStudioPane    ──> AlphaFactor (IC, IR, lookahead_clean)            │  |
|  │  2. TearsheetPane      ──> BacktestTearsheet (Sharpe, DSR, Calmar)          │  |
|  │  3. RiskAuditPane      ──> RiskPermitConfig (VaR 95%, Drawdown, Leverage)  │  |
|  │  4. ExecutionPane      ──> OrderTicket (TWAP/VWAP, Circuit Breakers)        │  |
|  └─────────────────────────────────────────────────────────────────────────────┘  |
+-----------------------------------------------------------------------------------+
```

---

## 2. Core Entities

### 2.1 `AlphaFactor` (`custos_domain::quant::AlphaFactor`)
Models statistical signals and predictive indicators with lookahead bias tracking:
- `id`: Unique identifier (`factor_<nanoid>`).
- `name`: Human-readable identifier (e.g. `momentum_reversal_5d`).
- `hypothesis`: Grounding description of the market inefficiency.
- `formula`: Vectorized expression (e.g. `ts_rank(close / delay(close, 5), 20)`).
- `ic_mean`, `ic_std`, `ir`: Information Coefficient mean, standard deviation, and Information Ratio.
- `decay_days`: Half-life decay horizon.
- `lookahead_clean`: Boolean gate asserted by AST inspection or independent verifiers.
- `is_viable()`: Checks $|IC| \ge 0.02$, $IR \ge 0.5$, and `lookahead_clean == true`.

### 2.2 `BacktestTearsheet` (`custos_domain::quant::BacktestTearsheet`)
Captures vectorized backtest performance, risk-adjusted returns, and distribution profiles:
- `strategy_id`: Strategy or factor identifier.
- `annualized_sharpe`: Classic annualized Sharpe Ratio.
- `deflated_sharpe`: Bailey & López de Prado (2016) Deflated Sharpe Ratio (DSR).
- `max_drawdown`: Peak-to-trough decline percentage (validated in range $[0.0, 1.0]$).
- `total_return`: Cumulative strategy return.
- `calmar_ratio`: Ratio of total return to max drawdown.
- `monthly_returns`: Vector of `MonthlyReturn { year, month, return_pct }`.
- `is_passing_hurdle(min_sharpe, max_dd_limit)`: Deterministic qualification gate.

### 2.3 `RiskPermitConfig` (`custos_domain::quant::RiskPermitConfig`)
Enforces hard quantitative boundaries and circuit breakers:
- `max_position_pct`: Maximum single-instrument portfolio weight (e.g. 10%).
- `max_leverage`: Gross leverage ceiling (e.g. 1.5x).
- `stop_loss_pct`: Maximum tolerable asset drawdown before automated liquidation.
- `var_95`: Parametric 95% 1-day Value at Risk.
- `max_drawdown_limit`: Portfolio-level shutdown trigger.
- `dsr_threshold`: Minimum required Deflated Sharpe Ratio (e.g. 0.0).
- `audit_tearsheet(&self, tearsheet)`: Returns `Result<(), RiskBreach>`.
- `validate_order_weight(&self, target_weight)`: Detects position size violations before submission.

### 2.4 `OrderTicket` (`custos_domain::quant::OrderTicket`)
Models individual trade instructions with algorithmic routing and governance:
- `symbol`: Ticker or contract identifier (e.g. `NVDA`, `BTC-USDT`).
- `direction`: `Buy`, `Sell`, `SellShort`, `BuyToCover`.
- `order_type`: `Market`, `Limit`, `Stop`, `Twap`, `Vwap`.
- `quantity`: Float unit quantity.
- `target_weight`: Target fraction of portfolio NAV.
- `estimated_slippage_bps`: Estimated transaction friction in basis points.
- `approved_by_human`: Invariant boolean requiring explicit operator sign-off.
- `status`: `Draft` $\to$ `Approved` | `Rejected` $\to$ `Routed` $\to$ `Filled` | `Cancelled`.
- `to_outbox_action_item(task_id)`: Bridges order execution directly into the Copilot Outbox with SHA256 digest locking.

---

## 3. Cross-Domain Governance Flow

1. **Discovery (Quant Workbench):** `quant__alpha_discover` proposes `AlphaFactor`. `LookaheadBiasChecker` verifies absence of future-data leakage.
2. **Simulation (Quant Workbench):** `quant__backtest_full` generates `BacktestTearsheet`. `DeflatedSharpeVerifier` validates $DSR > 0$.
3. **Audit (Quant Workbench):** `quant__risk_audit` tests performance against `RiskPermitConfig`.
4. **Handoff (Quant $\to$ Assistant / Copilot):** `quant__execution_plan` emits `OrderTicket` records. Calling `to_outbox_action_item()` creates governed `OutboxActionItem` entries with `action_type: "quant_order_execution"`.
5. **Execution (Authority):** Operator reviews order tickets in `ExecutionPane` or `OutboxPane` and approves them with single-use `ExecutionPermit` tokens.
