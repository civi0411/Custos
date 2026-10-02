# KHÔNG GIAN LÀM VIỆC: VĨ — CHIEF ARCHITECT & AI/COGNITIVE LEAD
## (Vi Workspace — Core Architecture, Invariants & AI Platform)

> **Kỹ sư phụ trách:** Vĩ (Founder, Chief Architect & AI/DS Lead)  
> **Trạng thái:** Active Maintainer Workspace  
> **Quy chuẩn quản trị tối cao:** Tuân thủ [`AGENTS.md`](../../AGENTS.md) và [`Custos.md`](../../Custos.md)

---

## 1. Trách Nhiệm Cốt Lõi & Các Crate Trực Tiếp Quản Trị

Vĩ là Kiến trúc sư trưởng của Custos, trực tiếp bảo hộ tính toàn vẹn của mô hình miền, máy trạng thái tác vụ, cổng nghiệm thu bằng chứng, và toàn bộ nền tảng AI/Cognition:

| Phân hệ đảm nhiệm | Đường dẫn Workspace | Trách nhiệm kỹ thuật độc quyền |
|---|---|---|
| **Domain Entities** | `crates/custos-domain` | Định nghĩa toàn bộ thực thể thuần khiết (Task, Session, Evidence, Permit). **Bảo hộ bất biến Zero-I/O**. |
| **Task Kernel** | `crates/custos-core` | `TaskStateMachine`, `AuthorityEngine`, `CompletionGate`, và đường ống `ContextCompiler` 8 bước. |
| **Model Contracts** | `crates/custos-provider` | Định nghĩa trait `ProviderPort`, xử lý token streaming, cost tracking, tokenizers. |
| **Composition Root**| `crates/custos-daemon` | **Người duy nhất chỉnh sửa `src/main.rs`**: ráp nối storage, runtime, adapters thành tiến trình chạy. |
| **Domain Packs** | `crates/custos-packs` | Khai báo quy trình làm việc chuẩn cho Engineering Pack, Research Pack, Assistant Pack. |
| **Wire Schemas** | `schemas/` | Quản trị toàn bộ JSON schemas giao tiếp mạng và persistence mapping. |
| **Evaluation Suite**| `evals/` | Bộ benchmark đánh giá chất lượng S1/S2, kiểm thử token budget, độ chính xác của bằng chứng. |

---

## 2. Các Bất Biến Bắt Buộc Tuân Thủ Tuyệt Đối

1. **Bảo vệ Zero-I/O của Domain:** Tuyệt đối không chấp nhận bất kỳ thư viện async runtime, filesystem, mạng, hoặc database nào (`tokio`, `reqwest`, `rusqlite`, `std::fs`) lọt vào `custos-domain`.
2. **Quyền hạn Composition Root:** Mọi module do Trường và Vinh phát triển đều ở dạng thư viện (library crates). Chỉ có Vĩ mới thực hiện liên kết các module này vào `custos-daemon`.
3. **Chống Hallucination Bằng Chứng:** Cổng `CompletionGate` trong `custos-core` không bao giờ chấp nhận claim hoàn thành nếu thiếu chữ ký xác thực từ Verifier tin cậy.

---

## 3. Lệnh Phát Triển & Kiểm Thử Thường Dùng (Daily Runbook)

```bash
# Kiểm tra nhanh tính toàn vẹn của các crate thuộc quyền quản lý
cargo check -p custos-domain -p custos-core -p custos-provider -p custos-packs

# Chạy test suite logic miền và máy trạng thái
cargo test -p custos-domain --lib
cargo test -p custos-core --lib

# Kiểm tra Composition Root daemon
cargo check -p custos-daemon
RUST_LOG=debug cargo run -p custos-daemon -- --dry-run
```

---

## 4. Danh Sách Nhiệm Vụ Sprint Hiện Tại (Gate 1 Focus)

- [ ] Hiện thực hóa trọn vẹn đường ống đóng gói `ContextPack` chuẩn trong `custos-core`.
- [ ] Hoàn thiện trait `ProviderPort` và cơ chế streaming tokens trong `custos-provider`.
- [ ] Phối hợp với Trường nghiệm thu hợp đồng lưu trữ `TaskRepository` trên SQLite WAL.
- [ ] Phối hợp với Vinh kiểm thử luồng thăng cấp `Session-to-Task` qua Local API.

---

## 5. Cấu Trúc Không Gian Làm Việc

- `notes/`: Ghi chú kỹ thuật, công thức tối ưu hóa prompt, thuật toán cắt lát AST Tree-sitter, phân bổ ngân sách token.
- `reports/`: Kết quả benchmark mô hình, báo cáo đánh giá evals, biên bản nghiệm thu kiến trúc.
