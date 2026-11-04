# Quant Finance Domain Pack Specification

> **Scope**: `crates/custos-packs/src/quant/`  
> **Classification**: Architecture Reference  
> **Status**: Canonical Specification (Phase 3 / F08)

---

## 1. Overview & Architecture

The Quant Finance Domain Pack (`QuantPack`) provides specialized roles, jobs, and verification gates for algorithmic trading and quantitative finance workflows.

```
       [Alpha Scout] ──► [Backtest Runner] ──► [Risk Supervisor] ──► [Execution Broker]
             │                    │                      │                     │
      alpha_discover        backtest_full           risk_audit           execution_plan
             │                    │                      │                     │
       LookaheadBias        DeflatedSharpe         DeflatedSharpe       User Acceptance
          Checker              Verifier               Verifier              Oracle
```

---

## 2. Worker Roles

| Role ID | Read-Only | Token Budget | Allowed Skills | Mandate |
|---|---|---|---|---|
| `alpha_scout` | `true` | 8,000 | `file_read`, `lexical_search`, `http_fetch`, `shell_exec` | Discovers alpha signals; evaluates Information Coefficient (IC) and Information Ratio (IR) across cross-sectional market data. |
| `backtest_runner` | `false` | 20,000 | `file_read`, `shell_exec` | Executes vectorized backtests. Generates portfolio tearsheet artifacts (Sharpe, Max Drawdown, Turnover). |
| `risk_supervisor` | `true` | 10,000 | `file_read` | Audits backtests for Deflated Sharpe Ratio (Bailey-de Prado) and lookahead bias leakage. |
| `exec_broker` | `false` | 15,000 | `file_read`, `shell_exec` | Prepares target allocation and execution tickets with slippage estimates. Enforces human sign-off before trade submission. |

---

## 3. Workbench Jobs & Verification Gates

1. **`quant__alpha_discover`**:
   - Primary role: `alpha_scout` (Topology: `T1SingleWorker`).
   - Verifier: `LookaheadBiasChecker` — checks evidence metadata to guarantee no future data leakage.
2. **`quant__backtest_full`**:
   - Primary role: `backtest_runner` (Topology: `T2StagedPipeline`).
   - Verifier: `DeflatedSharpeVerifier` — requires strictly positive Sharpe ratio and validates performance metrics.
3. **`quant__risk_audit`**:
   - Primary role: `risk_supervisor` (Topology: `T0Direct`).
   - Verifier: `DeflatedSharpeVerifier` — performs DSR statistical power testing.
4. **`quant__execution_plan`**:
   - Primary role: `exec_broker` (Topology: `T1SingleWorker`).
   - Verifier: `user_acceptance_oracle` — requires explicit human approval via outbox before any simulated or live order routing.
