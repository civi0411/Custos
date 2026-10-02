# KHÔNG GIAN LÀM VIỆC: VINH — RUNTIME & CLIENT LEAD
## (Vinh Workspace — Workflow Engine, Protocols & Client Experience)

> **Kỹ sư phụ trách:** Vinh (Runtime & Client Lead)  
> **Trạng thái:** Active Maintainer Workspace  
> **Quy chuẩn quản trị tối cao:** Tuân thủ [`AGENTS.md`](../../AGENTS.md) và [`Custos.md`](../../Custos.md)

---

## 1. Trách Nhiệm Cốt Lõi & Các Crate Trực Tiếp Quản Trị

Vinh là Trưởng nhóm Luồng Thực thi, Giao thức Công cụ và Trải nghiệm Ứng dụng, trực tiếp sở hữu vòng đời Session/Task, máy thực thi workflow, cầu nối giao thức MCP, và giao diện người dùng dòng lệnh (CLI/TUI):

| Phân hệ đảm nhiệm | Đường dẫn Workspace | Trách nhiệm kỹ thuật độc quyền |
|---|---|---|
| **Runtime Engine** | `crates/custos-runtime` | Vòng đời Session, vòng lặp thực thi từng bước (Step Loop), worker lease, hủy tác vụ, bộ nhớ tạm. |
| **Bridge Layer** | `crates/custos-bridge` | Thăng cấp Session-to-Task có tính lũy thừa, định tuyến Local API, chuyển đổi lệnh từ Client. |
| **Terminal CLI** | `crates/custos-cli` | Giao diện dòng lệnh terminal (`ratatui`, `clap`): render diff màu sắc, bảng tiến độ real-time. |
| **Client SDK** | `crates/custos-sdk` | Trừu tượng hóa kết nối IPC Unix Domain Socket (`.custos/daemon.sock`), versioned DTOs. |
| **Protocol Adapters**| `crates/custos-adapters` | Model Context Protocol (MCP STDIO/SSE), 5 bộ điều hợp Harness (Claude, Codex, Cursor, AGY, Goose). |
| **UI & Packages** | `ui/`, `packages/` | Giao diện Desktop (Tauri/Electron), VS Code extension wrapper, gói cài đặt NPM. |

---

## 2. Các Bất Biến Bắt Buộc Tuân Thủ Tuyệt Đối

1. **Phân Tách IPC Tuyệt Đối Với CLI:** `custos-cli` tuyệt đối không được import `rusqlite` để đọc/ghi trực tiếp vào SQLite. Mọi giao tiếp bắt buộc phải đi qua `custos-sdk` kết nối vào Daemon qua Unix Domain Socket.
2. **Tách Biệt Trạng Thái Bước Với Trạng Thái Tác Vụ:** `custos-runtime` chỉ quản lý tiến trình của từng bước thực thi (Step progress); quyền chuyển đổi trạng thái tổng thể của Task (`TaskStatus::Succeeded / Failed`) thuộc quyền hạn duy nhất của Kernel do Vĩ sở hữu.
3. **Kỷ Luật An Toàn Giao Thức Ngoại Vi:** Mọi dữ liệu trả về từ MCP Server hoặc Agent Harness bên ngoài đều được coi là không đáng tin cậy (untrusted) và bắt buộc phải gắn nhãn qua Taint Tracking Engine.

---

## 3. Lệnh Phát Triển & Kiểm Thử Thường Dùng (Daily Runbook)

```bash
# Kiểm tra biên dịch các crate thuộc quyền quản lý
cargo check -p custos-runtime -p custos-bridge -p custos-sdk -p custos-cli

# Chạy unit tests cho runtime và bridge
cargo test -p custos-runtime --lib
cargo test -p custos-bridge --lib

# Chạy thử nghiệm CLI tương tác terminal
cargo run -p custos-cli -- status
cargo run -p custos-cli -- task list

# Kiểm tra tương thích giao thức MCP adapter
cargo test -p custos-adapters --test mcp_stdio_conformance
```

---

## 4. Danh Sách Nhiệm Vụ Sprint Hiện Tại (Gate 1 Focus)

- [ ] Hoàn thiện luồng thăng cấp `Session-to-Task` trong `custos-bridge` với khóa Idempotency Key chống trùng lặp.
- [ ] Xây dựng màn hình hiển thị tiến độ và bảng tổng kết kết quả bằng `ratatui` trong `custos-cli`.
- [ ] Hoàn thiện kết nối client `custos-sdk` qua Unix Domain Socket (`daemon.sock`).
- [ ] Phối hợp với Vĩ thử nghiệm điều phối lệnh từ CLI đến Task Kernel.

---

## 5. Cấu Trúc Không Gian Làm Việc

- `notes/`: Ghi chú kỹ thuật luồng Session-to-Task, thiết kế layout terminal `ratatui`, log giao tiếp MCP server.
- `reports/`: Biên bản nghiệm thu kiểm thử E2E client, kết quả benchmark độ trễ IPC socket.
