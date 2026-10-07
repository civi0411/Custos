import React, { useState } from 'react';
import {
  Play,
  FileCode,
  MessageSquare,
  ShieldCheck,
} from 'lucide-react';
import { NotebookCellData, ParsedTableData } from '@/types/research';
import { TableChart } from './TableChart';

const MOCK_NOTEBOOK_CELLS: NotebookCellData[] = [
  {
    id: 1,
    cellType: 'markdown',
    source:
      '# Empirical Verification: Invariant Fencing Benchmarks\nEvaluating double-dispatch latency curves across 10,000 synthetic task dispatches.',
  },
  {
    id: 2,
    cellType: 'code',
    source: `import numpy as np\n\n# Generate simulated network jitter latencies (ms) vs double-dispatch rate (%)\nlatencies = np.array([5, 10, 15, 20, 25, 30, 40, 50])\noptimistic_failure = np.array([0.1, 0.4, 2.1, 7.8, 14.2, 18.9, 26.5, 33.1])\ncustos_fenced_failure = np.zeros_like(latencies)\n\nprint("Benchmark simulation completed successfully.")\nprint(f"Max Optimistic Failure: {optimistic_failure.max()}% at 50ms")\nprint(f"Max Custos Fenced Failure: {custos_fenced_failure.max()}% across all latencies")`,
    executionCount: 1,
    status: 'success',
    output: `Benchmark simulation completed successfully.\nMax Optimistic Failure: 33.1% at 50ms\nMax Custos Fenced Failure: 0.0% across all latencies`,
  },
  {
    id: 3,
    cellType: 'code',
    source: `# Comparative Convergence Table\nresults = {\n  "Latency_ms": [5, 10, 15, 20, 25, 30, 40, 50],\n  "Optimistic_FailPct": [0.1, 0.4, 2.1, 7.8, 14.2, 18.9, 26.5, 33.1],\n  "Custos_FailPct": [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]\n}`,
    executionCount: 2,
    status: 'success',
  },
];

const MOCK_CHART_TABLE: ParsedTableData = {
  headers: ['Latency_ms', 'Optimistic_FailPct', 'Custos_FailPct'],
  rows: [
    ['5', '0.1', '0.0'],
    ['10', '0.4', '0.0'],
    ['15', '2.1', '0.0'],
    ['20', '7.8', '0.0'],
    ['25', '14.2', '0.0'],
    ['30', '18.9', '0.0'],
    ['40', '26.5', '0.0'],
    ['50', '33.1', '0.0'],
  ],
};

interface NotebookWorkspacePaneProps {
  onAskAgent?: (draftText: string) => void;
  onShowToast?: (msg: string) => void;
}

export const NotebookWorkspacePane: React.FC<NotebookWorkspacePaneProps> = ({
  onAskAgent,
  onShowToast,
}) => {
  const [cells, setCells] = useState<NotebookCellData[]>(MOCK_NOTEBOOK_CELLS);
  const [runningId, setRunningId] = useState<number | null>(null);
  const [isCleanRoomReplay, setIsCleanRoomReplay] = useState<boolean>(false);

  const runCell = (cellId: number) => {
    setRunningId(cellId);
    setTimeout(() => {
      setCells((prev) =>
        prev.map((c) =>
          c.id === cellId
            ? {
                ...c,
                executionCount: (c.executionCount ?? 0) + 1,
                status: 'success',
              }
            : c
        )
      );
      setRunningId(null);
      onShowToast?.(`Cell #${cellId} executed cleanly in sandbox.`);
    }, 600);
  };

  const runAll = () => {
    onShowToast?.('Running all cells in clean-room deterministic sandbox...');
    setRunningId(2);
    setTimeout(() => {
      setRunningId(null);
      onShowToast?.('All cells passed deterministic verification.');
    }, 1200);
  };

  const handleAskAgent = (cell: NotebookCellData) => {
    const draft = `Could you explain the implications of this computation?\n\n\`\`\`python\n${cell.source}\n\`\`\`\n\nOutput:\n\`\`\`\n${cell.output || 'No output'}\n\`\`\``;
    onAskAgent?.(draft);
    onShowToast?.('Drafted cell code & output into Research Chat!');
  };

  return (
    <div className="flex flex-col h-full w-full bg-[#0d1117] text-[#c9d1d9] overflow-hidden select-none">
      {/* 32px Standard Pane Header */}
      <header className="flex h-8 shrink-0 items-center justify-between border-b border-[#21262d] bg-[#090d13] px-3">
        <div className="flex items-center gap-2">
          <FileCode className="w-3.5 h-3.5 text-[#58a6ff]" />
          <span className="text-xs font-semibold text-white">
            benchmarks_invariant_fencing.ipynb
          </span>

          {/* Interpreter Pill */}
          <div className="flex items-center gap-1.5 ml-2 px-2 py-0.5 rounded bg-[#161b22] border border-[#30363d] text-[10.5px] font-mono text-[#8b949e]">
            <span className="w-1.5 h-1.5 rounded-full bg-[#3fb950]" />
            <span>Python 3.11.8 (sade-sandbox)</span>
          </div>
        </div>

        {/* Header Actions */}
        <div className="flex items-center gap-2 shrink-0">
          {/* Clean-Room Replay Toggle */}
          <button
            onClick={() => {
              setIsCleanRoomReplay(!isCleanRoomReplay);
              onShowToast?.(
                !isCleanRoomReplay
                  ? 'Clean-Room Replay enabled: All cells execute with fresh memory isolation.'
                  : 'Standard interactive notebook execution enabled.'
              );
            }}
            className={`flex items-center gap-1 text-[11px] px-2 py-0.5 rounded transition font-mono ${
              isCleanRoomReplay
                ? 'bg-[#a371f7]/20 text-[#a371f7] border border-[#a371f7]/40 font-bold'
                : 'text-[#8b949e] hover:text-white bg-[#161b22] border border-[#30363d]'
            }`}
            title="Execute from zero in an isolated copy-on-write workspace"
          >
            <ShieldCheck size={11} />
            <span>Clean-Room Replay</span>
          </button>

          <button
            onClick={runAll}
            className="flex items-center gap-1 text-[11px] px-2 py-0.5 rounded bg-[#238636] hover:bg-[#2ea043] text-white font-medium transition"
          >
            <Play size={10} fill="currentColor" />
            <span>Run All</span>
          </button>
        </div>
      </header>

      {/* Cells Canvas Viewport */}
      <div className="flex-1 overflow-y-auto p-6 space-y-4 max-w-4xl mx-auto w-full select-text">
        {cells.map((cell) => {
          const isRunning = runningId === cell.id;

          if (cell.cellType === 'markdown') {
            return (
              <div
                key={cell.id}
                className="p-4 rounded-xl bg-[#161b22]/40 border border-[#21262d] text-sm leading-relaxed text-[#e6edf3] font-serif"
              >
                {cell.source}
              </div>
            );
          }

          return (
            <div
              key={cell.id}
              className="rounded-xl border border-[#21262d] bg-[#161b22] overflow-hidden transition"
            >
              {/* Cell Toolbar Header */}
              <div className="h-7 px-3 bg-[#090d13] border-b border-[#21262d] flex items-center justify-between text-[11px] font-mono text-[#8b949e]">
                <div className="flex items-center gap-2">
                  <span className="text-[#a371f7] font-semibold">
                    In [{cell.executionCount ?? ' '}]
                  </span>
                  <span>python</span>
                </div>

                <div className="flex items-center gap-1">
                  <button
                    onClick={() => handleAskAgent(cell)}
                    className="flex items-center gap-1 px-2 py-0.5 rounded text-[#8b949e] hover:text-[#a371f7] hover:bg-[#161b22] transition text-[10.5px]"
                    title="Ask AI Agent about this cell"
                  >
                    <MessageSquare size={11} />
                    <span>Ask Agent</span>
                  </button>
                  <button
                    onClick={() => runCell(cell.id)}
                    disabled={isRunning}
                    className="p-1 rounded text-[#8b949e] hover:text-[#3fb950] hover:bg-[#161b22] transition disabled:opacity-50"
                    title="Run Cell"
                  >
                    <Play size={11} fill="currentColor" />
                  </button>
                </div>
              </div>

              {/* Code Editor Body */}
              <div className="p-3 bg-[#0d1117] font-mono text-xs leading-relaxed overflow-x-auto text-[#c9d1d9]">
                <pre>
                  <code>{cell.source}</code>
                </pre>
              </div>

              {/* Cell Output Area */}
              {cell.output && (
                <div className="border-t border-[#21262d] bg-[#090d13] p-3 font-mono text-[11.5px] leading-relaxed text-[#8b949e]">
                  <div className="text-[10px] text-[#3fb950] font-semibold mb-1 uppercase">
                    Out [{cell.executionCount}]
                  </div>
                  <pre className="whitespace-pre-wrap">{cell.output}</pre>
                </div>
              )}

              {/* Chart Output for Cell 3 */}
              {cell.id === 3 && (
                <div className="border-t border-[#21262d] bg-[#090d13]">
                  <div className="px-3 pt-2 text-[10px] text-[#58a6ff] font-semibold uppercase font-mono">
                    Out [2] · Visualizer (TableChart SVG)
                  </div>
                  <div className="h-[280px]">
                    <TableChart table={MOCK_CHART_TABLE} />
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};
