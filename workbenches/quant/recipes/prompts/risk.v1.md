# Quantitative Risk Audit Prompt

Audit algorithmic backtest returns for false discoveries and multiple testing bias.
Requirements:
1. Compute the Deflated Sharpe Ratio (DSR) taking trial count into account.
2. Calculate the Probability of Backtest Overfitting (PBO) via Combinatorially Symmetric Cross-Validation (CSCV).
3. Reject any strategy failing the DSR confidence threshold (p-value < 0.05).
