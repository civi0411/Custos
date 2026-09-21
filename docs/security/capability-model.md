# Mô Hình Thẩm Quyền & Phê Duyệt (Capability Model & Approvals)

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần V (§25.3-25.4) Canonical Specification

Custos xây dựng kiến trúc an ninh dựa trên nền tảng **Bảo Mật Hướng Thẩm Quyền (Capability-Based Security)**: một thành phần không thể hành động nếu không nắm giữ một tấm vé ủy quyền hợp lệ (*Capability Token*).

---

## 1. Nguyên Tắc Phê Duyệt Nội Dung Chính Xác (Exact-Payload Approval)

Một trong những lỗ hổng nguy hiểm nhất của các công cụ AI hiện nay là yêu cầu người dùng phê duyệt chung chung (ví dụ: *"Bạn có cho phép Agent chạy lệnh shell không?"*).

Custos áp dụng nguyên tắc **Exact-Payload Approval**:
- Người dùng **chỉ phê duyệt một lệnh duy nhất với nội dung hash chính xác**:
  $$	ext{PayloadHash} = 	ext{SHA-256}(	ext{command} + 	ext{args} + 	ext{target\_path})$$
- Nếu model tự ý thay đổi dù chỉ một ký tự hoặc tham số cờ lệnh, mã băm sẽ thay đổi, và `ExecutionPermit` đã cấp sẽ bị vô hiệu hóa ngay lập tức.
- Tuyệt đối không hỗ trợ tùy chọn *"Cho phép tất cả các lệnh từ giờ trở đi"* đối với các hành động có rủi ro cao.

---

## 2. Quản Lý Bí Mật Tuyệt Đối (Zero Secrets In Database/Logs)

- **Không lưu plain-text keys:** API keys của các nhà cung cấp AI (OpenAI, Anthropic) và các dịch vụ bên ngoài không bao giờ được lưu vào cơ sở dữ liệu SQLite hay tệp cấu hình phẳng.
- **Tận dụng OS Keychain:** Custos sử dụng trực tiếp dịch vụ lưu trữ an toàn của hệ điều hành:
  - macOS Keychain Services qua thư viện Security Framework.
  - Linux Secret Service API qua D-Bus / SecretStorage.
- **Xóa sạch trong bộ nhớ (Memory Scrubbing):** Các biến chứa khóa bí mật được gói trong struct tự động ghi đè bộ nhớ về số 0 khi biến ra khỏi phạm vi sử dụng (`zeroize` crate trong Rust).
