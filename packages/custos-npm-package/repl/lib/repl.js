import chalk from 'chalk';
import { input } from '@inquirer/prompts';
import { emitKeypressEvents } from 'node:readline';
import { stdin as inputDevice, stdout as outputDevice } from 'node:process';
import { daemonRequest } from './daemon.js';
import {
  enterAlternateScreen,
  leaveAlternateScreen,
  shutdownCli,
  waitForKeypress,
  getAgentTheme,
  promptSlashSelection,
} from './terminal_ui.js';
import {
  promptProviderModelSelection,
  promptModeOnly,
  promptProviderLogout,
} from './providers.js';
import {
  runVibeWizard,
  printTaskList,
  advanceTask,
  printUnifiedDiff,
  promptPermitApproval,
  printStatus,
  printHelp,
} from './commands.js';
import { renderHeroFrame, promptInteractiveChat } from './hero_chat.js';

export function constrainPromptForMode(prompt, mode) {
  const constraints = {
    custos: 'You are the general Custos chat assistant. Answer the user naturally and helpfully. Do not claim tools, files, or external actions were used unless the runtime provides a real result.',
    code: 'You are in Code mode. Only answer requests about programming, software engineering, debugging, testing, developer tools, or the current codebase. If the request is unrelated, briefly say it is outside Code mode and ask the user to switch mode.',
    research: 'You are in Research mode. Focus on evidence-oriented investigation, distinguish facts from inference, and state when sources or verification are missing. Decline unrelated execution work and suggest the appropriate mode.',
    assistant: 'You are in Assistant mode. Focus on planning, coordination, drafting, and user-approved actions. Do not claim an external action happened without a real receipt.',
  };
  const constraint = constraints[mode] || constraints.assistant;
  return `${constraint}\n\nUser request:\n${prompt}`;
}

export async function executeModelTurn({ prompt, mode, selection, chatTurns }) {
  if (!selection?.provider || !selection?.model) {
    throw new Error('Hãy đăng nhập provider và chọn model trước khi chạy agent.');
  }

  let frame = 0;
  let runId = null;
  let interrupted = false;
  const previewTurns = [...chatTurns, { query: prompt, mode, responseLines: [], meta: null }];
  const render = () => {
    outputDevice.write('\x1b[?25l\x1b[H' + renderHeroFrame({
      currentMode: mode,
      chatTurns: previewTurns,
      selectedProvider: selection.provider,
      selectedModel: selection.model,
      activity: { frame, label: 'agent đang chạy' },
    }));
  };
  const onExecutionKeypress = (_character, key) => {
    if (key?.name !== 'escape' || interrupted) return;
    interrupted = true;
    if (runId) {
      void daemonRequest('v1.workflow.cancel_run', { run_id: runId, reason: 'Interrupted from Custos CLI' });
    }
  };

  inputDevice.on('keypress', onExecutionKeypress);
  const animation = setInterval(() => {
    frame += 1;
    render();
  }, 90);
  render();

  try {
    const task = await daemonRequest('v1.tasks.create', {
      title: prompt,
      metadata: {
        mode,
        origin: 'custos-node-cli',
        provider_account_id: selection.provider.id,
        model_id: selection.model.id,
      },
    });
    const session = await daemonRequest('v1.sessions.create', { mode: 'assisted' });
    const handle = await daemonRequest('v1.workflow.start_run', {
      background: true,
      task_id: task.id,
      actor: 'custos-cli-user',
      preferred_mode: 'model',
      workspace_root: process.cwd(),
      session_id: session.id,
      prompt: constrainPromptForMode(prompt, mode),
      model: selection.model.id,
    });
    runId = handle.run_id;

    if (interrupted) {
      await daemonRequest('v1.workflow.cancel_run', { run_id: runId, reason: 'Interrupted from Custos CLI' });
    }

    let cursor = 0;
    while (true) {
      const replay = await daemonRequest('v1.workflow.live_run', {
        session_id: session.id,
        run_id: runId,
        cursor,
      });
      cursor = replay.snapshot?.cursor || cursor;
      const status = replay.snapshot?.status || 'running';
      if (['completed', 'failed', 'cancelled', 'interrupted'].includes(status)) {
        if (status !== 'completed') {
          throw new Error(replay.snapshot?.error || `Agent run ${status}.`);
        }
        return {
          text: replay.snapshot?.text || 'Agent hoàn tất nhưng không trả về nội dung.',
          usage: replay.snapshot?.usage,
          taskId: task.id,
          runId,
        };
      }
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
  } finally {
    clearInterval(animation);
    inputDevice.removeListener('keypress', onExecutionKeypress);
    outputDevice.write('\x1b[?25h');
  }
}

async function createTaskInteractive(currentMode) {
  const title = await input({
    message: chalk.bold.white('Nhập tiêu đề / mục tiêu nhiệm vụ mới:'),
    default: 'New sovereign agentic workflow',
  });
  if (!title || !title.trim()) return;
  const modeName = currentMode !== 'custos' ? (currentMode.charAt(0).toUpperCase() + currentMode.slice(1)) : 'Code';
  try {
    const task = await daemonRequest('v1.tasks.create', {
      title: title.trim(),
      metadata: {
        mode: modeName.toLowerCase(),
        origin: 'custos-cli',
      },
    });
    console.log(chalk.green(`\n  ✔ Đã tạo task mới [${task.id}] (${modeName}): "${task.title}"\n`));
  } catch (err) {
    console.log(chalk.red(`\n  ✖ Lỗi khi tạo task: ${err.message}\n`));
  }
  console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
  await waitForKeypress();
}

async function handleSlashAction(action, state) {
  switch (action) {
    case 'model':
    case 'login':
      state.modelSelection = await promptProviderModelSelection(state.modelSelection);
      return;
    case 'logout': {
      const result = await promptProviderLogout(state.modelSelection);
      state.modelSelection = result.selection;
      if (result.provider) {
        state.chatTurns.push({
          query: '/logout',
          mode: state.currentMode,
          responseLines: [`Đã đăng xuất khỏi ${result.provider.name}.`],
          meta: 'Custos · provider disconnected',
        });
      }
      return;
    }
    case 'vibe':
      await runVibeWizard(state.currentMode !== 'custos' ? state.currentMode : null);
      break;
    case 'list':
      await printTaskList();
      break;
    case 'create':
      await createTaskInteractive(state.currentMode);
      return;
    case 'advance':
      await advanceTask();
      break;
    case 'diff':
      console.log();
      printUnifiedDiff();
      break;
    case 'permit':
      await promptPermitApproval();
      break;
    case 'status':
      await printStatus();
      break;
    case 'clear':
      return;
    case 'help':
      printHelp();
      break;
    case 'exit':
      shutdownCli(0);
      return;
    default:
      return;
  }
  console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
  await waitForKeypress();
}

export async function startRepl() {
  enterAlternateScreen();
  emitKeypressEvents(inputDevice);

  const history = [];
  const state = {
    currentMode: 'custos',
    chatTurns: [],
    modelSelection: null,
  };

  while (true) {
    const res = await promptInteractiveChat({
      currentMode: state.currentMode,
      history,
      chatTurns: state.chatTurns,
      selectedProvider: state.modelSelection?.provider,
      selectedModel: state.modelSelection?.model,
    });

    if (res.type === 'tab') {
      try {
        const chosen = await promptModeOnly(state.currentMode, state.modelSelection);
        state.currentMode = chosen.mode;
        state.modelSelection = chosen.selection;
      } catch (err) {
        if (err.name !== 'ExitPromptError') {
          state.chatTurns.push({ query: 'Provider setup', mode: state.currentMode, responseLines: [err.message], meta: 'Custos · lỗi kết nối' });
        }
      }
      continue;
    }

    if (res.type === 'slash') {
      try {
        const chosen = await promptSlashSelection();
        if (chosen?.type === 'submenu' && chosen.value === 'mode') {
          const result = await promptModeOnly(state.currentMode, state.modelSelection);
          state.currentMode = result.mode;
          state.modelSelection = result.selection;
        } else if (chosen?.type === 'mode') {
          state.currentMode = chosen.value;
        } else if (chosen?.type === 'action') {
          await handleSlashAction(chosen.value, state);
        }
      } catch (err) {
        if (err.name !== 'ExitPromptError') {
          state.chatTurns.push({
            query: 'Provider setup',
            mode: state.currentMode,
            responseLines: [err.message],
            meta: 'Custos local · backend chưa sẵn sàng',
          });
        }
      }
      continue;
    }

    const line = (res.value || '').trim();
    if (!line) continue;
    history.push(line);

    const [cmd, ...args] = line.split(/\s+/);
    const argStr = args.join(' ');
    const lowerCmd = cmd.toLowerCase();
    const cleanCmd = lowerCmd.startsWith('/') ? lowerCmd.slice(1) : lowerCmd;

    if (cleanCmd === 'exit' || cleanCmd === 'quit' || cleanCmd === 'q') {
      shutdownCli(0);
    } else if (cleanCmd === 'clear' || cleanCmd === 'cls') {
      state.chatTurns.length = 0;
    } else if (lowerCmd === '/' || lowerCmd === '/mode' || (lowerCmd === 'mode' && !args[0])) {
      try {
        const chosen = await promptModeOnly(state.currentMode, state.modelSelection);
        state.currentMode = chosen.mode;
        state.modelSelection = chosen.selection;
      } catch (err) {
        if (err.name !== 'ExitPromptError') {
          state.chatTurns.push({ query: '/mode', mode: state.currentMode, responseLines: [err.message], meta: 'Custos · lỗi kết nối' });
        }
      }
    } else if (cleanCmd === 'model' || cleanCmd === 'login') {
      try {
        state.modelSelection = await promptProviderModelSelection(state.modelSelection);
      } catch (err) {
        state.chatTurns.push({ query: line, mode: state.currentMode, responseLines: [err.message], meta: 'Custos · lỗi kết nối' });
      }
    } else if (cleanCmd === 'logout') {
      try {
        const result = await promptProviderLogout(state.modelSelection);
        state.modelSelection = result.selection;
        if (result.provider) {
          state.chatTurns.push({
            query: line,
            mode: state.currentMode,
            responseLines: [`Đã đăng xuất khỏi ${result.provider.name}.`],
            meta: 'Custos · provider disconnected',
          });
        }
      } catch (err) {
        state.chatTurns.push({ query: line, mode: state.currentMode, responseLines: [err.message], meta: 'Custos · lỗi đăng xuất' });
      }
    } else if (cleanCmd === 'code' || cleanCmd === 'custos-code' || (cleanCmd === 'mode' && args[0]?.toLowerCase() === 'code')) {
      state.currentMode = 'code';
    } else if (cleanCmd === 'research' || cleanCmd === 'custos-research' || (cleanCmd === 'mode' && args[0]?.toLowerCase() === 'research')) {
      state.currentMode = 'research';
    } else if (cleanCmd === 'assistant' || cleanCmd === 'custos-assistant' || (cleanCmd === 'mode' && args[0]?.toLowerCase() === 'assistant')) {
      state.currentMode = 'assistant';
    } else if (cleanCmd === 'custos' || cleanCmd === 'exit-mode') {
      state.currentMode = 'custos';
    } else if (cleanCmd === 'vibe' || cleanCmd === 'v') {
      await runVibeWizard(state.currentMode !== 'custos' ? state.currentMode : null);
      console.log(chalk.dim('\n  Nhấn phím bất kỳ để tiếp tục...'));
      await waitForKeypress();
    } else if (cleanCmd === 'list' || cleanCmd === 'tasks' || cleanCmd === 'ls') {
      await printTaskList();
      console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
      await waitForKeypress();
    } else if (cleanCmd === 'create' || cleanCmd === 'new') {
      const title = argStr || 'New sovereign agentic workflow';
      const modeName = state.currentMode !== 'custos' ? (state.currentMode.charAt(0).toUpperCase() + state.currentMode.slice(1)) : 'Code';
      try {
        const task = await daemonRequest('v1.tasks.create', {
          title: title.trim(),
          metadata: { mode: modeName.toLowerCase(), origin: 'custos-cli' },
        });
        console.log(chalk.green(`  ✔ Đã tạo task mới [${task.id}] (${modeName}): "${task.title}"\n`));
      } catch (err) {
        console.log(chalk.red(`  ✖ Lỗi khi tạo task: ${err.message}\n`));
      }
      console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
      await waitForKeypress();
    } else if (cleanCmd === 'advance') {
      await advanceTask(args[0]);
      console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
      await waitForKeypress();
    } else if (cleanCmd === 'diff' || cleanCmd === 'd') {
      printUnifiedDiff();
      console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
      await waitForKeypress();
    } else if (cleanCmd === 'permit' || cleanCmd === 'p') {
      await promptPermitApproval();
      console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
      await waitForKeypress();
    } else if (cleanCmd === 'status') {
      await printStatus();
      console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
      await waitForKeypress();
    } else if (cleanCmd === 'help' || cleanCmd === '?') {
      printHelp();
      console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
      await waitForKeypress();
    } else if (state.currentMode !== 'custos' || state.modelSelection) {
      const agentName = getAgentTheme(state.currentMode).name;
      try {
        if (!state.modelSelection) {
          state.modelSelection = await promptProviderModelSelection();
        }
        if (!state.modelSelection) continue;
        const result = await executeModelTurn({
          prompt: line,
          mode: state.currentMode,
          selection: state.modelSelection,
          chatTurns: state.chatTurns,
        });
        const totalTokens = result.usage?.total_tokens;
        state.chatTurns.push({
          query: line,
          mode: state.currentMode,
          responseLines: result.text.split(/\r?\n/),
          meta: `${agentName} · ${state.modelSelection.model.name || state.modelSelection.model.id} · ${state.modelSelection.provider.name}${totalTokens ? ` · ${totalTokens} tokens` : ''}`,
        });
      } catch (error) {
        state.chatTurns.push({
          query: line,
          mode: state.currentMode,
          responseLines: [`Không thể chạy agent: ${error.message}`],
          meta: `${agentName} · lỗi thực thi`,
        });
      }
    } else {
      const lower = line.toLowerCase().trim();
      const responseLines = ['hi', 'hello', 'chào', 'hih', 'hey'].includes(lower)
        ? [
            'Xin chào! Custos Sovereign Agentic Runtime đã sẵn sàng.',
            'Bạn có thể trò chuyện tự do hoặc nhấn Tab để chọn agent chuyên biệt.',
          ]
        : [
            'Đây là chat nhẹ chạy cục bộ của Custos; chưa có model cloud nào được gọi.',
            'Dùng /mode để chọn chuyên môn, /login để kết nối provider và /model để chọn model.',
          ];
      state.chatTurns.push({
        query: line,
        mode: 'custos',
        responseLines,
        meta: 'Custos local · lightweight chat',
      });
    }
  }

  leaveAlternateScreen();
}

