import { NativeSelect } from '@/components/common/NativeSelect';
import React, { useState, useEffect, useCallback } from 'react';
import {
  Server,
  Plus,
  Play,
  Terminal,
  Activity,
  X,
  Layers,
  RefreshCw,
  Clock,
} from 'lucide-react';
import {
  RemoteHostNode,
  FleetExecReceipt,
  HeadlessAutomationJob,
  RegisterHostParams,
  CreateHeadlessJobParams,
} from '@/types/domain';
import { daemonClient } from '@/api/daemon_client';

interface RemoteFleetWorkbenchPaneProps {
  onShowToast?: (message: string) => void;
  supportedOperations: string[];
  capabilityReason?: string;
}

export const RemoteFleetWorkbenchPane: React.FC<RemoteFleetWorkbenchPaneProps> = ({
  onShowToast,
  supportedOperations,
  capabilityReason,
}) => {
  const can = useCallback(
    (operation: string) => supportedOperations.includes(operation),
    [supportedOperations]
  );
  const [activeTab, setActiveTab] = useState<'nodes' | 'automation'>('nodes');
  const [hosts, setHosts] = useState<RemoteHostNode[]>([]);
  const [selectedHostId, setSelectedHostId] = useState<string | null>(null);
  const [jobs, setJobs] = useState<HeadlessAutomationJob[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [commandInput, setCommandInput] = useState<string>('');
  const [execReceipts, setExecReceipts] = useState<FleetExecReceipt[]>([]);
  const [isExecuting, setIsExecuting] = useState<boolean>(false);
  const [isPinging, setIsPinging] = useState<boolean>(false);

  // Modals
  const [isRegisterModalOpen, setIsRegisterModalOpen] = useState<boolean>(false);
  const [isCreateJobModalOpen, setIsCreateJobModalOpen] = useState<boolean>(false);

  // New Host Form
  const [newHostName, setNewHostName] = useState<string>('');
  const [newHostAddress, setNewHostAddress] = useState<string>('');
  const [newHostPort, setNewHostPort] = useState<number>(22);
  const [newHostUser, setNewHostUser] = useState<string>('');
  const [newHostKeyPath, setNewHostKeyPath] = useState<string>('');

  // New Job Form
  const [newJobName, setNewJobName] = useState<string>('');
  const [newJobCommand, setNewJobCommand] = useState<string>('cargo check --workspace');
  const [newJobTargetType, setNewJobTargetType] = useState<string>('local_process');

  const selectedHost = hosts.find((h) => h.id === selectedHostId) ?? hosts[0] ?? null;

  const canExec = can('fleet.exec') || can('exec');
  const canPing = can('hosts.ping');

  const loadHosts = useCallback(async () => {
    if (!can('hosts.list')) {
      setHosts([]);
      setIsLoading(false);
      return;
    }
    setIsLoading(true);
    try {
      const list = await daemonClient.listRemoteHosts();
      setHosts(list);
      if (list.length > 0 && !selectedHostId) {
        setSelectedHostId(list[0].id);
      }
    } catch (err) {
      onShowToast?.(`Failed to load remote hosts: ${String(err)}`);
    } finally {
      setIsLoading(false);
    }
  }, [can, selectedHostId, onShowToast]);

  const loadJobs = useCallback(async () => {
    if (!can('jobs.list')) {
      setJobs([]);
      return;
    }
    try {
      const jobList = await daemonClient.listHeadlessJobs();
      setJobs(jobList);
    } catch (err) {
      onShowToast?.(`Failed to load automation jobs: ${String(err)}`);
    }
  }, [can, onShowToast]);

  useEffect(() => {
    loadHosts();
    loadJobs();
  }, [loadHosts, loadJobs]);

  const handleRegisterHost = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newHostName.trim() || !newHostAddress.trim() || !newHostUser.trim()) {
      onShowToast?.('Please fill in required host fields.');
      return;
    }

    try {
      const params: RegisterHostParams = {
        name: newHostName.trim(),
        host: newHostAddress.trim(),
        port: newHostPort,
        user: newHostUser.trim(),
        private_key_path: newHostKeyPath.trim() || undefined,
        labels: { managed_by: 'custos-daemon' },
      };

      const registered = await daemonClient.registerRemoteHost(params);
      setHosts((prev) => [...prev, registered]);
      setSelectedHostId(registered.id);
      setIsRegisterModalOpen(false);
      setNewHostName('');
      setNewHostAddress('');
      setNewHostUser('');
      setNewHostKeyPath('');
      onShowToast?.(`Host '${registered.name}' registered successfully.`);
    } catch (err) {
      onShowToast?.(`Failed to register host: ${String(err)}`);
    }
  };

  const handlePingHost = async (hostId: string) => {
    setIsPinging(true);
    try {
      const res = await daemonClient.pingRemoteHost(hostId);
      setHosts((prev) =>
        prev.map((h) =>
          h.id === hostId
            ? {
                ...h,
                status: res.status === 'online' ? 'online' : 'offline',
                lastPingMs: res.ping_ms ?? h.lastPingMs,
              }
            : h
        )
      );
      if (res.status === 'online') {
        onShowToast?.(`Node online. Latency: ${res.ping_ms ?? 0}ms.`);
      } else {
        onShowToast?.(`Host ping offline: ${res.error || res.status}`);
      }
    } catch (err) {
      onShowToast?.(`Ping probe failed: ${String(err)}`);
    } finally {
      setIsPinging(false);
    }
  };

  const handleExecCommand = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedHost || !commandInput.trim()) return;
    setIsExecuting(true);
    try {
      const receipt = await daemonClient.execFleet(selectedHost.id, commandInput.trim());
      setExecReceipts((prev) => [receipt, ...prev]);
      onShowToast?.(`Executed on ${selectedHost.name}. Exit code: ${receipt.exit_code ?? 0} (${receipt.duration_ms}ms)`);
    } catch (err) {
      onShowToast?.(`Execution failed: ${String(err)}`);
    } finally {
      setIsExecuting(false);
    }
  };

  const handleCreateJob = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newJobName.trim() || !newJobCommand.trim()) {
      onShowToast?.('Please specify job name and command.');
      return;
    }

    try {
      const params: CreateHeadlessJobParams = {
        name: newJobName.trim(),
        spec: {
          target_type: newJobTargetType,
          command_or_script: newJobCommand.trim(),
          timeout_secs: 600,
          env: {},
          required_evidence: ['EXECUTION-EVIDENCE-01'],
        },
        trigger: { type: 'manual' },
      };
      const job = await daemonClient.createHeadlessJob(params);
      setJobs((prev) => [job, ...prev]);
      setIsCreateJobModalOpen(false);
      setNewJobName('');
      onShowToast?.(`Headless automation job '${job.name}' created.`);
    } catch (err) {
      onShowToast?.(`Failed to create job: ${String(err)}`);
    }
  };

  const handleRunJob = async (jobId: string) => {
    try {
      const updated = await daemonClient.runHeadlessJob(jobId);
      setJobs((prev) => prev.map((j) => (j.id === jobId ? updated : j)));
      onShowToast?.(`Job '${updated.name}' executed successfully.`);
    } catch (err) {
      onShowToast?.(`Job execution failed: ${String(err)}`);
    }
  };

  return (
    <div className="flex flex-col h-full w-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] text-xs overflow-hidden select-none font-sans relative">
      {/* Standard Pane Header */}
      <header className="flex h-11 shrink-0 items-center justify-between border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-4 z-10">
        <div className="flex items-center gap-2">
          <Server className="w-4 h-4 text-white/70" />
          <span className="text-xs font-semibold text-[var(--color-editor-fg)]">
            Remote Fleet
          </span>
          <span className="text-[10px] text-white/50 font-mono">
            {hosts.length} node{hosts.length === 1 ? '' : 's'}
          </span>
        </div>

        <div className="flex items-center gap-2">
          {selectedHost?.status === 'online' ? (
            <div className="flex items-center gap-1.5 px-2 py-0.5 rounded bg-white/10 border border-white/15 text-white text-[10px] font-mono">
              <span className="w-1.5 h-1.5 rounded-full bg-white/90" />
              <span>Online</span>
            </div>
          ) : canPing && canExec ? (
            <div className="flex items-center gap-1.5 px-2 py-0.5 rounded bg-white/5 border border-white/10 text-white/80 text-[10px] font-mono">
              <span className="w-1.5 h-1.5 rounded-full bg-white/60" />
              <span>SSH Ready</span>
            </div>
          ) : (
            <div className="flex items-center gap-1.5 px-2 py-0.5 rounded bg-white/5 border border-white/10 text-white/60 text-[10px] font-mono" title={capabilityReason || 'OpenSSH transport not connected'}>
              <span className="w-1.5 h-1.5 rounded-full bg-white/40" />
              <span>Fail-Closed</span>
            </div>
          )}

          <div className="flex items-center gap-0.5 bg-[var(--color-surface-2)] p-0.5 rounded-lg border border-[var(--color-border-default)]">
            <button
              onClick={() => setActiveTab('nodes')}
              className={`flex items-center gap-1.5 px-2.5 py-1 rounded-md text-[11px] font-medium transition ${
                activeTab === 'nodes'
                  ? 'bg-white/10 text-white shadow-xs font-semibold'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              <Server size={12} />
              <span>Nodes</span>
            </button>
            <button
              onClick={() => setActiveTab('automation')}
              className={`flex items-center gap-1.5 px-2.5 py-1 rounded-md text-[11px] font-medium transition ${
                activeTab === 'automation'
                  ? 'bg-white/10 text-white shadow-xs font-semibold'
                  : 'text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]'
              }`}
            >
              <Layers size={12} />
              <span>Jobs</span>
            </button>
          </div>
        </div>
      </header>

      {/* Main Tab Content */}
      {activeTab === 'nodes' ? (
        <div className="flex-1 flex overflow-hidden">
          {/* Left: Nodes List */}
          <div className="w-72 flex flex-col border-r border-[var(--color-border-muted)] bg-[var(--color-surface-1)]">
            <div className="flex items-center justify-between p-3 border-b border-[var(--color-border-muted)]">
              <span className="text-[11px] font-semibold text-[var(--color-fg-muted)] uppercase tracking-wider">
                Nodes
              </span>
              <button
                disabled={!can('hosts.register')}
                onClick={() => setIsRegisterModalOpen(true)}
                className="flex items-center gap-1 px-2.5 py-1 bg-white text-black hover:bg-white/90 disabled:opacity-40 disabled:cursor-not-allowed rounded-md text-[11px] font-semibold transition shadow-sm"
              >
                <Plus size={12} />
                <span>Add Node</span>
              </button>
            </div>

            <div className="flex-1 overflow-y-auto p-2.5 space-y-1.5">
              {isLoading ? (
                <div className="text-center p-6 text-[var(--color-fg-muted)] flex items-center justify-center gap-2">
                  <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                  <span>Loading nodes...</span>
                </div>
              ) : hosts.length === 0 ? (
                <div className="text-center p-8 space-y-3">
                  <Server size={28} className="text-white/30 mx-auto" />
                  <p className="text-white/50 text-xs">No remote nodes registered</p>
                  <button
                    disabled={!can('hosts.register')}
                    onClick={() => setIsRegisterModalOpen(true)}
                    className="px-3.5 py-1.5 bg-white text-black hover:bg-white/90 font-semibold rounded-lg text-xs transition shadow-sm"
                  >
                    Register Node
                  </button>
                </div>
              ) : (
                hosts.map((host) => {
                  const isSelected = host.id === selectedHostId;
                  const isOnline = host.status === 'online';
                  return (
                    <div
                      key={host.id}
                      onClick={() => setSelectedHostId(host.id)}
                      className={`p-3 rounded-lg border cursor-pointer transition space-y-1.5 ${
                        isSelected
                          ? 'bg-[var(--color-surface-0)] border-[var(--workbench-accent)] text-[var(--color-editor-fg)] shadow-xs'
                          : 'bg-[var(--color-surface-2)]/60 border-[var(--color-border-subtle)] text-[var(--color-fg-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-editor-fg)]'
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <span className="font-semibold text-xs text-[var(--color-editor-fg)]">{host.name}</span>
                        <span
                          className={`px-1.5 py-0.5 rounded text-[10px] font-mono border ${
                            isOnline
                              ? 'bg-white/10 text-white border-white/20'
                              : 'bg-[var(--color-surface-1)] text-[var(--color-fg-subtle)] border-[var(--color-border-muted)]'
                          }`}
                        >
                          {host.status}
                        </span>
                      </div>
                      <div className="flex items-center justify-between text-[10px] text-[var(--color-fg-muted)] font-mono">
                        <span>{host.user}@{host.host}:{host.port}</span>
                        {host.last_ping_ms != null && (
                          <span className="text-white/80 font-medium">{host.last_ping_ms}ms</span>
                        )}
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          </div>

          {/* Right: Node Detail & Execution Console */}
          <div className="flex-1 flex flex-col bg-[var(--color-canvas)] overflow-hidden">
            {!selectedHost ? (
              <div className="flex-1 flex flex-col items-center justify-center text-center p-8 text-white/50 space-y-2">
                <Server size={32} className="text-white/30" />
                <p className="text-xs">Select a node to inspect</p>
              </div>
            ) : (
              <div className="flex-1 flex flex-col overflow-hidden">
                {/* Node Top Banner */}
                <div className="flex items-center justify-between p-3 border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)]">
                  <div className="space-y-1">
                    <div className="flex items-center gap-2">
                      <span className="font-semibold text-[var(--color-editor-fg)] text-xs">
                        {selectedHost.name}
                      </span>
                      <span className="text-[10px] font-mono text-[var(--color-fg-muted)]">
                        ({selectedHost.user}@{selectedHost.host}:{selectedHost.port})
                      </span>
                      <span
                        className={`px-1.5 py-0.2 rounded text-[10px] font-mono border ${
                          selectedHost.status === 'online'
                            ? 'bg-white/10 text-white border-white/20'
                            : 'bg-[var(--color-surface-2)] text-[var(--color-fg-subtle)] border-[var(--color-border-muted)]'
                        }`}
                      >
                        {selectedHost.status}
                      </span>
                    </div>
                    <div className="flex items-center gap-2 text-[10px] text-[var(--color-fg-muted)] font-mono">
                      <span>ID: {selectedHost.id}</span>
                      <span>•</span>
                      <span>Auth: {selectedHost.auth_method.type}</span>
                      {selectedHost.last_ping_ms != null && (
                        <>
                          <span>•</span>
                          <span className="text-white/80">RTT: {selectedHost.last_ping_ms}ms</span>
                        </>
                      )}
                    </div>
                  </div>

                  <button
                    disabled={!canPing || isPinging}
                    onClick={() => handlePingHost(selectedHost.id)}
                    title="Send OpenSSH ping probe"
                    className="flex items-center gap-1.5 px-3 py-1 bg-white/10 hover:bg-white/15 disabled:opacity-40 disabled:cursor-not-allowed border border-white/20 rounded-lg text-white text-xs font-medium transition"
                  >
                    <Activity size={12} className={isPinging ? 'text-white/80 animate-spin' : 'text-white/70'} />
                    <span>{isPinging ? 'Probing...' : 'Ping Probe'}</span>
                  </button>
                </div>

                {/* Command Runner Bar */}
                <form
                  onSubmit={handleExecCommand}
                  className="flex items-center gap-2 p-3 border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)]/60"
                >
                  <Terminal size={14} className="text-white/50 shrink-0" />
                  <input
                    type="text"
                    value={commandInput}
                    onChange={(e) => setCommandInput(e.target.value)}
                    disabled={!canExec || isExecuting}
                    placeholder={canExec ? "Run command on remote host (e.g. nvidia-smi, docker ps)..." : "Remote execution unavailable"}
                    className="flex-1 bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-lg px-3 py-1.5 font-mono text-xs text-[var(--color-editor-fg)] placeholder:text-white/30 focus:border-white/30 focus:outline-none transition disabled:opacity-50 disabled:cursor-not-allowed"
                  />
                  <button
                    type="submit"
                    disabled={!canExec || isExecuting || !commandInput.trim()}
                    className="flex items-center gap-1.5 px-4 py-1.5 bg-white hover:bg-white/90 disabled:opacity-30 disabled:cursor-not-allowed rounded-lg text-black font-semibold text-xs transition shadow-sm"
                  >
                    <Play size={11} className="fill-current" />
                    <span>{isExecuting ? 'Running...' : 'Run'}</span>
                  </button>
                </form>

                {/* Execution Receipts Stream */}
                <div className="flex-1 overflow-y-auto p-4 space-y-3 font-mono text-[11px]">
                  <div className="flex items-center justify-between">
                    <span className="text-[10px] font-sans font-semibold uppercase text-white/50 tracking-wider">
                      History ({execReceipts.length})
                    </span>
                    {execReceipts.length > 0 && (
                      <button
                        onClick={() => setExecReceipts([])}
                        className="text-[10px] font-sans text-white/40 hover:text-white"
                      >
                        Clear
                      </button>
                    )}
                  </div>

                  {execReceipts.length === 0 ? (
                    <div className="p-8 text-center text-white/40 font-sans text-xs">
                      No commands executed yet
                    </div>
                  ) : (
                    execReceipts.map((rcpt) => (
                      <div
                        key={rcpt.execution_id}
                        className="rounded-lg bg-[var(--color-surface-1)] border border-[var(--color-border-default)] p-3 space-y-2 shadow-xs"
                      >
                        <div className="flex items-center justify-between border-b border-[var(--color-border-muted)] pb-1.5 text-[var(--color-fg-muted)]">
                          <div className="flex items-center gap-2">
                            <span className="workbench-accent font-bold">$ {rcpt.command}</span>
                            <span
                              className={`text-[10px] px-1.5 py-0.5 rounded font-mono ${
                                rcpt.exit_code === 0
                                  ? 'bg-white/10 text-white border border-white/20'
                                  : 'bg-red-500/10 text-red-400 border border-red-500/20'
                              }`}
                            >
                              exit {rcpt.exit_code ?? 0}
                            </span>
                          </div>
                          <span className="text-[var(--color-fg-subtle)] text-[10px] flex items-center gap-1">
                            <Clock size={10} />
                            <span>{rcpt.duration_ms}ms</span>
                          </span>
                        </div>
                        <div className="text-[var(--color-editor-fg)] whitespace-pre-wrap text-xs font-mono bg-[var(--color-surface-0)] p-2.5 rounded-md border border-[var(--color-border-muted)] overflow-x-auto">
                          {rcpt.stdout || <span className="text-[var(--color-fg-subtle)] italic">&lt;empty stdout&gt;</span>}
                          {rcpt.stderr && (
                            <span className="text-red-400 block mt-1.5 pt-1.5 border-t border-[var(--color-border-muted)]">
                              {rcpt.stderr}
                            </span>
                          )}
                        </div>
                      </div>
                    ))
                  )}
                </div>
              </div>
            )}
          </div>
        </div>
      ) : (
        /* Automation Tab Content */
        <div className="flex-1 flex flex-col bg-[var(--color-canvas)] overflow-hidden">
          <div className="flex items-center justify-between p-3.5 border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)]">
            <div className="flex items-center gap-2">
              <span className="font-semibold text-[var(--color-editor-fg)] text-xs">Automation Jobs</span>
              <span className="text-[10px] font-mono px-1.5 py-0.5 bg-white/5 border border-white/10 rounded text-white/60">
                {jobs.length}
              </span>
            </div>
            <button
              disabled={!can('jobs.create')}
              onClick={() => setIsCreateJobModalOpen(true)}
              className="flex items-center gap-1.5 px-2.5 py-1 bg-white text-black hover:bg-white/90 disabled:opacity-40 disabled:cursor-not-allowed rounded-md text-xs font-semibold transition shadow-sm"
            >
              <Plus size={12} className="stroke-[2.5]" />
              <span>Create Job</span>
            </button>
          </div>

          <div className="flex-1 overflow-y-auto p-4 space-y-3">
            {jobs.length === 0 ? (
              <div className="text-center p-12 text-[var(--color-fg-muted)] space-y-3">
                <Layers size={28} className="text-white/20 mx-auto" />
                <p className="text-white/80 text-xs font-medium">No automation jobs</p>
                <button
                  disabled={!can('jobs.create')}
                  onClick={() => setIsCreateJobModalOpen(true)}
                  className="inline-flex items-center gap-1.5 px-3 py-1.5 bg-white text-black hover:bg-white/90 disabled:opacity-40 rounded-md text-xs font-semibold transition shadow-sm"
                >
                  <Plus size={12} className="stroke-[2.5]" />
                  <span>Create Job</span>
                </button>
              </div>
            ) : (
              jobs.map((job) => {
                const isCompleted = job.status === 'completed';
                return (
                  <div
                    key={job.id}
                    className="p-4 rounded-lg bg-[var(--color-surface-1)] border border-[var(--color-border-default)] space-y-3 shadow-xs"
                  >
                    <div className="flex items-center justify-between">
                      <div className="space-y-1">
                        <div className="flex items-center gap-2">
                          <span className="font-semibold text-[var(--color-editor-fg)] text-xs">{job.name}</span>
                          <span
                            className={`px-1.5 py-0.5 rounded text-[10px] font-mono border ${
                              isCompleted
                                ? 'bg-white/10 text-white border-white/20'
                                : 'bg-white/5 text-white/70 border-white/10'
                            }`}
                          >
                            {job.status}
                          </span>
                        </div>
                        <p className="text-[11px] text-[var(--color-fg-muted)] font-mono">
                          ID: {job.id} • Target: {job.spec.target_type}
                        </p>
                      </div>

                      <button
                        onClick={() => handleRunJob(job.id)}
                        className="flex items-center gap-1.5 px-2.5 py-1 bg-white/10 hover:bg-white/20 text-white border border-white/20 rounded-md text-xs font-medium transition shadow-xs"
                      >
                        <Play size={11} className="fill-white" />
                        <span>Run</span>
                      </button>
                    </div>

                    <div className="p-2.5 rounded-md bg-[var(--color-surface-0)] border border-[var(--color-border-muted)] font-mono text-[11px] space-y-1 text-[var(--color-editor-fg)]">
                      <div className="text-[var(--color-fg-muted)] text-[10px]">Command / Script:</div>
                      <div>{job.spec.command_or_script}</div>
                      {job.output_log && (
                        <div className="pt-2 border-t border-[var(--color-border-muted)] text-[var(--color-fg-muted)] whitespace-pre-wrap text-[10px]">
                          {job.output_log}
                        </div>
                      )}
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </div>
      )}

      {/* Modal: Register SSH Host */}
      {isRegisterModalOpen && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-xs flex items-center justify-center p-4 z-50">
          <div className="bg-[var(--color-surface-1)] border border-[var(--color-border-default)] rounded-xl w-full max-w-md p-5 space-y-4 shadow-xl">
            <div className="flex items-center justify-between border-b border-[var(--color-border-muted)] pb-2.5">
              <span className="font-semibold text-[var(--color-editor-fg)] text-xs">Register Remote SSH Host</span>
              <button
                onClick={() => setIsRegisterModalOpen(false)}
                className="text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
              >
                <X size={14} />
              </button>
            </div>

            <form onSubmit={handleRegisterHost} className="space-y-3">
              <div>
                <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">Node Name</label>
                <input
                  type="text"
                  placeholder="e.g. Cluster GPU Node 01"
                  value={newHostName}
                  onChange={(e) => setNewHostName(e.target.value)}
                  className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none"
                  required
                />
              </div>

              <div className="grid grid-cols-3 gap-2">
                <div className="col-span-2">
                  <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">Host IP / FQDN</label>
                  <input
                    type="text"
                    value={newHostAddress}
                    onChange={(e) => setNewHostAddress(e.target.value)}
                    placeholder="192.168.1.50 or host.internal"
                    className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none"
                    required
                  />
                </div>
                <div>
                  <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">Port</label>
                  <input
                    type="number"
                    value={newHostPort}
                    onChange={(e) => setNewHostPort(parseInt(e.target.value) || 22)}
                    className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none"
                  />
                </div>
              </div>

              <div>
                <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">SSH Username</label>
                <input
                  type="text"
                  placeholder="e.g. ubuntu, root, dev"
                  value={newHostUser}
                  onChange={(e) => setNewHostUser(e.target.value)}
                  className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none"
                  required
                />
              </div>

              <div>
                <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">Private Key Path (optional)</label>
                <input
                  type="text"
                  placeholder="~/.ssh/id_ed25519"
                  value={newHostKeyPath}
                  onChange={(e) => setNewHostKeyPath(e.target.value)}
                  className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none font-mono"
                />
              </div>

              <div className="flex justify-end gap-2 pt-2 border-t border-[var(--color-border-muted)]">
                <button
                  type="button"
                  onClick={() => setIsRegisterModalOpen(false)}
                  className="px-3 py-1.5 rounded-md bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-0)] text-[var(--color-editor-fg)] text-xs border border-[var(--color-border-default)]"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={!can('hosts.register')}
                  className="px-3.5 py-1.5 rounded-md bg-white hover:bg-white/90 text-black text-xs font-semibold shadow-sm"
                >
                  Register Node
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Modal: Create Headless Job */}
      {isCreateJobModalOpen && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-xs flex items-center justify-center p-4 z-50">
          <div className="bg-[var(--color-surface-1)] border border-[var(--color-border-default)] rounded-xl w-full max-w-md p-5 space-y-4 shadow-xl">
            <div className="flex items-center justify-between border-b border-[var(--color-border-muted)] pb-2.5">
              <span className="font-semibold text-[var(--color-editor-fg)] text-xs">Create Headless Job</span>
              <button
                onClick={() => setIsCreateJobModalOpen(false)}
                className="text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]"
              >
                <X size={14} />
              </button>
            </div>

            <form onSubmit={handleCreateJob} className="space-y-3">
              <div>
                <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">Job Name</label>
                <input
                  type="text"
                  placeholder="e.g. Nightly Benchmark Sweep"
                  value={newJobName}
                  onChange={(e) => setNewJobName(e.target.value)}
                  className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none"
                  required
                />
              </div>

              <div>
                <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">Target Execution Type</label>
                <NativeSelect
                  value={newJobTargetType}
                  onChange={(e) => setNewJobTargetType(e.target.value)}
                  className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none"
                >
                  <option value="local_process">local_process (Host Process)</option>
                  <option value="remote_ssh">remote_ssh (Fleet Node)</option>
                  <option value="browser_session">browser_session (Headless Browser)</option>
                </NativeSelect>
              </div>

              <div>
                <label className="text-[var(--color-fg-muted)] text-[11px] block mb-1">Command or Script</label>
                <textarea
                  rows={3}
                  value={newJobCommand}
                  onChange={(e) => setNewJobCommand(e.target.value)}
                  className="w-full bg-[var(--color-surface-0)] border border-[var(--color-border-default)] rounded-md px-2.5 py-1.5 text-xs text-[var(--color-editor-fg)] focus:border-[var(--workbench-accent)] focus:outline-none font-mono"
                  required
                />
              </div>

              <div className="flex justify-end gap-2 pt-2 border-t border-[var(--color-border-muted)]">
                <button
                  type="button"
                  onClick={() => setIsCreateJobModalOpen(false)}
                  className="px-3 py-1.5 rounded-md bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-0)] text-[var(--color-editor-fg)] text-xs border border-[var(--color-border-default)]"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={!can('jobs.create')}
                  className="px-3.5 py-1.5 rounded-md bg-white hover:bg-white/90 text-black text-xs font-semibold shadow-sm"
                >
                  Save Job
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
