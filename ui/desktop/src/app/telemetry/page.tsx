import React from 'react';
import { useNavigate } from 'react-router-dom';
import { Activity, ServerOff, ArrowLeft } from 'lucide-react';
import { useDaemon } from '@/hooks/useDaemon';

export const TelemetryPage: React.FC = () => {
  const navigate = useNavigate();
  const { isOnline, isChecking, lastChecked, checkHealth } = useDaemon();

  return (
    <main className="flex h-full flex-col bg-[var(--color-canvas)] text-[var(--color-editor-fg)] font-sans">
      <header className="flex h-12 items-center justify-between border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] px-5">
        <div className="flex items-center gap-3">
          <button
            onClick={() => navigate('/studio')}
            className="p-1.5 rounded-lg bg-surface-2 hover:bg-surface-3 border border-border-default text-fg-muted hover:text-fg-editor transition flex items-center gap-1.5 text-xs"
          >
            <ArrowLeft className="w-3.5 h-3.5" />
            <span>Studio</span>
          </button>
          <div className="h-4 w-[1px] bg-border-muted" />
          <div className="flex items-center gap-2">
            <Activity className="h-4 w-4 text-[var(--color-fg-muted)]" />
            <span className="text-sm font-semibold">Runtime telemetry</span>
          </div>
        </div>
        <button
          type="button"
          onClick={() => void checkHealth()}
          disabled={isChecking}
          className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-2)] px-2.5 py-1.5 text-xs disabled:opacity-50"
        >
          {isChecking ? 'Checking…' : 'Check daemon'}
        </button>
      </header>
      <div className="flex flex-1 items-center justify-center p-8">
        <div className="max-w-md text-center">
          <ServerOff className="mx-auto mb-4 h-6 w-6 text-[var(--color-fg-muted)]" />
          <h1 className="text-sm font-semibold">{isOnline ? 'Daemon connected' : 'Daemon unavailable'}</h1>
          <p className="mt-2 text-xs leading-5 text-[var(--color-fg-muted)]">
            Runtime traces will appear only after the daemon exposes a versioned telemetry stream.
            Custos does not generate example traces or imply that an effect occurred.
          </p>
          {lastChecked && (
            <p className="mt-3 font-mono text-[10px] text-[var(--color-fg-subtle)]">
              Last checked {new Date(lastChecked).toLocaleTimeString()}
            </p>
          )}
        </div>
      </div>
    </main>
  );
};

export default TelemetryPage;
