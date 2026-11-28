/**
 * Layer 4 of Custos Mobile:
 * - NativeE2eeCodec: unified fail-closed AEAD envelope codec supporting both
 *   synchronous native JSI ciphers (`AeadCipher`) and async WebCrypto (`AsyncAeadCipher`)
 * - AuthorityEpochWatcher: wipes local session keys immediately when Desktop revokes trust
 */

import type { AsyncAeadCipher } from './WebCryptoAesGcmCipher';

export interface E2eeKeyPair {
  publicKeyHex: string;
  epoch: number;
}

export interface AeadCipher {
  seal(plaintext: string, epoch: number): string;
  open(ciphertextBase64: string, epoch: number): string;
}

export class AuthorityEpochWatcher {
  private activeEpoch: number | null = null;
  private wiped = false;

  observeEpoch(epoch: number, onRevoked: () => void): void {
    if (this.activeEpoch !== null && epoch < 0) {
      this.wiped = true;
      onRevoked();
      return;
    }
    if (this.activeEpoch !== null && epoch > this.activeEpoch) {
      // Epoch bumped after revocation -> require re-pairing & wipe stale keys
      this.wiped = true;
      onRevoked();
    }
    this.activeEpoch = epoch;
  }

  isWiped(): boolean {
    return this.wiped;
  }
}

export interface E2eeEnvelope {
  epoch: number;
  compressed: boolean;
  ciphertextBase64: string;
}

export class NativeE2eeCodec {
  constructor(
    private readonly keyPair: E2eeKeyPair,
    private readonly cipher?: AeadCipher,
    private readonly epochWatcher?: AuthorityEpochWatcher,
    private readonly asyncCipher?: AsyncAeadCipher
  ) {}

  private assertActive(): void {
    if (this.epochWatcher?.isWiped()) {
      throw new Error('E2EE session keys wiped after authority epoch revocation');
    }
  }

  private assertEpoch(envelope: E2eeEnvelope): void {
    if (envelope.epoch !== this.keyPair.epoch) {
      throw new Error(
        `Authority epoch mismatch: expected ${this.keyPair.epoch}, got ${envelope.epoch}`
      );
    }
  }

  seal(plaintext: string): E2eeEnvelope {
    this.assertActive();
    if (!this.cipher) {
      throw new Error('Sync AeadCipher not configured; use sealAsync or attach native cipher');
    }
    return {
      epoch: this.keyPair.epoch,
      compressed: plaintext.length > 256,
      ciphertextBase64: this.cipher.seal(plaintext, this.keyPair.epoch),
    };
  }

  open(envelope: E2eeEnvelope): string {
    this.assertActive();
    this.assertEpoch(envelope);
    if (!this.cipher) {
      throw new Error('Sync AeadCipher not configured; use openAsync or attach native cipher');
    }
    return this.cipher.open(envelope.ciphertextBase64, envelope.epoch);
  }

  async sealAsync(plaintext: string): Promise<E2eeEnvelope> {
    this.assertActive();
    const ciphertextBase64 = this.asyncCipher
      ? await this.asyncCipher.sealAsync(plaintext, this.keyPair.epoch)
      : this.cipher
        ? this.cipher.seal(plaintext, this.keyPair.epoch)
        : (() => {
            throw new Error('E2EE cipher backend is not configured; refusing cleartext relay envelope');
          })();
    return {
      epoch: this.keyPair.epoch,
      compressed: plaintext.length > 256,
      ciphertextBase64,
    };
  }

  async openAsync(envelope: E2eeEnvelope): Promise<string> {
    this.assertActive();
    this.assertEpoch(envelope);
    if (this.asyncCipher) {
      return this.asyncCipher.openAsync(envelope.ciphertextBase64, envelope.epoch);
    }
    if (this.cipher) {
      return this.cipher.open(envelope.ciphertextBase64, envelope.epoch);
    }
    throw new Error('E2EE cipher backend is not configured; refusing cleartext decode');
  }
}
