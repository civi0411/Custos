import React, { useState, useEffect } from 'react';
import {
  FlaskConical,
  Search,
  Check,
  RotateCcw,
  ChevronDown,
  ChevronRight,
  Cpu,
  Package,
  FileCode2,
  FileOutput,
  Copy,
  Terminal,
} from 'lucide-react';
import { ResearchExperimentRun } from '@/types/research';
import { daemonClient } from '@/api/daemon_client';

const MOCK_RUNS: ResearchExperimentRun[] = [
  {
    runId: 'run_9941a_det',
    sessionId: 'session_01',
    command: 'python simulate_jitter.py --trials 10000 --fence strict --out results.csv',
    cwd: '/workspace/experiments',
    status: 'ok',
    wallMs: 4120,
    surface: 'sandbox',
    reproducibility: 'deterministic',
    inputMerkleRoot: 'blake3_root_in_8829a',
    outputMerkleRoot: 'blake3_root_out_3310b',
    envSnapshot: {
      pythonVersion: '3.11.8',
      lockfileHash: 'bafy2bzace4v3k99a77z',
      packageCount: 64,
      hardware: 'Apple M2 Pro (10 Cores, 32GB RAM)',
    },
    codeFiles: [{ path: 'simulate_jitter.py', hash: 'blake3_f721a' }],
    outputFiles: [{ path: 'results.csv', hash: 'blake3_bb91a', size: 14820 }],
    logText: `[Custos SADE Sandbox] Spawning bounded workspace with ticket #perm_8821\n[Run] Loading parameters: trials=10000, fence=strict\n[Run] Processed 10000 iterations. Zero divergence.\n[Result] Output written to results.csv. Merkle hash verified.\n[SADE] Invariant gate sealed: cas://bafy2bzace4v3k99a77z`,
    ts: Date.now() - 1800000,
  },
  {
    runId: 'run_7718b_hpc',
    sessionId: 'session_01',
    command: 'torchrun --nproc_per_node=4 train_esm_folding.py --epochs 5',
    cwd: '/workspace/models',
    status: 'ok',
    wallMs: 84210,
    surface: 'hpc',
    reproducibility: 'deterministic',
    inputMerkleRoot: 'blake3_root_in_1120x',
    outputMerkleRoot: 'blake3_root_out_9941y',
    envSnapshot: {
      pythonVersion: '3.10.12',
      lockfileHash: 'bafy2bzace4v78q1b99m',
      packageCount: 112,
      hardware: '4x NVIDIA A100-SXM4-80GB (CUDA 12.2)',
    },
    codeFiles: [{ path: 'train_esm_folding.py', hash: 'blake3_c441b' }],
    outputFiles: [
      { path: 'checkpoint_epoch5.pt', hash: 'blake3_a001z', size: 1420994120 },
      { path: 'metrics.json', hash: 'blake3_m112a', size: 4096 },
    ],
    logText: `[HPC Job 99281] Slurm allocation granted on node cn-a100-04\n[Epoch 1/5] Loss: 0.412 - Val RMSD: 1.28 A\n[Epoch 5/5] Loss: 0.089 - Val RMSD: 0.44 A\n[Checkpoint] Saved to checkpoint_epoch5.pt`,
    ts: Date.now() - 7200000,
  },
  {
    runId: 'run_4412c_fail',
    sessionId: 'session_01',
    command: 'python evaluate_raw_bash.py --no-sandbox',
    cwd: '/workspace/tests',
    status: 'failed',
    wallMs: 820,
    surface: 'local',
    reproducibility: 'unverified',
    inputMerkleRoot: 'blake3_root_in_bad99',
    outputMerkleRoot: 'blake3_root_out_empty',
    envSnapshot: {
      pythonVersion: '3.11.8',
      lockfileHash: 'bafy2bzace_unverified',
      packageCount: 64,
      hardware: 'Apple M2 Pro',
    },
    logText: `[FATAL] Invariant Gate INV-03 Zero-IO Violation: Command attempted unpermitted socket connection to 0.0.0.0.\nExecution fenced immediately. Exit Code: 137`,
    ts: Date.now() - 14400000,
  },
];

interface RunsLedgerPaneProps {
  onReproduce?: (run: ResearchExperimentRun) => void;
  onShowToast?: (msg: string) => void;
}

export const RunsLedgerPane: React.FC<RunsLedgerPaneProps> = ({
  onReproduce,
  onShowToast,
}) => {
  const [runs, setRuns] = useState<ResearchExperimentRun[]>(MOCK_RUNS);
  const [search, setSearch] = useState('');
  const [statusFilter, setStatusFilter] = useState<'all' | 'ok' | 'failed'>('all');
  const [expandedId, setExpandedId] = useState<string | null>(MOCK_RUNS[0].runId);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  useEffect(() => {
    daemonClient
      .listResearchRuns()
      .then((loaded) => {
        if (loaded && loaded.length > 0) {
          setRuns(loaded);
          setExpandedId(loaded[0].runId);
        }
      })
      .catch((err) => console.warn('Using mock runs fallback:', err));
  }, []);

  const filteredRuns = runs.filter((r) => {
    const matchesSearch =
      r.command.toLowerCase().includes(search.toLowerCase()) ||
      r.runId.toLowerCase().includes(search.toLowerCase());
    const matchesStatus =
      statusFilter === 'all' || (statusFilter === 'ok' ? r.status === 'ok' : r.status === 'failed');
    return matchesSearch && matchesStatus;
  });

  const handleCopyCommand = (run: ResearchExperimentRun) => {
    navigator.clipboard.writeText(run.command);
    setCopiedId(run.runId);
    setTimeout(() => setCopiedId(null), 1500);
  };

  const handleReproduce = (run: ResearchExperimentRun) => {
    onReproduce?.(run);
    onShowToast?.(`Drafted reproducible prompt for ${run.runId} into Research Chat!`);
  };

  return (
    <div className="flex flex-col h-full w-full bg-[#04080F] text-[#e0e6ed] overflow-hidden select-none font-sans relative">
      
      {/* 32px Standard Pane Header */}
      <header className="flex h-14 shrink-0 items-center justify-between border-b border-[#1e2430] bg-[#080d16]/80  px-5 relative z-10">
        <div className="flex items-center gap-3">
          <div className="p-1.5 rounded-lg bg-[#21262d] shadow-lg ">
            <FlaskConical className="w-4 h-4 text-white" />
          </div>
          <span className="text-[14px] font-bold text-white tracking-wide">
            Experiment Ledger
          </span>
          <span className="text-[11px] text-[#8b949e] font-sans font-semibold bg-[#21262d] px-2 py-0.5 rounded-md border border-[#30363d] ml-2">
            {runs.length} recorded runs
          </span>
        </div>
      </header>

      {/* Filter Toolbar */}
      <div className="p-4 border-b border-[#1e2430] bg-[#21262d] flex flex-wrap items-center gap-4 text-xs relative z-10">
        {/* Search */}
        <div className="relative min-w-[250px] flex-1 group">
          <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-[#6e7681] group-focus-within:text-white transition-colors" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search commands, run IDs..."
            className="w-full bg-[#111722] border border-[#2d3342] rounded-xl py-2 pl-9 pr-3 text-sm text-[#e0e6ed] placeholder-[#6e7681] outline-none focus:outline-none transition-all shadow-inner"
          />
        </div>

        {/* Facet Chips */}
        <div className="flex items-center gap-2">
          <button
            onClick={() => setStatusFilter(statusFilter === 'ok' ? 'all' : 'ok')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-[11.5px] font-bold transition-all ${
              statusFilter === 'ok'
                ? 'bg-emerald-500/10 border-emerald-500/40 text-emerald-400 '
                : 'bg-[#111722] border-[#2d3342] text-[#8b949e] hover:bg-[#161c28] hover:text-white'
            }`}
          >
            <span className={`w-1.5 h-1.5 rounded-full ${statusFilter === 'ok' ? 'bg-emerald-400 ' : 'bg-[#6e7681]'}`} />
            <span>Success</span>
          </button>

          <button
            onClick={() => setStatusFilter(statusFilter === 'failed' ? 'all' : 'failed')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-[11.5px] font-bold transition-all ${
              statusFilter === 'failed'
                ? 'bg-rose-500/10 border-rose-500/40 text-rose-400 '
                : 'bg-[#111722] border-[#2d3342] text-[#8b949e] hover:bg-[#161c28] hover:text-white'
            }`}
          >
            <span className={`w-1.5 h-1.5 rounded-full ${statusFilter === 'failed' ? 'bg-rose-400 ' : 'bg-[#6e7681]'}`} />
            <span>Failed</span>
          </button>
        </div>
      </div>

      {/* Runs List Scroll Area */}
      <div className="flex-1 overflow-y-auto p-6 space-y-4 select-text relative z-10 max-w-5xl mx-auto w-full">
        {filteredRuns.map((run) => {
          const isExpanded = expandedId === run.runId;
          const isOk = run.status === 'ok';

          return (
            <div
              key={run.runId}
              className={`rounded-2xl border transition-all duration-300 overflow-hidden ${
                isExpanded
                  ? `bg-[#0d131f] border-${isOk ? 'emerald-500/30' : 'rose-500/30'} shadow-xl`
                  : 'bg-[#111722]/80 border-[#1e2430] hover:border-[#2d3342] hover:bg-[#161c28]'
              }`}
            >
              {/* Row Header Button */}
              <div
                onClick={() => setExpandedId(isExpanded ? null : run.runId)}
                className={`p-4 flex flex-wrap items-center justify-between gap-2 gap-4 cursor-pointer transition select-none ${isExpanded ? `bg-${isOk ? 'emerald-500' : 'rose-500'}/5` : 'hover:bg-white/5'}`}
              >
                <div className="flex items-center gap-3 min-w-0">
                  <div className={`p-1 rounded-md ${isExpanded ? (isOk ? 'bg-emerald-500/20 text-emerald-400' : 'bg-rose-500/20 text-rose-400') : 'bg-[#1e2430] text-[#8b949e]'}`}>
                    {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                  </div>
                  <span
                    className={`w-2.5 h-2.5 rounded-full shrink-0 shadow-sm ${
                      isOk ? 'bg-emerald-400 ' : 'bg-rose-400 '
                    }`}
                  />
                  <span className="font-sans text-[13px] text-white font-medium truncate tracking-tight">
                    {run.command}
                  </span>
                </div>

                <div className="flex items-center gap-4 shrink-0 text-[11px] text-[#8b949e] font-sans">
                  {run.surface && (
                    <span className={`px-2 py-0.5 rounded-md uppercase font-bold text-[10px] ${
                      run.surface === 'sandbox' ? 'bg-[#21262d] text-neutral-300 border border-[#30363d]' : 
                      run.surface === 'hpc' ? 'bg-[#21262d] text-neutral-300 border border-[#30363d]' : 
                      'bg-amber-500/10 text-amber-400 border border-amber-500/20'
                    }`}>
                      {run.surface}
                    </span>
                  )}
                  <span className="bg-[#1e2430] px-2 py-0.5 rounded-md">{(run.wallMs / 1000).toFixed(1)}s</span>
                  <span className="text-[#6e7681]">{new Date(run.ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>
                </div>
              </div>

              {/* Expanded Details Pane */}
              <div className={`transition-all duration-300 ease-in-out ${isExpanded ? 'max-h-[1000px] opacity-100' : 'max-h-0 opacity-0 overflow-hidden'}`}>
                <div className="border-t border-[#1e2430] p-6 bg-[#080d16]/50 space-y-6">
                  
                  {/* Action Bar */}
                  <div className="flex flex-wrap items-center justify-between gap-2 text-[11.5px]">
                    <div className="flex items-center gap-3">
                      <button
                        onClick={() => handleReproduce(run)}
                        className="group flex items-center gap-2 px-4 py-1.5 rounded-lg bg-[#21262d] text-white font-bold  transition-all  "
                        title="Generate verification prompt with exact environment & lockfile"
                      >
                        <RotateCcw size={14} className="group-hover:-rotate-90 transition-transform duration-500" />
                        <span>Reproduce Run</span>
                      </button>

                      <button
                        onClick={() => handleCopyCommand(run)}
                        className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-[#111722] border border-[#2d3342] text-[#a1abb7] hover:text-white hover:border-[#6e7681] transition-colors font-medium"
                      >
                        {copiedId === run.runId ? (
                          <Check size={14} className="text-emerald-400" />
                        ) : (
                          <Copy size={14} />
                        )}
                        <span>Copy Command</span>
                      </button>
                    </div>

                    <div className="font-sans text-[11px] text-[#6e7681] bg-[#111722] px-3 py-1.5 rounded-lg border border-[#1e2430]">
                      Merkle Hash: <span className="text-neutral-300 ml-1 font-semibold">{run.outputMerkleRoot}</span>
                    </div>
                  </div>

                  {/* Hardware & Env Chips */}
                  <div className="flex flex-wrap items-center gap-3 font-sans text-[11px]">
                    <span className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#0d131f] text-[#c9d1d9] border border-[#1e2430] shadow-inner">
                      <Cpu size={12} className="text-neutral-300" />
                      <span className="font-semibold">{run.envSnapshot.hardware}</span>
                    </span>

                    <span className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#0d131f] text-[#c9d1d9] border border-[#1e2430] shadow-inner">
                      <Package size={12} className="text-[#8b949e]" />
                      <span className="font-semibold">Python {run.envSnapshot.pythonVersion}</span>
                      <span className="text-[#6e7681]">({run.envSnapshot.packageCount} pkgs)</span>
                    </span>

                    <span className="px-3 py-1.5 rounded-lg bg-[#21262d] text-neutral-300 border border-[#30363d] font-semibold shadow-inner">
                      Lockfile: {run.envSnapshot.lockfileHash.slice(0, 16)}...
                    </span>
                  </div>

                  {/* Input Code & Outputs */}
                  <div className="grid grid-cols-2 gap-4 text-[11px]">
                    {run.codeFiles && (
                      <div className="p-4 rounded-xl bg-[#0d131f]/80 border border-[#1e2430] space-y-3">
                        <div className="text-[10px] text-[#6e7681] font-bold uppercase tracking-wider flex items-center gap-1.5">
                          <FileCode2 size={12} className="text-neutral-300" /> 
                          Code Inputs
                        </div>
                        <div className="space-y-2">
                          {run.codeFiles.map((f) => (
                            <div key={f.path} className="font-sans text-[#e0e6ed] flex justify-between items-center bg-[#111722] p-2 rounded-lg border border-[#2d3342]">
                              <span>{f.path}</span>
                              <span className="text-[#6e7681] text-[10px]">{f.hash}</span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}

                    {run.outputFiles && (
                      <div className="p-4 rounded-xl bg-[#0d131f]/80 border border-[#1e2430] space-y-3">
                        <div className="text-[10px] text-[#6e7681] font-bold uppercase tracking-wider flex items-center gap-1.5">
                          <FileOutput size={12} className="text-[#8b949e]" /> 
                          Captured Outputs
                        </div>
                        <div className="space-y-2">
                          {run.outputFiles.map((f) => (
                            <div key={f.path} className="font-sans text-[#e0e6ed] flex justify-between items-center bg-[#111722] p-2 rounded-lg border border-[#2d3342]">
                              <span>{f.path}</span>
                              <span className="text-[#8b949e] font-bold bg-[#21262d] px-1.5 py-0.5 rounded text-[10px]">{(f.size / 1024).toFixed(1)} KB</span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>

                  {/* Stdout / Stderr Log */}
                  {run.logText && (
                    <div className="rounded-xl bg-[#04080F] border border-[#1e2430] overflow-hidden shadow-inner">
                      <div className="bg-[#080d16] px-4 py-2 border-b border-[#1e2430] text-[10px] text-neutral-300 font-bold uppercase tracking-widest flex items-center gap-1.5">
                        <Terminal size={12} /> 
                        Execution Trace
                      </div>
                      <div className="p-4 font-mono text-[11.5px] leading-[1.7] text-[#8b949e] overflow-x-auto">
                        <pre className="whitespace-pre-wrap">
                          {run.logText.split('\n').map((line, i) => {
                            let colorClass = 'text-[#8b949e]';
                            if (line.includes('[FATAL]') || line.includes('Violation')) colorClass = 'text-rose-400 font-bold';
                            else if (line.includes('[Result]') || line.includes('verified')) colorClass = 'text-emerald-400';
                            else if (line.includes('[SADE]')) colorClass = 'text-neutral-300';
                            
                            return (
                              <div key={i} className={colorClass}>{line}</div>
                            );
                          })}
                        </pre>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
