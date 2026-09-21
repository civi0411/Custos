# 🧑‍🔧 Không Gian Làm Việc Của Trường (Software Engineer)

> **Vai trò:** Backend & Systems Engineer  
> **Lãnh địa phụ trách:** `crates/persistence-sqlite`, `crates/capability-gateway`, `crates/local-api`, `apps/custos-cli`, `apps/custosd`.

---

## 🎯 Trọng Tâm Công Việc
1. **Persistence (SQLite):**
   - Thiết kế bảng `tasks`, `spans`, `events` với WAL mode và crash resilience.
   - Viết các hàm CRUD bằng Rust (`rusqlite` hoặc `sqlx`) dựa trên các struct do Vĩ định nghĩa trong `crates/core-domain`.
2. **Local API & CLI:**
   - Xây dựng CLI binary `custos-cli` bằng `clap` (nhận lệnh từ người dùng).
   - Xây dựng HTTP daemon `custosd` hoặc local server bằng `axum`.
3. **Capability Gateway & Sandbox:**
   - Thực thi an toàn: Quản lý worktree, hạn chế quyền ghi/đọc, chặn truy cập mạng trái phép bằng Seatbelt (macOS) và Bubblewrap (Linux).

---

## 📁 Cấu Trúc Thư Mục Cá Nhân
- `notes/`: Ghi chú thiết kế DB migration, benchmark hiệu năng I/O, lệnh CLI, cấu hình sandbox.
- `reports/`: Báo cáo tiến độ backend theo ngày/tuần để chia sẻ cho Vĩ review trên nhánh `report`.

---

## 📋 Checklist Hiện Tại
- [ ] Khởi tạo kết nối SQLite với cấu hình WAL trong `crates/persistence-sqlite`.
- [ ] Viết migration tạo bảng `tasks`.
- [ ] Implement `insert_task` và `get_task`.
