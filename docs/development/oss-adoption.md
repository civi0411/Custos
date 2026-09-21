# Chiến Lược Tiếp Nhận Nguồn Mở (Open Source Adoption Strategy)

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần XIII (§35-36) & Phần VII (§50-53, §57-58) Canonical Specification

Custos áp dụng triết lý nguồn mở thực dụng: **Tự sở hữu lõi ngữ nghĩa cốt lõi (Custos-owned semantic core), không copy mã nguồn bừa bãi, chỉ tích hợp thư viện chất lượng cao qua ranh giới rõ ràng.**

---

## 1. Năm Chế Độ Tiếp Nhận Nguồn Mở (5 Adoption Modes)

| Chế độ (Mode) | Định nghĩa | Chính sách áp dụng | Ví dụ dự án |
|---|---|---|---|
| **Direct Dependency** | Thêm trực tiếp vào `Cargo.toml` | Thư viện chuẩn mực, kiểm toán license khắt khe (MIT/Apache 2.0). | `tokio`, `serde`, `rusqlite`, `tree-sitter`, `cedar-policy` |
| **Clean Integration** | Tích hợp qua Adapter riêng biệt | Giao tiếp qua trait trừu tượng; có thể thay thế mà không sửa core. | `modelcontextprotocol/rust-sdk`, `opentelemetry` |
| **Reference / Borrow** | Nghiên cứu kiến trúc, viết lại theo chuẩn Custos | Học hỏi pattern thiết kế; tái hiện bằng unit test của Custos. | Học pattern state machine từ Temporal; outbox từ Restate |
| **Shadow / Evaluation**| Chạy thử nghiệm đo kiểm song song | Không ảnh hưởng luồng chính; chỉ dùng để thu thập số liệu so sánh. | TypeSafe Jev adapter ở chế độ Advisory |
| **Reject / No-Adopt** | Từ chối tiếp nhận | Tránh phụ thuộc cồng kềnh, sai lệch mô hình hoặc dính copyleft (GPL). | Các framework agent chat phức tạp (LangChain, CrewAI) |

---

## 2. Bản Đồ Repository Nguồn Mở Được Chọn Lọc

Custos chọn lọc các thư viện hàng đầu thế giới để gia tốc quá trình phát triển:

```text
┌─────────────────────────────────────────────────────────────┐
│                    RECOMMENDED REPO MAP                     │
├─────────────────────────┬───────────────────────────────────┤
│ Foundation & Parsing    │ tree-sitter/tree-sitter           │
│                         │ ast-grep/ast-grep                 │
│                         │ BurntSushi/ripgrep                │
├─────────────────────────┼───────────────────────────────────┤
│ Security & Policy       │ cedar-policy/cedar                │
│                         │ containers/bubblewrap             │
├─────────────────────────┼───────────────────────────────────┤
│ Protocols & Standards   │ modelcontextprotocol/rust-sdk     │
│                         │ open-telemetry/opentelemetry-rust │
├─────────────────────────┼───────────────────────────────────┤
│ Provider SDKs           │ openai/codex                      │
│                         │ anthropics/claude-agent-sdk-*     │
└─────────────────────────┴───────────────────────────────────┘
```

---

## 3. Chính Sách Chuỗi Cung Ứng & Giấy Phép (Supply Chain Policy)

- **Kiểm soát License:** Tự động kiểm tra qua công cụ `cargo-deny`. Chỉ chấp nhận giấy phép **MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause**. Cấm tuyệt đối mã nguồn có giấy phép GPL / AGPL trong toàn bộ mã nguồn đóng gói nhị phân.
- **Khóa Phiên Bản (Pinned Dependencies):** File `Cargo.lock` được commit vào Git; cập nhật phiên bản phụ thuộc phải đi kèm PR riêng biệt có kiểm thử hồi quy.
