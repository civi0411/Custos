export const isReadMethod = (method: string): boolean => {
  const m = method.toLowerCase();
  return (
    m === 'v1.ping' ||
    m === 'v1.health' ||
    m.endsWith('.get') ||
    m.endsWith('.list') ||
    m.endsWith('.read') ||
    m.endsWith('.tree') ||
    m.endsWith('.diff') ||
    m.endsWith('.status') ||
    m.endsWith('.history') ||
    m.endsWith('.inspect_dirty') ||
    m.endsWith('.detect') ||
    m.endsWith('.pick_folder') ||
    m.endsWith('.compare') ||
    m.endsWith('.pricing') ||
    m.endsWith('.probe') ||
    m.endsWith('.catalog') ||
    m.endsWith('.snapshot') ||
    m.endsWith('.screenshot')
  );
};

/**
 * Endpoint semantics: Conservative check for methods that are strictly idempotent
 * and safe to retry automatically on network blip without side effects or UI prompts.
 * Excludes `.probe`, `.detect`, `.compare`, `.pick_folder` and mutation commands.
 */
export const isRetrySafeMethod = (method: string): boolean => {
  const m = method.toLowerCase();
  return (
    m === 'v1.ping' ||
    m === 'v1.health' ||
    m.endsWith('.get') ||
    m.endsWith('.list') ||
    m.endsWith('.read') ||
    m.endsWith('.tree') ||
    m.endsWith('.diff') ||
    m.endsWith('.status') ||
    m.endsWith('.history') ||
    m.endsWith('.inspect_dirty') ||
    m.endsWith('.pricing') ||
    m.endsWith('.catalog') ||
    m.endsWith('.snapshot') ||
    m.endsWith('.screenshot')
  );
};

export const isTransportError = (err: unknown): boolean => {
  const errMsg = String((err as any)?.message || err || '');
  return (
    errMsg.includes('failed to connect') ||
    errMsg.includes('channel closed') ||
    errMsg.includes('Broken pipe') ||
    errMsg.includes('NetworkError') ||
    errMsg.includes('IPC') ||
    errMsg.includes('Failed to fetch') ||
    errMsg.includes('Load failed')
  );
};

export const generateCommandId = (): string => {
  return typeof crypto !== 'undefined' && crypto.randomUUID
    ? `cmd_${crypto.randomUUID()}`
    : `cmd_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
};

export function prepareRequestParams(method: string, params: Record<string, any> = {}): Record<string, any> {
  const finalParams = { ...params };
  if (!isReadMethod(method) && !finalParams.command_id) {
    finalParams.command_id = generateCommandId();
  }
  return finalParams;
}

export interface RequestRetryDecision {
  shouldRetry: boolean;
  preserveCommandId: boolean;
  reason: string;
}

export function evaluateRetry(method: string, params: Record<string, any>, error: unknown): RequestRetryDecision {
  const errMsg = String((error as any)?.message || error || '');
  // Business rejections (Validation, NotFound, Conflict, Forbidden) must NEVER retry
  if (!isTransportError(error) && errMsg.length > 0 && !errMsg.includes('timeout')) {
    return { shouldRetry: false, preserveCommandId: false, reason: 'business_rejection' };
  }
  // Pure idempotent read methods can always be retried
  if (isRetrySafeMethod(method)) {
    return { shouldRetry: true, preserveCommandId: false, reason: 'idempotent_read' };
  }
  // If it's a mutation with a stable command_id, retry is permissible while preserving the exact command_id
  if (params.command_id) {
    return { shouldRetry: true, preserveCommandId: true, reason: 'idempotent_command_id' };
  }
  // Conservative no-retry for unknown or effectful methods
  return { shouldRetry: false, preserveCommandId: false, reason: 'effectful_no_retry' };
}
