import React, { useState, useEffect, useCallback } from 'react';
import {
  Server,
  Plus,
  Play,
  Terminal,
  Activity,
  X,
  Layers,
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
}

export const RemoteFleetWorkbenchPane: React.FC<RemoteFleetWorkbenchPaneProps> = ({
  onShowToast,
}) => {
  const [activeTab, setActiveTab] = useState<'nodes' | 'automation'>('nodes');
  const [hosts, setHosts] = useState<RemoteHostNode[]>([]);
  const [selectedHostId, setSelectedHostId] = useState<string | null>(null);
  const [jobs, setJobs] = useState<HeadlessAutomationJob[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [commandInput, setCommandInput] = useState<string>('uname -a');
  const [execReceipts] = useState<FleetExecReceipt[]>([]);

  // Modals
  const [isRegisterModalOpen, setIsRegisterModalOpen] = useState<boolean>(false);
  const [isCreateJobModalOpen, setIsCreateJobModalOpen] = useState<boolean>(false);

  // New Host Form
  const [newHostName, setNewHostName] = useState<string>('');
  const [newHostAddress, setNewHostAddress] = useState<string>('127.0.0.1');
  const [newHostPort, setNewHostPort] = useState<number>(22);
  const [newHostUser, setNewHostUser] = useState<string>('ubuntu');
  const [newHostKeyPath, setNewHostKeyPath] = useState<string>('~/.ssh/id_ed25519');

  // New Job Form
  const [newJobName, setNewJobName] = useState<string>('');
  const [newJobCommand, setNewJobCommand] = useState<string>('cargo check --workspace');
  const [newJobTargetType, setNewJobTargetType] = useState<string>('local_process');

  const selectedHost = hosts.find((h) => h.id === selectedHostId) ?? hosts[0] ?? null;

  const loadHosts = useCallback(async () => {
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
  }, [selectedHostId, onShowToast]);

  const loadJobs = useCallback(async () => {
    try {
      const jobList = await daemonClient.listHeadlessJobs();
      setJobs(jobList);
    } catch (err) {
      onShowToast?.(`Failed to load automation jobs: ${String(err)}`);
    }
  }, [onShowToast]);

  useEffect(() => {
    loadHosts();
    loadJobs();
  }, [loadHosts, loadJobs]);

  const handleRegisterHost = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newHostName.trim() || !newHostAddress.trim() || !newHostUser.trim()) {
      onShowToast?.('Please fill in name, address, and user.');
      return;
    }

    try {
      const params: RegisterHostParams = {
        name: newHostName.trim(),
        host: newHostAddress.trim(),
        port: newHostPort || 22,
        user: newHostUser.trim(),
        private_key_path: newHostKeyPath.trim() || undefined,
        labels: { cluster: 'default', tier: 'worker' },
      };
      const node = await daemonClient.registerRemoteHost(params);
      setHosts((prev) => [...prev, node]);
      setSelectedHostId(node.id);
      setIsRegisterModalOpen(false);
      setNewHostName('');
      onShowToast?.(`Remote host '${node.name}' registered.`);
    } catch (err) {
      onShowToast?.(`Failed to register host: ${String(err)}`);
    }
  };

  const handlePingHost = async (hostId: string) => {
    try {
      const res = await daemonClient.pingRemoteHost(hostId);
      setHosts((prev) =>
        prev.map((h) =>
          h.id === hostId
            ? {
                ...h,
                status: 'online',
                last_ping_ms: res.ping_ms ?? 24,
                lastPingMs: res.ping_ms ?? 24,
              }
            : h
        )
      );
      onShowToast?.(`Node online. Latency: ${res.ping_ms ?? 24}ms.`);
    } catch (err) {
      onShowToast?.(`Ping probe failed: ${String(err)}`);
    }
  };

  const handleExecCommand = async (e: React.FormEvent) => {
    e.preventDefault();
    onShowToast?.('Remote execution unavailable: SSH transport adapter is not connected.');
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
    <div className="flex flex-col h-full bg-[#121214] text-zinc-200 text-xs overflow-hidden select-none font-sans">
      {/* Top Header / Mode Switcher */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-zinc-800 bg-[#18181b]">
        <div className="flex items-center gap-2">
          <Server size={14} className="text-cyan-400 shrink-0" />
          <span className="font-semibold text-zinc-100 tracking-wide text-[11px] uppercase">
            Remote SSH Fleet & Headless Automation
          </span>
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-400 font-mono">
            Roadmap Step 10
          </span>
        </div>

        <div className="flex items-center gap-1 bg-zinc-900 p-0.5 rounded border border-zinc-800">
          <button
            onClick={() => setActiveTab('nodes')}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded text-[11px] font-medium transition ${
              activeTab === 'nodes'
                ? 'bg-zinc-800 text-zinc-100 shadow-xs'
                : 'text-zinc-400 hover:text-zinc-200'
            }`}
          >
            <Server size={12} />
            <span>Fleet Nodes ({hosts.length})</span>
          </button>
          <button
            onClick={() => setActiveTab('automation')}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded text-[11px] font-medium transition ${
              activeTab === 'automation'
                ? 'bg-zinc-800 text-zinc-100 shadow-xs'
                : 'text-zinc-400 hover:text-zinc-200'
            }`}
          >
            <Layers size={12} />
            <span>Headless Jobs ({jobs.length})</span>
          </button>
        </div>
      </div>

      {/* Fail-closed Degraded Status Notification */}
      <div className="bg-amber-500/10 border-b border-amber-500/20 px-3 py-1.5 flex items-center justify-between text-[11px] text-amber-400 shrink-0">
        <div className="flex items-center gap-1.5">
          <span className="h-1.5 w-1.5 rounded-full bg-amber-400 animate-pulse" />
          <span>SSH Transport Gated: Remote execution, ping probes, and headless job execution are degraded (metadata-only mode).</span>
        </div>
        <span className="font-mono text-[10px] text-amber-400/80">Fail-Closed</span>
      </div>

      {/* Main Tab Content */}
      {activeTab === 'nodes' ? (
        <div className="flex-1 flex overflow-hidden">
          {/* Left: Nodes List */}
          <div className="w-72 flex flex-col border-r border-zinc-800 bg-[#161619]">
            <div className="flex items-center justify-between p-3 border-b border-zinc-800">
              <span className="text-[11px] font-semibold text-zinc-400">Inventory Nodes</span>
              <button
                onClick={() => setIsRegisterModalOpen(true)}
                className="flex items-center gap-1 px-2 py-0.5 bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 rounded text-zinc-200 text-[11px] transition"
              >
                <Plus size={11} />
                <span>Add Node</span>
              </button>
            </div>

            <div className="flex-1 overflow-y-auto p-2 space-y-1.5">
              {isLoading ? (
                <div className="text-center p-4 text-zinc-500">Loading nodes...</div>
              ) : hosts.length === 0 ? (
                <div className="text-center p-6 space-y-2">
                  <Server size={24} className="text-zinc-700 mx-auto" />
                  <p className="text-zinc-500 text-[11px]">No remote SSH nodes.</p>
                  <button
                    onClick={() => setIsRegisterModalOpen(true)}
                    className="px-2.5 py-1 bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 rounded text-zinc-300 text-[11px]"
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
                      className={`p-2.5 rounded-lg border cursor-pointer transition space-y-1 ${
                        isSelected
                          ? 'bg-zinc-800/80 border-cyan-800 text-zinc-100 shadow-xs'
                          : 'bg-zinc-900/40 border-zinc-800/80 text-zinc-400 hover:bg-zinc-800/40 hover:text-zinc-200'
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <span className="font-semibold text-xs text-zinc-200">{host.name}</span>
                        <span
                          className={`px-1.5 py-0.2 rounded text-[10px] font-mono border ${
                            isOnline
                              ? 'bg-emerald-950/60 text-emerald-400 border-emerald-800/40'
                              : 'bg-zinc-900 text-zinc-500 border-zinc-800'
                          }`}
                        >
                          {host.status}
                        </span>
                      </div>
                      <div className="flex items-center justify-between text-[10px] text-zinc-500 font-mono">
                        <span>{host.user}@{host.host}:{host.port}</span>
                        {host.last_ping_ms != null && (
                          <span className="text-emerald-400">{host.last_ping_ms}ms</span>
                        )}
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          </div>

          {/* Right: Node Detail & Execution Console */}
          <div className="flex-1 flex flex-col bg-zinc-950 overflow-hidden">
            {!selectedHost ? (
              <div className="flex-1 flex flex-col items-center justify-center text-center p-8 text-zinc-500">
                <Server size={32} className="text-zinc-700 mb-2" />
                <p>Select or register an SSH host node to inspect and execute tasks.</p>
              </div>
            ) : (
              <div className="flex-1 flex flex-col overflow-hidden">
                {/* Node Top Banner */}
                <div className="flex items-center justify-between p-3 border-b border-zinc-800 bg-[#18181b]">
                  <div className="flex items-center gap-3">
                    <div className="space-y-0.5">
                      <div className="flex items-center gap-2">
                        <span className="font-semibold text-zinc-100 text-xs">
                          {selectedHost.name}
                        </span>
                        <span className="text-[10px] font-mono text-zinc-400">
                          ({selectedHost.user}@{selectedHost.host}:{selectedHost.port})
                        </span>
                      </div>
                      <div className="flex items-center gap-2 text-[10px] text-zinc-500 font-mono">
                        <span>ID: {selectedHost.id}</span>
                        <span>•</span>
                        <span>Auth: {selectedHost.auth_method.type}</span>
                      </div>
                    </div>
                  </div>

                  <button
                    disabled={true}
                    title="Ping probe unavailable: SSH transport adapter is not connected"
                    onClick={() => handlePingHost(selectedHost.id)}
                    className="flex items-center gap-1 px-2.5 py-1 bg-zinc-800/50 border border-zinc-800 rounded text-zinc-500 text-[11px] cursor-not-allowed"
                  >
                    <Activity size={12} className="text-zinc-500" />
                    <span>Ping Probe</span>
                  </button>
                </div>

                {/* Command Runner Bar */}
                <form
                  onSubmit={handleExecCommand}
                  className="flex items-center gap-2 p-3 border-b border-zinc-800 bg-[#161619]"
                >
                  <Terminal size={14} className="text-zinc-500 shrink-0" />
                  <input
                    type="text"
                    value={commandInput}
                    onChange={(e) => setCommandInput(e.target.value)}
                    disabled={true}
                    placeholder="Remote command execution unavailable (SSH transport adapter not connected)..."
                    className="flex-1 bg-zinc-950/60 border border-zinc-850 rounded px-3 py-1 font-mono text-xs text-zinc-500 focus:outline-none cursor-not-allowed"
                  />
                  <button
                    type="submit"
                    disabled={true}
                    title="Remote execution unavailable: SSH transport adapter is not connected"
                    className="flex items-center gap-1.5 px-3 py-1 bg-zinc-850 border border-zinc-800 rounded text-zinc-500 font-medium text-[11px] cursor-not-allowed"
                  >
                    <Play size={11} />
                    <span>Execute</span>
                  </button>
                </form>

                {/* Execution Receipts Stream */}
                <div className="flex-1 overflow-y-auto p-4 space-y-3 font-mono text-[11px]">
                  <span className="text-[10px] font-sans font-semibold uppercase text-zinc-500 tracking-wider">
                    Execution Receipts Ledger
                  </span>

                  {execReceipts.length === 0 ? (
                    <div className="p-6 text-center text-zinc-600 font-sans text-xs">
                      No executions recorded on this node yet. Enter a command above to dispatch.
                    </div>
                  ) : (
                    execReceipts.map((rcpt) => (
                      <div
                        key={rcpt.execution_id}
                        className="rounded-lg bg-zinc-900/60 border border-zinc-800 p-3 space-y-2"
                      >
                        <div className="flex items-center justify-between border-b border-zinc-800/60 pb-1.5 text-zinc-400">
                          <div className="flex items-center gap-2">
                            <span className="text-cyan-400 font-bold">$ {rcpt.command}</span>
                            <span className="text-[10px] px-1 py-0.2 rounded bg-zinc-800 text-zinc-400">
                              exit {rcpt.exit_code ?? 0}
                            </span>
                          </div>
                          <span className="text-zinc-500 text-[10px]">{rcpt.duration_ms}ms</span>
                        </div>
                        <div className="text-zinc-300 whitespace-pre-wrap text-xs font-mono bg-zinc-950 p-2 rounded border border-zinc-900">
                          {rcpt.stdout || '<empty stdout>'}
                          {rcpt.stderr && (
                            <span className="text-red-400 block mt-1">{rcpt.stderr}</span>
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
        <div className="flex-1 flex flex-col bg-zinc-950 overflow-hidden">
          <div className="flex items-center justify-between p-3 border-b border-zinc-800 bg-[#161619]">
            <div className="space-y-0.5">
              <span className="font-semibold text-zinc-200 text-xs">Headless Job Registry</span>
              <p className="text-zinc-500 text-[11px]">
                Autonomous batch executions, verification recipes, and scheduled regression checks.
              </p>
            </div>
            <button
              onClick={() => setIsCreateJobModalOpen(true)}
              className="flex items-center gap-1.5 px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 rounded text-zinc-200 text-xs font-medium transition"
            >
              <Plus size={12} />
              <span>Create Headless Job</span>
            </button>
          </div>

          <div className="flex-1 overflow-y-auto p-4 space-y-3">
            {jobs.length === 0 ? (
              <div className="text-center p-12 text-zinc-600 space-y-2">
                <Layers size={32} className="text-zinc-700 mx-auto" />
                <p className="text-zinc-400 text-xs font-medium">No headless jobs registered.</p>
                <p className="text-[11px]">Define automated batch jobs with verification evidence.</p>
              </div>
            ) : (
              jobs.map((job) => {
                const isCompleted = job.status === 'completed';
                return (
                  <div
                    key={job.id}
                    className="p-4 rounded-lg bg-zinc-900/50 border border-zinc-800 space-y-3"
                  >
                    <div className="flex items-center justify-between">
                      <div className="space-y-1">
                        <div className="flex items-center gap-2">
                          <span className="font-semibold text-zinc-200 text-xs">{job.name}</span>
                          <span
                            className={`px-1.5 py-0.2 rounded text-[10px] font-mono border ${
                              isCompleted
                                ? 'bg-emerald-950/60 text-emerald-400 border-emerald-800/40'
                                : 'bg-amber-950/60 text-amber-400 border-amber-800/40'
                            }`}
                          >
                            {job.status}
                          </span>
                        </div>
                        <p className="text-[11px] text-zinc-500 font-mono">
                          ID: {job.id} • Target: {job.spec.target_type}
                        </p>
                      </div>

                      <button
                        disabled={true}
                        title="Headless job execution unavailable: executor adapter not connected"
                        onClick={() => handleRunJob(job.id)}
                        className="flex items-center gap-1.5 px-3 py-1 bg-zinc-800/50 border border-zinc-800 rounded text-zinc-500 text-xs font-medium cursor-not-allowed"
                      >
                        <Play size={11} className="text-zinc-500" />
                        <span>Run Job</span>
                      </button>
                    </div>

                    <div className="p-2.5 rounded bg-zinc-950 border border-zinc-900 font-mono text-[11px] space-y-1 text-zinc-300">
                      <div className="text-zinc-500 text-[10px]">Command / Script:</div>
                      <div>{job.spec.command_or_script}</div>
                      {job.output_log && (
                        <div className="pt-2 border-t border-zinc-900 text-zinc-400 whitespace-pre-wrap text-[10px]">
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
        <div className="fixed inset-0 bg-black/70 backdrop-blur-xs flex items-center justify-center p-4 z-50">
          <div className="bg-[#18181b] border border-zinc-800 rounded-xl w-full max-w-md p-5 space-y-4 shadow-xl">
            <div className="flex items-center justify-between border-b border-zinc-800 pb-2">
              <span className="font-semibold text-zinc-100 text-xs">Register Remote SSH Host</span>
              <button
                onClick={() => setIsRegisterModalOpen(false)}
                className="text-zinc-500 hover:text-zinc-300"
              >
                <X size={14} />
              </button>
            </div>

            <form onSubmit={handleRegisterHost} className="space-y-3">
              <div>
                <label className="text-zinc-400 text-[11px] block mb-1">Node Name</label>
                <input
                  type="text"
                  placeholder="e.g. GPU Worker 01"
                  value={newHostName}
                  onChange={(e) => setNewHostName(e.target.value)}
                  className="w-full bg-zinc-900 border border-zinc-800 rounded px-2.5 py-1 text-xs text-zinc-200 focus:outline-none"
                  required
                />
              </div>

              <div className="grid grid-cols-3 gap-2">
                <div className="col-span-2">
                  <label className="text-zinc-400 text-[11px] block mb-1">Host IP / FQDN</label>
                  <input
                    type="text"
                    value={newHostAddress}
                    onChange={(e) => setNewHostAddress(e.target.value)}
                    className="w-full bg-zinc-900 border border-zinc-800 rounded px-2.5 py-1 text-xs text-zinc-200 focus:outline-none"
                    required
                  />
                </div>
                <div>
                  <label className="text-zinc-400 text-[11px] block mb-1">Port</label>
                  <input
                    type="number"
                    value={newHostPort}
                    onChange={(e) => setNewHostPort(parseInt(e.target.value) || 22)}
                    className="w-full bg-zinc-900 border border-zinc-800 rounded px-2.5 py-1 text-xs text-zinc-200 focus:outline-none"
                  />
                </div>
              </div>

              <div>
                <label className="text-zinc-400 text-[11px] block mb-1">SSH Username</label>
                <input
                  type="text"
                  value={newHostUser}
                  onChange={(e) => setNewHostUser(e.target.value)}
                  className="w-full bg-zinc-900 border border-zinc-800 rounded px-2.5 py-1 text-xs text-zinc-200 focus:outline-none"
                  required
                />
              </div>

              <div>
                <label className="text-zinc-400 text-[11px] block mb-1">Private Key Path</label>
                <input
                  type="text"
                  value={newHostKeyPath}
                  onChange={(e) => setNewHostKeyPath(e.target.value)}
                  className="w-full bg-zinc-900 border border-zinc-800 rounded px-2.5 py-1 text-xs text-zinc-200 focus:outline-none font-mono"
                />
              </div>

              <div className="flex justify-end gap-2 pt-2">
                <button
                  type="button"
                  onClick={() => setIsRegisterModalOpen(false)}
                  className="px-3 py-1.5 rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300 text-xs"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="px-3 py-1.5 rounded bg-cyan-700 hover:bg-cyan-600 text-zinc-100 text-xs font-medium"
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
        <div className="fixed inset-0 bg-black/70 backdrop-blur-xs flex items-center justify-center p-4 z-50">
          <div className="bg-[#18181b] border border-zinc-800 rounded-xl w-full max-w-md p-5 space-y-4 shadow-xl">
            <div className="flex items-center justify-between border-b border-zinc-800 pb-2">
              <span className="font-semibold text-zinc-100 text-xs">Create Headless Job</span>
              <button
                onClick={() => setIsCreateJobModalOpen(false)}
                className="text-zinc-500 hover:text-zinc-300"
              >
                <X size={14} />
              </button>
            </div>

            <form onSubmit={handleCreateJob} className="space-y-3">
              <div>
                <label className="text-zinc-400 text-[11px] block mb-1">Job Name</label>
                <input
                  type="text"
                  placeholder="e.g. Nightly Benchmark Sweep"
                  value={newJobName}
                  onChange={(e) => setNewJobName(e.target.value)}
                  className="w-full bg-zinc-900 border border-zinc-800 rounded px-2.5 py-1 text-xs text-zinc-200 focus:outline-none"
                  required
                />
              </div>

              <div>
                <label className="text-zinc-400 text-[11px] block mb-1">Target Execution Type</label>
                <select
                  value={newJobTargetType}
                  onChange={(e) => setNewJobTargetType(e.target.value)}
                  className="w-full bg-zinc-900 border border-zinc-800 rounded px-2.5 py-1 text-xs text-zinc-200 focus:outline-none"
                >
                  <option value="local_process">local_process (Host Process)</option>
                  <option value="remote_ssh">remote_ssh (Fleet Node)</option>
                  <option value="browser_session">browser_session (Headless Browser)</option>
                </select>
              </div>

              <div>
                <label className="text-zinc-400 text-[11px] block mb-1">Command or Script</label>
                <textarea
                  rows={3}
                  value={newJobCommand}
                  onChange={(e) => setNewJobCommand(e.target.value)}
                  className="w-full bg-zinc-900 border border-zinc-800 rounded px-2.5 py-1 text-xs text-zinc-200 focus:outline-none font-mono"
                  required
                />
              </div>

              <div className="flex justify-end gap-2 pt-2">
                <button
                  type="button"
                  onClick={() => setIsCreateJobModalOpen(false)}
                  className="px-3 py-1.5 rounded bg-zinc-800 hover:bg-zinc-700 text-zinc-300 text-xs"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="px-3 py-1.5 rounded bg-cyan-700 hover:bg-cyan-600 text-zinc-100 text-xs font-medium"
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
