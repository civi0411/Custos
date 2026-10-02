# Quy Tắc Đồng Bộ Kiến Trúc & Tài Liệu Dành Cho AI Coding Agents
## (Architecture Decision & Documentation Synchronization Triad)

> **Phạm vi áp dụng:** Toàn bộ AI Coding Agents (Cursor, Claude Code, Antigravity, Copilot, Windsurf) khi làm việc trên repository Custos.  
> **Nguyên tắc tối thượng:** Bất kỳ thay đổi hoặc thống nhất mới nào về mặt kiến trúc hệ thống đều BẮT BUỘC phải được cập nhật đồng bộ vào 3 tầng tài liệu trước khi được phép viết mã nguồn.

---

### 1. Mô Hình 3 Tầng Tài Liệu (The Documentation Triad)

Mọi thông tin kiến trúc trong Custos được phân bổ nghiêm ngặt vào đúng 3 tầng:

1. **Tầng 1 — `Custos.md` (Root Master Specification — SSOT):**
   - **Bản chất:** Nguồn chân lý duy nhất (Single Source of Truth) định nghĩa triết lý sản phẩm, 10 Bất biến hệ thống, State Machines, Trust Zones, Threat Defense, và Protocol Specifications.
   - **Quy tắc:** Khi thống nhất quyết định kiến trúc mới, phải tìm đúng Chương/Chủ đề liên quan trong `Custos.md` (ví dụ: Phần 2 cho Tầng kiến trúc, Phần 3 cho Kernel/Task, Phần 4 cho Authority, Phần 15 cho Repo Boundaries) để cập nhật hoặc bổ sung.

2. **Tầng 2 — Thư mục `docs/` (Authoritative Topic-Based Engineering Specifications):**
   - **Bản chất:** Đặc tả kỹ thuật chuyên sâu theo từng chủ đề:
     - `docs/architecture/`: 9 trụ cột kiến trúc (Kernel, Authority, Evidence, Memory, Protocols, Security, Orchestration, Packs).
     - `docs/reference/`: Invariants, Naming Conventions, Schema Mapping, Glossary.
     - `docs/development/`: Quy chuẩn kỹ thuật, Delivery Blueprint, Testing.
   - **Quy tắc:** Cập nhật hoặc tạo mới tài liệu tương ứng đúng chủ đề, không để tài liệu kỹ thuật bị lệch pha so với `Custos.md`.

3. **Tầng 3 — `docs/development/codebase-architecture.md` (Master Physical Codebase Catalog):**
   - **Bản chất:** Bản đồ vật lý chi tiết đến từng file mã nguồn, quản lý vị trí, số dòng code, vai trò kiến trúc và danh mục struct/trait/hàm cốt lõi của 11 canonical product crates.
   - **Quy tắc:** Mọi file mới hoặc module được thêm/sửa/xóa đều phải được ghi nhận và chèn đúng vào bảng danh mục của crate tương ứng trong tài liệu này.

---

### 2. Quy Trình 5 Bước Bắt Buộc (Mandatory 5-Step Workflow)

Khi có bất kỳ thay đổi kiến trúc nào:

```
[Đồng thuận Kiến trúc với User/Team]
               │
               ▼
   [Bước 1: Cập nhật Custos.md (SSOT)]
               │
               ▼
   [Bước 2: Cập nhật docs/ đúng chủ đề]
               │
               ▼
   [Bước 3: Chèn đúng chỗ vào docs/development/codebase-architecture.md]
               │
               ▼
   [Bước 4: Triển khai Code & Chạy Kiểm thử cargo check / cargo test]
```

- **Bước 1: Đồng thuận Kiến trúc:** Xác định rõ thay đổi thuộc về crate nào trong 11 canonical crates, vi phạm hay tuân thủ bất biến nào, và ai là người sở hữu (Vĩ / Trường / Vinh).
- **Bước 2: Cập nhật `Custos.md`:** Cập nhật nội dung vào đúng chương mục trong `Custos.md`.
- **Bước 3: Cập nhật `docs/`:** Cập nhật chi tiết kỹ thuật vào đúng file chuyên đề trong `docs/`.
- **Bước 4: Chèn đúng chỗ vào `docs/development/codebase-architecture.md`:**
  - Xác định đúng Layer (Layer 0 đến Layer 4) và Crate (`crates/custos-...`).
  - Chèn dòng mới vào bảng Danh mục file của Crate theo định dạng 4 cột chuẩn:
    `| Cột 1: Link file mã nguồn | Cột 2: Số dòng | Cột 3: Vai trò kiến trúc | Cột 4: Các Struct / Trait / Hàm cốt lõi |`
    (ví dụ cụ thể: `| [`src/action.rs`](../../crates/custos-domain/src/action.rs) | 163 | Vai trò... | Các Struct... |`)
  - Cập nhật số lượng file và số dòng mã ở tiêu đề của crate đó.
- **Bước 5: Viết code và xác thực:** Viết code, chạy `cargo check --workspace` và `cargo test`. Tuyệt đối không viết code trước khi hoàn tất Bước 1, 2, 3, 4!

---

### 3. Quy Tắc Đọc Dành Cho AI Coding Agents (Shared Mental Model)

Trước khi thực hiện bất kỳ lệnh sửa đổi mã nguồn nào:
1. **Đọc `docs/development/codebase-architecture.md` trước tiên:**
   - Để biết chính xác file mình sắp sửa nằm ở crate nào, tầng nào, và đang chịu ràng buộc gì.
   - Để tránh việc tự ý tạo thêm file mới bừa bãi hoặc tạo các thư mục rác (như `scratch/`, `templates/`, `services/`).
2. **Không tự ý suy đoán ranh giới (Zero Guesswork):**
   - Nếu một struct hay trait chưa rõ nên thuộc về `custos-domain` hay `custos-core`, hãy tra cứu `Custos.md` và `docs/development/codebase-architecture.md`.
3. **Cấm tuyệt đối:**
   - Không được tạo crate mới ngoài 11 canonical product crates.
   - Không được phá vỡ tính bất biến Zero-I/O của `custos-domain`.
   - Không được để `custos-bridge` gọi trực tiếp vào `custos-persistence`.
