# Phân Tích So Sánh & Tính Độc Bản (Competitive Analysis & Moat)

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần I (§1.4) & Phần VI (§42) Canonical Specification

Tài liệu này phân tích vị thế kiến trúc của Custos trong hệ sinh thái phần mềm agent hiện nay, làm rõ sự khác biệt bản chất so với các giải pháp khác và định hình chiến lược phòng thủ (*product moat*).

---

## 1. Bảng So Sánh Chi Tiết Với Hệ Sinh Thái

| Tiêu chí | LangGraph / CrewAI | Temporal / Workflow Engines | Claude Code / Aider / Cursor | **Custos** |
|---|---|---|---|---|
| **Đơn vị trung tâm** | Graph node / Agent loop | Deterministic Activity / Workflow | Chat session / Diff buffer | **Durable, Evidence-carrying Task** |
| **Chủ quyền trạng thái (State Ownership)** | Framework memory / Python process | Central Workflow Cluster | Model session context | **Local Kernel (SQLite + WAL + Outbox)** |
| **Sự độc lập Provider** | Bị khóa vào API abstractions | Không tích hợp sẵn suy luận AI | Khóa chặt vào 1 model/vendor | **Model-Agnostic (Thay não không mất hồn)** |
| **Cơ chế Phán đoán (System One)** | Prompt chain / LLM router | Không có | Heuristic đơn giản trong client | **Judgment Fabric đa backend chuyên biệt** |
| **Bảo mật & Quyền hạn** | Tool-call trực tiếp không kiểm soát | Worker permissions cố định | Prompt confirmation cục bộ | **Capability-based ExecutionPermits** |
| **Tiêu chí Hoàn thành** | LLM output text | Activity return code | Diff accepted | **Verifiable Outcome Bundle (Đa tầng chứng cứ)** |
| **Kiến trúc Vận hành** | In-memory library | Client-Server cluster | Interactive local CLI/IDE | **Local-first Daemon + Ephemeral Workers** |

---

## 2. Điểm Đột Phá Kiến Trúc Của Custos (Unique Architectural Fusion)

Sự khác biệt của Custos không đến từ việc đặt tên protocol phức tạp mà từ **sự kết hợp chuẩn mực của 10 yếu tố**:

1. **Durable Task State độc lập Provider:** Trạng thái công việc thuộc về Kernel địa phương, cho phép chuyển đổi giữa Codex, Claude và local model mà không bị mất tiến trình.
2. **Domain Pack + Ephemeral Workers:** Loại bỏ các agent nguyên khối nặng nề; worker được sinh ra theo vai trò cho từng subtask và hủy ngay khi hoàn thành.
3. **System One bao quanh System Two:** Hạ tầng phán đoán nhanh, chi phí thấp (System One) kiểm soát, điều hướng và giảm tải cho suy luận đắt đỏ của LLM (System Two).
4. **Typed Cognitive Escalation & Commit:** Mọi quyết định leo thang hay cam kết đều được định kiểu dữ liệu chặt chẽ qua giao thức RDC (Request-Decision-Challenge).
5. **Capability-based Execution:** Áp dụng mô hình bảo mật hướng quyền năng; không có công cụ nào được chạy ngoài giấy phép `ExecutionPermit` đã được ký duyệt.
6. **Context Compilation có Provenance:** Ngữ cảnh nạp cho model được tổng hợp có truy xuất nguồn gốc rõ ràng, ghi điểm liên quan và loại bỏ trùng lặp.
7. **Evidence-based Completion:** Chỉ đóng task khi có chứng cứ khách quan từ máy đo kiểm, không dựa vào câu trả lời khẳng định của model.
8. **Human Attention như một định mức ngân sách:** Tôn trọng sự chú ý của con người; gom nhóm phê duyệt theo mức độ rủi ro, không làm phiền bởi những câu hỏi vụn vặt.
9. **Local-first Knowledge Continuity:** Dữ liệu dự án, bộ nhớ và tri thức tích lũy thuộc về người dùng, hoạt động ngoại tuyến và không rò rỉ lên cloud.
10. **Artifact-based Cross-provider Handoff:** Bàn giao kết quả giữa các tác vụ và nhà cung cấp bằng artifacts có địa chỉ nội dung (CAS), không chuyển giao qua lịch sử chat dài dòng.

---

## 3. Khả Năng Khó Sao Chép (Defensible Product Moat)

Moat của Custos không nằm ở lớp code bọc API (*thin wrapper*) mà tích lũy từ dữ liệu vận hành thực tế:

- **Decision Ledger:** Sổ cái ghi lại toàn bộ quyết định, lý do, rủi ro và phản hồi của con người theo thời gian.
- **Calibrated Question Packs:** Bộ câu hỏi định chuẩn giúp System One phân loại nhanh ý định và phát hiện mơ hồ.
- **Failure & Recovery Corpus:** Dữ liệu về các tình huống lỗi thực tế và cách khôi phục tự động thành công.
- **Provider Compatibility Knowledge:** Tri thức đo kiểm thực tế về điểm mạnh, điểm yếu và chi phí của từng model.
- **Verifier & Evidence Recipes:** Thư viện các bộ kiểm tra tự động cho từng loại ngôn ngữ và bài toán kỹ thuật.

---

## 4. Chỉ Số Mục Tiêu Tối Thượng (North-Star Metric)

> **Verified outcomes per unit of total cost**  
> *(Kết quả được xác minh trên mỗi đơn vị tổng chi phí)*

Trong đó **Tổng chi phí (Total Cost)** bao gồm:
$$	ext{Total Cost} = 	ext{Tài chính (API fees)} + 	ext{Thời gian chờ} + 	ext{Sự chú ý của con người (Human Interruptions)} + 	ext{Chi phí sửa sai (Remediation)}$$
