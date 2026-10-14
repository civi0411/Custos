export type ThemePreference = 'light' | 'dark' | 'system';
export interface AppearancePreferences {
  theme: ThemePreference;
  density: 'comfortable' | 'compact';
  reduceMotion: boolean;
}
export const APPEARANCE_KEY = 'custos.appearance.preferences';

export function readAppearance(): AppearancePreferences {
  let saved: Partial<AppearancePreferences> = {};
  try {
    const raw = localStorage.getItem(APPEARANCE_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : null;
    if (parsed && typeof parsed === 'object') saved = parsed;
  } catch { /* Unavailable or malformed preferences use device defaults. */ }
  return {
    theme: saved.theme === 'light' || saved.theme === 'dark' ? saved.theme : 'system',
    density: saved.density === 'compact' ? 'compact' : 'comfortable',
    reduceMotion: saved.reduceMotion === true,
  };
}

export function applyAppearance(preferences: AppearancePreferences, theme: 'light' | 'dark') {
  const root = document.documentElement;
  root.dataset.theme = theme;
  root.classList.toggle('dark', theme === 'dark');
  root.style.colorScheme = theme;
  root.dataset.density = preferences.density;
  root.dataset.reduceMotion = String(preferences.reduceMotion);
}
