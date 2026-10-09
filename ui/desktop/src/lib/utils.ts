import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}

export function isMacOS(): boolean {
  if (typeof navigator === 'undefined') return false;
  return /Mac|iPhone|iPad|iPod/.test(navigator.userAgent || navigator.platform || '');
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
