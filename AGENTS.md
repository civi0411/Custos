# 🛡️ CUSTOS — AI VIBECODING RULES (SSOT)

> **Single Source of Truth (SSOT)** dành cho mọi AI Assistants (Cursor, Claude Code, Antigravity, Codex, Windsurf, Copilot).  
> **Nguyên tắc:** Ngăn chặn tuyệt đối ảo giác (hallucination), code sai kiến trúc hoặc phá vỡ ranh giới an toàn.  
> Chi tiết mô hình cộng tác: xem [dev_docs/README.md](dev_docs/README.md) | Tiến độ: xem [dev_docs/SPRINT_STATUS.md](dev_docs/SPRINT_STATUS.md).

---

## 🗺️ Bảng Ranh Giới & Phân Quyền Nhanh

| Thư mục / Crate | Ngôn ngữ | Người phụ trách | Quy tắc tối thượng cho AI |
|---|---|---|---|
| `crates/core-domain` | **Rust** | **Vĩ** (AI Engineer) | **Zero I/O, zero crate nội bộ.** Không network, không filesystem. |
| `crates/task-kernel`, `workflow-runtime` | **Rust** | **Vĩ & Trường** | State machine & orchestration. Không query trực tiếp DB. |
| `crates/persistence-sqlite` | **Rust + SQL** | **Trường** (SE) | Cư xử như Backend SE thuần túy. Lưu trữ & query DB. Không LLM fluff. |
| `apps/custos-cli`, `crates/local-api` | **Rust** | **Trường** (SE) | CLI / Axum HTTP API. Nhận input, validate JSON. |
| `crates/capability-gateway` | **Rust** | **Trường** (SE) | Sandbox, Worktree, permission gates, human approval. |
| `sidecars/ts-claude-agent` | **TypeScript** | **Vĩ** (AI Engineer) | Claude CLI adapter / VS Code connector. Giao tiếp qua JSON-RPC. |
| `sidecars/python-judgment` | **Python** | **Vĩ** (AI Engineer) | Fast ML / System 1 heuristic. Giao tiếp qua JSON-RPC. Không LangChain! |


---

## ⛔ 6 Luật Bất Khả Xâm Phạm

### 1. Ranh Giới Ngôn Ngữ (Rust-First Polyglot)
- **Rust là lõi:** `crates/`, `apps/`, `adapters/` (ngoại trừ sidecars) **100% Rust**. Tuyệt đối không thêm Python hay JS vào các thư mục này.
- **Sidecars cô lập:** Python và TypeScript **chỉ** sống trong `sidecars/`.
- **Giao tiếp:** Sidecars gọi Rust Core qua JSON-RPC (stdio/HTTP). Không sidecar nào được query trực tiếp file SQLite.

### 2. Dependency Flow (Luồng Phụ Thuộc 1 Chiều)
```text
apps (custos-cli) 
  ──> adapters / persistence (persistence-sqlite) 
        ──> workflow-runtime 
              ──> task-kernel 
                    ──> core-domain (Tâm điểm - Không phụ thuộc bất kỳ ai)
```
- **Cấm:** Không bao giờ `use persistence_sqlite::...` hay `use local_api::...` trong `core-domain` hoặc `task-kernel`.

### 3. Quy Tắc Ứng Xử Cho Role SE (Software Engineer)
Khi làm việc trong DB (`persistence-sqlite`), API (`local-api`), CLI (`custos-cli`):
- AI phải cư xử như một backend systems engineer thuần túy.
- **Không** giải thích các khái niệm LLM, Prompt, Token, Temperature.
- Xem mọi request từ AI (Arbiter) chỉ là các chuỗi JSON cần validate, parse và lưu trữ.

### 4. Chính Sách "Không Thư Viện Rác" (Zero Bloat Policy)
- **Rust:** Không tự ý thêm crate ngoài `tokio`, `serde`, `sqlx`/`rusqlite`, `thiserror`, `tracing` nếu chưa có chỉ định.
- **Python Sidecar:** Cấm `langchain`, `llama-index`, `langgraph`. Chỉ dùng native HTTP/API client, `pydantic`, `httpx`.
- Custos tự xây dựng engine runtime riêng.

### 5. Phương Pháp Code "Chậm Mà Chắc"
- Không sinh file dài hàng trăm dòng trong một lần generate.
- Giữ nguyên các `todo!()` nếu không được yêu cầu implement chi tiết.
- Không tự ý đổi tên các struct/trait cốt lõi đã được định nghĩa chuẩn hóa.

### 6. Xử Lý Lỗi (Error Handling)
- **Cấm:** Tuyệt đối không dùng `.unwrap()` hay `.expect()` trong code production (chỉ cho phép trong test).
- Luôn định nghĩa `enum Error` bằng `thiserror` cho mỗi crate và trả về `Result<T, CrateError>`.

