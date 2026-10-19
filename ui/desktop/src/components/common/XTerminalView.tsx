import React, { useEffect, useRef, useCallback } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { daemonClient } from '@/api/daemon_client';

export type ClientPlatform = 'macos' | 'linux' | 'windows';

export function getClientPlatform(): ClientPlatform {
  if (typeof navigator === 'undefined') return 'macos';
  const ua = (navigator.userAgent || '').toLowerCase();
  const platform = (navigator.platform || '').toLowerCase();
  if (ua.includes('mac') || platform.includes('mac')) return 'macos';
  if (ua.includes('win') || platform.includes('win')) return 'windows';
  return 'linux';
}

export interface XTerminalViewProps {
  sessionId: string;
  onSessionTerminated?: () => void;
  className?: string;
  autoFocus?: boolean;
}

export const XTerminalView: React.FC<XTerminalViewProps> = ({
  sessionId,
  onSessionTerminated,
  className = '',
  autoFocus = true,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const seqRef = useRef<number>(0);
  const lastColsRowsRef = useRef<{ cols: number; rows: number }>({ cols: 80, rows: 24 });

  const platform = getClientPlatform();
  const isMac = platform === 'macos';
  const isWindows = platform === 'windows';

  // Font family based on operating system
  const fontFamily = isMac
    ? 'Menlo, Monaco, "Courier New", monospace'
    : isWindows
    ? '"Cascadia Code", Consolas, "Courier New", monospace'
    : '"DejaVu Sans Mono", "Ubuntu Mono", "Liberation Mono", monospace';

  const handleResize = useCallback(() => {
    if (!fitAddonRef.current || !termRef.current || !sessionId) return;
    try {
      fitAddonRef.current.fit();
      const cols = termRef.current.cols;
      const rows = termRef.current.rows;
      if (
        cols > 0 &&
        rows > 0 &&
        (cols !== lastColsRowsRef.current.cols || rows !== lastColsRowsRef.current.rows)
      ) {
        lastColsRowsRef.current = { cols, rows };
        daemonClient.resizeTerminal(sessionId, cols, rows).catch(() => {});
      }
    } catch {
      // Ignore fit layout errors when container is hidden
    }
  }, [sessionId]);

  useEffect(() => {
    if (!containerRef.current || !sessionId) return;

    // 1. Initialize xterm instance with clean JetBlack palette
    const term = new Terminal({
      cursorBlink: true,
      cursorStyle: 'block',
      fontFamily,
      fontSize: 12.5,
      lineHeight: 1.45,
      letterSpacing: 0,
      macOptionIsMeta: isMac,
      macOptionClickForcesSelection: isMac,
      rightClickSelectsWord: true,
      allowTransparency: true,
      theme: {
        background: '#0a0a0c',
        foreground: '#e6edf3',
        cursor: '#38bdf8',
        cursorAccent: '#0a0a0c',
        selectionBackground: 'rgba(56, 189, 248, 0.28)',
        selectionForeground: '#ffffff',
        black: '#0a0a0c',
        red: '#f87171',
        green: '#4ade80',
        yellow: '#facc15',
        blue: '#60a5fa',
        magenta: '#c084fc',
        cyan: '#38bdf8',
        white: '#f1f5f9',
        brightBlack: '#64748b',
        brightRed: '#fca5a5',
        brightGreen: '#86efac',
        brightYellow: '#fde047',
        brightBlue: '#93c5fd',
        brightMagenta: '#d8b4fe',
        brightCyan: '#67e8f9',
        brightWhite: '#ffffff',
      },
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);

    term.open(containerRef.current);
    termRef.current = term;
    fitAddonRef.current = fitAddon;

    // Initial sizing
    requestAnimationFrame(() => {
      handleResize();
      if (autoFocus) {
        term.focus();
      }
    });

    // 2. Keystrokes -> PTY stdin write
    const onDataDisposable = term.onData((data) => {
      daemonClient.writeTerminal(sessionId, data).catch((err) => {
        console.warn('Failed to send terminal data:', err);
      });
    });

    // 3. PTY stdout/stderr polling stream -> term.write()
    let isSubscribed = true;
    seqRef.current = 0;

    const pollStream = async () => {
      if (!isSubscribed) return;
      try {
        const chunk = await daemonClient.readTerminal(sessionId, seqRef.current, 65536);
        if (!isSubscribed) return;

        if (chunk.data && chunk.data.length > 0) {
          term.write(chunk.data);
          seqRef.current = chunk.nextSeq ?? chunk.next_seq ?? seqRef.current;
        }

        if (chunk.isEof ?? chunk.is_eof) {
          onSessionTerminated?.();
        }
      } catch {
        // Stream polling error
      }
    };

    void pollStream();
    const pollInterval = setInterval(pollStream, 25);

    // 4. ResizeObserver for responsive layout changes
    const resizeObserver = new ResizeObserver(() => {
      requestAnimationFrame(handleResize);
    });
    resizeObserver.observe(containerRef.current);

    // 5. Window resize listener
    window.addEventListener('resize', handleResize);

    return () => {
      isSubscribed = false;
      clearInterval(pollInterval);
      window.removeEventListener('resize', handleResize);
      resizeObserver.disconnect();
      onDataDisposable.dispose();
      term.dispose();
      termRef.current = null;
      fitAddonRef.current = null;
    };
  }, [sessionId, autoFocus, fontFamily, isMac, handleResize, onSessionTerminated]);

  return (
    <div
      ref={containerRef}
      onClick={() => termRef.current?.focus()}
      className={`h-full w-full bg-[#0a0a0c] overflow-hidden ${className}`}
      style={{ padding: '8px 10px' }}
    />
  );
};
