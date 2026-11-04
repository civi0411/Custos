# Tool Output Bounds & LimitedWriter Specification

> **Scope**: `crates/custos-runtime/src/agent/runtime_port.rs`  
> **Classification**: Architecture Reference  
> **Status**: Canonical Specification (Phase 1 / F05)

---

## 1. Problem & Architecture Rationale

When agents execute tools (e.g., shell commands, MCP tools, large JSON data fetches), unconstrained output serialization poses severe memory exhaustion risks:
- Calling `serde_json::to_string(&val)` naively allocates the entire JSON payload in RAM before checking truncation limits. A 500MB payload allocates 500MB even if `max_tool_output_bytes` is capped at 100KB.
- In multi-agent concurrent topologies (T2, T3), multiple workers serializing large unconstrained outputs can easily trigger out-of-memory crashes.

---

## 2. Solution: Bounded `LimitedWriter`

`LimitedWriter` wraps a byte buffer with a strict capacity ceiling and implements `std::io::Write`:

```rust
pub struct LimitedWriter<'a> {
    inner: &'a mut Vec<u8>,
    limit: usize,
    capped: bool,
}
```

### Key Guarantees:
1. **Pre-allocated Bounded Capacity**: Buffer capacity is clamped: `cap.min(65_536)`.
2. **Streaming Early Termination**: When bytes reach `limit`, subsequent writes return `Err(WriteZero)` or write only the remaining available slice, immediately setting `capped = true`.
3. **UTF-8 Char Boundary Safety**: When truncated, `truncate_to_char_boundary()` is invoked on the resulting string to prevent invalid UTF-8 codepoint splits before appending the canonical truncation marker:
   `\n... [TRUNCATED: output exceeded max tool output limit]`.
