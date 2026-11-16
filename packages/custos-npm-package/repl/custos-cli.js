#!/usr/bin/env node

import chalk from 'chalk';
import { stdin as inputDevice, stdout as outputDevice } from 'node:process';
import { VERSION, daemonRequest } from './lib/daemon.js';
import {
  printMainBanner,
  printModeCard,
  promptSlashSelection,
  promptModeSelection,
  printUserMessageCard,
  printUnknownCommand,
  leaveAlternateScreen,
} from './lib/terminal_ui.js';
import { modelDisplayName } from './lib/providers.js';
import {
  printUnifiedDiff,
  printTaskList,
  printStatus,
  advanceTask,
  promptPermitApproval,
  runVibeWizard,
  printHelp,
} from './lib/commands.js';
import { startRepl } from './lib/repl.js';

export {
  printMainBanner,
  printModeCard,
  promptSlashSelection,
  promptModeSelection,
  modelDisplayName,
  printUnifiedDiff,
  printTaskList,
  printStatus,
  advanceTask,
  promptPermitApproval,
  runVibeWizard,
  printUserMessageCard,
  startRepl,
  printHelp,
};

async function main() {
  const args = process.argv.slice(2);
  const first = args[0]?.toLowerCase();

  if (!first) {
    if (!inputDevice.isTTY || !outputDevice.isTTY) {
      printHelp();
      return;
    }
    await startRepl();
    return;
  }

  if (first === '--version' || first === '-v') {
    console.log(VERSION);
    return;
  }

  if (first === 'vibe' || first === 'v') {
    await runVibeWizard();
  } else if (first === 'code') {
    printModeCard('Code');
  } else if (first === 'research') {
    printModeCard('Research');
  } else if (first === 'assistant') {
    printModeCard('Assistant');
  } else if (first === 'list' || first === 'tasks' || first === 'ls') {
    await printTaskList();
  } else if (first === 'create' || first === 'new') {
    const title = args.slice(1).join(' ') || 'New sovereign agentic workflow';
    try {
      const task = await daemonRequest('v1.tasks.create', {
        title: title.trim(),
        metadata: {
          mode: 'code',
          origin: 'custos-cli',
        },
      });
      console.log(chalk.green(`\n  ✔ Đã tạo task mới [${task.id}]: "${task.title}"\n`));
    } catch (err) {
      console.log(chalk.red(`\n  ✖ Lỗi khi tạo task: ${err.message}\n`));
    }
  } else if (first === 'advance') {
    await advanceTask(args[1]);
  } else if (first === 'diff' || first === 'd') {
    console.log();
    printUnifiedDiff();
  } else if (first === 'permit' || first === 'p') {
    await promptPermitApproval();
  } else if (first === 'status') {
    await printStatus();
  } else if (first === '--help' || first === '-h' || first === 'help' || first === '?') {
    printHelp();
  } else {
    printUnknownCommand(first);
    process.exitCode = 1;
  }
}

main().catch((err) => {
  leaveAlternateScreen();
  if (err.name === 'ExitPromptError') {
    console.log(chalk.dim('\n  Đã hủy thao tác.\n'));
    process.exit(0);
  }
  console.error(err);
  process.exit(1);
});
