import { useCallback } from 'react';
import type { ResourceTabsState, OrcaTabId, OrcaTab } from '@/components/workbenches/OrcaTabbedContainer';
import { getTabType, findScopedResourceTab } from '@/components/workbenches/OrcaTabbedContainer';
import type { HarnessDescriptor } from '@/types/domain';
import type { Session } from '@/types';

interface UseWorkspaceTabRouterOptions {
  isWebTabOpen: boolean;
  onToggleWebTab: () => void;
  resourceTabs: ResourceTabsState;
  activeWorkspaceId?: string;
  activeSession: Session;
  selectedTaskId: string | null;
  onShowToast?: (msg: string) => void;
  setIsResourcePickerOpen: (open: boolean) => void;
}

export function useWorkspaceTabRouter({
  isWebTabOpen,
  onToggleWebTab,
  resourceTabs,
  activeWorkspaceId,
  activeSession,
  selectedTaskId,
  onShowToast,
  setIsResourcePickerOpen,
}: UseWorkspaceTabRouterOptions) {
  const openTab = useCallback((
    id: OrcaTabId,
    title: string,
    url: string,
    agentId?: string,
    workspaceId?: string,
    targetPane: 'primary' | 'split' = 'primary',
    targetSessionId?: string,
  ) => {
    if (!isWebTabOpen) {
      onToggleWebTab();
    }
    const activeTab = resourceTabs.tabs.find((t) => t.id === resourceTabs.activeTabId);
    const newTabId = `${id}-${crypto.randomUUID()}`;
    const isSessionIndependent = ['fleet', 'contacts', 'calendar', 'notes'].includes(id);
    const newTab: OrcaTab = {
      id: newTabId,
      type: id,
      title,
      url,
      agentId,
      workspaceId: workspaceId || activeWorkspaceId || undefined,
      sessionId: isSessionIndependent ? undefined : (targetSessionId || activeSession.id || undefined),
      taskId: isSessionIndependent ? undefined : (selectedTaskId || activeSession.taskId),
    };

    if (activeTab && getTabType(activeTab) === 'tools') {
      resourceTabs.setTabs((prev) =>
        prev.map((t) => (t.id === activeTab.id ? newTab : t))
      );
    } else {
      const isMultiInstance = ['terminal', 'agents', 'browser', 'emulator', 'conversation', 'chat', 'tasks', 'task'].includes(id);
      const isSpecificFile = id === 'files' && Boolean(url && (url.startsWith('custos://file?') || url.startsWith('custos://diff?')));

      if (!isMultiInstance && !isSpecificFile) {
        const existing = findScopedResourceTab(resourceTabs.tabs, id, {
          agentId,
          workspaceId: workspaceId || activeWorkspaceId || undefined,
          sessionId: isSessionIndependent ? undefined : (targetSessionId || activeSession.id || undefined),
          taskId: isSessionIndependent ? undefined : (selectedTaskId || activeSession.taskId),
        });
        if (existing) {
          if (targetPane === 'split') {
            resourceTabs.setSplitTabId?.(existing.id);
          } else {
            resourceTabs.setActiveTabId(existing.id);
          }
          return;
        }
      } else if (isSpecificFile) {
        const resolvedWorkspaceId = workspaceId || activeWorkspaceId || undefined;
        const existingFile = resourceTabs.tabs.find((t) =>
          t.type === 'files' && t.url === url && t.workspaceId === resolvedWorkspaceId
        );
        if (existingFile) {
          if (targetPane === 'split') {
            resourceTabs.setSplitTabId?.(existingFile.id);
          } else {
            resourceTabs.setActiveTabId(existingFile.id);
          }
          return;
        }
      }

      resourceTabs.setTabs((prev) => [...prev, newTab]);
    }

    if (targetPane === 'split') {
      resourceTabs.setSplitTabId?.(newTabId);
    } else {
      resourceTabs.setActiveTabId(newTabId);
    }
  }, [isWebTabOpen, onToggleWebTab, resourceTabs, activeWorkspaceId, activeSession.id, activeSession.taskId, selectedTaskId]);

  const handleOpenTab = useCallback((
    id: OrcaTabId,
    customUrl?: string,
    customTitle?: string,
    targetPane: 'primary' | 'split' = 'primary',
    targetWorkspaceId?: string,
  ) => {
    const effectiveWsId = targetWorkspaceId || activeWorkspaceId;

    // Full file and change workflows belong in the center work area. The Inspector only quick-opens them.
    if ((id === 'files' || id === 'changes') && !customUrl) {
      const existing = resourceTabs.tabs.find((tab) =>
        (getTabType(tab) === 'files' || getTabType(tab) === 'changes') && tab.workspaceId === effectiveWsId
      );
      if (existing) {
        if (!isWebTabOpen) onToggleWebTab();
        if (targetPane === 'split') resourceTabs.setSplitTabId?.(existing.id);
        else resourceTabs.setActiveTabId(existing.id);
      } else {
        openTab(id, customTitle || (id === 'changes' ? 'Changes' : 'Files'), `custos://resource/${id}`, undefined, effectiveWsId, targetPane);
      }
      return;
    }

    // Specific file editor or diff tab in center canvas
    if (id === 'files' && customUrl && (customUrl.startsWith('custos://file?') || customUrl.startsWith('custos://diff?'))) {
      const existingFile = resourceTabs.tabs.find(t => t.type === 'files' && t.url === customUrl && t.workspaceId === effectiveWsId);
      if (existingFile) {
        if (!isWebTabOpen) onToggleWebTab();
        if (targetPane === 'split') resourceTabs.setSplitTabId?.(existingFile.id);
        else resourceTabs.setActiveTabId(existingFile.id);
        return;
      }
      const parsedTitle = customTitle || decodeURIComponent(customUrl.replace(/^custos:\/\/(file|diff)\?path=/, '')).split('/').pop() || 'File';
      openTab('files', parsedTitle, customUrl, undefined, effectiveWsId, targetPane);
      return;
    }

    // Parallel multi-instance runtimes (Terminal, Agents, Browser, Phone Simulator, Chat, Tasks)
    if (['terminal', 'browser', 'emulator', 'agents', 'conversation', 'chat', 'tasks', 'task'].includes(id)) {
      if (id === 'conversation' || id === 'chat') {
        const existingCount = resourceTabs.tabs.filter(t => t.type === 'conversation' || t.type === 'chat' || t.id.startsWith('conversation')).length;
        const count = existingCount + 1;
        const title = customTitle || (count > 1 ? `Chat #${count}` : (activeSession?.title || 'Chat'));
        const url = customUrl || `custos://chat/${count}`;
        openTab('conversation', title, url, undefined, effectiveWsId, targetPane);
        return;
      }
      if (id === 'tasks' || id === 'task') {
        const existingCount = resourceTabs.tabs.filter(t => t.type === 'tasks' || t.type === 'task' || t.id.startsWith('tasks')).length;
        const count = existingCount + 1;
        const title = customTitle || (count > 1 ? `Tasks #${count}` : 'Tasks');
        const url = customUrl || `custos://tasks/${count}`;
        openTab('tasks', title, url, undefined, effectiveWsId, targetPane);
        return;
      }
      if (id === 'terminal') {
        const existingCount = resourceTabs.tabs.filter(t => t.type === 'terminal' || t.id.startsWith('terminal')).length;
        const count = existingCount + 1;
        const title = customTitle || (count > 1 ? `Terminal ${count}` : 'Terminal');
        const url = customUrl || `custos://terminal/${count}`;
        openTab('terminal', title, url, undefined, effectiveWsId, targetPane);
        return;
      }
      if (id === 'browser') {
        const existingCount = resourceTabs.tabs.filter(t => t.type === 'browser' || t.id.startsWith('browser')).length;
        const count = existingCount + 1;
        const title = customTitle || (count > 1 ? `Browser ${count}` : 'Web Preview');
        const url = customUrl || `custos://browser/${count}`;
        openTab('browser', title, url, undefined, effectiveWsId, targetPane);
        return;
      }
      if (id === 'emulator') {
        const existingCount = resourceTabs.tabs.filter(t => t.type === 'emulator' || t.id.startsWith('emulator')).length;
        const count = existingCount + 1;
        const title = customTitle || (count > 1 ? `Phone Simulator ${count}` : 'Phone Simulator');
        const url = customUrl || `custos://emulator/${count}`;
        openTab('emulator', title, url, undefined, effectiveWsId, targetPane);
        return;
      }
      if (id === 'agents') {
        const existingCount = resourceTabs.tabs.filter(t => t.type === 'agents' || t.id.startsWith('agents')).length;
        const count = existingCount + 1;
        const title = customTitle || (count > 1 ? `Coding Agents #${count}` : 'Coding Agents');
        const url = customUrl || `custos://agent/session-${count}`;
        openTab('agents', title, url, undefined, effectiveWsId, targetPane);
        return;
      }
    }

    // Singleton scoped lenses (worktrees, activity, lab, literature, findings, drafts, outbox, etc.)
    const isSessionIndependent = ['fleet', 'contacts', 'calendar', 'notes'].includes(id);
    const existing = findScopedResourceTab(resourceTabs.tabs, id, {
      workspaceId: effectiveWsId || undefined,
      sessionId: isSessionIndependent ? undefined : (activeSession.id || undefined),
      taskId: isSessionIndependent ? undefined : (selectedTaskId || activeSession.taskId),
    });
    if (existing) {
      if (id === 'synthesis' && customUrl) {
        resourceTabs.setTabs(previous => previous.map(tab => tab.id === existing.id ? { ...tab, url: customUrl } : tab));
      }
      if (!isWebTabOpen) {
        onToggleWebTab();
      }
      if (targetPane === 'split') {
        resourceTabs.setSplitTabId?.(existing.id);
      } else {
        resourceTabs.setActiveTabId(existing.id);
      }
    } else {
      openTab(id, customTitle || (id.charAt(0).toUpperCase() + id.slice(1)), customUrl || `custos://resource/${id}`, undefined, effectiveWsId, targetPane);
    }
  }, [resourceTabs, isWebTabOpen, onToggleWebTab, openTab, activeWorkspaceId, activeSession?.id, activeSession?.taskId, activeSession?.title, selectedTaskId]);

  const handleLaunchAgentTab = useCallback((harness: HarnessDescriptor, targetPane: 'primary' | 'split' = 'primary') => {
    const existingAgentTabs = resourceTabs.tabs.filter((t) => t.type === 'agents' || t.agentId);
    const count = existingAgentTabs.length + 1;
    const title = count > 1 ? `${harness.name} #${count}` : harness.name;
    openTab('agents', title, `custos://agent/${harness.id}`, harness.id, activeWorkspaceId, targetPane);
    setIsResourcePickerOpen(false);
    onShowToast?.(`Opened ${harness.name}. Choose a Task and start the agent when ready.`);
  }, [openTab, resourceTabs.tabs, activeWorkspaceId, onShowToast, setIsResourcePickerOpen]);

  return {
    openTab,
    handleOpenTab,
    handleLaunchAgentTab,
  };
}
