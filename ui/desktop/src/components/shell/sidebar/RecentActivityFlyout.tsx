import React, { useState, useMemo } from 'react';
import { Clock, X, MessageSquare, CheckSquare, Search, Trash2 } from 'lucide-react';
import type { Session } from '@/types';
import type { Task } from '@/types/domain';

export interface RecentItem {
  id: string;
  kind: 'chat' | 'task';
  title: string;
  status?: string;
  timeAgo: string;
  timestamp: number;
  sessionId?: string;
  taskId?: string;
}

export function buildRecentItems(
  allSessions: Session[],
  tasks: Task[],
  formatRelativeTime: (iso?: string) => string
): RecentItem[] {
  const list: RecentItem[] = [];
  allSessions.forEach((s) => {
    const isRunning =
      s.taskStatus === 'running' || s.messages?.some((m) => m.runStatus === 'running' || m.runStatus === 'queued');
    const isBlocked =
      s.taskStatus === 'blocked' || s.messages?.some((m) => m.runStatus === 'blocked');
    list.push({
      id: s.id,
      kind: 'chat',
      title: s.title || s.preview || 'Untitled conversation',
      status: isRunning ? 'running' : isBlocked ? 'blocked' : 'completed',
      timeAgo: s.time || '',
      timestamp: (s as any).updatedAt || 0,
      sessionId: s.id,
    });
  });
  tasks.forEach((t) => {
    const isRunning =
      t.status === 'running' ||
      t.status === 'Active' ||
      String(t.status).toLowerCase() === 'running';
    const isBlocked =
      t.status === 'Blocked' ||
      String(t.status).toLowerCase() === 'blocked' ||
      t.metadata?.approval_required === 'true';
    list.push({
      id: t.id,
      kind: 'task',
      title: t.title || t.id,
      status: isRunning ? 'running' : isBlocked ? 'blocked' : 'completed',
      timeAgo: formatRelativeTime(t.updated_at || t.created_at),
      timestamp: t.updated_at ? new Date(t.updated_at).getTime() : 0,
      taskId: t.id,
    });
  });
  return list.sort((a, b) => b.timestamp - a.timestamp);
}

export interface RecentActivityFlyoutProps {
  isOpen: boolean;
  onClose: () => void;
  allSessions: Session[];
  tasks: Task[];
  formatRelativeTime: (iso?: string) => string;
  activeSessionId?: string;
  onSelectSession?: (id: string) => void;
  onSelectTask?: (id: string) => void;
  onDeleteSession?: (id: string) => void | Promise<void>;
}

export const RecentActivityFlyout: React.FC<RecentActivityFlyoutProps> = ({
  isOpen,
  onClose,
  allSessions,
  tasks,
  formatRelativeTime,
  activeSessionId,
  onSelectSession,
  onSelectTask,
  onDeleteSession,
}) => {
  const [recentFilter, setRecentFilter] = useState<'all' | 'chats' | 'tasks'>('all');
  const [recentSearch, setRecentSearch] = useState('');

  const recentItems = useMemo(
    () => buildRecentItems(allSessions, tasks, formatRelativeTime),
    [allSessions, tasks, formatRelativeTime]
  );

  if (!isOpen) return null;

  const filteredRecentItems = recentItems.filter((item) => {
    if (recentFilter === 'chats' && item.kind !== 'chat') return false;
    if (recentFilter === 'tasks' && item.kind !== 'task') return false;
    if (recentSearch.trim()) {
      const query = recentSearch.toLowerCase();
      return item.title.toLowerCase().includes(query) || item.id.toLowerCase().includes(query);
    }
    return true;
  });

  const handleSelectRecentItem = (item: RecentItem) => {
    if (item.sessionId) {
      onSelectSession?.(item.sessionId);
    } else if (item.taskId) {
      onSelectTask?.(item.taskId);
    }
    onClose();
  };

  const handleDeleteRecentItem = async (e: React.MouseEvent, item: RecentItem) => {
    e.stopPropagation();
    if (item.sessionId && onDeleteSession) {
      await onDeleteSession(item.sessionId);
    }
  };

  return (
    <div className="absolute inset-x-0 top-[41px] bottom-9 z-40 bg-[#141518]/98 backdrop-blur-md flex flex-col border-b border-white/8 shadow-2xl animate-in fade-in duration-150">
      {/* Header of Recent Activity */}
      <div className="p-2.5 pb-2 border-b border-white/6 flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <Clock className="size-3.5 text-white/60" />
          <span className="text-[11px] font-semibold text-white/90">Recent Activity</span>
          <span className="text-[9px] font-mono px-1.5 py-0.2 rounded-full bg-white/8 text-white/50">
            {recentItems.length}
          </span>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="p-1 text-white/40 hover:text-white hover:bg-white/8 rounded transition cursor-pointer"
          aria-label="Close recent activity"
        >
          <X className="size-3.5" />
        </button>
      </div>

      {/* Quick Filters: All | Chats | Tasks */}
      <div className="px-2.5 pt-2 pb-1.5 flex items-center gap-1">
        <button
          type="button"
          onClick={() => setRecentFilter('all')}
          className={`px-2 py-0.5 rounded text-[10.5px] transition cursor-pointer font-medium ${
            recentFilter === 'all'
              ? 'bg-white/12 text-white'
              : 'text-white/40 hover:text-white/70'
          }`}
        >
          All ({recentItems.length})
        </button>
        <button
          type="button"
          onClick={() => setRecentFilter('chats')}
          className={`px-2 py-0.5 rounded text-[10.5px] transition cursor-pointer font-medium flex items-center gap-1 ${
            recentFilter === 'chats'
              ? 'bg-white/12 text-white'
              : 'text-white/40 hover:text-white/70'
          }`}
        >
          <MessageSquare className="size-2.5" />
          <span>Chats ({recentItems.filter((i) => i.kind === 'chat').length})</span>
        </button>
        <button
          type="button"
          onClick={() => setRecentFilter('tasks')}
          className={`px-2 py-0.5 rounded text-[10.5px] transition cursor-pointer font-medium flex items-center gap-1 ${
            recentFilter === 'tasks'
              ? 'bg-white/12 text-white'
              : 'text-white/40 hover:text-white/70'
          }`}
        >
          <CheckSquare className="size-2.5" />
          <span>Tasks ({recentItems.filter((i) => i.kind === 'task').length})</span>
        </button>
      </div>

      {/* Search Input for Recent Activity */}
      <div className="px-2.5 pb-2">
        <div className="relative flex items-center">
          <Search className="size-3 text-white/35 absolute left-2 pointer-events-none" />
          <input
            type="text"
            value={recentSearch}
            onChange={(e) => setRecentSearch(e.target.value)}
            placeholder="Search recent activity..."
            className="w-full bg-[#18191d] border border-white/8 rounded-md pl-6 pr-6 py-1 text-[11px] text-white placeholder-white/30 focus:outline-hidden focus:border-white/20 transition"
          />
          {recentSearch && (
            <button
              type="button"
              onClick={() => setRecentSearch('')}
              className="absolute right-2 text-white/30 hover:text-white/70"
            >
              ✕
            </button>
          )}
        </div>
      </div>

      {/* List of Recent Items */}
      <div className="flex-1 overflow-y-auto no-scrollbar p-2 space-y-1">
        {filteredRecentItems.length === 0 ? (
          <div className="p-4 text-center text-white/40 text-xs">
            No recent activity found
          </div>
        ) : (
          filteredRecentItems.map((item) => {
            const isActive = item.sessionId === activeSessionId;
            const isRunning = item.status === 'running';
            const isBlocked = item.status === 'blocked';

            return (
              <div
                key={`${item.kind}-${item.id}`}
                onClick={() => handleSelectRecentItem(item)}
                className={`group flex items-center justify-between px-2.5 py-1.5 rounded-lg cursor-pointer transition text-xs ${
                  isActive
                    ? 'bg-white/12 text-white font-medium border border-white/10 shadow-xs'
                    : 'text-white/70 hover:text-white hover:bg-white/5 border border-transparent'
                }`}
              >
                <div className="flex items-center gap-2 min-w-0 flex-1 mr-1">
                  <span
                    className={`size-1.5 rounded-full shrink-0 ${
                      isRunning
                        ? 'bg-emerald-400 animate-pulse'
                        : isBlocked
                          ? 'bg-amber-400'
                          : 'bg-white/25'
                    }`}
                  />
                  {item.kind === 'chat' ? (
                    <MessageSquare className="size-3 text-white/40 shrink-0" />
                  ) : (
                    <CheckSquare className="size-3 text-white/40 shrink-0" />
                  )}
                  <span className="truncate">{item.title}</span>
                </div>

                <div className="flex items-center gap-1.5 shrink-0">
                  <span className="text-[10px] text-white/30 font-mono">
                    {item.timeAgo}
                  </span>
                  {item.sessionId && onDeleteSession && (
                    <button
                      type="button"
                      onClick={(e) => handleDeleteRecentItem(e, item)}
                      className="opacity-0 group-hover:opacity-100 p-0.5 rounded text-white/40 hover:text-white hover:bg-white/10 transition cursor-pointer"
                      title="Delete session"
                    >
                      <Trash2 className="size-3" />
                    </button>
                  )}
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};
