import React from 'react';
import { Save, Tag } from 'lucide-react';

export interface NoteEditorProps {
  title: string;
  onTitleChange: (title: string) => void;
  tagsInput: string;
  onTagsInputChange: (tags: string) => void;
  content: string;
  onContentChange: (content: string) => void;
  viewMode: 'edit' | 'preview' | 'split';
  isDirty: boolean;
  isSaving: boolean;
  onSave: () => void;
}

export const NoteEditor: React.FC<NoteEditorProps> = ({
  title,
  onTitleChange,
  tagsInput,
  onTagsInputChange,
  content,
  onContentChange,
  viewMode,
  isDirty,
  isSaving,
  onSave,
}) => {
  return (
    <div className="flex flex-1 flex-col overflow-hidden bg-[var(--color-canvas)]">
      {/* Metadata Bar */}
      <div className="border-b border-[var(--color-border-default)] px-4 py-2.5 bg-[var(--color-surface-1)]">
        <div className="flex items-center gap-3">
          <input
            type="text"
            placeholder="Note Title"
            value={title}
            onChange={(e) => onTitleChange(e.target.value)}
            className="flex-1 bg-transparent text-sm font-semibold text-[var(--color-editor-fg)] placeholder:text-[var(--color-fg-subtle)] focus:outline-none"
          />

          {isDirty && (
            <button
              onClick={onSave}
              disabled={isSaving}
              className="flex items-center gap-1 rounded-md bg-white hover:bg-white/90 px-2.5 py-1 text-xs font-semibold text-black transition shadow-sm"
              title="Save now"
            >
              <Save className="h-3 w-3 text-black" />
              Save
            </button>
          )}
        </div>

        <div className="mt-1.5 flex items-center gap-2">
          <Tag className="h-3 w-3 text-[var(--color-fg-subtle)] shrink-0" />
          <input
            type="text"
            placeholder="Tags (comma-separated, e.g. task, prompt, experiment)"
            value={tagsInput}
            onChange={(e) => onTagsInputChange(e.target.value)}
            className="w-full bg-transparent text-[11px] text-[var(--color-fg-muted)] placeholder:text-[var(--color-fg-subtle)] focus:outline-none"
          />
        </div>
      </div>

      {/* Editor Workspace */}
      <div className="flex flex-1 overflow-hidden">
        {/* Markdown Textarea */}
        {(viewMode === 'edit' || viewMode === 'split') && (
          <div
            className={`flex-1 h-full p-4 overflow-y-auto ${
              viewMode === 'split' ? 'border-r border-[var(--color-border-default)]' : ''
            }`}
          >
            <textarea
              value={content}
              onChange={(e) => onContentChange(e.target.value)}
              placeholder="Start writing markdown notes..."
              className="h-full w-full resize-none bg-transparent font-mono text-xs leading-relaxed text-[var(--color-editor-fg)] placeholder:text-[var(--color-fg-subtle)] focus:outline-none"
            />
          </div>
        )}

        {/* Markdown Rendered Preview */}
        {(viewMode === 'preview' || viewMode === 'split') && (
          <div className="flex-1 h-full p-6 overflow-y-auto prose prose-invert prose-xs max-w-none">
            <div className="text-xs leading-relaxed text-[var(--color-editor-fg)] whitespace-pre-wrap font-sans">
              {content}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
