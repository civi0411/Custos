import { Bot } from 'lucide-react';
import type { WorkbenchContribution } from '@/types/workbench';

export const copilotContribution: WorkbenchContribution = {
  id: 'copilot',
  canonicalId: 'assistant',
  label: 'Copilot',
  icon: Bot,
  shortcut: '⌥3',
  builtin: true,
  color: '#34d399',
  description: 'Personal agenda triage, communication drafting, and revocable grant automation.',
  supportedTabTypes: ['drafts', 'outbox', 'contacts', 'calendar', 'notes'],
  panes: [
    { id: 'conversation', title: 'Conversation', position: 'main', defaultWidth: 'full' },
    { id: 'drafts', title: 'Drafts', position: 'side', defaultWidth: 'third' },
    { id: 'outbox', title: 'Outbox', position: 'side', defaultWidth: 'third' },
    { id: 'contacts', title: 'Contacts & Calendar', position: 'side', defaultWidth: 'third' },
  ],
};
