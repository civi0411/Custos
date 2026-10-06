import React from 'react';
import { MessageCircleQuestion } from 'lucide-react';
import { cn } from '@/lib/utils';

type AgentQuestionIconProps = React.ComponentProps<typeof MessageCircleQuestion>;

export function AgentQuestionIcon({
  className,
  ...props
}: AgentQuestionIconProps): React.JSX.Element {
  return (
    <MessageCircleQuestion
      {...props}
      className={cn('text-amber-400', className)}
      aria-hidden="true"
    />
  );
}
