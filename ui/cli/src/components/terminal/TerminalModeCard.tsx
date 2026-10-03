import React from 'react';
import { OperationalMode } from '../../types';
import { AVAILABLE_MODES } from './types';
import { Sliders } from 'lucide-react';

interface TerminalModeCardProps {
  mode: OperationalMode;
}

export const TerminalModeCard: React.FC<TerminalModeCardProps> = ({ mode }) => {
  const cfg = AVAILABLE_MODES.find((m) => m.mode === mode) || AVAILABLE_MODES[0];

  return (
    <div className={`terminal-mode-card mode-${mode.toLowerCase()}`} style={{ borderColor: `${cfg.color}45` }}>
      <div className="terminal-mode-card-header">
        <div
          className="engine-badge"
          style={{
            color: cfg.color,
            backgroundColor: `${cfg.color}15`,
            borderColor: `${cfg.color}35`,
          }}
        >
          <Sliders size={13} />
          <span>{cfg.slug}</span>
        </div>
        <span className="engine-name" style={{ color: cfg.color }}>
          {cfg.name}
        </span>
      </div>

      <div className="terminal-mode-card-speech">
        <div className="speech-badge" style={{ color: cfg.color }}>
          [ CHẾ ĐỘ: {cfg.name.toUpperCase()} ({cfg.slug}) ]
        </div>
        <div className="speech-quote">"{cfg.tagline}"</div>

        <ul className="speech-bullets">
          {cfg.capabilities.map((c, idx) => (
            <li key={idx}>
              <span style={{ color: cfg.color }}>•</span> {c}
            </li>
          ))}
        </ul>

        <div className="speech-hint">
          Nhập yêu cầu hoặc câu hỏi bên dưới. Nhấn '/' để đổi mode bất cứ lúc nào.
        </div>
      </div>
    </div>
  );
};
