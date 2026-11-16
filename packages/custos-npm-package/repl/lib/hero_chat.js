import chalk from 'chalk';
import { stdin as inputDevice, stdout as outputDevice } from 'node:process';
import { VERSION } from './daemon.js';
import { stripAnsi, padToWidth, wrapString, renderTerminalMarkdown } from './markdown.js';
import {
  getTerminalWidth,
  BANNER_ART_RAW,
  BANNER_WIDTH,
  getBannerLogoLines,
  getAgentTheme,
  shutdownCli,
} from './terminal_ui.js';

export function renderHeroFrame({
  currentInput = '',
  cursorPos = 0,
  currentMode = 'custos',
  chatTurns = [],
  scrollOffset = 0,
  scrollState,
  selectedProvider,
  selectedModel,
  activity,
}) {
  const termW = getTerminalWidth();
  const termH = process.stdout.rows || 30;

  const agentTheme = getAgentTheme(currentMode);
  const modeBadge = agentTheme.name;
  const agentColor = agentTheme.color;
  const rawModelName = selectedModel?.name || selectedModel?.id || (currentMode === 'custos' ? 'Local lightweight' : 'Chưa chọn model');
  const modelName = rawModelName.length > 28 ? `${rawModelName.slice(0, 27)}…` : rawModelName;
  const rawEngineName = selectedProvider?.name || (currentMode === 'custos' ? 'Không cần đăng nhập' : 'Provider');
  const engineName = rawEngineName.length > 16 ? `${rawEngineName.slice(0, 15)}…` : rawEngineName;

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
      renderedInputChars += i === cursorPos
        ? chalk.bgWhite.black(currentInput[i])
        : chalk.rgb(240, 240, 240)(currentInput[i]);
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
    const totalLines = BANNER_ART_RAW.length + 3 + 5 + 3;
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
    output.push(
      boxIndentStr +
      chalk.rgb(115, 115, 115)(`${modeBadge} · ${engineName} · nhấn '/' để mở lệnh hoặc nhập yêu cầu bên dưới`) +
      '\x1b[K',
    );
    output.push('\x1b[K');
  } else {
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

      const renderedResponse = renderTerminalMarkdown(turn.responseLines || [], maxTextW, turnTheme.color);
      for (const responseLine of renderedResponse) {
        allHistoryLines.push(boxIndentStr + '  ' + padToWidth(responseLine, innerW - 1));
      }

      if (turn.meta) {
        allHistoryLines.push(boxIndentStr + '  ' + chalk.rgb(105, 105, 105)(padToWidth(turn.meta, innerW - 1)));
      }
      allHistoryLines.push('');
    }

    const promptRows = activity ? 6 : 5;
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
  if (activity) {
    const railWidth = 10;
    const position = activity.frame % (railWidth * 2 - 2);
    const cursor = position < railWidth ? position : railWidth * 2 - 2 - position;
    const rail = Array.from({ length: railWidth }, (_, index) => index === cursor ? '■' : '·').join('');
    output.push(
      boxIndentStr +
      chalk.hex(agentColor)(rail) +
      '  ' +
      chalk.white.bold(activity.label || 'agent đang chạy') +
      '  ' +
      chalk.rgb(115, 115, 115)('esc interrupt') +
      '\x1b[K',
    );
  }
  output.push(lineShortcuts + '\x1b[K');

  const currentTotal = output.length;
  for (let i = currentTotal; i < termH; i++) {
    output.push('\x1b[K');
  }

  return output.join('\r\n');
}

export function promptInteractiveChat({ currentMode, history, chatTurns = [], selectedProvider, selectedModel }) {
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
        selectedProvider,
        selectedModel,
      });
      scrollOffset = scrollState.effectiveScroll;
      outputDevice.write('\x1b[?25l\x1b[H' + frame);
    };

    const onResize = () => render();
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
        shutdownCli(0);
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

      if (key?.name === 'pageup') {
        const pageSize = Math.max(3, Math.floor((process.stdout.rows || 30) / 2));
        scrollOffset += pageSize;
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
        scrollOffset += 1;
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
        } else {
          cursorPos = 0;
        }
        render();
        return;
      }

      if (key?.name === 'end') {
        if (scrollOffset > 0) {
          scrollOffset = 0;
        } else {
          cursorPos = inputStr.length;
        }
        render();
        return;
      }

      if (key?.name === 'escape' && scrollOffset > 0) {
        scrollOffset = 0;
        render();
        return;
      }

      if (key?.name === 'up') {
        if (scrollOffset > 0) {
          scrollOffset += 1;
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

