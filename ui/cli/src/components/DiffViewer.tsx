import React, { useState } from 'react';
import { Copy, Check, FileCode2 } from 'lucide-react';

interface DiffViewerProps {
  filePath: string;
  oldCode: string;
  newCode: string;
  compact?: boolean;
}

export const DiffViewer: React.FC<DiffViewerProps> = ({
  filePath,
  oldCode,
  newCode,
  compact = false,
}) => {
  const [copied, setCopied] = useState(false);

  const oldLines = oldCode.split('\n');
  const newLines = newCode.split('\n');

  interface DiffLine {
    type: 'add' | 'del' | 'eq';
    content: string;
    oldNum?: number;
    newNum?: number;
  }

  const diffLines: DiffLine[] = [];
  let additions = 0;
  let deletions = 0;

  oldLines.forEach((line, idx) => {
    if (!newLines.includes(line)) {
      deletions++;
      diffLines.push({ type: 'del', content: line, oldNum: idx + 1 });
    }
  });

  newLines.forEach((line, idx) => {
    if (!oldLines.includes(line)) {
      additions++;
      diffLines.push({ type: 'add', content: line, newNum: idx + 1 });
    } else {
      diffLines.push({ type: 'eq', content: line, newNum: idx + 1 });
    }
  });

  const handleCopy = () => {
    const raw = `--- a/${filePath}\n+++ b/${filePath}\n` +
      diffLines.map((l) => (l.type === 'add' ? `+${l.content}` : l.type === 'del' ? `-${l.content}` : ` ${l.content}`)).join('\n');
    navigator.clipboard.writeText(raw);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className={`diff-viewer-card ${compact ? 'compact' : ''}`}>
      <div className="diff-header">
        <div className="diff-title-wrapper">
          <span className="diff-badge">DIFF</span>
          <FileCode2 size={14} className="diff-file-icon" />
          <span className="diff-filepath">{filePath}</span>
        </div>

        <div className="diff-actions">
          <div className="diff-summary-stats">
            <span className="additions-count">+{additions}</span>
            <span className="deletions-count">-{deletions}</span>
          </div>
          <button className="diff-copy-btn" onClick={handleCopy} title="Copy raw unified diff">
            {copied ? <Check size={12} className="copy-ok" /> : <Copy size={12} />}
            <span>{copied ? 'Copied' : 'Copy'}</span>
          </button>
        </div>
      </div>

      <div className="diff-meta-stripes">
        <div className="diff-meta-line del">--- a/{filePath}</div>
        <div className="diff-meta-line add">+++ b/{filePath}</div>
      </div>

      <div className="diff-code-container">
        <div className="diff-hunk-header">@@ -1,{oldLines.length} +1,{newLines.length} @@</div>
        {diffLines.map((line, index) => (
          <div key={index} className={`diff-line diff-line-${line.type}`}>
            <span className="diff-line-indicator">
              {line.type === 'add' ? '+' : line.type === 'del' ? '-' : ' '}
            </span>
            <span className="diff-line-number">
              {line.newNum || line.oldNum || ''}
            </span>
            <span className="diff-line-text">{line.content || ' '}</span>
          </div>
        ))}
      </div>

      <div className="diff-footer">
        <span className="summary-label">Summary:</span>
        <span className="summary-add">+{additions} additions</span>,{' '}
        <span className="summary-del">-{deletions} deletions</span>
      </div>
    </div>
  );
};
