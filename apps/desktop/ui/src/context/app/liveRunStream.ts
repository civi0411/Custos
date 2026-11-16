import { useRef, useCallback, useEffect } from 'react';
import type { ProjectData } from '@/types';
import type { AppearancePreferences } from '@/lib/appearance';
import type { LiveRunSnapshot } from '@/types/domain';
import { daemonClient } from '@/api/daemon_client';
import { scopedLiveRunStore } from './scopedLiveRun';

export interface LiveRunStreamDeps {
  currentProject: string;
  activeSessionId: string;
  appearance: AppearancePreferences;
  setProjectData: React.Dispatch<React.SetStateAction<ProjectData>>;
  refreshCatalogAvailability: () => Promise<void>;
  showToast: (msg: string) => void;
}

export function useLiveRunStream({
  currentProject,
  activeSessionId,
  appearance,
  setProjectData,
  refreshCatalogAvailability,
  showToast,
}: LiveRunStreamDeps) {
  const liveConnections = useRef(new Map<string, AbortController>());
  const livePresentations = useRef(new Map<string, () => void>());
  const activeLiveView = useRef({ project: currentProject, conversation: activeSessionId });
  activeLiveView.current = { project: currentProject, conversation: activeSessionId };
  const streamReducedMotion = useRef(appearance.reduceMotion);
  streamReducedMotion.current = appearance.reduceMotion;

  useEffect(() => {
    const sync = () => livePresentations.current.forEach(refresh => refresh());
    sync();
    document.addEventListener('visibilitychange', sync);
    return () => document.removeEventListener('visibilitychange', sync);
  }, [currentProject, activeSessionId, appearance.reduceMotion]);

  useEffect(() => () => {
    liveConnections.current.forEach(controller => controller.abort());
    liveConnections.current.clear();
  }, []);

  const handleStopRun = useCallback(async (runId: string) => {
    try {
      await daemonClient.cancelRun({ run_id: runId, reason: 'Stopped from chat' });
    } catch (error) {
      showToast(error instanceof Error ? error.message : String(error));
    }
  }, [showToast]);

  const followLiveRun = useCallback((project: string, conversation: string, session: string, run: string, author: string) => {
    if (liveConnections.current.has(run)) return;
    const controller = new AbortController();
    liveConnections.current.set(run, controller);
    let timer: number | undefined;
    let frame: number | undefined;
    let lastPaint = -Infinity;
    let latest: LiveRunSnapshot | null = null;
    let displayedText = '';
    let initialFlush = true;
    const systemMotion = window.matchMedia('(prefers-reduced-motion: reduce)');
    const isVisible = () => !document.hidden && activeLiveView.current.project === project && activeLiveView.current.conversation === conversation;

    const cancelPending = () => {
      if (timer !== undefined) window.clearTimeout(timer);
      if (frame !== undefined) window.cancelAnimationFrame(frame);
      timer = undefined;
      frame = undefined;
    };

    const schedule = () => {
      if (controller.signal.aborted || frame !== undefined || timer !== undefined) return;
      if (isVisible()) frame = window.requestAnimationFrame(paint);
      else timer = window.setTimeout(() => { timer = undefined; flush(true); }, 200);
    };

    const paint = (timestamp: number) => {
      frame = undefined;
      if (controller.signal.aborted) return;
      if (!isVisible()) { flush(true); return; }
      if (timestamp - lastPaint >= 40) flush();
      else schedule();
    };

    const flush = (immediate = false) => {
      const snapshot = latest;
      if (!snapshot || controller.signal.aborted) return;
      const bypass = immediate || initialFlush || !isVisible() || streamReducedMotion.current || systemMotion.matches || !snapshot.text.startsWith(displayedText);
      const remaining = snapshot.text.length - displayedText.length;
      let end = bypass || remaining > 4096 ? snapshot.text.length : Math.min(snapshot.text.length, displayedText.length + Math.max(12, Math.ceil(remaining * 0.6)));
      const trailing = snapshot.text.charCodeAt(end - 1);
      if (end < snapshot.text.length && trailing >= 0xD800 && trailing <= 0xDBFF) end += 1;
      displayedText = snapshot.text.slice(0, end);
      const wasInitial = initialFlush;
      initialFlush = false;
      lastPaint = performance.now();
      const text = displayedText;

      scopedLiveRunStore.emit(run, {
        ...snapshot,
        text,
      });

      const isSettled = ['completed', 'failed', 'cancelled', 'interrupted'].includes(snapshot.status);
      if (immediate || isSettled || wasInitial) {
        setProjectData(previous => ({
          ...previous,
          [project]: (previous[project] || []).map(chat => chat.id !== conversation ? chat : {
            ...chat,
            messages: chat.messages.some(message => message.runId === run)
              ? chat.messages.map(message => message.runId !== run ? message : {
                  ...message,
                  text,
                  runStatus: snapshot.status,
                  badge: snapshot.reconnecting ? 'Reconnecting' : snapshot.status,
                  activities: snapshot.activities,
                  liveError: snapshot.error || undefined,
                })
              : [
                  ...chat.messages,
                  {
                    id: `live-${run}`,
                    role: 'assistant' as const,
                    author,
                    text,
                    runId: run,
                    runStatus: snapshot.status,
                    badge: snapshot.reconnecting ? 'Reconnecting' : snapshot.status,
                    activities: snapshot.activities,
                    liveError: snapshot.error || undefined,
                  },
                ],
          }),
        }));
      }
      if (displayedText !== snapshot.text && !immediate) schedule();
    };

    const refresh = () => {
      cancelPending();
      if (latest) flush(true);
    };

    livePresentations.current.set(run, refresh);
    systemMotion.addEventListener('change', refresh);
    controller.signal.addEventListener('abort', cancelPending, { once: true });

    void daemonClient.watchRun(session, run, snapshot => {
      latest = snapshot;
      if (['completed', 'failed', 'cancelled', 'interrupted'].includes(snapshot.status)) {
        cancelPending();
        flush(true);
        if (snapshot.status === 'failed') void refreshCatalogAvailability();
      } else {
        schedule();
      }
    }, controller.signal).finally(() => {
      cancelPending();
      if (!controller.signal.aborted) flush(true);
      systemMotion.removeEventListener('change', refresh);
      livePresentations.current.delete(run);
      liveConnections.current.delete(run);
      scopedLiveRunStore.clear(run);
    });
  }, [refreshCatalogAvailability, setProjectData]);

  return {
    followLiveRun,
    handleStopRun,
  };
}
