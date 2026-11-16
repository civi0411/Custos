import chalk from 'chalk';

export function stripAnsi(str) {
  return (str || '').replace(/\x1b\[[0-9;?]*[a-zA-Z]/g, '');
}

export function padToWidth(content, width) {
  return `${content}${' '.repeat(Math.max(0, width - stripAnsi(content).length))}`;
}

export function wrapString(str, maxWidth) {
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

export function cleanMarkdownInline(text) {
  return String(text || '')
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '$1 ($2)')
    .replace(/\*\*([^*]+)\*\*/g, '$1')
    .replace(/__([^_]+)__/g, '$1')
    .replace(/~~([^~]+)~~/g, '$1')
    .replace(/`([^`]+)`/g, '$1')
    .replace(/(^|\s)[*_]([^*_]+)[*_](?=\s|$)/g, '$1$2')
    .replace(/\\([\\`*_{}\[\]()#+\-.!>])/g, '$1');
}

export function renderTerminalMarkdown(sourceLines, maxWidth, accentColor) {
  const rendered = [];
  let inCode = false;

  for (const sourceLine of sourceLines || []) {
    const raw = String(sourceLine || '');
    const fence = raw.match(/^\s*```\s*([^`]*)$/);
    if (fence) {
      if (!inCode) {
        const language = fence[1].trim();
        rendered.push(chalk.rgb(82, 82, 82)(`┌${'─'.repeat(Math.max(1, maxWidth - 1))}`));
        if (language) rendered.push(chalk.rgb(125, 125, 125)(`  ${language}`));
      } else {
        rendered.push(chalk.rgb(82, 82, 82)(`└${'─'.repeat(Math.max(1, maxWidth - 1))}`));
      }
      inCode = !inCode;
      continue;
    }

    if (inCode) {
      for (const line of wrapString(raw, Math.max(8, maxWidth - 2))) {
        rendered.push(chalk.bgRgb(22, 22, 22).rgb(205, 205, 205)(`  ${line}`));
      }
      continue;
    }

    if (/^\s*([-*_])(?:\s*\1){2,}\s*$/.test(raw)) {
      rendered.push(chalk.rgb(75, 75, 75)('─'.repeat(maxWidth)));
      continue;
    }

    const heading = raw.match(/^\s{0,3}#{1,6}\s+(.+)$/);
    if (heading) {
      for (const line of wrapString(cleanMarkdownInline(heading[1]), maxWidth)) {
        rendered.push(chalk.hex(accentColor).bold(line));
      }
      continue;
    }

    const bullet = raw.match(/^\s*[-*+]\s+(.+)$/);
    if (bullet) {
      const lines = wrapString(cleanMarkdownInline(bullet[1]), Math.max(8, maxWidth - 2));
      lines.forEach((line, index) => rendered.push(
        `${index === 0 ? chalk.hex(accentColor)('•') : ' '} ${chalk.rgb(215, 215, 215)(line)}`,
      ));
      continue;
    }

    const ordered = raw.match(/^\s*(\d+)[.)]\s+(.+)$/);
    if (ordered) {
      const prefix = `${ordered[1]}.`;
      const lines = wrapString(cleanMarkdownInline(ordered[2]), Math.max(8, maxWidth - prefix.length - 1));
      lines.forEach((line, index) => rendered.push(
        `${index === 0 ? chalk.hex(accentColor)(prefix) : ' '.repeat(prefix.length)} ${chalk.rgb(215, 215, 215)(line)}`,
      ));
      continue;
    }

    const quote = raw.match(/^\s*>\s?(.*)$/);
    if (quote) {
      for (const line of wrapString(cleanMarkdownInline(quote[1]), Math.max(8, maxWidth - 2))) {
        rendered.push(`${chalk.hex(accentColor)('│')} ${chalk.italic.rgb(175, 175, 175)(line)}`);
      }
      continue;
    }

    if (/^\s*\|?(?:\s*:?-+:?\s*\|)+\s*$/.test(raw)) continue;
    const tableText = raw.includes('|')
      ? raw.replace(/^\s*\|?|\|?\s*$/g, '').split('|').map((cell) => cleanMarkdownInline(cell.trim())).join(' │ ')
      : cleanMarkdownInline(raw);
    for (const line of wrapString(tableText, maxWidth)) {
      rendered.push(chalk.rgb(215, 215, 215)(line));
    }
  }

  if (inCode) rendered.push(chalk.rgb(82, 82, 82)(`└${'─'.repeat(Math.max(1, maxWidth - 1))}`));
  return rendered;
}

