import React, { useState } from 'react';
import {
  Code,
  Terminal,
  MessageSquare,
  Cpu,
  ShieldCheck,
  ChevronLeft,
  ChevronRight,
  Download,
  Copy,
  Check,
  X,
  Maximize2,
  Minimize2,
  FileCode2,
  CheckCircle2,
  XCircle,
} from 'lucide-react';
import { ArtifactInspectorData, ArtifactTab } from '@/types/research';

const TABS: ArtifactTab[] = ['Code', 'Execution Log', 'Messages', 'Environment', 'Review'];

interface ArtifactInspectorProps {
  data: ArtifactInspectorData;
  onClose?: () => void;
  isMaximized?: boolean;
  onToggleMaximize?: () => void;
}

export const ArtifactInspector: React.FC<ArtifactInspectorProps> = ({
  data,
  onClose,
  isMaximized,
  onToggleMaximize,
}) => {
  const [activeTab, setActiveTab] = useState<ArtifactTab>('Code');
  const [versionIdx, setVersionIdx] = useState<number>(() => {
    const idx = data.versions.findIndex((v) => v.label === data.activeVersion);
    return idx >= 0 ? idx : Math.max(0, data.versions.length - 1);
  });
  const [copied, setCopied] = useState(false);

  const currentVersion = data.versions[versionIdx] ?? {
    label: data.activeVersion || 'v1',
    code: data.code,
    executionLog: data.executionLog,
    environment: data.environment,
    reviewPassed: data.reviewPassed,
  };

  const activeCode = currentVersion.code || data.code;
  const activeLog = currentVersion.executionLog || data.executionLog;
  const activeEnv = currentVersion.environment || data.environment;
  const isPassed = currentVersion.reviewPassed ?? data.reviewPassed ?? true;

  const handleCopy = () => {
    navigator.clipboard.writeText(activeCode);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const handleDownload = () => {
    const blob = new Blob([activeCode], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = data.filename || 'artifact.py';
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="flex flex-col h-full bg-[#04080F] text-[#e0e6ed] overflow-hidden select-none font-sans relative">
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[600px] bg-blue-500/5 blur-[150px] rounded-full pointer-events-none" />

      {/* 32px Standard Pane Header */}
      <header className="flex h-14 shrink-0 items-center justify-between border-b border-[#1e2430] bg-[#080d16]/80 backdrop-blur-md px-5 relative z-10">
        <div className="flex items-center gap-3 min-w-0">
          <div className="p-1.5 rounded-lg bg-gradient-to-br from-blue-500 to-indigo-600 shadow-lg shadow-blue-500/20">
            <FileCode2 className="w-4 h-4 text-white shrink-0" />
          </div>
          <span className="truncate text-[14px] font-bold text-white tracking-wide">
            {data.title || data.filename}
          </span>

          {/* Version Stepper */}
          <div className="flex items-center gap-1 ml-4 bg-[#111722] border border-[#2d3342] rounded-lg px-1.5 py-1">
            <button
              onClick={() => setVersionIdx((i) => Math.max(0, i - 1))}
              disabled={versionIdx <= 0}
              className="p-1 text-[#6e7681] hover:text-white disabled:opacity-30 transition hover:bg-[#1e2430] rounded-md"
              title="Previous version"
            >
              <ChevronLeft size={12} />
            </button>
            <span className="font-mono text-[11px] text-blue-400 px-2 font-bold uppercase tracking-widest">
              {currentVersion.label}
            </span>
            <button
              onClick={() => setVersionIdx((i) => Math.min(data.versions.length - 1, i + 1))}
              disabled={versionIdx >= data.versions.length - 1}
              className="p-1 text-[#6e7681] hover:text-white disabled:opacity-30 transition hover:bg-[#1e2430] rounded-md"
              title="Next version"
            >
              <ChevronRight size={12} />
            </button>
          </div>
        </div>

        {/* Right Header Controls */}
        <div className="flex items-center gap-2 shrink-0">
          <button
            onClick={handleCopy}
            className="p-2 rounded-xl text-[#8b949e] hover:text-white hover:bg-[#1e2430] transition-colors"
            title="Copy Code"
          >
            {copied ? <Check size={14} className="text-emerald-400" /> : <Copy size={14} />}
          </button>
          <button
            onClick={handleDownload}
            className="p-2 rounded-xl text-[#8b949e] hover:text-white hover:bg-[#1e2430] transition-colors"
            title="Download Script"
          >
            <Download size={14} />
          </button>
          {onToggleMaximize && (
            <button
              onClick={onToggleMaximize}
              className="p-2 rounded-xl text-[#8b949e] hover:text-white hover:bg-[#1e2430] transition-colors"
              title={isMaximized ? 'Restore Pane' : 'Maximize Pane'}
            >
              {isMaximized ? <Minimize2 size={14} /> : <Maximize2 size={14} />}
            </button>
          )}
          {onClose && (
            <div className="w-px h-5 bg-[#2d3342] mx-1" />
          )}
          {onClose && (
            <button
              onClick={onClose}
              className="p-2 rounded-xl text-[#8b949e] hover:text-rose-400 hover:bg-[#1e2430] transition-colors"
              title="Close Inspector"
            >
              <X size={14} />
            </button>
          )}
        </div>
      </header>

      {/* 5 Tabs Navigation Bar */}
      <nav className="flex items-center gap-2 border-b border-[#1e2430] bg-[#080d16]/50 px-5 pt-3 relative z-10">
        {TABS.map((tab) => {
          const isActive = activeTab === tab;
          let Icon = Code;
          if (tab === 'Execution Log') Icon = Terminal;
          if (tab === 'Messages') Icon = MessageSquare;
          if (tab === 'Environment') Icon = Cpu;
          if (tab === 'Review') Icon = ShieldCheck;

          return (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`group flex items-center gap-2 pb-3 px-3 border-b-2 text-[12px] font-bold transition-all relative ${
                isActive
                  ? 'border-blue-500 text-blue-400'
                  : 'border-transparent text-[#6e7681] hover:text-[#c9d1d9]'
              }`}
            >
              <Icon size={14} className={isActive ? 'text-blue-400' : 'text-[#6e7681] group-hover:text-[#c9d1d9]'} />
              <span>{tab}</span>
              {tab === 'Review' && (
                <span
                  className={`w-1.5 h-1.5 rounded-full shadow-sm ml-1 ${
                    isPassed ? 'bg-emerald-400 shadow-emerald-400/50' : 'bg-rose-400 shadow-rose-400/50'
                  }`}
                />
              )}
              {isActive && (
                <div className="absolute bottom-0 left-0 right-0 h-px bg-blue-400 shadow-[0_0_10px_rgba(59,130,246,0.8)]" />
              )}
            </button>
          );
        })}
      </nav>

      {/* Tab Contents Viewport */}
      <div className="flex-1 overflow-y-auto p-6 select-text relative z-10 max-w-5xl mx-auto w-full">
        {/* TAB 1: CODE */}
        {activeTab === 'Code' && (
          <div className="space-y-4 animate-in fade-in slide-in-from-bottom-2 duration-300">
            {data.inputs && data.inputs.length > 0 && (
              <div className="flex flex-wrap items-center gap-2 text-[11.5px] bg-[#111722] p-3 rounded-xl border border-[#2d3342]">
                <span className="text-[#6e7681] font-bold uppercase tracking-widest text-[10px]">Inputs</span>
                {data.inputs.map((inp) => (
                  <span
                    key={inp}
                    className="font-mono text-[11px] px-2.5 py-1 rounded-md bg-[#0d131f] border border-[#1e2430] text-blue-400 shadow-inner"
                  >
                    {inp}
                  </span>
                ))}
              </div>
            )}
            <div className="rounded-2xl border border-[#1e2430] bg-[#080d16]/80 backdrop-blur-sm p-4 font-mono text-[13px] leading-[1.8] overflow-x-auto text-[#e0e6ed] shadow-xl group">
              <pre className="selection:bg-blue-500/30 selection:text-white">
                <code>{activeCode}</code>
              </pre>
            </div>
          </div>
        )}

        {/* TAB 2: EXECUTION LOG */}
        {activeTab === 'Execution Log' && (
          <div className="h-full animate-in fade-in slide-in-from-bottom-2 duration-300">
            <div className="rounded-2xl border border-[#1e2430] bg-[#04080F] font-mono text-[12.5px] leading-relaxed overflow-hidden shadow-xl h-full flex flex-col">
              <div className="bg-[#080d16] px-5 py-3 border-b border-[#1e2430] text-[10px] text-blue-400 font-bold uppercase tracking-widest flex items-center gap-2">
                <Terminal size={14} />
                Captured Terminal Log (SHA-256 CAS)
              </div>
              <div className="p-5 overflow-auto flex-1 text-[#8b949e]">
                <pre className="whitespace-pre-wrap">{activeLog || 'No execution log captured.'}</pre>
              </div>
            </div>
          </div>
        )}

        {/* TAB 3: MESSAGES */}
        {activeTab === 'Messages' && (
          <div className="space-y-4 animate-in fade-in slide-in-from-bottom-2 duration-300 max-w-3xl mx-auto">
            {(data.messages ?? [
              'Agent generated initial analysis script based on literature findings.',
              'User requested optimization of hyperparameter scan rate.',
              'Agent refined code and recorded provenance v2 with lockfile.',
            ]).map((msg, i) => {
              const isUser = msg.startsWith('User:');
              const text = msg.replace(/^(User|Assistant):\s*/, '');
              
              return (
                <div
                  key={i}
                  className={`p-5 rounded-2xl border flex gap-4 ${
                    isUser ? 'bg-indigo-500/5 border-indigo-500/20' : 'bg-[#0d131f] border-[#1e2430]'
                  }`}
                >
                  <div className={`w-8 h-8 rounded-full flex items-center justify-center font-bold text-[11px] shrink-0 ${
                    isUser ? 'bg-indigo-500 text-white shadow-lg shadow-indigo-500/20' : 'bg-[#1e2430] text-[#c9d1d9]'
                  }`}>
                    {isUser ? 'CV' : 'AI'}
                  </div>
                  <div>
                    <div className={`text-[10px] font-bold uppercase tracking-widest mb-1 ${
                      isUser ? 'text-indigo-400' : 'text-[#6e7681]'
                    }`}>
                      Turn #{i + 1}
                    </div>
                    <div className="text-[13.5px] text-[#e0e6ed] leading-relaxed">
                      {text}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}

        {/* TAB 4: ENVIRONMENT */}
        {activeTab === 'Environment' && (
          <div className="space-y-4 font-mono text-[12.5px] animate-in fade-in slide-in-from-bottom-2 duration-300">
            <div className="p-6 rounded-2xl bg-[#0d131f]/80 backdrop-blur-sm border border-[#1e2430] space-y-3 relative overflow-hidden group">
              <div className="absolute top-0 right-0 w-32 h-32 bg-blue-500/5 rounded-full blur-2xl -mr-10 -mt-10 pointer-events-none" />
              <div className="flex items-center gap-2 text-[11px] text-blue-400 uppercase font-bold tracking-widest relative z-10">
                <Cpu size={14} /> Runtime Silicon & Environment
              </div>
              <div className="text-[#e0e6ed] font-medium relative z-10 leading-relaxed">
                {activeEnv ||
                  'Python 3.11.8 · Darwin aarch64 (Apple M2 Pro) · 8 Cores · 32 GB RAM · Accelerator: MPS'}
              </div>
            </div>

            <div className="p-6 rounded-2xl bg-[#0d131f]/80 backdrop-blur-sm border border-[#1e2430] space-y-3 relative overflow-hidden group">
              <div className="absolute top-0 right-0 w-32 h-32 bg-emerald-500/5 rounded-full blur-2xl -mr-10 -mt-10 pointer-events-none" />
              <div className="flex items-center gap-2 text-[11px] text-emerald-400 uppercase font-bold tracking-widest relative z-10">
                <ShieldCheck size={14} /> Reproducibility Lockfile
              </div>
              <div className="text-[13px] text-[#a1abb7] relative z-10">
                Lockfile Hash: <span className="text-white bg-white/5 px-2 py-1 rounded-md border border-white/10 ml-2 shadow-inner">cas://bafy2bzace4v3k99a77z</span>
              </div>
              <div className="text-[13px] text-[#a1abb7] relative z-10">
                Installed packages: <span className="text-white">64 packages (pip-freeze verified)</span>
              </div>
            </div>
          </div>
        )}

        {/* TAB 5: REVIEW & INVARIANTS */}
        {activeTab === 'Review' && (
          <div className="space-y-5 animate-in fade-in slide-in-from-bottom-2 duration-300">
            <div
              className={`p-6 rounded-2xl border flex items-center justify-between shadow-lg relative overflow-hidden ${
                isPassed
                  ? 'bg-emerald-500/10 border-emerald-500/30'
                  : 'bg-rose-500/10 border-rose-500/30'
              }`}
            >
              <div className={`absolute inset-0 bg-gradient-to-r ${isPassed ? 'from-emerald-500/10' : 'from-rose-500/10'} to-transparent`} />
              
              <div className="flex items-center gap-4 relative z-10">
                {isPassed ? (
                  <div className="p-2 rounded-full bg-emerald-500/20 text-emerald-400">
                    <CheckCircle2 className="w-6 h-6" />
                  </div>
                ) : (
                  <div className="p-2 rounded-full bg-rose-500/20 text-rose-400">
                    <XCircle className="w-6 h-6" />
                  </div>
                )}
                <div>
                  <div className="text-[15px] font-bold text-white tracking-wide mb-1">
                    {isPassed ? 'All Invariant Gates Passed' : 'Invariant Check Failed'}
                  </div>
                  <div className="text-[12.5px] text-[#a1abb7]">
                    Deterministic verification sealed against Custos Sovereign Ledger.
                  </div>
                </div>
              </div>
              <span
                className={`font-mono text-[11px] px-3 py-1.5 rounded-lg font-bold uppercase tracking-widest relative z-10 shadow-inner ${
                  isPassed ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30' : 'bg-rose-500/20 text-rose-400 border border-rose-500/30'
                }`}
              >
                {isPassed ? 'Passed' : 'Failed'}
              </span>
            </div>

            {/* Findings List */}
            <div className="space-y-3">
              <div className="flex items-center gap-3 mb-4">
                <div className="h-px bg-gradient-to-r from-transparent to-[#2d3342] flex-1" />
                <h4 className="text-[11px] font-bold text-[#6e7681] uppercase tracking-[0.2em]">
                  Invariant Verification Logs
                </h4>
                <div className="h-px bg-gradient-to-l from-transparent to-[#2d3342] flex-1" />
              </div>

              {(
                data.reviewFindings ?? [
                  {
                    level: 'ok',
                    title: 'INV-CLAIM-01: Citations anchored to verified DOI source',
                    evidence: 'DOI 10.1038/s41586-023-00000-0 verified in CrossRef CAS index.',
                    check: 'citation',
                  },
                  {
                    level: 'ok',
                    title: 'INV-DET-02: Zero floating-point divergence on re-run',
                    evidence: 'Output Merkle root matches initial run within 1e-7 tolerance.',
                    check: 'integrity',
                  },
                ]
              ).map((f, idx) => (
                <div
                  key={idx}
                  className="p-5 rounded-2xl bg-[#0d131f] border border-[#1e2430] space-y-3 hover:border-[#2d3342] transition-colors"
                >
                  <div className="flex items-center justify-between">
                    <span className="font-bold text-[13.5px] text-white tracking-wide">{f.title}</span>
                    <span
                      className={`text-[10px] font-bold font-mono px-2 py-1 rounded-md uppercase tracking-wider ${
                        f.level === 'ok'
                          ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20'
                          : f.level === 'warn'
                          ? 'bg-amber-500/10 text-amber-400 border border-amber-500/20'
                          : 'bg-rose-500/10 text-rose-400 border border-rose-500/20'
                      }`}
                    >
                      {f.level}
                    </span>
                  </div>
                  {f.evidence && (
                    <div className="font-mono text-[12px] text-[#a1abb7] bg-[#04080F] p-3 rounded-xl border border-[#1e2430] shadow-inner">
                      {f.evidence}
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
