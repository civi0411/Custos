<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../assets/banner-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="../assets/banner.png">
  <img alt="Custos" src="../assets/banner.png" width="100%">
</picture>

[ EN ](../../README.md) · [ VI ](README.vi.md) · [ DE ](README.de.md) · [ ZH ](README.zh.md) · [ JA ](README.ja.md) · [ KO ](README.ko.md) · [ ES ](README.es.md)

</div>

# Custos SADE

> **Môi Trường Phát Triển Agent Có Giám Sát (SADE) — Ưu Tiên Cục Bộ, Gắn Liền Bằng Chứng, Con Người Làm Chủ.**

Custos là môi trường phát triển và thực thi ưu tiên cục bộ (local-first) dành cho các agent tự trị trong các lĩnh vực lập trình (coding), nghiên cứu khoa học (research) và trợ lý quy trình cá nhân (assistant). Khác biệt hoàn toàn với các vỏ bọc LLM đơn giản hoặc các hệ thống đa agent mờ ám, Custos áp dụng cơ chế giám sát nghiêm ngặt của con người, lưu trữ trạng thái tất định, cổng năng lực được kiểm toán bằng mật mã và yêu cầu bằng chứng thực nghiệm khách quan trước khi bất kỳ tác vụ nào được công nhận hoàn thành.

---

## Ba Trụ Cột Không Thể Thương Lượng

| Trụ cột | Nguyên lý cốt lõi | Bảo đảm kiến trúc |
|---|---|---|
| **P1 — Tác Vụ Là Thực Thể Hạng Nhất** | Tác vụ tồn tại độc lập với phiên làm việc và kết nối | Lưu trữ SQLite WAL bền bỉ, phục hồi tất định sau sự cố, và sổ cái lệnh (Command Ledger) lũy kế tránh trùng lặp. |
| **P2 — Đề Xuất ≠ Quyết Định ≠ Tác Động** | Mô hình đề xuất; Nhân cấp phép; Cổng thực thi | Mô hình AI chỉ phát sinh `ActionIntent`; Nhân (Kernel) ký giấy phép đơn lần `ExecutionPermit`; Sandbox kiểm soát cổng năng lực nghiêm ngặt. |
| **P3 — Hoàn Thành Gắn Liền Bằng Chứng** | Không có hoàn thành nếu thiếu bằng chứng thực tế | Tiêu chí nghiệm thu chỉ vượt qua khi có tạo phẩm độc lập (diffs, vết chạy kiểm thử, đối chiếu trích dẫn); triệt tiêu hoàn toàn hoàn thành ảo. |

---

## 10 Bất Biến Hệ Thống Bất Di Bất Dịch

1. **INV-01 (Chủ Quyền Ưu Tiên Cục Bộ):** Toàn bộ trạng thái, hợp đồng, tác vụ và dữ liệu đo đạc telemetry nằm hoàn toàn trên đĩa cục bộ.
2. **INV-02 (Không Nguyên Tử Ảo):** Mọi chuyển dịch trạng thái phải được commit vào SQLite trước khi phát lệnh hành động ra ngoài.
3. **INV-03 (Giấy Phép Trước Tác Động):** Không một đột biến ngoại vi nào được phép chạy nếu thiếu `ExecutionPermit` hợp lệ.
4. **INV-04 (Ràng Buộc Danh Tính Tác Tử):** Danh tính của actor được xác thực từ tầng transport, không bao giờ tin payload tự khai.
5. **INV-05 (Bằng Chứng Vượt Trên Lời Khai):** Nghiệm thu tác vụ đòi hỏi tạo phẩm thực nghiệm được kiểm chứng, không tin khẳng định suông của mô hình.
6. **INV-06 (Trung Thực Năng Lực Giao Diện):** Giao diện UI lập tức chuyển sang trạng thái an toàn đóng (`unavailable`/`degraded`) khi dịch vụ nền thiếu hụt.
7. **INV-07 (Cách Ly Lease Cây Làm Việc):** Các thao tác ghi đồng thời trên cùng working tree được fencing chặt chẽ để chống mất dữ liệu.
8. **INV-08 (Giới Hạn Biên Tài Nguyên):** Mọi tiến trình thực thi, bộ đệm dòng và cắt tỉa token đều tuân thủ ngưỡng byte nghiêm ngặt.
9. **INV-09 (Kiểm Toán Toàn Bộ Transports):** Mọi đột biến đều đi qua Command Ledger với xác thực băm digest chống replay.
10. **INV-10 (Chủ Quyền Tối Thượng Của Con Người):** Người vận hành giữ toàn quyền tạm dừng, điều hướng hoặc hủy bỏ tác vụ ở bất kỳ thời điểm nào.

---

## Cơ Chế Vận Hành

```mermaid
flowchart LR
    Human[Người Vận Hành] --> Session[Phiên Làm Việc / Workbench]
    Session --> Task[Tác Vụ Bền Vững]
    Task --> Context[Biên Dịch Ngữ Cảnh]
    Context --> Worker[Worker Mô Hình / Harness]
    Worker --> Intent[Ý Định Hành Động]
    Intent --> Gate[Cổng Thẩm Quyền & Giấy Phép]
    Gate --> Effect[Tác Dụng Phụ Trong Sandbox]
    Effect --> Evidence[Bằng Chứng Khách Quan]
    Evidence --> Outcome[Hoàn Thành Được Kiểm Chứng]
```

---

## Kiến Trúc Hệ Thống: 11 Crate Rust Chuẩn Mực

Custos phân định nghiêm ngặt giữa logic thuần túy và tác dụng phụ qua 11 crate Rust có cấu trúc phân tầng:

| Phân tầng | Crate | Trách nhiệm cốt lõi |
|---|---|---|
| **Tầng 0 (Domain)** | `custos-domain` | Định danh miền thuần túy, mô hình thực thể, máy trạng thái và các bất biến Zero-I/O. |
| **Tầng 1 (Core)** | `custos-core` | Dịch vụ điều phối tác vụ, engine thẩm quyền và các bộ kiểm chứng hoàn thành. |
| | `custos-persistence` | Lưu trữ SQLite WAL, mô hình Outbox, chuyển dịch trạng thái CAS và phục hồi sự cố. |
| | `custos-provider` | Hợp đồng mô hình trung lập nhà cung cấp, bộ giải mã streaming SSE và tokenizer. |
| **Tầng 2 (Runtime & Bridge)** | `custos-bridge` | Thăng cấp Session thành Task, định tuyến RPC và dispatch API nội bộ. |
| | `custos-runtime` | Vòng lặp nhận thức của agent, nén ngữ cảnh (RTK), các tầng bộ nhớ và bộ lập kế hoạch. |
| | `custos-adapters` | Sandbox hệ điều hành, adapter client MCP, sidecar suy luận cục bộ và bộ chuyển đổi giao thức. |
| **Tầng 3 (Domain Packs)** | `custos-packs` | Quy trình chuyên biệt theo miền nghiệp vụ: Gói Kỹ thuật (Engineering), Nghiên cứu (Research), Trợ lý (Assistant). |
| **Tầng 4 (Phân Phối & Giao Diện)** | `custos-daemon` | Điểm khởi tạo và lắp ráp duy nhất (Composition Root), dịch vụ daemon nền, HTTP/JSON-RPC cục bộ. |
| | `custos-sdk` | Đối tượng truyền dữ liệu (DTO), UniFFI bindings và SDK client. |
| | `custos-cli` | Giao diện dòng lệnh tương tác qua TUI (`ratatui`). |

---

## Giao Diện Desktop

Giao diện Desktop Custos được phát triển trên nền **Vite, React 19 và Tauri 2**:

- **Cấu Trúc 3 Cấp:** Điều hướng phân cấp trực quan qua `Dự Án -> Không Gian Làm Việc -> Cây Làm Việc (Project -> Workspace -> Worktree)`.
- **Hệ Thống Token Graphite JetBlack:** Tông màu tối hiện đại, tương phản cao, tối ưu hóa cho các phiên làm việc kéo dài.
- **Bàn Làm Việc Chuyên Biệt (Workbenches):**
  - **Kỹ thuật (Engineering):** Tệp & Thay đổi, Bằng chứng thực nghiệm, và Trình quản lý Worktree.
  - **Nghiên cứu (Research):** Phòng tổng hợp (Synthesis Studio), Phát hiện khoa học, và Sổ cái lượt chạy.
  - **Trợ lý & Dùng chung (Assistant & Shared):** Trình duyệt nhúng, Terminal, Hạm đội máy chủ từ xa, và Ghi chú.
- **Chính Sách Không Dữ Liệu Ảo (Zero-Mock Policy):** Mọi chỉ số trên UI phản ánh chính xác số liệu viễn trắc thật từ daemon và chuyển sang chế độ an toàn khi dịch vụ bị gián đoạn.

---

## Các Trục Giao Thức (Hubs) & Tích Hợp Harness Ngoài

Daemon Custos đóng vai trò là cổng liên thông trung tâm thông qua 6 Hub chức năng:

- **Model Hub:** Định tuyến yêu cầu giữa các engine cục bộ (GGUF, llama.cpp) và các nhà cung cấp đám mây (Anthropic, OpenAI, Gemini, Bedrock) với khả năng tự động fallback.
- **Tool Hub:** Quản trị các server Model Context Protocol (MCP) và các công cụ thực thi sandbox.
- **Agent Hub:** Điều phối giao thức giao tiếp Agent-to-Agent (A2A) và Agent Communication Protocol (ACP).
- **Storage Hub:** Quản lý lưu trữ SQLite WAL, lưu trữ blob và các chỉ mục có cấu trúc.
- **Event Hub:** Phân phối các sự kiện vòng đời và nhật ký kiểm toán thông lượng cao.
- **Operator Hub:** Cung cấp endpoint bảo mật cho CLI và giao diện Desktop.

### Adapter Tích Hợp Harness Ngoài
Custos bao bọc và giám sát các harness lập trình phổ biến mà không làm xáo trộn cơ chế nội bộ của chúng:
- **Claude Code:** Cách ly tiến trình con qua Stdio và bao bọc lệnh.
- **Codex:** Hoàn thiện mã nguồn và chạy kiểm thử trong sandbox.
- **Cursor:** Đồng bộ bộ đệm editor và áp dụng diff an toàn.
- **Antigravity:** Đồng bộ tác vụ và phối hợp runtime lập trình agentic.

---

## Cổng Kiểm Soát Chất Lượng & Kiểm Thử

Mọi đóng góp mã nguồn đều phải vượt qua các cổng kiểm soát nghiêm ngặt:

```bash
# 1. Cổng chặn cứng Clippy Rust (0 cảnh báo, 0 lỗi)
cargo clippy --workspace --all-targets -- -D warnings

# 2. Toàn bộ test suite (>1,670 tests, 100% pass)
cargo test --workspace --all-targets

# 3. Kiểm tra kiểu và đóng gói giao diện Desktop
cd ui/desktop && npm run build
```

---

## Tài Liệu & Tham Chiếu

- **Đặc Tả Kiến Trúc Chuẩn:** [Custos.md](../../Custos.md)
- **Quy Tắc Agent & Đóng Góp:** [AGENTS.md](../../AGENTS.md)
- **Tiêu Chuẩn Thiết Kế Desktop UI:** [.agents/rules/desktop_ui_architecture_standards.md](../../.agents/rules/desktop_ui_architecture_standards.md)
- **Quy Trình Git PR:** [.agents/rules/git_pr_workflow.md](../../.agents/rules/git_pr_workflow.md)
- **Thư Mục Tài Liệu:** [docs/README.md](../README.md)

---

## Bản Quyền

Custos được phân phối dưới giấy phép **GNU AGPL-3.0-or-later**. Giấy phép bao gồm điều khoản nghiêm ngặt về Nhãn hiệu nhằm bảo vệ thương hiệu Custos và nghiêm cấm việc đóng gói thành dịch vụ SaaS thương mại mã nguồn đóng. Xem chi tiết tại [LICENSE](../../LICENSE).
