import {
  CheckCircle2,
  XCircle,
  AlertTriangle,
  Clock,
  FileCode,
  GitBranch,
  CheckSquare,
  Terminal,
  FileText,
  Cpu,
  ShieldCheck,
} from 'lucide-react';
import type {
  ReviewStatus,
  FindingSeverity,
  ReviewTargetType,
} from '@/types/domain';

export const renderStatusBadge = (status: ReviewStatus) => {
  switch (status) {
    case 'approved':
      return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
          <CheckCircle2 className="w-3 h-3" /> Approved
        </span>
      );
    case 'rejected':
      return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-rose-500/10 text-rose-400 border border-rose-500/20">
          <XCircle className="w-3 h-3" /> Rejected
        </span>
      );
    case 'degraded':
      return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20">
          <AlertTriangle className="w-3 h-3" /> Degraded
        </span>
      );
    case 'pending':
      return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium bg-sky-500/10 text-sky-400 border border-sky-500/20">
          <Clock className="w-3 h-3" /> Pending
        </span>
      );
    default:
      return null;
  }
};

export const renderSeverityBadge = (severity: FindingSeverity) => {
  switch (severity) {
    case 'info':
      return (
        <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20">
          info
        </span>
      );
    case 'warning':
      return (
        <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20">
          warning
        </span>
      );
    case 'error':
      return (
        <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-rose-500/10 text-rose-400 border border-rose-500/20">
          error
        </span>
      );
    case 'blocker':
      return (
        <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-red-600/20 text-red-300 border border-red-500/30">
          blocker
        </span>
      );
    default:
      return null;
  }
};

export const renderTargetIcon = (targetType: ReviewTargetType) => {
  switch (targetType) {
    case 'artifact':
      return <FileCode className="w-3.5 h-3.5 text-blue-400" />;
    case 'diff':
      return <GitBranch className="w-3.5 h-3.5 text-emerald-400" />;
    case 'task':
      return <CheckSquare className="w-3.5 h-3.5 text-purple-400" />;
    case 'run':
      return <Terminal className="w-3.5 h-3.5 text-amber-400" />;
    case 'note':
      return <FileText className="w-3.5 h-3.5 text-teal-400" />;
    case 'workspace':
      return <Cpu className="w-3.5 h-3.5 text-rose-400" />;
    default:
      return <ShieldCheck className="w-3.5 h-3.5 text-fg-muted" />;
  }
};
