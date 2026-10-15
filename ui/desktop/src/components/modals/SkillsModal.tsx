import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Sparkles,
  X,
  Search,
  Dna,
  BookOpen,
  FlaskConical,
  Binary,
  Code2,
  Workflow,
} from 'lucide-react';
import { useAppContext } from '@/context/AppContext';

export interface SkillsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

interface SkillItem {
  id: string;
  name: string;
  category: 'science' | 'code' | 'workflow';
  description: string;
  icon: React.ComponentType<{ className?: string }>;
  tags: string[];
}

const INITIAL_SKILLS: SkillItem[] = [
  {
    id: 'alphafold-database',
    name: 'AlphaFold 3D Structure Fetch & Analysis',
    category: 'science',
    description: 'Retrieve AlphaFold predicted protein structures by UniProt ID with pLDDT structural confidence and domain boundary analysis.',
    icon: Dna,
    tags: ['Proteins', 'AlphaFold', 'Structural Biology'],
  },
  {
    id: 'alphagenome-single-variant',
    name: 'AlphaGenome Variant Impact Scoring',
    category: 'science',
    description: 'Score non-coding variant functional effects on gene expression (RNA-seq), chromatin accessibility, and transcription factor binding.',
    icon: Binary,
    tags: ['Genomics', 'Variant Analysis', 'DeepMind'],
  },
  {
    id: 'chembl-database',
    name: 'ChEMBL Bioactive Molecules & Targets',
    category: 'science',
    description: 'Query ChEMBL for bioactive compounds, IC50/Ki affinities, approved drug mechanisms, and chemical SMILES substructure queries.',
    icon: FlaskConical,
    tags: ['Pharmacology', 'Small Molecules', 'ChEMBL'],
  },
  {
    id: 'pubmed-database',
    name: 'PubMed & PMC Biomedical Literature',
    category: 'science',
    description: 'Search PubMed for scientific papers and clinical trial publications with deep links to NCBI sequence records and compounds.',
    icon: BookOpen,
    tags: ['Literature', 'NCBI', 'Clinical Trials'],
  },
  {
    id: 'code-refactor-suite',
    name: 'Hexagonal Architecture Refactor Suite',
    category: 'code',
    description: 'Autonomous zero-I/O domain preservation and hexagonal port/adapter decoupling across Rust and TypeScript.',
    icon: Code2,
    tags: ['Rust', 'Hexagonal', 'Refactoring'],
  },
  {
    id: 'workflow-skill-creator',
    name: 'Dynamic Workflow Skill Generator',
    category: 'workflow',
    description: 'Distill multi-step developer interactions into reproducible prompt templates and autonomous subagent scripts.',
    icon: Workflow,
    tags: ['Meta', 'Workflows', 'Automation'],
  },
];

export const SkillsModal: React.FC<SkillsModalProps> = ({ isOpen, onClose }) => {
  const { showToast } = useAppContext();
  const [skills] = useState<SkillItem[]>(INITIAL_SKILLS);
  const [searchQuery, setSearchQuery] = useState('');
  const [activeCategory, setActiveCategory] = useState<'all' | 'science' | 'code' | 'workflow'>('all');

  const filtered = skills.filter((s) => {
    const matchesSearch =
      s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      s.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
      s.tags.some((t) => t.toLowerCase().includes(searchQuery.toLowerCase()));
    const matchesCat = activeCategory === 'all' || s.category === activeCategory;
    return matchesSearch && matchesCat;
  });

  const handleUseSkill = (skill: SkillItem) => {
    showToast(`Skill loaded: ${skill.name}`);
    onClose();
  };

  return (
    <AnimatePresence>
      {isOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={onClose}
            className="absolute inset-0 bg-black/60 backdrop-blur-sm"
          />

          <motion.div
            initial={{ scale: 0.95, opacity: 0, y: 10 }}
            animate={{ scale: 1, opacity: 1, y: 0 }}
            exit={{ scale: 0.95, opacity: 0, y: 10 }}
            transition={{ type: 'spring', damping: 25, stiffness: 300 }}
            className="relative flex flex-col h-[640px] w-full max-w-2xl overflow-hidden rounded-2xl border border-border-default/60 bg-[#19191c] text-editor-fg shadow-2xl z-10"
          >
            {/* Modal Header */}
            <div className="flex items-center justify-between border-b border-border-default/40 px-6 py-4">
              <div className="flex items-center gap-3">
                <div className="flex size-9 items-center justify-center rounded-xl bg-purple-500/10 text-purple-400">
                  <Sparkles className="size-5" />
                </div>
                <div>
                  <h2 className="text-base font-semibold text-white tracking-tight">Agent Skills & Toolsets</h2>
                  <p className="text-xs text-fg-muted">Domain-specific reasoning skills loaded on-demand by local agents</p>
                </div>
              </div>
              <button
                type="button"
                onClick={onClose}
                className="rounded-lg p-1.5 text-fg-muted hover:bg-white/5 hover:text-white transition"
              >
                <X className="size-4" />
              </button>
            </div>

            {/* Filter & Search Bar */}
            <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border-default/30 px-6 py-3 bg-[#161619]">
              <div className="flex items-center gap-1.5 overflow-x-auto text-xs">
                {(['all', 'science', 'code', 'workflow'] as const).map((cat) => (
                  <button
                    key={cat}
                    type="button"
                    onClick={() => setActiveCategory(cat)}
                    className={`rounded-lg px-2.5 py-1 font-medium capitalize transition ${
                      activeCategory === cat
                        ? 'bg-white/10 text-white shadow-sm'
                        : 'text-fg-muted hover:bg-white/5 hover:text-white'
                    }`}
                  >
                    {cat}
                  </button>
                ))}
              </div>

              <div className="relative flex-1 max-w-xs">
                <Search className="absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-fg-muted" />
                <input
                  type="text"
                  placeholder="Search skills & keywords…"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="w-full rounded-lg border border-border-default/40 bg-surface-1 py-1.5 pl-8 pr-3 text-xs text-white placeholder-fg-muted/60 focus:border-workbench-accent focus:outline-none"
                />
              </div>
            </div>

            {/* Skills List */}
            <div className="flex-1 overflow-y-auto p-6 space-y-3">
              {filtered.map((skill) => {
                const Icon = skill.icon;
                return (
                  <div
                    key={skill.id}
                    className="flex items-start justify-between gap-4 rounded-xl border border-border-default/40 bg-surface-1/60 p-4 transition hover:border-border-default hover:bg-surface-1"
                  >
                    <div className="flex items-start gap-3.5">
                      <div className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-purple-500/10 text-purple-400 border border-purple-500/20 mt-0.5">
                        <Icon className="size-5" />
                      </div>
                      <div className="space-y-1.5">
                        <span className="text-sm font-semibold text-white">{skill.name}</span>
                        <p className="text-xs text-fg-muted leading-relaxed line-clamp-2">
                          {skill.description}
                        </p>
                        <div className="flex flex-wrap items-center gap-1.5 pt-1">
                          {skill.tags.map((tag) => (
                            <span
                              key={tag}
                              className="rounded bg-white/5 px-2 py-0.5 font-mono text-[10px] text-fg-muted border border-white/5"
                            >
                              {tag}
                            </span>
                          ))}
                        </div>
                      </div>
                    </div>

                    <div className="shrink-0 pt-1">
                      <button
                        type="button"
                        onClick={() => handleUseSkill(skill)}
                        className="rounded-lg border border-border-default/60 bg-white/5 px-3 py-1.5 text-xs font-medium text-white transition hover:bg-workbench-accent hover:border-workbench-accent"
                      >
                        Use in Chat
                      </button>
                    </div>
                  </div>
                );
              })}

              {filtered.length === 0 && (
                <div className="flex flex-col items-center justify-center py-16 text-center text-fg-muted">
                  <Sparkles className="size-8 stroke-1 text-fg-muted/40 mb-2" />
                  <p className="text-sm font-medium">No skills found</p>
                  <p className="text-xs text-fg-muted/70 mt-0.5">Try a different search query</p>
                </div>
              )}
            </div>

            {/* Modal Footer */}
            <div className="flex items-center justify-between border-t border-border-default/40 px-6 py-3 bg-[#161619] text-xs text-fg-muted">
              <span>{skills.length} sovereign agent skills available</span>
              <span className="text-fg-muted/60">Loaded automatically via SKILL.md specs</span>
            </div>
          </motion.div>
        </div>
      )}
    </AnimatePresence>
  );
};
