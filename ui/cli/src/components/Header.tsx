import React from 'react';
import { OperationalMode, ResponsiveTier, ViewMode } from '../types';
import { OPERATIONAL_MODES } from '../data/constants';
import { Terminal, Layout, Split, ShieldCheck, Monitor } from 'lucide-react';

interface HeaderProps {
  currentMode: OperationalMode;
  onModeChange: (mode: OperationalMode) => void;
  viewMode: ViewMode;
  onViewModeChange: (view: ViewMode) => void;
  responsiveTier: ResponsiveTier;
  crtEffect: boolean;
  onToggleCrt: () => void;
  termWidth: number;
}

export const Header: React.FC<HeaderProps> = ({
  currentMode,
  onModeChange,
  viewMode,
  onViewModeChange,
  responsiveTier,
  crtEffect,
  onToggleCrt,
  termWidth,
}) => {

  return (
    <header className="cli-header">
      <div className="header-left">
        <div className="brand-badge">
          <span className="brand-snowflake">❄</span>
          <span className="brand-name">Custos</span>
          <span className="brand-version">v0.1.0</span>
          <span className="brand-pill">CLI UI</span>
        </div>

        <div className="header-divider" />

        <div className="mode-selector-group">
          {(['Code', 'Research', 'Assitant'] as OperationalMode[]).map((mode) => {
            const cfg = OPERATIONAL_MODES[mode];
            const isActive = currentMode === mode;
            return (
              <button
                key={mode}
                className={`mode-btn ${isActive ? 'active' : ''}`}
                style={{
                  borderColor: isActive ? cfg.badgeColor : 'transparent',
                  color: isActive ? cfg.badgeColor : 'var(--text-muted)',
                }}
                onClick={() => onModeChange(mode)}
                title={cfg.tagline}
              >
                <span className="mode-dot" style={{ backgroundColor: cfg.badgeColor }} />
                <span className="mode-label">{cfg.name}</span>
              </button>
            );
          })}
        </div>
      </div>

      <div className="header-center">
        <div className="tier-badge" title={`Terminal Columns: ${termWidth}`}>
          <span className="tier-tag">{responsiveTier}</span>
          <span className="tier-cols">{termWidth} cols</span>
        </div>
      </div>

      <div className="header-right">
        <div className="view-toggle-group">
          <button
            className={`view-btn ${viewMode === 'terminal' ? 'active' : ''}`}
            onClick={() => onViewModeChange('terminal')}
            title="Interactive Terminal Console View"
          >
            <Terminal size={14} />
            <span>Terminal</span>
          </button>
          <button
            className={`view-btn ${viewMode === 'split' ? 'active' : ''}`}
            onClick={() => onViewModeChange('split')}
            title="Side-by-side Terminal + Visual View"
          >
            <Split size={14} />
            <span>Split</span>
          </button>
          <button
            className={`view-btn ${viewMode === 'visual' ? 'active' : ''}`}
            onClick={() => onViewModeChange('visual')}
            title="Visual Workflow & Mascot Cards View"
          >
            <Layout size={14} />
            <span>Visual</span>
          </button>
        </div>

        <button
          className={`icon-toggle-btn ${crtEffect ? 'active' : ''}`}
          onClick={onToggleCrt}
          title={crtEffect ? 'Disable CRT Scanlines' : 'Enable CRT Scanlines'}
        >
          <Monitor size={14} />
          <span>CRT</span>
        </button>

        <div className="human-permit-indicator" title="Human-in-the-Loop Governance Active">
          <ShieldCheck size={14} className="shield-icon" />
          <span>Permits: ENFORCED</span>
        </div>
      </div>
    </header>
  );
};
