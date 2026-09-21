# 🚀 Sprint Status & Sync (Vĩ & Trường)

> File này được dùng để 2 thành viên cập nhật nhanh trạng thái công việc và ghi nhận blocker.  
> Cứ hoàn thành việc nào thì tick `[x]` vào ô tương ứng.

---

## 📍 Sprint 1: Local Kernel & SQLite (Tuần Hiện Tại)
**Mục tiêu chung:** Xây dựng xong bộ khung lưu trữ dữ liệu (Task) xuống SQLite và có thể dùng CLI để tạo/xem task trên máy local. Chưa cần gọi AI bên ngoài.

### 🧑‍🔧 Nhiệm Vụ Của Trường (Software Engineer)
- [ ] Thiết lập kết nối `rusqlite` hoặc `sqlx` trong `crates/persistence-sqlite`.
- [ ] Viết hàm tạo bảng (Schema Migration) cho: `tasks`, `spans`, `events`.
- [ ] Viết API `insert_task(task: &Task) -> Result<(), PersistenceError>` và `get_task(id: &TaskId) -> Result<Option<Task>, PersistenceError>`.
- [ ] Dùng `clap` trong `apps/custos-cli` để nhận lệnh `custos run "mô tả công việc"`.
- [ ] Gọi hàm `insert_task` từ CLI xuống SQLite để test thử luồng chạy thực tế.

### 🧑‍💻 Nhiệm Vụ Của Vĩ (AI Engineer)
- [ ] Bổ sung các trường dữ liệu cần thiết cho struct `Task` và enum `TaskStatus` trong `crates/core-domain`.
- [ ] Chốt chuẩn giao tiếp (JSON Schema / Action Payload) giữa Kernel và Cognitive Arbiter.
- [ ] Khởi tạo khung thư mục `sidecars/python-judgment` (thiết lập pyproject.toml / uv).
- [ ] Khởi tạo khung thư mục `sidecars/ts-claude-agent` (thiết lập package.json / pnpm).

---

## 📍 Ghi Chú & Blocker (Trở Ngại Cần Họp Thảo Luận)

### Trường (SE)
- *Ghi chú / Blocker:* (Chưa có)

### Vĩ (AI Engineer)
- *Ghi chú / Blocker:* (Chưa có)
