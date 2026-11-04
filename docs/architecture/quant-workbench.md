# Quant Workbench UI & Screen Presets Specification

> **Scope**: `ui/desktop/src/components/workbenches/quant/`, `ui/desktop/src/types/screen.ts`  
> **Classification**: UI & Architecture Reference  
> **Status**: Canonical Specification (Phase 3 / F09)

---

## 1. 4-Panel Grid Architecture

The Quant Workbench provides a 4-pane analytical canvas:

```
┌─────────────────────────────────┬─────────────────────────────────┐
│ Alpha Studio Pane               │ Tearsheet Pane                  │
│ • Signal Roster & Hypothesis    │ • Equity Curve (Log / Linear)   │
│ • IC Mean, IC Std, IR Metrics   │ • Monthly Returns Heatmap       │
│ • Factor Decay Profile          │ • Max Drawdown & Underwater Ser │
├─────────────────────────────────┼─────────────────────────────────┤
│ Risk Audit Pane                 │ Execution Pane                  │
│ • Deflated Sharpe Ratio (DSR)   │ • Target Position Rebalance     │
│ • Lookahead Bias Clean Verifier │ • Estimated Slippage (bps)      │
│ • Overfit Gauge (0-1 Score)     │ • Human Sign-off Order Ticket   │
└─────────────────────────────────┴─────────────────────────────────┘
```

---

## 2. Lazy Loading & Bundle Optimization
All four quant pane components are imported dynamically via `React.lazy()` in `ResourceContent.tsx`:
- `AlphaStudioPane.tsx`
- `TearsheetPane.tsx`
- `RiskAuditPane.tsx`
- `ExecutionPane.tsx`

Unused quant panes produce discrete Vite code-split chunks, preserving startup performance for standard coding and research sessions.

---

## 3. Screen Presets
The workbench registers two canonical screen presets in `ui/desktop/src/types/screen.ts`:
1. **`preset_quant_studio`**:
   - Split: 50/50 horizontal.
   - Left: Conversation / Strategy Briefing (`ref: { kind: 'conversation' }`).
   - Right: Alpha Studio Factor Analysis (`ref: { kind: 'alpha' }`).
2. **`preset_quant_risk`**:
   - Split: 50/50 horizontal.
   - Left: Performance Tearsheet (`ref: { kind: 'tearsheet' }`).
   - Right: Risk Audit & Overfit Verification (`ref: { kind: 'risk_audit' }`).
