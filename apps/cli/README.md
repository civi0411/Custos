# Custos Native Headless CLI (`custos-app-cli`)

> **Authoritative Specification & Architecture Guide for CLI Engineering**  
> **Package**: `custos-app-cli` (`Cargo.toml`)  
> **Binary Name**: `custos`  
> **Architecture Pattern**: Thin Headless Client over `custos-sdk` and `custos-protocol` (Zero-Direct-Persistence, Zero-Daemon-Runtime)

---

## 1. Vai Trò Kiến Trúc & Phân Định Ranh Giới (SSOT Boundary)

Custos có 2 công cụ dòng lệnh với phân nhiệm rõ ràng:
1. **`apps/cli` (Native Rust CLI — Đây là thư mục này)**:
   - **Mục đích**: Client headless native chính thức phục vụ automation, shell scripting, CI/CD pipelines, và quản lý task tự động.
   - **Công nghệ**: Pure-Rust (`clap`, `serde_json`, `tokio`).
   - **Giao tiếp**: Kết nối tới daemon đang chạy qua `custos_sdk::LocalApiClient` trên loopback (`127.0.0.1`).
   - **Bảo mật**: Tuyệt đối **không** nhúng database SQLite hay đọc trực tiếp Vault credentials. Mọi thao tác đều thông qua API của `custosd`.
2. **`ui/cli/bin/custos-cli.js` (Interactive Node.js Shell)**:
   - **Mục đích**: Công cụ REPL tương tác console cho developer thích giao diện terminal với prompt động (`@inquirer/prompts`, `ora`, `chalk`).

---

## 2. Cấu Trúc Nhóm Lệnh Chuẩn (Command Hierarchy)

```bash
custos [OPTIONS] <COMMAND>

Commands:
  ping          Kiểm tra sức khoẻ kết nối với daemon (v1.ping)
  capabilities  Liệt kê các năng lực đã đăng ký của hệ thống (v1.capabilities.list)
  tasks         Quản lý vòng đời task
    ├── list    Liệt kê danh sách tasks (v1.tasks.list)
    ├── get     Xem chi tiết một task theo ID (v1.tasks.get)
    ├── create  Tạo task mới (v1.tasks.create)
    ├── advance Chuyển trạng thái task (v1.tasks.advance)
    └── cancel  Huỷ task đang chạy (v1.tasks.cancel)
  supervision   Truy vấn chẩn đoán và giám sát vận hành
    ├── overview  Tổng quan số lượng run, lỗi và profile hoạt động
    ├── runs      Liệt kê các lượt chạy tác vụ kèm bộ lọc status/workbench
    ├── inspect   Xem chi tiết waterfall steps, logs, errors của một run
    ├── findings  Liệt kê các phát hiện bất thường từ deterministic rules
    ├── errors    Tổng hợp các nhóm lỗi phổ biến theo chữ ký số
    └── retries   Xem danh sách các lần thử lại và kết quả
  request       Gửi payload JSON-RPC tuỳ ý tới bất kỳ phương thức nào của daemon
```

---

## 3. Quy Tắc Phát Triển Dành Cho Kỹ Sư Tiếp Quản (Developer Guidelines)

1. **Tuân thủ Clean Client Boundary**:
   - Chỉ phụ thuộc vào `custos-contracts` và `custos-sdk`.
   - Không được thêm dependency vào `custos-storage`, `custos-engine`, `custos-integrations`, hay `rusqlite`.
2. **Tuân thủ Chuẩn Ponytail Senior Dev**:
   - Giữ mã nguồn tinh gọn, trực quan, không tạo abstraction wrapper thừa.
   - Không tệp tin nào được vượt quá **600 dòng mã (LOC)**. Nếu mở rộng thêm subcommands, hãy tách thành các module con trong `src/commands/`.
3. **Quản lý Session & Profile**:
   - Sử dụng `ProfileResolver::from_env()` để phát hiện profile và cổng kết nối daemon (hỗ trợ cờ `--profile <id>`).
   - Nếu daemon chưa chạy, client có thể sử dụng `ensure_daemon_client_async` để báo lỗi rõ ràng hoặc hỗ trợ khởi động daemon.

