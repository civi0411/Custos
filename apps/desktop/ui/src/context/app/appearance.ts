import { useState, useEffect, useRef, useCallback } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import {
  APPEARANCE_KEY,
  readAppearance,
  applyAppearance,
  type AppearancePreferences,
} from '@/lib/appearance';

export interface AppearanceSlice {
  appearance: AppearancePreferences;
  resolvedTheme: 'light' | 'dark';
  updateAppearance: (next: Partial<AppearancePreferences>) => void;
  uiScale: number;
  handleSetUiScale: (val: number) => void;
  handleStepUiScale: (delta: number) => void;
}

export function useAppearanceSlice(showToast?: (msg: string) => void): AppearanceSlice {
  const [appearance, setAppearance] = useState<AppearancePreferences>(readAppearance);
  const [systemDark, setSystemDark] = useState(() => window.matchMedia('(prefers-color-scheme: dark)').matches);
  const resolvedTheme = appearance.theme === 'system' ? (systemDark ? 'dark' : 'light') : appearance.theme;

  useEffect(() => {
    if (appearance.theme !== 'system') return;
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const update = () => setSystemDark(media.matches);
    update();
    media.addEventListener('change', update);
    return () => media.removeEventListener('change', update);
  }, [appearance.theme]);

  useEffect(() => {
    const sync = (event: StorageEvent) => {
      if (event.key !== APPEARANCE_KEY && event.key !== null) return;
      const saved = readAppearance();
      if (saved.theme === 'system') setSystemDark(window.matchMedia('(prefers-color-scheme: dark)').matches);
      setAppearance(saved);
    };
    window.addEventListener('storage', sync);
    return () => window.removeEventListener('storage', sync);
  }, []);

  useEffect(() => {
    applyAppearance(appearance, resolvedTheme);
  }, [appearance, resolvedTheme]);

  const themeQueue = useRef<Promise<void>>(Promise.resolve());
  useEffect(() => {
    if (!isTauri()) return;
    const theme = appearance.theme === 'system' ? null : resolvedTheme;
    themeQueue.current = themeQueue.current
      .then(async () => {
        await getCurrentWindow().setTheme(theme);
      })
      .catch((error) => console.warn('Native window theme could not be applied', error));
  }, [appearance.theme, resolvedTheme]);

  const updateAppearance = useCallback((next: Partial<AppearancePreferences>) => {
    if (next.theme === 'system') setSystemDark(window.matchMedia('(prefers-color-scheme: dark)').matches);
    setAppearance((previous) => {
      const updated = { ...previous, ...next };
      try {
        localStorage.setItem(APPEARANCE_KEY, JSON.stringify(updated));
      } catch (error) {
        console.warn('Device appearance could not be saved', error);
      }
      return updated;
    });
  }, []);

  // UI Scale state
  const [uiScale, setUiScale] = useState<number>(() => {
    const saved = localStorage.getItem('custos_ui_scale');
    const value = Number(saved || 100);
    return Math.min(160, Math.max(70, Math.round((Number.isFinite(value) ? value : 100) / 10) * 10));
  });

  const zoomQueue = useRef<Promise<void>>(Promise.resolve());
  const applyUiScale = useCallback((scaleVal: number) => {
    const rounded = Math.round((Number.isFinite(scaleVal) ? scaleVal : 100) / 10) * 10;
    const clamped = Math.min(160, Math.max(70, rounded));
    setUiScale(clamped);
    localStorage.setItem('custos_ui_scale', clamped.toString());
  }, []);

  useEffect(() => {
    const factor = uiScale / 100;
    document.documentElement.style.zoom = '';
    document.body.style.zoom = '';
    document.documentElement.style.setProperty('--app-browser-zoom', '1');
    if (isTauri()) {
      zoomQueue.current = zoomQueue.current
        .then(() => getCurrentWebview().setZoom(factor))
        .catch((error) => {
          showToast?.(`Unable to zoom app: ${String(error)}`);
        });
    } else {
      document.documentElement.style.zoom = String(factor);
      document.documentElement.style.setProperty('--app-browser-zoom', String(factor));
    }
  }, [uiScale, showToast]);

  const handleSetUiScale = (val: number) => {
    applyUiScale(val);
  };

  const handleStepUiScale = (delta: number) => {
    handleSetUiScale(uiScale + delta);
  };

  return {
    appearance,
    resolvedTheme,
    updateAppearance,
    uiScale,
    handleSetUiScale,
    handleStepUiScale,
  };
}
