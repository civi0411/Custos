/**
 * WorkspaceTabBar — SADE Lens Switcher + Task Strip + Pane Tabs
 *
 * Three focused concerns, clearly separated:
 *   1. TaskStrip  — persistent task anchor (top banner)
 *   2. LensSwitcher — Copilot | Coding | Research tabs
 *   3. PaneTabStrip — open pane tabs within current lens
 */

import React, { useState } from 'react';
import {
  MessageSquare,
  Terminal,
  FlaskConical,
  FileDiff,
  Globe,
  FileText,
  Bot,
  PanelRight,
  Plus,
  X,
  ChevronDown,
  Columns2,
  Square,
} from 'lucide-react';
import type { WorkspacePaneType, WorkbenchLens } from '@/types';

export type WorkspacePane = { id: WorkspacePaneType; title: string };
export type LayoutMode = 'single' | 'split-2' | 'split-3' | 'split-4' | 'split-5';

/* ─────────────────────────────────────────────────────────────
   Lens definitions
───────────────────────────────────────────────────────────── */

interface LensDef {
  id: WorkbenchLens;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  cssClass: string;
  description: string;
}

const LENSES: LensDef[] = [
  {
    id: 'copilot',
    label: 'Claude Chat',
    icon: MessageSquare,
    cssClass: 'active-copilot',
    description: 'Conversation & assistant actions (Claude Desktop)',
  },
  {
    id: 'coding',
    label: 'Claude Code',
    icon: Terminal,
    cssClass: 'active-coding',
    description: 'Codex IDE, diff review, terminal & tests (Claude Code)',
  },
  {
    id: 'research',
    label: 'Claude Science',
    icon: FlaskConical,
    cssClass: 'active-research',
    description: 'Literature corpus, claims & experiments (Claude Science)',
  },
];

/* Pane label/icon map */
const PANE_META: Record<WorkspacePaneType, { label: string; icon: React.ComponentType<{ className?: string }> }> = {
  chat:        { label: 'Claude Chat',    icon: MessageSquare },
  engineering: { label: 'Codex IDE',      icon: Terminal },
  research:    { label: 'Claude Science', icon: FlaskConical },
  assistant:   { label: 'Agent Fleet',    icon: Bot },
  diff:        { label: 'Diff Review',    icon: FileDiff },
  browser:     { label: 'Browser',        icon: Globe },
  markdown:    { label: 'Notes.md',       icon: FileText },
};

/* Layout mode icons */
const LAYOUT_OPTIONS: { mode: LayoutMode; label: string; icon: React.ComponentType<{ className?: string }> }[] = [
  { mode: 'single',  label: 'Focus',  icon: Square },
  { mode: 'split-2', label: 'Split',  icon: Columns2 },
];

/* ─────────────────────────────────────────────────────────────
   Props
───────────────────────────────────────────────────────────── */

interface WorkspaceTabBarProps {
  panes: WorkspacePane[];
  selected: WorkspacePaneType;
  layoutMode?: LayoutMode;
  onChangeLayoutMode?: (mode: LayoutMode) => void;
  inspectorOpen: boolean;
  onSelect: (pane: WorkspacePaneType) => void;
  onClose: (pane: WorkspacePaneType) => void;
  onAdd: (pane: WorkspacePaneType) => void;
  onToggleInspector: () => void;
  onOpenSettings?: () => void;
  activeLens?: WorkbenchLens;
  onSwitchLens?: (lens: WorkbenchLens) => void;
  activeTask?: { title: string; status: string; model?: string; budget?: string } | null;
  onOpenTaskDetailsModal?: () => void;

  // Legacy compat — unused but kept to avoid prop errors in parent
  split?: boolean;
  onToggleSplit?: () => void;
  onSelectAgentModel?: (model: string) => void;
  onOpenDelegateModal?: () => void;
}

/* ─────────────────────────────────────────────────────────────
   Sub-components
───────────────────────────────────────────────────────────── */

/** 1. TaskStrip — top of bar, always visible */
function TaskStrip({
  task,
  onOpenDetails,
}: {
  task?: { title: string; status: string; model?: string; budget?: string } | null;
  onOpenDetails?: () => void;
}) {
  const statusColor =
    !task ? 'var(--color-fg-subtle)' :
    task.status === 'Active'    ? 'var(--color-task-active)' :
    task.status === 'Blocked'   ? 'var(--color-task-blocked)' :
    task.status === 'Done'      ? 'var(--color-task-done)' :
    task.status === 'Failed'    ? 'var(--color-task-failed)' :
    'var(--color-task-draft)';

  if (!task) {
    return (
      <div
        className="task-strip flex items-center justify-between"
        style={{ borderBottom: '1px solid var(--color-border-muted)' }}
      >
        <div className="flex items-center gap-2">
          <span className="status-dot unknown" />
          <span className="font-mono text-[11px] text-[#8b949e]">No active task</span>
          <span className="font-mono text-[10px] text-[#6e7681]">· Ready for instruction</span>
        </div>
        {onOpenDetails && (
          <button
            onClick={onOpenDetails}
            className="font-mono text-[10px] px-2 py-0.5 rounded text-[#58a6ff] hover:bg-surface-2 transition flex items-center gap-1"
            style={{ border: '1px solid rgba(88,166,255,0.25)' }}
          >
            <span>+ Delegate Task</span>
          </button>
        )}
      </div>
    );
  }

  const isActive = task.status === 'Active' || task.status === 'Running';

  return (
    <button
      className="task-strip w-full text-left transition-colors flex items-center justify-between"
      style={{
        borderBottom: '1px solid var(--color-border-muted)',
        cursor: onOpenDetails ? 'pointer' : 'default',
      }}
      onClick={onOpenDetails}
      onMouseEnter={e => onOpenDetails && (e.currentTarget.style.background = 'var(--color-surface-2)')}
      onMouseLeave={e => (e.currentTarget.style.background = '')}
    >
      <div className="flex items-center gap-2 truncate">
        <span 
          className={`status-dot ${isActive ? 'animate-pulse' : ''}`} 
          style={{ background: statusColor }} 
        />
        <span
          className="font-medium truncate max-w-sm"
          style={{ fontSize: '0.8125rem', color: 'var(--color-editor-fg)' }}
        >
          {task.title}
        </span>
        <span
          className="font-mono text-[10px] shrink-0 font-semibold"
          style={{ color: statusColor }}
        >
          ● {task.status}
        </span>
        {task.model && (
          <span
            className="font-mono text-[10px] shrink-0 hidden md:inline text-[#8b949e]"
          >
            · {task.model}
          </span>
        )}
      </div>

      <div className="flex items-center gap-2 shrink-0 ml-auto">
        {/* Orca Managed Worktree Chip */}
        <span 
          className="font-mono text-[10px] px-1.5 py-0.2 rounded hidden lg:inline"
          style={{
            background: 'var(--color-surface-2, #1c2128)',
            color: 'var(--color-editor-fg, #e6edf3)',
            border: '1px solid var(--color-border-default, #30363d)'
          }}
          title="Orca Managed Worktree: feat/simd-dispatch (#a3f2d1e)"
        >
          🌳 feat/simd-dispatch <span className="text-[#8b949e]">#a3f2d1e</span>
        </span>

        {/* SADE Assurance Level Badge */}
        <span 
          className="font-mono text-[10px] px-1.5 py-0.2 rounded hidden sm:inline"
          style={{
            background: 'rgba(63, 185, 80, 0.12)',
            color: '#3fb950',
            border: '1px solid rgba(63, 185, 80, 0.3)'
          }}
          title="Assurance Level: Mediated Authority under PermitGate contracts"
        >
          🛡️ Mediated Authority
        </span>

        {task.budget && (
          <span
            className="font-mono text-[10px] text-[#8b949e]"
          >
            {task.budget}
          </span>
        )}

        {onOpenDetails && (
          <ChevronDown className="w-3.5 h-3.5 text-[#6e7681]" />
        )}
      </div>
    </button>
  );
}

/** 2. LensSwitcher — Copilot | Coding | Research */
function LensSwitcher({
  activeLens,
  onSwitch,
}: {
  activeLens: WorkbenchLens;
  onSwitch: (lens: WorkbenchLens) => void;
}) {
  return (
    <div className="flex items-center gap-0.5 px-2 shrink-0">
      {LENSES.map(l => {
        const Icon = l.icon;
        const active = activeLens === l.id;
        return (
          <button
            key={l.id}
            onClick={() => onSwitch(l.id)}
            className={`lens-tab ${active ? l.cssClass : ''}`}
            title={l.description}
            aria-pressed={active}
          >
            <Icon className="w-3.5 h-3.5 shrink-0" />
            {l.label}
          </button>
        );
      })}
    </div>
  );
}

/** 3. PaneTabStrip — tabs for open panes */
function PaneTabStrip({
  panes,
  selected,
  onSelect,
  onClose,
  onAdd,
  activeLens,
}: {
  panes: WorkspacePane[];
  selected: WorkspacePaneType;
  onSelect: (pane: WorkspacePaneType) => void;
  onClose: (pane: WorkspacePaneType) => void;
  onAdd: (pane: WorkspacePaneType) => void;
  activeLens: WorkbenchLens;
}) {
  const [isAddOpen, setIsAddOpen] = useState(false);

  // Panes available to add per lens
  const availableToAdd: WorkspacePaneType[] =
    activeLens === 'coding'   ? ['engineering', 'diff', 'browser'] :
    activeLens === 'research' ? ['research', 'browser', 'markdown'] :
    ['chat', 'assistant'];

  const alreadyOpen = new Set(panes.map(p => p.id));
  const toAdd = availableToAdd.filter(p => !alreadyOpen.has(p));

  const accentColor =
    activeLens === 'copilot'  ? 'var(--color-copilot)'  :
    activeLens === 'coding'   ? 'var(--color-coding)'   :
    'var(--color-research)';

  return (
    <div className="flex items-center flex-1 overflow-x-auto min-w-0 gap-0.5 px-1" style={{ scrollbarWidth: 'none' }}>
      {panes.map(pane => {
        const meta   = PANE_META[pane.id];
        const Icon   = meta?.icon ?? Square;
        const active = pane.id === selected;
        return (
          <div
            key={pane.id}
            className="flex items-center shrink-0 group"
            style={{
              height: '32px',
              padding: '0 6px 0 8px',
              borderRadius: '4px',
              background: active ? 'var(--color-surface-2)' : 'transparent',
              border: `1px solid ${active ? 'var(--color-border-default)' : 'transparent'}`,
              cursor: 'pointer',
              transition: 'background-color 0.08s',
            }}
            onClick={() => onSelect(pane.id)}
            onMouseEnter={e => !active && (e.currentTarget.style.background = 'rgba(255,255,255,0.03)')}
            onMouseLeave={e => !active && (e.currentTarget.style.background = 'transparent')}
          >
            <span className="shrink-0 mr-1.5 flex items-center" style={{ color: active ? accentColor : 'var(--color-fg-muted)' }}>
              <Icon className="w-3.5 h-3.5" />
            </span>
            <span
              className="text-[12px] whitespace-nowrap"
              style={{
                color: active ? 'var(--color-editor-fg)' : 'var(--color-fg-muted)',
                fontWeight: active ? 500 : 400,
              }}
            >
              {meta?.label ?? pane.title}
            </span>
            {panes.length > 1 && (
              <button
                className="ml-1.5 rounded opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center"
                style={{
                  width: '16px',
                  height: '16px',
                  color: 'var(--color-fg-muted)',
                }}
                onClick={(e) => { e.stopPropagation(); onClose(pane.id); }}
                title={`Close ${meta?.label}`}
                onMouseEnter={e => (e.currentTarget.style.color = 'var(--color-task-failed)')}
                onMouseLeave={e => (e.currentTarget.style.color = 'var(--color-fg-muted)')}
              >
                <X className="w-3 h-3" />
              </button>
            )}
          </div>
        );
      })}

      {/* Add pane */}
      {toAdd.length > 0 && (
        <div className="relative shrink-0">
          <button
            className="btn-icon"
            onClick={() => setIsAddOpen(!isAddOpen)}
            title="Add pane"
          >
            <Plus className="w-3.5 h-3.5" />
          </button>
          {isAddOpen && (
            <div
              className="absolute top-full left-0 mt-1 rounded"
              style={{
                background: 'var(--color-surface-overlay)',
                border: '1px solid var(--color-border-default)',
                boxShadow: '0 4px 16px rgba(0,0,0,0.5)',
                zIndex: 999,
                padding: '4px',
                minWidth: '120px',
              }}
            >
              {toAdd.map(p => {
                const meta = PANE_META[p];
                const Icon = meta?.icon ?? Square;
                return (
                  <button
                    key={p}
                    onClick={() => { onAdd(p); setIsAddOpen(false); }}
                    className="w-full text-left flex items-center gap-2 px-2 py-1 rounded transition-colors"
                    style={{ fontSize: '0.75rem', color: 'var(--color-fg-muted)' }}
                    onMouseEnter={e => {
                      e.currentTarget.style.background = 'var(--color-surface-2)';
                      e.currentTarget.style.color = 'var(--color-editor-fg)';
                    }}
                    onMouseLeave={e => {
                      e.currentTarget.style.background = 'transparent';
                      e.currentTarget.style.color = 'var(--color-fg-muted)';
                    }}
                  >
                    <Icon className="w-3.5 h-3.5 shrink-0" />
                    {meta?.label ?? p}
                  </button>
                );
              })}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

/* ─────────────────────────────────────────────────────────────
   Main component
───────────────────────────────────────────────────────────── */

export function WorkspaceTabBar({
  panes,
  selected,
  layoutMode = 'split-2',
  onChangeLayoutMode,
  inspectorOpen,
  onSelect,
  onClose,
  onAdd,
  onToggleInspector,
  onOpenSettings: _onOpenSettings,
  activeLens = 'copilot',
  onSwitchLens,
  activeTask,
  onOpenTaskDetailsModal,
}: WorkspaceTabBarProps) {
  return (
    <div
      className="flex flex-col shrink-0"
      style={{ background: 'var(--color-surface-1)', userSelect: 'none' }}
    >
      {/* Row 1: Task Strip */}
      <TaskStrip task={activeTask} onOpenDetails={onOpenTaskDetailsModal} />

      {/* Row 2: Lens switcher + Pane tabs + Toolbar */}
      <div
        className="flex items-center"
        style={{
          height: '32px',
          borderBottom: '1px solid var(--color-border-muted)',
        }}
      >
        {/* Lens switcher — left */}
        <LensSwitcher
          activeLens={activeLens}
          onSwitch={(lens) => onSwitchLens?.(lens)}
        />

        {/* Divider */}
        <div style={{ width: '1px', height: '16px', background: 'var(--color-border-muted)', margin: '0 4px' }} />

        {/* Pane tabs — flexible middle */}
        <PaneTabStrip
          panes={panes}
          selected={selected}
          onSelect={onSelect}
          onClose={onClose}
          onAdd={onAdd}
          activeLens={activeLens}
        />

        {/* Toolbar — right */}
        <div className="flex items-center gap-0.5 px-2 shrink-0">
          {/* Layout mode */}
          {onChangeLayoutMode && LAYOUT_OPTIONS.map(opt => {
            const Icon = opt.icon;
            const active = layoutMode === opt.mode;
            return (
              <button
                key={opt.mode}
                className="btn-icon"
                onClick={() => onChangeLayoutMode(opt.mode)}
                title={opt.label}
                style={active ? { color: 'var(--color-editor-fg)', background: 'var(--color-surface-2)' } : undefined}
              >
                <Icon className="w-3.5 h-3.5" />
              </button>
            );
          })}

          {/* Inspector toggle */}
          <button
            className="btn-icon"
            onClick={onToggleInspector}
            title={inspectorOpen ? 'Close inspector' : 'Open inspector'}
            style={inspectorOpen ? { color: 'var(--color-research)', background: 'var(--color-surface-2)' } : undefined}
          >
            <PanelRight className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </div>
  );
}

export default WorkspaceTabBar;
