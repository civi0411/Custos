# FROZEN MODULE — CUSTOS ENGINE (ADR-COMPAT-01)

> **TRẠNG THÁI: ĐÓNG BĂNG HOÀN TOÀN**
> **QUY TẮC BẤT DI BẤT DỊCH**: 
> 1. KHÔNG thêm code mới vào thư mục này.
> 2. KHÔNG sửa đổi các file trong thư mục này.
> 3. KHÔNG đưa `custos-engine` vào `[workspace.members]` của root `Cargo.toml`.
> 4. CHỈ BÓC TÁCH (extract) từng module sang các crate đích sạch (`custos-context`, `custos-cognitive`, `custos-adapters-mcp`, `custos-agent`, v.v.).

---

### Danh mục bóc tách 4 tầng:
- **Tầng 1 (Đã bóc tách)**:
  - `token_counter.rs` → `crates/runtime/custos-context/src/token_counter.rs` ✅
  - `action_required_manager.rs` → `crates/runtime/custos-cognitive/src/human_gate.rs` ✅
  - `agents/tool_confirmation_*.rs` → `crates/runtime/custos-cognitive/src/approval_router.rs` ✅
- **Tầng 2 (Đang bóc tách - Pha 1)**:
  - `agents/mcp_client.rs` → `crates/adapters/custos-adapters-mcp/`
  - `agents/tool_execution.rs` → `crates/runtime/custos-agent/`
  - `context_mgmt/mod.rs` → `crates/runtime/custos-context-management/`
  - `agents/prompt_manager.rs` → `crates/runtime/custos-cognitive/`
- **Tầng 3 (Pha 2)**:
  - `agents/subagent_handler.rs` → `crates/runtime/custos-agent/` (System Two Deliberation)
  - `agents/reply_parts.rs` → `crates/runtime/custos-agent/`
  - `scheduler/*.rs` → `crates/runtime/custos-workflow/`
- **Tầng 4 (Giữ đóng băng lâu dài)**:
  - `gateway/`, `hooks/`, `oauth/`, `doctor.rs`
