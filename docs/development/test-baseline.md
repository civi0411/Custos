# Test Baseline Counts

> Snapshot `28ab973`, offline test results for core four crates.

| Crate | Target Tests | Actual Passed | Failed | Ignored | Result |
|---|---|---|---|---|---|
| `custos-domain` | 79 | 79 | 0 | 0 | OK |
| `custos-core` | 61 | 61 | 0 | 0 | OK |
| `custos-persistence` | 42 | 42 | 0 | 0 | OK |
| `custos-runtime` | 190 | 190 | 0 | 0 | OK |
| **Total** | **372** | **372** | **0** | **0** | **PASS** |

Command executed:
```bash
cargo test -q -p custos-domain -p custos-core -p custos-persistence -p custos-runtime --lib --offline
```
