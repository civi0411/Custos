import React from 'react';
import { FlaskConical, FolderPlus } from 'lucide-react';
import type { WorkspaceDetectionResult } from '@/types/domain';

export interface ResearchStudyExplorerProps {
  researchPath: string | null;
  researchDetection: WorkspaceDetectionResult | null;
  handleOpenStudyFolder: () => void | Promise<void>;
}

export const ResearchStudyExplorer: React.FC<ResearchStudyExplorerProps> = ({
  researchPath,
  researchDetection,
  handleOpenStudyFolder,
}) => {
  if (!researchPath) {
    return (
      <div className="p-4 text-center space-y-3 border border-dashed border-white/10 rounded-xl my-4">
        <FlaskConical className="size-7 text-white/40 mx-auto" />
        <div className="space-y-1">
          <div className="text-white/80 font-medium text-xs">No research study opened</div>
          <div className="text-white/40 text-[11px] leading-relaxed">
            Open a study folder, scientific datasets, or experiment notebooks.
          </div>
        </div>
        <button
          type="button"
          onClick={handleOpenStudyFolder}
          className="px-3 py-1.5 bg-white/[0.06] hover:bg-white/[0.1] text-white/90 border border-white/10 text-xs rounded-lg transition font-medium cursor-pointer flex items-center gap-1.5 mx-auto"
        >
          <FolderPlus className="size-3.5" />
          <span>Open Study Folder (⌘O)</span>
        </button>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <div className="p-2.5 rounded-xl border border-white/10 bg-white/[0.03] space-y-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 min-w-0">
            <FlaskConical className="size-4 text-white/70 shrink-0" />
            <span className="font-bold text-xs text-white truncate">
              {researchDetection?.name || researchPath.split('/').filter(Boolean).pop()}
            </span>
          </div>
          <span className="text-[9px] font-mono px-1.5 py-0.2 rounded bg-white/[0.06] text-white/60 border border-white/10">
            Study
          </span>
        </div>

        <div className="text-[10px] text-white/40 font-mono truncate" title={researchPath}>
          {researchPath}
        </div>

        <div className="pt-1.5 border-t border-white/6 flex items-center justify-between">
          <span className="text-[10.5px] text-white/60">Study workspace ready</span>
          <button
            type="button"
            onClick={handleOpenStudyFolder}
            className="text-[10px] text-white/60 hover:text-white transition underline cursor-pointer"
          >
            Switch study
          </button>
        </div>
      </div>
    </div>
  );
};
