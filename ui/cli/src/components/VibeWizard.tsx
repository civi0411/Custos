import React, { useState } from 'react';
import { ExecutionPermit, OperationalMode, Task } from '../types';
import { OPERATIONAL_MODES, SAMPLE_DIFF } from '../data/constants';
import { DiffViewer } from './DiffViewer';
import { CustosApi } from '../services/custosApi';
import { Zap, ArrowRight, Loader2, CheckCircle, XCircle, RotateCcw, ShieldAlert, Terminal, Code2, Compass, Bot } from 'lucide-react';

interface VibeWizardProps {
  initialMode: OperationalMode;
  onOpenTerminalWithCommand?: (cmd: string) => void;
  onTaskCreated?: (task: Task) => void;
}

export const VibeWizard: React.FC<VibeWizardProps> = ({
  initialMode,
  onOpenTerminalWithCommand,
  onTaskCreated,
}) => {
  const [mode, setMode] = useState<OperationalMode>(initialMode);
  const [goal, setGoal] = useState('');

  const getModeIcon = (m: OperationalMode) => {
    switch (m) {
      case 'Code':
        return <Code2 size={16} />;
      case 'Research':
        return <Compass size={16} />;
      case 'Assistant':
        return <Bot size={16} />;
    }
  };
  const [stage, setStage] = useState<
    'input' | 'analyzing' | 'synthesizing' | 'review_diff' | 'executing' | 'completed' | 'cancelled'
  >('input');
  const [currentTask, setCurrentTask] = useState<Task | null>(null);
  const [pendingPermit, setPendingPermit] = useState<ExecutionPermit | null>(null);
  const [progressMsg, setProgressMsg] = useState('');

  const sampleGoals = [
    'Add verified JWT validation to crates/runtime/src/handler.rs',
    'Investigate SQLite WAL concurrency bottlenecks & propose benchmark',
    'Coordinate multi-agent policy audit across workflow recipes',
  ];

  const handleStartVibe = async () => {
    if (!goal.trim()) return;

    setStage('analyzing');
    setProgressMsg(`Analyzing repository and compiling context recipe for [${mode}] mode...`);

    await new Promise((r) => setTimeout(r, 700));

    setStage('synthesizing');
    setProgressMsg('Synthesizing solution trajectory with LLM Provider...');

    await new Promise((r) => setTimeout(r, 900));

    const task = await CustosApi.createTask(goal, mode);
    setCurrentTask(task);
    if (onTaskCreated) onTaskCreated(task);

    const permit = CustosApi.createPermit(
      task.id,
      'ExecutionPermit: Apply Code Diff',
      'crates/runtime/src/handler.rs',
      'Medium',
      SAMPLE_DIFF.filePath,
      SAMPLE_DIFF.oldCode,
      SAMPLE_DIFF.newCode
    );
    setPendingPermit(permit);

    setStage('review_diff');
  };

  const handleApprovePermit = async () => {
    if (!currentTask || !pendingPermit) return;

    CustosApi.resolvePermit(pendingPermit.id, true);
    setStage('executing');
    setProgressMsg('Permit granted by operator. Worktree isolated and patched...');

    await CustosApi.advanceTask(currentTask.id, 'Queued', 'Permit approved by operator');
    await new Promise((r) => setTimeout(r, 500));

    setProgressMsg('Provider actively coding and executing task payload in sandbox worktree...');
    await CustosApi.advanceTask(currentTask.id, 'Running', 'Active coding in sandbox');
    await new Promise((r) => setTimeout(r, 800));

    const completed = await CustosApi.completeTask(
      currentTask.id,
      `Task '${goal}' successfully executed, verified, and committed`
    );
    setCurrentTask(completed);
    setStage('completed');
  };

  const handleRejectPermit = async () => {
    if (!currentTask || !pendingPermit) return;

    CustosApi.resolvePermit(pendingPermit.id, false);
    const cancelled = await CustosApi.cancelTask(
      currentTask.id,
      'Execution rejected by operator. Worktree untouched.'
    );
    setCurrentTask(cancelled);
    setStage('cancelled');
  };

  const handleReset = () => {
    setGoal('');
    setStage('input');
    setCurrentTask(null);
    setPendingPermit(null);
    setProgressMsg('');
  };

  const currentCfg = OPERATIONAL_MODES[mode];

  return (
    <div className="vibe-wizard-card">
      <div className="vibe-wizard-header">
        <div className="vibe-title-group">
          <div className="vibe-icon-pill" style={{ backgroundColor: `${currentCfg.badgeColor}20`, color: currentCfg.badgeColor }}>
            <Zap size={16} />
            <span>Interactive Vibe Flow</span>
          </div>
          <span className="vibe-subtitle">
            Autonomous trajectory planner with Human-in-the-Loop ExecutionPermit governance
          </span>
        </div>

        <div className="vibe-stage-pills">
          <span className={`stage-step ${stage === 'input' ? 'current' : 'done'}`}>
            1. Goal
          </span>
          <span className={`stage-step ${['analyzing', 'synthesizing'].includes(stage) ? 'current' : ['review_diff', 'executing', 'completed', 'cancelled'].includes(stage) ? 'done' : ''}`}>
            2. Trajectory
          </span>
          <span className={`stage-step ${stage === 'review_diff' ? 'current' : ['executing', 'completed', 'cancelled'].includes(stage) ? 'done' : ''}`}>
            3. Permit
          </span>
          <span className={`stage-step ${['completed', 'cancelled'].includes(stage) ? 'current' : ''}`}>
            4. Result
          </span>
        </div>
      </div>

      <div className="vibe-wizard-body">
        {stage === 'input' && (
          <div className="wizard-input-stage">
            <div className="mode-select-row">
              <label className="field-label">Active Mode:</label>
              <div className="mode-pills">
                {(['Code', 'Research', 'Assistant'] as OperationalMode[]).map((m) => {
                  const cfg = OPERATIONAL_MODES[m];
                  return (
                    <button
                      key={m}
                      className={`mode-select-btn ${mode === m ? 'active' : ''}`}
                      style={{
                        borderColor: mode === m ? cfg.badgeColor : 'transparent',
                        color: mode === m ? cfg.badgeColor : 'var(--text-muted)',
                      }}
                      onClick={() => setMode(m)}
                    >
                      <span className="btn-mode-icon" style={{ color: mode === m ? cfg.badgeColor : 'inherit' }}>
                        {getModeIcon(m)}
                      </span>
                      <span>{cfg.name}</span>
                    </button>
                  );
                })}
              </div>
            </div>

            <div className="goal-input-box">
              <label className="field-label">
                ❄ Tôi có thể giúp gì cho bạn trong chế độ [{currentCfg.name}]? Nhập mục tiêu:
              </label>
              <textarea
                className="goal-textarea"
                rows={3}
                placeholder="Ví dụ: Triển khai kiểm tra chữ ký token trong handler.rs, thêm unit test..."
                value={goal}
                onChange={(e) => setGoal(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                    handleStartVibe();
                  }
                }}
              />

              <div className="quick-suggestions">
                <span className="suggest-label">Gợi ý nhanh:</span>
                {sampleGoals.map((g, idx) => (
                  <button
                    key={idx}
                    className="suggestion-tag"
                    onClick={() => setGoal(g)}
                  >
                    {g}
                  </button>
                ))}
              </div>
            </div>

            <div className="wizard-actions">
              <button
                className="launch-vibe-btn"
                disabled={!goal.trim()}
                onClick={handleStartVibe}
                style={{ backgroundColor: currentCfg.badgeColor }}
              >
                <Zap size={16} />
                <span>Khởi chạy Trajectory (Ctrl + Enter)</span>
                <ArrowRight size={16} />
              </button>
            </div>
          </div>
        )}

        {(stage === 'analyzing' || stage === 'synthesizing' || stage === 'executing') && (
          <div className="wizard-loading-stage">
            <div className="loading-mascot-box engine-loading" style={{ borderColor: `${currentCfg.badgeColor}40`, backgroundColor: `${currentCfg.badgeColor}12` }}>
              <div className="loading-center-icon" style={{ color: currentCfg.badgeColor }}>
                {getModeIcon(mode)}
              </div>
              <Loader2 size={40} className="spinner-icon animate-spin" style={{ color: currentCfg.badgeColor }} />
            </div>

            <h3 className="loading-title">Custos Runtime Active</h3>
            <p className="loading-status-text">{progressMsg}</p>

            <div className="pipeline-steps">
              <div className={`step-item ${stage === 'analyzing' ? 'running' : 'done'}`}>
                <span className="step-dot" />
                <span>Compiling Context Recipe & Worktree Topology</span>
              </div>
              <div className={`step-item ${stage === 'synthesizing' ? 'running' : stage === 'executing' ? 'done' : ''}`}>
                <span className="step-dot" />
                <span>Synthesizing AST Code Diff & Domain Invariants</span>
              </div>
              <div className={`step-item ${stage === 'executing' ? 'running' : ''}`}>
                <span className="step-dot" />
                <span>Bubblewrap Sandbox Verification & Permit Enforcement</span>
              </div>
            </div>
          </div>
        )}

        {stage === 'review_diff' && pendingPermit && (
          <div className="wizard-diff-stage">
            <div className="permit-request-banner">
              <div className="banner-left">
                <ShieldAlert size={20} className="shield-icon" />
                <div>
                  <h4>ExecutionPermit Requested</h4>
                  <p>
                    Target: <code>{pendingPermit.target}</code> • Risk Level:{' '}
                    <strong style={{ color: '#f59e0b' }}>[{pendingPermit.risk} RISK]</strong>
                  </p>
                </div>
              </div>
              <div className="banner-right">
                <button className="permit-decline-btn" onClick={handleRejectPermit}>
                  <XCircle size={14} />
                  <span>Reject Permit</span>
                </button>
                <button className="permit-grant-btn" onClick={handleApprovePermit}>
                  <CheckCircle size={14} />
                  <span>Approve & Execute Permit</span>
                </button>
              </div>
            </div>

            <DiffViewer
              filePath={pendingPermit.filePath}
              oldCode={pendingPermit.oldCode}
              newCode={pendingPermit.newCode}
            />
          </div>
        )}

        {stage === 'completed' && currentTask && (
          <div className="wizard-result-stage success">
            <div className="result-icon-wrapper success">
              <CheckCircle size={42} />
            </div>
            <h3>Task Succeeded & Committed</h3>
            <p className="task-summary-text">{currentTask.result_summary}</p>

            <div className="task-id-badge">
              <span>Task ID:</span> <code>{currentTask.id}</code>
              <span>• Epoch:</span> <code>{currentTask.epoch}</code>
              <span>• Status:</span> <strong style={{ color: '#10b981' }}>{currentTask.status}</strong>
            </div>

            <div className="result-actions">
              <button className="secondary-btn" onClick={handleReset}>
                <RotateCcw size={14} />
                <span>Chạy mục tiêu khác</span>
              </button>
              {onOpenTerminalWithCommand && (
                <button
                  className="primary-btn"
                  onClick={() => onOpenTerminalWithCommand(`custos status -i ${currentTask.id}`)}
                >
                  <Terminal size={14} />
                  <span>Xem Status & Spans trong CLI</span>
                </button>
              )}
            </div>
          </div>
        )}

        {stage === 'cancelled' && currentTask && (
          <div className="wizard-result-stage cancelled">
            <div className="result-icon-wrapper cancelled">
              <XCircle size={42} />
            </div>
            <h3>Task Cancelled by Operator</h3>
            <p className="task-summary-text">
              {currentTask.failure_reason || 'Execution permit rejected. Sandbox worktree untouched.'}
            </p>

            <div className="task-id-badge">
              <span>Task ID:</span> <code>{currentTask.id}</code>
              <span>• Status:</span> <strong style={{ color: '#ef4444' }}>{currentTask.status}</strong>
            </div>

            <div className="result-actions">
              <button className="secondary-btn" onClick={handleReset}>
                <RotateCcw size={14} />
                <span>Thử lại với mục tiêu khác</span>
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
