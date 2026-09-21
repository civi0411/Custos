# Thuật Ngữ Cốt Lõi (Core Concepts Glossary)

> **Status:** Canonical Baseline v4.0  
> **Scope:** Toàn bộ hệ thống Custos

Tài liệu này định nghĩa chính xác và nhất quán các khái niệm nền tảng được sử dụng trong mã nguồn, tài liệu và giao thức của Custos.

---

## 1. Các Thực Thể Trung Tâm (Core Entities)

### Task
Đơn vị công việc trung tâm của Custos. Task mang trạng thái bền vững, được định nghĩa bằng một bản hợp đồng (*Task Contract*), thực thi qua nhiều bước bởi các worker và kết thúc bằng các artifacts kèm bằng chứng kiểm định.

### Task Contract
Thỏa thuận ràng buộc giữa người dùng và runtime về mục tiêu công việc, bao gồm:
- **Intent:** Mục đích cốt lõi cần đạt được.
- **Scope:** Phạm vi tệp tin hoặc tài nguyên được phép tác động.
- **Constraints:** Các giới hạn (ngân sách token, thời gian tối đa, mức độ tự chủ).
- **Verification Criteria:** Tiêu chí nghiệm thu khách quan cần thỏa mãn.

### Run
Một phiên thực thi cụ thể của một Task. Một Task có thể trải qua nhiều Run (ví dụ: chạy lần đầu thất bại, người dùng điều chỉnh yêu cầu và chạy Run thứ 2).

### Step
Một bước thực thi đơn lẻ trong một Run, gắn liền với một hành động suy luận hoặc công cụ cụ thể, kèm theo trạng thái trước/sau và biên nhận kiểm tra.

### Artifact
Sản phẩm đầu ra được tạo ra hoặc biến đổi trong quá trình thực thi Task (mã nguồn, tệp tài liệu, bản phân tích, báo cáo kiểm thử). Artifacts lớn được lưu trữ trong hệ thống lưu trữ địa chỉ theo nội dung (CAS).

---

## 2. Hạ Tầng Nhận Thức & Phán Đoán (Cognitive & Judgment)

### System One (Judgment Fabric)
Hạ tầng phán đoán nhanh, cục bộ, chi phí thấp (đáp ứng trong mili-giây). Đóng vai trò phản xạ kiểm tra: phân loại ý định, phát hiện thiếu thông tin, đánh giá rủi ro, kiểm tra bất biến và quyết định khi nào cần leo thang lên con người.

### System Two (Deliberation Fabric)
Tầng suy luận sâu, chi phí cao, thực hiện bởi các mô hình ngôn ngữ lớn (OpenAI Codex, Claude, DeepSeek, v.v.). Chịu trách nhiệm lập kế hoạch phức tạp, viết mã nguồn và tổng hợp tri thức.

### RDC Protocol (Request-Decision-Challenge)
Giao thức trao đổi chuẩn giữa các tầng nhận thức:
- **Request:** Yêu cầu phán đoán hoặc hành động kèm ngữ cảnh.
- **Decision:** Quyết định được đưa ra kèm độ tin cậy và lý do.
- **Challenge:** Cơ chế phản biện ngược lại quyết định khi phát hiện mâu thuẫn hoặc rủi ro tiềm ẩn (*Reflexive Challenge*).

### Question Registry
Kho câu hỏi chuẩn hóa có phiên bản, dùng để truy vấn System One một cách nhất quán nhằm đưa ra các quyết định có cấu trúc thay vì viết prompt tự do.

---

## 3. Bảo Mật & Thực Thi (Execution & Security)

### Capability Gateway
Cổng kiểm soát duy nhất cho mọi hành động tạo ra tác động ngoại cảnh (*side effect*). Không có bất kỳ worker nào được phép gọi trực tiếp lệnh hệ thống hay API ngoài mà không qua Gateway này.

### ExecutionPermit
Giấy phép thực thi được ký số bởi Kernel, xác nhận rằng một hành động cụ thể đã được kiểm tra quyền hạn, thỏa mãn chính sách bảo mật và (nếu cần) đã được con người phê duyệt cụ thể (*Exact-Payload Approval*). Có thời hạn hiệu lực (*TTL*) và phạm vi nghiêm ngặt.

### Exact-Payload Approval
Nguyên tắc phê duyệt minh bạch: con người không bao giờ ký một "tấm séc trắng" (ví dụ: "cho phép chạy lệnh bash bất kỳ"). Người dùng luôn nhìn thấy chính xác nội dung lệnh, diff tệp tin hoặc API payload trước khi bấm duyệt.

### Worktree Isolation
Cơ chế cô lập môi trường làm việc bằng cách tạo ra một `git worktree` riêng biệt cho mỗi Task/Run. Worker chỉ được thao tác trong worktree này; nhánh chính (`main`) chỉ được cập nhật sau khi toàn bộ bước kiểm định hoàn tất.

---

## 4. Chứng Cứ & Nghiệm Thu (Evidence & Verification)

### Evidence-Carrying Action (ECA)
Hành động mang theo chứng cứ: mọi hành động làm biến đổi hệ thống đều bắt buộc phải tạo ra bằng chứng chứng minh tính đúng đắn và lý do nó được phép diễn ra.

### Verifiable Outcome Bundle
Gói bàn giao kết quả cuối cùng của Task, bao gồm:
- Toàn bộ artifacts đầu ra.
- Nhật ký thực thi đầy đủ (*Execution Trace*).
- Tập hợp chứng cứ nghiệm thu (test passes, linter receipts, build logs).
- Chữ ký xác thực và hash toàn vẹn.

### Completion Gate
Cổng kiểm tra tự động trước khi đóng Task: đối chiếu kết quả thực tế với tiêu chí nghiệm thu trong Task Contract. Nếu thiếu chứng cứ, Task không thể chuyển sang trạng thái `Completed`.

---

## 5. Dữ Liệu & Giao Tiếp (Data & Communication)

### ContextPack
Gói ngữ cảnh được biên soạn và tối ưu hóa trước khi gửi cho model, được chọn lọc theo công thức tính điểm liên quan và gắn nhãn nguồn gốc (*provenance*) cho từng đoạn dữ liệu.

### ContinuationPacket
Gói dữ liệu độc lập với provider, đóng gói toàn bộ trạng thái tiến trình, biến môi trường và hợp đồng công việc để có thể chuyển giao mượt mà sang một AI provider khác (*provider switching*) hoặc tiếp tục sau sự cố.

### Star Topology
Kiến trúc liên lạc dạng sao: mọi trao đổi giữa các worker đều phải đi qua trung tâm điều phối của Kernel; không cho phép các worker chat trực tiếp tự do với nhau.

### Human Attention Budget
Định mức chú ý của con người: một ngân sách đo lường số lần làm phiền người dùng. Hệ thống tối ưu hóa để giảm thiểu số lần ngắt quãng con người, chỉ yêu cầu can thiệp ở các quyết định rủi ro cao.
