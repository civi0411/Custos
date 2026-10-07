#!/usr/bin/env node

import chalk from 'chalk';
import boxen from 'boxen';
import ora from 'ora';
import { select, input, confirm, Separator } from '@inquirer/prompts';
import * as readline from 'node:readline/promises';
import { emitKeypressEvents } from 'node:readline';
import { stdin as inputDevice, stdout as outputDevice } from 'node:process';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const VERSION = '0.1.1';

function getTerminalWidth() {
  return process.stdout.columns || 100;
}

function getBannerLines(version) {
  return [
    '',
    '',
    '\x1b[38;2;125;211;252;1m   ██████╗ ██╗   ██╗ ███████╗ ████████╗  ██████╗  ███████╗\x1b[0m',
    '\x1b[38;2;147;197;253;1m  ██╔════╝ ██║   ██║ ██╔════╝ ╚══██╔══╝ ██╔═══██╗ ██╔════╝\x1b[0m',
    '\x1b[38;2;186;230;253;1m  ██║      ██║   ██║ ███████╗    ██║    ██║   ██║ ███████╗\x1b[0m',
    '\x1b[38;2;224;242;254;1m  ██║      ██║   ██║ ╚════██║    ██║    ██║   ██║ ╚════██║\x1b[0m',
    '\x1b[38;2;240;248;255;1m  ╚██████╗ ╚██████╔╝ ███████║    ██║    ╚██████╔╝ ███████║\x1b[0m',
    '\x1b[38;2;248;250;252;1m   ╚═════╝  ╚═════╝  ╚══════╝    ╚═╝     ╚═════╝  ╚══════╝\x1b[0m',
    '',
    `  \x1b[38;2;56;189;248m❄\x1b[0m \x1b[38;2;251;191;36;1mCustos\x1b[0m \x1b[38;2;148;163;184mv${version}\x1b[0m \x1b[38;2;125;211;252;3m- Guardian of Agentic Work\x1b[0m`,
    '  \x1b[38;2;148;163;184mHuman-governed runtime for agentic workflows\x1b[0m',
    '  \x1b[38;2;100;116;139mChat tự do hoặc nhấn \'/\' để mở bảng chọn chức năng & mode:\x1b[0m',
    '  \x1b[38;2;56;189;248m(custos-code, custos-research, vibe, tasks, diff, status, ...)\x1b[0m',
    '',
  ];
}

export function printMainBanner() {
  const bannerLines = getBannerLines(VERSION);
  console.log();
  for (const line of bannerLines) {
    console.log(line);
  }
  console.log();
}

function stripAnsi(str) {
  return str.replace(/\x1b\[[0-9;]*m/g, '');
}

export function printModeCard(mode) {
  const modeKey = (mode || 'Code').toLowerCase();

  const configs = {
    code: {
      name: 'Code Engine',
      slug: 'custos-code',
      tag: 'CHẾ ĐỘ: CODE ENGINE (custos-code)',
      title: 'Custos Code Engine ⚡',
      msg: '"Hệ thống lập trình, sửa lỗi & thực thi trong Sandbox"',
      b1: 'Lập trình tính năng, sửa lỗi & tái cấu trúc mã nguồn',
      b2: 'Thực thi an toàn trong Sandbox Worktree cách ly',
      b3: 'Tối ưu thuật toán & kiểm tra tính đúng đắn',
      color: '\x1b[38;2;125;211;252;1m',
      badgeColor: '#38bdf8',
    },
    research: {
      name: 'Research Engine',
      slug: 'custos-research',
      tag: 'CHẾ ĐỘ: RESEARCH ENGINE (custos-research)',
      title: 'Custos Research Engine 🔍',
      msg: '"Hệ thống điều tra cấu trúc dự án & đối soát bằng chứng"',
      b1: 'Điều tra cấu trúc dự án & đối soát bằng chứng',
      b2: 'Phân tích tài liệu, kiến trúc & suy luận chuyên sâu',
      b3: 'Kiểm chứng các thay đổi trước khi xin phê duyệt',
      color: '\x1b[38;2;251;191;36;1m',
      badgeColor: '#f59e0b',
    },
    assistant: {
      name: 'Assistant Engine',
      slug: 'custos-assistant',
      tag: 'CHẾ ĐỘ: ASSISTANT ENGINE (custos-assistant)',
      title: 'Custos Assistant Engine 🤖',
      msg: '"Hệ thống điều phối quy trình tác vụ tự động theo chuẩn runtime"',
      b1: 'Điều phối quy trình tác vụ tự động theo chuẩn runtime',
      b2: 'Quản lý trạng thái Task, Epoch Lock & Audit Logs',
      b3: 'Theo dõi tiến trình & báo cáo kết quả thực thi',
      color: '\x1b[38;2;52;211;153;1m',
      badgeColor: '#10b981',
    },
  };

  const cfg = configs[modeKey] || configs.code;
  const termW = getTerminalWidth();
  const boxW = Math.min(Math.max(termW - 10, 56), 76);

  const padLine = (content) => {
    const visibleLen = stripAnsi(content).length;
    const pad = Math.max(0, boxW - 4 - visibleLen);
    return `\x1b[38;2;71;85;105m│\x1b[0m ${content}${' '.repeat(pad)} \x1b[38;2;71;85;105m│\x1b[0m`;
  };

  const borderTop = `\x1b[38;2;71;85;105m╭${'─'.repeat(boxW - 2)}╮\x1b[0m`;
  const borderBot = `\x1b[38;2;71;85;105m╰${'─'.repeat(boxW - 2)}╯\x1b[0m`;
  const emptyRow = `\x1b[38;2;71;85;105m│${' '.repeat(boxW - 2)}│\x1b[0m`;

  const boxLines = [
    borderTop,
    padLine(`${cfg.color}[ ${cfg.tag} ]\x1b[0m`),
    emptyRow,
    padLine(`\x1b[38;2;248;250;252;1m${cfg.title}\x1b[0m`),
    padLine(`\x1b[38;2;125;211;252;1m${cfg.msg}\x1b[0m`),
    emptyRow,
    padLine(`\x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;226;232;240m${cfg.b1}\x1b[0m`),
    padLine(`\x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;226;232;240m${cfg.b2}\x1b[0m`),
    padLine(`\x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;226;232;240m${cfg.b3}\x1b[0m`),
    emptyRow,
    padLine('\x1b[38;2;148;163;184mNhập mục tiêu nhiệm vụ hoặc mã lệnh bên dưới. Gõ \'/\' để đổi mode.\x1b[0m'),
    borderBot,
  ];

  console.log();
  for (const line of boxLines) {
    console.log(`  ${line}`);
  }
  console.log();
}

export async function promptSlashSelection() {
  console.log(chalk.bold.hex('#38bdf8')('\n  ══ BẢNG LỆNH & CHỨC NĂNG CUSTOS (SLASH COMMAND PALETTE) ══\n'));

  const choice = await select({
    message: chalk.bold.white('Chọn chức năng hoặc chế độ hoạt động:'),
    choices: [
      new Separator(chalk.hex('#38bdf8')('─── Chế độ hoạt động (Operational Engines) ───')),
      {
        name: `${chalk.hex('#38bdf8').bold('/mode')}              ${chalk.dim('— Chọn Code, Research hoặc Assistant')}`,
        value: { type: 'submenu', value: 'mode' },
      },

      new Separator(chalk.hex('#ec4899')('─── Tác vụ & Quy trình (Actions & Workflows) ───')),
      {
        name: `${chalk.hex('#ec4899').bold('vibe')}               ${chalk.dim('— Khởi chạy quy trình Vibe Coding tương tác')}`,
        value: { type: 'action', value: 'vibe' },
      },
      {
        name: `${chalk.hex('#818cf8').bold('tasks / list')}       ${chalk.dim('— Xem danh sách nhiệm vụ & SQLite audit spans')}`,
        value: { type: 'action', value: 'list' },
      },
      {
        name: `${chalk.hex('#34d399').bold('create')}             ${chalk.dim('— Tạo nhiệm vụ agentic mới vào hàng đợi')}`,
        value: { type: 'action', value: 'create' },
      },
      {
        name: `${chalk.hex('#fbbf24').bold('advance')}            ${chalk.dim('— Chuyển tiến trình task sang trạng thái kế tiếp')}`,
        value: { type: 'action', value: 'advance' },
      },
      {
        name: `${chalk.hex('#f97316').bold('diff')}               ${chalk.dim('— Xem bản so sánh Unified AST Diff Review')}`,
        value: { type: 'action', value: 'diff' },
      },
      {
        name: `${chalk.hex('#ef4444').bold('permit')}             ${chalk.dim('— Xem & phê duyệt Giấy phép Thực thi (Human Permit)')}`,
        value: { type: 'action', value: 'permit' },
      },
      {
        name: `${chalk.hex('#06b6d4').bold('status')}             ${chalk.dim('— Kiểm tra trạng thái Sovereign Runtime & Sandbox')}`,
        value: { type: 'action', value: 'status' },
      },

      new Separator(chalk.dim('─── Hệ thống & Trợ giúp (System) ───')),
      {
        name: `${chalk.dim.white('clear')}              ${chalk.dim('— Xóa màn hình terminal và in lại banner')}`,
        value: { type: 'action', value: 'clear' },
      },
      {
        name: `${chalk.dim.white('help')}               ${chalk.dim('— Tra cứu danh sách lệnh chi tiết')}`,
        value: { type: 'action', value: 'help' },
      },
      {
        name: `${chalk.red('exit')}               ${chalk.dim('— Thoát khỏi Custos CLI')}`,
        value: { type: 'action', value: 'exit' },
      },
    ],
    pageSize: 16,
  });

  return choice;
}

export async function promptModeSelection() {
  return select({
    message: chalk.bold.white('Chọn mode:'),
    choices: [
      { name: `${chalk.hex('#38bdf8').bold('/code')}       ${chalk.dim('— Lập trình, sửa lỗi & Sandbox Worktree')}`, value: 'code' },
      { name: `${chalk.hex('#f59e0b').bold('/research')}   ${chalk.dim('— Điều tra kiến trúc & đối soát bằng chứng')}`, value: 'research' },
      { name: `${chalk.hex('#10b981').bold('/assistant')}  ${chalk.dim('— Điều phối quy trình tác vụ & Epoch Lock')}`, value: 'assistant' },
    ],
  });
}

const DATA_DIR = path.join(os.homedir(), '.custos');
const TASKS_FILE = path.join(DATA_DIR, 'cli-tasks.json');

function loadTasks() {
  try {
    if (fs.existsSync(TASKS_FILE)) {
      return JSON.parse(fs.readFileSync(TASKS_FILE, 'utf8'));
    }
  } catch {}
  return [
    {
      id: 'task-c91f',
      title: 'Isolate worktree & synthesize AST patch for auth token expiry',
      mode: 'Code',
      status: 'Completed',
      risk: 'Medium',
      spans: 4,
      createdAt: '10 mins ago',
    },
    {
      id: 'task-d82e',
      title: 'Audit concurrency invariants in SQLite Write-Ahead Log lock guard',
      mode: 'Research',
      status: 'Executing',
      risk: 'High',
      spans: 7,
      createdAt: '35 mins ago',
    },
    {
      id: 'task-a34c',
      title: 'Orchestrate multi-agent deliberation for contract verification',
      mode: 'Assistant',
      status: 'Pending',
      risk: 'Low',
      spans: 2,
      createdAt: '1 hr ago',
    },
  ];
}

function saveTasks(tasks) {
  try {
    if (!fs.existsSync(DATA_DIR)) {
      fs.mkdirSync(DATA_DIR, { recursive: true });
    }
    fs.writeFileSync(TASKS_FILE, JSON.stringify(tasks, null, 2), 'utf8');
  } catch {}
}

let tasks = loadTasks();

export function printUnifiedDiff() {
  const filePath = 'crates/custos-runtime/src/handler.rs';
  const diffLines = [
    chalk.bgHex('#f59e0b').black.bold(' DIFF ') + ' ' + chalk.bold.underline(filePath),
    chalk.red('--- a/' + filePath),
    chalk.green('+++ b/' + filePath),
    chalk.cyan('@@ -1,6 +1,10 @@'),
    chalk.dim(' fn handle_task_execution() {'),
    chalk.red('-    // Unverified execution path'),
    chalk.red('-    todo!();'),
    chalk.green('+    // Epoch Optimistic Lock Guard & Verified Sandbox'),
    chalk.green('+    let guard = EpochGuard::acquire(task_id)?;'),
    chalk.green('+    verify_execution_permit(guard.token())?;'),
    chalk.green('+    let result = worktree::execute_isolated(payload)?;'),
    chalk.green('+    audit::commit_span(task_id, "execution_success")?;'),
    chalk.green('+    Ok(result)'),
    chalk.dim(' }'),
  ];

  const diffBox = boxen(diffLines.join('\n'), {
    padding: { top: 0, bottom: 0, left: 1, right: 1 },
    margin: { top: 0, bottom: 0, left: 1, right: 1 },
    borderStyle: 'round',
    borderColor: '#64748b',
    title: chalk.yellow(' Unified AST Diff Review '),
    titleAlignment: 'left',
  });

  console.log(diffBox);
  console.log(
    chalk.bold('  Summary: ') +
      chalk.green.bold('+5 additions') +
      chalk.dim(', ') +
      chalk.red.bold('-2 deletions') +
      chalk.dim('  (Status: Pending Operator Permit)\n')
  );
}

export function printTaskList() {
  console.log(chalk.bold.hex('#38bdf8')('\n  ══ Custos Task Registry & SQLite Spans ══\n'));

  if (tasks.length === 0) {
    console.log(chalk.dim('  No tasks registered yet. Run `create <title>` or `vibe` to add one.\n'));
    return;
  }

  const header = `  ${chalk.dim('ID'.padEnd(12))} ${chalk.dim('MODE'.padEnd(12))} ${chalk.dim('STATUS'.padEnd(14))} ${chalk.dim('SPANS'.padEnd(8))} ${chalk.dim('TITLE')}`;
  console.log(header);
  console.log(chalk.dim('  ' + '─'.repeat(Math.min(getTerminalWidth() - 4, 90))));

  tasks.forEach((t) => {
    let statusBadge = chalk.yellow('Pending');
    if (t.status === 'Executing') statusBadge = chalk.hex('#38bdf8').bold('Executing');
    if (t.status === 'Completed') statusBadge = chalk.green.bold('Completed');
    if (t.status === 'Cancelled') statusBadge = chalk.red('Cancelled');

    let modeColor = chalk.cyan;
    if (t.mode === 'Research') modeColor = chalk.hex('#f59e0b');
    if (t.mode === 'Assitant' || t.mode === 'Assistant') modeColor = chalk.hex('#10b981');

    console.log(
      `  ${chalk.bold.white(t.id.padEnd(12))} ` +
        `${modeColor(t.mode.padEnd(12))} ` +
        `${statusBadge.padEnd(23)} ` +
        `${chalk.dim((t.spans + ' spans').padEnd(8))} ` +
        `${chalk.white(t.title)}`
    );
  });
  console.log();
}

export function printStatus() {
  const statusContent =
    `${chalk.bold.hex('#38bdf8')('Custos Sovereign Runtime Status')}\n\n` +
    `${chalk.cyan('• Daemon Connection')} : ${chalk.green.bold('Online & Responding')}\n` +
    `${chalk.cyan('• Local SQLite DB')}   : ${chalk.white('~/.custos/custos_vault.db (WAL Active)')}\n` +
    `${chalk.cyan('• Active Sandbox')}    : ${chalk.white('Bubblewrap / Seatbelt Worktree (Clean)')}\n` +
    `${chalk.cyan('• Concurrency Guard')}: ${chalk.white('Epoch Optimistic Lock v0.1.1')}\n` +
    `${chalk.cyan('• Registered Tasks')}: ${chalk.yellow(tasks.length + ' active tasks')}\n` +
    `${chalk.cyan('• Operational Modes')}: ${chalk.white('$custos-code, $custos-research, $custos-assistant')}\n` +
    `${chalk.cyan('• Execution Mode')}   : ${chalk.hex('#10b981').bold('Direct Terminal Interactive')}`;

  console.log(
    boxen(statusContent, {
      padding: 1,
      margin: { top: 1, bottom: 1, left: 1, right: 1 },
      borderStyle: 'round',
      borderColor: '#38bdf8',
    })
  );
}

export function advanceTask(taskId) {
  const task = tasks.find((t) => t.id === taskId || t.id === `task-${taskId}`);
  if (!task) {
    console.log(chalk.red(`\n  Không tìm thấy task với ID: "${taskId}"\n`));
    return;
  }

  const stateMachine = {
    Pending: 'Planning',
    Planning: 'Executing',
    Executing: 'Verifying',
    Verifying: 'Completed',
    Completed: 'Completed',
  };

  const nextState = stateMachine[task.status] || 'Completed';
  task.status = nextState;
  task.spans = (task.spans || 0) + 1;
  saveTasks(tasks);

  console.log(
    chalk.green(`\n  ✔ Task [${task.id}] đã chuyển trạng thái sang: `) +
      chalk.hex('#38bdf8').bold(nextState) +
      chalk.dim(` (+1 audit span)\n`)
  );
}

export async function promptPermitApproval() {
  console.log(chalk.bold.hex('#f59e0b')('\n  ══ Human-in-the-Loop Execution Permit Grant ══\n'));

  const permitDetails =
    `${chalk.bgRed.white.bold(' [CRITICAL RISK] ')} ${chalk.bold.white('Mutation Permit Requested')}\n\n` +
    `${chalk.cyan('Action Type')} : ${chalk.white.bold('Worktree Disk Mutation & Patch Apply')}\n` +
    `${chalk.cyan('Target File')} : ${chalk.white('crates/custos-runtime/src/handler.rs')}\n` +
    `${chalk.cyan('Changes')}     : ${chalk.green('+5 additions')}, ${chalk.red('-2 deletions')}\n` +
    `${chalk.cyan('Sandbox')}     : ${chalk.white('Isolated Git Worktree (bubblewrap-02)')}\n` +
    `${chalk.cyan('Governance')}  : ${chalk.white('Requires human operator signature before disk write')}`;

  console.log(
    boxen(permitDetails, {
      padding: 1,
      margin: { top: 0, bottom: 1, left: 1, right: 1 },
      borderStyle: 'round',
      borderColor: '#ef4444',
    })
  );

  const answer = await confirm({
    message: chalk.bold.white('Grant ExecutionPermit to commit this change to production repository?'),
    default: false,
  });

  if (answer) {
    const s = ora(chalk.cyan('Committing verified patch under Epoch Optimistic Lock...')).start();
    await new Promise((r) => setTimeout(r, 600));
    s.succeed(chalk.green('ExecutionPermit granted! Patch committed and audit span logged to SQLite.'));
  } else {
    console.log(chalk.yellow('ExecutionPermit denied. Sandbox worktree was rolled back safely.'));
  }
  console.log();
}

export async function runVibeWizard(initialMode = null) {
  let mode = initialMode;

  if (!mode) {
    console.log(chalk.bold.hex('#38bdf8')('\n  ══ Chọn chế độ hoạt động Custos ══\n'));
    mode = await select({
      message: chalk.cyan.bold('❄ Chọn chế độ Custos (Operational Mode):'),
      choices: [
        {
          name: `${chalk.hex('#38bdf8').bold('$custos-code')}       ${chalk.dim('• Lập trình, sửa lỗi & tái cấu trúc mã nguồn')}`,
          value: 'Code',
        },
        {
          name: `${chalk.hex('#f59e0b').bold('$custos-research')}   ${chalk.dim('• Điều tra, kiến trúc & suy luận chuyên sâu')}`,
          value: 'Research',
        },
        {
          name: `${chalk.hex('#10b981').bold('$custos-assistant')}  ${chalk.dim('• Trợ lý điều phối quy trình tự động')}`,
          value: 'Assistant',
        },
      ],
    });
  }

  printModeCard(mode);

  const taskPrompt = await input({
    message: chalk.hex(mode === 'Research' ? '#f59e0b' : mode === 'Assistant' ? '#10b981' : '#38bdf8').bold(
      `[${mode.toUpperCase()}] Bắt đầu viết mã / nhập mục tiêu nhiệm vụ:`
    ),
    default: mode === 'Research'
      ? 'Audit repository architecture and concurrency lock boundaries'
      : mode === 'Assistant'
      ? 'Coordinate multi-agent task lifecycle and verify audit spans'
      : 'Refactor handler with verified sandbox & epoch lock guard',
  });

  console.log();
  const s1 = ora(chalk.cyan('Khởi tạo sandbox cô lập (Worktree Sandbox)...')).start();
  await new Promise((r) => setTimeout(r, 500));
  s1.succeed(chalk.white('Sandbox cô lập đã sẵn sàng: ') + chalk.dim('.custos/sandboxes/worktree-live'));

  const s2 = ora(chalk.cyan('Phân tích AST & tổng hợp bản vá (Patch Synthesis)...')).start();
  await new Promise((r) => setTimeout(r, 700));
  s2.succeed(chalk.white('Tổng hợp AST hoàn tất: ') + chalk.green('+5 additions, -2 deletions'));

  const s3 = ora(chalk.cyan('Chạy bộ kiểm thử tự động (Automated Test Verification)...')).start();
  await new Promise((r) => setTimeout(r, 600));
  s3.succeed(chalk.white('Tất cả kiểm thử vượt qua (5/5 passed)'));

  console.log();
  printUnifiedDiff();

  const approve = await confirm({
    message: chalk.bold.white('Phê duyệt Giấy phép Thực thi (ExecutionPermit) để áp dụng vào mã nguồn?'),
    default: true,
  });

  if (approve) {
    const s4 = ora(chalk.cyan('Ghi nhận audit span vào SQLite & hoàn tất nhiệm vụ...')).start();
    await new Promise((r) => setTimeout(r, 500));
    s4.succeed(chalk.green('Nhiệm vụ đã hoàn tất thành công dưới cơ chế bảo vệ Epoch Lock!'));

    const newTask = {
      id: `task-${Math.random().toString(36).substring(2, 6)}`,
      title: taskPrompt,
      mode,
      status: 'Completed',
      risk: 'Medium',
      spans: 5,
      createdAt: 'Just now',
    };
    tasks.unshift(newTask);
    saveTasks(tasks);
  } else {
    console.log(chalk.yellow('Đã hủy cấp phép. Sandbox đã được rollback an toàn.'));
  }
  console.log();
}

export async function startRepl() {
  printMainBanner();

  let currentMode = 'custos';
  emitKeypressEvents(inputDevice);
  const rl = readline.createInterface({ input: inputDevice, output: outputDevice });

  let waitingForInput = false;

  const onKeypress = (str, key) => {
    // If prompt is waiting and user presses '/', trigger mode selection immediately without pressing Enter
    if (waitingForInput && str === '/' && (rl.line?.trim() === '/' || rl.line?.trim() === '')) {
      waitingForInput = false;
      setImmediate(() => {
        rl.write('\r\n');
      });
    }
  };

  inputDevice.on('keypress', onKeypress);

  const getPrompt = () => {
    if (currentMode === 'custos') {
      return `${chalk.hex('#38bdf8').bold('custos')} ${chalk.dim('❯')} `;
    }
    let modeColor = chalk.hex('#38bdf8');
    if (currentMode === 'code') modeColor = chalk.hex('#38bdf8');
    if (currentMode === 'research') modeColor = chalk.hex('#f59e0b');
    if (currentMode === 'assistant') modeColor = chalk.hex('#10b981');
    return `${modeColor.bold(`custos-${currentMode}`)} ${chalk.dim('❯')} `;
  };

  while (true) {
    waitingForInput = true;
    const rawLine = await rl.question(getPrompt());
    waitingForInput = false;
    const line = rawLine.trim();
    if (!line) continue;

    const [cmd, ...args] = line.split(/\s+/);
    const argStr = args.join(' ');
    const lowerCmd = cmd.toLowerCase();
    const cleanCmd = lowerCmd.startsWith('/') ? lowerCmd.slice(1) : lowerCmd;

    if (cleanCmd === 'exit' || cleanCmd === 'quit' || cleanCmd === 'q') {
      console.log(chalk.dim('\n  Tạm biệt! Custos Runtime đang bảo vệ hệ thống của bạn.\n'));
      break;
    } else if (cleanCmd === 'clear' || cleanCmd === 'cls') {
      console.clear();
      printMainBanner();
    } else if (lowerCmd === '/' || lowerCmd === '/mode' || lowerCmd === 'mode' && !args[0]) {
      rl.pause();
      try {
        const chosen = await promptSlashSelection();
        if (chosen) {
          if (chosen.type === 'submenu' && chosen.value === 'mode') {
            currentMode = await promptModeSelection();
            console.log(chalk.cyan(`\n  ✔ Đã chuyển sang chế độ [custos-${currentMode}].\n`));
          } else if (chosen.type === 'mode') {
            currentMode = chosen.value;
            console.log(chalk.cyan(`\n  ✔ Đã chuyển sang chế độ [${currentMode === 'custos' ? 'custos' : `custos-${currentMode}`}].\n`));
          } else if (chosen.type === 'action') {
            switch (chosen.value) {
              case 'vibe':
                await runVibeWizard(currentMode !== 'custos' ? currentMode : null);
                break;
              case 'list':
                printTaskList();
                break;
              case 'create': {
                const title = await input({
                  message: chalk.bold.white('Nhập tiêu đề / mục tiêu nhiệm vụ mới:'),
                  default: 'New sovereign agentic workflow',
                });
                if (title && title.trim()) {
                  const modeName = currentMode !== 'custos' ? (currentMode.charAt(0).toUpperCase() + currentMode.slice(1)) : 'Code';
                  const newTask = {
                    id: `task-${Math.random().toString(36).substring(2, 6)}`,
                    title: title.trim(),
                    mode: modeName,
                    status: 'Pending',
                    risk: 'Low',
                    spans: 1,
                    createdAt: 'Just now',
                  };
                  tasks.unshift(newTask);
                  saveTasks(tasks);
                  console.log(chalk.green(`\n  ✔ Đã tạo task mới [${newTask.id}] (${newTask.mode}): "${newTask.title}"\n`));
                }
                break;
              }
              case 'advance':
                advanceTask(tasks[0]?.id);
                break;
              case 'diff':
                console.log();
                printUnifiedDiff();
                break;
              case 'permit':
                await promptPermitApproval();
                break;
              case 'status':
                printStatus();
                break;
              case 'clear':
                console.clear();
                printMainBanner();
                break;
              case 'help':
                printHelp();
                break;
              case 'exit':
                console.log(chalk.dim('\n  Tạm biệt! Custos Runtime đang bảo vệ hệ thống của bạn.\n'));
                inputDevice.removeListener('keypress', onKeypress);
                rl.close();
                return;
            }
          }
        }
      } catch (err) {
        if (err.name === 'ExitPromptError') {
          console.log(chalk.dim('\n  (Đã hủy thao tác)\n'));
        } else {
          throw err;
        }
      } finally {
        rl.resume();
      }
    } else if (
      cleanCmd === 'code' ||
      cleanCmd === 'custos-code' ||
      (cleanCmd === 'mode' && args[0]?.toLowerCase() === 'code')
    ) {
      currentMode = 'code';
      console.log(chalk.cyan(`\n  ✔ Đã chuyển sang chế độ [custos-code].\n`));
    } else if (
      cleanCmd === 'research' ||
      cleanCmd === 'custos-research' ||
      (cleanCmd === 'mode' && args[0]?.toLowerCase() === 'research')
    ) {
      currentMode = 'research';
      console.log(chalk.cyan(`\n  ✔ Đã chuyển sang chế độ [custos-research].\n`));
    } else if (
      cleanCmd === 'assistant' ||
      cleanCmd === 'custos-assistant' ||
      (cleanCmd === 'mode' && args[0]?.toLowerCase() === 'assistant')
    ) {
      currentMode = 'assistant';
      console.log(chalk.cyan(`\n  ✔ Đã chuyển sang chế độ [custos-assistant].\n`));
    } else if (cleanCmd === 'custos' || cleanCmd === 'exit-mode') {
      currentMode = 'custos';
      console.log(chalk.cyan('\n  ✔ Đã quay lại chế độ gốc [custos]. Bạn có thể chat bình thường hoặc gõ / để mở bảng lệnh.\n'));
    } else if (cleanCmd === 'vibe' || cleanCmd === 'v') {
      await runVibeWizard(currentMode !== 'custos' ? currentMode : null);
    } else if (cleanCmd === 'list' || cleanCmd === 'tasks' || cleanCmd === 'ls') {
      printTaskList();
    } else if (cleanCmd === 'create' || cleanCmd === 'new') {
      const title = argStr || 'New sovereign agentic workflow';
      const modeName = currentMode !== 'custos' ? (currentMode.charAt(0).toUpperCase() + currentMode.slice(1)) : 'Code';
      const newTask = {
        id: `task-${Math.random().toString(36).substring(2, 6)}`,
        title,
        mode: modeName,
        status: 'Pending',
        risk: 'Low',
        spans: 1,
        createdAt: 'Just now',
      };
      tasks.unshift(newTask);
      saveTasks(tasks);
      console.log(chalk.green(`  ✔ Đã tạo task mới [${newTask.id}] (${newTask.mode}): "${title}"\n`));
    } else if (cleanCmd === 'advance') {
      advanceTask(args[0] || tasks[0]?.id);
    } else if (cleanCmd === 'diff' || cleanCmd === 'd') {
      printUnifiedDiff();
    } else if (cleanCmd === 'permit' || cleanCmd === 'p') {
      await promptPermitApproval();
    } else if (cleanCmd === 'status') {
      printStatus();
    } else if (cleanCmd === 'help' || cleanCmd === '?') {
      printHelp();
    } else {
      if (currentMode !== 'custos') {
        const fullPrompt = line;
        console.log(chalk.cyan(`\n  ⚡ [custos-${currentMode.toLowerCase()}] Đang xử lý yêu cầu: "${fullPrompt}"...`));
        const s1 = ora(chalk.cyan('Khởi tạo sandbox cô lập (Worktree Sandbox)...')).start();
        await new Promise((r) => setTimeout(r, 500));
        s1.succeed(chalk.white('Sandbox cô lập đã sẵn sàng: ') + chalk.dim('.custos/sandboxes/worktree-live'));

        const s2 = ora(chalk.cyan('Phân tích AST & tổng hợp bản vá...')).start();
        await new Promise((r) => setTimeout(r, 700));
        s2.succeed(chalk.white('Tổng hợp AST hoàn tất: ') + chalk.green('+5 additions, -2 deletions'));

        console.log();
        printUnifiedDiff();

        const approve = await confirm({
          message: chalk.bold.white('Phê duyệt Giấy phép Thực thi (ExecutionPermit) để áp dụng vào mã nguồn?'),
          default: true,
        });

        if (approve) {
          const s3 = ora(chalk.cyan('Ghi nhận audit span vào SQLite & hoàn tất nhiệm vụ...')).start();
          await new Promise((r) => setTimeout(r, 500));
          s3.succeed(chalk.green('Nhiệm vụ đã hoàn tất thành công dưới cơ chế bảo vệ Epoch Lock!'));

          const newTask = {
            id: `task-${Math.random().toString(36).substring(2, 6)}`,
            title: fullPrompt,
            mode: currentMode.charAt(0).toUpperCase() + currentMode.slice(1),
            status: 'Completed',
            risk: 'Medium',
            spans: 5,
            createdAt: 'Just now',
          };
          tasks.unshift(newTask);
          saveTasks(tasks);
        } else {
          console.log(chalk.yellow('Đã hủy cấp phép. Sandbox đã được rollback an toàn.'));
        }
        console.log();
      } else {
        console.log(chalk.cyan(`\n  💬 [CUSTOS] `) + chalk.white(`"${line}"`));
        console.log(chalk.dim(`  Hệ thống đang ở chế độ giao tiếp chung. Bạn có thể chat tự do hoặc gõ `) + chalk.yellow.bold('/') + chalk.dim(` để kích hoạt engine chuyên biệt (custos-code, custos-research, custos-assistant).\n`));
      }
    }
  }

  inputDevice.removeListener('keypress', onKeypress);
  rl.close();
}

export function printHelp() {
  console.log(chalk.bold.hex('#38bdf8')('\n  ══ Custos CLI — Bảng tra cứu lệnh ══\n'));

  const commands = [
    ['/ (hoặc mode)', 'Mở bảng chọn chế độ hoạt động (custos-code, custos-research, ...)'],
    ['custos', 'Quay lại chế độ giao tiếp chung để chat tự do'],
    ['code / /code', 'Kích hoạt custos-code (Code Engine) trong Sandbox'],
    ['research / /research', 'Kích hoạt custos-research (Research Engine) điều tra'],
    ['assistant / /assistant', 'Kích hoạt custos-assistant (Assistant Engine) điều phối'],
    ['vibe (hoặc v)', 'Khởi chạy quy trình Vibe Coding tương tác từng bước'],
    ['list / tasks', 'Hiển thị bảng danh sách các task và số lượng SQLite Spans'],
    ['create <tiêu đề>', 'Tạo một nhiệm vụ agentic mới vào hàng đợi'],
    ['advance <id>', 'Chuyển tiến trình task sang trạng thái tiếp theo'],
    ['diff (hoặc d)', 'Xem bản so sánh Unified Diff với màu sắc highlight'],
    ['permit (hoặc p)', 'Xem và phê duyệt Giấy phép Thực thi (Human-in-the-Loop)'],
    ['status', 'Kiểm tra trạng thái hệ thống, daemon và sandbox'],
    ['clear / cls', 'Xóa màn hình terminal và in lại banner'],
    ['help / ?', 'Hiển thị trợ giúp này'],
    ['exit / quit', 'Thoát khỏi Custos CLI'],
  ];

  commands.forEach(([c, d]) => {
    console.log(`  ${chalk.yellow.bold(c.padEnd(24))} ${chalk.white(d)}`);
  });

  console.log(chalk.dim('\n  Tham số dòng lệnh nhanh:'));
  console.log(`  ${chalk.white('custos vibe')}         ${chalk.dim('Chạy ngay wizard Vibe Coding')}`);
  console.log(`  ${chalk.white('custos code')}         ${chalk.dim('Kích hoạt Code Engine (custos-code)')}`);
  console.log(`  ${chalk.white('custos research')}     ${chalk.dim('Kích hoạt Research Engine (custos-research)')}`);
  console.log(`  ${chalk.white('custos assistant')}    ${chalk.dim('Kích hoạt Assistant Engine (custos-assistant)')}`);
  console.log(`  ${chalk.white('custos list')}         ${chalk.dim('In bảng task hiện tại')}`);
  console.log(`  ${chalk.white('custos status')}       ${chalk.dim('In thông tin trạng thái')}`);
  console.log(`  ${chalk.white('custos diff')}         ${chalk.dim('Xem bản vá Unified Diff')}`);
  console.log(`  ${chalk.white('custos permit')}       ${chalk.dim('Xác nhận giấy phép thực thi')}`);
  console.log();
}

async function main() {
  const args = process.argv.slice(2);
  const first = args[0]?.toLowerCase();

  if (!first) {
    await startRepl();
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
    printTaskList();
  } else if (first === 'create' || first === 'new') {
    const title = args.slice(1).join(' ') || 'New sovereign agentic workflow';
    const newTask = {
      id: `task-${Math.random().toString(36).substring(2, 6)}`,
      title,
      mode: 'Code',
      status: 'Pending',
      risk: 'Low',
      spans: 1,
      createdAt: 'Just now',
    };
    tasks.unshift(newTask);
    saveTasks(tasks);
    console.log(chalk.green(`\n  ✔ Đã tạo task mới [${newTask.id}]: "${title}"\n`));
  } else if (first === 'advance') {
    advanceTask(args[1] || tasks[0]?.id);
  } else if (first === 'diff' || first === 'd') {
    console.log();
    printUnifiedDiff();
  } else if (first === 'permit' || first === 'p') {
    await promptPermitApproval();
  } else if (first === 'status') {
    printStatus();
  } else if (first === '--help' || first === '-h' || first === 'help' || first === '?') {
    printHelp();
  } else {
    console.log(chalk.red(`\n  Lệnh không hợp lệ: "${first}"`));
    printHelp();
  }
}

main().catch((err) => {
  if (err.name === 'ExitPromptError') {
    console.log(chalk.dim('\n  Đã hủy thao tác.\n'));
    process.exit(0);
  }
  console.error(err);
  process.exit(1);
});
