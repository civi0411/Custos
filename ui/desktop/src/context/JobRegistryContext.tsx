import React, { createContext, useContext, useState, useCallback, useMemo } from 'react';
import type { OrcaTabId } from '@/components/workbenches/OrcaTabbedContainer';

export interface JobEntry {
  id: string;            // UUID generated at dispatch time
  jobId: string;         // e.g. "engineering__review"
  jobName: string;
  taskId?: string;       // returned by daemon after start
  status: 'queued' | 'running' | 'started' | 'done' | 'failed';
  sourceTabId?: string;  // which tab triggered it (supports tab instance IDs e.g. "files-123...")
  startedAt: number;
  endedAt?: number;
  error?: string;
}

export interface JobRegistryValue {
  jobs: JobEntry[];
  activeCount: number;
  dispatchJob: (entry: Omit<JobEntry, 'id' | 'startedAt' | 'status'>) => string;
  updateJob: (id: string, patch: Partial<JobEntry>) => void;
  clearCompleted: () => void;
  removeJob: (id: string) => void;
  isJobRunning: (jobId: string, sourceTabId?: string) => boolean;
  getRunningCountForTab: (tabId: string) => number;
}

export const JOB_RESULT_TAB: Partial<Record<string, OrcaTabId>> = {
  'engineering__review': 'kanban',
  'research__source_qa': 'notes',
  'research__deep_research': 'synthesis',
  'research__result_analysis': 'artifacts',
  'engineering__diagnose': 'evidence',
};

const JobRegistryContext = createContext<JobRegistryValue | undefined>(undefined);

export const JobRegistryProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [jobs, setJobs] = useState<JobEntry[]>([]);

  const dispatchJob = useCallback((entry: Omit<JobEntry, 'id' | 'startedAt' | 'status'>): string => {
    const id = typeof crypto !== 'undefined' && crypto.randomUUID
      ? crypto.randomUUID()
      : `job_${Date.now()}_${Math.random().toString(36).substring(2, 7)}`;

    const newJob: JobEntry = {
      ...entry,
      id,
      startedAt: Date.now(),
      status: 'running',
    };

    setJobs((prev) => [newJob, ...prev]);
    return id;
  }, []);

  const updateJob = useCallback((id: string, patch: Partial<JobEntry>) => {
    setJobs((prev) =>
      prev.map((job) => {
        if (job.id === id) {
          const updated = { ...job, ...patch };
          if ((patch.status === 'done' || patch.status === 'started' || patch.status === 'failed') && !updated.endedAt) {
            updated.endedAt = Date.now();
          }
          return updated;
        }
        return job;
      })
    );
  }, []);

  const clearCompleted = useCallback(() => {
    setJobs((prev) => prev.filter((j) => j.status === 'running' || j.status === 'queued'));
  }, []);

  const removeJob = useCallback((id: string) => {
    setJobs((prev) => prev.filter((j) => j.id !== id));
  }, []);

  const isJobRunning = useCallback(
    (jobId: string, sourceTabId?: string): boolean => {
      return jobs.some((j) => {
        if (j.status !== 'running' && j.status !== 'queued') return false;
        if (j.jobId !== jobId) return false;
        if (sourceTabId !== undefined) {
          const jBase = j.sourceTabId?.split('-')[0];
          const queryBase = sourceTabId.split('-')[0];
          if (j.sourceTabId !== sourceTabId && jBase !== queryBase) return false;
        }
        return true;
      });
    },
    [jobs]
  );

  const getRunningCountForTab = useCallback(
    (tabId: string): number => {
      const targetBase = tabId.split('-')[0];
      return jobs.filter((j) => {
        if (j.status !== 'running' && j.status !== 'queued') return false;
        if (!j.sourceTabId) return false;
        return j.sourceTabId === tabId || j.sourceTabId.split('-')[0] === targetBase;
      }).length;
    },
    [jobs]
  );

  const activeCount = useMemo(() => {
    return jobs.filter((j) => j.status === 'running' || j.status === 'queued').length;
  }, [jobs]);

  const value = useMemo<JobRegistryValue>(
    () => ({
      jobs,
      activeCount,
      dispatchJob,
      updateJob,
      clearCompleted,
      removeJob,
      isJobRunning,
      getRunningCountForTab,
    }),
    [
      jobs,
      activeCount,
      dispatchJob,
      updateJob,
      clearCompleted,
      removeJob,
      isJobRunning,
      getRunningCountForTab,
    ]
  );

  return <JobRegistryContext.Provider value={value}>{children}</JobRegistryContext.Provider>;
};

const NOOP_REGISTRY: JobRegistryValue = {
  jobs: [],
  activeCount: 0,
  dispatchJob: () => 'noop_job_id',
  updateJob: () => {},
  clearCompleted: () => {},
  removeJob: () => {},
  isJobRunning: () => false,
  getRunningCountForTab: () => 0,
};

export function useJobRegistry(): JobRegistryValue {
  const context = useContext(JobRegistryContext);
  if (!context) {
    // Graceful fallback to avoid hard crashes if used outside provider
    return NOOP_REGISTRY;
  }
  return context;
}
