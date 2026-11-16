import React, { useState } from 'react';
import { Task, TaskStatus, ExecutionWorkspace } from '@/types/domain';
import { Session } from '@/types';
import { getPackInfo, getTaskBucket } from './types';
import {
  ArrowLeft,
  CircleDot,
  CheckCircle2,
  MessageSquare,
  FolderGit2,
  ListChecks,
  Plus,
  ExternalLink,
} from 'lucide-react';

interface TaskDetailViewProps {
  task: Task;
  sessions: Session[];
  workspaces: ExecutionWorkspace[];
  onBack: () => void;
  onOpenSession: (sessionId: string) => void;
  onNewSessionForTask: (taskId: string) => void;
  onAdvanceStatus: (taskId: string, status: TaskStatus) => Promise<void>;
  onCompleteTask: (taskId: string, summary?: string) => Promise<void>;
  onCancelTask: (taskId: string, reason?: string) => Promise<void>;
  onAttachWorkspace?: (taskId: string, workspaceId: string) => Promise<void>;
}

export const TaskDetailView: React.FC<TaskDetailViewProps> = ({
  task,
  sessions,
  workspaces,
  onBack,
  onOpenSession,
  onNewSessionForTask,
  onAdvanceStatus,
  onCompleteTask,
  onCancelTask,
  onAttachWorkspace,
}) => {
  const bucket = getTaskBucket(task.status);
  const packInfo = getPackInfo(task.contract?.pack_id || task.metadata?.pack_id);
  const criteria = task.contract?.criteria || [];
  const taskSessions = sessions.filter((s) => (s.task_id || s.taskId) === task.id);

  const [isUpdatingStatus, setIsUpdatingStatus] = useState(false);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string>(
    task.metadata?.workspace_id || ''
  );

  const boundWorkspace = workspaces.find(
    (w) => w.id === (task.metadata?.workspace_id || selectedWorkspaceId)
  );

  const handleStatusChange = async (newStatus: string) => {
    setIsUpdatingStatus(true);
    try {
      if (newStatus === 'Done' || newStatus === 'succeeded') {
        await onCompleteTask(task.id, 'Task verified and completed');
      } else if (newStatus === 'Cancelled' || newStatus === 'cancelled') {
        await onCancelTask(task.id, 'Cancelled by user');
      } else {
        await onAdvanceStatus(task.id, newStatus as TaskStatus);
      }
    } finally {
      setIsUpdatingStatus(false);
    }
  };

  const handleWorkspaceChange = async (wsId: string) => {
    setSelectedWorkspaceId(wsId);
    if (onAttachWorkspace) {
      await onAttachWorkspace(task.id, wsId);
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col overflow-y-auto pr-1 scrollbar-sleek">
      {/* Top Header Navigation */}
      <div className="flex flex-wrap items-center justify-between gap-3 pb-4 border-b border-border-default/80">
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={onBack}
            className="inline-flex items-center gap-1.5 rounded-lg border border-border-default bg-surface-1 px-3 py-1.5 text-xs font-medium text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition-colors"
          >
            <ArrowLeft className="size-3.5" />
            <span>Tasks</span>
          </button>

          <span className="font-mono text-xs text-fg-muted/70">#{task.id}</span>

          <span
            className={`inline-flex items-center rounded-md border px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide ${packInfo.badgeClass}`}
          >
            {packInfo.label}
          </span>
        </div>

        {/* Status Dropdown & Actions */}
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1.5 rounded-lg border border-border-default bg-surface-1 px-2.5 py-1 text-xs">
            <span className="text-[11px] text-fg-muted mr-1">Status:</span>
            <select
              value={String(task.status)}
              disabled={isUpdatingStatus}
              onChange={(e) => handleStatusChange(e.target.value)}
              className="bg-transparent font-medium text-fg-editor focus:outline-none cursor-pointer"
            >
              <option value="draft">Draft</option>
              <option value="running">In Progress (Active)</option>
              <option value="blocked">Needs Review (Blocked)</option>
              <option value="succeeded">Done (Succeeded)</option>
              <option value="cancelled">Cancelled</option>
            </select>
          </div>

          <button
            type="button"
            onClick={() => onNewSessionForTask(task.id)}
            className="inline-flex items-center gap-1.5 rounded-lg bg-workbench-accent px-3 py-1.5 text-xs font-semibold text-white shadow-xs hover:opacity-90 active:scale-98 transition-all"
          >
            <MessageSquare className="size-3.5" />
            <span>Start Conversation</span>
          </button>
        </div>
      </div>

      {/* Main 2-Column Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 pt-5">
        {/* Left Column: Spec, Criteria, Conversations (Span 2) */}
        <div className="lg:col-span-2 space-y-6">
          {/* Title & Description Box */}
          <div className="rounded-xl border border-border-default bg-surface-1 p-5 shadow-xs">
            <h1 className="text-xl font-semibold tracking-tight text-fg-editor">
              {task.title}
            </h1>

            {task.contract?.description ? (
              <div className="mt-4 text-xs leading-relaxed text-fg-muted whitespace-pre-wrap">
                {task.contract.description}
              </div>
            ) : (
              <p className="mt-3 text-xs italic text-fg-muted/60">
                No specification details provided for this task contract.
              </p>
            )}
          </div>

          {/* Acceptance Criteria Checklist (Orca Criteria Supervision) */}
          <div className="rounded-xl border border-border-default bg-surface-1 p-5 shadow-xs">
            <div className="flex items-center justify-between pb-3 border-b border-border-muted/50">
              <div className="flex items-center gap-2">
                <ListChecks className="size-4 text-workbench-accent" />
                <h2 className="text-xs font-semibold uppercase tracking-wider text-fg-editor">
                  Acceptance Criteria
                </h2>
              </div>
              <span className="font-mono text-[11px] text-fg-muted tabular-nums">
                {criteria.length} {criteria.length === 1 ? 'criterion' : 'criteria'}
              </span>
            </div>

            {criteria.length === 0 ? (
              <p className="py-4 text-xs text-fg-muted text-center italic">
                No acceptance criteria defined yet. Add criteria to verify task outcome.
              </p>
            ) : (
              <ul className="divide-y divide-border-muted/30 pt-1">
                {criteria.map((c, i) => (
                  <li
                    key={`${c.name}-${i}`}
                    className="flex items-start justify-between gap-3 py-3 text-xs"
                  >
                    <div className="flex items-start gap-2.5 min-w-0">
                      <div className="mt-0.5">
                        {bucket === 'done' ? (
                          <CheckCircle2 className="size-4 text-emerald-400" />
                        ) : (
                          <CircleDot className="size-4 text-fg-muted" />
                        )}
                      </div>
                      <div className="space-y-0.5">
                        <div className="font-medium text-fg-editor">{c.name}</div>
                        {c.description && (
                          <div className="text-[11px] text-fg-muted">{c.description}</div>
                        )}
                      </div>
                    </div>

                    <span
                      className={`shrink-0 rounded px-1.5 py-0.5 text-[9px] font-mono uppercase tracking-wider ${
                        c.mandatory
                          ? 'bg-amber-500/10 text-amber-400 border border-amber-500/30'
                          : 'bg-surface-3 text-fg-muted'
                      }`}
                    >
                      {c.mandatory ? 'Required' : 'Optional'}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </div>

          {/* Linked Conversations Section (Orca Continuity Trace) */}
          <div className="rounded-xl border border-border-default bg-surface-1 p-5 shadow-xs">
            <div className="flex items-center justify-between pb-3 border-b border-border-muted/50">
              <div className="flex items-center gap-2">
                <MessageSquare className="size-4 text-workbench-accent" />
                <h2 className="text-xs font-semibold uppercase tracking-wider text-fg-editor">
                  Conversations
                </h2>
              </div>
              <button
                type="button"
                onClick={() => onNewSessionForTask(task.id)}
                className="inline-flex items-center gap-1 text-[11px] font-medium text-workbench-accent hover:underline"
              >
                <Plus className="size-3" />
                <span>New Conversation</span>
              </button>
            </div>

            {taskSessions.length === 0 ? (
              <div className="py-6 text-center">
                <p className="text-xs text-fg-muted">No conversation has been attached to this task yet.</p>
                <button
                  type="button"
                  onClick={() => onNewSessionForTask(task.id)}
                  className="mt-3 inline-flex items-center gap-1.5 rounded-lg border border-border-default bg-surface-2 px-3 py-1.5 text-xs font-medium text-fg-editor hover:border-border-emphasis transition-colors"
                >
                  <MessageSquare className="size-3.5" />
                  <span>Start First Conversation</span>
                </button>
              </div>
            ) : (
              <div className="divide-y divide-border-muted/30 pt-1">
                {taskSessions.map((session) => (
                  <div
                    key={session.id}
                    onClick={() => onOpenSession(session.id)}
                    className="group flex items-center justify-between gap-3 py-3 cursor-pointer hover:bg-surface-2/60 rounded-lg px-2.5 transition-colors"
                  >
                    <div className="flex items-center gap-2.5 min-w-0">
                      <MessageSquare className="size-3.5 text-workbench-accent shrink-0" />
                      <div className="truncate">
                        <div className="text-xs font-medium text-fg-editor group-hover:text-workbench-accent truncate">
                          {session.title || 'Untitled Session'}
                        </div>
                        <div className="text-[10px] text-fg-muted font-mono">
                          {session.id}
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center gap-2 shrink-0">
                      <span className="text-[10px] text-fg-muted">
                        {new Date(session.created_at || session.time || Date.now()).toLocaleDateString(undefined, {
                          month: 'short',
                          day: 'numeric',
                        })}
                      </span>
                      <span className="inline-flex items-center gap-1 rounded bg-surface-3 px-2 py-0.5 text-[11px] text-fg-editor group-hover:bg-workbench-accent group-hover:text-white transition-colors">
                        <span>Open</span>
                        <ExternalLink className="size-2.5" />
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Right Column: Execution Workspace, Metadata, Budget (Span 1) */}
        <div className="space-y-6">
          {/* Execution Workspace Card */}
          <div className="rounded-xl border border-border-default bg-surface-1 p-4 shadow-xs space-y-3">
            <div className="flex items-center gap-2 text-xs font-semibold text-fg-editor">
              <FolderGit2 className="size-4 text-workbench-accent" />
              <span>Execution Workspace</span>
            </div>

            <div className="space-y-1.5">
              <label className="text-[10px] uppercase tracking-wider text-fg-muted">Bound Worktree / Repo</label>
              <select
                value={selectedWorkspaceId}
                onChange={(e) => handleWorkspaceChange(e.target.value)}
                className="w-full rounded-lg border border-border-default bg-surface-2 px-2.5 py-1.5 text-xs text-fg-editor focus:border-border-emphasis focus:outline-none"
              >
                <option value="">(No specific workspace)</option>
                {workspaces.map((w) => {
                  const br = w.active_branch || w.activeBranch || w.branch;
                  return (
                    <option key={w.id} value={w.id}>
                      {w.name} {br ? `(${br})` : ''}
                    </option>
                  );
                })}
              </select>
            </div>

            {boundWorkspace && (
              <div className="rounded-lg bg-surface-2 p-2.5 text-[11px] space-y-1">
                <div className="flex items-center justify-between text-fg-muted">
                  <span>Branch:</span>
                  <span className="font-mono text-fg-editor">
                    {boundWorkspace.active_branch || boundWorkspace.activeBranch || boundWorkspace.branch || 'main'}
                  </span>
                </div>
                <div className="flex items-center justify-between text-fg-muted">
                  <span>Base commit:</span>
                  <span className="font-mono text-fg-editor">
                    {boundWorkspace.base_commit_hash?.slice(0, 7) || 'HEAD'}
                  </span>
                </div>
              </div>
            )}
          </div>

          {/* Budget & Economics */}
          <div className="rounded-xl border border-border-default bg-surface-1 p-4 shadow-xs space-y-3">
            <h3 className="text-xs font-semibold text-fg-editor">Budget & Governance</h3>
            <div className="space-y-2 text-xs">
              <div className="flex items-center justify-between text-fg-muted">
                <span>Token Headroom:</span>
                <span className="font-mono text-fg-editor">
                  {task.contract?.budget_tokens
                    ? `${(task.contract.budget_tokens / 1000).toFixed(0)}k tokens`
                    : 'Uncapped'}
                </span>
              </div>
              <div className="flex items-center justify-between text-fg-muted">
                <span>Max Cost:</span>
                <span className="font-mono text-fg-editor">
                  {task.contract?.max_cost_usd ? `$${task.contract.max_cost_usd}` : 'Default policy'}
                </span>
              </div>
            </div>
          </div>

          {/* Task Info & Timestamps */}
          <div className="rounded-xl border border-border-default bg-surface-1 p-4 shadow-xs space-y-2 text-[11px] text-fg-muted">
            <div className="flex items-center justify-between">
              <span>Created:</span>
              <span className="text-fg-editor tabular-nums">
                {new Date(task.created_at).toLocaleString()}
              </span>
            </div>
            <div className="flex items-center justify-between">
              <span>Updated:</span>
              <span className="text-fg-editor tabular-nums">
                {new Date(task.updated_at || task.created_at).toLocaleString()}
              </span>
            </div>
          </div>

          {/* Danger zone: Cancel Task */}
          {task.status !== 'cancelled' && task.status !== 'Cancelled' && (
            <div className="pt-2">
              <button
                type="button"
                onClick={() => onCancelTask(task.id, 'User cancelled task')}
                className="w-full rounded-lg border border-red-500/20 bg-red-500/5 px-3 py-2 text-xs font-medium text-red-400 hover:bg-red-500/10 hover:border-red-500/30 transition-colors"
              >
                Cancel This Task
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
