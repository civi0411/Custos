# Triết Lý Thiết Kế, Bất Biến & Quyết Định Kiến Trúc

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần II (§6-8) & Phần VI (§43) Canonical Specification

Tài liệu này xác định các **hàng rào bảo vệ kiến trúc (architectural guardrails)** của Custos: những điều hệ thống cam kết luôn tuân thủ, các nguyên lý không thể nhân nhượng, và các quyết định chiến lược đã qua phản biện.

---

## 1. Mười Quyết Định Không Thể Đảo Ngược (Irreversible Decisions)

Các quyết định này là nền móng cấu trúc; không được phép thay đổi nếu không có sự đồng thuận kiến trúc toàn diện (Major Architecture Review):

1. **Kernel sở hữu toàn quyền:** Kernel (chứ không phải model hay agent) sở hữu state, authority, budget và commit.
2. **State độc lập Provider Session:** Task state hoàn toàn độc lập với phiên làm việc của AI provider; đổi provider không làm mất trạng thái công việc.
3. **Domain Pack thay vì Agent nguyên khối:** Sử dụng các Domain Pack mở rộng theo module nghiệp vụ thay cho các "siêu agent" đơn lẻ.
4. **Worker theo vai trò ngắn hạn (Ephemeral Workers):** Worker được sinh ra theo vai trò cụ thể cho từng subtask và kết thúc ngay khi hoàn thành scope, không giữ state vĩnh viễn.
5. **System One là Judgment Fabric đa backend:** Tách biệt phán đoán nhanh (System One) khỏi suy luận sâu (System Two); System One là hạ tầng đa backend có thể cắm ghép, không phụ thuộc vào duy nhất một nhà cung cấp.
6. **Confidence không bao giờ tạo Capability:** Độ tin cậy (confidence score) dù cao đến đâu cũng không tự ý nâng quyền hạn thực thi; quyền hạn chỉ đến từ cấp phép tường minh của con người (Human Grant).
7. **Nội bộ dùng Typed Contract:** Bên trong hệ thống chỉ giao tiếp bằng typed commands, events và artifacts; các giao thức mở như MCP hay A2A chỉ nằm tại ranh giới tích hợp (boundary).
8. **Mọi Side Effect qua Capability Gateway:** Mọi hành động làm thay đổi môi trường bên ngoài đều phải đi qua Capability Gateway, kiểm tra ExecutionPermit và ghi lại receipt/audit log.
9. **Hoàn thành cần Evidence:** Trạng thái "Completed" của một Task đòi hỏi phải có bằng chứng kiểm định (evidence) đáp ứng Task Contract, không chấp nhận lời khẳng định chay của model.
10. **Benchmark tự thân:** Mọi công bố về mức độ tiết kiệm token hay cải thiện hiệu năng phải xuất phát từ benchmark độc lập và có bằng chứng của Custos.

---

## 2. Bảy Nguyên Tắc Sản Phẩm (Product Principles)

| # | Nguyên tắc | Diễn giải chi tiết |
|---|---|---|
| **1** | **Task-Centered** | Đơn vị vận hành trung tâm là **Task** với hợp đồng rõ ràng, không phải phiên chat, luồng hội thoại hay danh sách prompt. |
| **2** | **Human-Governed** | Con người giữ chủ quyền tối cao (*Human Sovereignty*): con người định nghĩa mục tiêu (*intent*), ràng buộc (*constraints*), giá trị và phê duyệt các điểm rủi ro. |
| **3** | **Local-First** | Toàn bộ dữ liệu nhạy cảm, trạng thái task, lịch sử và mã nguồn được lưu trữ và kiểm soát tại máy trạm cá nhân; không có dữ liệu nào bị đưa lên mây nếu không có sự cho phép. |
| **4** | **Provider-Neutral** | Tương thích linh hoạt với OpenAI Codex, Anthropic Claude, Google Antigravity, hoặc Local LLMs (Ollama, llama.cpp, vLLM) mà không làm rách vỡ ngữ cảnh task. |
| **5** | **Evidence-Driven** | Công việc chỉ được coi là hoàn tất khi có bằng chứng khách quan: unit test pass, build clean, static analysis sạch, hash trùng khớp hoặc chữ ký phê duyệt. |
| **6** | **Capability-Based** | Kiến trúc bảo mật dựa trên quyền hạn rõ ràng (*Capability-based Security*): công cụ không được tự ý thực thi ngoài phạm vi được cấp phép trong `ExecutionPermit`. |
| **7** | **Durable by Design** | Mọi chuyển dịch trạng thái được lưu trữ bền vững vào SQLite + WAL và CAS; sự cố ngắt nguồn, crash ứng dụng hay mất mạng đều có thể tiếp tục (*resume*) chính xác. |

---

## 3. Tám Bất Biến Hệ Thống (System Invariants)

Các bất biến này được kiểm tra bằng các bài kiểm thử xác minh hình thức (Formal & Integration Tests):

| Mã | Invariant | Định nghĩa hình thức | Diễn giải |
|---|---|---|---|
| **I1** | **Authority Invariant** | `dispatch(a) → valid(Auth_a, t_dispatch)` | Một hành động $a$ chỉ được phát động khi tồn tại ủy quyền hợp lệ tại thời điểm phát động. |
| **I2** | **Provenance Invariant** | `context ⊬ authority` | Sự xuất hiện của thông tin trong context hay prompt không bao giờ tương đương với việc được cấp quyền hành động. |
| **I3** | **Budget Invariant** | `reserve + settle ≤ ceiling` | Tổng chi phí đã giải ngân cộng khoản dự trữ không bao giờ vượt quá ngân sách trần của task. |
| **I4** | **Evidence Invariant** | `SUCCEEDED(a) → receipt(a) ∧ verifier_passed(a)` | Hành động chỉ được đánh dấu thành công khi có biên nhận thực thi và vượt qua bộ kiểm tra (*verifier*). |
| **I5** | **Continuation Invariant** | `resume(a) → StateValid ∧ AuthValid ∧ PreValid ∧ EffectsResolved` | Khôi phục task đòi hỏi trạng thái hợp lệ, quyền hạn còn hiệu lực, điều kiện tiên quyết thỏa mãn và side effect trước đó đã được đối soát. |
| **I6** | **Fencing Invariant** | `publish(result) → lease_epoch(result) = current_epoch` | Kết quả của worker chỉ được chấp nhận nếu epoch phát hành trùng khớp với epoch hợp lệ hiện tại của task (chống split-brain). |
| **I7** | **Privacy Invariant** | `secret ∉ C_a ∧ (egress(C_a) → privacy_gate(C_a))` | Bí mật thông tin không bao giờ lọt vào ngữ cảnh $C_a$; mọi dữ liệu gửi ra ngoài đều phải qua cổng kiểm duyệt quyền riêng tư. |
| **I8** | **Fallback Invariant** | `fallback(model) → privacy_gate ∧ authority_gate` | Chuyển đổi model dự phòng bắt buộc phải kích hoạt lại toàn bộ cổng bảo mật và kiểm tra quyền hạn tương ứng. |

---

## 4. Quyết Định Chiến Lược: Giữ, Sửa, Hoãn, Loại (Mục 43)

Trong quá trình tiến hóa từ các phiên bản sơ khởi lên Canonical Architecture v4.0, các quyết định sau đã được ấn định dứt khoát:

| Ý tưởng đề xuất ban đầu | Quyết định cuối cùng | Lý do kiến trúc & Định hướng thay thế |
|---|---|---|
| **Local-first** | **GIỮ** | Nền tảng cốt lõi cho quyền riêng tư và quyền làm chủ dữ liệu của người dùng. |
| **Human-in-the-loop** | **SỬA** | Nâng cấp từ "hỏi ý kiến" bị động thành **Human Sovereignty & Attention Budget** (giao dịch chủ quyền có định mức). |
| **Jev** | **SỬA** | Không để Jev làm engine độc quyền; giữ Jev như **một adapter first-class** trong Judgment Fabric đa backend (Pluggable System One). |
| **LLM agents** | **GIỮ** | Đặt vào tầng Deliberation Fabric để suy luận sâu, nhưng bị kiểm soát bởi System One và Kernel. |
| **3 Agent cố định nguyên khối** | **SỬA** | Chuyển thành **Domain Packs** (Engineering, Research, Personal) sinh ra các vai trò ngắn hạn (*ephemeral roles*). |
| **Agent-to-agent chat tự do** | **LOẠI** | Loại bỏ hoàn toàn chat tự do giữa các agent để tránh loop và ảo giác; thay bằng **bàn giao qua Event và Artifact có kiểm chứng**. |
| **MCP dùng trong toàn bộ nội bộ** | **LOẠI** | MCP có overhead lớn và thiếu typed safety cho internal kernel; chỉ dùng MCP ở **biên kết nối công cụ và tài nguyên ngoài**. |
| **A2A (Agent-to-Agent protocol)** | **HOÃN** | Hoãn lại cho đến chân trời liên kết mạng phân tán (H3 Federation Mesh); không đưa vào core MVP. |
| **Tên tự chế (JEP, UJE, CP)** | **LOẠI** | Loại bỏ tên riêng không chuẩn; dùng typed schema chuẩn công nghiệp và giao thức **RDC (Request-Decision-Challenge)**. |
| **Memory Graph phức tạp từ đầu** | **HOÃN** | Bắt đầu với SQLite relational edges và FTS5; hoãn cơ sở dữ liệu graph chuyên dụng cho đến khi có nhu cầu thực tế. |
| **Full Research/Personal trong MVP** | **HOÃN** | Tập trung làm xuất sắc **Engineering Domain Pack v1** trong H1 trước khi mở rộng toàn diện sang Research và Personal. |
| **Tự động push git / deploy / gửi email** | **LOẠI KHỎI MVP** | Mọi hành động có side-effect mức cao bắt buộc phải có xác nhận tường minh của con người (Exact-Payload Approval). |
| **Tuyên bố hiểu toàn bộ repo** | **SỬA** | Thay bằng **Measured Coverage** (độ bao phủ được đo lường chính xác bằng ripgrep và tree-sitter AST). |
| **Viết paper nghiên cứu là mục tiêu** | **LOẠI** | Ưu tiên **sản phẩm phần mềm thực tế, bền bỉ và tạo ra giá trị đo lường được** thay vì lý thuyết học thuật suông. |
