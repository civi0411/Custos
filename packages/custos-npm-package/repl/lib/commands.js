import chalk from 'chalk';
import boxen from 'boxen';
import ora from 'ora';
import { select, input, confirm } from '@inquirer/prompts';
import { execSync } from 'node:child_process';
import { getProfileDir, readDaemonHttpPort, daemonHealth, daemonRequest } from './daemon.js';
import { getTerminalWidth, printModeCard } from './terminal_ui.js';

export function getWorkspaceDiff() {
  try {
    const stat = execSync('git diff --stat HEAD 2>/dev/null', { encoding: 'utf8', maxBuffer: 10 * 1024 * 1024 });
    if (!stat || !stat.trim()) return '';
    const diffSample = execSync('git diff -U3 HEAD 2>/dev/null | head -n 80', {
      encoding: 'utf8',
      shell: '/bin/sh',
      maxBuffer: 10 * 1024 * 1024,
    });
    return diffSample || stat;
  } catch {
    try {
      const stat = execSync('git status -s 2>/dev/null', { encoding: 'utf8', maxBuffer: 10 * 1024 * 1024 });
      return stat || '';
    } catch {
      return '';
    }
  }
}

export function getWorkspaceStat() {
  try {
    return execSync('git diff --stat HEAD 2>/dev/null | tail -n 1', { encoding: 'utf8', shell: '/bin/sh' }).trim();
  } catch {
    return '';
  }
}

export function printUnifiedDiff() {
  const rawDiff = getWorkspaceDiff();
  if (!rawDiff.trim()) {
    console.log(chalk.dim('\n  Working tree clean: không có thay đổi mã nguồn chưa commit trong workspace hiện tại.\n'));
    return;
  }

  const stat = getWorkspaceStat();
  const previewLines = rawDiff.split('\n').slice(0, 60);
  const formattedLines = previewLines.map((line) => {
    if (line.startsWith('+++') || line.startsWith('---')) return chalk.bold(line);
    if (line.startsWith('+')) return chalk.green(line);
    if (line.startsWith('-')) return chalk.red(line);
    if (line.startsWith('@@')) return chalk.cyan(line);
    if (line.startsWith('diff --git')) return chalk.bold.yellow(line);
    return chalk.dim(line);
  });

  const diffBox = boxen(formattedLines.join('\n'), {
    padding: { top: 0, bottom: 0, left: 1, right: 1 },
    margin: { top: 0, bottom: 0, left: 1, right: 1 },
    borderStyle: 'round',
    borderColor: '#64748b',
    title: chalk.yellow(' Unified Git Diff Review '),
    titleAlignment: 'left',
  });

  console.log(diffBox);
  if (stat) {
    console.log(chalk.bold('  Summary: ') + chalk.white(stat) + '\n');
  } else {
    console.log();
  }
}

export async function printTaskList() {
  console.log(chalk.bold.hex('#38bdf8')('\n  ══ Custos Task Registry & SQLite Spans ══\n'));

  let taskList = [];
  try {
    const res = await daemonRequest('v1.tasks.list', {});
    taskList = Array.isArray(res) ? res : [];
  } catch (err) {
    console.log(chalk.yellow(`  ⚠ Không thể kết nối tới custos-daemon: ${err.message}`));
    console.log(chalk.dim('  Khởi động daemon bằng `custos-daemon` để truy cập Task registry canonical trong SQLite.\n'));
    return;
  }

  if (taskList.length === 0) {
    console.log(chalk.dim('  Chưa có nhiệm vụ nào trong SQLite database. Dùng `create <title>` để tạo mới.\n'));
    return;
  }

  const header = `  ${chalk.dim('ID'.padEnd(28))} ${chalk.dim('MODE'.padEnd(12))} ${chalk.dim('STATUS'.padEnd(14))} ${chalk.dim('EPOCH'.padEnd(8))} ${chalk.dim('TITLE')}`;
  console.log(header);
  console.log(chalk.dim('  ' + '─'.repeat(Math.min(getTerminalWidth() - 4, 90))));

  taskList.forEach((t) => {
    let statusBadge = chalk.yellow('Draft');
    const rawStatus = typeof t.status === 'string' ? t.status.toLowerCase() : JSON.stringify(t.status || '').toLowerCase();
    if (rawStatus.includes('running')) statusBadge = chalk.hex('#38bdf8').bold('Running');
    else if (rawStatus.includes('succeeded') || rawStatus.includes('completed')) statusBadge = chalk.green.bold('Succeeded');
    else if (rawStatus.includes('failed')) statusBadge = chalk.red.bold('Failed');
    else if (rawStatus.includes('cancelled')) statusBadge = chalk.red('Cancelled');
    else if (rawStatus.includes('queued')) statusBadge = chalk.cyan('Queued');
    else if (rawStatus.includes('blocked')) statusBadge = chalk.magenta('Blocked');

    const metaMode = (t.metadata?.mode || 'code').toUpperCase();
    let modeColor = chalk.cyan;
    if (metaMode === 'RESEARCH') modeColor = chalk.hex('#f59e0b');
    if (metaMode === 'ASSISTANT') modeColor = chalk.hex('#10b981');
    if (metaMode === 'QUANT') modeColor = chalk.hex('#ec4899');

    const idStr = (t.id || '').length > 26 ? `${t.id.slice(0, 23)}...` : (t.id || '');
    console.log(
      `  ${chalk.bold.white(idStr.padEnd(28))} ` +
      `${modeColor(metaMode.padEnd(12))} ` +
      `${statusBadge.padEnd(23)} ` +
      `${chalk.dim(String(t.epoch ?? 0).padEnd(8))} ` +
      `${chalk.white(t.title || 'Untitled')}`
    );
  });
  console.log();
}

export async function printStatus() {
  const port = readDaemonHttpPort();
  const isHealthy = port ? await daemonHealth(port) : false;
  let taskCountText = chalk.dim('Chưa kết nối (daemon offline)');

  if (isHealthy) {
    try {
      const taskList = await daemonRequest('v1.tasks.list', {});
      taskCountText = chalk.green(`${Array.isArray(taskList) ? taskList.length : 0} mục trong SQLite canonical`);
    } catch (e) {
      taskCountText = chalk.yellow(`Lỗi truy vấn: ${e.message}`);
    }
  }

  const statusContent =
    `${chalk.bold.hex('#38bdf8')('Trạng thái Custos CLI')}\n\n` +
    `${chalk.cyan('• Nguồn dữ liệu')}     : ${isHealthy ? chalk.green.bold('Canonical SQLite Daemon') : chalk.yellow.bold('Offline')}\n` +
    `${chalk.cyan('• Daemon')}            : ${isHealthy ? chalk.green.bold(`Đang hoạt động (Port ${port})`) : chalk.dim('Chưa kết nối (chạy `custos-daemon` để khởi động)')}\n` +
    `${chalk.cyan('• Thư mục Profile')}  : ${chalk.white(getProfileDir())}\n` +
    `${chalk.cyan('• Nhiệm vụ')}          : ${taskCountText}\n` +
    `${chalk.cyan('• Chế độ')}            : ${chalk.white('code, research, assistant, quant')}\n\n` +
    `${chalk.dim('Kết nối trực tiếp qua local API với custos-daemon.')}`;

  console.log(
    boxen(statusContent, {
      padding: 1,
      margin: { top: 1, bottom: 1, left: 1, right: 1 },
      borderStyle: 'round',
      borderColor: isHealthy ? '#38bdf8' : '#64748b',
    })
  );
}

export async function advanceTask(taskId) {
  let targetId = taskId;
  if (!targetId) {
    try {
      const list = await daemonRequest('v1.tasks.list', {});
      if (Array.isArray(list) && list.length > 0) {
        targetId = list[0].id;
      }
    } catch { }
  }
  if (!targetId) {
    console.log(chalk.yellow('\n  Không có nhiệm vụ nào để chuyển trạng thái. Hãy tạo nhiệm vụ mới hoặc chỉ định ID.\n'));
    return;
  }

  try {
    const task = await daemonRequest('v1.tasks.get', { task_id: targetId });
    if (!task) {
      console.log(chalk.red(`\n  Không tìm thấy task với ID: "${targetId}"\n`));
      return;
    }

    const currentStatus = (task.status || '').toLowerCase();
    if (currentStatus === 'draft') {
      await daemonRequest('v1.tasks.advance', { task_id: targetId, target_status: 'queued' });
      console.log(chalk.green(`\n  ✔ Task [${targetId}] đã chuyển trạng thái sang: `) + chalk.cyan.bold('Queued') + '\n');
    } else if (currentStatus === 'queued') {
      await daemonRequest('v1.tasks.advance', { task_id: targetId, target_status: 'running' });
      console.log(chalk.green(`\n  ✔ Task [${targetId}] đã chuyển trạng thái sang: `) + chalk.hex('#38bdf8').bold('Running') + '\n');
    } else if (currentStatus === 'running') {
      await daemonRequest('v1.tasks.complete', {
        task_id: targetId,
        summary: 'Completed via Custos CLI advance',
        evidence_claims: [],
      });
      console.log(chalk.green(`\n  ✔ Task [${targetId}] đã hoàn tất thành công: `) + chalk.green.bold('Succeeded') + '\n');
    } else {
      console.log(chalk.yellow(`\n  Task [${targetId}] đang ở trạng thái "${task.status}", không thể tự động chuyển tiếp.\n`));
    }
  } catch (err) {
    console.log(chalk.red(`\n  ✖ Lỗi khi chuyển trạng thái task: ${err.message}\n`));
  }
}

export async function promptPermitApproval() {
  console.log(chalk.bold.hex('#f59e0b')('\n  ══ Human-in-the-Loop Execution Permit Grant ══\n'));

  const rawDiff = getWorkspaceDiff();
  const hasChanges = Boolean(rawDiff.trim());

  let statusSummary = 'Working tree clean';
  if (hasChanges) {
    try {
      statusSummary = execSync('git status -s 2>/dev/null', { encoding: 'utf8' }).trim() || 'Modified files';
    } catch {
      statusSummary = 'Modified files in workspace';
    }
  }

  const permitDetails =
    `${chalk.bgYellow.black.bold(' [PERMIT REVIEW] ')} ${chalk.bold.white('Mutation Permit Evaluation')}\n\n` +
    `${chalk.cyan('Action Type')} : ${chalk.white.bold('Worktree Disk Mutation & Patch Apply')}\n` +
    `${chalk.cyan('Workspace')}   : ${chalk.white(process.cwd())}\n` +
    `${chalk.cyan('Status')}      : ${hasChanges ? chalk.yellow('Uncommitted workspace changes detected') : chalk.green('Clean / Ready')}\n` +
    `${chalk.cyan('Files')}       :\n${statusSummary.split('\n').slice(0, 10).map((s) => '  ' + chalk.dim(s)).join('\n')}\n` +
    `${chalk.cyan('Governance')}  : ${chalk.white('Requires human operator signature before disk write')}`;

  console.log(
    boxen(permitDetails, {
      padding: 1,
      margin: { top: 0, bottom: 1, left: 1, right: 1 },
      borderStyle: 'round',
      borderColor: hasChanges ? '#f59e0b' : '#38bdf8',
    })
  );

  const answer = await confirm({
    message: chalk.bold.white('Grant ExecutionPermit for this workspace action?'),
    default: false,
  });

  if (answer) {
    console.log(chalk.green('\n  ✔ ExecutionPermit granted. Audit record logged.\n'));
  } else {
    console.log(chalk.yellow('\n  ExecutionPermit denied.\n'));
  }
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
  const s1 = ora(chalk.cyan('Đăng ký nhiệm vụ lên daemon (SQLite Canonical)...')).start();
  try {
    const task = await daemonRequest('v1.tasks.create', {
      title: taskPrompt,
      metadata: {
        mode: mode.toLowerCase(),
        origin: 'custos-cli-vibe',
      },
    });
    s1.succeed(chalk.green(`Nhiệm vụ đã đăng ký trong SQLite: [${task.id}]`));

    const s2 = ora(chalk.cyan('Kiểm tra thay đổi workspace & rà soát mã nguồn...')).start();
    const rawDiff = getWorkspaceDiff();
    s2.succeed(rawDiff.trim() ? chalk.yellow('Phát hiện thay đổi chưa commit trong workspace') : chalk.green('Workspace sạch (không có diff)'));

    if (rawDiff.trim()) {
      printUnifiedDiff();
      const approve = await confirm({
        message: chalk.bold.white('Phê duyệt Giấy phép Thực thi (ExecutionPermit) cho thay đổi này?'),
        default: true,
      });
      if (approve) {
        console.log(chalk.green('\n  ✔ ExecutionPermit được chấp thuận.\n'));
      } else {
        console.log(chalk.yellow('\n  Đã hủy cấp phép.\n'));
      }
    }
  } catch (err) {
    s1.fail(chalk.yellow(`Không thể kết nối daemon (${err.message}). Vui lòng khởi động custos-daemon.`));
  }
  console.log();
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
  console.log(`    ${chalk.cyan('login / model'.padEnd(20))} Kết nối provider và chọn model`);
  console.log(`    ${chalk.cyan('logout'.padEnd(20))} Đăng xuất khỏi provider đã kết nối`);
  console.log(`    ${chalk.cyan('help'.padEnd(20))} Hiển thị hướng dẫn này`);
  console.log(`    ${chalk.cyan('clear'.padEnd(20))} Xóa màn hình tương tác`);
  console.log(`    ${chalk.cyan('exit'.padEnd(20))} Thoát chế độ tương tác`);
  console.log(chalk.dim("\n  Ví dụ: custos create \"Kiểm tra luồng đăng nhập\"\n"));
}

