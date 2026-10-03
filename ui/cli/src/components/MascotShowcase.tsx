import React from 'react';
import { OperationalMode } from '../types';
import { OPERATIONAL_MODES } from '../data/constants';
import { Code2, Compass, Bot, CheckCircle2, Zap, Cpu, Terminal } from 'lucide-react';

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
        return <Code2 size={22} />;
      case 'Research':
        return <Compass size={22} />;
      case 'Assitant':
        return <Bot size={22} />;
    }
  };

  const getModeSlug = (mode: OperationalMode) =>
    mode.toLowerCase() === 'assitant' ? '$custos-assistant' : `$custos-${mode.toLowerCase()}`;

  return (
    <div className="mascot-showcase-container">
      <div className="guardian-hero-card">
        <div className="guardian-avatar-wrapper engine-avatar">
          <div className="engine-core-icon">
            <Cpu size={36} className="core-chip" />
          </div>
          <div className="pulse-aura" />
        </div>
        <div className="guardian-content">
          <div className="guardian-tag">
            <span className="snow-emoji">❄</span> CUSTOS AGENTIC ENGINE
          </div>
          <h2 className="guardian-title">Custos Autonomous Intelligence Engine</h2>
          <p className="guardian-desc">
            Vận hành quy trình tác vụ chuyên biệt hóa với nhật ký SQLite bất biến, Bubblewrap Sandbox cách ly,
            và cơ chế cấp quyền ExecutionPermit chủ động.
          </p>
          <div className="guardian-meta">
            <span className="meta-badge">Runtime: Rust Core</span>
            <span className="meta-badge">IPC: LocalApiClient</span>
            <span className="meta-badge">Sandbox: Bubblewrap</span>
            <span className="meta-badge">Active: {getModeSlug(currentMode)}</span>
          </div>
        </div>
      </div>

      <div className="modes-grid-title">
        <h3>Chế độ hoạt động (Operational Modes)</h3>
        <p>Gõ '/' trong Terminal hoặc nhấp bên dưới để dán mode vào prompt ($custos-code, $custos-research, $custos-assistant)</p>
      </div>

      <div className="modes-cards-grid">
        {modes.map((mode) => {
          const cfg = OPERATIONAL_MODES[mode];
          const isSelected = currentMode === mode;
          const slug = getModeSlug(mode);

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
                    ACTIVE
                  </span>
                )}
              </div>

              <div className="mode-slug-bar" style={{ borderColor: `${cfg.badgeColor}40`, backgroundColor: `${cfg.badgeColor}12` }}>
                <Terminal size={14} style={{ color: cfg.badgeColor }} />
                <code style={{ color: cfg.badgeColor, fontWeight: 700 }}>{slug}</code>
              </div>

              <div className="mode-card-body">
                <h4 className="mascot-title" style={{ color: isSelected ? cfg.badgeColor : 'var(--text-main)' }}>
                  {cfg.name} Engine
                </h4>
                <div className="mode-tagline">{cfg.tagline}</div>
                <p className="mode-description">{cfg.description}</p>

                <div className="capabilities-list">
                  <div className="cap-label">Khả năng chuyên biệt:</div>
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
                  <span>Chọn {slug}</span>
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
