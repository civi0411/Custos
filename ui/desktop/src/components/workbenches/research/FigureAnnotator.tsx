import React, { useState, useRef } from 'react';
import {
  MapPin,
  Sparkles,
  Send,
  Trash2,
  X,
  ZoomIn,
  ZoomOut,
  RotateCcw,
  Check,
  Split,
} from 'lucide-react';
import { AnnotationRecord } from '@/types/research';

interface FigureAnnotatorProps {
  imageUrl: string;
  artifactPath: string;
  artifactVersion?: number;
  initialAnnotations?: AnnotationRecord[];
  onSaveAnnotation?: (annotation: AnnotationRecord) => void;
  onStartSideChat?: (annotation: AnnotationRecord, prompt: string) => void;
  onSubmitBatch?: (annotations: AnnotationRecord[]) => void;
  onDeleteAnnotation?: (id: string) => void;
}

export const FigureAnnotator: React.FC<FigureAnnotatorProps> = ({
  imageUrl,
  artifactPath,
  artifactVersion = 1,
  initialAnnotations = [],
  onSaveAnnotation,
  onStartSideChat,
  onSubmitBatch,
  onDeleteAnnotation,
}) => {
  const [annotations, setAnnotations] = useState<AnnotationRecord[]>(initialAnnotations);
  const [activePin, setActivePin] = useState<{ x: number; y: number } | null>(null);
  const [noteText, setNoteText] = useState('');
  const [zoomLevel, setZoomLevel] = useState(1);
  const [selectedAnnId, setSelectedAnnId] = useState<string | null>(null);
  const imgRef = useRef<HTMLImageElement>(null);

  const pendingAnnotations = annotations.filter((a) => a.status === 'pending');

  const handleImageClick = (e: React.MouseEvent<HTMLImageElement>) => {
    if (!imgRef.current) return;
    const rect = imgRef.current.getBoundingClientRect();
    const xRatio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    const yRatio = Math.max(0, Math.min(1, (e.clientY - rect.top) / rect.height));

    setActivePin({ x: xRatio, y: yRatio });
    setNoteText('');
    setSelectedAnnId(null);
  };

  const handleSavePin = () => {
    if (!activePin) return;
    const newPinNumber = annotations.length + 1;
    const newAnn: AnnotationRecord = {
      id: `ann_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
      artifactPath,
      artifactVersion,
      target: {
        type: 'image_pin',
        xRatio: activePin.x,
        yRatio: activePin.y,
        pinNumber: newPinNumber,
      },
      note: noteText.trim() || 'Inspect marked region',
      actor: 'researcher',
      status: 'pending',
      createdAt: Date.now(),
    };

    const next = [...annotations, newAnn];
    setAnnotations(next);
    onSaveAnnotation?.(newAnn);
    setActivePin(null);
    setNoteText('');
  };

  const handleAskSideChat = () => {
    if (!activePin) return;
    const prompt = noteText.trim() || "What's going on here?";
    const newPinNumber = annotations.length + 1;
    const newAnn: AnnotationRecord = {
      id: `ann_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
      artifactPath,
      artifactVersion,
      target: {
        type: 'image_pin',
        xRatio: activePin.x,
        yRatio: activePin.y,
        pinNumber: newPinNumber,
      },
      note: prompt,
      actor: 'researcher',
      status: 'submitted',
      sideChatSessionId: `side_chat_${Date.now()}`,
      createdAt: Date.now(),
    };

    const next = [...annotations, newAnn];
    setAnnotations(next);
    onSaveAnnotation?.(newAnn);
    onStartSideChat?.(newAnn, prompt);
    setActivePin(null);
    setNoteText('');
  };

  const handleDelete = (id: string) => {
    setAnnotations((prev) => prev.filter((a) => a.id !== id));
    if (selectedAnnId === id) setSelectedAnnId(null);
    onDeleteAnnotation?.(id);
  };

  const handleSubmitBatch = () => {
    if (pendingAnnotations.length === 0) return;
    const submitted = annotations.map((a) =>
      a.status === 'pending' ? { ...a, status: 'submitted' as const } : a
    );
    setAnnotations(submitted);
    onSubmitBatch?.(pendingAnnotations);
  };

  return (
    <div className="flex flex-col h-full bg-[var(--color-canvas)] text-[var(--color-editor-fg)] select-none font-sans relative overflow-hidden">
      {/* Header controls */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-[var(--color-border-muted)] bg-[var(--color-surface-1)] z-20">
        <div className="flex items-center gap-2 text-xs text-[var(--color-fg-muted)]">
          <MapPin size={13} className="workbench-accent" />
          <span className="font-semibold text-[var(--color-editor-fg)]">Figure Annotator</span>
          <span className="text-[var(--color-border-default)]">·</span>
          <span>Click anywhere on figure to drop numbered pin</span>
        </div>

        <div className="flex items-center gap-2">
          {/* Zoom controls */}
          <div className="flex items-center bg-[var(--color-surface-2)] border border-[var(--color-border-default)] rounded-lg p-0.5">
            <button
              onClick={() => setZoomLevel((z) => Math.max(0.5, z - 0.25))}
              className="p-1 hover:text-[var(--color-editor-fg)] text-[var(--color-fg-muted)] rounded transition"
              title="Zoom out"
            >
              <ZoomOut size={13} />
            </button>
            <span className="text-[11px] font-mono px-2 text-[var(--color-editor-fg)] font-semibold">
              {Math.round(zoomLevel * 100)}%
            </span>
            <button
              onClick={() => setZoomLevel((z) => Math.min(3, z + 0.25))}
              className="p-1 hover:text-[var(--color-editor-fg)] text-[var(--color-fg-muted)] rounded transition"
              title="Zoom in"
            >
              <ZoomIn size={13} />
            </button>
            <button
              onClick={() => setZoomLevel(1)}
              className="p-1 hover:text-[var(--color-editor-fg)] text-[var(--color-fg-muted)] rounded transition ml-0.5"
              title="Reset zoom"
            >
              <RotateCcw size={12} />
            </button>
          </div>
        </div>
      </div>

      {/* Main Canvas Area */}
      <div className="flex-1 overflow-auto relative p-6 flex items-center justify-center bg-[var(--color-canvas-inset)]">
        <div
          className="relative inline-block transition-transform duration-150 origin-center"
          style={{ transform: `scale(${zoomLevel})` }}
        >
          <img
            ref={imgRef}
            src={imageUrl}
            alt="Research Figure"
            onClick={handleImageClick}
            className="max-w-full max-h-[70vh] rounded-xl border border-[var(--color-border-default)] shadow-lg cursor-crosshair object-contain bg-[var(--color-canvas)]"
          />

          {/* Render Existing Pins */}
          {annotations.map((ann, idx) => {
            if (ann.target.type !== 'image_pin' || ann.target.xRatio === undefined) return null;
            const isSelected = selectedAnnId === ann.id;
            const isPending = ann.status === 'pending';
            const pinNum = ann.target.pinNumber ?? idx + 1;

            return (
              <div
                key={ann.id}
                onClick={(e) => {
                  e.stopPropagation();
                  setSelectedAnnId(isSelected ? null : ann.id);
                  setActivePin(null);
                }}
                style={{
                  left: `${ann.target.xRatio * 100}%`,
                  top: `${ann.target.yRatio! * 100}%`,
                }}
                className="absolute -translate-x-1/2 -translate-y-1/2 cursor-pointer z-30 group"
              >
                <div
                  className={`w-5 h-5 rounded-full flex items-center justify-center font-bold text-[10px] shadow transition-all ${
                    isPending
                      ? 'bg-purple-600 text-white ring-2 ring-purple-400 ring-offset-2 ring-offset-[var(--color-canvas)] animate-pulse'
                      : 'bg-emerald-600 text-white ring-1 ring-emerald-400'
                  } ${isSelected ? 'scale-125 ring-2 ring-[var(--color-editor-fg)]' : 'hover:scale-110'}`}
                >
                  {pinNum}
                </div>

                {/* Tooltip on hover/select */}
                {(isSelected || isPending) && (
                  <div
                    className="absolute left-6 top-0 w-64 bg-[var(--color-surface-1)] border border-[var(--color-border-default)] rounded-xl p-3 shadow-xl text-xs z-40 text-[var(--color-editor-fg)] pointer-events-auto"
                    onClick={(e) => e.stopPropagation()}
                  >
                    <div className="flex items-center justify-between pb-1.5 border-b border-[var(--color-border-muted)] mb-2 text-[10px] text-[var(--color-fg-muted)]">
                      <span className="font-semibold uppercase tracking-wider text-[var(--workbench-accent)]">
                        Pin #{pinNum} · {ann.status}
                      </span>
                      <button
                        onClick={() => handleDelete(ann.id)}
                        className="text-[var(--color-fg-subtle)] hover:text-rose-500 transition"
                      >
                        <Trash2 size={11} />
                      </button>
                    </div>
                    <p className="text-xs text-[var(--color-editor-fg)] leading-relaxed select-text">{ann.note}</p>
                    {ann.sideChatSessionId && (
                      <div className="mt-2 text-[10px] text-blue-500 flex items-center gap-1 font-mono">
                        <Split size={10} /> Side-Chat Active
                      </div>
                    )}
                  </div>
                )}
              </div>
            );
          })}

          {/* Active Pin Placement Dialog */}
          {activePin && (
            <div
              style={{
                left: `${activePin.x * 100}%`,
                top: `${activePin.y * 100}%`,
              }}
              className="absolute -translate-x-1/2 -translate-y-1/2 z-40"
              onClick={(e) => e.stopPropagation()}
            >
              {/* Pulsing indicator */}
              <div className="w-5 h-5 rounded-full bg-purple-600 text-white flex items-center justify-center text-[10px] font-bold shadow-lg ring-2 ring-[var(--color-editor-fg)]">
                {annotations.length + 1}
              </div>

              {/* Pin Popover */}
              <div className="absolute left-6 top-0 w-72 bg-[var(--color-surface-1)] border border-[var(--color-border-default)] rounded-xl p-3.5 shadow-2xl text-xs z-50">
                <div className="flex items-center justify-between pb-2 border-b border-[var(--color-border-muted)] mb-2.5">
                  <span className="font-semibold text-xs text-[var(--color-editor-fg)] flex items-center gap-1.5">
                    <Sparkles size={12} className="workbench-accent" /> Figure Annotation
                  </span>
                  <button
                    onClick={() => setActivePin(null)}
                    className="text-[var(--color-fg-subtle)] hover:text-[var(--color-editor-fg)]"
                  >
                    <X size={12} />
                  </button>
                </div>

                <textarea
                  value={noteText}
                  onChange={(e) => setNoteText(e.target.value)}
                  placeholder="e.g. Change Y axis to log scale, or inspect error bars..."
                  rows={3}
                  className="w-full bg-[var(--color-canvas)] border border-[var(--color-border-default)] rounded-lg p-2.5 text-xs text-[var(--color-editor-fg)] placeholder-[var(--color-fg-subtle)] focus:outline-none focus:border-[var(--workbench-accent)] resize-none mb-3"
                  autoFocus
                />

                <div className="flex items-center gap-2">
                  <button
                    onClick={handleSavePin}
                    className="flex-1 flex items-center justify-center gap-1.5 py-1.5 px-3 rounded-lg bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] text-[var(--color-editor-fg)] border border-[var(--color-border-default)] font-medium text-xs transition"
                  >
                    <Check size={12} className="text-emerald-500" />
                    Save Pin
                  </button>
                  <button
                    onClick={handleAskSideChat}
                    className="flex-1 flex items-center justify-center gap-1.5 py-1.5 px-3 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-3)] hover:bg-[var(--color-surface-2)] text-[var(--color-editor-fg)] font-medium text-xs transition"
                    title="Start independent side-chat branch"
                  >
                    <Split size={12} />
                    Side-Chat
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Pending Annotations Bar (Above Composer Queue) */}
      {pendingAnnotations.length > 0 && (
        <div className="border-t border-[var(--color-border-muted)] bg-[var(--color-surface-1)] p-2.5 px-4 z-20 flex items-center justify-between gap-4">
          <div className="flex items-center gap-2 overflow-x-auto py-1">
            <span className="text-[11px] font-semibold text-[var(--color-fg-muted)] shrink-0">
              Pending Pins ({pendingAnnotations.length}):
            </span>
            {pendingAnnotations.map((ann, i) => (
              <div
                key={ann.id}
                className="flex items-center gap-1.5 bg-[var(--color-surface-2)] border border-[var(--color-border-muted)] rounded-lg px-2.5 py-1 text-xs text-[var(--color-editor-fg)] shrink-0"
              >
                <span className="w-3.5 h-3.5 rounded-full bg-purple-500/20 text-purple-400 flex items-center justify-center text-[9px] font-bold">
                  {ann.target.type === 'image_pin' ? ann.target.pinNumber : i + 1}
                </span>
                <span className="max-w-[120px] truncate text-[var(--color-editor-fg)]">{ann.note}</span>
                <button
                  onClick={() => handleDelete(ann.id)}
                  className="text-[var(--color-fg-subtle)] hover:text-rose-500 ml-1"
                >
                  <X size={10} />
                </button>
              </div>
            ))}
          </div>

          <button
            onClick={handleSubmitBatch}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-[var(--color-border-default)] bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)] text-[var(--color-editor-fg)] font-medium text-xs shrink-0 transition"
          >
            <Send size={12} />
            Send All to Agent
          </button>
        </div>
      )}
    </div>
  );
};
