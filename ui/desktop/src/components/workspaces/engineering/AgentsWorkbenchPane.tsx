import React, { useState, useEffect, useCallback } from 'react';
import {
  Bot,
  Play,
  RefreshCw,
  Terminal,
  ShieldAlert,
  ShieldCheck,
  CheckCircle2,
  AlertTriangle,
  FolderGit2,
  Zap,
  Eye,
  Send,
  Copy,
  Check,
} from 'lucide-react';
import { daemonClient } from '@/api/daemon_client';
import {
  HarnessDescriptor,
  ExecutionWorkspace,
  HarnessExecutionResult,
  ToolMediationLevel,
} from '@/types/domain';

export const AgentsWorkbenchPane: React.FC = () => {
  const [harnesses, setHarnesses] = useState<HarnessDescriptor[]>([]);
  const [loadingHarnesses, setLoadingHarnesses] = useState<boolean>(true);
  const [selectedHarnessId, setSelectedHarnessId] = useState<string>('claude-code');

  const [workspaces, setWorkspaces] = useState<ExecutionWorkspace[]>([]);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string>('');

  const [instruction, setInstruction] = useState<string>('');
  const [running, setRunning] = useState<boolean>(false);
  const [runResult, setRunResult] = useState<HarnessExecutionResult | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [copiedStdout, setCopiedStdout] = useState<boolean>(false);

  // Steer guidance input
  const [steerGuidance, setSteerGuidance] = useState<string>('');
  const [steering, setSteering] = useState<boolean>(false);

  const fetchHarnesses = useCallback(async () => {
    try {
      setLoadingHarnesses(true);
      setErrorMessage(null);
      const list = await daemonClient.listHarnesses();
      setHarnesses(list);
      if (list.length > 0 && !list.some((h) => h.id === selectedHarnessId)) {
        setSelectedHarnessId(list[0].id);
      }
    } catch (err: any) {
      setErrorMessage(err?.message || 'Failed to list agent harnesses from daemon');
    } finally {
      setLoadingHarnesses(false);
    }
  }, [selectedHarnessId]);

  const fetchWorkspaces = useCallback(async () => {
    try {
      const list = await daemonClient.listWorkspaces();
      const active = list.filter((w) => w.status !== 'archived');
      setWorkspaces(active);
      if (active.length > 0 && !selectedWorkspaceId) {
        setSelectedWorkspaceId(active[0].id);
      }
    } catch {
      // workspace optional
    }
  }, [selectedWorkspaceId]);

  useEffect(() => {
    fetchHarnesses();
    fetchWorkspaces();
  }, [fetchHarnesses, fetchWorkspaces]);

  const handleRun = async () => {
    if (!instruction.trim()) {
      setErrorMessage('Please provide an instruction for the native harness run');
      return;
    }

    setRunning(true);
    setErrorMessage(null);
    setStatusMessage(`Executing native instruction with '${selectedHarnessId}'...`);
    setRunResult(null);

    try {
      const result = await daemonClient.runNativeHarness({
        harnessId: selectedHarnessId,
        instruction: instruction.trim(),
        workspaceId: selectedWorkspaceId || undefined,
      });
      setRunResult(result);
      setStatusMessage(
        result.success
          ? `Run finished successfully (exit code ${result.exit_code ?? 0})`
          : `Run failed (exit code ${result.exit_code ?? 'non-zero'})`
      );
    } catch (err: any) {
      setErrorMessage(err?.message || 'Execution error encountered');
      setStatusMessage(null);
    } finally {
      setRunning(false);
    }
  };

  const handleSteer = async () => {
    if (!steerGuidance.trim()) return;
    setSteering(true);
    try {
      await daemonClient.steerHarness(selectedHarnessId, 'active_run', steerGuidance.trim());
      setStatusMessage('Guidance injected into harness session');
      setSteerGuidance('');
    } catch (err: any) {
      setErrorMessage(err?.message || 'Failed to steer harness');
    } finally {
      setSteering(false);
    }
  };

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedStdout(true);
    setTimeout(() => setCopiedStdout(false), 2000);
  };

  const selectedHarness = harnesses.find((h) => h.id === selectedHarnessId);

  const renderMediationBadge = (tier: ToolMediationLevel | string) => {
    switch (tier) {
      case 'custos_mediated':
        return (
          <span className="inline-flex items-center gap-1.5 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-[10px] font-medium text-emerald-400">
            <ShieldCheck className="h-3 w-3" />
            Custos Mediated (Permit Enforced)
          </span>
        );
      case 'provider_governed':
        return (
          <span className="inline-flex items-center gap-1.5 rounded-full border border-amber-500/30 bg-amber-500/10 px-2 py-0.5 text-[10px] font-medium text-amber-400">
            <ShieldAlert className="h-3 w-3" />
            Provider Governed (Gate 4 Bypass)
          </span>
        );
      case 'observe_only':
      default:
        return (
          <span className="inline-flex items-center gap-1.5 rounded-full border border-sky-500/30 bg-sky-500/10 px-2 py-0.5 text-[10px] font-medium text-sky-400">
            <Eye className="h-3 w-3" />
            Observe Only
          </span>
        );
    }
  };

  return (
    <div className="flex h-full flex-col overflow-hidden bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
      {/* Top Header */}
      <div className="flex items-center justify-between border-b border-[var(--color-border-default)] px-6 py-4">
        <div>
          <div className="flex items-center gap-2">
            <span className="workbench-kicker">Native Coding Runtimes</span>
            <span className="rounded bg-[var(--color-surface-2)] px-1.5 py-0.5 text-[10px] font-semibold text-[var(--color-fg-muted)]">
              Gate 4 Enforced
            </span>
          </div>
          <h1 className="text-base font-semibold tracking-[-0.01em] text-[var(--color-fg-default)]">
            Coding Agents & Native Harnesses
          </h1>
          <p className="mt-0.5 text-xs text-[var(--color-fg-muted)]">
            Unified execution path for Claude Code CLI, OpenAI Codex, and Block Goose under Custos Task & Assurance kernel.
          </p>
        </div>
        <button
          type="button"
          onClick={fetchHarnesses}
          disabled={loadingHarnesses}
          className="flex items-center gap-1.5 rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-3 py-1.5 text-xs font-medium text-[var(--color-fg-muted)] transition hover:bg-[var(--color-surface-2)] hover:text-[var(--color-fg-default)] disabled:opacity-50"
        >
          <RefreshCw className={`h-3.5 w-3.5 ${loadingHarnesses ? 'animate-spin' : ''}`} />
          Refresh Fleet
        </button>
      </div>

      {/* Main Grid: Left Fleet List, Right Run & Inspector */}
      <div className="flex min-h-0 flex-1 overflow-hidden">
        {/* Left: Harness Fleet List */}
        <div className="w-80 flex-shrink-0 overflow-y-auto border-r border-[var(--color-border-default)] p-4">
          <div className="mb-3 text-[11px] font-semibold uppercase tracking-wider text-[var(--color-fg-muted)]">
            Registered Harnesses ({harnesses.length})
          </div>

          <div className="space-y-2.5">
            {harnesses.map((harness) => {
              const isSelected = harness.id === selectedHarnessId;
              const isAvailable = harness.is_available ?? (harness as any).isAvailable;
              return (
                <div
                  key={harness.id}
                  onClick={() => setSelectedHarnessId(harness.id)}
                  className={`group relative cursor-pointer rounded-xl border p-3.5 transition ${
                    isSelected
                      ? 'border-[var(--color-border-focus)] bg-[var(--color-surface-2)] shadow-sm'
                      : 'border-[var(--color-border-muted)] bg-[var(--color-surface-1)] hover:border-[var(--color-border-default)] hover:bg-[var(--color-surface-2)]'
                  }`}
                >
                  <div className="flex items-start justify-between">
                    <div className="flex items-center gap-2">
                      <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-[var(--color-surface-3)] text-[var(--color-fg-muted)]">
                        <Bot className="h-4 w-4" />
                      </div>
                      <div>
                        <div className="text-xs font-semibold text-[var(--color-fg-default)]">
                          {harness.name}
                        </div>
                        <div className="font-mono text-[10px] text-[var(--color-fg-muted)]">
                          {harness.id}
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-1">
                      <div
                        className={`h-2 w-2 rounded-full ${
                          isAvailable ? 'bg-emerald-400' : 'bg-amber-400'
                        }`}
                        title={isAvailable ? 'Binary available on PATH' : 'Binary missing'}
                      />
                    </div>
                  </div>

                  <p className="mt-2 text-[11px] leading-relaxed text-[var(--color-fg-muted)]">
                    {harness.description}
                  </p>

                  <div className="mt-3 flex flex-wrap items-center gap-1.5">
                    {renderMediationBadge(harness.profile.tool_mediation)}
                  </div>

                  <div className="mt-2.5 grid grid-cols-2 gap-1 rounded bg-[var(--color-canvas)] p-1.5 text-[10px] text-[var(--color-fg-muted)]">
                    <div>
                      <span className="text-[var(--color-fg-subtle)]">Worktree: </span>
                      {harness.profile.worktree_ownership}
                    </div>
                    <div>
                      <span className="text-[var(--color-fg-subtle)]">Cost: </span>
                      {harness.profile.cost_visibility}
                    </div>
                    <div>
                      <span className="text-[var(--color-fg-subtle)]">Cancel: </span>
                      {harness.profile.supports_cancel ? 'Yes' : 'No'}
                    </div>
                    <div>
                      <span className="text-[var(--color-fg-subtle)]">Steer: </span>
                      {harness.profile.supports_steer ? 'Yes' : 'No'}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Right: Interactive Run Console & Observability */}
        <div className="flex min-w-0 flex-1 flex-col overflow-y-auto p-6">
          {selectedHarness && (
            <div className="mb-6 rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-5">
              <div className="flex items-center justify-between border-b border-[var(--color-border-muted)] pb-3">
                <div className="flex items-center gap-3">
                  <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-[var(--color-surface-2)]">
                    <Zap className="h-5 w-5 text-amber-400" />
                  </div>
                  <div>
                    <h2 className="text-sm font-semibold text-[var(--color-fg-default)]">
                      {selectedHarness.name} Console
                    </h2>
                    <div className="text-xs text-[var(--color-fg-muted)]">
                      Binary: <code className="rounded bg-[var(--color-canvas)] px-1 py-0.5 font-mono text-[11px]">{selectedHarness.binary_path}</code> • Assurance Tier:{' '}
                      <span className="font-medium text-[var(--color-fg-default)]">{selectedHarness.profile.tool_mediation}</span>
                    </div>
                  </div>
                </div>
                {renderMediationBadge(selectedHarness.profile.tool_mediation)}
              </div>

              {/* Run Form */}
              <div className="mt-4 space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--color-fg-muted)]">
                      Target Execution Workspace
                    </label>
                    <div className="relative">
                      <select
                        value={selectedWorkspaceId}
                        onChange={(e) => setSelectedWorkspaceId(e.target.value)}
                        className="w-full appearance-none rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3 py-2 pr-8 text-xs text-[var(--color-fg-default)] focus:border-[var(--color-border-focus)] focus:outline-none"
                      >
                        <option value="">Current Daemon Directory (Default)</option>
                        {workspaces.map((ws) => (
                          <option key={ws.id} value={ws.id}>
                            {ws.name} ({ws.path})
                          </option>
                        ))}
                      </select>
                      <FolderGit2 className="pointer-events-none absolute right-2.5 top-2.5 h-3.5 w-3.5 text-[var(--color-fg-muted)]" />
                    </div>
                  </div>

                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--color-fg-muted)]">
                      Active Harness Binary
                    </label>
                    <div className="flex items-center gap-2 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3 py-2 text-xs">
                      <Terminal className="h-3.5 w-3.5 text-[var(--color-fg-muted)]" />
                      <span className="font-mono text-[11px] text-[var(--color-fg-default)]">
                        {selectedHarness.binary_path}
                      </span>
                      <span className="ml-auto text-[10px] text-[var(--color-fg-muted)]">
                        {selectedHarness.is_available ? 'Ready' : 'Not installed'}
                      </span>
                    </div>
                  </div>
                </div>

                <div>
                  <label className="mb-1 block text-xs font-medium text-[var(--color-fg-muted)]">
                    Instruction / Agent Goal Prompt
                  </label>
                  <textarea
                    rows={3}
                    value={instruction}
                    onChange={(e) => setInstruction(e.target.value)}
                    placeholder="e.g. Inspect the current workspace and propose test suite improvements under Gate 4 assurance..."
                    className="w-full rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] p-3 font-mono text-xs text-[var(--color-fg-default)] placeholder:text-[var(--color-fg-subtle)] focus:border-[var(--color-border-focus)] focus:outline-none"
                  />
                </div>

                <div className="flex items-center justify-between pt-1">
                  <div className="text-[11px] text-[var(--color-fg-muted)]">
                    Gate 4 Invariant: Native mutations will be tagged as{' '}
                    <strong className="text-amber-400">provider-governed</strong> without fake assurance.
                  </div>
                  <button
                    type="button"
                    onClick={handleRun}
                    disabled={running || !instruction.trim()}
                    className="flex items-center gap-2 rounded-lg bg-[var(--color-btn-primary-bg)] px-4 py-2 text-xs font-medium text-[var(--color-btn-primary-fg)] transition hover:opacity-90 disabled:opacity-50"
                  >
                    {running ? (
                      <>
                        <RefreshCw className="h-3.5 w-3.5 animate-spin" />
                        Running...
                      </>
                    ) : (
                      <>
                        <Play className="h-3.5 w-3.5 fill-current" />
                        Dispatch Native Run
                      </>
                    )}
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* Error Banner */}
          {errorMessage && (
            <div className="mb-4 flex items-center gap-2 rounded-xl border border-rose-500/30 bg-rose-500/10 p-3 text-xs text-rose-400">
              <AlertTriangle className="h-4 w-4 flex-shrink-0" />
              <span>{errorMessage}</span>
            </div>
          )}

          {/* Status Banner */}
          {statusMessage && (
            <div className="mb-4 flex items-center gap-2 rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-3 text-xs text-[var(--color-fg-default)]">
              <CheckCircle2 className="h-4 w-4 flex-shrink-0 text-emerald-400" />
              <span>{statusMessage}</span>
            </div>
          )}

          {/* Live Run Output & Observed Effects */}
          {runResult && (
            <div className="space-y-4">
              {/* Output terminal */}
              <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-canvas)] p-4 shadow-sm">
                <div className="flex items-center justify-between border-b border-[var(--color-border-muted)] pb-2.5">
                  <div className="flex items-center gap-2">
                    <Terminal className="h-4 w-4 text-[var(--color-fg-muted)]" />
                    <span className="text-xs font-semibold text-[var(--color-fg-default)]">
                      Harness Execution Output
                    </span>
                    <span
                      className={`rounded px-1.5 py-0.5 text-[10px] font-semibold ${
                        runResult.success
                          ? 'bg-emerald-500/10 text-emerald-400'
                          : 'bg-rose-500/10 text-rose-400'
                      }`}
                    >
                      Exit Code: {runResult.exit_code ?? 0}
                    </span>
                  </div>
                  <button
                    type="button"
                    onClick={() => handleCopy(runResult.stdout || runResult.stderr)}
                    className="flex items-center gap-1 rounded bg-[var(--color-surface-1)] px-2 py-1 text-[11px] text-[var(--color-fg-muted)] hover:text-[var(--color-fg-default)]"
                  >
                    {copiedStdout ? <Check className="h-3 w-3 text-emerald-400" /> : <Copy className="h-3 w-3" />}
                    {copiedStdout ? 'Copied' : 'Copy Output'}
                  </button>
                </div>
                <pre className="mt-3 max-h-64 overflow-y-auto font-mono text-[11px] leading-relaxed text-[var(--color-fg-default)]">
                  {runResult.stdout || runResult.stderr || '(No output produced)'}
                </pre>
              </div>

              {/* Observed Effects Under Gate 4 */}
              <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-4">
                <div className="mb-3 flex items-center justify-between">
                  <div>
                    <h3 className="text-xs font-semibold text-[var(--color-fg-default)]">
                      Observed Action Intents ({runResult.observed_effects?.length || 0})
                    </h3>
                    <p className="text-[11px] text-[var(--color-fg-muted)]">
                      Normalized actions emitted during execution, tagged with honest Gate 4 assurance.
                    </p>
                  </div>
                  <span className="rounded bg-[var(--color-surface-2)] px-2 py-0.5 text-[10px] font-medium text-[var(--color-fg-muted)]">
                    Mediation: {runResult.mediation_level}
                  </span>
                </div>

                {runResult.observed_effects && runResult.observed_effects.length > 0 ? (
                  <div className="space-y-2">
                    {runResult.observed_effects.map((intent, idx) => (
                      <div
                        key={intent.id || idx}
                        className="flex items-center justify-between rounded-lg border border-[var(--color-border-muted)] bg-[var(--color-canvas)] p-2.5 text-xs"
                      >
                        <div className="flex items-center gap-2">
                          <code className="rounded bg-[var(--color-surface-2)] px-1.5 py-0.5 font-mono text-[10px] font-bold text-sky-400">
                            {intent.name}
                          </code>
                          <span className="font-mono text-[11px] text-[var(--color-fg-default)]">
                            {intent.target}
                          </span>
                        </div>
                        <div className="flex items-center gap-2">
                          <span className="text-[10px] text-[var(--color-fg-muted)]">
                            Risk: {intent.risk_level || (intent as any).riskLevel || 'Medium'}
                          </span>
                          <span className="rounded-full bg-amber-500/10 px-2 py-0.5 text-[9px] font-semibold uppercase text-amber-400">
                            {intent.assurance}
                          </span>
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="rounded-lg border border-dashed border-[var(--color-border-muted)] p-4 text-center text-xs text-[var(--color-fg-muted)]">
                    No discrete tool calls or mutations observed for this run.
                  </div>
                )}
              </div>

              {/* Dynamic Steering Interface */}
              {selectedHarness?.profile.supports_steer && (
                <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-4">
                  <div className="mb-2 text-xs font-semibold text-[var(--color-fg-default)]">
                    Inject Run Steering Guidance
                  </div>
                  <div className="flex items-center gap-2">
                    <input
                      type="text"
                      value={steerGuidance}
                      onChange={(e) => setSteerGuidance(e.target.value)}
                      placeholder="e.g. Focus on edge cases in parse_turn..."
                      className="flex-1 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3 py-1.5 text-xs text-[var(--color-fg-default)] focus:border-[var(--color-border-focus)] focus:outline-none"
                    />
                    <button
                      type="button"
                      onClick={handleSteer}
                      disabled={steering || !steerGuidance.trim()}
                      className="flex items-center gap-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3 py-1.5 text-xs font-medium text-[var(--color-fg-default)] hover:bg-[var(--color-surface-3)] disabled:opacity-50"
                    >
                      <Send className="h-3 w-3" />
                      Steer
                    </button>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
