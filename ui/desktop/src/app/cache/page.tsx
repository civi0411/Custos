import React from 'react';
import { useNavigate } from 'react-router-dom';
import { ArrowLeft, Database } from 'lucide-react';

/** Cache diagnostics require a daemon inventory API; never invent keys or metrics. */
export const CachePage: React.FC = () => {
  const navigate = useNavigate();

  return (
    <main className="flex h-full flex-1 flex-col bg-canvas text-fg-editor">
      <header className="flex h-12 items-center gap-3 border-b border-border-muted px-4">
        <button
          type="button"
          onClick={() => navigate('/studio')}
          className="rounded p-1.5 text-fg-muted hover:bg-surface-2 hover:text-fg-editor"
          aria-label="Back to Studio"
        >
          <ArrowLeft className="h-4 w-4" />
        </button>
        <Database className="h-4 w-4 text-fg-muted" />
        <h1 className="text-sm font-medium">Cache diagnostics</h1>
      </header>
      <section className="mx-auto flex w-full max-w-xl flex-1 flex-col justify-center px-6 text-sm">
        <h2 className="font-medium">Cache inventory is not available yet</h2>
        <p className="mt-2 leading-6 text-fg-muted">
          Custos does not currently expose cache keys, hit rates, or storage maintenance through
          the daemon API. No cache data is shown here, and this screen cannot delete or vacuum data.
        </p>
      </section>
    </main>
  );
};

export default CachePage;
