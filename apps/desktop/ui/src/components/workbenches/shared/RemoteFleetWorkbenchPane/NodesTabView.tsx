import React from 'react';
import {
  Server,
  Plus,
  RefreshCw,
  Activity,
  Terminal,
  Play,
  Clock,
} from 'lucide-react';
import type { RemoteHostNode, FleetExecReceipt } from '@/types/domain';

export interface NodesTabViewProps {
  hosts: RemoteHostNode[];
  selectedHostId: string | null;
  selectedHost: RemoteHostNode | null;
  isLoading: boolean;
  canRegister: boolean;
  canExec: boolean;
  canPing: boolean;
  isExecuting: boolean;
  isPinging: boolean;
  commandInput: string;
  execReceipts: FleetExecReceipt[];
  onSelectHost: (id: string) => void;
  onOpenRegisterModal: () => void;
  onPingHost: (id: string) => void;
  onCommandInputChange: (v: string) => void;
  onExecCommand: (e: React.FormEvent) => void;
  onClearReceipts: () => void;
}

export const NodesTabView: React.FC<NodesTabViewProps> = ({
  hosts,
  selectedHostId,
  selectedHost,
  isLoading,
  canRegister,
  canExec,
  canPing,
  isExecuting,
  isPinging,
  commandInput,
  execReceipts,
  onSelectHost,
  onOpenRegisterModal,
  onPingHost,
  onCommandInputChange,
  onExecCommand,
  onClearReceipts,
}) => {
  return (
    <div className="flex-1 flex overflow-hidden">
      {/* Left: Nodes List */}
      <div className="w-72 flex flex-col border-r border-[var(--color-border-muted)] bg-[var(--color-surface-1)]">
        <div className="flex items-center justify-between p-3 border-b border-[var(--color-border-muted)]">
          <span className="text-[11px] font-semibold text-[var(--color-fg-muted)] uppercase tracking-wider">
            Nodes
          </span>
          <button
            disabled={!canRegister}
            onClick={onOpenRegisterModal}
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
                disabled={!canRegister}
                onClick={onOpenRegisterModal}
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
                  onClick={() => onSelectHost(host.id)}
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
                onClick={() => onPingHost(selectedHost.id)}
                title="Send OpenSSH ping probe"
                className="flex items-center gap-1.5 px-3 py-1 bg-white/10 hover:bg-white/15 disabled:opacity-40 disabled:cursor-not-allowed border border-white/20 rounded-lg text-white text-xs font-medium transition"
              >
                <Activity size={12} className={isPinging ? 'text-white/80 animate-spin' : 'text-white/70'} />
                <span>{isPinging ? 'Probing...' : 'Ping Probe'}</span>
              </button>
            </div>

            {/* Command Runner Bar */}
            <form
              onSubmit={onExecCommand}
              className="flex items-center gap-2 p-3 border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)]/60"
            >
              <Terminal size={14} className="text-white/50 shrink-0" />
              <input
                type="text"
                value={commandInput}
                onChange={(e) => onCommandInputChange(e.target.value)}
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
                    onClick={onClearReceipts}
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
  );
};
