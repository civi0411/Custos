import {
  ShieldCheck,
  CheckCircle2,
  XCircle,
  Clock,
  Lock,
  FileCheck,
  AlertCircle,
} from 'lucide-react';
import type { SynthesisProposalStatus } from '@/types/domain';
import type { ClaimGroundingLevel } from '@/types/research';

export const renderGroundingBadge = (level: ClaimGroundingLevel | string) => {
  switch (level) {
    case 'L3_SEALED':
      return (
        <span className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-white/[0.08] text-white border border-white/15 font-mono text-[10px] font-semibold">
          <Lock size={10} /> L3 SEALED
        </span>
      );
    case 'L2_VERIFIED':
      return (
        <span className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-white/[0.06] text-white/90 border border-white/10 font-mono text-[10px] font-semibold">
          <CheckCircle2 size={10} /> L2 VERIFIED
        </span>
      );
    case 'L1_CITED':
      return (
        <span className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-white/[0.04] text-white/70 border border-white/10 font-mono text-[10px] font-semibold">
          <FileCheck size={10} /> L1 CITED
        </span>
      );
    case 'L0_UNGROUNDED':
    default:
      return (
        <span className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-white/[0.02] text-white/40 border border-white/5 font-mono text-[10px] font-semibold">
          <AlertCircle size={10} /> L0 UNGROUNDED
        </span>
      );
  }
};

export const renderStatusBadge = (status: SynthesisProposalStatus) => {
  switch (status) {
    case 'handoff_completed':
      return (
        <span className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-white/[0.08] text-white border border-white/15 text-[10px] font-semibold uppercase tracking-wider">
          <CheckCircle2 size={10} /> Completed
        </span>
      );
    case 'submitted':
      return (
        <span className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-white/[0.06] text-white/80 border border-white/10 text-[10px] font-semibold uppercase tracking-wider">
          <ShieldCheck size={10} /> Submitted
        </span>
      );
    case 'rejected':
      return (
        <span className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-white/[0.03] text-white/40 border border-white/5 text-[10px] font-semibold uppercase tracking-wider">
          <XCircle size={10} /> Rejected
        </span>
      );
    case 'draft':
    default:
      return (
        <span className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-white/[0.04] text-white/60 border border-white/10 text-[10px] font-semibold uppercase tracking-wider">
          <Clock size={10} /> Draft
        </span>
      );
  }
};
