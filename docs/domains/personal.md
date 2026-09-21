# Gói Tác Vụ Cá Nhân (Personal Operations Domain Pack)

> **Status:** Canonical Baseline v4.0 (Horizon 2)  
> **Source:** Phần VI (§20) & Phần VII (§55) Canonical Specification

Personal Operations Domain Pack phục vụ các tác vụ điều phối công việc hàng ngày của người dùng (quản lý email, lịch làm việc, ghi chú, danh sách việc cần làm) với cam kết bảo mật tuyệt đối.

---

## 1. Thang Đo Tự Chủ (The Autonomy Ladder)

Để ngăn chặn việc AI tự ý gửi email sai hoặc xóa nhầm sự kiện lịch, Custos thiết lập Thang Đo Tự Chủ 5 cấp độ:

| Cấp độ | Năng lực cho phép | Trạng thái mặc định | Cơ chế an toàn |
|---|---|---|---|
| **A0** | Tìm kiếm và đọc thông tin | Bật sẵn trong scope | Chỉ đọc (Read-only) |
| **A1** | Soạn thảo bản nháp (Draft) | Bật sẵn | Luôn hiển thị bản xem trước (*Preview*) |
| **A2** | Ghi cục bộ có thể hoàn tác | Cần người dùng bật (Opt-in) | Hỗ trợ nút Hoàn tác (*Undo*) |
| **A3** | Tác động ngoại cảnh (Gửi mail, tạo lịch) | **Bắt buộc phê duyệt** | Exact-Payload Approval |
| **A4** | Hành động phá hủy / Nhạy cảm cao | Chỉ làm thủ công | AI chỉ gợi ý, con người tự bấm |

> [!IMPORTANT]
> Trong các phiên bản MVP và Alpha, runtime **chỉ vận hành ở mức A0 và A1**. Tuyệt đối không tự động phát tán email ra ngoài nếu chưa có người dùng xác nhận trực tiếp.

---

## 2. Ngân Sách Chú Ý (Human Attention Budget)

Một trợ lý thông minh không phải là trợ lý gửi hàng trăm thông báo mỗi ngày. Custos áp dụng khái niệm **Human Attention Budget**:
- Gom nhóm các việc cần duyệt vào các khung giờ cố định (Daily Review).
- Phân loại thông báo: việc khẩn cấp có rủi ro cao mới phát chuông; việc bình thường được đưa vào hàng đợi `Approval Inbox`.
- Giữ cho người dùng ở trạng thái tập trung sâu (*Deep Work*).
