import React from 'react';
import { OperationalMode, ResponsiveTier } from '../types';
import { OPERATIONAL_MODES } from '../data/constants';
import { ShieldCheck, Database, Lock, Box, Cpu } from 'lucide-react';

interface StatusBarProps {
  currentMode: OperationalMode;
  responsiveTier: ResponsiveTier;
  termWidth: number;
}

export const StatusBar: React.FC<StatusBarProps> = ({
  currentMode,
  responsiveTier,
  termWidth,
}) => {
  const currentCfg = OPERATIONAL_MODES[currentMode];

  return (
    <footer className="cli-status-bar">
      <div className="status-bar-left">
        <div className="status-item connection">
          <span className="status-pulse-dot" />
          <Database size={12} />
          <span>custos-daemon: ONLINE (SQLite)</span>
        </div>

        <div className="status-divider" />

        <div className="status-item mode">
          <Cpu size={12} style={{ color: currentCfg.badgeColor }} />
          <span>Mode:</span>
          <strong style={{ color: currentCfg.badgeColor }}>{currentCfg.name}</strong>
          <span className="status-mascot-label">({currentCfg.mascotName})</span>
        </div>
      </div>

      <div className="status-bar-right">
        <div className="status-item">
          <Lock size={12} className="status-icon-accent" />
          <span>Epoch Optimistic Lock: ON</span>
        </div>

        <div className="status-divider" />

        <div className="status-item">
          <Box size={12} className="status-icon-accent" />
          <span>Bubblewrap Sandbox: ACTIVE</span>
        </div>

        <div className="status-divider" />

        <div className="status-item">
          <ShieldCheck size={12} style={{ color: '#10b981' }} />
          <span>Permits: GATED</span>
        </div>

        <div className="status-divider" />

        <div className="status-item terminal-dim">
          <span>{termWidth}w × 36h [{responsiveTier}]</span>
        </div>
      </div>
    </footer>
  );
};
