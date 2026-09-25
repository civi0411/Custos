# Báo cáo Tiến độ Tuần 02 — NPM Wrapper Packaging, Update Notification & Release Pipeline

> **Người thực hiện:** Nguyễn Đinh Nhật Trường (Truong)  
> **Vai trò:** Core Platform & Security Lead  
> **Chu kỳ:** Sprint 2 — CLI Distribution, Auto-Update Experience & CI/CD Release Pipeline  
> **Thời gian:** 25/09/2026 – 01/10/2026  
> **Nhánh phát triển:** `truong` ➔ `dev`  

---

## 1. Tổng quan Mục tiêu & Kết quả Tuần 02

Tiếp nối nền tảng Core Kernel và đồ họa Mascot ở Tuần 01, trọng tâm của Tuần 02 chuyển sang trải nghiệm phân phối sản phẩm đến tay người dùng cuối (Distribution & Developer Experience):

1. **Đóng gói CLI Chuẩn NPM Wrapper (`custos-cli`):** Đóng gói ứng dụng dòng lệnh Rust dưới dạng gói NPM nhẹ, giúp người dùng cài đặt toàn cầu chỉ với một lệnh `npm install -g custos-cli` hoặc chạy nhanh tức thì `npx custos-cli` mà không cần cài đặt Rust toolchain hay Cargo.
2. **Hệ thống Thông báo Cập nhật Tự động (Update Notifier):** Tích hợp khung thông báo cập nhật phiên bản mới bằng tiếng Anh trang nhã với viền vàng bo góc UTF-8, hiển thị trực quan ngay khi người dùng khởi động giao diện Custos CLI UI.
3. **Cơ chế Kiểm tra Ngầm Không Trễ (Non-blocking & Zero-Latency):** Kiểm tra phiên bản mới từ NPM Registry trong luồng nền, lưu cache 24 giờ cục bộ tại `~/.custos/update-check.json`, tự động ẩn khi xuất dữ liệu `--json` hoặc trong môi trường CI/CD.
4. **Tự động hóa CI/CD & Chuẩn hóa Quy trình Phát hành (Release Pipeline):** Xây dựng workflow GitHub Actions (`release-cli.yml`) và chuẩn hóa tài liệu hướng dẫn từng bước phát hành bản mới lên GitHub Releases và NPM Registry.

---

## 2. Bảng tổng hợp Pull Requests & Commits trong Tuần

| PR / Commit | Thời gian | Mô tả công việc & Thay đổi chính | Trạng thái |
|---|---|---|---|
| **Commit `feat(cli)`** | 2026-09-25 | **Khởi tạo NPM Wrapper (`custos-cli`):** Thiết lập `package.json`, script launcher `bin/index.js`, cơ chế đóng gói `npm pack` và map lệnh `custos`. | Sẵn sàng PR |
| **Commit `feat(ui)`** | 2026-09-25 | **Xây dựng Update Notifier Box:** Bổ sung hàm render khung thông báo chuẩn ANSI, thuật toán SemVer đa tầng (Node.js & Rust Core) và khóa nguyên tử `AtomicBool` chống trùng lặp. | Sẵn sàng PR |
| **Commit `ci(release)`** | 2026-09-25 | **Thiết lập GitHub Actions Release Workflow:** Tạo workflow `.github/workflows/release-cli.yml` tự động build release binary và tạo GitHub Release khi push tag `v*`. | Sẵn sàng PR |

---

## 3. Chi tiết các Hạng mục Đã Thực hiện

### 3.1. Thiết kế Kiến trúc Đóng gói NPM Wrapper (`custos-cli`)

Áp dụng mô hình bọc nhị phân (Binary Wrapper Pattern) phổ biến ở các CLI hiện đại:
```text
custos-npm-package/
├── bin/
│   ├── index.js      <-- Launcher Node.js trung gian (phát hiện OS, kiểm tra update, spawn binary)
│   └── custos.exe    <-- File binary biên dịch release từ Rust
├── .gitignore        <-- Loại trừ file *.exe và *.tgz để không làm nặng Git
├── package.json      <-- Khai báo name: "custos-cli", bin: { "custos": "./bin/index.js" }
└── README.md         <-- Tài liệu hướng dẫn sử dụng và bảng biến môi trường
```

* **Trường `"bin"`:** Gán alias `"custos": "./bin/index.js"`, đảm bảo dù tên gói tải về là `custos-cli` thì câu lệnh người dùng gõ trên Terminal vẫn luôn là `custos`.
* **Kế thừa I/O (`stdio: 'inherit'`):** Giữ nguyên toàn bộ phím bấm tương tác, màu sắc ANSI, bảng diff và đồ họa mascot pixel art của Rust.

---

### 3.2. Hệ thống Thông báo Cập nhật (Auto Update Notifier)

Khi có bản cập nhật mới trên NPM Registry, giao diện CLI UI sẽ hiển thị ngay khung thông báo nổi bật bằng tiếng Anh:

```text
╭───────────────────────────────────────────────────────────────────╮
│                                                                   │
│   Update available 0.1.0-alpha → 0.2.0                            │
│   Run npm i -g custos-cli to update to the latest version         │
│                                                                   │
│   Changelog: https://github.com/civi0411/Custos/releases          │
│                                                                   │
╰───────────────────────────────────────────────────────────────────╯
```

* **Vị trí hiển thị:** Xuất hiện ngay sau Banner linh vật Cú Tuyết, trước khi vào Thẻ chế độ hoạt động (Code, Research, Assistant), giúp người dùng chú ý ngay lập tức.
* **Đồng bộ SemVer 2 tầng:** Cả script Node.js và Rust Core đều tích hợp hàm phân tích `is_newer_version(latest, current)` so khớp chính xác các tiền tố prerelease (`0.1.0-alpha` < `0.1.0` < `0.2.0`).
* **Khóa chống lặp (`AtomicBool`):** Đảm bảo khung thông báo chỉ in đúng 1 lần duy nhất trong suốt phiên làm việc của người dùng.

---

### 3.3. Quy trình Thao tác Phát hành Bản Mới (Release Pipeline)

Dưới đây là các thao tác chuẩn xác cần thực hiện mỗi khi phát hành phiên bản mới:

#### Thao tác 1: Biên dịch File Binary Release Tối ưu
```powershell
cargo build --release -p custos-cli
```
*Tạo file thực thi tối ưu cao tại `target/release/custos.exe`.*

#### Thao tác 2: Sao chép File Thực thi vào Thư mục Đóng gói
```powershell
Copy-Item .\target\release\custos.exe .\custos-npm-package\bin\custos.exe -Force
```
*Đưa binary mới nhất vào thư mục `bin/` của gói `custos-npm-package`.*

#### Thao tác 3: Đẩy Mã nguồn lên Nhánh Phát triển Git
```powershell
git add .
git commit -m "feat(cli): package as custos-cli with auto update notification and release workflow"
git push origin truong
```
*Lưu vết toàn bộ mã nguồn lên nhánh `truong` trên repository `https://github.com/civi0411/Custos.git`.*

#### Thao tác 4: Tạo Git Tag Phiên bản và Đẩy lên GitHub
```powershell
git tag v0.1.0-alpha
git push origin v0.1.0-alpha
```
*Gắn mốc phiên bản phát hành trên Git, đồng thời kích hoạt GitHub Actions tự động.*

#### Thao tác 5: Tạo GitHub Release trên Trình duyệt
1. Truy cập: `https://github.com/civi0411/Custos/releases/new`
2. Chọn tag phiên bản: `v0.1.0-alpha`.
3. Đặt tiêu đề Release và viết Changelog tóm tắt các cải tiến mới.
4. Kéo thả file `custos.exe` và file nén `custos-cli-0.1.0-alpha.tgz` vào mục **Attach binaries by dropping them here**.
5. Nhấn **Publish release**.

#### Thao tác 6: Chuyển vào Thư mục Đóng gói và Chạy Lệnh Publish lên NPM
```powershell
cd "d:\Agentic Work Runtime\Custos\custos-npm-package"
npm publish
```
*Sau khi publish thành công, người dùng toàn cầu chỉ cần gõ `npm install -g custos-cli` hoặc `npx custos-cli` là có thể sử dụng ngay.*

---

### 3.4. Tự động hóa Pipeline với GitHub Actions

Đã cấu hình file workflow [.github/workflows/release-cli.yml](file:///d:/Agentic%20Work%20Runtime/Custos/.github/workflows/release-cli.yml):
* **Trigger:** Tự động kích hoạt mỗi khi có tag mới bắt đầu bằng `v*` được đẩy lên GitHub (`on: push: tags: ['v*']`).
* **Jobs:**
  1. Checkout code và thiết lập toolchain Rust + Node.js.
  2. Chạy `cargo build --release -p custos-cli`.
  3. Đóng gói binary vào thư mục npm wrapper và chạy `npm pack`.
  4. Tự động tạo bản GitHub Release kèm đính kèm sẵn các artifacts binary.
  5. Tự động xuất bản lên NPM Registry nếu cấu hình secret `NPM_TOKEN`.

---

## 4. Kết quả Kiểm thử & Xác minh (Verification)

Mọi chức năng đều được kiểm thử và xác minh thực tế 100%:
- **CLI Subsystem Unit Tests (`custos-cli`):** `13/13` tests PASS (100%).
  - `ui::art::tests::test_compute_showcase_dims_no_overflow` ✅
  - `ui::art::tests::test_responsive_tiers` ✅
  - `ui::banner::tests::test_is_newer_version` ✅
  - `ui::banner::tests::test_print_update_notification_executes` ✅
  - `ui::art::tests::test_print_mode_cards_executes` ✅
  - `ui::art::tests::test_wide_banner_frame` ✅
  - `ui::art::tests::test_print_banner_executes` ✅
  - `ui::art::tests::test_play_bouncing_owl_non_interactive` ✅
  - `ui::art::tests::test_print_mascot_sizes` ✅
  - `ui::art::tests::test_print_modes_showcase_executes` ✅
  - `ui::assets::tests::test_print_task_lifecycle_cards` ✅
  - `ui::assets::tests::test_asset_loading_all` ✅
  - `ui::assets::tests::test_render_lines_dimensions` ✅
- **Kiểm thử Đóng gói NPM Cục bộ:** Chạy thành công `npm pack` sinh file `custos-cli-0.1.0-alpha.tgz` (5.4 MB), cài đặt toàn cục thành công qua `npm install -g ./custos-cli-0.1.0-alpha.tgz`.
- **Kiểm thử Giao diện Thông báo Update:** Chạy giả lập với `$env:CUSTOS_SIMULATE_UPDATE="0.2.0"`, khung thông báo màu vàng hiện lên chuẩn xác, trang nhã ngay sau banner mascot.
- **Bảo vệ Luồng Dữ liệu Máy Đọc:** Khi chạy kèm cờ `--json`, khung thông báo được triệt tiêu hoàn toàn, bảo toàn 100% định dạng JSON đầu ra.

---

## 5. Kế hoạch & Mục tiêu Tiếp theo

1. **Kết nối CLI Client với Daemon Supervisor (`custosd`):**
   - Triển khai kênh giao tiếp IPC qua Local API endpoints.
   - Hỗ trợ chuyển tiếp trạng thái tiến trình và stream log theo thời gian thực về CLI.
2. **Nâng cấp Cơ chế Cấp phép Tương tác (Interactive Execution Permits):**
   - Kết nối hộp thoại `confirm_execution` trực tiếp với `AuthorityEngine` khi xuất hiện payload cần quyền ghi ổ đĩa hoặc chạy lệnh shell.
3. **Phối hợp Tích hợp cùng Vi & Vinh:**
   - Phối hợp với Vi chuẩn hóa schema `ContextRecipe` và `EvidenceCitation` hiển thị trên CLI diff view.
   - Hỗ trợ Vinh hoàn thiện bộ adapter và sandbox worktree execution trên Linux/macOS.
