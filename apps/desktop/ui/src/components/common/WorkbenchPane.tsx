import type { ReactNode } from 'react';
import { AlertCircle, Loader2 } from 'lucide-react';
import type { WorkbenchPaneProps } from '@/types/workbench';

export function WorkbenchPane({
  title,
  paneId: _paneId,
  toolbar,
  statusBar,
  isLoading = false,
  error = null,
  emptyState = null,
  children,
  className = '',
}: WorkbenchPaneProps) {
  return (
    <section className={`wb-pane ${className}`}>
      {(title || toolbar) && (
        <header className="wb-pane-toolbar">
          {title && <span className="wb-pane-title font-medium text-xs text-neutral-300">{title}</span>}
          {toolbar}
        </header>
      )}
      {error ? (
        <WorkbenchError message={error} />
      ) : isLoading ? (
        <div className="wb-empty" role="status">
          <Loader2 size={18} className="animate-spin text-fg-muted" aria-hidden="true" />
          <span>Loading...</span>
        </div>
      ) : emptyState ? (
        emptyState
      ) : (
        children
      )}
      {statusBar && (
        <footer className="wb-pane-statusbar text-2xs text-neutral-500 border-t border-border-subtle px-3 py-1">
          {statusBar}
        </footer>
      )}
    </section>
  );
}

export function WorkbenchEmptyState({
  title,
  description,
  action,
  busy = false,
}: {
  title: string;
  description?: ReactNode;
  action?: ReactNode;
  busy?: boolean;
}) {
  return (
    <div className="wb-empty" role={busy ? 'status' : undefined}>
      {busy && <Loader2 size={18} className="animate-spin text-fg-muted" aria-hidden="true" />}
      <h2>{title}</h2>
      {description && <p>{description}</p>}
      {action && <div className="wb-empty-actions">{action}</div>}
    </div>
  );
}

export function WorkbenchError({
  message,
  onRetry,
}: {
  message: string | null;
  onRetry?: () => void;
}) {
  if (!message) return null;
  return (
    <div className="wb-error" role="alert">
      <AlertCircle size={14} className="shrink-0" aria-hidden="true" />
      <span className="min-w-0 flex-1">{message}</span>
      {onRetry && (
        <button type="button" className="canvas-open-button shrink-0" onClick={onRetry}>
          Retry
        </button>
      )}
    </div>
  );
}
