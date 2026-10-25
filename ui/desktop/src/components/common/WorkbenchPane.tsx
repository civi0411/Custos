import type { ReactNode } from 'react';
import { AlertCircle, Loader2 } from 'lucide-react';

export function WorkbenchPane({ toolbar, children, className = '' }: { toolbar?: ReactNode; children: ReactNode; className?: string }) {
  return <section className={`wb-pane ${className}`}>
    {toolbar && <header className="wb-pane-toolbar">{toolbar}</header>}
    {children}
  </section>;
}

export function WorkbenchEmptyState({ title, description, action, busy = false }: { title: string; description?: ReactNode; action?: ReactNode; busy?: boolean }) {
  return <div className="wb-empty" role={busy ? 'status' : undefined}>
    {busy && <Loader2 size={18} className="animate-spin text-fg-muted" aria-hidden="true" />}
    <h2>{title}</h2>
    {description && <p>{description}</p>}
    {action && <div className="wb-empty-actions">{action}</div>}
  </div>;
}

export function WorkbenchError({ message, onRetry }: { message: string | null; onRetry?: () => void }) {
  if (!message) return null;
  return <div className="wb-error" role="alert">
    <AlertCircle size={14} className="shrink-0" aria-hidden="true" />
    <span className="min-w-0 flex-1">{message}</span>
    {onRetry && <button type="button" className="canvas-open-button shrink-0" onClick={onRetry}>Retry</button>}
  </div>;
}
