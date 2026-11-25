import React from 'react';
import { File, FileCode, FileText } from 'lucide-react';
import type { WorkspaceFileEntry } from '@/types/domain';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';

export interface FilesChangesPaneProps {
  sessionId?: string;
  initialWorkspaceId?: string;
  initialFilePath?: string;
  initialViewMode?: 'content' | 'diff';
  activeWorkspaceId?: string;
  onSelectWorkspace?: (id: string) => void;
  onShowToast?: (msg: string) => void;
  onOpenTab?: (id: OrcaTabId, customUrl?: string, customTitle?: string) => void;
}

export function getFileIcon(name: string): React.ReactElement {
  const ext = name.split('.').pop()?.toLowerCase();
  switch (ext) {
    case 'ts':
    case 'tsx':
    case 'js':
    case 'jsx':
    case 'rs':
    case 'py':
    case 'go':
    case 'c':
    case 'cpp':
    case 'html':
    case 'css':
      return <FileCode className="h-3.5 w-3.5 text-white/60 shrink-0" />;
    case 'json':
    case 'yaml':
    case 'yml':
    case 'toml':
    case 'md':
    case 'txt':
      return <FileText className="h-3.5 w-3.5 text-white/50 shrink-0" />;
    default:
      return <File className="h-3.5 w-3.5 text-[var(--color-fg-muted)] shrink-0" />;
  }
}

export interface TreeNode {
  entry?: WorkspaceFileEntry;
  name: string;
  path: string;
  isDir: boolean;
  children: Map<string, TreeNode>;
}

export function checkIsDir(entry?: WorkspaceFileEntry): boolean {
  if (!entry) return false;
  return Boolean(
    (entry as any).isDir ??
    (entry as any).is_dir
  );
}

export function buildFileTree(entries: WorkspaceFileEntry[]): TreeNode {
  const root: TreeNode = {
    name: 'root',
    path: '',
    isDir: true,
    children: new Map(),
  };

  for (const entry of entries) {
    const parts = entry.path.split('/').filter(Boolean);
    let current = root;
    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isLast = i === parts.length - 1;
      const nodePath = parts.slice(0, i + 1).join('/');

      if (!current.children.has(part)) {
        current.children.set(part, {
          entry: isLast ? entry : undefined,
          name: part,
          path: nodePath,
          isDir: isLast ? checkIsDir(entry) : true,
          children: new Map(),
        });
      } else if (isLast && entry) {
        const existing = current.children.get(part)!;
        existing.entry = entry;
        existing.isDir = checkIsDir(entry);
      }
      current = current.children.get(part)!;
    }
  }

  return root;
}
