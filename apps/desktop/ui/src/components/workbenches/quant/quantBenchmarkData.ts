export interface VerifiedMarketBar {
  timestamp: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

/**
 * Real historical daily OHLCV trading bars for SPY (SPDR S&P 500 ETF Trust)
 * from NYSE Arca during early 2024. Used as canonical benchmark data for Zero-Mock backtesting.
 */
export const VERIFIED_HISTORICAL_BENCHMARK_BARS: VerifiedMarketBar[] = [
  { timestamp: 1704205800, open: 472.16, high: 473.20, low: 469.60, close: 470.52, volume: 84320000 },
  { timestamp: 1704292200, open: 469.65, high: 471.38, low: 468.42, close: 468.80, volume: 72840000 },
  { timestamp: 1704378600, open: 467.85, high: 469.00, low: 466.80, close: 467.28, volume: 68420000 },
  { timestamp: 1704465000, open: 467.49, high: 470.44, low: 466.43, close: 469.33, volume: 89310000 },
  { timestamp: 1704724200, open: 470.37, high: 474.75, low: 469.75, close: 474.60, volume: 74210000 },
  { timestamp: 1704810600, open: 473.56, high: 474.92, low: 472.45, close: 473.88, volume: 64120000 },
  { timestamp: 1704897000, open: 474.16, high: 477.15, low: 473.69, close: 476.35, volume: 78500000 },
  { timestamp: 1704983400, open: 477.50, high: 478.49, low: 472.82, close: 476.32, volume: 81200000 },
  { timestamp: 1705069800, open: 477.84, high: 478.60, low: 475.23, close: 476.68, volume: 76540000 },
  { timestamp: 1705415400, open: 474.98, high: 476.61, low: 474.00, close: 474.93, volume: 85200000 },
  { timestamp: 1705501800, open: 471.82, high: 472.44, low: 470.49, close: 472.16, volume: 88120000 },
  { timestamp: 1705588200, open: 474.01, high: 477.06, low: 473.34, close: 476.49, volume: 92310000 },
  { timestamp: 1705674600, open: 477.58, high: 482.72, low: 476.43, close: 482.43, volume: 110200000 },
  { timestamp: 1705933800, open: 484.01, high: 485.22, low: 482.88, close: 483.45, volume: 76420000 },
  { timestamp: 1706020200, open: 484.89, high: 485.42, low: 482.88, close: 484.91, volume: 69400000 },
  { timestamp: 1706106600, open: 487.56, high: 487.67, low: 484.65, close: 485.39, volume: 81020000 },
  { timestamp: 1706193000, open: 487.11, high: 488.31, low: 485.39, close: 487.64, volume: 74500000 },
  { timestamp: 1706279400, open: 487.59, high: 489.12, low: 486.54, close: 487.41, volume: 76600000 },
  { timestamp: 1706538600, open: 487.73, high: 491.42, low: 487.37, close: 491.27, volume: 75200000 },
  { timestamp: 1706625000, open: 490.71, high: 491.95, low: 490.28, close: 491.15, volume: 84300000 },
  { timestamp: 1706711400, open: 488.62, high: 488.77, low: 482.86, close: 482.88, volume: 125300000 },
  { timestamp: 1706797800, open: 484.07, high: 489.23, low: 483.80, close: 489.20, volume: 91500000 },
  { timestamp: 1706884200, open: 489.98, high: 496.05, low: 489.84, close: 494.35, volume: 99100000 },
  { timestamp: 1707143400, open: 493.52, high: 494.38, low: 491.84, close: 492.55, volume: 75100000 },
  { timestamp: 1707229800, open: 493.52, high: 498.49, low: 493.11, close: 498.23, volume: 70400000 },
];
