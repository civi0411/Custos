import React from 'react';
import { OperationalMode } from '../types';
import { OPERATIONAL_MODES, MASCOT_ASSETS } from '../data/constants';
import { Code2, Compass, Bot, CheckCircle2, Zap } from 'lucide-react';

interface MascotShowcaseProps {
  currentMode: OperationalMode;
  onSelectMode: (mode: OperationalMode) => void;
  onStartVibe: (mode: OperationalMode) => void;
}

export const MascotShowcase: React.FC<MascotShowcaseProps> = ({
  currentMode,
  onSelectMode,
  onStartVibe,
}) => {
  const modes: OperationalMode[] = ['Code', 'Research', 'Assitant'];

  const getModeIcon = (mode: OperationalMode) => {
    switch (mode) {
      case 'Code':
        return <Code2 size={18} />;
      case 'Research':
        return <Compass size={18} />;
      case 'Assitant':
        return <Bot size={18} />;
    }
  };

  return (
    <div className="mascot-showcase-container">
      {/* Top Guardian Hero Card */}
      <div className="guardian-hero-card">
        <div className="guardian-avatar-wrapper">
          <img
            src={MASCOT_ASSETS.mascot}
            alt="Custos Snowy Owl Mascot"
            className="guardian-avatar-img"
            onError={(e) => {
              // fallback if public asset path varies
              (e.target as HTMLImageElement).src = '/assets/owl.png';
            }}
          />
          <div className="pulse-aura" />
        </div>
        <div className="guardian-content">
          <div className="guardian-tag">
            <span className="snow-emoji">❄</span> GUARDIAN OF AGENTIC WORK
          </div>
          <h2 className="guardian-title">Custos Autonomous Intelligence Engine</h2>
          <p className="guardian-desc">
            Governing specialized agentic workflows with immutable SQLite audit logs, worktree sandbox isolation,
            and explicit operator ExecutionPermit checks.
          </p>
          <div className="guardian-meta">
            <span className="meta-badge">Runtime: Rust Core</span>
            <span className="meta-badge">IPC: LocalApiClient</span>
            <span className="meta-badge">Sandbox: Bubblewrap</span>
          </div>
        </div>
      </div>

      {/* Operational Modes Grid */}
      <div className="modes-grid-title">
        <h3>Operational Modes & Specialized Owls</h3>
        <p>Select a mode to specialize the agent trajectory, AST tools, and governance checks</p>
      </div>

      <div className="modes-cards-grid">
        {modes.map((mode) => {
          const cfg = OPERATIONAL_MODES[mode];
          const isSelected = currentMode === mode;

          return (
            <div
              key={mode}
              className={`mode-card ${isSelected ? 'selected' : ''}`}
              style={{
                borderColor: isSelected ? cfg.badgeColor : 'var(--border-subtle)',
                boxShadow: isSelected ? `0 0 25px ${cfg.badgeColor}25` : 'none',
              }}
              onClick={() => onSelectMode(mode)}
            >
              <div className="mode-card-header">
                <div
                  className="mode-icon-pill"
                  style={{ backgroundColor: `${cfg.badgeColor}20`, color: cfg.badgeColor }}
                >
                  {getModeIcon(mode)}
                  <span>{cfg.name}</span>
                </div>
                {isSelected && (
                  <span className="active-tag" style={{ color: cfg.badgeColor }}>
                    ACTIVE MODE
                  </span>
                )}
              </div>

              <div className="mode-image-frame">
                <img
                  src={cfg.mascotImage}
                  alt={cfg.mascotName}
                  className="mode-owl-img"
                  onError={(e) => {
                    // Fallback to guardian mascot if mode asset is loading
                    (e.target as HTMLImageElement).src = MASCOT_ASSETS.mascot;
                  }}
                />
              </div>

              <div className="mode-card-body">
                <h4 className="mascot-title" style={{ color: isSelected ? cfg.badgeColor : 'var(--text-main)' }}>
                  {cfg.mascotName}
                </h4>
                <div className="mode-tagline">{cfg.tagline}</div>
                <p className="mode-description">{cfg.description}</p>

                <div className="capabilities-list">
                  <div className="cap-label">Engine Capabilities:</div>
                  {cfg.capabilities.map((cap, i) => (
                    <div key={i} className="cap-item">
                      <CheckCircle2 size={12} style={{ color: cfg.badgeColor }} />
                      <span>{cap}</span>
                    </div>
                  ))}
                </div>
              </div>

              <div className="mode-card-footer">
                <button
                  className="start-vibe-btn"
                  style={{
                    backgroundColor: isSelected ? cfg.badgeColor : 'rgba(255, 255, 255, 0.05)',
                    color: isSelected ? '#0b0f19' : cfg.badgeColor,
                    borderColor: cfg.badgeColor,
                  }}
                  onClick={(e) => {
                    e.stopPropagation();
                    onSelectMode(mode);
                    onStartVibe(mode);
                  }}
                >
                  <Zap size={14} />
                  <span>Launch Vibe ({cfg.name})</span>
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
