import React, { useEffect } from 'react';
import { ArrowLeft, Settings2 } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { useAppContext } from '@/context/AppContext';

/** Settings use one shared surface so Studio and standalone routes cannot drift. */
export const SettingsPage: React.FC = () => {
  const navigate = useNavigate();
  const { setIsSettingsOpen } = useAppContext();

  useEffect(() => {
    setIsSettingsOpen(true);
  }, [setIsSettingsOpen]);

  return (
    <main className="flex h-full w-full items-center justify-center bg-[var(--color-canvas)] p-8 text-[var(--color-editor-fg)]">
      <section className="max-w-md text-center">
        <div className="mx-auto flex h-10 w-10 items-center justify-center rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-1)]">
          <Settings2 className="h-4 w-4 text-[var(--color-fg-muted)]" />
        </div>
        <h1 className="mt-4 text-sm font-semibold">Settings</h1>
        <p className="mt-2 text-xs leading-5 text-[var(--color-fg-muted)]">
          One surface owns device appearance, daemon health, providers, authority and storage.
          Closing it does not fabricate or mutate backend state.
        </p>
        <div className="mt-5 flex justify-center gap-2">
          <button type="button" onClick={() => setIsSettingsOpen(true)} className="rounded-md border border-[var(--color-border-default)] bg-[var(--color-surface-1)] px-3 py-2 text-xs hover:bg-[var(--color-surface-2)]">
            Open settings
          </button>
          <button type="button" onClick={() => navigate('/studio')} className="flex items-center gap-1.5 rounded-md px-3 py-2 text-xs text-[var(--color-fg-muted)] hover:text-[var(--color-editor-fg)]">
            <ArrowLeft className="h-3.5 w-3.5" /> Studio
          </button>
        </div>
      </section>
    </main>
  );
};

export default SettingsPage;
