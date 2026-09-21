# 🤝 Custos Dev Docs — Quy Trình Hợp Tác & Nghiệp Vụ (Vĩ & Trường)

> Thư mục này là trung tâm điều phối nội bộ giữa **Vĩ** (AI Engineer) và **Trường** (Software Engineer).  
> Toàn bộ tài liệu kỹ thuật chuẩn của sản phẩm nằm riêng tại thư mục [docs/](../docs/00-start-here.md).

---

## 🗺️ 1. Phân Chia Ranh Giới Nghiệp Vụ

Hệ thống được thiết kế theo nguyên tắc **"Rust-First Polyglot"**: Rust là lõi bất biến, các ngôn ngữ phụ trợ (Python/TypeScript) chỉ chạy cô lập dưới dạng Sidecar giao tiếp qua JSON-RPC.

```mermaid
flowchart TD
    subgraph "Lãnh Địa Của Trường (Software Engineer - 100% Rust & SQL)"
        DB[(SQLite Persistence)] <--> Kernel[Task Kernel & State Machine]
        Kernel <--> CLI[Custos CLI]
        Kernel <--> API[Axum Local API]
        Kernel <--> Sandbox[Capability Gateway & Sandbox]
    end

    subgraph "Lãnh Địa Của Vĩ (AI Engineer - Core Domain & Sidecars)"
        Arbiter[Cognitive Arbiter (Rust)]
        Arbiter <--> TS[Claude Agent Sidecar (TypeScript)]
        Arbiter <--> PY[Local ML / Heuristics (Python)]
        Arbiter <--> LLM[External Providers API]
    end
    
    Kernel ===|JSON-RPC / Core Domain Types| Arbiter
```

### 🧑‍💻 Vĩ — AI Engineer (Core Architect)
- **Vị trí phụ trách:** `crates/core-domain`, `crates/cognitive-runtime`, `sidecars/python-judgment`, `sidecars/ts-claude-agent`.
- **Ngôn ngữ:** Rust (ở tầng interface/contracts), Python & TypeScript (ở Sidecars).
- **Trách nhiệm chính:**
  - Định nghĩa Core Domain types & schemas trong `crates/core-domain` (Zero I/O, Single Source of Truth).
  - Thiết kế logic Cognitive Arbiter (System 1: Fast Heuristics / System 2: Deep LLM Reasoning).
  - Viết và bảo trì các Sidecar (Python cho Local ML, TypeScript cho Claude API/CLI adapter).
  - Tinh chỉnh Prompt, Context Compiler, token budget.
- **Thư mục làm việc & báo cáo:** [dev_docs/vi/](./vi/README.md)

### 🧑‍🔧 Trường — Software Engineer (SE)
- **Vị trí phụ trách:** `crates/persistence-sqlite`, `crates/capability-gateway`, `crates/local-api`, `apps/custos-cli`, `apps/custosd`.
- **Ngôn ngữ:** 100% Rust thuần & SQL.
- **Tâm thế làm việc:**
  - **Coi AI là một "Black Box" (Hộp đen):** Không cần học hay quan tâm các khái niệm Prompt, Token, Temperature hay LLM.
  - **Quy đổi mọi thứ thành Data & System:** Mọi quyết định từ AI chỉ là chuỗi JSON cần validate, lưu trữ vào SQLite bền vững và kiểm tra quyền sandbox trước khi cho phép chạy lệnh OS.
- **Trách nhiệm chính:**
  - Thiết kế bảng SQLite, migration, CRUD operations bằng `rusqlite` / `sqlx`.
  - Viết CLI với `clap` và HTTP API cục bộ với `axum`.
  - Xây dựng rào chắn bảo vệ OS: Worktree isolation, Seatbelt (macOS) / Bubblewrap (Linux).
  - Hạ tầng quan sát: Logging, OpenTelemetry tracing.
- **Thư mục làm việc & báo cáo:** [dev_docs/truong/](./truong/README.md)

---

## 🌿 2. Chiến Lược Phân Nhánh Git (Git Workflow)

Để tránh xung đột code, không làm bẩn git commit log và dễ dàng theo dõi tiến độ của nhau, dự án sử dụng chiến lược 5 nhánh:

| Nhánh | Mục đích | Ai làm việc trên nhánh này? | Quy tắc |
|---|---|---|---|
| `main` | **Production Release** | Cả hai | Nhánh sạch nhất, chỉ chứa code đã kiểm thử hoàn chỉnh, tài liệu hoàn thiện và sẵn sàng phát hành. |
| `dev` | **Tích hợp & Test tổng (Integration)** | Cả hai | Nơi merge code từ nhánh `vi` và `truong` để kiểm tra liên kết hệ thống, chạy full test (`cargo test --workspace`). |
| `vi` | **Nhánh riêng của Vĩ** | Vĩ | Code phần AI Domain, Prompts, Sidecars. Tự do commit trong phạm vi được giao. |
| `truong` | **Nhánh riêng của Trường** | Trường | Code phần Backend, SQLite, CLI, Gateway. Tự do commit trong phạm vi được giao. |
| `report` | **Trao đổi Docs & Báo cáo** | Cả hai | Nơi 2 người cập nhật `dev_docs/`, viết báo cáo ngày/sprint, review tài liệu của nhau mà **không làm ô nhiễm lịch sử commit của nhánh code `dev`**. |

### Chu Trình Làm Việc Hàng Ngày:
1. **Làm tính năng:** Vĩ làm trên `vi`, Trường làm trên `truong`.
2. **Báo cáo & trao đổi:** Checkout sang nhánh `report`, viết notes/report vào `dev_docs/vi/` hoặc `dev_docs/truong/`, commit và push để người kia đọc review.
3. **Ghép code:** Khi một mốc tính năng sẵn sàng, tạo PR/Merge từ `vi` hoặc `truong` vào `dev`. Chạy test kiểm thử tích hợp trên `dev`.
4. **Phát hành:** Khi `dev` vượt qua toàn bộ test và ổn định, merge `dev` vào `main`.

---

## 🔄 3. Giao Thức Ghép Code (Hand-off Protocol)

1. **Bước 1:** Vĩ định nghĩa `struct` hoặc `trait` bằng Rust trong `crates/core-domain` (Ví dụ: `struct Task { pub id: TaskId, pub status: TaskStatus }`).
2. **Bước 2:** Trường nhìn vào struct này để viết code `INSERT INTO tasks...` lưu xuống SQLite trong `crates/persistence-sqlite` hoặc tạo endpoint trong `crates/local-api`.
3. **Bước 3:** Không ai được tự ý đổi tên các file, struct hoặc trait dùng chung mà chưa thông báo trước.

---

## 📍 4. Theo Dõi Tiến Độ Sprint
Xem chi tiết các đầu việc đang thực hiện tại [SPRINT_STATUS.md](./SPRINT_STATUS.md).
