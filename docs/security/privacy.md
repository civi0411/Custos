# Quyền Riêng Tư & Kiểm Soát Dữ Liệu (Privacy & Data Retention)

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần V (§25.5-25.6) Canonical Specification

Custos được tạo ra nhằm bảo vệ tài sản trí tuệ và sự riêng tư tuyệt đối của người dùng trong kỷ nguyên các nhà cung cấp đám mây liên tục thu thập dữ liệu để huấn luyện mô hình.

---

## 1. Chính Sách Không Thoát Dữ Liệu Mặc Định (Zero Egress by Default)

1. **Không gửi Telemetry về máy chủ Custos:** Custos là phần mềm nguồn mở chạy cục bộ; runtime không tự động gửi bất kỳ gói tin thống kê, theo dõi hành vi người dùng hay báo cáo crash nào về máy chủ từ xa.
2. **Kiểm Soát Xuất Dữ Liệu Ra AI Providers (Egress Gate):**
   - Trước khi bất kỳ `ContextPack` nào được truyền qua Internet đến OpenAI hay Anthropic, nó phải đi qua bộ lọc **Data Sanitizer**.
   - Tự động phát hiện và che giấu (*redact*) các mẫu thông tin nhạy cảm: địa chỉ email, số điện thoại, token bí mật (`sk-ant-...`, `ghp_...`, AWS Access Keys).

---

## 2. Chính Sách Lưu Giữ & Dọn Dẹp Dữ Liệu Cục Bộ (Data Retention Policy)

Người dùng có toàn quyền kiểm soát dữ liệu nằm trong thư mục `.custos/`:
- **Lệnh hủy sạch dấu vết:** `custos purge --task <task_id>` sẽ xóa vĩnh viễn toàn bộ sự kiện, logs và artifacts liên quan đến task đó khỏi máy tính.
- **Tự động dọn dẹp CAS:** Các artifacts tạm thời (build logs cũ, diff trung gian không được merge) được tự động dọn dẹp định kỳ sau 30 ngày để tiết kiệm dung lượng ổ đĩa.
