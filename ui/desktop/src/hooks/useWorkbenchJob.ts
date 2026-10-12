import { useState, useEffect, useCallback, useRef } from 'react';
import { daemonClient } from '@/api/daemon_client';
import { PacksJobCatalogItem, RunPacksJobResult } from '@/types/domain';
import { useJobRegistry, JOB_RESULT_TAB } from '@/context/JobRegistryContext';
import type { OrcaTabId } from '@/components/views/OrcaTabbedContainer';

// Global cache for catalog items to avoid repeated network round-trips
let cachedJobsCatalog: PacksJobCatalogItem[] | null = null;
let catalogFetchPromise: Promise<PacksJobCatalogItem[]> | null = null;

async function getPacksJobsCatalog(): Promise<PacksJobCatalogItem[]> {
  if (cachedJobsCatalog) {
    return cachedJobsCatalog;
  }
  if (catalogFetchPromise) {
    return catalogFetchPromise;
  }
  catalogFetchPromise = daemonClient.listPacksJobs()
    .then((jobs) => {
      cachedJobsCatalog = jobs;
      catalogFetchPromise = null;
      return jobs;
    })
    .catch((err) => {
      catalogFetchPromise = null;
      throw err;
    });
  return catalogFetchPromise;
}

export interface UseWorkbenchJobOptions {
  jobId: string;
  sessionId?: string;
  workspaceId?: string;
  sourceTabId?: string;
  allowParallel?: boolean;
  navigateOnSuccess?: OrcaTabId;
  getInputs: () => Record<string, unknown> | null;
  onSuccess?: (taskId: string, result: RunPacksJobResult) => void;
  onError?: (error: Error) => void;
  onShowToast?: (msg: string) => void;
  onNavigate?: (targetTab: OrcaTabId) => void;
}

export interface UseWorkbenchJobReturn {
  isReady: boolean;
  isRunning: boolean;
  jobMeta: PacksJobCatalogItem | null;
  run: (extraInputs?: Record<string, unknown>) => Promise<RunPacksJobResult | null>;
  loadJobMeta: () => Promise<PacksJobCatalogItem | null>;
}

export function useWorkbenchJob({
  jobId,
  sessionId,
  workspaceId,
  sourceTabId,
  allowParallel = false,
  navigateOnSuccess,
  getInputs,
  onSuccess,
  onError,
  onShowToast,
  onNavigate,
}: UseWorkbenchJobOptions): UseWorkbenchJobReturn {
  const [isRunning, setIsRunning] = useState<boolean>(false);
  const [jobMeta, setJobMeta] = useState<PacksJobCatalogItem | null>(null);
  const registry = useJobRegistry();

  // Keep latest getInputs in a ref to avoid stale closures
  const getInputsRef = useRef(getInputs);
  useEffect(() => {
    getInputsRef.current = getInputs;
  }, [getInputs]);

  const loadJobMeta = useCallback(async () => {
    try {
      const catalog = await getPacksJobsCatalog();
      const match = catalog.find((j) => (j.jobId || j.job_id) === jobId) || null;
      setJobMeta(match);
      return match;
    } catch {
      return null;
    }
  }, [jobId]);

  useEffect(() => {
    loadJobMeta();
  }, [loadJobMeta]);

  // isReady is computed dynamically by checking if getInputs() provides valid object
  const currentInputs = getInputs();
  const isReady = currentInputs !== null;

  // Check if this exact job is already running on the same source tab
  const isAlreadyRunning = !allowParallel && registry.isJobRunning(jobId, sourceTabId);

  const run = useCallback(async (extraInputs?: Record<string, unknown>): Promise<RunPacksJobResult | null> => {
    if (jobMeta && jobMeta.status === 'planned') {
      const msg = `Job "${jobMeta.name}" is planned for a future release.`;
      onShowToast?.(msg);
      return null;
    }

    if (!allowParallel && registry.isJobRunning(jobId, sourceTabId)) {
      onShowToast?.(`Job "${jobMeta?.name || jobId}" is already running.`);
      return null;
    }

    const baseInputs = getInputsRef.current();
    if (baseInputs === null) {
      onShowToast?.(`Cannot start "${jobMeta?.name || jobId}": required context is missing.`);
      return null;
    }

    const mergedInputs = {
      ...baseInputs,
      ...(extraInputs || {}),
    };

    const jobName = jobMeta?.name || jobId;
    const entryId = registry.dispatchJob({
      jobId,
      jobName,
      sourceTabId,
    });

    setIsRunning(true);
    try {
      const result = await daemonClient.runPacksJob({
        jobId,
        sessionId,
        workspaceId,
        inputs: mergedInputs,
      });

      const taskId = result.taskId || result.task_id || 'unknown';
      registry.updateJob(entryId, {
        taskId,
        status: 'started',
        endedAt: Date.now(),
      });

      onShowToast?.(`Started "${jobName}" (Task ${taskId})`);
      onSuccess?.(taskId, result);

      // Auto-navigation after 1.5s delay if configured or mapped
      const targetTab = navigateOnSuccess || JOB_RESULT_TAB[jobId];
      if (targetTab && onNavigate) {
        setTimeout(() => {
          onNavigate(targetTab);
        }, 1500);
      }

      return result;
    } catch (err: any) {
      const errorObj = err instanceof Error ? err : new Error(String(err));
      registry.updateJob(entryId, {
        status: 'failed',
        error: errorObj.message,
        endedAt: Date.now(),
      });
      onShowToast?.(`Failed to run "${jobName}": ${errorObj.message}`);
      onError?.(errorObj);
      return null;
    } finally {
      setIsRunning(false);
    }
  }, [
    jobId,
    sessionId,
    workspaceId,
    sourceTabId,
    allowParallel,
    navigateOnSuccess,
    jobMeta,
    registry,
    onShowToast,
    onSuccess,
    onError,
    onNavigate,
  ]);

  return {
    isReady,
    isRunning: isRunning || isAlreadyRunning,
    jobMeta,
    run,
    loadJobMeta,
  };
}

