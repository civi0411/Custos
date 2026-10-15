import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}

export function isMacOS(): boolean {
  if (typeof navigator === 'undefined') return false;
  return /Mac|iPhone|iPad|iPod/i.test(navigator.userAgent || navigator.platform || '');
}

export function isWindows(): boolean {
  if (typeof navigator === 'undefined') return false;
  return /Win/i.test(navigator.userAgent || navigator.platform || '');
}

export function isLinux(): boolean {
  if (typeof navigator === 'undefined') return false;
  return /Linux/i.test(navigator.userAgent || navigator.platform || '') && !/Android/i.test(navigator.userAgent || '');
}

export type PlatformOS = 'macos' | 'windows' | 'linux';

export function getPlatformOS(): PlatformOS {
  if (isMacOS()) return 'macos';
  if (isWindows()) return 'windows';
  return 'linux';
}

export interface KeyCombo {
  ctrlOrCmd?: boolean;
  shift?: boolean;
  alt?: boolean;
  key: string;
}

export function formatKeyCombo(combo: KeyCombo): string {
  const mac = isMacOS();
  const parts: string[] = [];

  if (combo.ctrlOrCmd) {
    parts.push(mac ? '⌘' : 'Ctrl');
  }
  if (combo.alt) {
    parts.push(mac ? '⌥' : 'Alt');
  }
  if (combo.shift) {
    parts.push(mac ? '⇧' : 'Shift');
  }
  parts.push(combo.key);

  return mac ? parts.join('') : parts.join('+');
}
