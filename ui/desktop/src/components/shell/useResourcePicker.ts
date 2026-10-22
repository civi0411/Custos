import { useState, useEffect, useCallback } from 'react';
import { daemonClient } from '@/api/daemon_client';
import type { CapabilityDescriptor } from '@/types/domain';
import { resolveResources, workbenchResourceIds } from '@/components/workbenches/OrcaTabbedContainer';
import type { ActiveWorkbench } from '@/components/shell/AppHeader';

interface UseResourcePickerOptions {
  isResourcePickerOpen: boolean;
  setIsResourcePickerOpen: (open: boolean) => void;
  isWebTabOpen: boolean;
  onToggleWebTab: () => void;
  mode: ActiveWorkbench;
}

export function useResourcePicker({
  isResourcePickerOpen,
  setIsResourcePickerOpen,
  isWebTabOpen,
  onToggleWebTab,
  mode,
}: UseResourcePickerOptions) {
  const [pickerCapabilities, setPickerCapabilities] = useState<CapabilityDescriptor[]>([]);
  const [pickerCapabilityState, setPickerCapabilityState] = useState<'loading' | 'ready' | 'failed'>('loading');
  const [pickerTargetPane, setPickerTargetPane] = useState<'primary' | 'split'>('primary');
  const [pickerAnchor, setPickerAnchor] = useState<{ top: number; left: number } | null>(null);

  useEffect(() => {
    if (!isResourcePickerOpen) return;
    let mounted = true;
    setPickerCapabilityState('loading');
    daemonClient.listCapabilities()
      .then((capabilities) => {
        if (!mounted) return;
        setPickerCapabilities(capabilities);
        setPickerCapabilityState('ready');
      })
      .catch(() => { if (mounted) setPickerCapabilityState('failed'); });
    return () => { mounted = false; };
  }, [isResourcePickerOpen]);

  useEffect(() => {
    if (!isResourcePickerOpen) return;
    const dismiss = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setIsResourcePickerOpen(false);
    };
    window.addEventListener('keydown', dismiss);
    return () => window.removeEventListener('keydown', dismiss);
  }, [isResourcePickerOpen, setIsResourcePickerOpen]);

  const handleOpenResourcePicker = useCallback((
    targetPane: 'primary' | 'split' = 'primary',
    anchor?: { top: number; left: number }
  ) => {
    setPickerTargetPane(targetPane);
    setPickerAnchor(anchor || null);
    if (!isWebTabOpen) onToggleWebTab();
    setIsResourcePickerOpen(true);
  }, [setIsResourcePickerOpen, isWebTabOpen, onToggleWebTab]);

  const pickerResources = resolveResources(
    pickerCapabilities,
    pickerCapabilityState !== 'loading',
    pickerCapabilityState === 'failed',
    typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
  ).filter(resource => workbenchResourceIds(mode).includes(resource.id));

  return {
    pickerTargetPane,
    pickerAnchor,
    setPickerAnchor,
    handleOpenResourcePicker,
    pickerResources,
  };
}
