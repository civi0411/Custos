# Custos UX/UI & Client Applications — Kickoff & Implementation Guide
> **Tài Liệu Đánh Dấu Khởi Động & Đặc Tả Triển Khai Cho Frontend, Desktop & CLI**  
> **Phiên bản:** 1.0.0-draft | **Ngày khởi động:** 2026-09-22  
> **Nhóm phát triển dự án (Core Team):**  
> - **Nguyễn Đinh Nhật Trường** — Lead UX/UI & Client Applications (CLI / Desktop / VS Code)  
> - **Trần Chí Vĩ** — Lead Systems Architecture & Core Engine (`custosd` / Kernel / Gateway / Persistence)

---

## 🎯 1. Định Vị Vai Trò & Ranh Giới Hợp Tác

Dự án **Custos** được chia tách rõ ràng thành hai khối chuyên trách giữa hai thành viên:

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│             NGUYỄN ĐINH NHẬT TRƯỜNG (EXPERIENCE PLANE / CLIENTS)            │
│   [ custos CLI ]       [ VS Code Extension ]       [ Desktop Shell (Tauri) ]│
│   (Rust / clap)        (TypeScript / Webview)      (Frontend / Local Web)   │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │ JSON-RPC 2.0 (IPC / Socket / Pipe)
┌──────────────────────────────────────▼──────────────────────────────────────┐
│                TRẦN CHÍ VĨ (CORE ENGINE & SYSTEMS RUNTIME)                  │
│   [ custosd Daemon ]                                                        │
│   - Task Kernel & State Machine       - Capability Gateway & Sandbox        │
│   - SQLite Persistence & WAL          - AI Provider Adapters & System One   │
└─────────────────────────────────────────────────────────────────────────────┘
```

- **Trần Chí Vĩ (Core Engine / Systems Team):** Phụ trách logic nghiệp vụ ngầm (`custosd`, kernel, SQLite, state machine, AI reasoning, sandboxing, execution permits).
- **Nguyễn Đinh Nhật Trường (UX/UI & Client Team):** Phụ trách **bộ mặt của sản phẩm** — cách người dùng giao tiếp, ra lệnh, theo dõi tiến độ và đặc biệt là cơ chế **Exact-Payload Approval** (trải nghiệm phê duyệt bảo mật). Bạn chỉ hiển thị dữ liệu chiếu (*projections*) và gửi lệnh (*commands*), **không lưu giữ trạng thái bền vững riêng**.

---

## 📚 2. Danh Sách Tài Liệu Cần Đọc Đầu Tiên (Reading Checklist)

Để nắm bắt nhanh nhất mà không bị ngợp giữa hàng chục tài liệu hệ thống, hãy đọc theo thứ tự sau:

| Thứ tự | Tài liệu | Đường dẫn | Trọng tâm cần nắm |
|---|---|---|---|
| **1** | **Tổng quan & Bản sắc** | [00-start-here.md](file:///d:/Agentic%20Work%20Runtime/Custos/docs/00-start-here.md)<br>[identity.md](file:///d:/Agentic%20Work%20Runtime/Custos/docs/product/identity.md) | Triết lý Human-Centered, vì sao Custos không làm Agent tự chạy mất kiểm soát. |
| **2** | **Phạm vi & Lộ trình** | [scope.md](file:///d:/Agentic%20Work%20Runtime/Custos/docs/product/scope.md)<br>[roadmap.md](file:///d:/Agentic%20Work%20Runtime/Custos/docs/development/roadmap.md) | Horizon 1 ưu tiên CLI; Horizon 2 thêm VS Code Extension; Horizon 3 thêm Desktop Shell. Tiêu chí khởi động CLI `< 30ms`. |
| **3** | **Kiến trúc phân tầng** | [overview.md](file:///d:/Agentic%20Work%20Runtime/Custos/docs/architecture/overview.md) | Đọc kỹ **Layer 7 (Interface Layer)** và **Experience Plane**. |
| **4** | **Giao thức kết nối IPC** | [communication.md](file:///d:/Agentic%20Work%20Runtime/Custos/docs/architecture/communication.md) | Cách Client kết nối `custosd` qua Unix Domain Socket / Windows Named Pipe bằng JSON-RPC 2.0. |
| **5** | **Vòng đời Task & Approval** | [task-lifecycle.md](file:///d:/Agentic%20Work%20Runtime/Custos/docs/architecture/task-lifecycle.md)<br>[capability-model.md](file:///d:/Agentic%20Work%20Runtime/Custos/docs/security/capability-model.md) | Các trạng thái của Task (`draft` → `running` → `waiting_approval` → `completed`). Logic hiển thị diff phê duyệt an toàn. |
| **6** | **Quy ước đặt tên** | [naming.md](file:///d:/Agentic%20Work%20Runtime/Custos/docs/reference/naming.md) | Tên lệnh CLI (`kebab-case`), định danh RPC method, quy cách hiển thị Task ID (`tsk_...`). |

---

## 🔌 3. Hợp Đồng Giao Tiếp Giữa Client & Core (IPC / JSON-RPC Contract)

Hai bên giao tiếp qua kênh IPC cục bộ:
- **macOS / Linux:** Unix Domain Socket (mặc định: `~/.custos/custosd.sock`)
- **Windows:** Named Pipe (mặc định: `\\.\pipe\custosd`)

### 3.1. Các Lệnh Client Gửi Cho Core (Methods)
| RPC Method | Tham số chính | Trách nhiệm của Client |
|---|---|---|
| `task.create` | `{ "intent": string, "workspace_root": string, "budget_tokens"?: number }` | Khởi tạo task mới từ câu lệnh người dùng |
| `task.get` | `{ "task_id": string }` | Lấy chi tiết trạng thái, chi phí, run hiện tại |
| `task.list` | `{ "status"?: string, "limit"?: number }` | Liệt kê danh sách task đang chạy/lịch sử |
| `task.pause` | `{ "task_id": string }` | Tạm dừng task khi người dùng cần can thiệp |
| `task.resume` | `{ "task_id": string }` | Tiếp tục task đã tạm dừng |
| `task.cancel` | `{ "task_id": string }` | Hủy bỏ task |
| `permit.approve` | `{ "permit_id": string, "payload_hash": string }` | Ký duyệt thực thi hành động rủi ro (Exact-payload) |
| `permit.reject` | `{ "permit_id": string, "reason": string }` | Từ chối hành động rủi ro |
| `events.subscribe` | `{ "task_id": string }` | Lắng nghe luồng sự kiện realtime (Server-Sent Events / JSON-RPC stream) |

### 3.2. Sự Kiện Core Bắn Lên Client (Realtime Stream)
- `TaskCreated`, `TaskStatusChanged`: Cập nhật trạng thái tổng quan.
- `StepStarted`, `StepProgress`: Cập nhật tiến độ bước làm, spinner, logs.
- `ApprovalRequested`: **Quan trọng nhất** — Core dừng lại và gửi chi tiết diff, yêu cầu Client bật prompt/modal để người dùng xác nhận.
- `TaskCompleted`: Hiển thị Outcome Bundle (diff cuối, kết quả test, chi phí token/USD).
- `TaskFailed`: Hiển thị lỗi, gợi ý rollback hoặc chỉnh sửa.

---

## 🖥️ 4. Thiết Kế Trải Nghiệm (UX/UI Spec)

### 4.1. CLI Experience (`custos`) — Trải nghiệm Dòng lệnh Hiện đại
Xây dựng bằng Rust (dùng `clap`, `colored`/`owo-colors`, `indicatif`, `inquire`):
- **Cực nhanh:** Khởi động `< 30ms`.
- **Tương tác trực quan:**
  ```text
  $ custos run "Fix bug in authentication token expiration"
  [✓] Workspace verified: /Users/mac/Project/Custos
  [✓] Task tsk_01J8N6Z8 created (Budget: 50,000 tokens | $0.50)
  
  ● Exploring codebase... [2.1s]
  ● Generating patch for src/auth/token.rs... [3.4s]
  
  ⚠️ APPROVAL REQUIRED (High-Risk Action):
  ------------------------------------------------------------
  Action: Sửa đổi tệp mã nguồn
  Path  : src/auth/token.rs
  Hash  : a9f4c3...8b21
  Diff  :
  -   let expires_at = now + Duration::from_secs(3600);
  +   let expires_at = now + Duration::from_secs(custom_expiry.unwrap_or(3600));
  ------------------------------------------------------------
  [Y] Approve & Execute   [N] Reject   [V] View Full Context   [C] Cancel
  > Y
  
  [✓] Permit granted. Applying patch...
  ● Running test suite: cargo test auth... [Passed 14/14]
  🎉 Task Completed successfully! (Cost: 1,420 tokens | $0.014)
  ```

### 4.2. VS Code Extension & Desktop Shell — Trải nghiệm Đồ họa
- **Activity Bar Icon:** Biểu tượng Custos trên thanh công cụ bên trái.
- **Sidebar Tree View:**
  - Active Tasks (đang chạy với animation pulse).
  - Attention Needed (danh sách các hành động đang chờ phê duyệt).
  - Recent Outcomes (kết quả đã hoàn thành).
- **Exact-Payload Review Panel:** Tận dụng VS Code Native Diff Editor để hiển thị diff hai cột trực quan trước khi bấm "Approve".

---

## 🚀 5. Kế Hoạch Triển Khai Cho Bạn (Action Roadmap)

### Tuần 1–2: Nền tảng CLI & IPC Mock (Bắt tay làm ngay)
- [ ] Khởi tạo thư mục `apps/custos-cli/` trong Monorepo (Rust binary).
- [ ] Cấu hình `Cargo.toml` với `clap` (derive macro), `tokio`, `interprocess` (cho Named Pipe / UDS), `serde_json`.
- [ ] Tạo Mock Daemon RPC để test CLI độc lập mà không cần đợi đồng đội viết xong Core.
- [ ] Hoàn thiện các lệnh cơ sở:
  - `custos --version`, `custos --help` (đo benchmark `< 30ms`).
  - `custos status` (hiển thị bảng tóm tắt task đẹp mắt).

### Tuần 3–4: Interactive Approval & Realtime Streaming
- [ ] Tích hợp `indicatif` cho progress bars và step spinners.
- [ ] Xây dựng màn hình `Approval Prompt` với hiển thị Git syntax highlighting.
- [ ] Hoàn thiện các lệnh: `custos run "<prompt>"`, `custos approve <permit-id>`, `custos pause <task-id>`, `custos resume <task-id>`.

### Tuần 5+: Khởi động VS Code Extension & Desktop Webview
- [ ] Tạo thư mục `apps/custos-vscode/` (TypeScript / VS Code Extension API).
- [ ] Kết nối Extension Client với socket daemon.
- [ ] Xây dựng WebView cho Human Attention Packet.

---

> 💡 *File này đánh dấu sự khởi đầu chính thức của phân hệ UI/UX/Client trong dự án Custos. Hãy cùng đồng đội đối soát hợp đồng IPC JSON-RPC ở Mục 3 trước khi code tính năng đầu tiên!*
