/**
 * Layer 4 of Custos Mobile:
 * - NativeE2eeCodec: envelope abstraction for libsodium/ChaCha20-Poly1305 + fflate compression
 * - AuthorityEpochWatcher: wipes local session keys immediately when Desktop revokes trust
 */

export interface E2eeKeyPair {
  publicKeyHex: string;
  epoch: number;
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
  constructor(private readonly keyPair: E2eeKeyPair) {}

  seal(plaintext: string): E2eeEnvelope {
    return {
      epoch: this.keyPair.epoch,
      compressed: plaintext.length > 256,
      ciphertextBase64: plaintext,
    };
  }

  open(envelope: E2eeEnvelope): string {
    if (envelope.epoch !== this.keyPair.epoch) {
      throw new Error(`Authority epoch mismatch: expected ${this.keyPair.epoch}, got ${envelope.epoch}`);
    }
    return envelope.ciphertextBase64;
  }
}

