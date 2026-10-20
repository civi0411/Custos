import { useCallback, useEffect, useRef, useState } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { daemonClient } from '@/api/daemon_client';

export type ClientPlatform = 'macos' | 'linux' | 'windows';

export function getClientPlatform(): ClientPlatform {
  if (typeof navigator === 'undefined') return 'macos';
  const platform = ((navigator.userAgent || '') + ' ' + (navigator.platform || '')).toLowerCase();
  return platform.includes('mac') ? 'macos' : platform.includes('win') ? 'windows' : 'linux';
}

export interface XTerminalViewProps {
  sessionId: string;
  onSessionTerminated?: () => void;
  className?: string;
  autoFocus?: boolean;
  visible?: boolean;
  clearToken?: number;
}

export const XTerminalView = ({ sessionId, onSessionTerminated, className = '', autoFocus = true, visible = true, clearToken = 0 }: XTerminalViewProps) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const visibleRef = useRef(visible);
  visibleRef.current = visible;
  const exitRef = useRef(onSessionTerminated);
  exitRef.current = onSessionTerminated;
  const [error, setError] = useState<string | null>(null);
  const [retry, setRetry] = useState(0);
  const platform = getClientPlatform();
  const fontFamily = platform === 'macos' ? 'Menlo, Monaco, monospace' : platform === 'windows' ? '"Cascadia Code", Consolas, monospace' : '"DejaVu Sans Mono", monospace';

  const fit = useCallback(() => {
    const term = termRef.current, container = containerRef.current;
    if (!term || !fitRef.current || !container || !visibleRef.current || container.clientWidth < 20 || container.clientHeight < 20) return;
    try {
      const before = [term.cols, term.rows];
      fitRef.current.fit();
      if (before[0] !== term.cols || before[1] !== term.rows) {
        void daemonClient.resizeTerminal(sessionId, term.cols, term.rows).catch(reason => setError('Could not resize terminal: ' + String(reason)));
      }
    } catch (reason) { setError('Could not fit terminal: ' + String(reason)); }
  }, [sessionId]);

  useEffect(() => {
    if (!containerRef.current || !sessionId) return;
    let disposed = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let resizeFrame = 0;
    let cursor = 0;
    let failures = 0;
    setError(null);
    const term = new Terminal({
      fontFamily, fontSize: 13, lineHeight: 1.35, scrollback: 5000,
      cursorBlink: true, cursorStyle: 'bar', macOptionIsMeta: platform === 'macos',
      rightClickSelectsWord: true, allowTransparency: false,
    });
    const addon = new FitAddon();
    term.loadAddon(addon);
    term.open(containerRef.current);
    termRef.current = term;
    fitRef.current = addon;
    const theme = () => {
      const style = getComputedStyle(document.documentElement);
      const token = (name: string, fallback: string) => style.getPropertyValue(name).trim() || fallback;
      term.options.theme = {
        background: token('--color-canvas', '#101113'),
        foreground: token('--color-fg-default', '#e7e7e7'),
        cursor: token('--color-fg-default', '#e7e7e7'),
        selectionBackground: token('--color-surface-3', '#45464a'),
      };
    };
    theme();
    const themeObserver = new MutationObserver(theme);
    themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['class', 'data-theme', 'style'] });
    const scheduleFit = () => {
      cancelAnimationFrame(resizeFrame);
      resizeFrame = requestAnimationFrame(() => { if (!disposed) fit(); });
    };
    scheduleFit();
    if (autoFocus && visibleRef.current) term.focus();
    const input = term.onData(data => {
      if (!visibleRef.current) return;
      void daemonClient.writeTerminal(sessionId, data).catch(reason => {
        if (!disposed) setError('Terminal input was not confirmed: ' + String(reason));
      });
    });
    const poll = async () => {
      if (disposed) return;
      let eof = false;
      try {
        const chunk = await daemonClient.readTerminal(sessionId, cursor, 65536);
        if (disposed) return;
        const next = chunk.next_seq ?? chunk.nextSeq ?? cursor;
        const start = chunk.start_seq ?? chunk.startSeq ?? cursor;
        if (start > cursor) term.writeln('\r\n[Earlier output is no longer retained]\r\n');
        if (chunk.data) await new Promise<void>(resolve => term.write(chunk.data, resolve));
        if (disposed) return;
        cursor = next;
        failures = 0;
        setError(null);
        eof = chunk.is_eof ?? chunk.isEof ?? false;
        if (eof) exitRef.current?.();
      } catch (reason) {
        if (disposed) return;
        failures++;
        setError('Terminal connection interrupted. Reconnecting… ' + String(reason));
      }
      if (!disposed && !eof) timer = setTimeout(() => void poll(), failures ? Math.min(5000, 500 * 2 ** Math.min(failures, 4)) : visibleRef.current ? 80 : 500);
    };
    void poll();
    const resizeObserver = new ResizeObserver(scheduleFit);
    resizeObserver.observe(containerRef.current);
    window.addEventListener('resize', scheduleFit);
    return () => {
      disposed = true;
      clearTimeout(timer);
      cancelAnimationFrame(resizeFrame);
      window.removeEventListener('resize', scheduleFit);
      resizeObserver.disconnect();
      themeObserver.disconnect();
      input.dispose();
      term.dispose();
      if (termRef.current === term) { termRef.current = null; fitRef.current = null; }
    };
  }, [sessionId, fontFamily, platform, fit, retry, autoFocus]);

  useEffect(() => { if (visible) fit(); }, [visible, fit]);
  useEffect(() => { if (clearToken) termRef.current?.clear(); }, [clearToken]);

  return <div className={'flex h-full min-h-0 flex-col bg-canvas ' + className}>
    {error && <div role="alert" className="wb-error">
      <span className="min-w-0 flex-1">{error}</span>
      <button type="button" className="canvas-open-button" onClick={() => setRetry(value => value + 1)}>Reconnect</button>
    </div>}
    <div ref={containerRef} className="min-h-0 flex-1 overflow-hidden p-2" onClick={() => termRef.current?.focus()} />
  </div>;
};
