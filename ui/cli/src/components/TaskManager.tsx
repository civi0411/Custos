import React, { useState, useEffect } from 'react';
import { Task, TaskSpan, TaskStatus } from '../types';
import { CustosApi, subscribeToApi } from '../services/custosApi';
import { RefreshCw, Play, Check, X, ChevronRight, Layers } from 'lucide-react';

interface TaskManagerProps {
  onSelectTaskInCli?: (taskId: string) => void;
}

export const TaskManager: React.FC<TaskManagerProps> = ({ onSelectTaskInCli }) => {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [filter, setFilter] = useState<string>('All');
  const [selectedTask, setSelectedTask] = useState<Task | null>(null);
  const [taskSpans, setTaskSpans] = useState<TaskSpan[]>([]);
  const [loading, setLoading] = useState(false);

  const fetchTasks = async () => {
    setLoading(true);
    try {
      const data = await CustosApi.listTasks();
      setTasks(data);
      if (selectedTask) {
        const updated = data.find((t) => t.id === selectedTask.id);
        if (updated) setSelectedTask(updated);
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchTasks();
    const unsub = subscribeToApi(() => {
      fetchTasks();
    });
    return unsub;
  }, []);

  const handleSelectTask = async (task: Task) => {
    setSelectedTask(task);
    const spans = await CustosApi.listSpans(task.id);
    setTaskSpans(spans);
  };

  const handleAdvance = async (id: string, status: TaskStatus) => {
    await CustosApi.advanceTask(id, status, `Manual advance to ${status}`);
    fetchTasks();
  };

  const handleComplete = async (id: string) => {
    await CustosApi.completeTask(id, 'Marked complete by operator');
    fetchTasks();
  };

  const handleCancel = async (id: string) => {
    await CustosApi.cancelTask(id, 'Cancelled by operator via UI');
    fetchTasks();
  };

  const filteredTasks = tasks.filter((t) => {
    if (filter === 'All') return true;
    return t.status.toLowerCase() === filter.toLowerCase();
  });

  const getStatusColor = (status: TaskStatus) => {
    switch (status) {
      case 'Draft':
        return '#94a3b8';
      case 'Queued':
        return '#f59e0b';
      case 'Running':
        return '#38bdf8';
      case 'Blocked':
        return '#d946ef';
      case 'Succeeded':
        return '#10b981';
      case 'Failed':
        return '#ef4444';
      case 'Cancelled':
        return '#f43f5e';
    }
  };

  return (
    <div className="task-manager-card">
      <div className="task-manager-header">
        <div className="header-left-title">
          <Layers size={18} className="title-icon" />
          <h3>Agentic Task Registry & Audit Spans</h3>
          <span className="task-count-pill">{tasks.length} tasks</span>
        </div>

        <div className="header-right-filters">
          <div className="filter-pills">
            {['All', 'Running', 'Queued', 'Succeeded', 'Cancelled'].map((f) => (
              <button
                key={f}
                className={`filter-btn ${filter === f ? 'active' : ''}`}
                onClick={() => setFilter(f)}
              >
                {f}
              </button>
            ))}
          </div>

          <button className="refresh-btn" onClick={fetchTasks} title="Refresh task list">
            <RefreshCw size={14} className={loading ? 'animate-spin' : ''} />
          </button>
        </div>
      </div>

      <div className="task-manager-body">
        {/* Task List Table */}
        <div className="task-table-wrapper">
          <table className="tasks-table">
            <thead>
              <tr>
                <th>ID</th>
                <th>Title / Goal</th>
                <th>Status</th>
                <th>Epoch</th>
                <th>Mode</th>
                <th>Updated</th>
                <th>Action</th>
              </tr>
            </thead>
            <tbody>
              {filteredTasks.length === 0 ? (
                <tr>
                  <td colSpan={7} className="empty-row">
                    No tasks found in filter [{filter}]. Start one with <code>custos vibe</code>.
                  </td>
                </tr>
              ) : (
                filteredTasks.map((t) => {
                  const statusColor = getStatusColor(t.status);
                  const isSelected = selectedTask?.id === t.id;
                  const mode = (t.metadata?.mode as string) || 'code';

                  return (
                    <tr
                      key={t.id}
                      className={`task-row ${isSelected ? 'selected' : ''}`}
                      onClick={() => handleSelectTask(t)}
                    >
                      <td className="task-id-cell">
                        <code>{t.id}</code>
                      </td>
                      <td className="task-title-cell" title={t.title}>
                        {t.title}
                      </td>
                      <td className="task-status-cell">
                        <span
                          className="status-badge"
                          style={{
                            backgroundColor: `${statusColor}18`,
                            color: statusColor,
                            borderColor: `${statusColor}40`,
                          }}
                        >
                          {t.status}
                        </span>
                      </td>
                      <td className="task-epoch-cell">e{t.epoch}</td>
                      <td className="task-mode-cell">
                        <span className="mode-mini-tag">{mode}</span>
                      </td>
                      <td className="task-time-cell">
                        {new Date(t.updated_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
                      </td>
                      <td className="task-actions-cell" onClick={(e) => e.stopPropagation()}>
                        {t.status === 'Draft' && (
                          <button
                            className="mini-action-btn run"
                            onClick={() => handleAdvance(t.id, 'Queued')}
                            title="Queue Task"
                          >
                            <Play size={12} />
                          </button>
                        )}
                        {t.status === 'Queued' && (
                          <button
                            className="mini-action-btn run"
                            onClick={() => handleAdvance(t.id, 'Running')}
                            title="Run Task"
                          >
                            <Play size={12} />
                          </button>
                        )}
                        {t.status === 'Running' && (
                          <>
                            <button
                              className="mini-action-btn success"
                              onClick={() => handleComplete(t.id)}
                              title="Mark Complete"
                            >
                              <Check size={12} />
                            </button>
                            <button
                              className="mini-action-btn danger"
                              onClick={() => handleCancel(t.id)}
                              title="Cancel Task"
                            >
                              <X size={12} />
                            </button>
                          </>
                        )}
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>

        {/* Selected Task Detail Drawer */}
        {selectedTask && (
          <div className="task-detail-pane">
            <div className="detail-pane-header">
              <div className="detail-title-group">
                <span className="detail-id-tag">{selectedTask.id}</span>
                <h4>{selectedTask.title}</h4>
              </div>
              <button className="close-detail-btn" onClick={() => setSelectedTask(null)}>
                <X size={14} />
              </button>
            </div>

            <div className="detail-stats-grid">
              <div className="stat-box">
                <span className="stat-label">Current Status</span>
                <span
                  className="stat-val status"
                  style={{ color: getStatusColor(selectedTask.status) }}
                >
                  {selectedTask.status}
                </span>
              </div>
              <div className="stat-box">
                <span className="stat-label">Concurrency Epoch</span>
                <span className="stat-val">Epoch {selectedTask.epoch}</span>
              </div>
              <div className="stat-box">
                <span className="stat-label">Created At</span>
                <span className="stat-val time">
                  {new Date(selectedTask.created_at).toLocaleTimeString()}
                </span>
              </div>
            </div>

            {selectedTask.result_summary && (
              <div className="summary-block success">
                <strong>Result Summary:</strong> {selectedTask.result_summary}
              </div>
            )}

            {selectedTask.failure_reason && (
              <div className="summary-block error">
                <strong>Cancellation / Failure:</strong> {selectedTask.failure_reason}
              </div>
            )}

            {/* Audit Spans Timeline */}
            <div className="spans-timeline-section">
              <div className="timeline-title">Immutable SQLite Audit Spans ({taskSpans.length}):</div>
              <div className="spans-list">
                {taskSpans.map((s) => (
                  <div key={s.id} className="span-item">
                    <div className="span-dot" />
                    <div className="span-info">
                      <div className="span-top">
                        <span className="span-name">{s.name}</span>
                        <span className="span-stage">{s.stage}</span>
                        {s.duration_ms && <span className="span-dur">{s.duration_ms}ms</span>}
                      </div>
                      {s.details && <div className="span-details">{s.details}</div>}
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {onSelectTaskInCli && (
              <div className="detail-footer-actions">
                <button
                  className="cli-inspect-btn"
                  onClick={() => onSelectTaskInCli(selectedTask.id)}
                >
                  Inspect in Terminal Console
                  <ChevronRight size={14} />
                </button>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
