import React, { useState } from 'react';
import { ArtifactInspectorData } from '@/types/research';
import { ArtifactInspector } from './ArtifactInspector';
import { FileCode } from 'lucide-react';

interface DeepInspectorPaneProps {
  data?: ArtifactInspectorData | null;
  onClose?: () => void;
  onShowToast?: (msg: string) => void;
}

export const DeepInspectorPane: React.FC<DeepInspectorPaneProps> = ({
  data: externalData,
  onClose,
  onShowToast: _onShowToast,
}) => {
  const [data] = useState<ArtifactInspectorData | null>(() => externalData ?? null);
  const [isMaximized, setIsMaximized] = useState(false);

  if (!data) {
    return (
      <div className="h-full w-full flex flex-col items-center justify-center p-8 text-center bg-[var(--color-canvas)] text-[var(--color-editor-fg)]">
        <div className="w-12 h-12 rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-border-default)] flex items-center justify-center text-[var(--color-fg-muted)] mb-3">
          <FileCode className="w-5 h-5" />
        </div>
        <h3 className="text-sm font-semibold text-[var(--color-editor-fg)]">No Artifact Selected</h3>
        <p className="text-xs text-[var(--color-fg-muted)] mt-1.5 max-w-xs leading-relaxed">
          Select an artifact, research claim, or execution receipt from the workbench to inspect proof-closure and evidence chains.
        </p>
        {onClose && (
          <button
            onClick={onClose}
            className="mt-4 px-3 py-1.5 text-xs bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] text-[var(--color-editor-fg)] rounded-lg border border-[var(--color-border-default)] transition"
          >
            Close Inspector
          </button>
        )}
      </div>
    );
  }

  return (
    <div className="h-full w-full">
      <ArtifactInspector
        data={data}
        onClose={onClose}
        isMaximized={isMaximized}
        onToggleMaximize={() => setIsMaximized(!isMaximized)}
      />
    </div>
  );
};
