import chalk from 'chalk';
import { stdin as inputDevice, stdout as outputDevice } from 'node:process';
import { VERSION } from './daemon.js';
import { stripAnsi, padToWidth } from './markdown.js';

export const CHAT_WIDTH = 58;
let alternateScreenActive = false;

export const COMMANDS = [
  'assistant',
  'clear',
  'code',
  'create',
  'diff',
  'help',
  'list',
  'login',
  'logout',
  'mode',
  'model',
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

export function suggestCommands(inputCommand) {
  return COMMANDS
    .map((command) => ({ command, distance: editDistance(inputCommand, command) }))
    .filter(({ distance }) => distance <= Math.max(2, Math.floor(inputCommand.length / 3)))
    .sort((left, right) => left.distance - right.distance || left.command.localeCompare(right.command))
    .slice(0, 3)
    .map(({ command }) => command);
}

export function printUnknownCommand(inputCommand) {
  console.error(chalk.red(`\n  Không tìm thấy lệnh "${inputCommand}".`));
  const suggestions = suggestCommands(inputCommand);
  if (suggestions.length > 0) {
    console.error(chalk.dim('  Có phải bạn muốn dùng: ') + suggestions.map((item) => chalk.cyan(item)).join(', '));
  }
  console.error(chalk.dim("  Bước tiếp theo: chạy 'custos help' hoặc nhấn '/' trong chế độ tương tác.\n"));
}

export function getTerminalWidth() {
  return process.stdout.columns || 100;
}

export function enterAlternateScreen() {
  if (!outputDevice.isTTY || alternateScreenActive) return;
  outputDevice.write('\x1b[?1049h\x1b[2J\x1b[H\x1b[?25h');
  alternateScreenActive = true;
}

export function leaveAlternateScreen() {
  if (!alternateScreenActive) return;
  outputDevice.write('\x1b[?1049l');
  alternateScreenActive = false;
}

export function shutdownCli(exitCode = 0) {
  outputDevice.write('\x1b[?1006l\x1b[?1000l\x1b[?25h');
  if (inputDevice.isTTY && inputDevice.isRaw) inputDevice.setRawMode(false);
  inputDevice.pause();
  leaveAlternateScreen();
  outputDevice.write(chalk.dim('\n  Tạm biệt! Custos đã thoát.\n\n'));
  process.exit(exitCode);
}

export function getPanelIndent(width = CHAT_WIDTH) {
  return ' '.repeat(Math.max(0, Math.floor((getTerminalWidth() - width) / 2)));
}

export const BANNER_ART_RAW = [
  ' ██████╗██╗   ██╗███████╗████████╗ ██████╗ ███████╗',
  '██╔════╝██║   ██║██╔════╝╚══██╔══╝██╔═══██╗██╔════╝',
  '██║     ██║   ██║███████╗   ██║   ██║   ██║███████╗',
  '██║     ██║   ██║╚════██║   ██║   ██║   ██║╚════██║',
  '╚██████╗╚██████╔╝███████║   ██║   ╚██████╔╝███████║',
  ' ╚═════╝ ╚═════╝ ╚══════╝   ╚═╝    ╚═════╝ ╚══════╝',
];
export const BANNER_WIDTH = 52;

export function getBannerLogoLines() {
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

export function printCentered(content = '') {
  const visibleWidth = stripAnsi(content).length;
  const leftPadding = Math.max(0, Math.floor((getTerminalWidth() - visibleWidth) / 2));
  console.log(`${' '.repeat(leftPadding)}${content}`);
}

export function printPanelLine(content = '') {
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
    { name: 'mode', description: 'Chọn phạm vi Code, Research hoặc Assistant', value: { type: 'submenu', value: 'mode' } },
    { name: 'model', description: 'Đổi provider hoặc model hiện tại', value: { type: 'action', value: 'model' } },
    { name: 'login', description: 'Đăng nhập Codex, Claude hoặc Antigravity', value: { type: 'action', value: 'login' } },
    { name: 'logout', description: 'Đăng xuất khỏi provider đang kết nối', value: { type: 'action', value: 'logout' } },
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

export function promptCenteredPalette(items, _title) {
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

      const inputDisplay = query ? ` /${query}` : ` /${chalk.rgb(100, 100, 100)('gõ để lọc lệnh...')}`;
      output.push(indentStr + blueBar + bg(padToWidth(inputDisplay, innerW)) + '\x1b[K');

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

export function getAgentTheme(mode = 'code') {
  const modeKey = (mode || 'code').toLowerCase();
  const configs = {
    code: { name: 'Build', color: '#38bdf8' },
    research: { name: 'Plan', color: '#f59e0b' },
    assistant: { name: 'Assistant', color: '#10b981' },
    custos: { name: 'Chat', color: '#a78bfa' },
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

export function waitForKeypress() {
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

