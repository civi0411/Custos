# Bản Đồ Năng Lực & Phân Hạng Tính Năng (Capability Map & Taxonomy)

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần I (§3) & Phần VI (§31) Canonical Specification

---

## 1. Bản Đồ Năng Lực 6 Miền (Capability Map)

Kiến trúc của Custos bao quát 6 miền năng lực cốt lõi:

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                             CUSTOS CAPABILITY MAP                           │
├──────────────────────┬──────────────────────┬───────────────────────────────┤
│ 1. TASK GOVERNANCE   │ 2. COGNITIVE CONTROL │ 3. EXECUTION & SAFETY         │
│ - Contract definition│ - Pluggable System 1 │ - Capability Gateway          │
│ - State transitions  │ - System 2 adapters  │ - Tiered sandboxing           │
│ - Budget enforcement │ - RDC protocol       │ - Exact-payload approvals     │
│ - Resumable runs     │ - Question Registry  │ - Git worktree isolation      │
├──────────────────────┼──────────────────────┼───────────────────────────────┤
│ 4. CONTEXT & MEMORY  │ 5. INTEROPERABILITY  │ 6. EVIDENCE & AUDIT           │
│ - ContextPack scoring│ - ProviderPort       │ - 6 Evidence classes          │
│ - Provenance tracking│ - ContinuationPacket │ - Verifiable Outcome Bundle   │
│ - 5 Memory layers    │ - Safe model switch  │ - Decision Ledger             │
│ - Relational graph   │ - Official MCP client│ - Tamper-evident receipts     │
└──────────────────────┴──────────────────────┴───────────────────────────────┘
```

### 1.1 Quản Trị Task (Task Governance)
- Định nghĩa và xác thực Task Contract trước khi khởi động.
- Máy trạng thái bền vững (*State Machine*) kiểm soát chu kỳ sống: Draft → Ready → Running → Suspended → Completed / Failed.
- Ràng buộc ngân sách: kiểm soát trần chi phí API và định mức số bước thực thi.
- Hồi phục và tiếp tục tiến trình sau mọi sự cố dừng đột ngột.

### 1.2 Điều Khiển Nhận Thức (Cognitive Control)
- Phân tầng nhận thức rõ rệt: System One (phán đoán nhanh) và System Two (suy luận sâu).
- Giao thức chuẩn hóa RDC (Request-Decision-Challenge).
- Quản lý kho câu hỏi định chuẩn (*Versioned Question Registry*).
- Phản xạ tự vấn (*Reflexive Challenge*) phát hiện giả định sai trước khi thực hiện hành động tốn kém.

### 1.3 Thực Thi & An Toàn (Execution & Safety)
- Cổng Capability Gateway duy nhất làm trung gian cho mọi tương tác hệ điều hành và mạng.
- Cấp quyền hạt mịn qua `ExecutionPermit` kèm mã băm chữ ký và thời hạn hiệu lực.
- Cô lập hoàn toàn thay đổi mã nguồn trong Git worktree riêng biệt.
- Môi trường thực thi lệnh được đóng gói bằng sandbox cục bộ (macOS Seatbelt, Linux bubblewrap).

### 1.4 Ngữ Cảnh & Bộ Nhớ (Context & Memory)
- Thu thập và chấm điểm liên quan cho ngữ cảnh (*ContextPack Scoring*).
- Gắn thẻ nguồn gốc (*provenance*) và mức độ nhạy cảm (*sensitivity tier*) cho từng mẩu dữ liệu.
- Quản lý 5 tầng bộ nhớ: Working, Task Episodic, Workspace Semantic, Procedural, Human Preference.
- Cơ chế thăng hạng và thu hồi bộ nhớ dựa trên phản hồi của người dùng.

### 1.5 Tương Tác Nhà Cung Cấp (Interoperability)
- Trừu tượng hóa `ProviderPort` độc lập với API đặc thù của từng hãng.
- Gói `ContinuationPacket` cho phép bàn giao trạng thái công việc sang model khác tại các điểm an toàn.
- Tích hợp chuẩn giao thức công cụ MCP (Model Context Protocol) ở ranh giới bên ngoài.

### 1.6 Chứng Cứ & Kiểm Toán (Evidence & Audit)
- Phân loại và thu thập 6 lớp chứng cứ khách quan từ môi trường.
- Đóng gói kết quả vào `Verifiable Outcome Bundle` kèm diff, logs và receipts.
- Sổ cái quyết định `Decision Ledger` ghi nhận mọi lựa chọn kỹ thuật và lý do tương ứng.

---

## 2. Bảng Phân Hạng Tính Năng Theo Thứ Tự Ưu Tiên (Feature Taxonomy)

### 2.1 Bắt Buộc cho Alpha (Must — Alpha)
1. Đăng ký workspace và repository cục bộ.
2. Thiết lập và kiểm tra hợp đồng Task Contract.
3. Quản lý trạng thái Task/Step bền vững trong SQLite.
4. Giao diện dòng lệnh (CLI) hoàn chỉnh cho vòng đời task.
5. Tạo snapshot chỉ mục và tìm kiếm nhanh mã nguồn (Git + ripgrep).
6. Bộ biên soạn ContextPack cơ bản có kiểm tra ngân sách.
7. Adapter ProviderPort đầu tiên (Codex hoặc Claude).
8. Cơ chế cô lập thay đổi bằng Git worktree.
9. Capability Gateway kiểm soát áp dụng patch và lệnh shell cơ bản.
10. Cơ chế dừng chờ phê duyệt với payload cụ thể.
11. Bộ xác thực (Verifier) kiểm tra kết quả bằng test command.
12. Hồi phục sau sự cố ngắt tiến trình (Pause/Resume/Crash Recovery).
13. Xuất gói Verifiable Outcome Bundle.
14. Ghi nhận và hiển thị chi phí token cục bộ.

### 2.2 Bắt Buộc cho Cognitive Beta (Must — Cognitive Beta)
1. Cấu trúc schema DecisionCase / RDC hoàn chỉnh.
2. Kho câu hỏi Versioned Question Registry.
3. Adapter JudgmentPort kết nối System One.
4. Chế độ vận hành ngầm (Shadow/Advisory mode) để đo lường độ chính xác.
5. Gói câu hỏi lọc ngữ cảnh (Context-Triage Pack).
6. Gói câu hỏi tiếp nhận bài toán (Task-Intake Pack).
7. Gói câu hỏi phản biện bản vá (Patch-Challenge Pack).
8. Báo cáo định chuẩn độ tin cậy (Calibration Report).
9. Ghi nhận lịch sử vào Decision Ledger.
10. Kiểm soát lọc dữ liệu nhạy cảm trước khi gửi sang engine phán đoán.

### 2.3 Nên Có cho Tính Di Động (Should — Portability & DX)
1. Adapter ProviderPort thứ hai (chuyển đổi linh hoạt giữa 2 hãng lớn).
2. Thử nghiệm năng lực mô hình tự động (Capability Probes).
3. Đóng gói và chuyển giao tiến trình qua ContinuationPacket.
4. Điểm dừng an toàn để chuyển đổi provider giữa chừng.
5. Giao diện hiển thị Task và Artifacts trên VS Code.
6. Bộ lưu lại và phát lại sự kiện provider để phục vụ kiểm thử hồi quy.
7. Tự động sinh chỉ dẫn tối ưu cho từng provider đặc thù.

### 2.4 Mở Rộng Sau: Research Pack (Later — Research)
1. Nạp và phân tích tài liệu PDF, HTML.
2. Mô hình hóa mối quan hệ khẳng định - chứng cứ (Claim-Evidence Model).
3. Bản đồ trích dẫn và liên kết tài liệu.
4. Tự động kiểm chứng nguồn trích dẫn.
5. Xuất báo cáo tổng hợp dạng Markdown hoặc Obsidian Vault.
6. Kết nối trích xuất thư mục từ Zotero.

### 2.5 Mở Rộng Sau: Personal Pack (Later — Personal Operations)
1. Kết nối an toàn với Email, Lịch, Trình nhắc việc.
2. Tự động tra cứu và phân giải danh tính người nhận.
3. Quy trình 3 bước: Soạn thảo → Xem trước xác nhận → Cam kết thực hiện.
4. Chính sách ưu tiên cá nhân hóa theo từng người dùng.
5. Lập lịch thông báo thông minh tránh gián đoạn tập trung.
6. Báo cáo tổng kết chéo các lĩnh vực vào cuối ngày.

### 2.6 Tuyên Bố Hoãn Tường Minh (Explicitly Deferred)
- Không xây dựng máy chủ đám mây dùng chung cho nhóm trong giai đoạn này.
- Không xây dựng chợ mua bán plugin/agent (Marketplace).
- Hoãn giao thức phân tán A2A (Agent-to-Agent) qua mạng từ xa.
- Không hỗ trợ tự động triển khai mã nguồn lên môi trường Production.
- Hoãn việc tích hợp cơ sở dữ liệu đồ thị độc lập chuyên dụng.
- Không phát triển ứng dụng trên thiết bị di động.
