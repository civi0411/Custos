# Triển Khai Cục Bộ & Môi Trường (Local-First Deployment Architecture)

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần VI (§28, §34) & Phần VII (§47) Canonical Specification

Custos được thiết kế hoàn toàn theo tư duy **Local-First**: bảo đảm dữ liệu của người dùng nằm trọn vẹn trên máy trạm của họ, không đòi hỏi phụ thuộc vào bất kỳ hạ tầng đám mây tập trung nào.

---

## 1. Kiến Trúc Tiến Trình Cục Bộ

Hệ thống bao gồm một daemon nền duy nhất và các ứng dụng client mỏng kết nối qua Unix Domain Socket:

```text
┌─────────────────────────────────────────────────────────────┐
│                      LOCAL WORKSTATION                      │
│                                                             │
│   [ custos CLI ]               [ VS Code Extension ]        │
│          │                              │                   │
│          └──────────────┬───────────────┘                   │
│                         │ JSON-RPC (Unix Socket)            │
│                         ▼                                   │
│            ┌─────────────────────────┐                      │
│            │  custosd (Rust Daemon)  │                      │
│            └────────────┬────────────┘                      │
│                         │                                   │
│       ┌─────────────────┼─────────────────┐                 │
│       ▼                 ▼                 ▼                 │
│  [ SQLite DB ]    [ Git Worktrees ]  [ Tiered Sandbox ]     │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Các Tầng Sandbox (Tiered Sandboxing Backends)

Custos phân tầng sandbox dựa trên hệ điều hành của máy trạm:

### Tier 1: macOS Native Sandbox (`sandbox-exec`)
Trên macOS, mọi lệnh shell và công cụ được thực thi dưới cấu hình **Seatbelt** nghiêm ngặt:
- Chỉ cho phép đọc/ghi vào thư mục Git worktree được chỉ định.
- Cấm đọc các thư mục cá nhân nhạy cảm (`~/.ssh`, `~/.aws`, `~/Library/Keychains`).
- Khóa toàn bộ kết nối mạng ngoại trừ các domain đã được cấp phép trong task contract.

### Tier 2: Linux Native Sandbox (`bubblewrap` / Namespaces)
Trên Linux, Custos tận dụng `bwrap` (công nghệ đứng sau Flatpak):
- Tạo unshare mount/network/PID namespaces riêng rẽ.
- Môi trường root ảo hóa chỉ chứa các thư viện tối thiểu cần thiết để build/test code.

### Tier 3: OCI / Docker Container (Fallback & Polyglot)
Dành cho các tác vụ đòi hỏi môi trường dịch vụ phức tạp (ví dụ: cần khởi động PostgreSQL hoặc Redis cục bộ để chạy integration test).

---

## 3. Định Mức Tài Nguyên & Giám Sát Cục Bộ

Daemon `custosd` tự áp đặt giới hạn tài nguyên nghiêm ngặt:
- **Dung lượng RAM tối đa:** `< 60MB` khi nhàn rỗi, `< 250MB` khi điều phối tác vụ nặng.
- **CPU:** Tự động điều chỉnh độ ưu tiên (`nice` level) để không làm đơ giao diện người dùng máy trạm khi chạy các bài test dài.
