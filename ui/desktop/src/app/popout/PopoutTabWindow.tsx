import React from 'react';

// A separate renderer cannot safely recover scope or mutable state from a tab ID.
export const PopoutTabWindow: React.FC = () => (
  <main className="flex h-screen items-center justify-center bg-canvas p-8 text-ink-1">
    <section className="max-w-md space-y-3" aria-labelledby="detached-title">
      <h1 id="detached-title" className="text-lg font-semibold">Open this resource in the main workspace</h1>
      <p className="text-sm text-ink-3">Detached editing is unavailable until windows can share the same resource scope and reconcile changes. Your resource remains available in the main workspace.</p>
      <a href="/studio" className="inline-block rounded border border-line-2 px-3 py-2 text-sm">Open workspace</a>
    </section>
  </main>
);
