import React from 'react';
import { Task } from '@/types/domain';
import { getPackInfo, getTaskBucket } from './types';
import {
  CircleDot,
  CheckCircle2,
  AlertCircle,
  Clock,
  MessageSquare,
  FolderGit2,
  ListChecks,
} from 'lucide-react';

interface TaskCardProps {
  task: Task;
  linkedSessionCount?: number;
  workspaceName?: string;
  onClick: () => void;
}

export const TaskCard: React.FC<TaskCardProps> = ({
  task,
  linkedSessionCount = 0,
  workspaceName,
  onClick,
}) => {
  const bucket = getTaskBucket(task.status);
  const packInfo = getPackInfo(task.contract?.pack_id || task.metadata?.pack_id);
  const criteria = task.contract?.criteria || [];
  const totalCriteria = criteria.length;

  return (
    <div
      onClick={onClick}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onClick();
        }
      }}
      className="group relative flex flex-col gap-2.5 rounded-xl border border-border-default bg-surface-1 p-3.5 text-left transition-all duration-150 hover:border-border-emphasis hover:bg-surface-2 hover:shadow-md cursor-pointer select-none"
    >
      {/* Top row: Pack badge & status icon */}
      <div className="flex items-center justify-between gap-2">
        <span
          className={`inline-flex items-center rounded-md border px-2 py-0.5 text-[10px] font-medium tracking-wide uppercase ${packInfo.badgeClass}`}
        >
          {packInfo.label}
        </span>

        <div className="flex items-center gap-1 text-[11px]">
          {bucket === 'running' && (
            <span className="flex items-center gap-1 text-sky-400 font-medium">
              <span className="relative flex h-2 w-2">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-sky-400 opacity-75" />
                <span className="relative inline-flex rounded-full h-2 w-2 bg-sky-500" />
              </span>
              Active
            </span>
          )}
          {bucket === 'attention' && (
            <span className="flex items-center gap-1 text-amber-400 font-medium">
              <AlertCircle className="size-3 text-amber-400" />
              Review
            </span>
          )}
          {bucket === 'done' && (
            <span className="flex items-center gap-1 text-emerald-400 font-medium">
              <CheckCircle2 className="size-3 text-emerald-400" />
              Done
            </span>
          )}
          {bucket === 'draft' && (
            <span className="flex items-center gap-1 text-fg-muted">
              <CircleDot className="size-3 text-fg-muted" />
              Draft
            </span>
          )}
        </div>
      </div>

      {/* Task Title */}
      <h3 className="text-xs font-semibold leading-snug text-fg-editor line-clamp-2 group-hover:text-workbench-accent">
        {task.title}
      </h3>

      {/* Description Snippet */}
      {task.contract?.description && (
        <p className="text-[11px] leading-relaxed text-fg-muted line-clamp-2">
          {task.contract.description}
        </p>
      )}

      {/* Criteria Progress */}
      {totalCriteria > 0 && (
        <div className="flex items-center gap-1.5 pt-1 text-[11px] text-fg-muted border-t border-border-muted/50">
          <ListChecks className="size-3.5 text-fg-muted shrink-0" />
          <span className="tabular-nums font-mono text-[10px]">
            {totalCriteria} {totalCriteria === 1 ? 'criterion' : 'criteria'}
          </span>
          <div className="ml-auto flex items-center gap-1">
            {criteria.some((c) => c.mandatory) && (
              <span className="text-[9px] px-1 py-0.2 rounded bg-surface-3 text-fg-muted font-mono">
                Mandatory
              </span>
            )}
          </div>
        </div>
      )}

      {/* Card Footer: Linked sessions, Workspace, Timestamp */}
      <div className="flex items-center justify-between gap-2 pt-1 text-[10px] text-fg-muted/80 border-t border-border-muted/40">
        <div className="flex items-center gap-2 truncate">
          {linkedSessionCount > 0 ? (
            <span className="inline-flex items-center gap-1 text-fg-muted hover:text-fg-editor">
              <MessageSquare className="size-3 shrink-0" />
              <span>{linkedSessionCount}</span>
            </span>
          ) : (
            <span className="inline-flex items-center gap-1 opacity-60">
              <MessageSquare className="size-3 shrink-0" />
              <span>0</span>
            </span>
          )}

          {workspaceName && (
            <span className="inline-flex items-center gap-1 truncate max-w-[120px]" title={workspaceName}>
              <FolderGit2 className="size-3 shrink-0 text-fg-muted" />
              <span className="truncate">{workspaceName}</span>
            </span>
          )}
        </div>

        <div className="flex items-center gap-1 shrink-0">
          <Clock className="size-2.5 opacity-60" />
          <span className="tabular-nums opacity-80">
            {new Date(task.updated_at || task.created_at).toLocaleDateString(undefined, {
              month: 'short',
              day: 'numeric',
            })}
          </span>
        </div>
      </div>
    </div>
  );
};
