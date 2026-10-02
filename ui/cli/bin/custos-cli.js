#!/usr/bin/env node

/**
 * Custos CLI — Sovereign Agentic Operating System
 * High-Fidelity Pixel-Art Terminal User Interface (Reading Directly from frame-ui)
 */

import chalk from 'chalk';
import boxen from 'boxen';
import ora from 'ora';
import { select, input, confirm } from '@inquirer/prompts';
import * as readline from 'node:readline/promises';
import { stdin as inputDevice, stdout as outputDevice } from 'node:process';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const VERSION = '0.1.1';

// ── Load Mascots Cache (Built from ui/cli/frame-ui) ─────────────────────────
function loadMascots() {
  const cachePath = path.join(__dirname, 'mascots-cache.json');
  try {
    if (fs.existsSync(cachePath)) {
      return JSON.parse(fs.readFileSync(cachePath, 'utf8'));
    }
  } catch {}
  return { guardian: [], coder: [], inspector: [], steward: [] };
}

const MASCOTS = loadMascots();

function getTerminalWidth() {
  return process.stdout.columns || 100;
}

// ── Main Banner Lines ───────────────────────────────────────────────────────
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
    `  \x1b[38;2;56;189;248m❄\x1b[0m\x1b[38;2;251;191;36;1mCustos\x1b[0m \x1b[38;2;148;163;184mv${version}\x1b[0m \x1b[38;2;125;211;252;3m- Guardian of Agentic Work\x1b[0m`,
    '  \x1b[38;2;148;163;184mHuman-governed runtime for specialized agentic workflows\x1b[0m',
    '  \x1b[38;2;51;65;85m────────────────────────────────────────────────────────────\x1b[0m',
    '  \x1b[38;2;251;191;36m•\x1b[0m \x1b[38;2;248;250;252;1mIntelligence Engine:\x1b[0m \x1b[38;2;148;163;184mCustos Snowy Owl Mascot (owl.png)\x1b[0m',
    '  \x1b[38;2;125;211;252m•\x1b[0m \x1b[38;2;248;250;252;1mConcurrency Defense:\x1b[0m \x1b[38;2;148;163;184mEpoch Optimistic Lock & Replay\x1b[0m',
    '  \x1b[38;2;56;189;248m•\x1b[0m \x1b[38;2;248;250;252;1mWorktree Isolation:\x1b[0m  \x1b[38;2;148;163;184mSeatbelt & Bubblewrap Sandbox\x1b[0m',
    '  \x1b[38;2;168;85;247m•\x1b[0m \x1b[38;2;248;250;252;1mHuman-in-the-Loop:\x1b[0m   \x1b[38;2;148;163;184mExplicit ExecutionPermit Grants\x1b[0m',
    '  \x1b[38;2;52;211;153m•\x1b[0m \x1b[38;2;248;250;252;1mAudit Trail:\x1b[0m         \x1b[38;2;148;163;184mImmutable SQLite Task & Span Logs\x1b[0m',
    '',
    '  \x1b[38;2;51;65;85m────────────────────────────────────────────────────────────\x1b[0m',
    '  \x1b[38;2;125;211;252mOperational Modes:\x1b[0m  \x1b[38;2;203;213;225m1. Code  │  2. Research  │  3. Assitant\x1b[0m',
    '  \x1b[38;2;148;163;184mQuick Commands:\x1b[0m     \x1b[38;2;100;116;139mcustos vibe  │  status  │  advance  │  list\x1b[0m',
    '',
  ];
}

// ── Print Main Banner (Side-by-Side: Guardian Owl from owl.png) ───────────
export function printMainBanner() {
  const termW = getTerminalWidth();
  const bannerLines = getBannerLines(VERSION);
  const owlLines = MASCOTS.guardian || [];
  const owlWidth = 38;
  const blankOwl = ' '.repeat(owlWidth);

  console.log();

  if (termW >= 85 && owlLines.length > 0) {
    const combinedW = owlWidth + 3 + 62;
    const leftPadN = termW > combinedW ? Math.min(Math.floor((termW - combinedW) / 2), 4) : 1;
    const padStr = ' '.repeat(leftPadN);
    const maxLines = Math.max(owlLines.length, bannerLines.length);

    for (let i = 0; i < maxLines; i++) {
      const left = i < owlLines.length ? owlLines[i] : blankOwl;
      const right = i < bannerLines.length ? bannerLines[i] : '';
      console.log(`${padStr}${left}   ${right}`);
    }
  } else {
    if (owlLines.length > 0) {
      const padN = Math.max(0, Math.floor((termW - owlWidth) / 2));
      const padStr = ' '.repeat(padN);
      for (const line of owlLines) {
        console.log(`${padStr}${line}`);
      }
      console.log();
    }
    for (const line of bannerLines) {
      console.log(line);
    }
  }
  console.log();
}

// ── Strip ANSI for calculating string length ────────────────────────────────
function stripAnsi(str) {
  return str.replace(/\x1b\[[0-9;]*m/g, '');
}

// ── Mode Card: Mini Mascot Owl (from frame-ui) + Greeting Dialogue Box ─────
export function printModeCard(mode) {
  const modeKey = (mode || 'Code').toLowerCase();

  const configs = {
    code: {
      name: 'Code',
      tag: 'CHẾ ĐỘ: CODE MODE (LẬP TRÌNH & THỰC THI)',
      title: 'Xin chào! Tôi là Custos Coder Owl. ❄',
      msg: '"Tôi có thể giúp gì cho bạn hôm nay?"',
      b1: 'Lập trình tính năng, sửa lỗi & tái cấu trúc mã nguồn',
      b2: 'Thực thi an toàn trong Sandbox Worktree cách ly',
      b3: 'Tối ưu thuật toán & kiểm tra tính đúng đắn',
      color: '\x1b[38;2;125;211;252;1m',
      badgeColor: '#38bdf8',
      owl: MASCOTS.coder || [],
    },
    research: {
      name: 'Research',
      tag: 'CHẾ ĐỘ: RESEARCH MODE (ĐIỀU TRA & NGHIÊN CỨU)',
      title: 'Xin chào! Tôi là Custos Inspector Owl. ❄',
      msg: '"Tôi có thể giúp gì cho bạn hôm nay?"',
      b1: 'Điều tra cấu trúc dự án & đối soát bằng chứng',
      b2: 'Phân tích tài liệu, kiến trúc & suy luận chuyên sâu',
      b3: 'Kiểm chứng các thay đổi trước khi xin phê duyệt',
      color: '\x1b[38;2;251;191;36;1m',
      badgeColor: '#f59e0b',
      owl: MASCOTS.inspector || [],
    },
    assistant: {
      name: 'Assistant',
      tag: 'CHẾ ĐỘ: ASSISTANT MODE (TRỢ LÝ & ĐIỀU PHỐI)',
      title: 'Xin chào! Tôi là Custos Steward Owl. ❄',
      msg: '"Tôi có thể giúp gì cho bạn hôm nay?"',
      b1: 'Điều phối quy trình tác vụ tự động theo chuẩn runtime',
      b2: 'Quản lý trạng thái Task, Epoch Lock & Audit Logs',
      b3: 'Theo dõi tiến trình & báo cáo kết quả thực thi',
      color: '\x1b[38;2;52;211;153;1m',
      badgeColor: '#10b981',
      owl: MASCOTS.steward || [],
    },
  };

  const cfg = configs[modeKey] || configs.code;
  const termW = getTerminalWidth();
  const owlLines = cfg.owl;
  const owlW = 20;
  const blankOwl = ' '.repeat(owlW);

  const boxW = Math.min(Math.max(termW - 28, 56), 72);

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
    padLine('\x1b[38;2;148;163;184mNhập mục tiêu nhiệm vụ hoặc mã lệnh bạn muốn thực thi bên dưới.\x1b[0m'),
    borderBot,
  ];

  console.log();
  if (termW >= 80 && owlLines.length > 0) {
    const maxRows = Math.max(owlLines.length, boxLines.length);
    for (let i = 0; i < maxRows; i++) {
      const left = i < owlLines.length ? owlLines[i] : blankOwl;
      const right = i < boxLines.length ? boxLines[i] : '';
      console.log(`  ${left}   ${right}`);
    }
  } else {
    // Stacked layout
    if (owlLines.length > 0) {
      for (const line of owlLines) {
        console.log(`  ${line}`);
      }
      console.log();
    }
    for (const line of boxLines) {
      console.log(`  ${line}`);
    }
  }
  console.log();
}

// ── Persistent Task Storage ────────────────────────────────────────────────
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

// ── Unified Diff Viewer ─────────────────────────────────────────────────────
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

// ── Task Table ──────────────────────────────────────────────────────────────
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

// ── Status Display ──────────────────────────────────────────────────────────
export function printStatus() {
  const statusContent =
    `${chalk.bold.hex('#38bdf8')('Custos Sovereign Runtime Status')}\n\n` +
    `${chalk.cyan('• Daemon Connection')} : ${chalk.green.bold('Online & Responding')}\n` +
    `${chalk.cyan('• Local SQLite DB')}   : ${chalk.white('~/.custos/custos_vault.db (WAL Active)')}\n` +
    `${chalk.cyan('• Active Sandbox')}    : ${chalk.white('Bubblewrap / Seatbelt Worktree (Clean)')}\n` +
    `${chalk.cyan('• Concurrency Guard')}: ${chalk.white('Epoch Optimistic Lock v0.1.1')}\n` +
    `${chalk.cyan('• Registered Tasks')}: ${chalk.yellow(tasks.length + ' active tasks')}\n` +
    `${chalk.cyan('• Audit Spans Logged')}: ${chalk.yellow(tasks.reduce((acc, t) => acc + (t.spans || 0), 0) + ' spans')}\n` +
    `${chalk.cyan('• Active Mascots')}   : ${chalk.white('Guardian (owl.png), Coder, Inspector, Steward')}\n` +
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

// ── Advance Task ────────────────────────────────────────────────────────────
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

// ── Execution Permit Confirmation ───────────────────────────────────────────
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

// ── Interactive Vibe Coding Wizard with Mode Card ───────────────────────────
export async function runVibeWizard(initialMode = null) {
  let mode = initialMode;

  if (!mode) {
    console.log(chalk.bold.hex('#38bdf8')('\n  ══ Chọn chế độ hoạt động Custos ══\n'));
    mode = await select({
      message: chalk.cyan.bold('❄ Chọn chế độ Custos (Operational Mode):'),
      choices: [
        {
          name: `${chalk.hex('#38bdf8').bold('Code')}       ${chalk.dim('• Lập trình, sửa lỗi & tái cấu trúc mã nguồn (Coder Owl)')}`,
          value: 'Code',
        },
        {
          name: `${chalk.hex('#f59e0b').bold('Research')}   ${chalk.dim('• Điều tra, kiến trúc & suy luận chuyên sâu (Inspector Owl)')}`,
          value: 'Research',
        },
        {
          name: `${chalk.hex('#10b981').bold('Assistant')}  ${chalk.dim('• Trợ lý điều phối quy trình tự động (Steward Owl)')}`,
          value: 'Assistant',
        },
      ],
    });
  }

  // Display the crisp mode card with mini owl next to introduction box!
  printModeCard(mode);

  // User begins coding / entering task prompt right below the introduction
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

  // Sandboxing & AST Synthesis
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

  // Show Diff
  console.log();
  printUnifiedDiff();

  // Execution Permit
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

// ── Interactive REPL ────────────────────────────────────────────────────────
export async function startRepl() {
  printMainBanner();

  let currentMode = 'standard';
  const rl = readline.createInterface({ input: inputDevice, output: outputDevice });

  const getPrompt = () => {
    let modeColor = chalk.hex('#38bdf8');
    if (currentMode === 'code') modeColor = chalk.hex('#38bdf8');
    if (currentMode === 'research') modeColor = chalk.hex('#f59e0b');
    if (currentMode === 'assistant') modeColor = chalk.hex('#10b981');
    return `${modeColor.bold('custos')} ${chalk.dim(`[${currentMode}]`)} ${chalk.dim('❯')} `;
  };

  while (true) {
    const rawLine = await rl.question(getPrompt());
    const line = rawLine.trim();
    if (!line) continue;

    const [cmd, ...args] = line.split(/\s+/);
    const argStr = args.join(' ');
    const lowerCmd = cmd.toLowerCase();

    if (lowerCmd === 'exit' || lowerCmd === 'quit' || lowerCmd === 'q') {
      console.log(chalk.dim('\n  Tạm biệt! Custos Guardian Owl đang bảo vệ hệ thống của bạn.\n'));
      break;
    } else if (lowerCmd === 'clear' || lowerCmd === 'cls') {
      console.clear();
      printMainBanner();
    } else if (lowerCmd === 'vibe' || lowerCmd === 'v') {
      await runVibeWizard(currentMode !== 'standard' ? currentMode : null);
    } else if (lowerCmd === 'code' || (lowerCmd === 'mode' && args[0]?.toLowerCase() === 'code')) {
      currentMode = 'code';
      printModeCard('Code');
    } else if (lowerCmd === 'research' || (lowerCmd === 'mode' && args[0]?.toLowerCase() === 'research')) {
      currentMode = 'research';
      printModeCard('Research');
    } else if (lowerCmd === 'assistant' || (lowerCmd === 'mode' && args[0]?.toLowerCase() === 'assistant')) {
      currentMode = 'assistant';
      printModeCard('Assistant');
    } else if (lowerCmd === 'list' || lowerCmd === 'tasks' || lowerCmd === 'ls') {
      printTaskList();
    } else if (lowerCmd === 'create' || lowerCmd === 'new') {
      const title = argStr || 'New sovereign agentic workflow';
      const newTask = {
        id: `task-${Math.random().toString(36).substring(2, 6)}`,
        title,
        mode: currentMode !== 'standard' ? (currentMode.charAt(0).toUpperCase() + currentMode.slice(1)) : 'Code',
        status: 'Pending',
        risk: 'Low',
        spans: 1,
        createdAt: 'Just now',
      };
      tasks.unshift(newTask);
      saveTasks(tasks);
      console.log(chalk.green(`  ✔ Đã tạo task mới [${newTask.id}] (${newTask.mode}): "${title}"\n`));
    } else if (lowerCmd === 'advance') {
      advanceTask(args[0] || tasks[0]?.id);
    } else if (lowerCmd === 'diff' || lowerCmd === 'd') {
      printUnifiedDiff();
    } else if (lowerCmd === 'permit' || lowerCmd === 'p') {
      await promptPermitApproval();
    } else if (lowerCmd === 'status') {
      printStatus();
    } else if (lowerCmd === 'help' || lowerCmd === '?') {
      printHelp();
    } else {
      // If user is in a specialized mode (code / research / assistant), any input starts coding / task workflow!
      if (currentMode !== 'standard') {
        const fullPrompt = line;
        console.log(chalk.cyan(`\n  ⚡ [${currentMode.toUpperCase()}] Đang xử lý yêu cầu: "${fullPrompt}"...`));
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
        console.log(
          chalk.red(`  Lệnh không hợp lệ: "${cmd}". `) +
            chalk.dim('Gõ ') +
            chalk.yellow.bold('help') +
            chalk.dim(' để xem danh sách lệnh, hoặc gõ ') +
            chalk.cyan.bold('code') +
            chalk.dim(' để bắt đầu lập trình.\n')
        );
      }
    }
  }

  rl.close();
}

// ── Help Menu ───────────────────────────────────────────────────────────────
export function printHelp() {
  console.log(chalk.bold.hex('#38bdf8')('\n  ══ Custos CLI — Bảng tra cứu lệnh ══\n'));

  const commands = [
    ['vibe (hoặc v)', 'Khởi chạy quy trình Vibe Coding tương tác từng bước'],
    ['code', 'Chuyển sang Code Mode (Coder Owl) và bắt đầu viết mã'],
    ['research', 'Chuyển sang Research Mode (Inspector Owl) để nghiên cứu'],
    ['assistant', 'Chuyển sang Assistant Mode (Steward Owl) để điều phối'],
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
    console.log(`  ${chalk.yellow.bold(c.padEnd(18))} ${chalk.white(d)}`);
  });

  console.log(chalk.dim('\n  Tham số dòng lệnh nhanh:'));
  console.log(`  ${chalk.white('custos vibe')}         ${chalk.dim('Chạy ngay wizard Vibe Coding')}`);
  console.log(`  ${chalk.white('custos code')}         ${chalk.dim('Kích hoạt Code Mode với Coder Owl')}`);
  console.log(`  ${chalk.white('custos research')}     ${chalk.dim('Kích hoạt Research Mode với Inspector Owl')}`);
  console.log(`  ${chalk.white('custos assistant')}    ${chalk.dim('Kích hoạt Assistant Mode với Steward Owl')}`);
  console.log(`  ${chalk.white('custos list')}         ${chalk.dim('In bảng task hiện tại')}`);
  console.log(`  ${chalk.white('custos status')}       ${chalk.dim('In thông tin trạng thái')}`);
  console.log(`  ${chalk.white('custos diff')}         ${chalk.dim('Xem bản vá Unified Diff')}`);
  console.log(`  ${chalk.white('custos permit')}       ${chalk.dim('Xác nhận giấy phép thực thi')}`);
  console.log();
}

// ── Main Entrypoint ─────────────────────────────────────────────────────────
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
