import type { ConnectedAccount } from '@/types';
import {
  type ModelOption,
  getProviderInfo,
  normalizeModelPresentation,
} from './types';

export function getConnectedModelOptions(accounts: ConnectedAccount[]): ModelOption[] {
  const seen = new Set<string>();
  const options: ModelOption[] = [];

  for (const account of accounts) {
    if (account.status !== 'active' && account.statusLabel !== 'Connected') continue;

    const modelIds = (account.supportedModels && account.supportedModels.length > 0)
      ? account.supportedModels
      : account.catalogModels?.map((m) => m.id) || [];

    for (const id of modelIds) {
      const key = `${account.id}:${id}`;
      if (seen.has(key)) continue;
      seen.add(key);

      const metadata = account.catalogModels?.find((m) => m.id === id);
      const info = getProviderInfo(id);
      // A route namespace identifies the serving connection, regardless of model family.
      const provider = info.provider === 'copilot' ? 'copilot' : (account.providerId as ModelOption['provider']) || info.provider;
      const providerName = provider === 'copilot' ? 'GitHub Copilot' : account.providerName || info.label;

      const idLower = id.toLowerCase();
      const efforts = metadata?.efforts || [];
      const presentation = normalizeModelPresentation(id, metadata?.label, provider);
      const isReasoning = efforts.length > 0 || Boolean(presentation.variantEffort);

      let isLeavingSoon = false;
      let notice: string | undefined = undefined;

      if (idLower.includes('3.6') || idLower.includes('3.7') || idLower.includes('3.1-pro')) {
        isLeavingSoon = true;
      }
      if (
        idLower.includes('sonnet-4') ||
        idLower.includes('sonnet-5') ||
        idLower.includes('opus-4') ||
        idLower.includes('opus-5') ||
        idLower.includes('gpt-oss-120b')
      ) {
        notice = 'Notice ⓘ';
      }

      options.push({
        accountId: account.id,
        id,
        name: presentation.name,
        familyKey: JSON.stringify([(provider === 'antigravity' || provider === 'gemini') ? 'antigravity' : provider, presentation.family]),
        variantEffort: presentation.variantEffort,
        provider,
        providerName,
        badge: account.catalogSource === 'cached' ? 'Cached' : 'Live',
        notice,
        isLeavingSoon,
        description: metadata?.description || `Account: ${account.accountName}`,
        contextWindow: metadata?.context_window ? `${Math.round(metadata.context_window / 1000)}k tokens` : undefined,
        isConfigured: account.status === 'active',
        efforts,
        defaultEffort: metadata?.default_effort,
        supportsThinking: isReasoning,
      });
    }
  }

  return options;
}
