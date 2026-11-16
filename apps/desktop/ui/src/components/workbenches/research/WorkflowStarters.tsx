import React from 'react';
import { FlaskConical, LineChart, FileSearch, Globe2, ArrowRight } from 'lucide-react';

export interface ResearchStarter {
  id: string;
  title: string;
  description: string;
  icon: React.ElementType;
  iconColor: string;
  prompt: string;
}

export const RESEARCH_STARTERS: ResearchStarter[] = [
  {
    id: 'demo',
    title: 'End-to-End Analysis',
    description: 'Simulate data, fit statistical model, generate publication figure and write markdown report.',
    icon: FlaskConical,
    iconColor: 'text-white/60',
    prompt:
      'Run a complete scientific analysis end to end: generate or load a dataset, perform statistical analysis (model fitting and summary stats), save one publication-quality figure as a PNG artifact, and write a summary report citing the exact computed numbers.',
  },
  {
    id: 'analyze',
    title: 'Workspace Dataset Exploration',
    description: 'Explore an existing dataset, run analysis in Python/R, and generate traced charts.',
    icon: LineChart,
    iconColor: 'text-white/60',
    prompt:
      'Analyze the scientific dataset in this workspace end to end: inspect columns and missing values, compute key correlation and distribution statistics, save at least one figure as an artifact, and write findings with every number traced to producing code.',
  },
  {
    id: 'audit',
    title: 'Traceability & Citation Review',
    description: 'Audit report or manuscript: resolve citations, check claims, flag numbers without sources.',
    icon: FileSearch,
    iconColor: 'text-white/60',
    prompt:
      'Perform a traceability review on the scientific manuscript/report in this workspace: verify all citations against real literature, check whether figures match generating code, and flag any numbers that lack an exact computational provenance.',
  },
  {
    id: 'climate-trends',
    title: 'Climate Benchmark Analysis',
    description: 'Analyze real-world time-series anomalies, quantify decadal warming rates, and export figures.',
    icon: Globe2,
    iconColor: 'text-white/60',
    prompt:
      'Perform a climate time-series analysis: load land-ocean temperature anomaly series, calculate the warming rate per decade, compare multi-decadal means, output a publication-ready warming trend plot, and produce an executive findings brief.',
  },
];

interface WorkflowStartersProps {
  onPick: (prompt: string) => void;
  className?: string;
}

export const WorkflowStarters: React.FC<WorkflowStartersProps> = ({ onPick, className = '' }) => {
  return (
    <div className={`flex flex-col items-center justify-center p-6 text-center select-none ${className}`}>
      <div className="w-full max-w-2xl space-y-6">
        <div>
          <span className="text-[10px] font-semibold tracking-widest text-white/50 uppercase font-mono">
            Research Workbench Starters
          </span>
          <h2 className="text-xl font-semibold text-white/90 mt-1.5">
            Accelerate Scientific Investigation
          </h2>
          <p className="text-xs text-white/40 mt-1 max-w-md mx-auto leading-relaxed">
            Choose a starting prompt, review it in the conversation, and adapt it to your sources and workspace.
          </p>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 text-left">
          {RESEARCH_STARTERS.map((s) => {
            const Icon = s.icon;
            return (
              <button
                key={s.id}
                type="button"
                onClick={() => onPick(s.prompt)}
                className="group flex flex-col justify-between p-3.5 rounded-xl border border-white/10 bg-white/[0.03] hover:bg-white/[0.06] hover:border-white/20 transition cursor-pointer text-xs"
              >
                <div>
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <div className="p-1.5 rounded-lg bg-white/[0.04] border border-white/8">
                        <Icon className={`size-4 ${s.iconColor}`} />
                      </div>
                      <span className="font-semibold text-white/80 group-hover:text-white transition">
                        {s.title}
                      </span>
                    </div>
                    <ArrowRight className="size-3.5 text-white/30 group-hover:text-white/80 group-hover:translate-x-0.5 transition-transform" />
                  </div>
                  <p className="text-[11px] text-white/40 leading-relaxed">
                    {s.description}
                  </p>
                </div>
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
};
