# Quy Chuẩn Kỹ Thuật Mã Nguồn & An Toàn Dành Cho AI Coding Agents
## (Coding Standards & Safety Protocol)

> **Phạm vi áp dụng:** Toàn bộ AI Coding Agents (Cursor, Claude Code, Antigravity, Copilot, Windsurf) khi viết code trong repository Custos.

---

### 1. Chuẩn Mực Rust Code (Rust Standards)

1. **Tuyệt đối không `.unwrap()` hoặc `.expect()` trong production code:**
   - Mọi thao tác có khả năng thất bại phải trả về `Result<T, E>`.
   - `E` phải là kiểu enum lỗi có định kiểu rõ ràng, được định nghĩa tại cấp crate bằng `thiserror`.
2. **Tuyệt đối không silent panic:**
   - Cấm sử dụng `panic!()`, `unreachable!()`, hoặc `unimplemented!()` trong các luồng thực thi production.
   - Chỉ sử dụng `todo!()` khi được con người yêu cầu giữ placeholder và đánh dấu chưa hoàn thiện.
3. **Bắt buộc Tracing:**
   - Mọi hoạt động trọng yếu phải phát ra span hoặc log bằng `tracing::info!` hoặc `tracing::debug!`.
   - Bắt buộc đính kèm các khóa ngữ cảnh (`task_id`, `run_id`, `session_id`).
4. **Nhận thức Hủy bỏ (Cancellation-Aware):**
   - Mọi tác vụ async chạy dài phải theo dõi `CancellationToken` thông qua `tokio::select!`.
5. **Đột biến trạng thái phải qua Event / Outbox:**
   - Mọi thao tác ghi dữ liệu vào SQLite store phải thông qua Transactional Outbox pattern hoặc domain event. Không được ghi ngầm bỏ qua kiểm toán.

---

### 2. Ranh Giới 11 Canonical Crates Bất Biến

1. **`crates/custos-domain`:**
   - Bất biến Zero-I/O tuyệt đối.
   - Không phụ thuộc `tokio`, `rusqlite`, `reqwest`, `std::fs`, hoặc bất kỳ external provider SDK nào.
2. **`crates/custos-core`:**
   - Hạt nhân điều phối và máy trạng thái.
   - Phụ thuộc duy nhất vào `custos-domain`. Tuyệt đối không import các crate adapter hoặc SDK bên ngoài.
3. **`crates/custos-persistence`:**
   - Cơ sở dữ liệu SQLite chế độ WAL.
   - Bắt buộc duy trì kết nối Single-Writer duy nhất để triệt tiêu lỗi tranh chấp ghi và WAL starvation.
4. **`crates/custos-bridge`:**
   - Cầu nối phiên hội thoại và Task Kernel.
   - CẤM TUYỆT ĐỐI việc import trực tiếp `custos-persistence`. Mọi tương tác lưu trữ phải đi qua `custos-core::ports::KernelPort`.
5. **`crates/custos-daemon`:**
   - Điểm ráp nối duy nhất (Sole Composition Root).
   - Nơi duy nhất được phép import các hiện thực cụ thể (concrete adapters, storage) để khởi tạo daemon.

---

### 3. Quy Chuẩn Định Dạng & Tài Liệu (Zero-Emoji Policy)

1. **Tuyệt đối không dùng Emoji trang trí:**
   - Không sử dụng emoji (như biểu tượng tên lửa, ngọn lửa, ngôi sao, icon mặt cười...) trong bất kỳ tài liệu markdown, tiêu đề, ô bảng, ghi chú code (comments), hoặc commit message nào.
2. **Không gán số phiên bản hoặc ngày tháng vào tiêu đề:**
   - Không đặt tiêu đề tài liệu dạng "v2", "v3.1", hoặc ngày tháng cụ thể. Tài liệu phải mang tính thường xanh (evergreen).
3. **Commit Messages (Conventional Commits):**
   - Định dạng: `<type>(<scope>): <mô tả ngắn bằng tiếng Anh kỹ thuật, tối đa 72 ký tự>`.
   - Không chứa emoji trong commit message.

---

### 4. Giao Thức An Toàn Git (Git Safety Protocol)

1. Không chạy `git reset --hard` khi chưa có xác nhận rõ ràng 2 bước từ con người.
2. Không chạy `git push --force` dưới bất kỳ hình thức nào.
3. Trước khi chỉnh sửa, luôn kiểm tra `git status --short`. Bảo tồn các thay đổi dở dang của người dùng.
4. Không tự ý thực hiện commit hoặc push khi chưa được yêu cầu rõ ràng.
