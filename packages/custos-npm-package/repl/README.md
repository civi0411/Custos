# Custos Terminal Interactive CLI (`packages/custos-npm-package/repl`)

> **Interactive Node.js Terminal Shell & Developer REPL**  
> **Launcher**: `repl/custos-cli.js` (`npm run repl` hoặc `npm run cli` từ repo root)  
> **API Backend**: Custos Daemon Loopback HTTP/JSON-RPC (`http://127.0.0.1:<port>`)

---

## 1. Phân Định Kiến Trúc (Architecture Demarcation)

- **Interactive Node Shell (`repl/custos-cli.js`)**:
  - Dành cho nhà phát triển muốn trải nghiệm dòng lệnh tương tác dạng console REPL (`@inquirer/prompts`, `ora`, `chalk`).
  - Giao tiếp trực tiếp với daemon nền tảng qua Local API HTTP loopback.
- **Native Headless Rust CLI (`apps/cli`)**:
  - Binary native chính thức (`custos`) cho CI/CD, automation script, và quản trị headless. Xem [`apps/cli/README.md`](../../../apps/cli/README.md).
- **Official Desktop GUI (`apps/desktop/ui`)**:
  - Toàn bộ giao diện đồ họa chính thức của sản phẩm Custos nằm tại [`apps/desktop/ui`](../../../apps/desktop/ui).
  - Tauri desktop trỏ trực tiếp vào `apps/desktop/ui/dist`.

---

## 2. Cách Chạy (Quickstart)

```bash
# Khởi chạy interactive terminal console từ root
npm run cli

# Hoặc chạy trực tiếp từ package
npm run repl

# Xem danh sách lệnh trợ giúp
npm run repl -- help
```

Trong chế độ tương tác, gõ phím `/` để mở bảng chọn lệnh (searchable command palette) và dùng các phím mũi tên để điều hướng.
