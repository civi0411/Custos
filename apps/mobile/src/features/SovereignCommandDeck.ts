/**
 * 4 Exclusive Sovereign Command Deck Capabilities on Custos Mobile:
 * 1. Biometric Hardware Approver (signs SHA-256 arguments_digest of ExecutionPermit/Outbox)
 * 2. Quant Risk Radar & Atomic Kill Switch (v1.quant.kill_switch)
 * 3. GPU Training & Checkpoint Watchdog (Colab/Kaggle/SSH failover)
 * 4. Swipe-to-Govern FTS5 Memory (FactProposal approval/rejection)
 */

import type { StableLogicalRpcClient } from '../rpc/StableLogicalRpcClient';

export interface BiometricDigestSignature {
  permitId: string;
  argumentsDigestSha256: string;
  biometricSignature: string;
}

export class SovereignCommandDeck {
  constructor(private readonly rpc: StableLogicalRpcClient) {}

  approveOutboxWithBiometric(outboxId: string, signature: BiometricDigestSignature) {
    return this.rpc.request('v1.assistant.outbox.approve', {
      id: outboxId,
      arguments_digest: signature.argumentsDigestSha256,
      biometric_signature: signature.biometricSignature,
    });
  }

  triggerQuantKillSwitch(workspaceId: string, reason: string) {
    return this.rpc.request('v1.quant.kill_switch', { workspace_id: workspaceId, reason });
  }

  observeExperiment(experimentId: string) {
    return this.rpc.request('v1.experiment.observe', { experiment_id: experimentId });
  }

  resolveMemoryProposal(proposalId: string, decision: 'approve' | 'reject') {
    return this.rpc.request('v1.memory.proposals.resolve', {
      proposal_id: proposalId,
      decision,
    });
  }
}

