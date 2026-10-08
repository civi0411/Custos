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
  GitCompare,
} from 'lucide-react';
import { ArtifactInspectorData, ArtifactTab } from '@/types/research';

const TABS: ArtifactTab[] = ['Code', 'Execution Log', 'Messages', 'Environment', 'Review'];

interface ArtifactInspectorProps {
  data: ArtifactInspectorData;
  onClose?: () => void;
  isMaximized?: boolean;
  onToggleMaximize?: () => void;
}

function computeSimpleLineDiff(oldText: string, newText: string) {
  const oldLines = oldText.split('\n');
  const newLines = newText.split('\n');
  const result: { type: 'same' | 'added' | 'removed'; text: string; lineNum?: number }[] = [];
  let i = 0;
  let j = 0;

  while (i < oldLines.length || j < newLines.length) {
    if (i < oldLines.length && j < newLines.length && oldLines[i] === newLines[j]) {
      result.push({ type: 'same', text: oldLines[i] });
      i++;
      j++;
    } else if (j < newLines.length && (i >= oldLines.length || !oldLines.slice(i, i + 5).includes(newLines[j]))) {
      result.push({ type: 'added', text: newLines[j] });
      j++;
    } else if (i < oldLines.length) {
      result.push({ type: 'removed', text: oldLines[i] });
      i++;
    } else {
      result.push({ type: 'added', text: newLines[j] });
      j++;
    }
  }
  return result;
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
  const [compareIdx, setCompareIdx] = useState<number>(() => Math.max(0, versionIdx - 1));
  const [diffMode, setDiffMode] = useState<boolean>(false);
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
  const reviewStatus = currentVersion.reviewPassed ?? data.reviewPassed;
  const isPassed = reviewStatus === true;

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
    <div className="flex flex-col h-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] overflow-hidden select-none font-sans relative">
      {/* 48px Standard Pane Header */}
      <header className="flex h-12 shrink-0 items-center justify-between border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-5 z-10">
        <div className="flex items-center gap-3 min-w-0">
          <FileCode2 className="w-4 h-4 workbench-accent shrink-0" />
          <span className="truncate text-xs font-semibold text-[var(--color-editor-fg)]">
            {data.title || data.filename}
          </span>

          {/* Version Stepper */}
          <div className="flex items-center gap-1 ml-3 bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] rounded-lg px-1.5 py-0.5">
            <button
              onClick={() => setVersionIdx((i) => Math.max(0, i - 1))}
              disabled={versionIdx <= 0}
              className="p-1 text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] disabled:opacity-30 transition rounded"
              title="Previous version"
            >
              <ChevronLeft size={12} />
            </button>
            <span className="font-mono text-[10px] text-[var(--color-editor-fg)] px-1.5 font-bold uppercase tracking-wider">
              {currentVersion.label}
            </span>
            <button
              onClick={() => setVersionIdx((i) => Math.min(data.versions.length - 1, i + 1))}
              disabled={versionIdx >= data.versions.length - 1}
              className="p-1 text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] disabled:opacity-30 transition rounded"
              title="Next version"
            >
              <ChevronRight size={12} />
            </button>
          </div>

          {/* Diff Mode Toggle */}
          {data.versions.length > 1 && (
            <div className="flex items-center gap-1.5 ml-2">
              <button
                onClick={() => setDiffMode(!diffMode)}
                className={`flex items-center gap-1 px-2.5 py-1 rounded-lg text-[11px] font-medium border transition ${
                  diffMode
                    ? 'bg-purple-500/10 text-purple-400 border-purple-500/30'
                    : 'bg-[var(--color-surface-2)] text-[var(--color-fg-muted)] border-[var(--color-border-muted)] hover:text-[var(--color-editor-fg)]'
                }`}
                title="Toggle Diff comparison mode"
              >
                <GitCompare size={12} />
                Diff
              </button>

              {diffMode && (
                <div className="flex items-center gap-1 text-[11px] font-mono text-[var(--color-fg-muted)] bg-[var(--color-surface-2)] px-2 py-0.5 rounded-lg border border-[var(--color-border-muted)]">
                  <span>vs</span>
                  <select
                    value={compareIdx}
                    onChange={(e) => setCompareIdx(Number(e.target.value))}
                    className="bg-transparent text-[var(--color-editor-fg)] font-semibold focus:outline-none cursor-pointer"
                  >
                    {data.versions.map((v, i) => (
                      <option key={v.label} value={i} disabled={i === versionIdx} className="bg-[var(--color-surface-1)] text-[var(--color-editor-fg)]">
                        {v.label}
                      </option>
                    ))}
                  </select>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Right Header Controls */}
        <div className="flex items-center gap-1.5 shrink-0">
          <button
            onClick={handleCopy}
            className="p-1.5 rounded-lg text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
            title="Copy Code"
          >
            {copied ? <Check size={14} className="text-emerald-500" /> : <Copy size={14} />}
          </button>
          <button
            onClick={handleDownload}
            className="p-1.5 rounded-lg text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
            title="Download Script"
          >
            <Download size={14} />
          </button>
          {onToggleMaximize && (
            <button
              onClick={onToggleMaximize}
              className="p-1.5 rounded-lg text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-2)] transition"
              title={isMaximized ? 'Restore Pane' : 'Maximize Pane'}
            >
              {isMaximized ? <Minimize2 size={14} /> : <Maximize2 size={14} />}
            </button>
          )}
          {onClose && (
            <div className="w-px h-4 bg-[var(--color-border-muted)] mx-1" />
          )}
          {onClose && (
            <button
              onClick={onClose}
              className="p-1.5 rounded-lg text-[var(--color-fg-muted)] hover:text-rose-500 hover:bg-[var(--color-surface-2)] transition"
              title="Close Inspector"
            >
              <X size={14} />
            </button>
          )}
        </div>
      </header>

      {/* 5 Tabs Navigation Bar */}
      <nav className="flex items-center gap-1 border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-5 pt-2 z-10">
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
              className={`group flex items-center gap-2 pb-2.5 px-3 text-xs font-medium border-b-2 transition relative ${
                isActive
                  ? 'border-[var(--workbench-accent)] text-[var(--color-editor-fg)]'
                  : 'border-transparent text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              <Icon size={13} className={isActive ? 'workbench-accent' : 'text-[var(--color-fg-subtle)] group-hover:text-[var(--color-editor-fg)]'} />
              <span>{tab}</span>
              {tab === 'Review' && (
                <span
                  className={`w-1.5 h-1.5 rounded-full ml-1 ${
                    isPassed ? 'bg-emerald-500' : 'bg-rose-500'
                  }`}
                />
              )}
            </button>
          );
        })}
      </nav>

      {/* Tab Contents Viewport */}
      <div className="flex-1 overflow-y-auto p-5 select-text max-w-5xl mx-auto w-full">
        {/* TAB 1: CODE */}
        {activeTab === 'Code' && (
          <div className="space-y-4">
            {data.inputs && data.inputs.length > 0 && (
              <div className="flex flex-wrap items-center gap-2 text-xs bg-[var(--color-surface-1)] p-3 rounded-xl border border-[var(--color-border-muted)]">
                <span className="text-[var(--color-fg-subtle)] font-semibold uppercase tracking-wider text-[10px]">Inputs</span>
                {data.inputs.map((inp) => (
                  <span
                    key={inp}
                    className="font-mono text-xs px-2 py-0.5 rounded bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] text-[var(--color-fg-muted)]"
                  >
                    {inp}
                  </span>
                ))}
              </div>
            )}
            {diffMode ? (
              <div className="rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-4 font-mono text-xs leading-relaxed overflow-x-auto">
                <div className="flex items-center justify-between pb-2.5 border-b border-[var(--color-border-muted)] mb-3 text-xs text-[var(--color-fg-muted)]">
                  <span className="font-semibold text-[var(--color-editor-fg)]">
                    Comparing {currentVersion.label} with {data.versions[compareIdx]?.label ?? 'v1'}
                  </span>
                  <div className="flex items-center gap-3">
                    <span className="text-emerald-500 font-medium">+ Additions</span>
                    <span className="text-rose-500 font-medium">- Deletions</span>
                  </div>
                </div>
                <div className="space-y-0.5">
                  {computeSimpleLineDiff(data.versions[compareIdx]?.code ?? '', activeCode).map((line, idx) => (
                    <div
                      key={idx}
                      className={`px-2 py-0.5 rounded flex items-start gap-3 ${
                        line.type === 'added'
                          ? 'bg-emerald-500/10 text-emerald-500 font-medium'
                          : line.type === 'removed'
                          ? 'bg-rose-500/10 text-rose-500 line-through opacity-80'
                          : 'text-[var(--color-fg-muted)]'
                      }`}
                    >
                      <span className="w-4 select-none opacity-50 text-center font-bold">
                        {line.type === 'added' ? '+' : line.type === 'removed' ? '-' : ' '}
                      </span>
                      <pre className="font-mono text-xs flex-1 whitespace-pre-wrap">{line.text || ' '}</pre>
                    </div>
                  ))}
                </div>
              </div>
            ) : (
              <div className="rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-canvas-inset)] p-4 font-mono text-xs leading-relaxed overflow-x-auto text-[var(--color-editor-fg)]">
                <pre>
                  <code>{activeCode}</code>
                </pre>
              </div>
            )}
          </div>
        )}

        {/* TAB 2: EXECUTION LOG */}
        {activeTab === 'Execution Log' && (
          <div className="h-full">
            <div className="rounded-xl border border-[var(--color-border-muted)] bg-[var(--color-canvas-inset)] font-mono text-xs leading-relaxed overflow-hidden h-full flex flex-col">
              <div className="bg-[var(--color-surface-2)] px-4 py-2 border-b border-[var(--color-border-muted)] text-[10px] text-[var(--color-fg-muted)] font-semibold uppercase tracking-wider flex items-center gap-2">
                <Terminal size={13} />
                Captured Terminal Log (SHA-256 CAS)
              </div>
              <div className="p-4 overflow-auto flex-1 text-[var(--color-fg-muted)]">
                <pre className="whitespace-pre-wrap">{activeLog || 'No execution log captured.'}</pre>
              </div>
            </div>
          </div>
        )}

        {/* TAB 3: MESSAGES */}
        {activeTab === 'Messages' && (
          <div className="space-y-3 max-w-3xl mx-auto">
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
                  className={`p-4 rounded-xl border flex gap-3 ${
                    isUser ? 'bg-[var(--color-surface-1)] border-[var(--color-border-default)]' : 'bg-[var(--color-surface-0)] border-[var(--color-border-muted)]'
                  }`}
                >
                  <div className={`w-7 h-7 rounded-full flex items-center justify-center font-bold text-[10px] shrink-0 ${
                    isUser ? 'bg-[var(--color-surface-3)] text-[var(--color-editor-fg)]' : 'bg-[var(--color-surface-2)] text-[var(--color-fg-muted)]'
                  }`}>
                    {isUser ? 'ME' : 'AI'}
                  </div>
                  <div>
                    <div className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-fg-subtle)] mb-1">
                      Turn #{i + 1}
                    </div>
                    <div className="text-xs text-[var(--color-editor-fg)] leading-relaxed">
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
          <div className="space-y-3 font-mono text-xs">
            <div className="p-4 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] space-y-2">
              <div className="flex items-center gap-2 text-[10px] text-[var(--color-fg-subtle)] uppercase font-semibold tracking-wider">
                <Cpu size={13} /> Runtime Silicon & Environment
              </div>
              <div className="text-[var(--color-editor-fg)] leading-relaxed font-sans text-xs">
                {activeEnv || 'Environment receipt unavailable'}
              </div>
            </div>

            <div className="p-4 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] space-y-2">
              <div className="flex items-center gap-2 text-[10px] text-[var(--color-fg-subtle)] uppercase font-semibold tracking-wider">
                <ShieldCheck size={13} /> Reproducibility Lockfile
              </div>
              <div className="text-xs text-[var(--color-fg-muted)] font-sans">
                No lockfile receipt is attached to this artifact version.
              </div>
            </div>
          </div>
        )}

        {/* TAB 5: REVIEW & INVARIANTS */}
        {activeTab === 'Review' && (
          <div className="space-y-4">
            <div
              className={`p-4 rounded-xl border flex items-center justify-between ${
                reviewStatus == null
                  ? 'bg-[var(--color-surface-1)] border-[var(--color-border-muted)]'
                  : isPassed
                    ? 'bg-emerald-500/5 border-emerald-500/30'
                    : 'bg-rose-500/5 border-rose-500/30'
              }`}
            >
              <div className="flex items-center gap-3">
                {isPassed ? (
                  <div className="p-1.5 rounded-full bg-emerald-500/10 text-emerald-500">
                    <CheckCircle2 className="w-5 h-5" />
                  </div>
                ) : (
                  <div className="p-1.5 rounded-full bg-rose-500/10 text-rose-500">
                    <XCircle className="w-5 h-5" />
                  </div>
                )}
                <div>
                  <div className="text-xs font-semibold text-[var(--color-editor-fg)]">
                    {reviewStatus == null ? 'No Review Receipt' : isPassed ? 'All Invariant Gates Passed' : 'Invariant Check Failed'}
                  </div>
                  <div className="text-[11px] text-[var(--color-fg-muted)] mt-0.5">
                    {reviewStatus == null ? 'Attach a versioned verifier result before treating this artifact as accepted.' : 'Status reported by the attached artifact review record.'}
                  </div>
                </div>
              </div>
              <span
                className={`font-mono text-[10px] px-2.5 py-1 rounded-md font-semibold uppercase tracking-wider ${
                  isPassed ? 'bg-emerald-500/10 text-emerald-500 border border-emerald-500/30' : 'bg-rose-500/10 text-rose-500 border border-rose-500/30'
                }`}
              >
                {reviewStatus == null ? 'Unknown' : isPassed ? 'Passed' : 'Failed'}
              </span>
            </div>

            {/* Findings List */}
            <div className="space-y-2.5">
              <div className="flex items-center gap-3">
                <div className="h-px bg-[var(--color-border-muted)] flex-1" />
                <h4 className="text-[10px] font-semibold text-[var(--color-fg-subtle)] uppercase tracking-wider">
                  Invariant Verification Logs
                </h4>
                <div className="h-px bg-[var(--color-border-muted)] flex-1" />
              </div>

              {(data.reviewFindings ?? []).map((f, idx) => (
                <div
                  key={idx}
                  className="p-4 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border-muted)] space-y-2"
                >
                  <div className="flex items-center justify-between">
                    <span className="font-medium text-xs text-[var(--color-editor-fg)]">{f.title}</span>
                    <span
                      className={`text-[10px] font-semibold font-mono px-2 py-0.5 rounded uppercase ${
                        f.level === 'ok'
                          ? 'bg-emerald-500/10 text-emerald-500 border border-emerald-500/20'
                          : f.level === 'warn'
                          ? 'bg-amber-500/10 text-amber-500 border border-amber-500/20'
                          : 'bg-rose-500/10 text-rose-500 border border-rose-500/20'
                      }`}
                    >
                      {f.level}
                    </span>
                  </div>
                  {f.evidence && (
                    <div className="font-mono text-xs text-[var(--color-fg-muted)] bg-[var(--color-surface-2)] p-2.5 rounded-lg border border-[var(--color-border-muted)]">
                      {f.evidence}
                    </div>
                  )}
                </div>
              ))}
              {!data.reviewFindings?.length && (
                <div className="rounded-xl border border-[var(--color-border-muted)] p-5 text-center text-xs text-[var(--color-fg-muted)]">
                  No verifier findings are attached.
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
