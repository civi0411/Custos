import React, { useState, useEffect, useCallback } from 'react';
import { Server, Layers } from 'lucide-react';
import type {
  RemoteHostNode,
  FleetExecReceipt,
  HeadlessAutomationJob,
  RegisterHostParams,
  CreateHeadlessJobParams,
} from '@/types/domain';
import { daemonClient } from '@/api/daemon_client';
import type { RemoteFleetWorkbenchPaneProps } from './types';
import { RegisterHostModal } from './RegisterHostModal';
import { CreateJobModal } from './CreateJobModal';
import { NodesTabView } from './NodesTabView';
import { JobsTabView } from './JobsTabView';

export const RemoteFleetWorkbenchView: React.FC<RemoteFleetWorkbenchPaneProps> = ({
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
      <header className="wb-pane-toolbar flex h-11 shrink-0 items-center justify-between border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-4 z-10">
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
            <div
              className="flex items-center gap-1.5 px-2 py-0.5 rounded bg-white/5 border border-white/10 text-white/60 text-[10px] font-mono"
              title={capabilityReason || 'OpenSSH transport not connected'}
            >
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
        <NodesTabView
          hosts={hosts}
          selectedHostId={selectedHostId}
          selectedHost={selectedHost}
          isLoading={isLoading}
          canRegister={can('hosts.register')}
          canExec={canExec}
          canPing={canPing}
          isExecuting={isExecuting}
          isPinging={isPinging}
          commandInput={commandInput}
          execReceipts={execReceipts}
          onSelectHost={setSelectedHostId}
          onOpenRegisterModal={() => setIsRegisterModalOpen(true)}
          onPingHost={handlePingHost}
          onCommandInputChange={setCommandInput}
          onExecCommand={handleExecCommand}
          onClearReceipts={() => setExecReceipts([])}
        />
      ) : (
        <JobsTabView
          jobs={jobs}
          canCreate={can('jobs.create')}
          onOpenCreateModal={() => setIsCreateJobModalOpen(true)}
          onRunJob={handleRunJob}
        />
      )}

      {/* Modal: Register SSH Host */}
      <RegisterHostModal
        isOpen={isRegisterModalOpen}
        canRegister={can('hosts.register')}
        newHostName={newHostName}
        newHostAddress={newHostAddress}
        newHostPort={newHostPort}
        newHostUser={newHostUser}
        newHostKeyPath={newHostKeyPath}
        onClose={() => setIsRegisterModalOpen(false)}
        onNameChange={setNewHostName}
        onAddressChange={setNewHostAddress}
        onPortChange={setNewHostPort}
        onUserChange={setNewHostUser}
        onKeyPathChange={setNewHostKeyPath}
        onSubmit={handleRegisterHost}
      />

      {/* Modal: Create Headless Job */}
      <CreateJobModal
        isOpen={isCreateJobModalOpen}
        canCreate={can('jobs.create')}
        newJobName={newJobName}
        newJobTargetType={newJobTargetType}
        newJobCommand={newJobCommand}
        onClose={() => setIsCreateJobModalOpen(false)}
        onNameChange={setNewJobName}
        onTargetTypeChange={setNewJobTargetType}
        onCommandChange={setNewJobCommand}
        onSubmit={handleCreateJob}
      />
    </div>
  );
};
