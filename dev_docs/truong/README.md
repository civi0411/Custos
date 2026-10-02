# KHÔNG GIAN LÀM VIỆC: TRƯỜNG — SYSTEMS & PERSISTENCE LEAD
## (Truong Workspace — Storage, Persistence & OS Sandboxing)

> **Kỹ sư phụ trách:** Nguyễn Đinh Nhật Trường (Truong — Systems & Persistence Lead)  
> **Trạng thái:** Active Maintainer Workspace  
> **Quy chuẩn quản trị tối cao:** Tuân thủ [`AGENTS.md`](../../AGENTS.md) và [`Custos.md`](../../Custos.md)

---

## 1. Trách Nhiệm Cốt Lõi & Các Crate Trực Tiếp Quản Trị

Trường là Trưởng nhóm Hạ tầng Lưu trữ và An toàn Hệ thống, trực tiếp sở hữu độ bền vững quan hệ SQLite WAL, chống đói WAL (P0), cơ chế Transactional Outbox, và cách ly an toàn cấp OS:

| Phân hệ đảm nhiệm | Đường dẫn Workspace | Trách nhiệm kỹ thuật độc quyền |
|---|---|---|
| **Durable Storage** | `crates/custos-persistence` | Hiện thực hóa các Domain Repository traits, SQLite WAL mode, schema migrations, Outbox, CAS. |
| **OS Sandboxes** | `crates/custos-adapters/src/sandbox` | Cách ly tiến trình cấp OS: macOS Seatbelt (`sandbox-exec`) và Linux Bubblewrap (`bwrap`). |
| **Crash Resilience**| `tests/crash`, `tests/contract` | Test suite giả lập sập nguồn `SIGKILL` trước/sau commit transaction, kiểm thử checkpoint WAL. |

---

## 2. Các Bất Biến Bắt Buộc Tuân Thủ Tuyệt Đối

1. **Cô Lập Kiểu Dữ Liệu SQLite:** Tuyệt đối không để kiểu kết nối `rusqlite::Connection` hoặc kiểu dữ liệu SQLite rò rỉ ra khỏi `custos-persistence`. Mọi truy cập phải qua Trait trừu tượng do Vĩ định nghĩa.
2. **Triệt Tiêu Lỗi Đói WAL (WAL Starvation Prevention - P0):**
   - Phải thiết lập `busy_timeout = 5000ms`.
   - Bắt buộc phân tách rõ: 1 kết nối ghi chuyên dụng duy nhất (Dedicated Single Writer) và nhóm kết nối đọc (Read-only Pool).
   - Kiểm tra phiên bản SQLite lúc khởi động (Runtime Version Check).
3. **Bảo Đảm Transactional Outbox:** Mọi tác động ngoại vi (side-effect) phải được ghi đồng thời vào bảng `outbox_messages` trong cùng một transaction với Task state trước khi được dispatch.

---

## 3. Lệnh Phát Triển & Kiểm Thử Thường Dùng (Daily Runbook)

```bash
# Kiểm tra biên dịch crate persistence và sandbox adapter
cargo check -p custos-persistence
cargo check -p custos-adapters --features sandbox

# Chạy unit tests cho persistence layer
cargo test -p custos-persistence --lib

# Chạy test suite kiểm tra tranh chấp SQLite WAL
cargo test -p custos-persistence --test wal_concurrency

# Chạy test suite mô phỏng sập nguồn đột ngột (SIGKILL)
cargo test -p custos-persistence --test crash_recovery
```

---

## 4. Danh Sách Nhiệm Vụ Sprint Hiện Tại (Gate 1 Focus)

- [ ] Hoàn thành việc khóa phiên bản SQLite runtime và bổ sung kiểm tra version lúc khởi động (`rusqlite` version guard).
- [ ] Thiết lập kịch bản kiểm thử sập nguồn `SIGKILL` trước và sau commit transaction trong `tests/crash`.
- [ ] Tinh chỉnh tham số checkpoint WAL để đảm bảo file `-wal` không bị phình to vô hạn.
- [ ] Phối hợp với Vĩ xác thực việc lưu trữ `EvidenceRecord` và `TaskState` qua `TaskRepository`.

---

## 5. Cấu Trúc Không Gian Làm Việc

- `notes/`: Ghi chú kỹ thuật SQLite WAL, kết quả đo kiểm tranh chấp I/O, thử nghiệm cấu hình Bubblewrap/Seatbelt.
- `reports/`: Biên bản nghiệm thu kiểm thử sập nguồn, báo cáo benchmark hiệu năng đọc/ghi đĩa.
