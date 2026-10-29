import { File, FileCode, FileText } from 'lucide-react';

export function getFileIcon(name: string) {
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
