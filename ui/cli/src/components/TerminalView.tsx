import React, { useState, useRef, useEffect } from 'react';
import { OperationalMode } from '../types';
import { getModeSlug, MODE_SLASH_ITEMS, ROOT_SLASH_ITEMS, SlashItem } from './terminal/types';
import { useTerminalCommands } from './terminal/useTerminalCommands';
import { TerminalLineItem } from './terminal/TerminalLineItem';
import { ModeSelectorPalette } from './terminal/ModeSelectorPalette';
import { TerminalPrompt } from './terminal/TerminalPrompt';
import { Trash2, Copy, Check, Sparkles, HelpCircle, Sliders } from 'lucide-react';

interface TerminalViewProps {
  currentMode: OperationalMode;
  onModeChange: (mode: OperationalMode) => void;
  initialCommand?: string;
  onClearInitialCommand?: () => void;
  onOpenWorkspace?: () => void;
}

export const TerminalView: React.FC<TerminalViewProps> = ({
  currentMode,
  onModeChange,
  initialCommand,
  onClearInitialCommand,
  onOpenWorkspace,
}) => {
  const [inputVal, setInputVal] = useState('');
  const [copied, setCopied] = useState(false);
  const [paletteLevel, setPaletteLevel] = useState<'root' | 'mode'>('root');

  const terminalEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const {
    lines,
    history,
    historyIndex,
    setHistoryIndex,
    isProcessing,
    showModeSelector,
    setShowModeSelector,
    selectedModeIndex,
    setSelectedModeIndex,
    vibeSession,
    selectMode,
    resetToCustos,
    clearTerminal,
    executeCommand,
    handlePermitDecision,
  } = useTerminalCommands({ currentMode, onModeChange });

  const modeSlug = getModeSlug(currentMode);
  const isWelcome = lines.length === 1 && lines[0]?.type === 'banner';

  const paletteItems = paletteLevel === 'mode' ? MODE_SLASH_ITEMS : ROOT_SLASH_ITEMS;
  const filterQuery = inputVal.startsWith('/') ? inputVal.slice(1).trim() : '';
  const query = filterQuery.toLowerCase();
  const filteredSlashItems = paletteItems.filter((item) => {
    if (!query) return true;
    return item.name.toLowerCase().includes(query);
  });

  const scrollToBottom = () => {
    terminalEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [lines, vibeSession, showModeSelector]);

  useEffect(() => {
    if (initialCommand) {
      executeCommand(initialCommand);
      if (onClearInitialCommand) onClearInitialCommand();
    }
  }, [initialCommand]);

  useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      if (e.key === '/' && document.activeElement !== inputRef.current) {
        const target = e.target as HTMLElement | null;
        if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)) {
          return;
        }
        e.preventDefault();
        inputRef.current?.focus();
        setInputVal('/');
        setPaletteLevel('root');
        setShowModeSelector(true);
        setSelectedModeIndex(0);
      }
    };

    window.addEventListener('keydown', handleGlobalKeyDown);
    return () => window.removeEventListener('keydown', handleGlobalKeyDown);
  }, []);

  const handleCopyLogs = () => {
    const raw = lines.map((l) => l.content).join('\n');
    navigator.clipboard.writeText(raw);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleSelectSlashItem = (item: SlashItem) => {
    if (item.type === 'submenu') {
      setPaletteLevel('mode');
      setInputVal('/');
      setSelectedModeIndex(0);
      inputRef.current?.focus();
      return;
    }
    setShowModeSelector(false);
    setInputVal('');
    if (item.type === 'mode') {
      if (item.id === 'mode-custos' || item.mode === 'custos') {
        resetToCustos();
      } else if (item.mode) {
        selectMode(item.mode as OperationalMode);
      }
    } else if (item.type === 'command' && item.command) {
      executeCommand(item.command);
    }
    inputRef.current?.focus();
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setInputVal(val);
    if (val.startsWith('/')) {
      if (val === '/') setPaletteLevel('root');
      setShowModeSelector(true);
      setSelectedModeIndex(0);
    } else if (showModeSelector) {
      setShowModeSelector(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Tab' && isWelcome) {
      e.preventDefault();
      setPaletteLevel('mode');
      setShowModeSelector(true);
      setSelectedModeIndex(0);
      setInputVal('/');
      return;
    }

    if (showModeSelector) {
      if (e.key === 'Escape') {
        e.preventDefault();
        if (paletteLevel === 'mode') {
          setPaletteLevel('root');
          setInputVal('/');
          setSelectedModeIndex(0);
          return;
        }
        setShowModeSelector(false);
        setInputVal('');
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        const len = Math.max(1, filteredSlashItems.length);
        setSelectedModeIndex((prev) => (prev - 1 + len) % len);
        return;
      }
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        const len = Math.max(1, filteredSlashItems.length);
        setSelectedModeIndex((prev) => (prev + 1) % len);
        return;
      }
      if (e.key === 'Enter') {
        e.preventDefault();
        const item = filteredSlashItems[selectedModeIndex];
        if (item) {
          handleSelectSlashItem(item);
        }
        return;
      }
    }

    if (e.key === '/') {
      if (inputVal === '' || inputVal === '/') {
        setShowModeSelector(true);
        setSelectedModeIndex(0);
      }
    }

    if (e.key === 'Enter') {
      if (inputVal.trim() === '/mode' || inputVal.trim() === 'mode') {
        setPaletteLevel('mode');
        setShowModeSelector(true);
        setSelectedModeIndex(0);
        setInputVal('/');
        return;
      }
      executeCommand(inputVal);
      setInputVal('');
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (history.length > 0 && historyIndex < history.length - 1) {
        const nextIdx = historyIndex + 1;
        setHistoryIndex(nextIdx);
        setInputVal(history[nextIdx]);
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (historyIndex > 0) {
        const nextIdx = historyIndex - 1;
        setHistoryIndex(nextIdx);
        setInputVal(history[nextIdx]);
      } else if (historyIndex === 0) {
        setHistoryIndex(-1);
        setInputVal('');
      }
    }
  };

  const handleSend = () => {
    if (!inputVal.trim()) return;
    if (inputVal.trim() === '/mode' || inputVal.trim() === 'mode') {
      setPaletteLevel('mode');
      setShowModeSelector(true);
      setSelectedModeIndex(0);
      setInputVal('/');
      return;
    }
    executeCommand(inputVal);
    setInputVal('');
  };

  return (
    <div className={`terminal-view-card ${isWelcome ? 'is-welcome' : 'is-active'}`}>
      <div className="terminal-top-bar">
        <div className="terminal-controls">
          <span className="dot dot-red" />
          <span className="dot dot-yellow" />
          <span className="dot dot-green" />
          <span className="terminal-title">
            custos:~/workspace ({modeSlug}) — bash 80x24
          </span>
        </div>

        <div className="terminal-actions">
          <button
            className="term-action-btn mode-pill-btn"
            onClick={() => setShowModeSelector((prev) => !prev)}
            title="Chọn mode (/)"
          >
            <Sliders size={12} />
            <span>{modeSlug}</span>
          </button>
          <button className="term-action-btn" onClick={() => executeCommand('vibe')}>
            <Sparkles size={12} />
            <span>vibe</span>
          </button>
          <button className="term-action-btn" onClick={() => executeCommand('list')}>
            <span>list</span>
          </button>
          <button className="term-action-btn" onClick={() => executeCommand('help')}>
            <HelpCircle size={12} />
          </button>
          <button className="term-action-btn" onClick={handleCopyLogs} title="Copy logs">
            {copied ? <Check size={12} className="copy-ok" /> : <Copy size={12} />}
          </button>
          <button className="term-action-btn" onClick={clearTerminal} title="Clear terminal">
            <Trash2 size={12} />
          </button>
        </div>
      </div>

      <div className="terminal-screen" onClick={() => inputRef.current?.focus()}>
        {lines.map((line) => (
          <TerminalLineItem
            key={line.id}
            line={line}
            currentMode={currentMode}
            isPermitPending={vibeSession.step === 'permit'}
            onOpenModeSelector={() => setShowModeSelector(true)}
            onRunCommand={executeCommand}
            onPermitDecision={handlePermitDecision}
          />
        ))}
        <div ref={terminalEndRef} />
      </div>

      <div className="terminal-bottom-container">
        <div className="terminal-prompt-wrapper">
          {showModeSelector && (
            <ModeSelectorPalette
              items={paletteItems}
              title={paletteLevel === 'mode' ? 'CHỌN MODE' : 'LỆNH & CHỨC NĂNG CUSTOS'}
              filterQuery={filterQuery}
              selectedIndex={selectedModeIndex}
              onSelectIndex={setSelectedModeIndex}
              onSelectItem={handleSelectSlashItem}
            />
          )}

          <TerminalPrompt
            currentMode={currentMode}
            inputVal={inputVal}
            isProcessing={isProcessing}
            showModeSelector={showModeSelector}
            vibeSession={vibeSession}
            inputRef={inputRef}
            onInputChange={handleInputChange}
            onKeyDown={handleKeyDown}
            onToggleModeSelector={() => {
              setPaletteLevel('root');
              setShowModeSelector((prev) => !prev);
              setSelectedModeIndex(0);
              inputRef.current?.focus();
            }}
            onSend={handleSend}
          />

          {isWelcome && (
            <div className="focus-shortcuts" aria-label="Phím tắt và điều hướng">
              <button type="button" onClick={() => setShowModeSelector(true)}>
                <kbd>/</kbd> lệnh
              </button>
              <button type="button" onClick={() => setShowModeSelector(true)}>
                <kbd>Tab</kbd> đổi chế độ
              </button>
              <button type="button" onClick={onOpenWorkspace}>
                không gian làm việc
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
