export interface LMStudioModelItem {
  id: string;
  name?: string;
  architecture?: string;
  quantization?: string;
  contextLength?: number;
  isLoaded?: boolean;
  type?: string;
}

export interface LocalAiUnifiedPaneProps {
  onShowToast?: (msg: string) => void;
  onNavigateToWorkbench?: () => void;
}

export function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 B';
  const gb = bytes / (1024 * 1024 * 1024);
  if (gb >= 1) return `${gb.toFixed(1)} GB`;
  const mb = bytes / (1024 * 1024);
  return `${mb.toFixed(0)} MB`;
}
