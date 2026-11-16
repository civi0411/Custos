import React from 'react';
import { TaskViewMode, TaskFilterOptions } from './types';
import {
  Search,
  LayoutGrid,
  List,
  Plus,
  RefreshCw,
  X,
} from 'lucide-react';

interface TaskToolbarProps {
  filters: TaskFilterOptions;
  onChangeFilters: (filters: TaskFilterOptions) => void;
  viewMode: TaskViewMode;
  onChangeViewMode: (mode: TaskViewMode) => void;
  onCreateTask: () => void;
  onRefresh: () => void;
  isRefreshing?: boolean;
  totalTasks: number;
}

export const TaskToolbar: React.FC<TaskToolbarProps> = ({
  filters,
  onChangeFilters,
  viewMode,
  onChangeViewMode,
  onCreateTask,
  onRefresh,
  isRefreshing = false,
  totalTasks,
}) => {
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 pb-3 border-b border-border-default/80">
      {/* Left side: Search & Pack Filter */}
      <div className="flex flex-wrap items-center gap-2.5 flex-1 min-w-[280px]">
        {/* Search Input */}
        <div className="relative min-w-[200px] max-w-xs flex-1">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-fg-muted" />
          <input
            type="text"
            value={filters.query}
            onChange={(e) => onChangeFilters({ ...filters, query: e.target.value })}
            placeholder="Search tasks, specs..."
            className="w-full rounded-lg border border-border-default bg-surface-1 py-1.5 pl-8 pr-7 text-xs text-fg-editor placeholder:text-fg-muted/60 focus:border-border-emphasis focus:outline-none focus:ring-1 focus:ring-workbench-accent"
          />
          {filters.query && (
            <button
              type="button"
              onClick={() => onChangeFilters({ ...filters, query: '' })}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-fg-muted hover:text-fg-editor"
            >
              <X className="size-3.5" />
            </button>
          )}
        </div>

        {/* Pack Selector Filter */}
        <div className="flex items-center rounded-lg border border-border-default bg-surface-1 p-0.5 text-xs">
          {(
            [
              { id: 'all', label: 'All Packs' },
              { id: 'code', label: 'Engineering' },
              { id: 'research', label: 'Research' },
              { id: 'copilot', label: 'Assistant' },
            ] as const
          ).map((pack) => (
            <button
              key={pack.id}
              type="button"
              onClick={() => onChangeFilters({ ...filters, packId: pack.id })}
              className={`rounded-md px-2.5 py-1 text-[11px] font-medium transition-colors ${
                filters.packId === pack.id
                  ? 'bg-surface-3 text-fg-editor shadow-xs'
                  : 'text-fg-muted hover:text-fg-editor'
              }`}
            >
              {pack.label}
            </button>
          ))}
        </div>

        {/* Status Dropdown */}
        <select
          value={filters.status}
          onChange={(e) =>
            onChangeFilters({ ...filters, status: e.target.value as TaskFilterOptions['status'] })
          }
          className="rounded-lg border border-border-default bg-surface-1 px-2.5 py-1.5 text-xs text-fg-editor focus:border-border-emphasis focus:outline-none"
        >
          <option value="all">All Statuses</option>
          <option value="draft">Backlog / Draft</option>
          <option value="running">In Progress</option>
          <option value="blocked">Needs Review</option>
          <option value="done">Completed</option>
        </select>
      </div>

      {/* Right side: View Switcher, Total count, New Task */}
      <div className="flex items-center gap-2">
        <span className="text-[11px] font-mono text-fg-muted mr-1 hidden sm:inline">
          {totalTasks} {totalTasks === 1 ? 'task' : 'tasks'}
        </span>

        {/* View Mode Toggle */}
        <div className="flex items-center rounded-lg border border-border-default bg-surface-1 p-0.5 text-xs">
          <button
            type="button"
            onClick={() => onChangeViewMode('board')}
            title="Board view"
            className={`rounded-md p-1.5 text-xs transition-colors ${
              viewMode === 'board'
                ? 'bg-surface-3 text-workbench-accent shadow-xs'
                : 'text-fg-muted hover:text-fg-editor'
            }`}
          >
            <LayoutGrid className="size-3.5" />
          </button>
          <button
            type="button"
            onClick={() => onChangeViewMode('list')}
            title="List view"
            className={`rounded-md p-1.5 text-xs transition-colors ${
              viewMode === 'list'
                ? 'bg-surface-3 text-workbench-accent shadow-xs'
                : 'text-fg-muted hover:text-fg-editor'
            }`}
          >
            <List className="size-3.5" />
          </button>
        </div>

        {/* Refresh button */}
        <button
          type="button"
          onClick={onRefresh}
          disabled={isRefreshing}
          title="Refresh tasks"
          className="rounded-lg border border-border-default bg-surface-1 p-1.5 text-xs text-fg-muted hover:text-fg-editor hover:bg-surface-2 transition-colors disabled:opacity-50"
        >
          <RefreshCw className={`size-3.5 ${isRefreshing ? 'animate-spin' : ''}`} />
        </button>

        {/* + New Task button */}
        <button
          type="button"
          onClick={onCreateTask}
          className="inline-flex items-center gap-1.5 rounded-lg bg-workbench-accent px-3 py-1.5 text-xs font-semibold text-white shadow-xs hover:opacity-90 active:scale-98 transition-all"
        >
          <Plus className="size-3.5" />
          <span>New Task</span>
        </button>
      </div>
    </div>
  );
};
