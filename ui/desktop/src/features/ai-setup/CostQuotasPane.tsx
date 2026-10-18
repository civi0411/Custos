import React from 'react';
import { UsageMonitor } from '@/components/providers/UsageMonitor';

/** Usage is daemon-backed; an unenforced budget is not a setting. */
export const CostQuotasPane: React.FC = () => (
  <div className="space-y-4">
    <div className="rounded-lg border border-border-default bg-surface-1 p-4">
      <h2 className="text-sm font-semibold text-fg-editor">Usage and provider limits</h2>
      <p className="mt-1 text-xs leading-5 text-fg-muted">
        Recorded Custos attempts and provider-reported quota snapshots appear below. Missing usage remains unknown.
        Custos does not currently enforce a global or per-workbench token ceiling; no editable quota is shown here.
      </p>
    </div>
    <UsageMonitor />
  </div>
);
