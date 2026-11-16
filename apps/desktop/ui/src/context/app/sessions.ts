import React, { useRef } from 'react';
import type { Session, ProjectData, ModelCombo, ConnectedAccount } from '@/types';
import type { Task, ExecutionWorkspace } from '@/types/domain';
import type { AppearancePreferences } from '@/lib/appearance';
import { daemonClient } from '@/api/daemon_client';
import { isTransientAiLocalAccount } from './providers';
import { useLiveRunStream } from './liveRunStream';
import { useSessionsLifecycle } from './sessions_lifecycle';

export interface SessionsSliceDeps {
  currentProject: string;
  setCurrentProject: (name: string) => void;
  projectData: ProjectData;
  setProjectData: React.Dispatch<React.SetStateAction<ProjectData>>;
  activeSessionId: string;
  setActiveSessionId: (id: string | ((prev: string) => string)) => void;
  activeSession: Session | null;
  selectedModel: string;
  selectedModelAccountId: string;
  selectedReasoningEffort: string;
  selectedModelAvailable: boolean;
  combos: ModelCombo[];
  connectedAccounts: ConnectedAccount[];
  setConnectedAccounts: React.Dispatch<React.SetStateAction<ConnectedAccount[]>>;
  workspaces: ExecutionWorkspace[];
  setTasks: React.Dispatch<React.SetStateAction<Task[]>>;
  setIsNewSessionOpen: (open: boolean) => void;
  showToast: (msg: string) => void;
  refreshCatalogAvailability: () => Promise<void>;
  appearance: AppearancePreferences;
  handleOpenFolderDirect: () => Promise<void>;
}

export function useSessionsSlice({
  currentProject,
  setCurrentProject,
  projectData,
  setProjectData,
  activeSessionId,
  setActiveSessionId,
  activeSession,
  selectedModel,
  selectedModelAccountId,
  selectedReasoningEffort,
  selectedModelAvailable,
  combos,
  connectedAccounts,
  setConnectedAccounts,
  workspaces,
  setTasks,
  setIsNewSessionOpen,
  showToast,
  refreshCatalogAvailability,
  appearance,
  handleOpenFolderDirect,
}: SessionsSliceDeps) {
  const pendingTurns = useRef(new Set<string>());
  const { followLiveRun, handleStopRun } = useLiveRunStream({
    currentProject,
    activeSessionId,
    appearance,
    setProjectData,
    refreshCatalogAvailability,
    showToast,
  });

  const startNativeConversation = async (instruction: string, harness: string, workspace: string) => {
    const session = await daemonClient.createSession('assisted');
    const task = await daemonClient.createTask({
      title: instruction.slice(0, 120),
      metadata: {
        kind: 'native_coding_agent',
        pack_id: 'engineering',
        harness_id: harness,
        workspace_id: workspace,
        conversation_session_id: session.id,
      },
    });
    await daemonClient.attachSession({ session_id: session.id, task_id: task.id });
    await daemonClient.appendSessionMessage(session.id, 'user', instruction);
    const project = currentProject || 'AgentHub';
    const chat: Session = {
      nativeHarnessId: harness,
      workspaceId: workspace,
      id: session.id,
      sessionId: session.id,
      taskId: task.id,
      source: 'daemon',
      title: instruction.slice(0, 80),
      time: 'Live',
      preview: harness,
      model: harness,
      pack: 'engineering',
      fileName: '',
      diffHunk: '',
      diffLinesCount: '',
      summary: '',
      diffCode: [],
      messages: [{ role: 'user', author: 'You', text: instruction }],
    };
    setProjectData(previous => ({ ...previous, [project]: [...(previous[project] || []), chat] }));
    const run = await daemonClient.startRun({
      background: true,
      task_id: task.id,
      session_id: session.id,
      turn_id: `turn_${crypto.randomUUID()}`,
      preferred_mode: 'native',
      harness_id: harness,
      workspace_id: workspace,
      prompt: instruction,
    });
    setProjectData(previous => ({
      ...previous,
      [project]: (previous[project] || []).map(c => c.id !== session.id ? c : {
        ...c,
        messages: [
          ...c.messages,
          {
            id: `live-${run.run_id}`,
            role: 'assistant' as const,
            author: harness,
            runId: run.run_id,
            runStatus: 'queued',
            badge: 'queued',
            text: '',
          },
        ],
      }),
    }));
    setCurrentProject(project);
    setActiveSessionId(session.id);
    followLiveRun(project, session.id, session.id, run.run_id, harness);
    return { run, sessionId: session.id };
  };

  const handleSendMessage = async (text: string, lens?: 'assistant' | 'coding' | 'research' | 'quant') => {
    if (!activeSession) return false;
    if (activeSession.messages.some(message => message.runId && !['completed', 'failed', 'cancelled', 'interrupted'].includes(message.runStatus || ''))) {
      showToast('Stop or wait for the current run before sending another prompt.');
      return false;
    }
    if (pendingTurns.current.has(activeSession.id)) return false;
    if (!activeSession.nativeHarnessId && !selectedModelAvailable) {
      showToast('Select an available model/account and reasoning effort before sending.');
      return false;
    }
    pendingTurns.current.add(activeSession.id);
    let userJournalPersisted = false;
    let dispatchAttempted = false;
    const userMsg = {
      id: `pending-${crypto.randomUUID()}`,
      role: 'user' as const,
      author: 'You',
      text,
    };

    setProjectData((prev) => {
      const list = prev[currentProject] || [];
      const updatedList = list.map((s) => {
        if (s.id === activeSession.id) {
          return {
            ...s,
            messages: [...s.messages, userMsg],
          };
        }
        return s;
      });
      return {
        ...prev,
        [currentProject]: updatedList,
      };
    });

    const reportTurnFailure = (failure: {
      role: 'assistant';
      author: string;
      badge?: string;
      stepName?: string;
      text: string;
    }) => {
      setProjectData((prev) => {
        const list = prev[currentProject] || [];
        const updatedList = list.map((s) => {
          if (s.id === activeSession.id) {
            return {
              ...s,
              messages: s.messages.map((message) => message.id === userMsg.id
                ? { ...message, badge: dispatchAttempted ? 'Run status unknown' : userJournalPersisted ? 'Saved, not run' : 'Not sent', liveError: failure.text }
                : message),
            };
          }
          return s;
        });
        return {
          ...prev,
          [currentProject]: updatedList,
        };
      });
    };

    try {
      if (activeSession.nativeHarnessId && activeSession.workspaceId && activeSession.sessionId) {
        const harness = activeSession.nativeHarnessId;
        const taskId = activeSession.taskId;
        if (!taskId) {
          throw new Error('This coding conversation is not bound to a Task. Reopen it from Tasks before continuing.');
        }
        await daemonClient.appendSessionMessage(activeSession.sessionId, 'user', text);
        userJournalPersisted = true;
        dispatchAttempted = true;
        const run = await daemonClient.startRun({
          background: true,
          task_id: taskId,
          session_id: activeSession.sessionId,
          turn_id: `turn_${crypto.randomUUID()}`,
          preferred_mode: 'native',
          harness_id: harness,
          workspace_id: activeSession.workspaceId,
          prompt: text,
        });
        setProjectData(previous => ({
          ...previous,
          [currentProject]: (previous[currentProject] || []).map(chat => chat.id !== activeSession.id ? chat : {
            ...chat,
            messages: [
              ...chat.messages,
              {
                id: `live-${run.run_id}`,
                role: 'assistant' as const,
                author: harness,
                runId: run.run_id,
                runStatus: 'queued',
                badge: 'queued',
                text: '',
              },
            ],
          }),
        }));
        followLiveRun(currentProject, activeSession.id, activeSession.sessionId, run.run_id, harness);
        return true;
      }

      const chosenModel = (selectedModel || activeSession.model || '').trim();
      const selectedCombo = combos.find((combo) => combo.id.toLowerCase() === chosenModel.toLowerCase());

      if (!chosenModel) {
        reportTurnFailure({
          role: 'assistant',
          author: 'Custos Engine',
          badge: 'AI Local Unavailable',
          stepName: 'AI Local',
          text: 'No local model is available yet. Start Ollama and install at least one model, then use Retry models. You can chat without signing in as soon as AI Local discovers it.',
        });
        return false;
      }

      if (selectedCombo) {
        reportTurnFailure({
          role: 'assistant',
          author: 'Custos Engine',
          badge: 'Concrete Model Required',
          stepName: 'Model Gateway',
          text: 'Routing combos are not dispatched as a single model turn yet. Select one concrete live model from a connected account.',
        });
        return false;
      }

      const isNativeModel = chosenModel.startsWith('native:');
      let routedAccountId: string | undefined;
      let modelAccount: ConnectedAccount | undefined;
      let effort: string | undefined;

      if (isNativeModel) {
        const modelPath = chosenModel.slice('native:'.length);
        try {
          if (!modelPath) throw new Error('The selected local model has no file path');
          const loaded = await daemonClient.listLoadedInstances();
          if (!loaded.instances.some(instance => instance.model_path === modelPath)) {
            await daemonClient.loadLocalModel(modelPath);
          }
        } catch (error: any) {
          reportTurnFailure({ role: 'assistant', author: 'Custos Engine', badge: 'Local Model Unavailable', text: error?.message || String(error) });
          return false;
        }
      } else {
        const candidates = connectedAccounts.filter(account => account.status === 'active' && account.supportedModels.includes(chosenModel) &&
          (!selectedReasoningEffort || account.catalogModels?.some(model => model.id === chosenModel && model.efforts.includes(selectedReasoningEffort))));
        if (!candidates.length) {
          showToast('Selected model is unavailable. Refresh models or choose another model.');
          return false;
        }
        const transientAccount = candidates.find(isTransientAiLocalAccount);
        const reservation = transientAccount ? undefined : await daemonClient.reserveProviderAccount(candidates[0].providerId, chosenModel, selectedReasoningEffort || undefined);
        modelAccount = transientAccount || candidates.find(account => account.id === reservation?.account_id);
        if (!modelAccount) {
          showToast('No available account supports this model and reasoning effort. Refresh models.');
          return false;
        }
        routedAccountId = isTransientAiLocalAccount(modelAccount) ? undefined : modelAccount.id;
        const metadata = modelAccount.catalogModels?.find(model => model.id === chosenModel);
        effort = selectedReasoningEffort || metadata?.default_effort;
        if (effort && !metadata?.efforts.includes(effort)) {
          showToast('Reasoning effort is no longer available. Select the model again.');
          return false;
        }
      }

      if (chosenModel.startsWith('ollama/')) {
        const account = connectedAccounts.find(a => a.id === routedAccountId && a.status === 'active');
        if (!account) {
          reportTurnFailure({ role: 'assistant', author: 'Custos Engine', badge: 'Ollama Unavailable', stepName: 'Model Gateway', text: 'Ollama is offline or the selected model is no longer installed. Start Ollama and use Retry models. Your prompt will not be sent to another provider.' });
          return false;
        }
        try {
          const probe = await daemonClient.probeModels(account.endpointUrl || 'http://localhost:11434', 'ollama');
          if (!probe.models.some(model => model.id === chosenModel)) {
            reportTurnFailure({ role: 'assistant', author: 'Custos Engine', badge: 'Model Removed', text: 'The selected Ollama model is no longer installed. Refresh models and choose an installed model.' });
            return false;
          }
        } catch {
          setConnectedAccounts(previous => previous.map(a => a.id === account.id ? { ...a, status: 'offline', statusLabel: 'Ollama offline', badgeColor: '#6b7280', catalogSource: 'cached', catalogError: 'Ollama is unavailable. Start Ollama and retry models.' } : a));
          reportTurnFailure({ role: 'assistant', author: 'Custos Engine', badge: 'Ollama Offline', text: 'Cannot connect to Ollama. Start Ollama and retry models, then resend your prompt. No other provider was used.' });
          return false;
        }
      }

      const selectedAccount = modelAccount;
      const hasActiveAccount = Boolean(selectedAccount);
      if (!isNativeModel && !hasActiveAccount) {
        reportTurnFailure({
          role: 'assistant',
          author: 'Custos Engine',
          badge: 'Model Not Connected',
          stepName: 'Model Gateway',
          text: `Model "${chosenModel}" cannot be dispatched because no active provider accounts are connected. Please connect a provider in Settings > Providers first.`,
        });
        return false;
      }

      const llmStatus = await daemonClient.checkLlmStatus();
      if (!llmStatus.configured) {
        const noLlmMsg = {
          role: 'assistant' as const,
          author: 'Custos Engine',
          badge: 'No LLM Available',
          stepName: 'LLM Gateway',
          text: `⚠️ No LLM providers configured.\n\n${llmStatus.message || 'The system is running in Sovereign & Offline Kernel mode. Please configure an API Key (Anthropic Claude, OpenAI Codex, Google Gemini...) or select a Local Model in Settings > Providers to enable AI reasoning.'}`,
        };
        reportTurnFailure(noLlmMsg);
        return false;
      }

      let targetSessionId = activeSession.sessionId;
      let runTaskId = '';
      let createTaskError: unknown = null;

      if (!targetSessionId) {
        try {
          const createdSession = await daemonClient.createSession('assisted');
          targetSessionId = createdSession.id;
        } catch (e) {
          console.warn('[DaemonClient] Could not create session on daemon:', e);
        }
      }

      try {
        const rawTitle = (activeSession.title || chosenModel).replace(/^(Chat turn:\s*)+/, '').trim();
        const turnTitle = rawTitle ? `Chat turn: ${rawTitle}` : `Chat turn: ${chosenModel}`;
        const createdTask = await daemonClient.createTask({
          title: turnTitle,
          metadata: {
            kind: 'conversation_chat_turn',
            pack_id: lens || activeSession.pack || 'assistant',
            conversation_session_id: targetSessionId || activeSession.id,
            source_task_id: activeSession.taskId || activeSession.id,
            model: chosenModel,
            ...(!isNativeModel && !isTransientAiLocalAccount(selectedAccount)
              ? { provider_account_id: selectedAccount?.id }
              : {}),
          },
        });
        runTaskId = createdTask.id;
      } catch (e) {
        createTaskError = e;
        console.warn('[DaemonClient] Could not create chat turn task on daemon:', e);
      }

      if (!runTaskId) {
        reportTurnFailure({
          role: 'assistant',
          author: 'Custos Kernel',
          badge: 'Task Required',
          stepName: 'Conversation Dispatch',
          text: `Could not create an isolated chat task for this turn: ${
            createTaskError instanceof Error ? createTaskError.message : String(createTaskError || 'unknown error')
          }`,
        });
        return false;
      }

      if (targetSessionId) {
        await daemonClient.appendSessionMessage(targetSessionId, 'user', text);
        userJournalPersisted = true;
      }

      dispatchAttempted = true;
      const run = await daemonClient.startRun({
        background: true,
        turn_id: `turn_${crypto.randomUUID()}`,
        task_id: runTaskId,
        preferred_mode: 'model',
        session_id: targetSessionId,
        prompt: text,
        model: chosenModel,
        provider_account_id: routedAccountId,
        reasoning_effort: effort,
      });

      const runId = run.run_id;
      setProjectData(previous => ({
        ...previous,
        [currentProject]: (previous[currentProject] || []).map(chat => chat.id !== activeSession.id ? chat : {
          ...chat,
          messages: [
            ...chat.messages.map(message => message.id === userMsg.id ? { ...message, badge: undefined, liveError: undefined } : message),
            { id: `live-${runId}`, role: 'assistant' as const, author: chosenModel, badge: 'queued', stepName: `Run #${runId.slice(0, 8)}`, text: '', runId, runStatus: 'queued' },
          ],
        }),
      }));
      if (targetSessionId) followLiveRun(currentProject, activeSession.id, targetSessionId, runId, chosenModel);
      return true;
    } catch (err: any) {
      void refreshCatalogAvailability();
      if ((selectedModel || activeSession.model || '').startsWith('ollama/') && String(err?.message || err).includes('Ollama connection failed')) {
        setConnectedAccounts(previous => previous.map(a => a.providerId === 'local' ? { ...a, status: 'offline', statusLabel: 'Ollama offline', badgeColor: '#6b7280', catalogSource: 'cached', catalogError: 'Ollama stopped during the request. Start Ollama and resend your prompt.' } : a));
      }
      reportTurnFailure({ role: 'assistant', author: 'Custos', text: `${dispatchAttempted ? 'Could not confirm run start; check Activity before retrying' : 'Could not prepare the run'}: ${err?.message || err}` });
      return false;
    } finally {
      pendingTurns.current.delete(activeSession.id);
    }
  };

  const {
    handleClearHistory,
    handleDeleteConversation,
    handleAcceptAndRun,
    handleRejectDiff,
    handleCopyDiff,
    handleCreateNewSession,
    handleCreateChatSession,
    handleNewProjectPrompt,
  } = useSessionsLifecycle({
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
  });

  return {
    followLiveRun,
    handleStopRun,
    startNativeConversation,
    handleSendMessage,
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
