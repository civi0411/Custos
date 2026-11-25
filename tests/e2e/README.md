# End-to-End Test Suite

Tests full end-to-end workflows:
- User creates task via CLI
- Task transitions from Draft -> Queued -> Running -> Succeeded
- Continuation packets are persisted and verified across spans
- Policy gate evaluates and permits valid actions
