import chalk from 'chalk';
import ora from 'ora';
import { input } from '@inquirer/prompts';
import { stdout as outputDevice } from 'node:process';
import { daemonRequest, openExternalUrl } from './daemon.js';
import { promptCenteredPalette, promptModeSelection } from './terminal_ui.js';

export const PROVIDERS = [
  { id: 'openai', serviceType: 'openai', name: 'OpenAI Codex', manualCode: false },
  { id: 'anthropic', serviceType: 'anthropic', name: 'Claude', manualCode: true },
  { id: 'antigravity', serviceType: 'antigravity', name: 'Antigravity', manualCode: false },
];

export async function providerConnection(provider) {
  const connection = await daemonRequest('v1.oauth.get', { provider_id: provider.id });
  return connection?.connected ? { ...provider, connection } : null;
}

export async function loginProvider(provider) {
  const authorization = await daemonRequest('v1.oauth.authorize', {
    provider_id: provider.id,
    service_type: provider.serviceType,
  });
  if (!authorization?.authorization_url) {
    throw new Error(`Daemon không trả về authorization URL cho ${provider.name}.`);
  }

  openExternalUrl(authorization.authorization_url);
  outputDevice.write('\x1b[2J\x1b[H');
  console.log(chalk.cyan(`\n  Đã mở trang đăng nhập ${provider.name} trong trình duyệt.`));

  if (provider.manualCode) {
    const codeOrUrl = await input({
      message: chalk.white('Dán authorization code hoặc callback URL sau khi đăng nhập:'),
    });
    await daemonRequest('v1.oauth.exchange', {
      provider_id: provider.id,
      service_type: provider.serviceType,
      code_or_url: codeOrUrl.trim(),
      code_verifier: authorization.code_verifier,
    });
  } else {
    const spinner = ora(chalk.cyan(`Đang chờ ${provider.name} xác nhận đăng nhập...`)).start();
    const deadline = Date.now() + 180_000;
    let completed = false;
    while (Date.now() < deadline) {
      const status = await daemonRequest('v1.oauth.status');
      if (status?.status === 'completed') {
        completed = true;
        break;
      }
      if (status?.status === 'failed') {
        spinner.fail(chalk.red(status.error || 'Đăng nhập thất bại.'));
        throw new Error(status.error || `Đăng nhập ${provider.name} thất bại.`);
      }
      await new Promise((resolve) => setTimeout(resolve, 750));
    }
    if (!completed) {
      spinner.fail(chalk.red('Đăng nhập hết thời gian chờ.'));
      throw new Error(`Đăng nhập ${provider.name} hết thời gian chờ.`);
    }
    spinner.succeed(chalk.green(`Đã kết nối ${provider.name}.`));
  }

  const connected = await providerConnection(provider);
  if (!connected) throw new Error(`${provider.name} chưa có credential hợp lệ sau đăng nhập.`);
  return connected;
}

export function modelDisplayName(modelId, providerName = '') {
  const rawId = String(modelId || '').split('/').pop() || String(modelId || '');
  const id = rawId.toLowerCase();

  const claude = id.match(/^claude-(opus|sonnet|haiku)-(\d+)-(\d+)(.*)$/);
  if (claude) {
    const suffix = claude[4].split('-').filter(Boolean);
    const qualifiers = suffix.map((part) => part === 'thinking' ? 'Thinking' : part[0].toUpperCase() + part.slice(1));
    return `Claude ${claude[1][0].toUpperCase()}${claude[1].slice(1)} ${claude[2]}.${claude[3]}${qualifiers.length ? ` (${qualifiers.join(', ')})` : ''}`;
  }

  const gemini = id.match(/^gemini-(\d+(?:\.\d+)?)-(.+)$/);
  if (gemini) {
    const parts = gemini[2].split('-');
    const family = parts.shift();
    let name = `Gemini ${gemini[1]} ${family ? family[0].toUpperCase() + family.slice(1) : ''}`.trim();
    const qualifiers = [];
    for (let index = 0; index < parts.length; index += 1) {
      const part = parts[index];
      if (part === 'lite' || part === 'image') name += ` ${part[0].toUpperCase()}${part.slice(1)}`;
      else if (part === 'extra' && parts[index + 1] === 'low') {
        qualifiers.push('Extra Low');
        index += 1;
      } else qualifiers.push(part[0].toUpperCase() + part.slice(1));
    }
    return `${name}${qualifiers.length ? ` (${qualifiers.join(', ')})` : ''}`;
  }

  const gpt = id.match(/^gpt-(\d+(?:\.\d+)?)(?:-(.+))?$/);
  if (gpt) {
    const suffix = gpt[2]?.split('-').map((part) => part[0].toUpperCase() + part.slice(1)).join(' ');
    return `GPT-${gpt[1]}${suffix ? ` ${suffix}` : ''}`;
  }

  const openAiReasoning = id.match(/^(o\d+)(?:-(.+))?$/);
  if (openAiReasoning) {
    const suffix = openAiReasoning[2]?.split('-').map((part) => part[0].toUpperCase() + part.slice(1)).join(' ');
    return `${openAiReasoning[1].toUpperCase()}${suffix ? ` ${suffix}` : ''}`;
  }

  const words = rawId.split(/[-_:]+/).filter(Boolean).map((part) => {
    if (/^\d+(?:\.\d+)*$/.test(part)) return part;
    if (['ai', 'llm', 'vl', 'r1'].includes(part.toLowerCase())) return part.toUpperCase();
    return part[0].toUpperCase() + part.slice(1);
  });
  return words.join(' ') || providerName || 'Model';
}

export async function probeProviderModels(provider) {
  const response = await daemonRequest('v1.models.probe', {
    provider_id: provider.id,
    provider_type: provider.serviceType,
  });
  return (response?.models || [])
    .filter((model) => model?.id)
    .map((model) => ({
      ...model,
      providerName: model.name,
      name: modelDisplayName(model.id, provider.name),
    }));
}

export async function promptProviderModelSelection(currentSelection = null) {
  const states = await Promise.all(PROVIDERS.map(async (provider) => {
    try {
      return await providerConnection(provider);
    } catch {
      return null;
    }
  }));

  const provider = await promptCenteredPalette(
    PROVIDERS.map((item, index) => ({
      name: item.name,
      description: states[index] ? 'Đã đăng nhập' : 'Chưa đăng nhập · Enter để kết nối',
      value: { item, connection: states[index] },
    })),
    'Chọn provider',
  );
  if (!provider) return currentSelection;

  const connected = provider.connection || await loginProvider(provider.item);
  const spinner = ora(chalk.cyan(`Đang tải model từ ${connected.name}...`)).start();
  let models;
  try {
    models = await probeProviderModels(connected);
    spinner.stop();
  } catch (error) {
    spinner.fail(chalk.red(`Không tải được model: ${error.message}`));
    throw error;
  }
  if (models.length === 0) throw new Error(`${connected.name} không trả về model khả dụng.`);

  const model = await promptCenteredPalette(models.map((item) => ({
    name: item.name || item.id,
    description: item.description || item.id,
    value: item,
  })), `Model · ${connected.name}`);
  if (!model) return currentSelection;
  return { provider: connected, model };
}

export async function promptModeOnly(currentMode, currentSelection) {
  const mode = await promptModeSelection();
  if (!mode) return { mode: currentMode, selection: currentSelection };
  return { mode, selection: currentSelection };
}

export async function promptProviderLogout(currentSelection = null) {
  const connectedProviders = (await Promise.all(PROVIDERS.map(async (provider) => {
    try {
      return await providerConnection(provider);
    } catch {
      return null;
    }
  }))).filter(Boolean);

  if (connectedProviders.length === 0) {
    throw new Error('Chưa có provider nào đang đăng nhập.');
  }

  const provider = await promptCenteredPalette(connectedProviders.map((item) => ({
    name: item.name,
    description: 'Đã đăng nhập · Enter để đăng xuất',
    value: item,
  })), 'Đăng xuất provider');
  if (!provider) return { selection: currentSelection, provider: null };

  await daemonRequest('v1.oauth.delete', { provider_id: provider.id });
  return {
    selection: currentSelection?.provider?.id === provider.id ? null : currentSelection,
    provider,
  };
}

