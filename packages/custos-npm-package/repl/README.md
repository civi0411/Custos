# Custos Terminal Interactive CLI (`ui/cli`)

> **Interactive Node.js Terminal Shell & Developer REPL**  
> **Launcher**: `bin/custos-cli.js` (`npm start`)  
> **API Backend**: Custos Daemon Loopback HTTP/JSON-RPC (`http://127.0.0.1:<port>`)

---

## 1. Phân Định Kiến Trúc (Architecture Demarcation)

- **Interactive Node Shell (`bin/custos-cli.js`)**:
  - Dành cho nhà phát triển muốn trải nghiệm dòng lệnh tương tác dạng console REPL (`@inquirer/prompts`, `ora`, `chalk`).
  - Giao tiếp trực tiếp với daemon nền tảng qua Local API HTTP loopback.
- **Native Headless Rust CLI (`apps/cli`)**:
  - Binary native chính thức (`custos`) cho CI/CD, automation script, và quản trị headless. Xem [`apps/cli/README.md`](../../apps/cli/README.md).
- **Official Desktop GUI (`ui/desktop`)**:
  - Toàn bộ giao diện đồ họa chính thức của sản phẩm Custos nằm tại [`ui/desktop`](../desktop).
  - *Lưu ý về `ui/cli/src`*: Các thành phần React web trong thư mục `src/` là mã nguyên mẫu thử nghiệm ban đầu (archived web prototype) và **không** nằm trong đường dẫn build sản phẩm phân phối (Tauri desktop trỏ trực tiếp vào `ui/desktop/dist`).

---

## 2. Cách Chạy (Quickstart)

```bash
# Khởi chạy interactive terminal console
npm start

# Xem danh sách lệnh trợ giúp
npm start -- help

# Kiểm tra trạng thái kết nối daemon
npm start -- status
```

Trong chế độ tương tác, gõ phím `/` để mở bảng chọn lệnh (searchable command palette) và dùng các phím mũi tên để điều hướng.
