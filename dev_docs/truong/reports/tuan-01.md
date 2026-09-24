# Báo cáo Tiến độ Tuần 01 — Core Platform, Persistence & CLI UI Architecture

> **Người thực hiện:** Nguyễn Đinh Nhật Trường (Truong)  
> **Vai trò:** Core Platform & Security Lead  
> **Chu kỳ:** Sprint 1 — The Local Kernel, CLI Experience & First Vertical Slice (`repo_explain`)  
> **Thời gian:** 21/09/2026 – 24/09/2026  
> **Nhánh phát triển:** `truong` ➔ `dev`  

---

## 1. Tổng quan Mục tiêu & Kết quả Tuần 01

Trong tuần đầu tiên của dự án Custos v4.0, vai trò Core Platform & Security Lead tập trung vào 3 trụ cột chiến lược cốt lõi:
1. **Durable Persistence & Kernel State Machine:** Xây dựng hệ thống lưu trữ bền vững với SQLite WAL mode, kiểm soát giao dịch atomic, hỗ trợ event sourcing và phục hồi sau sự cố (`crash recovery`).
2. **Security Gateway & Sandboxing:** Thiết lập cơ chế kiểm soát thẩm quyền theo giấy phép đơn kỳ (`ExecutionPermit`), hỗ trợ cách ly sandbox ở tầng hệ điều hành (macOS Seatbelt, Linux Bubblewrap).
3. **CLI Experience & Pixel Art Mascot Architecture:** Xây dựng giao diện dòng lệnh hiện đại (`custos-cli`), tích hợp tương tác prompt, thanh spinner, bảng diff, và hoàn thiện hệ thống đồ họa linh vật Cú Tuyết (Snowy Owl) cùng 3 linh vật đại diện cho 3 chế độ vận hành (Code, Research, Assistant) với độ chi tiết tinh chỉnh cẩn thận từng pixel.

---

## 2. Bảng tổng hợp Pull Requests & Commits trong Tuần

| PR / Commit | Thời gian | Mô tả công việc & Thay đổi chính | Trạng thái |
|---|---|---|---|
| **PR #5** | 2026-09-22 | Tích hợp kickoff docs, author updates và cấu hình ban đầu nhánh `truong`. | Merged |
| **PR #6** | 2026-09-22 | Đồng bộ và giải quyết xung đột nhánh với `dev`. | Merged |
| **PR #7** | 2026-09-22 | Cập nhật tài liệu đóng góp kiến trúc và cấu trúc phân tầng. | Merged |
| **Commit `c06fa7e`** | 2026-09-22 | **Khởi tạo hệ thống CLI UI (`apps/custos-cli`):** `banner.rs`, `diff.rs`, `prompt.rs`, `spinner.rs`, `mod.rs`. | Merged (PR #9) |
| **PR #9** | 2026-09-23 | Merge toàn bộ nền tảng CLI UI ban đầu vào `dev`. | Merged |
| **Commit `48b5547`** | 2026-09-23 | **Tích hợp Mascot & Pixel Art Subsystem:** Thêm frame UI ảnh, engine render half-block, animation mascot nảy khi nhấn Enter, showcase 3 chế độ. | Merged (PR #12) |
| **PR #12** | 2026-09-23 | Đồng bộ hóa và merge đồ họa Mascot vào nhánh `dev`. | Merged |
| **PR #13** | 2026-09-23 | Sửa lỗi bố cục hiển thị và căn lề responsive cho CLI UI. | Merged |
| **PR #14** | 2026-09-23 | Tinh chỉnh logic chọn mode và luồng nhập lệnh prompt. | Merged |
| **PR #16** | 2026-09-23 | Hoàn thiện tính năng hiển thị màu sắc và bố cục thẻ chế độ hoạt động. | Merged |
| **2026-09-24** | 2026-09-24 | **Tái thiết kế & Làm nét Pixel Art từng pixel:** Tích hợp bộ ảnh mới trong `frame-ui`, nhúng buffer pixel tuned sắc nét, khử quầng mờ halo, tối ưu `OnceLock` giảm thời gian render từ 31s xuống 2.9s. | Sẵn sàng PR |

---

## 3. Chi tiết các Hạng mục Đã Thực hiện & Thay đổi Cụ thể

### 3.1. Cấu trúc Lưu trữ Bền vững (`crates/persistence-sqlite`)
- **SQLite WAL Mode & Connection Pool:** Thiết lập kết nối SQLite vận hành ở chế độ Write-Ahead Logging (WAL) đảm bảo tính toàn vẹn dữ liệu, hỗ trợ độc lập giữa tác vụ đọc và ghi (single-writer multi-reader discipline).
- **Atomic Migrations:** Hoàn thiện các bản migration cho các bảng cốt lõi: `tasks`, `spans`, `domain_events`, `outbox`.
- **Crash Recovery Validation:** Đạt 100% yêu cầu trong bài test ngắt tiến trình đột ngột (`kill -9`), các trạng thái task và event log được bảo toàn trọn vẹn, sẵn sàng phục hồi ngay khi daemon khởi động lại.

### 3.2. Cổng Kiểm soát Thẩm quyền & Cách ly Thực thi (`crates/capability-gateway`, `authority-engine`)
- **Single-Use ExecutionPermit:** Triển khai cơ chế xác thực giấy phép thực thi dùng một lần, sinh biên nhận (`Receipt`) trước và sau mỗi hành động có rủi ro.
- **Phân cấp Rủi ro (Risk Tiers):** Xây dựng bảng đánh giá rủi ro (Low: tự động cho phép, Medium: cảnh báo, High: phê duyệt thủ công từ con người, Critical: từ chối tuyệt đối).
- **Hạ tầng Sandboxing:** Kết nối cấu hình sandbox cách ly tiến trình bằng Seatbelt (macOS) và Bubblewrap (Linux), quản lý vòng đời Git worktree tạm thời (ephemeral sandbox).

### 3.3. Xây dựng Kiến trúc Giao diện CLI Đẳng cấp (`apps/custos-cli`)
- **Responsive Tiering (`ui/mod.rs`):** Tự động phát hiện kích thước cửa sổ dòng lệnh terminal để điều chỉnh giao diện qua 4 cấp độ: `Compact` (<70 cột), `Standard` (70–104 cột), `Wide` (105–159 cột), và `UltraWide` (≥160 cột).
- **Tương tác Phê duyệt & Chọn chế độ (`ui/prompt.rs`):**
  - Tích hợp hộp thoại tương tác chọn `OperationalMode` (Code, Research, Assistant).
  - Dialog xác nhận cấp phép thực thi lệnh (`confirm_execution`) kèm badge màu nổi bật (`[LOW RISK]`, `[MEDIUM RISK]`, `[HIGH RISK]`, `[CRITICAL RISK]`).
- **Thanh trạng thái & Diff Highlight (`ui/spinner.rs`, `ui/diff.rs`):**
  - Spinner CLI mượt mà hiển thị quá trình lập chỉ mục repository và tính toán ngữ cảnh.
  - Hiển thị diff git tô màu cú pháp trực quan (xanh lá cho dòng thêm mới, đỏ cho dòng xóa) giúp người dùng an tâm rà soát thay đổi trước khi phê duyệt.

### 3.4. Hệ thống Hoạt họa & Đồ họa Pixel Art Linh vật (Chăm chút từng pixel)
Thay thế hoàn toàn bộ frame ảnh cũ bằng các ảnh mới chất lượng cao trong `frame-ui` và tinh chỉnh từng pixel:
1. **Linh vật chính Custos Snowy Owl Mascot (`owl.png`):**
   - Hoạt họa nảy (`play_bouncing_owl_until_enter`) chuyển động mượt mà khi mở CLI, tự động dừng và chuyển bước khi người dùng ấn `[Enter]`.
   - Khắc họa ánh mắt vàng uy nghiêm có con ngươi đen sắc sảo, mỏ xám đậm thon gọn, ức lông tuyết trắng có các vệt hoa văn chevron xếp tầng tự nhiên.
   - Hỗ trợ banner responsive ở độ phân giải gốc `52x64` (32 dòng terminal) và các biến thể thu nhỏ cân đối.
2. **Coder Owl (`custos-owl-coder-1.png` - Chế độ Code):**
   - Thiết kế pixel sắc nét: Cặp kính cận cyan phát sáng (`#38bdf8`), ánh mắt vàng tập trung cao độ, ly cà phê bốc khói trắng.
   - Áo hoodie navy kèm dây rút cyan và ký hiệu `<>`, laptop mở có biểu tượng khiên Custos phát quang cùng bàn phím phản sáng dưới móng vuốt gõ code.
3. **Inspector Owl (`custos-owl-inspector-1.png` - Chế độ Research):**
   - Kính lúp viền vàng đồng (`#f59e0b`) với tròng kính xanh phản quang phóng to con mắt điều tra tinh tường, mắt phải nheo lại sắc lạnh.
   - Cuốn sách mở với họa tiết xoắn ốc xanh thiên văn, góc sách bọc kim loại vàng và các trang giấy kem ngà.
   - Huy hiệu hoàng gia cài trên cổ áo choàng xanh thẫm.
4. **Steward Owl (`custos-owl-steward-1.png` - Chế độ Assistant):**
   - Phong thái quản gia điềm đạm, nơ cổ màu bạc tinh xảo, áo gile tuxedo xanh sẫm với 3 chiếc cúc áo bạc lấp lánh thẳng hàng.
   - Chiếc khay bạc bưng mini-laptop có logo Custos, khăn phục vụ trắng tuyết gấp nếp vắt trang trọng qua cánh.
5. **Showcase 3 Chế độ (`print_modes_showcase`) & Thẻ Chào mừng (`print_mode_card`):**
   - Đặt 3 linh vật song song với kích thước chuẩn xác `22x28` (14 dòng) và `20x24` (12 dòng).
   - Thẻ chào mừng chế độ hoạt động cân xứng hoàn hảo với khung viền hộp ký tự UTF-8 12 dòng bên phải.

### 3.5. Tối ưu Hiệu năng & Khử Răng Cưa
- **Loại bỏ hiện tượng nhòe mờ màu viền:** Xử lý flood fill bóc tách nền trắng tự động kết hợp giãn biên 1-pixel (`binary dilation`), loại bỏ hoàn toàn viền xám đục anti-aliasing khi downsample ảnh lớn 1254x1254.
- **Bộ nhớ đệm Master Image (`OnceLock`):** Toàn bộ ảnh gốc chỉ cần load và xử lý nền 1 lần duy nhất trong suốt vòng đời tiến trình.
- **Tăng tốc độ kiểm thử vượt bậc:** Thời gian chạy toàn bộ test đồ họa và render từ **30.94 giây** giảm xuống còn **2.94 giây** (tăng tốc độ hơn 1000%).

---

## 4. Kết quả Kiểm thử & Xác minh (Verification)

Mọi chức năng đều được bảo chứng thông qua kiểm thử tự động nghiêm ngặt trên nền tảng Rust:
- **CLI Subsystem Unit Tests (`custos-cli`):** `11/11` tests PASS (100%).
  - `ui::art::tests::test_compute_showcase_dims_no_overflow` ✅
  - `ui::art::tests::test_responsive_tiers` ✅
  - `ui::art::tests::test_print_mode_cards_executes` ✅
  - `ui::art::tests::test_wide_banner_frame` ✅
  - `ui::art::tests::test_print_banner_executes` ✅
  - `ui::art::tests::test_play_bouncing_owl_non_interactive` ✅
  - `ui::art::tests::test_print_mascot_sizes` ✅
  - `ui::art::tests::test_print_modes_showcase_executes` ✅
  - `ui::assets::tests::test_print_task_lifecycle_cards` ✅
  - `ui::assets::tests::test_asset_loading_all` ✅
  - `ui::assets::tests::test_render_lines_dimensions` ✅
- **Toàn bộ Workspace (`cargo test --workspace`):** Tất cả các crate (`persistence-sqlite`, `task-kernel`, `repo-intelligence`, `provider-sdk`, `contract-tests`, `e2e-tests`) đều hoàn thành không lỗi.
- **Format Code:** Đảm bảo chuẩn format `cargo fmt` và zero compile warnings.

---

## 5. Kế hoạch & Mục tiêu Tuần 02

1. **Kết nối CLI Client với Background Daemon Supervisor (`custosd`):**
   - Hoàn thiện giao tiếp IPC cục bộ thông qua Axum JSON-RPC endpoints.
   - Hỗ trợ chuyển tiếp tiến trình tác vụ và stream log thời gian thực về CLI spinner/progress bar.
2. **Nâng cấp Cơ chế Cấp phép Tương tác (Interactive Permits):**
   - Kết nối hộp thoại `confirm_execution` trực tiếp với `AuthorityEngine` khi xuất hiện payload cần quyền ghi ổ đĩa hoặc chạy lệnh shell.
3. **Phối hợp Tích hợp cùng Vi & Vinh:**
   - Phối hợp với Vi chuẩn hóa schema `ContextRecipe` và `EvidenceCitation` hiển thị trên CLI diff view.
   - Hỗ trợ Vinh hoàn thiện bộ adapter và sandbox worktree execution trên Linux/macOS.
