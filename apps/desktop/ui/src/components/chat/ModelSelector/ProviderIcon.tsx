import React from 'react';
import { ProviderBrandIcon } from '@/components/common/AgentIcons';
import type { ProviderServiceId } from '@/types/provider_types';

export function ProviderIcon({
  provider,
  size = 14,
  className
}: {
  provider: ProviderServiceId;
  size?: number;
  className?: string;
}): React.JSX.Element {
  return <ProviderBrandIcon provider={provider} size={size} className={className} />;
}
