export type ViewMode = 'split' | 'chat' | 'diff';
export type MainTab = 'studio' | 'providers';

export interface ChatMessage {
  id?: string;
  role: 'user' | 'assistant';
  author: string;
  badge?: string;
  text: string;
  stepName?: string;
  duration?: string;
}

export interface DiffLine {
  type: 'context' | 'add' | 'del';
  text: string;
}

export interface Session {
  id: string;
  title: string;
  time: string;
  preview: string;
  model: string;
  fileName: string;
  diffHunk: string;
  diffLinesCount: string;
  summary: string;
  messages: ChatMessage[];
  diffCode: DiffLine[];
}

export interface ProjectData {
  [projectName: string]: Session[];
}

export interface ProviderItem {
  id: string;
  name: string;
  model: string;
  status: 'primary' | 'standby' | 'failover' | 'offline';
  statusLabel: string;
  badgeColor: string;
  apiKey: string;
  quotaUsed?: string;
  quotaTotal?: string;
  quotaPercent?: number;
  rateLimit?: string;
  vram?: string;
  endpoint?: string;
  latency: string;
  iconType: 'anthropic' | 'openai' | 'gemini' | 'deepseek';
}

export interface ClientApiKey {
  id: string;
  name: string;
  token: string;
  created: string;
  icon: 'laptop' | 'terminal';
}
