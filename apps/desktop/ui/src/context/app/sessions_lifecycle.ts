import { useRef, useCallback } from 'react';
import type { Session, ProjectData } from '@/types';
import type { Task, ExecutionWorkspace } from '@/types/domain';
import { daemonClient } from '@/api/daemon_client';

export interface SessionsLifecycleDeps {
  currentProject: string;
  projectData: ProjectData;
  setProjectData: React.Dispatch<React.SetStateAction<ProjectData>>;
  activeSessionId: string;
  setActiveSessionId: (id: string | ((prev: string) => string)) => void;
  activeSession: Session | null;
  selectedModel: string;
  selectedModelAccountId: string;
  selectedReasoningEffort: string;
  workspaces: ExecutionWorkspace[];
  setTasks: React.Dispatch<React.SetStateAction<Task[]>>;
  setIsNewSessionOpen: (open: boolean) => void;
  showToast: (msg: string) => void;
  handleOpenFolderDirect: () => Promise<void>;
}

export function useSessionsLifecycle({
  currentProject,
  projectData,
  setProjectData,
  activeSessionId,
  setActiveSessionId,
  activeSession,
  selectedModel,
  selectedModelAccountId,
  selectedReasoningEffort,
  workspaces,
  setTasks,
  setIsNewSessionOpen,
  showToast,
  handleOpenFolderDirect,
}: SessionsLifecycleDeps) {
  const pendingTaskConversations = useRef(new Set<string>());

  const handleClearHistory = () => {
    if (!activeSession) return;
    setProjectData((prev) => {
      const list = prev[currentProject] || [];
      const updatedList = list.map((s) => {
        if (s.id === activeSession.id) {
          return { ...s, messages: [] };
        }
        return s;
      });
      return { ...prev, [currentProject]: updatedList };
    });
    showToast('Conversation cleared in this window');
  };

  const handleDeleteConversation = useCallback(async (sessionId: string) => {
    const list = projectData[currentProject] || [];
    const targetIndex = list.findIndex((session) => session.id === sessionId);
    if (targetIndex < 0) {
      showToast('Conversation not found');
      return;
    }

    const target = list[targetIndex];
    const nextList = list.filter((session) => session.id !== sessionId);
    const fallbackSession = nextList[targetIndex] || nextList[targetIndex - 1] || nextList[0];

    const daemonSessionId = target.sessionId || (target.source === 'daemon' ? target.id : undefined);
    if (daemonSessionId) {
      try {
        await daemonClient.deleteSession(daemonSessionId);
      } catch (error) {
        showToast(`Could not delete session: ${error instanceof Error ? error.message : String(error)}`);
        return;
      }
    }

    setProjectData((prev) => ({
      ...prev,
      [currentProject]: nextList,
    }));

    if (activeSessionId === sessionId) {
      setActiveSessionId(fallbackSession?.id || '');
    }

    showToast(
      target.source === 'daemon'
        ? 'Session deleted from daemon'
        : 'Conversation deleted'
    );
  }, [activeSessionId, currentProject, projectData, setActiveSessionId, setProjectData, showToast]);

  const handleAcceptAndRun = async () => {
    if (!activeSession) return;
    try {
      await daemonClient.advanceTask(activeSession.taskId || activeSession.id, 'queued');
      showToast('Task queued. The daemon will report the run state when it starts.');
    } catch (error) {
      showToast(`Could not advance task: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const handleRejectDiff = () => {
    showToast('Reject diff is unavailable until a daemon-backed patch proposal is selected.');
  };

  const handleCopyDiff = async () => {
    if (!activeSession) return;
    const diffText = activeSession.diffCode.map((d) => d.text).join('\n');
    if (!diffText) {
      showToast('No diff is available to copy.');
      return;
    }
    try {
      await navigator.clipboard.writeText(diffText);
      showToast('Code diff copied to clipboard');
    } catch (error) {
      showToast(`Could not copy diff: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const handleCreateNewSession = async (title: string, pack: 'engineering' | 'research' | 'assistant' | 'quant') => {
    try {
      const createdTask = await daemonClient.createTask({
        title: title || 'New Supervised Task',
        contract: {
          pack_id: pack,
          name: title || 'New Supervised Task',
          description: `${pack} conversation task`,
          required_capabilities: [],
          evidence_requirements: [],
        },
      });

      const createdSession = await daemonClient.createSession('assisted');
      await daemonClient.attachSession({ session_id: createdSession.id, task_id: createdTask.id });
      await daemonClient.openSessionInLens(createdSession.id, pack === 'engineering' ? 'coding' : pack === 'assistant' ? 'assistant' : pack === 'quant' ? 'quant' : 'research');

      const newSessionItem: Session = {
        id: createdSession.id,
        taskId: createdTask.id,
        source: 'daemon',
        taskStatus: createdTask.status,
        sessionId: createdSession.id,
        pack,
        openedLenses: [pack === 'engineering' ? 'code' : pack === 'assistant' ? 'chat' : pack === 'quant' ? 'quant' : 'research'],
        title: createdTask.title,
        time: 'Just now',
        preview: `${pack} task · ${createdTask.status}`,
        model: selectedModel || '',
        modelAccountId: selectedModelAccountId,
        reasoningEffort: selectedReasoningEffort,
        fileName: '',
        diffHunk: '',
        diffLinesCount: '',
        summary: `Created task ${createdTask.id}`,
        messages: [],
        diffCode: [],
      };

      setProjectData((prev) => {
        const currentList = prev[currentProject] || [];
        return {
          ...prev,
          [currentProject]: [newSessionItem, ...currentList],
        };
      });

      setActiveSessionId(createdSession.id);
      setTasks((previous) => [createdTask, ...previous]);
      setIsNewSessionOpen(false);
      showToast(`Created Task ${createdTask.id.slice(0, 8)}`);
    } catch (err: any) {
      console.error('[CreateSession] Error:', err);
      showToast(`Error creating session: ${err?.message || err}`);
    }
  };

  const handleCreateChatSession = async (pack: 'engineering' | 'research' | 'assistant' | 'quant', taskId?: string) => {
    if (taskId && pendingTaskConversations.current.has(taskId)) throw new Error('A conversation is already being created for this Task.');
    if (taskId) pendingTaskConversations.current.add(taskId);
    try {
      const boundTask = taskId ? await daemonClient.getTask(taskId) : undefined;
      const taskWorkspaceId = typeof boundTask?.metadata?.workspace_id === 'string' ? boundTask.metadata.workspace_id : undefined;
      const registeredWorkspace = workspaces.find(workspace => workspace.id === taskWorkspaceId);
      const createdSession = await daemonClient.createSession('assisted');
      if (boundTask) await daemonClient.attachSession({ session_id: createdSession.id, task_id: boundTask.id });
      await daemonClient.openSessionInLens(createdSession.id, pack === 'engineering' ? 'coding' : pack === 'assistant' ? 'assistant' : pack === 'quant' ? 'quant' : 'research');
      const conversation: Session = {
        taskId: boundTask?.id,
        taskStatus: boundTask?.status,
        workspaceId: registeredWorkspace?.id,
        id: createdSession.id,
        sessionId: createdSession.id,
        source: 'daemon',
        pack,
        openedLenses: [pack === 'engineering' ? 'code' : pack === 'assistant' ? 'chat' : 'research'],
        title: boundTask?.title || `Conversation ${createdSession.id.slice(0, 8)}`,
        time: 'Just now',
        preview: boundTask ? `Task · ${boundTask.status}` : 'Unbound conversation',
        model: selectedModel || '',
        modelAccountId: selectedModelAccountId,
        reasoningEffort: selectedReasoningEffort,
        fileName: '',
        diffHunk: '',
        diffLinesCount: '',
        summary: '',
        messages: [],
        diffCode: [],
      };
      setProjectData((previous) => ({
        ...previous,
        [currentProject]: [conversation, ...(previous[currentProject] || [])],
      }));
      setActiveSessionId(createdSession.id);
      setIsNewSessionOpen(false);
    } catch (error) {
      showToast(`Could not create chat: ${error instanceof Error ? error.message : String(error)}`);
      if (taskId) throw error;
    } finally {
      if (taskId) pendingTaskConversations.current.delete(taskId);
    }
  };

  const handleNewProjectPrompt = () => {
    handleOpenFolderDirect();
  };

  return {
    handleClearHistory,
    handleDeleteConversation,
    handleAcceptAndRun,
    handleRejectDiff,
    handleCopyDiff,
    handleCreateNewSession,
    handleCreateChatSession,
    handleNewProjectPrompt,
  };
}
