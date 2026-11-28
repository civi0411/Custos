/**
 * Standards-based AES-256-GCM cipher adapter for `NativeE2eeCodec` using `globalThis.crypto.subtle`
 * (works in React Native Hermes/JSI polyfills, Capacitor, and Mobile Web without extra deps).
 */

export interface AsyncAeadCipher {
  sealAsync(plaintext: string, epoch: number): Promise<string>;
  openAsync(ciphertextBase64: string, epoch: number): Promise<string>;
}

interface SubtleCryptoLike {
  encrypt(
    algorithm: { name: string; iv: Uint8Array; additionalData?: Uint8Array },
    key: unknown,
    data: Uint8Array
  ): Promise<ArrayBuffer>;
  decrypt(
    algorithm: { name: string; iv: Uint8Array; additionalData?: Uint8Array },
    key: unknown,
    data: Uint8Array
  ): Promise<ArrayBuffer>;
}

interface CryptoHostLike {
  subtle?: SubtleCryptoLike;
  getRandomValues?(array: Uint8Array): Uint8Array;
}

function getCryptoHost(): CryptoHostLike {
  const g = globalThis as unknown as { crypto?: CryptoHostLike };
  if (!g.crypto?.subtle || !g.crypto?.getRandomValues) {
    throw new Error('WebCrypto SubtleCrypto + getRandomValues unavailable in this runtime');
  }
  return g.crypto;
}

function encodeUtf8(input: string): Uint8Array {
  const bytes: number[] = [];
  for (let i = 0; i < input.length; i++) {
    const code = input.charCodeAt(i);
    if (code < 0x80) bytes.push(code);
    else if (code < 0x800) bytes.push(0xc0 | (code >> 6), 0x80 | (code & 0x3f));
    else bytes.push(0xe0 | (code >> 12), 0x80 | ((code >> 6) & 0x3f), 0x80 | (code & 0x3f));
  }
  return new Uint8Array(bytes);
}

function decodeUtf8(bytes: Uint8Array): string {
  let out = '';
  let i = 0;
  while (i < bytes.length) {
    const b0 = bytes[i++];
    if (b0 < 0x80) {
      out += String.fromCharCode(b0);
    } else if ((b0 & 0xe0) === 0xc0 && i < bytes.length) {
      out += String.fromCharCode(((b0 & 0x1f) << 6) | (bytes[i++] & 0x3f));
    } else if (i + 1 < bytes.length) {
      out += String.fromCharCode(
        ((b0 & 0x0f) << 12) | ((bytes[i++] & 0x3f) << 6) | (bytes[i++] & 0x3f)
      );
    }
  }
  return out;
}

const B64 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';

function toBase64(bytes: Uint8Array): string {
  let out = '';
  for (let i = 0; i < bytes.length; i += 3) {
    const a = bytes[i];
    const b = i + 1 < bytes.length ? bytes[i + 1] : 0;
    const c = i + 2 < bytes.length ? bytes[i + 2] : 0;
    const n = (a << 16) | (b << 8) | c;
    out += B64[(n >> 18) & 63] + B64[(n >> 12) & 63];
    out += i + 1 < bytes.length ? B64[(n >> 6) & 63] : '=';
    out += i + 2 < bytes.length ? B64[n & 63] : '=';
  }
  return out;
}

function fromBase64(b64: string): Uint8Array {
  const clean = b64.replace(/=+$/, '');
  const out = new Uint8Array(Math.floor((clean.length * 3) / 4));
  let p = 0;
  for (let i = 0; i < clean.length; i += 4) {
    const n =
      (B64.indexOf(clean[i]) << 18) |
      (B64.indexOf(clean[i + 1]) << 12) |
      ((i + 2 < clean.length ? B64.indexOf(clean[i + 2]) : 0) << 6) |
      (i + 3 < clean.length ? B64.indexOf(clean[i + 3]) : 0);
    out[p++] = (n >> 16) & 0xff;
    if (i + 2 < clean.length) out[p++] = (n >> 8) & 0xff;
    if (i + 3 < clean.length) out[p++] = n & 0xff;
  }
  return out.subarray(0, p);
}

export class WebCryptoAesGcmCipher implements AsyncAeadCipher {
  constructor(private readonly cryptoKey: unknown) {}

  async sealAsync(plaintext: string, epoch: number): Promise<string> {
    const host = getCryptoHost();
    const iv = new Uint8Array(12);
    host.getRandomValues!(iv);
    const aad = encodeUtf8(`custos-e2ee:v1:epoch:${epoch}`);
    const ctBuf = await host.subtle!.encrypt(
      { name: 'AES-GCM', iv, additionalData: aad },
      this.cryptoKey,
      encodeUtf8(plaintext)
    );
    const ct = new Uint8Array(ctBuf);
    const packed = new Uint8Array(12 + ct.length);
    packed.set(iv, 0);
    packed.set(ct, 12);
    return toBase64(packed);
  }

  async openAsync(ciphertextBase64: string, epoch: number): Promise<string> {
    const host = getCryptoHost();
    const packed = fromBase64(ciphertextBase64);
    if (packed.length < 13) {
      throw new Error('Malformed AES-GCM envelope');
    }
    const iv = packed.subarray(0, 12);
    const ct = packed.subarray(12);
    const aad = encodeUtf8(`custos-e2ee:v1:epoch:${epoch}`);
    const ptBuf = await host.subtle!.decrypt(
      { name: 'AES-GCM', iv, additionalData: aad },
      this.cryptoKey,
      ct
    );
    return decodeUtf8(new Uint8Array(ptBuf));
  }
}

