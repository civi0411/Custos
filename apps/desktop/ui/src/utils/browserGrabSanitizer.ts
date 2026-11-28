/**
 * Trust-boundary sanitizer for BrowserWorkbenchPane "Pick Element" grabs.
 * Strips URL query/hash tokens, enforces an attribute allowlist, redacts
 * secret patterns, and hard-truncates payloads before attaching to agent prompts.
 */

export const GrabSanitizePolicy = {
  allowedAttributes: new Set([
    'id',
    'class',
    'role',
    'name',
    'type',
    'href',
  ]),
  maxTextLength: 500,
  maxAttrLength: 200,
  maxUrlLength: 512,
  maxAttributes: 12,
} as const;

export interface RawBrowserGrabPayload {
  url: string;
  tagName?: string;
  selector?: string;
  textContent?: string;
  attributes?: Record<string, string>;
  note?: string;
}

export interface SanitizedBrowserGrabPayload {
  url: string;
  tagName: string;
  selector?: string;
  textContent: string;
  attributes: Record<string, string>;
  note?: string;
}

const SECRET_PATTERNS: RegExp[] = [
  /Bearer\s+[A-Za-z0-9\-._~+/]+=*/gi,
  /\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b/g,
  /\bsk-[A-Za-z0-9]{16,}\b/g,
  /\bgh[pousr]_[A-Za-z0-9]{20,}\b/g,
  /\bxox[baprs]-[A-Za-z0-9-]{10,}\b/g,
  /\bAKIA[0-9A-Z]{16}\b/g,
  /(password|token|secret|api_key|apikey)\s*[=:]\s*["']?[^\s"'&;]{6,}/gi,
];

function truncate(value: string, maxLen: number): string {
  const trimmed = value.trim();
  return trimmed.length > maxLen ? `${trimmed.slice(0, maxLen)}…` : trimmed;
}

export function redactSecrets(input: string): string {
  let out = input;
  for (const pattern of SECRET_PATTERNS) {
    out = out.replace(pattern, '[REDACTED]');
  }
  return out;
}

/** Strips query strings (?...) and fragments (#...) so session tokens never leak. */
export function sanitizeGrabUrl(rawUrl: string): string {
  if (!rawUrl || rawUrl === 'about:blank') return 'about:blank';
  try {
    const parsed = new URL(rawUrl);
    parsed.search = '';
    parsed.hash = '';
    return truncate(redactSecrets(parsed.toString()), GrabSanitizePolicy.maxUrlLength);
  } catch {
    const clean = rawUrl.split(/[?#]/)[0] || '';
    return truncate(redactSecrets(clean), GrabSanitizePolicy.maxUrlLength);
  }
}

export function sanitizeGrabPayload(raw: RawBrowserGrabPayload): SanitizedBrowserGrabPayload {
  const attributes: Record<string, string> = {};
  if (raw.attributes) {
    let count = 0;
    for (const [key, val] of Object.entries(raw.attributes)) {
      if (count >= GrabSanitizePolicy.maxAttributes) break;
      const lower = key.toLowerCase();
      if (!GrabSanitizePolicy.allowedAttributes.has(lower) && !lower.startsWith('aria-')) {
        continue;
      }
      const cleaned =
        lower === 'href'
          ? sanitizeGrabUrl(val)
          : truncate(redactSecrets(String(val)), GrabSanitizePolicy.maxAttrLength);
      if (cleaned) {
        attributes[lower] = cleaned;
        count += 1;
      }
    }
  }

  return {
    url: sanitizeGrabUrl(raw.url),
    tagName: truncate((raw.tagName || 'element').toLowerCase().replace(/[^a-z0-9_-]/g, ''), 32) || 'element',
    selector: raw.selector ? truncate(redactSecrets(raw.selector), GrabSanitizePolicy.maxAttrLength) : undefined,
    textContent: truncate(redactSecrets(raw.textContent || ''), GrabSanitizePolicy.maxTextLength),
    attributes,
    note: raw.note ? truncate(redactSecrets(raw.note), GrabSanitizePolicy.maxTextLength) : undefined,
  };
}

export function formatGrabForAgent(payload: SanitizedBrowserGrabPayload): string {
  const lines = [
    `Browser Element Grab (sanitized):`,
    `URL: ${payload.url}`,
    `Element: <${payload.tagName}>`,
  ];
  if (payload.selector) lines.push(`Selector: ${payload.selector}`);
  const attrEntries = Object.entries(payload.attributes);
  if (attrEntries.length > 0) {
    lines.push(`Attributes: ${attrEntries.map(([k, v]) => `${k}="${v}"`).join(' ')}`);
  }
  if (payload.textContent) lines.push(`Text: ${payload.textContent}`);
  if (payload.note) lines.push(`Note: ${payload.note}`);
  return lines.join('\n');
}

