# Crash Consistency & Recovery Tests

This test suite verifies that `custosd` can recover state cleanly after unexpected
termination (SIGKILL, power loss, process crash) by verifying:
1. SQLite WAL integrity
2. Task state transitions never roll back to uncommitted states
3. Continuation packets retain cryptographically valid digests
