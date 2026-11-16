import React from 'react';
import {
  RefreshCw,
  Cpu,
  Save,
  Trash2,
  X,
} from 'lucide-react';
import type { LocalHardwareProfile } from '@/types/domain';
import { formatBytes } from './types';

export interface HardwareSpecsBarProps {
  serverStatus: 'idle' | 'online' | 'offline' | 'error';
  probeLatencyMs: number | null;
  instancesCount: number;
  isProbing: boolean;
  onProbe: () => void;
  hasExistingAccount: boolean;
  onConnectLMStudio: () => void;
  onDisconnectLMStudio: () => void;
  isSavingProvider: boolean;
  endpoint: string;
  tempEndpoint: string;
  setTempEndpoint: (val: string) => void;
  isEditingEndpoint: boolean;
  setIsEditingEndpoint: (val: boolean) => void;
  onApplyEndpoint: () => void;
  hardware: LocalHardwareProfile | null;
}

export const HardwareSpecsBar: React.FC<HardwareSpecsBarProps> = ({
  serverStatus,
  probeLatencyMs,
  instancesCount,
  isProbing,
  onProbe,
  hasExistingAccount,
  onConnectLMStudio,
  onDisconnectLMStudio,
  isSavingProvider,
  endpoint,
  tempEndpoint,
  setTempEndpoint,
  isEditingEndpoint,
  setIsEditingEndpoint,
  onApplyEndpoint,
  hardware,
}) => {
  return (
    <div className="rounded-2xl border border-[var(--color-border-default)] bg-[var(--color-surface-1)] p-4 sm:p-5 space-y-4 shadow-xs">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <div className="flex size-9 items-center justify-center rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] shrink-0 shadow-2xs">
            <Cpu className="size-4.5" />
          </div>
          <div>
            <div className="flex flex-wrap items-center gap-2">
              <h2 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-editor-fg)] font-mono">
                Local Models & Runtimes
              </h2>
              {/* Status Indicator */}
              <span className="inline-flex items-center gap-1.5 rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2 py-0.5 font-mono text-[10px] text-[var(--color-fg-muted)]">
                <span
                  className={`size-1.5 rounded-full ${
                    serverStatus === 'online' ? 'bg-emerald-400 ring-2 ring-emerald-400/20' : 'bg-[var(--color-fg-subtle)]'
                  }`}
                />
                {serverStatus === 'online'
                  ? `LM Studio (${probeLatencyMs ? `${probeLatencyMs}ms` : 'online'})`
                  : 'LM Studio Offline'}
              </span>
              {instancesCount > 0 && (
                <span className="inline-flex items-center gap-1 rounded-md border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 font-mono text-[10px] text-emerald-400 font-medium">
                  <span className="size-1.5 rounded-full bg-emerald-400" />
                  {instancesCount} in VRAM
                </span>
              )}
            </div>
            <p className="text-[11px] text-[var(--color-fg-muted)] mt-0.5">
              Configure context, GPU offload layers, and load multiple models into memory simultaneously.
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2 self-end sm:self-auto shrink-0">
          {/* Probe button */}
          <button
            type="button"
            onClick={onProbe}
            disabled={isProbing}
            className="flex items-center gap-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-3 py-1.5 text-xs font-medium text-[var(--color-editor-fg)] hover:bg-[var(--color-surface-3)] hover:border-[var(--color-border-emphasis)] transition disabled:opacity-50 cursor-pointer"
          >
            <RefreshCw className={`size-3 ${isProbing ? 'animate-spin' : ''}`} />
            {isProbing ? 'Probing…' : 'Probe Server'}
          </button>

          {/* Connect / Disconnect button */}
          {hasExistingAccount ? (
            <button
              type="button"
              onClick={onDisconnectLMStudio}
              className="flex items-center gap-1.5 rounded-lg border border-rose-500/30 bg-rose-500/10 px-3 py-1.5 text-xs font-medium text-rose-400 hover:bg-rose-500/20 hover:border-rose-500/50 transition cursor-pointer"
              title="Disconnect LM Studio account from Custos"
            >
              <Trash2 className="size-3" /> Disconnect
            </button>
          ) : (
            <button
              type="button"
              onClick={onConnectLMStudio}
              disabled={isSavingProvider || serverStatus !== 'online'}
              className="flex items-center gap-1.5 rounded-lg bg-[var(--color-editor-fg)] px-3.5 py-1.5 text-xs font-semibold text-[var(--color-canvas)] hover:opacity-90 active:scale-[0.98] transition disabled:opacity-40 cursor-pointer shadow-xs"
            >
              <Save className="size-3" /> Connect LM Studio
            </button>
          )}
        </div>
      </div>

      {/* Endpoint Row */}
      <div className="flex items-center justify-between text-xs pt-3 border-t border-[var(--color-border-muted)] font-mono text-[var(--color-fg-muted)]">
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-[var(--color-fg-subtle)] text-[10px] uppercase shrink-0">Server Endpoint:</span>
          {isEditingEndpoint ? (
            <div className="flex items-center gap-1.5 flex-wrap">
              <input
                type="text"
                value={tempEndpoint}
                onChange={(e) => setTempEndpoint(e.target.value)}
                className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-0)] px-2.5 py-0.5 text-xs text-[var(--color-editor-fg)] outline-none w-64 focus:border-[var(--color-border-emphasis)]"
              />
              <button
                type="button"
                onClick={onApplyEndpoint}
                className="rounded-md bg-[var(--color-editor-fg)] px-2.5 py-0.5 text-[11px] font-medium text-[var(--color-canvas)] hover:opacity-90 cursor-pointer"
              >
                Apply
              </button>
              <button
                type="button"
                onClick={() => {
                  setTempEndpoint(endpoint);
                  setIsEditingEndpoint(false);
                }}
                className="p-1 text-[var(--color-fg-subtle)] hover:text-[var(--color-editor-fg)] cursor-pointer"
              >
                <X className="size-3" />
              </button>
            </div>
          ) : (
            <span className="text-[var(--color-editor-fg)] font-mono truncate">{endpoint}</span>
          )}
        </div>
        {!isEditingEndpoint && (
          <button
            type="button"
            onClick={() => {
              setTempEndpoint(endpoint);
              setIsEditingEndpoint(true);
            }}
            className="text-[11px] text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)] underline underline-offset-2 cursor-pointer shrink-0 ml-2"
          >
            Change URL
          </button>
        )}
      </div>

      {/* Hardware specs row */}
      {hardware && (
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-2.5 pt-3 border-t border-[var(--color-border-muted)]">
          <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-3 shadow-2xs">
            <div className="text-[10px] text-[var(--color-fg-subtle)] font-mono uppercase tracking-wider">
              Processor
            </div>
            <div className="mt-1 text-xs font-semibold text-[var(--color-editor-fg)] truncate">
              {hardware.chip_name || 'Host CPU'}
            </div>
          </div>
          <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-3 shadow-2xs">
            <div className="text-[10px] text-[var(--color-fg-subtle)] font-mono uppercase tracking-wider">
              Unified RAM
            </div>
            <div className="mt-1 text-xs font-semibold text-[var(--color-editor-fg)] font-mono">
              {formatBytes(hardware.total_ram_bytes)}
            </div>
          </div>
          <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-surface-0)] p-3 shadow-2xs">
            <div className="text-[10px] text-[var(--color-fg-subtle)] font-mono uppercase tracking-wider">
              Metal VRAM Limit
            </div>
            <div className="mt-1 text-xs font-semibold text-[var(--color-editor-fg)] font-mono">
              {formatBytes(hardware.metal_vram_limit_bytes)}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
