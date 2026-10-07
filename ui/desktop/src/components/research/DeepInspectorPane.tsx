import React, { useState } from 'react';
import { ArtifactInspectorData } from '@/types/research';
import { ArtifactInspector } from './ArtifactInspector';

const MOCK_ARTIFACT_DATA: ArtifactInspectorData = {
  title: 'simulate_jitter.py',
  filename: 'simulate_jitter.py',
  activeVersion: 'v2',
  language: 'python',
  inputs: ['results_baseline.csv', 'env_config.json'],
  code: `import numpy as np
import pandas as pd
import argparse

def evaluate_jitter(trials=10000, fence_mode='strict'):
    """
    Sovereign Invariant Evaluation:
    Evaluates double-dispatch anomaly rates under synthetic network latency jitter.
    """
    np.random.seed(42)
    latencies = np.linspace(5, 50, 8)
    
    if fence_mode == 'strict':
        # Zero-trust CAS ticket fencing eliminates all phantom dispatch anomalies
        anomalies = np.zeros_like(latencies)
    else:
        # Optimistic locking experiences quadratic degradation
        anomalies = 0.015 * (latencies ** 1.8)
        
    df = pd.DataFrame({
        'latency_ms': latencies,
        'anomaly_pct': anomalies
    })
    return df

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--trials', type=int, default=10000)
    parser.add_argument('--fence', type=str, default='strict')
    args = parser.parse_args()
    
    df = evaluate_jitter(args.trials, args.fence)
    df.to_csv('results.csv', index=False)
    print(f"Executed {args.trials} trials with {args.fence} fencing. Zero violations.")`,
  versions: [
    {
      label: 'v1',
      code: `# Initial draft without invariant ticket acquisition\nprint("Running initial unshielded tests...")`,
      executionLog: 'Initial draft execution completed with 3 warnings.',
      reviewPassed: false,
      timestamp: Date.now() - 7200000,
    },
    {
      label: 'v2',
      code: `import numpy as np\n# Invariant Fencing v2\nprint("Strict fencing enabled. Zero I/O leakage.")`,
      executionLog: `[Custos SADE Sandbox] Spawning bounded workspace with ticket #perm_8821\n[Result] Output written to results.csv. Merkle hash verified.`,
      reviewPassed: true,
      timestamp: Date.now() - 1800000,
    },
  ],
  executionLog: `[Custos SADE Sandbox] Spawning bounded workspace with ticket #perm_8821\n[Run] Loading parameters: trials=10000, fence=strict\n[Run] Processed 10000 iterations. Zero divergence.\n[Result] Output written to results.csv. Merkle hash verified.\n[SADE] Invariant gate sealed: cas://bafy2bzace4v3k99a77z`,
  environment: 'Python 3.11.8 · Darwin aarch64 (Apple M2 Pro) · 8 Cores · 32 GB RAM · Accelerator: MPS',
  messages: [
    'User: Write a simulation script to measure double-dispatch failure rates under network latency.',
    'Assistant: Implemented evaluate_jitter() in simulate_jitter.py comparing strict zero-trust fencing against optimistic scheduling.',
    'User: Ensure all random seeds are deterministic and outputs write to results.csv.',
    'Assistant: Added fixed seed 42 and CSV persistence with Invariant receipt sealing.',
  ],
  reviewPassed: true,
  reviewFindings: [
    {
      level: 'ok',
      title: 'INV-DET-01: Zero Floating-Point Divergence',
      evidence: 'Deterministic seed 42 ensures identical output across clean-room replays.',
      check: 'determinism',
    },
    {
      level: 'ok',
      title: 'INV-BOUNDS-02: Bounded Memory & I/O Closure',
      evidence: 'No unbound heap allocation; socket connections blocked by sandbox profile.',
      check: 'security',
    },
  ],
};

interface DeepInspectorPaneProps {
  onClose?: () => void;
  onShowToast?: (msg: string) => void;
}

export const DeepInspectorPane: React.FC<DeepInspectorPaneProps> = ({
  onClose,
  onShowToast: _onShowToast,
}) => {
  const [data] = useState<ArtifactInspectorData>(MOCK_ARTIFACT_DATA);
  const [isMaximized, setIsMaximized] = useState(false);

  return (
    <div className="h-full w-full">
      <ArtifactInspector
        data={data}
        onClose={onClose}
        isMaximized={isMaximized}
        onToggleMaximize={() => setIsMaximized(!isMaximized)}
      />
    </div>
  );
};
