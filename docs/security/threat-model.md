# Mô Hình Đe Dọa & Phòng Vệ (Threat Model & Security Defenses)

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần V (§25) Canonical Specification  
> **Framework:** STRIDE Model & Principle of Least Privilege

Tài liệu này xác định các mối đe dọa bảo mật đối với một runtime AI chạy cục bộ trên máy trạm cá nhân và cách thức Custos phòng vệ chủ động.

---

## 1. Phân Tích Đe Dọa Theo Mô Hình STRIDE

| Danh mục STRIDE | Mối đe dọa thực tế đối với Agentic Runtime | Biện pháp phòng vệ của Custos |
|---|---|---|
| **Spoofing (Giả mạo)** | Lệnh từ mã độc giả mạo là chỉ thị từ người dùng | Định danh phiên làm việc qua Unix Socket có chứng thực UID cục bộ; ký số Ed25519 cho mọi Event và Permit. |
| **Tampering (Can thiệp trái phép)** | Worker tự ý sửa đổi file mã nguồn ngoài scope hoặc sửa lịch sử event | Cô lập Git Worktree; SQLite WAL chỉ cho phép ghi tuần tự; Event Store có chuỗi băm bất biến (*Hash-chained sequence*). |
| **Repudiation (Chối bỏ trách nhiệm)** | Không thể xác định model hay công cụ nào đã gây ra lỗi hệ thống | Sổ cái `Decision Ledger` ghi nhận mọi quyết định; biên nhận `Receipt` ghi lại chính xác exit code, diff và thời gian. |
| **Information Disclosure (Rò rỉ thông tin)** | Prompt injection trích xuất tệp nhạy cảm (`~/.ssh/id_rsa`, `.env`) gửi ra ngoài | Sandbox cấm đọc thư mục cá nhân; Màng bảo vệ riêng tư (*Privacy Membrane*) quét regex và chặn egress các secrets. |
| **Denial of Service (Từ chối dịch vụ)** | Vòng lặp suy luận vô tận làm cạn kiệt ngân sách hoặc treo CPU máy trạm | Ràng buộc ngân sách trần (*Budget Invariant*); giới hạn thời gian chạy cho từng step; hủy worker khi timeout. |
| **Elevation of Privilege (Leo thang đặc quyền)**| Model dùng prompt injection lừa hệ thống tự cấp quyền quản trị | **Confidence không bao giờ tạo Capability**; chỉ có con người mới có quyền phê duyệt mở rộng scope ngoài hợp đồng. |

---

## 2. Phòng Chống Tấn Công Tiêm Mã Nhắc Lệnh (Prompt Injection Defense)

Mọi dữ liệu ngoại cảnh (nội dung issue GitHub, tệp mã nguồn lạ, tệp PDF, kết quả tìm kiếm web, đầu ra của lệnh shell) đều được Custos coi là **Dữ Liệu Độc Hại Tiềm Tàng (Untrusted Data)**.

1. **Phân tách rạch ròi Data và Instruction:** Nội dung từ các tệp được đóng gói trong các thẻ dữ liệu phân định rõ ràng (ví dụ: `<untrusted_content>`), ngăn model nhầm lẫn giữa dữ liệu cần xử lý và câu lệnh điều khiển.
2. **Không thực thi trực tiếp từ output:** Kết quả trả về của model không bao giờ được chuyển thẳng tới `eval()` hay `sh -c`. Nó bắt buộc phải chuyển thành một `ActionProposal` có cấu trúc và đi qua Capability Gateway.
