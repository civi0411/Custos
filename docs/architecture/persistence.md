# Lưu Trữ Bền Vững & Dữ Liệu (Persistence Architecture)

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần V (§21) Canonical Specification

Custos tuân thủ triệt để nguyên lý **Local-First & Durable by Design**: toàn bộ dữ liệu trạng thái, lịch sử sự kiện và sản phẩm tạo ra đều được lưu trữ bền vững tại máy trạm cá nhân, hỗ trợ giao dịch ACID và sẵn sàng khôi phục ngay lập tức sau sự cố.

---

## 1. Các Trụ Cột Lưu Trữ

1. **SQLite (Core Metadata & Event Store):** Sử dụng SQLite nhúng với chế độ **Write-Ahead Logging (`WAL`)**, khóa an toàn `busy_timeout = 5000ms`, bảo đảm toàn vẹn giao dịch và hiệu năng đọc ghi song song cao.
2. **Content-Addressable Storage (CAS):** Các payloads có dung lượng lớn (diff lớn, tệp nhị phân, bundles, logs kiểm thử) không lưu trực tiếp trong bảng SQLite mà được ghi vào thư mục CAS (`.custos/cas/`) với tên tệp là mã băm `SHA-256` của nội dung.
3. **Outbox Pattern:** Mọi tác động ra bên ngoài (gửi webhook, phát sự kiện ra client) đều được ghi vào bảng `outbox` trong cùng một giao dịch cơ sở dữ liệu với sự kiện chính, bảo đảm nguyên tắc *At-Least-Once Delivery*.

---

## 2. Cấu Trúc Bảng Chi Tiết (Core Schema DDL)

```sql
-- Kích hoạt chế độ WAL và ràng buộc khóa ngoại
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;

-- 1. Bảng quản trị Task
CREATE TABLE IF NOT EXISTS tasks (
    task_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    intent TEXT NOT NULL,
    status TEXT NOT NULL,
    contract_json TEXT NOT NULL,
    budget_tokens_limit INTEGER NOT NULL,
    budget_usd_limit REAL NOT NULL,
    tokens_consumed INTEGER DEFAULT 0,
    cost_usd_consumed REAL DEFAULT 0.0,
    worktree_path TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 2. Sổ cái sự kiện bất biến (Event Store)
CREATE TABLE IF NOT EXISTS task_events (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL REFERENCES tasks(task_id) ON DELETE CASCADE,
    sequence_no INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    UNIQUE(task_id, sequence_no)
);

-- 3. Bảng các lần chạy (Runs)
CREATE TABLE IF NOT EXISTS runs (
    run_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(task_id) ON DELETE CASCADE,
    run_number INTEGER NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    UNIQUE(task_id, run_number)
);

-- 4. Bảng các bước thực thi (Steps)
CREATE TABLE IF NOT EXISTS steps (
    step_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(run_id) ON DELETE CASCADE,
    step_number INTEGER NOT NULL,
    role_name TEXT NOT NULL,
    status TEXT NOT NULL,
    action_type TEXT NOT NULL,
    action_payload_hash TEXT,
    receipt_id TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(run_id, step_number)
);

-- 5. Bảng Sổ cái Quyết định (Decision Ledger)
CREATE TABLE IF NOT EXISTS decision_ledger (
    decision_id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(task_id),
    step_id TEXT REFERENCES steps(step_id),
    question_id TEXT NOT NULL,
    decision_outcome TEXT NOT NULL,
    confidence_score REAL NOT NULL,
    rationale TEXT NOT NULL,
    decided_by TEXT NOT NULL, -- 'system_one' hoặc 'human_principal'
    created_at TEXT NOT NULL
);

-- 6. Bảng Outbox cho thông điệp bất đồng bộ
CREATE TABLE IF NOT EXISTS outbox_events (
    outbox_id INTEGER PRIMARY KEY AUTOINCREMENT,
    destination TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    status TEXT DEFAULT 'PENDING',
    created_at TEXT NOT NULL,
    sent_at TEXT
);
```

---

## 3. Chiến Lược Sao Lưu & Phục Hồi (Backup & Disaster Recovery)

- **Hot Backup Trực Tuyến:** Custos sử dụng SQLite Backup API để sao lưu định kỳ cơ sở dữ liệu sang tệp `.custos/backup/custos_snapshot.db` mà không làm gián đoạn các luồng đọc/ghi.
- **Toàn Vẹn CAS:** Thư mục Content-Addressable Storage có thể được đồng bộ hoặc phục hồi dễ dàng bằng các công cụ tệp tiêu chuẩn (`rsync`, `rclone`).
