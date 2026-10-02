import React, { useState } from 'react';
import { Layers, Plus, ChevronLeft, Search, Inbox } from 'lucide-react';
import { Session } from '../types';

interface SessionsSidebarProps {
  currentProject: string;
  sessions: Session[];
  activeSessionId: string;
  onSelectSession: (id: string) => void;
  onOpenNewSessionModal: () => void;
  isCollapsed: boolean;
  onToggleCollapse: () => void;
}

export const SessionsSidebar: React.FC<SessionsSidebarProps> = ({
  currentProject,
  sessions,
  activeSessionId,
  onSelectSession,
  onOpenNewSessionModal,
  isCollapsed,
  onToggleCollapse
}) => {
  const [searchQuery, setSearchQuery] = useState('');

  const filteredSessions = sessions.filter(
    (s) =>
      s.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      s.preview.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <aside 
      className={`bg-surface border-r border-surface-border flex flex-col shrink-0 transition-all duration-200 overflow-hidden min-w-0 ${
        isCollapsed 
          ? 'w-0 border-r-0 opacity-0 p-0 pointer-events-none' 
          : 'w-60 lg:w-64'
      }`}
    >
      {/* Sessions Column Header */}
      <div className="p-3 border-b border-surface-border flex items-center justify-between min-w-0">
        <div className="flex items-center gap-2 min-w-0">
          <Layers className="w-3.5 h-3.5 text-brand-blue shrink-0" />
          <span className="text-xs font-semibold text-neutral-200 truncate">
            {currentProject}
          </span>
        </div>
        <div className="flex items-center gap-1 shrink-0">
          <button 
            onClick={onOpenNewSessionModal} 
            className="w-6 h-6 rounded-md bg-surface-card hover:bg-surface-elevated border border-surface-border flex items-center justify-center text-neutral-400 hover:text-white transition" 
            title="New Session"
          >
            <Plus className="w-3.5 h-3.5" />
          </button>
          <button 
            onClick={onToggleCollapse} 
            className="w-6 h-6 rounded-md hover:bg-surface-elevated text-neutral-500 hover:text-neutral-300 flex items-center justify-center transition lg:hidden" 
            title="Collapse Sessions"
          >
            <ChevronLeft className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Search Input */}
      <div className="p-2 border-b border-surface-border/60">
        <div className="relative">
          <Search className="w-3 h-3 absolute left-2.5 top-1/2 -translate-y-1/2 text-neutral-500" />
          <input 
            type="text" 
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search sessions..." 
            className="w-full bg-surface-card border border-surface-border/80 rounded-md pl-7 pr-2 py-1 text-[11px] text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-brand-blue transition"
          />
        </div>
      </div>

      {/* Scrollable List of Sessions */}
      <div className="flex-1 overflow-y-auto p-2 space-y-1 min-w-0">
        {filteredSessions.length === 0 ? (
          <div className="p-6 text-center text-neutral-500 text-xs">
            <Inbox className="w-6 h-6 mx-auto mb-2 opacity-50" />
            <p>No sessions found</p>
          </div>
        ) : (
          filteredSessions.map((session) => {
            const isActive = session.id === activeSessionId;
            return (
              <div 
                key={session.id}
                onClick={() => onSelectSession(session.id)}
                className={`p-2.5 rounded-xl cursor-pointer text-xs transition border min-w-0 ${
                  isActive 
                    ? 'bg-surface-elevated border-brand-blue/50 shadow-md text-white' 
                    : 'bg-surface-card hover:bg-surface-elevated/70 border-surface-border text-neutral-300'
                }`}
              >
                <div className="flex items-center justify-between mb-1 min-w-0">
                  <span className={`font-semibold truncate ${isActive ? 'text-white' : 'text-neutral-200'}`}>
                    {session.title}
                  </span>
                  <span className="text-[10px] text-neutral-500 font-mono shrink-0 ml-1.5">
                    {session.time}
                  </span>
                </div>
                <p className="text-[11px] text-neutral-400 truncate leading-snug">
                  {session.preview}
                </p>
                <div className="flex items-center justify-between mt-2 pt-1.5 border-t border-surface-border/50 text-[10px] text-neutral-500 font-mono">
                  <span className="truncate max-w-[120px]">{session.model}</span>
                  <span className="text-emerald-400 shrink-0">{session.diffLinesCount}</span>
                </div>
              </div>
            );
          })
        )}
      </div>

      {/* Sessions Column Footer: Active Gateway Status */}
      <div className="p-2.5 border-t border-surface-border bg-black/40 flex items-center justify-between text-[11px] font-mono text-neutral-500 shrink-0">
        <div className="flex items-center gap-1.5 truncate">
          <span className="w-2 h-2 rounded-full bg-emerald-500 shrink-0 animate-pulse"></span>
          <span className="truncate">127.0.0.1:8045</span>
        </div>
        <span className="text-neutral-400 font-mono shrink-0 ml-1">18ms</span>
      </div>
    </aside>
  );
};
