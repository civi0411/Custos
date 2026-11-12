import { useState } from 'react';
import { OperationalMode, TerminalLine, Task } from '../../types';
import { CustosApi, isTauriEnvironment } from '../../services/custosApi';
import { getModeSlug, VibeSessionState } from './types';

interface UseTerminalCommandsProps {
  currentMode: OperationalMode | string;
  onModeChange: (mode: OperationalMode) => void;
}

export function useTerminalCommands({ currentMode, onModeChange }: UseTerminalCommandsProps) {
  const [lines, setLines] = useState<TerminalLine[]>([
    {
      id: 'init_banner',
      type: 'banner',
      content: '',
    },
  ]);

  const [history, setHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState<number>(-1);
  const [isProcessing, setIsProcessing] = useState(false);
  const [showModeSelector, setShowModeSelector] = useState(false);
  const [selectedModeIndex, setSelectedModeIndex] = useState(0);

  const [vibeSession, setVibeSession] = useState<VibeSessionState>({ step: 'none' });

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

  const selectMode = (m: OperationalMode) => {
    onModeChange(m);
    setShowModeSelector(false);
    const slug = getModeSlug(m);
    addLine('system', `✔ Đã chuyển sang chế độ [${slug}].`);
  };

  const resetToCustos = () => {
    onModeChange('Custos');
    setShowModeSelector(false);
    addLine('system', '✔ Đã quay lại chế độ [custos].');
  };

  const clearTerminal = () => {
    setLines([]);
    setVibeSession({ step: 'none' });
  };

  const runVibeExecution = async (goal: string, mode: OperationalMode) => {
    setIsProcessing(true);
    addLine('info', `[${mode.toUpperCase()}] Đăng ký nhiệm vụ tới custos-daemon: "${goal}"...`);

    try {
      const task = await CustosApi.createTask(goal, mode);
      addLine('success', `✔ Nhiệm vụ đã được ghi nhận: ${task.id} [${task.status}]`);

      const oldState = `status: ${task.status}\nepoch: ${task.epoch}`;
      const newState = `status: Queued -> Running -> Succeeded\nmode: ${mode.toLowerCase()}\ngoal: ${goal}`;

      addLine('diff', '', {
        filePath: `task/${task.id}`,
        oldCode: oldState,
        newCode: newState,
        additions: 3,
        deletions: 2,
      });

      const permit = CustosApi.createPermit(
        task.id,
        'ExecutionPermit: Queue & Execute Task',
        task.id,
        'Medium',
        `task/${task.id}`,
        oldState,
        newState
      );

      setIsProcessing(false);
      setVibeSession({
        step: 'permit',
        mode,
        goal,
        task,
        permit,
      });

      addLine('warning', '\n[MEDIUM RISK] ExecutionPermit: Cần xác nhận trước khi thực thi nhiệm vụ.');
      addLine('prompt', 'Phê duyệt thực thi nhiệm vụ này? [Y/n] (hoặc bấm nút bên dưới):', {
        permitId: permit.id,
        showControls: true,
      });
    } catch (err) {
      setIsProcessing(false);
      addLine('error', `✖ Không thể tạo nhiệm vụ: ${err instanceof Error ? err.message : String(err)}`);
    }
  };

  const handlePermitDecision = async (approved: boolean) => {
    if (!vibeSession.task || !vibeSession.permit) return;

    const task = vibeSession.task;
    const permit = vibeSession.permit;

    CustosApi.resolvePermit(permit.id, approved);
    setVibeSession({ step: 'none' });

    try {
      if (approved) {
        addLine('success', '✔ Đã phê duyệt. Đang chuyển trạng thái nhiệm vụ trên custos-daemon.');
        setIsProcessing(true);

        const queued = await CustosApi.advanceTask(task.id, 'Queued', 'Permit granted');
        addLine('info', `Nhiệm vụ ${queued.id} chuyển sang: [${queued.status}]`);

        const running = await CustosApi.advanceTask(task.id, 'Running', 'Operator approved execution');
        addLine('info', `Nhiệm vụ ${running.id} đang thực thi: [${running.status}]`);

        const comp = await CustosApi.completeTask(
          task.id,
          `Nhiệm vụ '${vibeSession.goal}' đã hoàn tất dưới cơ chế Epoch Lock`
        );
        addLine('success', `Nhiệm vụ ${comp.id}: [${comp.status}]`);
        addLine('success', '✔ Hoàn thành nhiệm vụ. Đã ghi audit span vào SQLite.');
        setIsProcessing(false);
      } else {
        const canc = await CustosApi.cancelTask(task.id, 'Execution rejected by operator.');
        addLine('warning', `Nhiệm vụ ${canc.id} đã hủy: [${canc.status}]`);
        addLine('error', '✖ Đã từ chối thực thi.');
      }
    } catch (err) {
      setIsProcessing(false);
      addLine('error', `✖ Lỗi cập nhật nhiệm vụ: ${err instanceof Error ? err.message : String(err)}`);
    }
  };

  const executeCommand = async (cmd: string) => {
    const trimmed = cmd.trim();
    if (!trimmed) return;

    setHistory((prev) => [trimmed, ...prev]);
    setHistoryIndex(-1);

    const activeSlug = getModeSlug(currentMode as OperationalMode);
    addLine('input', trimmed, { mode: currentMode as OperationalMode });

    if (trimmed === '/' || trimmed === '/mode' || trimmed === 'mode') {
      setShowModeSelector(true);
      return;
    }

    if (trimmed === '/code' || trimmed === 'code' || trimmed === 'custos-code') {
      selectMode('Code');
      return;
    }

    if (trimmed === '/research' || trimmed === 'research' || trimmed === 'custos-research') {
      selectMode('Research');
      return;
    }

    if (
      trimmed === '/assistant' ||
      trimmed === '/assitant' ||
      trimmed === 'assistant' ||
      trimmed === 'custos-assistant'
    ) {
      selectMode('Assistant');
      return;
    }

    if (trimmed === '/custos' || trimmed === 'custos' || trimmed === 'exit-mode') {
      resetToCustos();
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

    const parts = trimmed.split(/\s+/);
    let firstWord = parts[0].toLowerCase();
    if (firstWord.startsWith('/')) {
      firstWord = firstWord.slice(1);
    }
    const command = firstWord === 'custos' ? (parts[1] || '').toLowerCase() : firstWord;
    const cleanCommand = command.startsWith('/') ? command.slice(1) : command;
    const args = firstWord === 'custos' ? parts.slice(2) : parts.slice(1);

    switch (cleanCommand) {
      case 'clear':
      case 'cls':
        clearTerminal();
        break;

      case 'permit':
      case 'p': {
        const pending = CustosApi.getPendingPermits();
        if (pending.length === 0) {
          addLine('info', 'Hiện không có Giấy phép Thực thi (ExecutionPermit) nào đang chờ duyệt.');
        } else {
          const p = pending[0];
          addLine('warning', `[${p.risk.toUpperCase()} RISK] ${p.actionType}`);
          addLine('info', `Tệp mục tiêu: ${p.filePath}`);
          addLine('prompt', 'Phê duyệt giấy phép thực thi này? [Y/n] (hoặc bấm nút bên dưới):', {
            permitId: p.id,
            showControls: true,
          });
        }
        break;
      }

      case 'help':
      case '?':
        addLine('info', '══ BẢNG TRA CỨU LỆNH CUSTOS ══');
        addLine('info', "• / hoặc mode      : Mở bảng chọn mode (custos-code, custos-research, custos-assistant)");
        addLine('info', '• code / research  : Chuyển trực tiếp sang mode lập trình hoặc nghiên cứu');
        addLine('info', '• assistant        : Chuyển trực tiếp sang mode trợ lý điều phối');
        addLine('info', '• custos           : Quay lại chế độ gốc để chat tự do');
        addLine('info', '• vibe [mục tiêu]  : Khởi chạy quy trình Vibe Coding có kiểm duyệt Sandbox');
        addLine('info', '• list / tasks     : Xem danh sách nhiệm vụ và SQLite spans');
        addLine('info', '• create [tiêu đề] : Tạo nhiệm vụ mới vào hàng đợi');
        addLine('info', '• advance [id]     : Chuyển bước tiến trình nhiệm vụ');
        addLine('info', '• diff             : Xem so sánh bản vá AST');
        addLine('info', '• permit           : Xem xét và ký giấy phép thực thi');
        addLine('info', '• status           : Kiểm tra kết nối daemon và sandbox');
        addLine('info', '• clear / cls      : Xóa màn hình terminal');
        break;

      case 'status': {
        try {
          const tasks = await CustosApi.listTasks();
          const pending = CustosApi.getPendingPermits();
          const runtimeLabel = isTauriEnvironment() ? 'Desktop Tauri IPC' : 'Daemon HTTP RPC';
          addLine(
            'info',
            `${runtimeLabel} | Nhiệm vụ: ${tasks.length} | Chờ phê duyệt: ${pending.length} | Chế độ: [${activeSlug}]`
          );
        } catch (err) {
          addLine('error', `✖ Daemon không khả dụng: ${err instanceof Error ? err.message : String(err)}`);
        }
        break;
      }

      case 'list':
      case 'tasks':
      case 'ls': {
        try {
          const tasks = await CustosApi.listTasks();
          if (tasks.length === 0) {
            addLine('info', 'Chưa có nhiệm vụ nào trong hàng đợi.');
          } else {
            addLine('info', `ID        | STATUS    | MODE      | TIÊU ĐỀ`);
            addLine('info', '──────────┼───────────┼───────────┼─────────────────────────────────');
            tasks.forEach((t: Task) => {
              const taskMode = (t.metadata?.mode as string) || 'code';
              addLine(
                'info',
                `${t.id.padEnd(9)} | ${t.status.padEnd(9)} | ${taskMode.padEnd(9)} | ${t.title}`
              );
            });
          }
        } catch (err) {
          addLine('error', `✖ Lỗi đọc danh sách tasks: ${err instanceof Error ? err.message : String(err)}`);
        }
        break;
      }

      case 'create':
      case 'new': {
        const title = args.join(' ') || 'Nhiệm vụ mới từ CLI';
        const mode = (typeof currentMode === 'string' && ['Code', 'Research', 'Assistant'].includes(currentMode)
          ? currentMode
          : 'Code') as OperationalMode;
        try {
          const task = await CustosApi.createTask(title, mode);
          const taskMode = (task.metadata?.mode as string) || mode;
          addLine('success', `✔ Đã tạo task mới [${task.id}] (${taskMode}): "${task.title}"`);
        } catch (err) {
          addLine('error', `✖ Không thể tạo task: ${err instanceof Error ? err.message : String(err)}`);
        }
        break;
      }

      case 'advance': {
        try {
          const tasks = await CustosApi.listTasks();
          const targetId = args[0] || tasks[0]?.id;
          if (!targetId) {
            addLine('warning', 'Không tìm thấy task để advance.');
            break;
          }
          const updated = await CustosApi.advanceTask(targetId, 'Running', 'Manual CLI advance');
          addLine('success', `✔ Task [${updated.id}] đã chuyển trạng thái sang: [${updated.status}]`);
        } catch (err) {
          addLine('error', `✖ Không thể advance task: ${err instanceof Error ? err.message : String(err)}`);
        }
        break;
      }

      case 'diff':
      case 'd': {
        try {
          const tasks = await CustosApi.listTasks();
          const targetId = args[0] || tasks[0]?.id;
          if (!targetId) {
            addLine('info', 'Không có nhiệm vụ nào để kiểm tra spans/diff.');
            break;
          }
          const spans = await CustosApi.listSpans(targetId);
          addLine('info', `Spans thực tế của task ${targetId}: ${spans.length} span(s).`);
          if (spans.length > 0) {
            addLine('diff', '', {
              filePath: `task/${targetId}/spans`,
              oldCode: `task: ${targetId}`,
              newCode: spans.map((s) => `${s.stage}: ${s.name}${s.details ? ` — ${s.details}` : ''}`).join('\n'),
              additions: spans.length,
              deletions: 0,
            });
          }
        } catch (err) {
          addLine('error', `✖ Lỗi đọc diff/spans: ${err instanceof Error ? err.message : String(err)}`);
        }
        break;
      }

      case 'vibe':
      case 'v': {
        const goal = args.join(' ') || 'Tối ưu hóa và kiểm tra invariants trong Sandbox';
        const mode = (typeof currentMode === 'string' && ['Code', 'Research', 'Assistant'].includes(currentMode)
          ? currentMode
          : 'Code') as OperationalMode;
        await runVibeExecution(goal, mode);
        break;
      }

      default: {
        const normalizedMode = String(currentMode).toLowerCase();
        const isSpecialized = normalizedMode !== 'standard' && normalizedMode !== 'custos';
        if (isSpecialized) {
          await runVibeExecution(trimmed, currentMode as OperationalMode);
        } else {
          setIsProcessing(true);
          await new Promise((r) => setTimeout(r, 400));
          setIsProcessing(false);
          addLine(
            'info',
            'Custos Sovereign Agentic Runtime đang ở chế độ giao tiếp chung. Bạn có thể trò chuyện tự do hoặc nhấn \'/\' để kích hoạt engine chuyên biệt (custos-code, custos-research, custos-assistant).'
          );
        }
        break;
      }
    }
  };

  return {
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
    addLine,
    selectMode,
    resetToCustos,
    clearTerminal,
    executeCommand,
    handlePermitDecision,
  };
}
