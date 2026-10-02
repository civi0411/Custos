import React, { useState, useRef, useEffect } from 'react';
import { ExecutionPermit, OperationalMode, TerminalLine, Task, TaskStatus } from '../types';
import { ASCII_BANNER, CUSTOS_VERSION, SAMPLE_DIFF } from '../data/constants';
import { CustosApi } from '../services/custosApi';
import { DiffViewer } from './DiffViewer';
import { Trash2, Copy, Check, Sparkles, HelpCircle } from 'lucide-react';

interface TerminalViewProps {
  currentMode: OperationalMode;
  onModeChange: (mode: OperationalMode) => void;
  initialCommand?: string;
  onClearInitialCommand?: () => void;
}

export const TerminalView: React.FC<TerminalViewProps> = ({
  currentMode,
  onModeChange,
  initialCommand,
  onClearInitialCommand,
}) => {
  const [lines, setLines] = useState<TerminalLine[]>([
    {
      id: 'init_banner',
      type: 'banner',
      content: ASCII_BANNER,
    },
    {
      id: 'init_welcome',
      type: 'system',
      content: `Custos CLI Runtime initialized. Active mode: [${currentMode}]. Type 'help' or 'custos vibe' to begin.`,
    },
  ]);

  const [inputVal, setInputVal] = useState('');
  const [history, setHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState<number>(-1);
  const [copied, setCopied] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);

  // Guided interactive vibe session state inside terminal
  const [vibeSession, setVibeSession] = useState<{
    step: 'none' | 'mode' | 'goal' | 'permit';
    mode?: OperationalMode;
    goal?: string;
    task?: Task;
    permit?: ExecutionPermit;
  }>({ step: 'none' });

  const terminalEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Auto scroll to bottom
  const scrollToBottom = () => {
    terminalEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [lines, vibeSession]);

  // Execute initial command if provided
  useEffect(() => {
    if (initialCommand) {
      executeCommand(initialCommand);
      if (onClearInitialCommand) onClearInitialCommand();
    }
  }, [initialCommand]);

  const addLine = (type: TerminalLine['type'], content: string, metadata?: TerminalLine['metadata']) => {
    const newLine: TerminalLine = {
      id: `line_${Date.now()}_${Math.random().toString(36).substring(2, 6)}`,
      type,
      content,
      timestamp: new Date().toLocaleTimeString(),
      metadata,
    };
    setLines((prev) => [...prev, newLine]);
  };

  const handleCopyLogs = () => {
    const raw = lines.map((l) => l.content).join('\n');
    navigator.clipboard.writeText(raw);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const clearTerminal = () => {
    setLines([]);
    setVibeSession({ step: 'none' });
  };

  const executeCommand = async (cmd: string) => {
    const trimmed = cmd.trim();
    if (!trimmed) return;

    // Add to history
    setHistory((prev) => [trimmed, ...prev]);
    setHistoryIndex(-1);

    // Echo input
    addLine('input', `$ ${trimmed}`);

    // If currently in a guided vibe prompt step
    if (vibeSession.step === 'mode') {
      let selected: OperationalMode = 'Code';
      if (trimmed === '1' || trimmed.toLowerCase() === 'code') selected = 'Code';
      else if (trimmed === '2' || trimmed.toLowerCase() === 'research') selected = 'Research';
      else if (trimmed === '3' || trimmed.toLowerCase().includes('assist')) selected = 'Assitant';
      else {
        addLine('warning', `Invalid choice '${trimmed}', defaulting to Code mode.`);
      }

      onModeChange(selected);
      setVibeSession({ step: 'goal', mode: selected });
      addLine('card', '', { mode: selected });
      return;
    }

    if (vibeSession.step === 'goal') {
      const mode = vibeSession.mode || currentMode;
      runVibeExecution(trimmed, mode);
      return;
    }

    if (vibeSession.step === 'permit') {
      const lower = trimmed.toLowerCase();
      if (lower === 'y' || lower === 'yes') {
        await handlePermitDecision(true);
      } else {
        await handlePermitDecision(false);
      }
      return;
    }

    // Parse standard CLI commands
    const parts = trimmed.split(/\s+/);
    const command = parts[0].toLowerCase() === 'custos' ? (parts[1] || '').toLowerCase() : parts[0].toLowerCase();
    const args = parts[0].toLowerCase() === 'custos' ? parts.slice(2) : parts.slice(1);

    switch (command) {
      case 'vibe': {
        // Check for flags --prompt / -p and --mode / -m
        let promptArg: string | undefined;
        let modeArg: OperationalMode | undefined;

        for (let i = 0; i < args.length; i++) {
          if (args[i] === '-p' || args[i] === '--prompt') {
            promptArg = args.slice(i + 1).join(' ').replace(/^["']|["']$/g, '');
            break;
          }
          if (args[i] === '-m' || args[i] === '--mode') {
            const m = args[i + 1]?.toLowerCase();
            if (m === 'code') modeArg = 'Code';
            else if (m === 'research') modeArg = 'Research';
            else if (m === 'assistant' || m === 'assitant') modeArg = 'Assitant';
          }
        }

        if (modeArg) {
          onModeChange(modeArg);
        }

        if (promptArg) {
          runVibeExecution(promptArg, modeArg || currentMode);
        } else {
          // Launch interactive mode selection
          setVibeSession({ step: 'mode' });
          addLine('prompt', '❄ Chọn chế độ Custos (Operational Mode):');
          addLine('info', '  1. Code       • Lập trình, sửa lỗi & tái cấu trúc mã nguồn');
          addLine('info', '  2. Research   • Điều tra, kiến trúc & suy luận chuyên sâu');
          addLine('info', '  3. Assistant  • Trợ lý điều phối quy trình tự động');
          addLine('prompt', 'Nhập [1, 2, 3] hoặc tên chế độ:');
        }
        break;
      }

      case 'help': {
        addLine('system', `
Custos Agentic Work Runtime CLI (v${CUSTOS_VERSION})

USAGE:
  custos [OPTIONS] [COMMAND]

COMMANDS:
  vibe [-p <prompt>] [-m <mode>]   Interactive agentic vibe trajectory & permit loop
  create -t <title> [-m <json>]   Create and register a new agentic task
  status -i <id>                   Inspect task details, epoch, and audit spans
  list                             List all tasks in the local SQLite store
  advance -i <id> -s <status>      Advance task state with concurrency check
  cancel -i <id> [-r <reason>]     Cancel an active task
  complete -i <id> [-s <summary>]  Complete task and finalize audit trail
  explain [-q <query>]             Explain repository architecture slice
  mode <code|research|assistant>   Switch operational mode
  banner                           Display Custos Snowy Owl banner & pillars
  clear                            Clear terminal screen
  help                             Show this manual
`);
        break;
      }

      case 'banner': {
        addLine('banner', ASCII_BANNER);
        break;
      }

      case 'clear': {
        clearTerminal();
        break;
      }

      case 'code': {
        onModeChange('Code');
        addLine('card', '', { mode: 'Code' });
        break;
      }

      case 'research': {
        onModeChange('Research');
        addLine('card', '', { mode: 'Research' });
        break;
      }

      case 'assistant':
      case 'assitant': {
        onModeChange('Assitant');
        addLine('card', '', { mode: 'Assitant' });
        break;
      }

      case 'mode': {
        const target = args[0]?.toLowerCase();
        if (target === 'code') {
          onModeChange('Code');
          addLine('card', '', { mode: 'Code' });
        } else if (target === 'research') {
          onModeChange('Research');
          addLine('card', '', { mode: 'Research' });
        } else if (target === 'assistant' || target === 'assitant') {
          onModeChange('Assitant');
          addLine('card', '', { mode: 'Assitant' });
        } else {
          addLine('warning', `Unknown mode '${target}'. Options: code, research, assistant.`);
        }
        break;
      }

      case 'list': {
        setIsProcessing(true);
        const tasks = await CustosApi.listTasks();
        setIsProcessing(false);
        if (tasks.length === 0) {
          addLine('info', 'No tasks found. Use `custos vibe` or `custos create -t "title"` to create one.');
        } else {
          addLine('info', `Found ${tasks.length} task(s) in SQLite database:`);
          tasks.forEach((t) => {
            const mode = (t.metadata?.mode as string) || 'code';
            addLine('output', `  • [${t.id}] [${t.status}] e${t.epoch} [${mode}] - ${t.title}`);
          });
        }
        break;
      }

      case 'status': {
        const idIndex = args.indexOf('-i');
        const id = idIndex !== -1 ? args[idIndex + 1] : args[0];
        if (!id) {
          addLine('error', "Error: missing required argument '<id>'. Usage: custos status -i <task_id>");
          return;
        }

        setIsProcessing(true);
        const task = await CustosApi.getTask(id);
        const spans = await CustosApi.listSpans(id);
        setIsProcessing(false);

        if (!task) {
          addLine('error', `Task with ID '${id}' not found in database.`);
        } else {
          addLine('info', `
Task ID:     ${task.id}
Title:       ${task.title}
Status:      [${task.status}]
Epoch:       ${task.epoch}
Created:     ${task.created_at}
Updated:     ${task.updated_at}
Metadata:    ${JSON.stringify(task.metadata || {})}`);

          if (spans.length > 0) {
            addLine('info', `Spans (${spans.length}):`);
            spans.forEach((s) => {
              addLine('output', `  #${s.id} [${s.stage}] ${s.name} (${s.duration_ms || 0}ms) - ${s.details || ''}`);
            });
          }
        }
        break;
      }

      case 'create': {
        const titleIndex = args.indexOf('-t');
        const title = titleIndex !== -1 ? args.slice(titleIndex + 1).join(' ') : args.join(' ');
        if (!title.trim()) {
          addLine('error', "Error: missing task title. Usage: custos create -t <title>");
          return;
        }

        setIsProcessing(true);
        const task = await CustosApi.createTask(title, currentMode);
        setIsProcessing(false);
        addLine('success', `Created task: ${task.id} [${task.status}] (Epoch 1)`);
        break;
      }

      case 'advance': {
        const idIndex = args.indexOf('-i');
        const statusIndex = args.indexOf('-s');
        const id = idIndex !== -1 ? args[idIndex + 1] : args[0];
        const statusStr = statusIndex !== -1 ? args[statusIndex + 1] : args[1];

        if (!id || !statusStr) {
          addLine('error', "Usage: custos advance -i <id> -s <status> (e.g. Queued, Running, Succeeded)");
          return;
        }

        try {
          const updated = await CustosApi.advanceTask(id, statusStr as TaskStatus);
          addLine('success', `Task ${updated.id} advanced to [${updated.status}] (Epoch ${updated.epoch})`);
        } catch (err) {
          addLine('error', `Advance failed: ${err instanceof Error ? err.message : String(err)}`);
        }
        break;
      }

      case 'complete': {
        const idIndex = args.indexOf('-i');
        const id = idIndex !== -1 ? args[idIndex + 1] : args[0];
        if (!id) {
          addLine('error', "Usage: custos complete -i <id>");
          return;
        }

        try {
          const comp = await CustosApi.completeTask(id);
          addLine('success', `Task ${comp.id} completed: [${comp.status}] (Epoch ${comp.epoch})`);
        } catch (err) {
          addLine('error', `Complete failed: ${err instanceof Error ? err.message : String(err)}`);
        }
        break;
      }

      case 'cancel': {
        const idIndex = args.indexOf('-i');
        const id = idIndex !== -1 ? args[idIndex + 1] : args[0];
        if (!id) {
          addLine('error', "Usage: custos cancel -i <id>");
          return;
        }

        try {
          const canc = await CustosApi.cancelTask(id);
          addLine('warning', `Task ${canc.id} cancelled: [${canc.status}]`);
        } catch (err) {
          addLine('error', `Cancel failed: ${err instanceof Error ? err.message : String(err)}`);
        }
        break;
      }

      case 'explain': {
        setIsProcessing(true);
        const explanation = await CustosApi.explainArchitecture(args.join(' '));
        setIsProcessing(false);
        addLine('info', explanation);
        break;
      }

      default: {
        addLine('error', `custos: command not found: '${trimmed}'. Type 'help' for available commands.`);
        break;
      }
    }
  };

  const runVibeExecution = async (goal: string, mode: OperationalMode) => {
    setIsProcessing(true);
    addLine('info', `Inspecting repository, compiling context recipe & verifying trajectory for [${mode}] mode...`);

    // Multi-stage animated spinner emulation
    addLine('spinner', `⠋ Analyzing repository and compiling context recipe for ${mode} mode...`);
    await new Promise((r) => setTimeout(r, 600));

    addLine('spinner', '⠙ Synthesizing solution trajectory with LLM Provider...');
    await new Promise((r) => setTimeout(r, 700));

    const task = await CustosApi.createTask(goal, mode);
    addLine('success', `✔ Task registered: ${task.id} [${task.status}]`);

    // Proposed diff
    addLine('info', '\nProposed Worktree Modification:');
    addLine('diff', '', {
      filePath: SAMPLE_DIFF.filePath,
      additions: 4,
      deletions: 1,
    });

    // Create execution permit
    const permit = CustosApi.createPermit(
      task.id,
      'ExecutionPermit: Apply Code Diff',
      'crates/runtime/src/handler.rs',
      'Medium',
      SAMPLE_DIFF.filePath,
      SAMPLE_DIFF.oldCode,
      SAMPLE_DIFF.newCode
    );

    setIsProcessing(false);
    setVibeSession({
      step: 'permit',
      mode,
      goal,
      task,
      permit,
    });

    addLine('warning', '\n[MEDIUM RISK] ExecutionPermit: Apply Code Diff -> crates/runtime/src/handler.rs');
    addLine('prompt', 'Approve this execution permit? [Y/n] (or click buttons below):', {
      permitId: permit.id,
      showControls: true,
    });
  };

  const handlePermitDecision = async (approved: boolean) => {
    if (!vibeSession.task || !vibeSession.permit) return;

    const task = vibeSession.task;
    const permit = vibeSession.permit;

    CustosApi.resolvePermit(permit.id, approved);
    setVibeSession({ step: 'none' });

    if (approved) {
      addLine('success', '✔ Permit granted by operator. Worktree isolated and patched.');
      setIsProcessing(true);

      const queued = await CustosApi.advanceTask(task.id, 'Queued', 'Permit granted');
      addLine('info', `Task ${queued.id} advanced to: [${queued.status}]`);
      await new Promise((r) => setTimeout(r, 400));

      const running = await CustosApi.advanceTask(task.id, 'Running', 'Active coding in sandbox');
      addLine('info', `Task ${running.id} execution: [${running.status}]`);
      addLine('info', 'Provider actively coding and executing task payload in sandbox worktree...');
      await new Promise((r) => setTimeout(r, 700));

      const comp = await CustosApi.completeTask(
        task.id,
        `Task '${vibeSession.goal}' successfully executed and verified`
      );
      addLine('success', `Task ${comp.id} execution: [${comp.status}]`);
      addLine('success', '✔ Task completed successfully. All artifacts and judgments committed.');
      setIsProcessing(false);
    } else {
      const canc = await CustosApi.cancelTask(
        task.id,
        'Execution rejected by operator. Worktree untouched.'
      );
      addLine('warning', `Task ${canc.id} cancelled: [${canc.status}]`);
      addLine('error', '✖ Execution rejected by operator. Worktree untouched.');
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
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

  return (
    <div className="terminal-view-card">
      <div className="terminal-top-bar">
        <div className="terminal-controls">
          <span className="dot dot-red" />
          <span className="dot dot-yellow" />
          <span className="dot dot-green" />
          <span className="terminal-title">
            custos:~/workspace ({currentMode.toLowerCase()}) — bash 80x24
          </span>
        </div>

        <div className="terminal-actions">
          <button className="term-action-btn" onClick={() => executeCommand('custos vibe')}>
            <Sparkles size={12} />
            <span>vibe</span>
          </button>
          <button className="term-action-btn" onClick={() => executeCommand('custos list')}>
            <span>list</span>
          </button>
          <button className="term-action-btn" onClick={() => executeCommand('custos explain')}>
            <span>explain</span>
          </button>
          <button className="term-action-btn" onClick={() => executeCommand('help')}>
            <HelpCircle size={12} />
          </button>
          <button className="term-action-btn" onClick={handleCopyLogs} title="Copy terminal logs">
            {copied ? <Check size={12} className="copy-ok" /> : <Copy size={12} />}
          </button>
          <button className="term-action-btn" onClick={clearTerminal} title="Clear screen">
            <Trash2 size={12} />
          </button>
        </div>
      </div>

      <div className="terminal-screen" onClick={() => inputRef.current?.focus()}>
        {lines.map((line) => {
          if (line.type === 'banner') {
            return (
              <div key={line.id} className="terminal-banner-image-container">
                <img
                  src="/assets/banner-main.png"
                  alt="Custos CLI Guardian Banner"
                  className="terminal-banner-img"
                  onError={(e) => {
                    e.currentTarget.style.display = 'none';
                  }}
                />
              </div>
            );
          }

          if (line.type === 'card') {
            const mode = (line.metadata?.mode || currentMode) as OperationalMode;
            const modeImg =
              mode === 'Research'
                ? '/assets/custos-owl-inspector.png'
                : mode === 'Assitant'
                ? '/assets/custos-owl-steward.png'
                : '/assets/custos-owl-coder.png';

            const modeTitle =
              mode === 'Research'
                ? 'CHẾ ĐỘ: RESEARCH MODE (ĐIỀU TRA & NGHIÊN CỨU)'
                : mode === 'Assitant'
                ? 'CHẾ ĐỘ: ASSISTANT MODE (TRỢ LÝ & ĐIỀU PHỐI)'
                : 'CHẾ ĐỘ: CODE MODE (LẬP TRÌNH & THỰC THI)';

            const mascotName =
              mode === 'Research'
                ? 'Custos Inspector Owl'
                : mode === 'Assitant'
                ? 'Custos Steward Owl'
                : 'Custos Coder Owl';

            const bullets =
              mode === 'Research'
                ? [
                    'Điều tra cấu trúc dự án & đối soát bằng chứng',
                    'Phân tích tài liệu, kiến trúc & suy luận chuyên sâu',
                    'Kiểm chứng các thay đổi trước khi xin phê duyệt',
                  ]
                : mode === 'Assitant'
                ? [
                    'Điều phối quy trình tác vụ tự động theo chuẩn runtime',
                    'Quản lý trạng thái Task, Epoch Lock & Audit Logs',
                    'Theo dõi tiến trình & báo cáo kết quả thực thi',
                  ]
                : [
                    'Lập trình tính năng, sửa lỗi & tái cấu trúc mã nguồn',
                    'Thực thi an toàn trong Sandbox Worktree cách ly',
                    'Tối ưu thuật toán & kiểm tra tính đúng đắn',
                  ];

            return (
              <div key={line.id} className={`terminal-mode-card mode-${mode.toLowerCase()}`}>
                <div className="terminal-mode-card-owl">
                  <img src={modeImg} alt={mascotName} className="terminal-mode-owl-img" />
                </div>
                <div className="terminal-mode-card-speech">
                  <div className="speech-badge">[ {modeTitle} ]</div>
                  <div className="speech-title">Xin chào! Tôi là {mascotName}. ❄</div>
                  <div className="speech-quote">"Tôi có thể giúp gì cho bạn hôm nay?"</div>
                  <ul className="speech-bullets">
                    {bullets.map((b, idx) => (
                      <li key={idx}>• {b}</li>
                    ))}
                  </ul>
                  <div className="speech-hint">
                    Nhập mục tiêu nhiệm vụ hoặc mã lệnh bạn muốn thực thi bên dưới.
                  </div>
                </div>
              </div>
            );
          }

          if (line.type === 'diff') {
            return (
              <div key={line.id} className="terminal-diff-block">
                <DiffViewer
                  filePath={line.metadata?.filePath || SAMPLE_DIFF.filePath}
                  oldCode={SAMPLE_DIFF.oldCode}
                  newCode={SAMPLE_DIFF.newCode}
                  compact
                />
              </div>
            );
          }

          return (
            <div key={line.id} className={`terminal-output-line line-${line.type}`}>
              <span className="line-content">{line.content}</span>

              {/* Render inline permit buttons if requested */}
              {line.metadata?.showControls && vibeSession.step === 'permit' && (
                <div className="inline-permit-controls">
                  <button
                    className="inline-approve-btn"
                    onClick={() => handlePermitDecision(true)}
                  >
                    [ Y ] Approve Permit
                  </button>
                  <button
                    className="inline-reject-btn"
                    onClick={() => handlePermitDecision(false)}
                  >
                    [ N ] Reject Permit
                  </button>
                </div>
              )}
            </div>
          );
        })}

        {/* Live Prompt Line */}
        <div className="terminal-prompt-line">
          <span className="prompt-prefix">
            <span className="prompt-user">custos</span>
            <span className="prompt-at">@</span>
            <span className="prompt-host">local</span>
            <span className="prompt-sep">:</span>
            <span className="prompt-path">~/workspace</span>
            <span className="prompt-mode">({currentMode.toLowerCase()})</span>
            <span className="prompt-char">$</span>
          </span>

          <input
            ref={inputRef}
            type="text"
            className="terminal-input"
            value={inputVal}
            onChange={(e) => setInputVal(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={
              vibeSession.step === 'mode'
                ? 'Enter 1 (Code), 2 (Research), 3 (Assistant)...'
                : vibeSession.step === 'goal'
                ? 'Enter goal prompt...'
                : vibeSession.step === 'permit'
                ? 'Type y or n...'
                : "Type 'custos vibe', 'list', 'explain', 'help'..."
            }
            autoFocus
            disabled={isProcessing}
          />
          <div className="terminal-cursor" />
        </div>

        <div ref={terminalEndRef} />
      </div>
    </div>
  );
};
