import { useEffect } from 'react';

export interface ShortcutHandlers {
  onToggleSettings?: () => void;
  onToggleSidebar?: () => void;
  onZoomIn?: () => void;
  onZoomOut?: () => void;
  onResetZoom?: () => void;
  onEscape?: () => void;
}

export function useKeyboardShortcuts(handlers: ShortcutHandlers, deps: any[] = []) {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Toggle settings (Ctrl+, or Cmd+,)
      if ((e.metaKey || e.ctrlKey) && e.key === ',') {
        e.preventDefault();
        handlers.onToggleSettings?.();
      }
      // Toggle Sidebar (Ctrl+B or Cmd+B)
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'b') {
        e.preventDefault();
        handlers.onToggleSidebar?.();
      }
      // Zoom In (Ctrl++ or Cmd++)
      if ((e.metaKey || e.ctrlKey) && (e.key === '=' || e.key === '+')) {
        e.preventDefault();
        handlers.onZoomIn?.();
      }
      // Zoom Out (Ctrl+- or Cmd+-)
      if ((e.metaKey || e.ctrlKey) && (e.key === '-' || e.key === '_')) {
        e.preventDefault();
        handlers.onZoomOut?.();
      }
      // Reset Zoom (Ctrl+0 or Cmd+0)
      if ((e.metaKey || e.ctrlKey) && e.key === '0') {
        e.preventDefault();
        handlers.onResetZoom?.();
      }
      // Escape
      if (e.key === 'Escape') {
        handlers.onEscape?.();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handlers, ...deps]);
}
