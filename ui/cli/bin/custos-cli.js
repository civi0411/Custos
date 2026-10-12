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

const BANNER_ART_RAW = [
  ' ██████╗██╗   ██╗███████╗████████╗ ██████╗ ███████╗',
  '██╔════╝██║   ██║██╔════╝╚══██╔══╝██╔═══██╗██╔════╝',
  '██║     ██║   ██║███████╗   ██║   ██║   ██║███████╗',
  '██║     ██║   ██║╚════██║   ██║   ██║   ██║╚════██║',
  '╚██████╗╚██████╔╝███████║   ██║   ╚██████╔╝███████║',
  ' ╚═════╝ ╚═════╝ ╚══════╝   ╚═╝    ╚═════╝ ╚══════╝',
];
const BANNER_WIDTH = 52;

function getBannerLogoLines() {
  return [
    chalk.rgb(214, 214, 214).bold(BANNER_ART_RAW[0]),
    chalk.rgb(198, 198, 198).bold(BANNER_ART_RAW[1]),
    chalk.rgb(180, 180, 180).bold(BANNER_ART_RAW[2]),
    chalk.rgb(156, 156, 156).bold(BANNER_ART_RAW[3]),
    chalk.rgb(132, 132, 132).bold(BANNER_ART_RAW[4]),
    chalk.rgb(100, 100, 100).bold(BANNER_ART_RAW[5]),
  ];
}

export function printMainBanner() {
  const termW = getTerminalWidth();
  const termH = process.stdout.rows || 30;
  const leftIndent = Math.max(2, Math.floor((termW - BANNER_WIDTH) / 2));
  const indentStr = ' '.repeat(leftIndent);

  const totalLines = BANNER_ART_RAW.length + 3;
  const verticalPadding = Math.max(1, Math.floor((termH - totalLines) / 2));

  console.log('\n'.repeat(verticalPadding));
  for (const line of getBannerLogoLines()) {
    console.log(indentStr + line);
  }
  console.log();
  const subtitleText = `v${VERSION}  agent workspace có kiểm soát`;
  const subIndent = Math.max(2, Math.floor((termW - subtitleText.length) / 2));
  console.log(' '.repeat(subIndent) + chalk.rgb(115, 115, 115)(subtitleText));
  console.log();
}

function stripAnsi(str) {
  return (str || '').replace(/\x1b\[[0-9;?]*[a-zA-Z]/g, '');
}

function wrapString(str, maxWidth) {
  if (!str) return [''];
  const words = str.split(' ');
  const lines = [];
  let current = '';

  for (const word of words) {
    const visibleWordLen = stripAnsi(word).length;
    if (!current) {
      if (visibleWordLen > maxWidth) {
        let remaining = word;
        while (stripAnsi(remaining).length > maxWidth) {
          lines.push(remaining.slice(0, maxWidth));
          remaining = remaining.slice(maxWidth);
        }
        current = remaining;
      } else {
        current = word;
      }
    } else if (stripAnsi(current).length + 1 + visibleWordLen <= maxWidth) {
      current += ' ' + word;
    } else {
      lines.push(current);
      if (visibleWordLen > maxWidth) {
        let remaining = word;
        while (stripAnsi(remaining).length > maxWidth) {
          lines.push(remaining.slice(0, maxWidth));
          remaining = remaining.slice(maxWidth);
        }
        current = remaining;
      } else {
        current = word;
      }
    }
  }
  if (current) lines.push(current);
  return lines.length > 0 ? lines : [''];
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

    outputDevice.write('\x1b[2J\x1b[H');

    const render = () => {
      const visibleItems = filteredItems();
      selectedIndex = Math.min(selectedIndex, Math.max(0, visibleItems.length - 1));

      const termW = getTerminalWidth();
      const termHeight = process.stdout.rows || 30;
      const paletteWidth = Math.min(74, Math.max(58, termW - 8));
      const leftIndent = Math.max(2, Math.floor((termW - paletteWidth) / 2));
      const indentStr = ' '.repeat(leftIndent);
      const innerW = paletteWidth - 3;
      const blueBar = chalk.hex('#38bdf8').bold('▌');
      const bg = (s) => chalk.bgRgb(26, 26, 26)(s);

      const bannerLeft = Math.max(2, Math.floor((termW - BANNER_WIDTH) / 2));
      const bannerIndentStr = ' '.repeat(bannerLeft);

      const maxListRows = Math.min(visibleItems.length, 9);
      const totalLines = BANNER_ART_RAW.length + 3 + maxListRows + 3;
      const verticalPadding = Math.max(0, Math.floor((termHeight - totalLines) / 2));

      const subtitleText = `v${VERSION}  agent workspace có kiểm soát`;
      const subIndentStr = ' '.repeat(Math.max(2, Math.floor((termW - subtitleText.length) / 2)));

      const output = [];
      for (let i = 0; i < verticalPadding; i++) output.push('\x1b[K');
      for (const line of getBannerLogoLines()) output.push(bannerIndentStr + line + '\x1b[K');
      output.push('\x1b[K');
      output.push(subIndentStr + chalk.rgb(115, 115, 115)(subtitleText) + '\x1b[K');
      output.push('\x1b[K');

      // Top search row
      const inputDisplay = query ? ` /${query}` : ` /${chalk.rgb(100, 100, 100)('gõ để lọc lệnh...')}`;
      output.push(indentStr + blueBar + bg(padToWidth(inputDisplay, innerW)) + '\x1b[K');

      // Visible items slice with windowing
      const startIdx = Math.max(0, Math.min(selectedIndex - 3, visibleItems.length - maxListRows));
      const slice = visibleItems.slice(startIdx, startIdx + maxListRows);

      for (let i = 0; i < slice.length; i++) {
        const item = slice[i];
        const actualIdx = startIdx + i;
        const isSelected = actualIdx === selectedIndex;
        const marker = isSelected ? chalk.white('›') : ' ';
        const cmdName = isSelected ? chalk.hex('#38bdf8').bold(item.name.padEnd(12)) : chalk.white(item.name.padEnd(12));
        const desc = isSelected ? chalk.rgb(220, 220, 220)(item.description) : chalk.rgb(130, 130, 130)(item.description);
        const rowContent = ` ${marker} ${cmdName} ${desc}`;
        const rowSurface = isSelected
          ? chalk.bgRgb(38, 38, 38)(padToWidth(rowContent, innerW))
          : chalk.bgRgb(24, 24, 24)(padToWidth(rowContent, innerW));
        output.push(indentStr + blueBar + rowSurface + '\x1b[K');
      }

      if (visibleItems.length === 0) {
        output.push(indentStr + blueBar + bg(chalk.rgb(120, 120, 120)(padToWidth('   Không tìm thấy lệnh phù hợp', innerW))) + '\x1b[K');
      }

      output.push(indentStr + ' ' + chalk.rgb(105, 105, 105)(padToWidth('↑↓ chọn   Enter xác nhận   Esc đóng', innerW)) + '\x1b[K');

      outputDevice.write('\x1b[?25l\x1b[H' + output.join('\r\n'));
    };

    const finish = (value) => {
      inputDevice.removeListener('keypress', onPaletteKeypress);
      outputDevice.write('\x1b[2J\x1b[H');
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
  } catch { }
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
  } catch { }
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

function getAgentTheme(mode = 'code') {
  const modeKey = (mode || 'code').toLowerCase();
  const configs = {
    code: { name: 'Build', color: '#38bdf8' },
    research: { name: 'Plan', color: '#f59e0b' },
    assistant: { name: 'Assistant', color: '#10b981' },
    custos: { name: 'Build', color: '#38bdf8' },
  };
  return configs[modeKey] || configs.code;
}

export function printUserMessageCard(promptText, mode = 'code') {
  const termW = getTerminalWidth();
  const boxW = Math.min(76, Math.max(56, termW - 8));
  const boxLeft = Math.max(2, Math.floor((termW - boxW) / 2));
  const indentStr = ' '.repeat(boxLeft);
  const innerW = boxW - 3;
  const theme = getAgentTheme(mode);
  const blueBar = chalk.hex(theme.color).bold('▌');
  const bg = (s) => chalk.bgRgb(26, 26, 26)(s);

  console.log();
  const line = indentStr + blueBar + bg(' ' + chalk.white.bold(padToWidth(promptText, innerW)));
  console.log(line);
  console.log();
}

function renderHeroFrame({
  currentInput = '',
  cursorPos = 0,
  currentMode = 'custos',
  chatTurns = [],
  scrollOffset = 0,
  scrollState,
}) {
  const termW = getTerminalWidth();
  const termH = process.stdout.rows || 30;

  const agentTheme = getAgentTheme(currentMode);
  const modeBadge = agentTheme.name;
  const agentColor = agentTheme.color;
  const modelName = 'Claude Opus 4.5';
  const engineName = 'OpenCode Zen';

  const placeholderText = 'add a new field or task for Custos...';
  const minBoxW = 60;
  const maxBoxW = Math.min(76, termW - 6);
  const inputLen = stripAnsi(currentInput).length;
  const contentLen = Math.max(inputLen, currentInput.length === 0 ? placeholderText.length : 0);
  const boxW = Math.min(maxBoxW, Math.max(minBoxW, contentLen + 8));

  const bannerLeft = Math.max(2, Math.floor((termW - BANNER_WIDTH) / 2));
  const bannerIndentStr = ' '.repeat(bannerLeft);

  const boxLeft = Math.max(2, Math.floor((termW - boxW) / 2));
  const boxIndentStr = ' '.repeat(boxLeft);

  let renderedInputChars = '';
  if (currentInput.length === 0) {
    renderedInputChars = chalk.bgWhite.black(placeholderText[0]) + chalk.rgb(85, 85, 85)(placeholderText.slice(1));
  } else {
    for (let i = 0; i < currentInput.length; i++) {
      if (i === cursorPos) {
        renderedInputChars += chalk.bgWhite.black(currentInput[i]);
      } else {
        renderedInputChars += chalk.rgb(240, 240, 240)(currentInput[i]);
      }
    }
    if (cursorPos >= currentInput.length) {
      renderedInputChars += chalk.bgWhite.black(' ');
    }
  }

  const innerW = boxW - 3;
  const visibleInputLen = currentInput.length === 0
    ? placeholderText.length
    : Math.max(1, currentInput.length + (cursorPos >= currentInput.length ? 1 : 0));
  const padInputRight = Math.max(0, innerW - visibleInputLen);

  const bg = (s) => chalk.bgRgb(26, 26, 26)(s);
  const blueBar = chalk.hex(agentColor).bold('▌');

  const line1 = boxIndentStr + blueBar + bg(' ' + renderedInputChars + ' '.repeat(padInputRight));
  const line2 = boxIndentStr + blueBar + bg(' '.repeat(innerW + 1));
  const statusContent = ` ${chalk.hex(agentColor).bold(modeBadge)}  ${chalk.white.bold(modelName)}  ${chalk.rgb(115, 115, 115)(engineName)}`;
  const statusVisibleLen = stripAnsi(statusContent).length;
  const padStatusRight = Math.max(0, innerW + 1 - statusVisibleLen);
  const line3 = boxIndentStr + blueBar + bg(statusContent + ' '.repeat(padStatusRight));

  const shortcutHint = (chatTurns && chatTurns.length > 0)
    ? `${chalk.rgb(150, 150, 150)('wheel/pgup')} ${chalk.rgb(105, 105, 105)('cuộn')}  ${chalk.rgb(150, 150, 150)('tab')} ${chalk.rgb(105, 105, 105)('switch agent')}  ${chalk.rgb(150, 150, 150)('ctrl+p')} ${chalk.rgb(105, 105, 105)('commands')}`
    : `${chalk.rgb(150, 150, 150)('tab')} ${chalk.rgb(105, 105, 105)('switch agent')}  ${chalk.rgb(150, 150, 150)('ctrl+p')} ${chalk.rgb(105, 105, 105)('commands')}`;

  const shortcutsVisibleLen = stripAnsi(shortcutHint).length;
  const padShortcuts = Math.max(0, boxW - shortcutsVisibleLen);
  const lineShortcuts = ' '.repeat(boxLeft + padShortcuts) + shortcutHint;

  const output = [];

  if (scrollState) {
    scrollState.maxScroll = 0;
    scrollState.effectiveScroll = 0;
  }

  if (!chatTurns || chatTurns.length === 0) {
    const totalLines = BANNER_ART_RAW.length + 3 + 5 + 7;
    const verticalPadding = Math.max(0, Math.floor((termH - totalLines) / 2));
    const subtitleText = `v${VERSION}  agent workspace có kiểm soát`;
    const subIndentStr = ' '.repeat(Math.max(2, Math.floor((termW - subtitleText.length) / 2)));

    for (let i = 0; i < verticalPadding; i++) {
      output.push('\x1b[K');
    }
    for (const artLine of getBannerLogoLines()) {
      output.push(bannerIndentStr + artLine + '\x1b[K');
    }
    output.push('\x1b[K');
    output.push(subIndentStr + chalk.rgb(115, 115, 115)(subtitleText) + '\x1b[K');
    output.push('\x1b[K');

    // Show initial notification ONCE before human chats
    output.push(boxIndentStr + chalk.hex('#a855f7')('✔') + ' ' + chalk.rgb(220, 220, 220)('Khởi tạo Worktree Sandbox cô lập (.custos/sandboxes/worktree-live)') + '\x1b[K');
    output.push(boxIndentStr + chalk.hex('#a855f7')('✔') + ' ' + chalk.rgb(220, 220, 220)('Phân tích AST & tổng hợp bản vá: +5 additions, -2 deletions') + '\x1b[K');
    output.push(boxIndentStr + chalk.hex('#a855f7')('✔') + ' ' + chalk.rgb(220, 220, 220)('Nhiệm vụ hoàn tất an toàn dưới cơ chế Epoch Lock Guard!') + '\x1b[K');
    output.push(boxIndentStr + chalk.rgb(115, 115, 115)(`${modeBadge} · Claude Opus 4.5 · 1.2s · 42.1 tok/s`) + '\x1b[K');
    output.push('\x1b[K');
  } else {
    // Build full conversation history for the entire session
    const allHistoryLines = [];

    if (termH >= 36) {
      for (const artLine of getBannerLogoLines()) {
        allHistoryLines.push(bannerIndentStr + artLine);
      }
      const subtitleText = `v${VERSION}  agent workspace có kiểm soát`;
      const subIndentStr = ' '.repeat(Math.max(2, Math.floor((termW - subtitleText.length) / 2)));
      allHistoryLines.push(subIndentStr + chalk.rgb(115, 115, 115)(subtitleText));
      allHistoryLines.push('');
    } else {
      const topTitle = `${chalk.rgb(214, 214, 214).bold('CUSTOS')}  ${chalk.rgb(115, 115, 115)(`v${VERSION}  agent workspace có kiểm soát`)}`;
      const topIndent = Math.max(2, Math.floor((termW - stripAnsi(topTitle).length) / 2));
      allHistoryLines.push(' '.repeat(topIndent) + topTitle);
      allHistoryLines.push('');
    }

    const maxTextW = innerW - 3;

    // Render every turn of the session
    for (let tIdx = 0; tIdx < chatTurns.length; tIdx++) {
      const turn = chatTurns[tIdx];
      const turnTheme = getAgentTheme(turn.mode);
      const turnBar = chalk.hex(turnTheme.color).bold('▌');

      const wrappedQuery = wrapString(turn.query, maxTextW);
      wrappedQuery.forEach((qLine, i) => {
        if (i === 0) {
          allHistoryLines.push(boxIndentStr + turnBar + ' ' + chalk.white.bold(qLine));
        } else {
          allHistoryLines.push(boxIndentStr + '  ' + chalk.white.bold(qLine));
        }
      });

      for (const rLine of (turn.responseLines || [])) {
        const wrappedResp = wrapString(rLine, maxTextW);
        for (const wLine of wrappedResp) {
          allHistoryLines.push(boxIndentStr + '  ' + chalk.rgb(215, 215, 215)(padToWidth(wLine, innerW - 1)));
        }
      }

      if (turn.meta) {
        allHistoryLines.push(boxIndentStr + '  ' + chalk.rgb(105, 105, 105)(padToWidth(turn.meta, innerW - 1)));
      }
      allHistoryLines.push('');
    }

    const promptRows = 5; // empty row + line1 + line2 + line3 + shortcuts
    const indicatorRows = 1;
    const availableHistoryRows = Math.max(4, termH - promptRows - indicatorRows);

    const totalLines = allHistoryLines.length;
    const maxScroll = Math.max(0, totalLines - availableHistoryRows);
    const effectiveScroll = Math.max(0, Math.min(scrollOffset, maxScroll));

    if (scrollState) {
      scrollState.maxScroll = maxScroll;
      scrollState.effectiveScroll = effectiveScroll;
    }

    if (totalLines <= availableHistoryRows) {
      for (const hLine of allHistoryLines) {
        output.push(hLine + '\x1b[K');
      }
    } else {
      const endIndex = totalLines - effectiveScroll;
      const startIndex = Math.max(0, endIndex - availableHistoryRows);
      const visibleLines = allHistoryLines.slice(startIndex, endIndex);

      if (startIndex > 0) {
        output.push(boxIndentStr + chalk.rgb(120, 120, 120)('▲ ') + chalk.dim(`Còn ${startIndex} dòng phía trên [con lăn / PageUp để cuộn]`) + '\x1b[K');
      } else {
        output.push('\x1b[K');
      }

      for (const vLine of visibleLines) {
        output.push(vLine + '\x1b[K');
      }

      if (effectiveScroll > 0) {
        output.push(boxIndentStr + chalk.hex('#38bdf8')('▼ ') + chalk.hex('#38bdf8')(`Đang xem lịch sử [${endIndex}/${totalLines}]`) + chalk.dim(' · PageDown / End để về cuối') + '\x1b[K');
      }
    }
  }

  output.push(line1 + '\x1b[K');
  output.push(line2 + '\x1b[K');
  output.push(line3 + '\x1b[K');
  output.push(lineShortcuts + '\x1b[K');

  const currentTotal = output.length;
  for (let i = currentTotal; i < termH; i++) {
    output.push('\x1b[K');
  }

  return output.join('\r\n');
}

function waitForKeypress() {
  return new Promise((resolve) => {
    const onKey = () => {
      inputDevice.removeListener('keypress', onKey);
      resolve();
    };
    if (inputDevice.isTTY) inputDevice.setRawMode(true);
    inputDevice.resume();
    inputDevice.once('keypress', onKey);
  });
}

function promptInteractiveChat({ currentMode, history, chatTurns = [] }) {
  return new Promise((resolve) => {
    let inputStr = '';
    let cursorPos = 0;
    let historyIdx = history.length;
    let scrollOffset = 0;
    let mouseKeypressBlockedUntil = 0;
    const scrollState = { maxScroll: 0, effectiveScroll: 0 };

    outputDevice.write('\x1b[2J\x1b[H');

    const render = () => {
      const frame = renderHeroFrame({
        currentInput: inputStr,
        cursorPos,
        currentMode,
        chatTurns,
        scrollOffset,
        scrollState,
      });
      scrollOffset = scrollState.effectiveScroll;
      outputDevice.write('\x1b[?25l\x1b[H' + frame);
    };

    const onResize = () => {
      render();
    };
    process.stdout.on('resize', onResize);

    const cleanup = () => {
      inputDevice.removeListener('keypress', onKeypress);
      inputDevice.removeListener('data', onMouseData);
      process.stdout.removeListener('resize', onResize);
      outputDevice.write('\x1b[?1006l\x1b[?1000l\x1b[?25h');
    };

    const onMouseData = (data) => {
      const sequence = data.toString();
      if (sequence.includes('\x1b[<')) {
        mouseKeypressBlockedUntil = Date.now() + 50;
      }
      const mouseEvents = sequence.matchAll(/\x1b\[<(\d+);\d+;\d+[Mm]/g);
      let changed = false;

      for (const event of mouseEvents) {
        const button = Number(event[1]);
        if ((button & 64) !== 64) continue;
        const direction = (button & 1) === 0 ? 1 : -1;
        scrollOffset = Math.max(0, Math.min(scrollState.maxScroll, scrollOffset + direction * 3));
        changed = true;
      }

      if (changed) render();
    };

    const onKeypress = (str, key) => {
      if (Date.now() <= mouseKeypressBlockedUntil || (str && /\x1b\[<\d+;\d+;\d+[Mm]/.test(str))) {
        return;
      }

      if (key?.ctrl && key?.name === 'c') {
        cleanup();
        leaveAlternateScreen();
        process.exit(0);
      }

      if (key?.name === 'return' || key?.name === 'enter') {
        scrollOffset = 0;
        cleanup();
        resolve({ type: 'submit', value: inputStr });
        return;
      }

      if (key?.name === 'tab') {
        cleanup();
        resolve({ type: 'tab' });
        return;
      }

      if ((key?.ctrl && key?.name === 'p') || (str === '/' && inputStr === '')) {
        cleanup();
        resolve({ type: 'slash' });
        return;
      }

      // Scrolling controls for browsing session chat history
      if (key?.name === 'pageup') {
        const pageSize = Math.max(3, Math.floor((process.stdout.rows || 30) / 2));
        scrollOffset = scrollOffset + pageSize;
        render();
        return;
      }

      if (key?.name === 'pagedown') {
        const pageSize = Math.max(3, Math.floor((process.stdout.rows || 30) / 2));
        scrollOffset = Math.max(0, scrollOffset - pageSize);
        render();
        return;
      }

      if ((key?.ctrl && key?.name === 'up') || (key?.shift && key?.name === 'up')) {
        scrollOffset = scrollOffset + 1;
        render();
        return;
      }

      if ((key?.ctrl && key?.name === 'down') || (key?.shift && key?.name === 'down')) {
        scrollOffset = Math.max(0, scrollOffset - 1);
        render();
        return;
      }

      if (key?.name === 'home') {
        if (scrollOffset > 0 || key?.ctrl) {
          scrollOffset = 999999;
          render();
          return;
        }
        cursorPos = 0;
        render();
        return;
      }

      if (key?.name === 'end') {
        if (scrollOffset > 0) {
          scrollOffset = 0;
          render();
          return;
        }
        cursorPos = inputStr.length;
        render();
        return;
      }

      if (key?.name === 'escape') {
        if (scrollOffset > 0) {
          scrollOffset = 0;
          render();
          return;
        }
      }

      if (key?.name === 'up') {
        if (scrollOffset > 0) {
          scrollOffset = scrollOffset + 1;
          render();
          return;
        }
        if (history.length > 0 && historyIdx > 0) {
          historyIdx--;
          inputStr = history[historyIdx] || '';
          cursorPos = inputStr.length;
          render();
        }
        return;
      }

      if (key?.name === 'down') {
        if (scrollOffset > 0) {
          scrollOffset = Math.max(0, scrollOffset - 1);
          render();
          return;
        }
        if (historyIdx < history.length - 1) {
          historyIdx++;
          inputStr = history[historyIdx] || '';
          cursorPos = inputStr.length;
          render();
        } else if (historyIdx === history.length - 1) {
          historyIdx = history.length;
          inputStr = '';
          cursorPos = 0;
          render();
        }
        return;
      }

      if (key?.name === 'backspace') {
        scrollOffset = 0;
        if (cursorPos > 0) {
          inputStr = inputStr.slice(0, cursorPos - 1) + inputStr.slice(cursorPos);
          cursorPos--;
          render();
        }
        return;
      }

      if (key?.name === 'delete') {
        scrollOffset = 0;
        if (cursorPos < inputStr.length) {
          inputStr = inputStr.slice(0, cursorPos) + inputStr.slice(cursorPos + 1);
          render();
        }
        return;
      }

      if (key?.name === 'left') {
        cursorPos = Math.max(0, cursorPos - 1);
        render();
        return;
      }

      if (key?.name === 'right') {
        cursorPos = Math.min(inputStr.length, cursorPos + 1);
        render();
        return;
      }

      if (str && !key?.ctrl && !key?.meta && str.length >= 1) {
        scrollOffset = 0;
        inputStr = inputStr.slice(0, cursorPos) + str + inputStr.slice(cursorPos);
        cursorPos += str.length;
        render();
      }
    };

    if (inputDevice.isTTY) inputDevice.setRawMode(true);
    inputDevice.resume();
    inputDevice.on('keypress', onKeypress);
    inputDevice.prependListener('data', onMouseData);
    outputDevice.write('\x1b[?1000h\x1b[?1006h');
    render();
  });
}

export async function startRepl() {
  enterAlternateScreen();

  let currentMode = 'custos';
  emitKeypressEvents(inputDevice);
  const history = [];
  const chatTurns = [];

  while (true) {
    const res = await promptInteractiveChat({ currentMode, history, chatTurns });

    if (res.type === 'tab') {
      try {
        const chosen = await promptModeSelection();
        if (chosen) {
          currentMode = chosen;
        }
      } catch (err) {
        if (err.name !== 'ExitPromptError') throw err;
      }
      continue;
    }

    if (res.type === 'slash') {
      try {
        const chosen = await promptSlashSelection();
        if (chosen) {
          if (chosen.type === 'submenu' && chosen.value === 'mode') {
            currentMode = await promptModeSelection();
          } else if (chosen.type === 'mode') {
            currentMode = chosen.value;
          } else if (chosen.type === 'action') {
            switch (chosen.value) {
              case 'vibe':
                await runVibeWizard(currentMode !== 'custos' ? currentMode : null);
                console.log(chalk.dim('\n  Nhấn phím bất kỳ để tiếp tục...'));
                await waitForKeypress();
                break;
              case 'list':
                printTaskList();
                console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
                await waitForKeypress();
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
                  console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
                  await waitForKeypress();
                }
                break;
              }
              case 'advance':
                advanceTask(tasks[0]?.id);
                console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
                await waitForKeypress();
                break;
              case 'diff':
                console.log();
                printUnifiedDiff();
                console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
                await waitForKeypress();
                break;
              case 'permit':
                await promptPermitApproval();
                console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
                await waitForKeypress();
                break;
              case 'status':
                printStatus();
                console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
                await waitForKeypress();
                break;
              case 'clear':
                break;
              case 'help':
                printHelp();
                console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
                await waitForKeypress();
                break;
              case 'exit':
                console.log(chalk.dim('\n  Tạm biệt! Custos Runtime đang bảo vệ hệ thống của bạn.\n'));
                leaveAlternateScreen();
                return;
            }
          }
        }
      } catch (err) {
        if (err.name !== 'ExitPromptError') throw err;
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
      console.log(chalk.dim('\n  Tạm biệt! Custos Runtime đang bảo vệ hệ thống của bạn.\n'));
      break;
    } else if (cleanCmd === 'clear' || cleanCmd === 'cls') {
      chatTurns.length = 0;
      continue;
    } else if (lowerCmd === '/' || lowerCmd === '/mode' || lowerCmd === 'mode' && !args[0]) {
      try {
        const chosen = await promptModeSelection();
        if (chosen) {
          currentMode = chosen;
        }
      } catch (err) {
        if (err.name !== 'ExitPromptError') throw err;
      }
      continue;
    } else if (
      cleanCmd === 'code' ||
      cleanCmd === 'custos-code' ||
      (cleanCmd === 'mode' && args[0]?.toLowerCase() === 'code')
    ) {
      currentMode = 'code';
    } else if (
      cleanCmd === 'research' ||
      cleanCmd === 'custos-research' ||
      (cleanCmd === 'mode' && args[0]?.toLowerCase() === 'research')
    ) {
      currentMode = 'research';
    } else if (
      cleanCmd === 'assistant' ||
      cleanCmd === 'custos-assistant' ||
      (cleanCmd === 'mode' && args[0]?.toLowerCase() === 'assistant')
    ) {
      currentMode = 'assistant';
    } else if (cleanCmd === 'custos' || cleanCmd === 'exit-mode') {
      currentMode = 'custos';
    } else if (cleanCmd === 'vibe' || cleanCmd === 'v') {
      await runVibeWizard(currentMode !== 'custos' ? currentMode : null);
      console.log(chalk.dim('\n  Nhấn phím bất kỳ để tiếp tục...'));
      await waitForKeypress();
    } else if (cleanCmd === 'list' || cleanCmd === 'tasks' || cleanCmd === 'ls') {
      printTaskList();
      console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
      await waitForKeypress();
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
      console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
      await waitForKeypress();
    } else if (cleanCmd === 'advance') {
      advanceTask(args[0] || tasks[0]?.id);
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
      printStatus();
      console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
      await waitForKeypress();
    } else if (cleanCmd === 'help' || cleanCmd === '?') {
      printHelp();
      console.log(chalk.dim('  Nhấn phím bất kỳ để tiếp tục...'));
      await waitForKeypress();
    } else {
      if (currentMode !== 'custos') {
        const fullPrompt = line;
        const agentName = getAgentTheme(currentMode).name;
        const lower = fullPrompt.toLowerCase().trim();
        let responseLines;
        if (lower === 'hi' || lower === 'hello' || lower === 'chào' || lower === 'hih' || lower === 'hey') {
          responseLines = [
            `Xin chào! Tôi là Custos ${agentName}.`,
            'Tôi đã sẵn sàng hỗ trợ bạn phân tích, lập trình và điều phối nhiệm vụ an toàn.',
          ];
        } else {
          responseLines = [
            `Đã tiếp nhận yêu cầu: "${fullPrompt}".`,
            'Đang xử lý trong Sandbox cô lập dưới cơ chế bảo vệ Epoch Lock Guard.',
          ];
        }
        chatTurns.push({
          query: fullPrompt,
          mode: currentMode,
          responseLines,
          meta: `${agentName} · Claude Opus 4.5 · 0.4s · OpenCode Zen`
        });

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
        continue;
      } else {
        const lower = line.toLowerCase().trim();
        let responseLines;
        if (lower === 'hi' || lower === 'hello' || lower === 'chào' || lower === 'hih' || lower === 'hey') {
          responseLines = [
            'Xin chào! Custos Sovereign Agentic Runtime đã sẵn sàng.',
            'Bạn có thể trò chuyện tự do hoặc nhấn Tab để chọn agent chuyên biệt.',
          ];
        } else {
          responseLines = [
            'Custos Sovereign Agentic Runtime đang ở chế độ hội thoại.',
            'Nhấn Tab để chọn agent hoặc \'/\' để mở danh mục lệnh.',
          ];
        }
        chatTurns.push({
          query: line,
          mode: 'code',
          responseLines,
          meta: 'Build · Claude Opus 4.5 · 0.4s · OpenCode Zen'
        });
        continue;
      }
    }
  }

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
