# Gói Nghiệp Vụ Kỹ Thuật (Engineering Domain Pack v1)

> **Status:** Canonical Baseline v4.0 (Core Focus H1)  
> **Source:** Phần VI (§18) & Phần VII (§54) Canonical Specification

Engineering Domain Pack v1 là trọng tâm phát triển hàng đầu của Custos trong Horizon 1 (H1), cung cấp môi trường khép kín, an toàn và có kiểm chứng cho các tác vụ kỹ thuật phần mềm (sửa lỗi, tái cấu trúc, viết tính năng mới).

---

## 1. Pipeline Thực Thi Kỹ Thuật (Engineering Pipeline)

Mỗi tác vụ kỹ thuật đi qua 8 giai đoạn nghiêm ngặt:

```text
[ 1. Intake & Contract ]
         │
[ 2. Repo Intelligence ] ───> (Tree-sitter AST + ripgrep search + Git status)
         │
[ 3. Scoped ContextPack] ───> (Xếp hạng liên quan + Giới hạn ngân sách token)
         │
[ 4. Change Plan       ] ───> (Phân tích ảnh hưởng + Đề xuất giải pháp)
         │
[ 5. Isolated Worktree ] ───> (Tạo git worktree riêng, không chạm vào branch chính)
         │
[ 6. Verification Loop ] ───> (Chạy Linters + Typecheckers + Unit Tests)
         │
[ 7. Evidence Bundle   ] ───> (Đóng gói diff, receipts kiểm thử, chỉ số chi phí)
         │
[ 8. Human Review Gate ] ───> (Người dùng xem trước diff trước khi merge vào main)
```

---

## 2. Các Vai Trò Worker Ngắn Hạn (Ephemeral Worker Roles)

Trong Custos, không có một "Coding Agent" nguyên khối giữ quyền hạn từ đầu đến cuối. Thay vào đó, Kernel sinh ra các worker theo vai trò cụ thể:

| Vai trò (Role) | Bản chất | Công cụ được cấp phép | Hiệu ứng (Effect) | Cấp độ rủi ro |
|---|---|---|---|---|
| `engineering.explorer` | Deterministic / Fast | `repo.list_files`, `git_status`, `ripgrep` | Chỉ đọc (Read-only) | **R0** (An toàn) |
| `engineering.planner` | Deliberation (LLM) | `repo.read_symbol`, `search_fts5` | Chỉ đọc (Read-only) | **R0** (An toàn) |
| `engineering.patcher` | Agent Loop | `repo.read_symbol`, `apply_patch_to_worktree` | Ghi cục bộ (Local write) | **R1** (Có kiểm soát) |
| `engineering.test_author`| Deliberation (LLM) | `write_test_draft` | Ghi cục bộ (Local write) | **R1** (Có kiểm soát) |
| `engineering.verifier`| Deterministic runner | `cargo test`, `ruff`, `mypy`, `pytest` | Chạy sandbox | **R0** (Cách ly) |
| `engineering.reviewer`| Deliberation (LLM) | `git_diff_summary` | Chỉ đọc (Read-only) | **R0** (An toàn) |

---

## 3. Trí Tuệ Kho Mã Nguồn (Repository Intelligence)

Thay vì gửi mù quáng toàn bộ codebase lên LLM, Custos tích hợp bộ công cụ phân tích tĩnh cực nhanh chạy trực tiếp trên máy trạm:
- **`BurntSushi/ripgrep`:** Tìm kiếm văn bản và biểu thức chính quy với tốc độ hàng chục GB/giây.
- **`tree-sitter/tree-sitter` & `ast-grep`:** Phân tích cú pháp cây AST, trích xuất danh sách hàm, structs, classes, và tham chiếu liên tệp tin.
- **`git worktree`:** Tạo bản sao không gian làm việc cục bộ trong tích tắc mà không tốn dung lượng đĩa nhân bản, bảo đảm nhánh chính của lập trình viên không bao giờ bị xáo trộn.
