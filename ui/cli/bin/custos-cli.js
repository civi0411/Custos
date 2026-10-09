#!/usr/bin/env node

import chalk from 'chalk';
import boxen from 'boxen';
import ora from 'ora';
import { select, input, confirm } from '@inquirer/prompts';
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
const CHAT_WIDTH = 58;
let alternateScreenActive = false;
const COMMANDS = [
  'assistant',
  'clear',
  'code',
  'create',
  'diff',
  'help',
  'list',
  'mode',
  'permit',
  'research',
  'status',
  'vibe',
];

function editDistance(left, right) {
  const rows = Array.from({ length: right.length + 1 }, (_, index) => index);
  for (let leftIndex = 1; leftIndex <= left.length; leftIndex += 1) {
    let diagonal = rows[0];
    rows[0] = leftIndex;
    for (let rightIndex = 1; rightIndex <= right.length; rightIndex += 1) {
      const previous = rows[rightIndex];
      rows[rightIndex] = Math.min(
        rows[rightIndex] + 1,
        rows[rightIndex - 1] + 1,
        diagonal + (left[leftIndex - 1] === right[rightIndex - 1] ? 0 : 1),
      );
      diagonal = previous;
    }
  }
  return rows[right.length];
}

function suggestCommands(inputCommand) {
  return COMMANDS
    .map((command) => ({ command, distance: editDistance(inputCommand, command) }))
    .filter(({ distance }) => distance <= Math.max(2, Math.floor(inputCommand.length / 3)))
    .sort((left, right) => left.distance - right.distance || left.command.localeCompare(right.command))
    .slice(0, 3)
    .map(({ command }) => command);
}

function printUnknownCommand(inputCommand) {
  console.error(chalk.red(`\n  Không tìm thấy lệnh "${inputCommand}".`));
  const suggestions = suggestCommands(inputCommand);
  if (suggestions.length > 0) {
    console.error(chalk.dim('  Có phải bạn muốn dùng: ') + suggestions.map((item) => chalk.cyan(item)).join(', '));
  }
  console.error(chalk.dim("  Bước tiếp theo: chạy 'custos help' hoặc nhấn '/' trong chế độ tương tác.\n"));
}

function getTerminalWidth() {
  return process.stdout.columns || 100;
}

function enterAlternateScreen() {
  if (!outputDevice.isTTY || alternateScreenActive) return;
  outputDevice.write('\x1b[?1049h\x1b[2J\x1b[H\x1b[?25h');
  alternateScreenActive = true;
}

function leaveAlternateScreen() {
  if (!alternateScreenActive) return;
  outputDevice.write('\x1b[?1049l');
  alternateScreenActive = false;
}

function padToWidth(content, width) {
  return `${content}${' '.repeat(Math.max(0, width - stripAnsi(content).length))}`;
}

function getPanelIndent(width = CHAT_WIDTH) {
  return ' '.repeat(Math.max(0, Math.floor((getTerminalWidth() - width) / 2)));
}

function getBannerLines(version) {
  return [
    '',
    chalk.rgb(214, 214, 214).bold(' ██████╗██╗   ██╗███████╗████████╗ ██████╗ ███████╗'),
    chalk.rgb(198, 198, 198).bold('██╔════╝██║   ██║██╔════╝╚══██╔══╝██╔═══██╗██╔════╝'),
    chalk.rgb(180, 180, 180).bold('██║     ██║   ██║███████╗   ██║   ██║   ██║███████╗'),
    chalk.rgb(156, 156, 156).bold('██║     ██║   ██║╚════██║   ██║   ██║   ██║╚════██║'),
    chalk.rgb(132, 132, 132).bold('╚██████╗╚██████╔╝███████║   ██║   ╚██████╔╝███████║'),
    chalk.rgb(100, 100, 100).bold(' ╚═════╝ ╚═════╝ ╚══════╝   ╚═╝    ╚═════╝ ╚══════╝'),
    '',
    chalk.rgb(92, 92, 92)(`v${version}  agent workspace có kiểm soát`),
    '',
    padToWidth(`${chalk.hex('#eab308').bold('build')}  ${chalk.rgb(205, 205, 205)('custos')}  ${chalk.rgb(105, 105, 105)('tab đổi chế độ')}  ${chalk.hex('#eab308')('/ lệnh')}`, CHAT_WIDTH),
  ];
}

export function printMainBanner() {
  const bannerLines = getBannerLines(VERSION);
  const terminalWidth = getTerminalWidth();
  const terminalHeight = process.stdout.rows || 30;
  const verticalPadding = Math.max(2, Math.floor((terminalHeight - bannerLines.length - 3) / 2));
  console.clear();
  console.log('\n'.repeat(verticalPadding));
  for (const line of bannerLines) {
    const visibleWidth = stripAnsi(line).length;
    const leftPadding = Math.max(0, Math.floor((terminalWidth - visibleWidth) / 2));
    console.log(`${' '.repeat(leftPadding)}${line}`);
  }
  console.log('\n');
}

function stripAnsi(str) {
  return str.replace(/\x1b\[[0-9;]*m/g, '');
}

function printCentered(content = '') {
  const visibleWidth = stripAnsi(content).length;
  const leftPadding = Math.max(0, Math.floor((getTerminalWidth() - visibleWidth) / 2));
  console.log(`${' '.repeat(leftPadding)}${content}`);
}

function printPanelLine(content = '') {
  console.log(`${getPanelIndent()}${content}`);
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
    printCentered(line);
  }
  console.log();
}

export async function promptSlashSelection() {
  return promptCenteredPalette([
    { name: 'Code', description: 'Lập trình, sửa lỗi và kiểm thử', value: { type: 'mode', value: 'code' } },
    { name: 'Research', description: 'Phân tích và đối chiếu bằng chứng', value: { type: 'mode', value: 'research' } },
    { name: 'Assistant', description: 'Điều phối tác vụ và phê duyệt', value: { type: 'mode', value: 'assistant' } },
    { name: 'vibe', description: 'Khởi chạy quy trình tương tác', value: { type: 'action', value: 'vibe' } },
    { name: 'tasks', description: 'Xem danh sách nhiệm vụ', value: { type: 'action', value: 'list' } },
    { name: 'create', description: 'Tạo nhiệm vụ mới', value: { type: 'action', value: 'create' } },
    { name: 'advance', description: 'Chuyển bước nhiệm vụ', value: { type: 'action', value: 'advance' } },
    { name: 'diff', description: 'Xem thay đổi đề xuất', value: { type: 'action', value: 'diff' } },
    { name: 'permit', description: 'Xem yêu cầu phê duyệt', value: { type: 'action', value: 'permit' } },
    { name: 'status', description: 'Kiểm tra runtime và sandbox', value: { type: 'action', value: 'status' } },
    { name: 'clear', description: 'Làm sạch phiên hiển thị', value: { type: 'action', value: 'clear' } },
    { name: 'help', description: 'Xem hướng dẫn', value: { type: 'action', value: 'help' } },
    { name: 'exit', description: 'Thoát Custos', value: { type: 'action', value: 'exit' } },
  ], 'Lệnh và chế độ');
}

export async function promptModeSelection() {
  return promptCenteredPalette([
    { name: 'Code', description: 'Lập trình, sửa lỗi và kiểm thử', value: 'code' },
    { name: 'Research', description: 'Phân tích và đối chiếu bằng chứng', value: 'research' },
    { name: 'Assistant', description: 'Điều phối tác vụ và phê duyệt', value: 'assistant' },
  ], 'Chọn chế độ');
}

function promptCenteredPalette(items, title) {
  let selectedIndex = 0;
  let query = '';

  return new Promise((resolve) => {
    const filteredItems = () => items.filter((item) => {
      const searchText = `${item.name} ${item.description}`.toLowerCase();
      return searchText.includes(query.toLowerCase());
    });

    const render = () => {
      const visibleItems = filteredItems();
      selectedIndex = Math.min(selectedIndex, Math.max(0, visibleItems.length - 1));
      printMainBanner();
      const inputText = ` /${query}`;
      printPanelLine(chalk.bgRgb(29, 29, 29).rgb(210, 210, 210)(padToWidth(inputText, CHAT_WIDTH)));

      for (const [index, item] of visibleItems.entries()) {
        const marker = index === selectedIndex ? chalk.white('›') : ' ';
        const command = chalk.hex('#eab308').bold(item.name.padEnd(12));
        const description = chalk.rgb(145, 145, 145)(item.description);
        const row = padToWidth(`${marker} ${command}${description}`, CHAT_WIDTH);
        const surface = index === selectedIndex
          ? chalk.bgRgb(38, 38, 38)(row)
          : chalk.bgRgb(22, 22, 22)(row);
        printPanelLine(surface);
      }

      if (visibleItems.length === 0) {
        printPanelLine(chalk.bgRgb(22, 22, 22).gray(padToWidth('  Không tìm thấy lệnh phù hợp', CHAT_WIDTH)));
      }
      printPanelLine(chalk.rgb(105, 105, 105)(padToWidth('↑↓ chọn   Enter xác nhận   Backspace đóng', CHAT_WIDTH)));
    };

    const finish = (value) => {
      inputDevice.removeListener('keypress', onPaletteKeypress);
      printMainBanner();
      resolve(value);
    };

    const onPaletteKeypress = (character, key) => {
      const visibleItems = filteredItems();
      if (key?.name === 'up') {
        selectedIndex = (selectedIndex - 1 + Math.max(1, visibleItems.length)) % Math.max(1, visibleItems.length);
      } else if (key?.name === 'down') {
        selectedIndex = (selectedIndex + 1) % Math.max(1, visibleItems.length);
      } else if (key?.name === 'return' || key?.name === 'enter') {
        finish(visibleItems[selectedIndex]?.value ?? null);
        return;
      } else if (key?.name === 'escape') {
        finish(null);
        return;
      } else if (key?.name === 'backspace') {
        if (query.length === 0) {
          finish(null);
          return;
        }
        query = query.slice(0, -1);
        selectedIndex = 0;
      } else if (character && !key?.ctrl && !key?.meta && character.length === 1 && character !== '/') {
        query += character;
        selectedIndex = 0;
      } else {
        return;
      }
      render();
    };

    if (inputDevice.isTTY) inputDevice.setRawMode(true);
    inputDevice.resume();
    inputDevice.on('keypress', onPaletteKeypress);
    render();
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
    `${chalk.bold.hex('#38bdf8')('Trạng thái Custos CLI')}\n\n` +
    `${chalk.cyan('• Nguồn dữ liệu')}     : ${chalk.yellow.bold('Mô phỏng cục bộ')}\n` +
    `${chalk.cyan('• Daemon')}            : ${chalk.dim('Chưa được kết nối từ CLI Node')}\n` +
    `${chalk.cyan('• Tệp nhiệm vụ')}      : ${chalk.white(TASKS_FILE)}\n` +
    `${chalk.cyan('• Nhiệm vụ')}          : ${chalk.yellow(tasks.length + ' mục')}\n` +
    `${chalk.cyan('• Chế độ')}            : ${chalk.white('code, research, assistant')}\n\n` +
    `${chalk.dim('Bước tiếp theo: dùng ứng dụng Tauri để kết nối daemon thật.')}`;

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
  enterAlternateScreen();
  printMainBanner();

  let currentMode = 'custos';
  emitKeypressEvents(inputDevice);
  const rl = readline.createInterface({ input: inputDevice, output: outputDevice });

  let waitingForInput = false;

  const onKeypress = (str, key) => {
    if (waitingForInput && (key?.name === 'return' || key?.name === 'enter')) {
      outputDevice.write('\x1b[0m');
    }
    if (waitingForInput && str === '/' && (rl.line?.trim() === '/' || rl.line?.trim() === '')) {
      waitingForInput = false;
      setImmediate(() => {
        rl.write('\r\n');
      });
      return;
    }
    if (waitingForInput && key?.name === 'tab' && !rl.line) {
      waitingForInput = false;
      setImmediate(() => {
        rl.write('mode\r\n');
      });
      return;
    }
    if (waitingForInput && key?.ctrl && key?.name === 'p') {
      waitingForInput = false;
      setImmediate(() => {
        rl.write('\x15/\r\n');
      });
    }
  };

  inputDevice.on('keypress', onKeypress);

  const getPrompt = () => {
    const inputLeft = Math.max(0, Math.floor((getTerminalWidth() - CHAT_WIDTH) / 2));
    const inputPadding = ' '.repeat(inputLeft);
    if (currentMode === 'custos') {
      return `${inputPadding}${chalk.bgRgb(29, 29, 29).rgb(205, 205, 205)(' │ ')}\x1b[48;2;29;29;29m\x1b[38;2;205;205;205m`;
    }
    return `${inputPadding}${chalk.bgRgb(29, 29, 29).rgb(205, 205, 205)(' │ ')}${chalk.bgRgb(29, 29, 29).hex('#eab308')(`custos-${currentMode} › `)}\x1b[48;2;29;29;29m\x1b[38;2;205;205;205m`;
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
            printCentered(chalk.cyan(`Đã chuyển sang custos-${currentMode}`));
          } else if (chosen.type === 'mode') {
            currentMode = chosen.value;
            printCentered(chalk.cyan(`Đã chuyển sang ${currentMode === 'custos' ? 'custos' : `custos-${currentMode}`}`));
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
                leaveAlternateScreen();
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
      printCentered(chalk.cyan('Đã chuyển sang custos-code'));
    } else if (
      cleanCmd === 'research' ||
      cleanCmd === 'custos-research' ||
      (cleanCmd === 'mode' && args[0]?.toLowerCase() === 'research')
    ) {
      currentMode = 'research';
      printCentered(chalk.cyan('Đã chuyển sang custos-research'));
    } else if (
      cleanCmd === 'assistant' ||
      cleanCmd === 'custos-assistant' ||
      (cleanCmd === 'mode' && args[0]?.toLowerCase() === 'assistant')
    ) {
      currentMode = 'assistant';
      printCentered(chalk.cyan('Đã chuyển sang custos-assistant'));
    } else if (cleanCmd === 'custos' || cleanCmd === 'exit-mode') {
      currentMode = 'custos';
      printMainBanner();
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
        printCentered(chalk.white(`Bạn: ${line}`));
        printCentered(chalk.dim("Custos đang ở chế độ hội thoại. Nhấn Tab để chọn agent hoặc '/' để mở lệnh."));
        console.log();
      }
    }
  }

  inputDevice.removeListener('keypress', onKeypress);
  rl.close();
  leaveAlternateScreen();
}

export function printHelp() {
  const sections = [
    ['Bắt đầu', [
      ['vibe', 'Tạo quy trình có hướng dẫn từng bước'],
      ['create <mục tiêu>', 'Tạo nhiệm vụ mới'],
      ['list', 'Xem các nhiệm vụ hiện có'],
      ['status', 'Kiểm tra trạng thái runtime'],
    ]],
    ['Chế độ làm việc', [
      ['code', 'Lập trình, sửa lỗi và kiểm thử'],
      ['research', 'Phân tích và đối chiếu bằng chứng'],
      ['assistant', 'Điều phối nhiệm vụ và phê duyệt'],
      ['mode', 'Mở bộ chọn chế độ tương tác'],
    ]],
    ['Kiểm soát', [
      ['diff', 'Xem thay đổi đang được đề xuất'],
      ['permit', 'Xem yêu cầu đang chờ phê duyệt'],
      ['advance <id>', 'Chuyển nhiệm vụ sang bước tiếp theo'],
    ]],
  ];

  console.log(chalk.bold.hex('#38bdf8')('\n  Custos CLI'));
  console.log(chalk.dim('  Không gian làm việc agent có kiểm soát\n'));
  console.log(`  ${chalk.white('Cách dùng:')} custos <lệnh> [tham số]`);
  console.log(`  ${chalk.white('Tương tác:')} chạy ${chalk.cyan('custos')}, sau đó nhấn ${chalk.cyan('/')} để mở lệnh nhanh`);

  for (const [title, commands] of sections) {
    console.log(chalk.bold(`\n  ${title}`));
    for (const [command, description] of commands) {
      console.log(`    ${chalk.cyan(command.padEnd(20))} ${description}`);
    }
  }

  console.log(chalk.bold('\n  Hệ thống'));
  console.log(`    ${chalk.cyan('help'.padEnd(20))} Hiển thị hướng dẫn này`);
  console.log(`    ${chalk.cyan('clear'.padEnd(20))} Xóa màn hình tương tác`);
  console.log(`    ${chalk.cyan('exit'.padEnd(20))} Thoát chế độ tương tác`);
  console.log(chalk.dim("\n  Ví dụ: custos create \"Kiểm tra luồng đăng nhập\"\n"));
}

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
