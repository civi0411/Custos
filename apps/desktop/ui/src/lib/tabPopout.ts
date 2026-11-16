/**
 * tabPopout.ts
 * Manages tearing tabs out of the main Custos application into standalone native windows
 * (Chrome-style tear-off tabs) using Tauri WebviewWindow, with browser window.open fallback.
 */

export async function popoutTab(tabId: string, title?: string): Promise<void> {
  const windowLabel = `tab-popout-${tabId.replace(/[^a-zA-Z0-9_-]/g, '_')}`;
  const popoutUrl = `index.html#/popout/${tabId}`;

  // 1. Attempt native Tauri WebviewWindow creation
  try {
    const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');
    const existing = await WebviewWindow.getByLabel(windowLabel);
    if (existing) {
      await existing.setFocus();
      return;
    }

    const appWindow = new WebviewWindow(windowLabel, {
      url: popoutUrl,
      title: `${title || 'Custos Tab'} — Detached Workbench`,
      width: 1024,
      height: 720,
      minWidth: 480,
      minHeight: 360,
      decorations: true,
      titleBarStyle: 'overlay',
      hiddenTitle: false,
    });

    await appWindow.once('tauri://error', (e) => {
      console.warn('Tauri window error, falling back to window.open:', e);
      window.open(
        popoutUrl,
        windowLabel,
        'width=1024,height=720,menubar=no,toolbar=no,location=no,status=no'
      );
    });
  } catch {
    // 2. Browser/Vite dev server fallback
    window.open(
      popoutUrl,
      windowLabel,
      'width=1024,height=720,menubar=no,toolbar=no,location=no,status=no'
    );
  }
}
