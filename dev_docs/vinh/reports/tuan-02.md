# Báo cáo Tiến độ Tuần 02 (Sơ bộ) — Nghiên cứu Kiến trúc Goose & Giao thức AgentGateway

> **Người thực hiện:** Vinh (Vinh)  
> **Vai trò:** Agent Systems & Coordination Research Engineer  
> **Chu kỳ:** Sprint 2 — Multi-Agent Coordination, Goose Comparative Analysis & AgentGateway Protocol Architecture  
> **Thời gian:** 25/09/2026 – 01/10/2026  
> **Nhánh phát triển:** `vinh` ➔ `dev`  

---

## 1. Tổng quan Mục tiêu & Trọng tâm Nghiên cứu Tuần 02

Bước sang Tuần 02, trọng tâm nghiên cứu của vai trò Agent Systems & Coordination Research Engineer tập trung vào việc đối chuẩn (benchmarking) và phân tích sâu các hệ thống agent tương đương trong thực tế nhằm rút ngắn thời gian thiết kế, tránh các bẫy kiến trúc (anti-patterns), và định hình giao thức kết nối ngoại vi cho Custos:

1. **Phân tích Chuyên sâu Kiến trúc Goose (Hệ thống Local Agent bằng Rust của Block / AAIF):**
   - Mổ xẻ chi tiết vòng đời agent-loop trong Goose (`goose-agent` state machine, `MachineSession`, `Operation`, `InferenceRunner`).
   - Khảo sát mô hình mở rộng công cụ (`ExtensionManager`), tích hợp giao thức Model Context Protocol (`rmcp`), và hệ thống lưu trữ session SQLite.
   - Đánh giá cơ chế kiểm soát bảo mật, phân quyền (`permission modes`), và các hạn chế thực tế (fail-open vs fail-closed) của Goose để làm cơ sở đối chiếu với Custos.
2. **Nghiên cứu Kiến trúc & Logic của AgentGateway (A2A Protocol):**
   - Tìm hiểu cơ chế hoạt động của AgentGateway trong vai trò cổng trung chuyển giao tiếp giữa các tác tử (Agent-to-Agent / A2A).
   - Nghiên cứu mô hình đóng gói phong bì ngữ cảnh (`Handoff Envelope`), truyền tải trạng thái (`Context Propagation`), và cơ chế khám phá năng lực (`Capability Discovery`).
   - Định hình lớp adapter protocol dự kiến đặt tại `adapters/protocols/agentgateway/` kết nối vào `crates/workflow-runtime`.
3. **Xác lập Bài học Kiến trúc Áp dụng cho Custos:**
   - Xác định ranh giới những gì nên kế thừa (mô hình state machine tinh gọn của `goose-agent`) và những gì tuyệt đối không áp dụng (mô hình fail-open trong bảo mật, coupling chặt chẽ giữa session conversation và task execution).

---

## 2. Bảng tổng hợp Công việc & Nghiên cứu trong Tuần

| Hạng mục / Nhiệm vụ | Thời gian | Nội dung công việc & Phân tích chính | Trạng thái |
|---|---|---|---|
| **Nghiên cứu Goose Engine** | 2026-09-25 | **Khảo sát Repo & Vòng đời `goose-agent`:** Phân tích mã nguồn snapshot v1.52.0, bóc tách cấu trúc giữa loop legacy (`agent.rs`) và state machine mới (`goose-agent::machine`). | Đã hoàn thành |
| **Phân tích Bảo mật Goose** | 2026-09-25 | **Đánh giá Security & Permission Inspectors:** Phân tích `EgressInspector`, `AdversaryInspector`, chỉ ra điểm yếu fail-open và đối chiếu với mô hình fail-closed của Custos. | Đã hoàn thành |
| **Khảo sát AgentGateway** | 2026-09-26 | **Nghiên cứu Protocol Binding & Handoff:** Phân tích cấu trúc envelope, cơ chế định tuyến A2A message, chuẩn hóa schema ngữ cảnh luân chuyển giữa các agent. | Đang tiến hành |
| **Thiết kế Adapter Sơ bộ** | 2026-09-26 | **Phác thảo Module `adapters/protocols/agentgateway`:** Xác định trait giao tiếp giữa workflow runtime và các agent bên ngoài thông qua Gateway. | Đang tiến hành |

---

## 3. Chi tiết các Hạng mục Đã Thực hiện & Phát hiện Quan trọng

### 3.1. Phân tích Chi tiết Kiến trúc Hệ thống Goose (Block / AAIF)

Goose là một trong những hệ sinh thái local agent viết bằng Rust tiêu biểu nhất hiện nay. Qua khảo sát chuyên sâu mã nguồn thực tế tại snapshot commit `302b608`, ghi nhận các đặc trưng kiến trúc sau:

#### A. Cấu trúc Vòng đời Agent Loop (`goose-agent` vs `legacy`)
- Goose hiện tồn tại hai luồng thực thi song song:
  - **Legacy Loop (`agents/agent.rs`):** Cồng kềnh, ràng buộc chặt vào business logic của ứng dụng.
  - **State Machine Loop (`crates/goose-agent`):** Kiến trúc mới hướng tới GDK (Goose Development Kit). Vận hành dựa trên `MachineSession`, `Operation`, `InferenceRunner` và `ToolOperation`.
- **Cơ chế chuyển đổi trạng thái:** Machine chạy theo chuỗi các step, sinh ra `effects`, sau đó áp dụng vào session thông qua `apply_effects`.
- *Ý nghĩa cho Custos:* Crate `goose-agent` là một reference pattern rất tốt về tính tinh gọn cho worker state machine của Custos (`crates/workflow-runtime`), nhưng Custos cần bổ sung hợp đồng `ExecutionPermit` trước khi phát sinh bất kỳ effect nào ra hệ điều hành.

#### B. Cơ chế Quản lý Công cụ & Extension Manager
- Tích hợp chuẩn **Model Context Protocol (MCP)** thông qua crate `rmcp 3.2.0`.
- Hỗ trợ đa dạng transport: `stdio` (chạy tiến trình con cục bộ) và `streamable HTTP` (kết nối remote/local server).
- Goose quản lý vòng đời extension qua `ExtensionManager` đảm nhiệm discovery, auth OAuth và event dispatching.
- *Lưu ý cho Custos:* MCP server bản chất là một process có thể gây tác động ngay khi startup. Do đó, trong Custos, việc khởi chạy và gọi MCP phải chịu sự giám sát của `AuthorityEngine` và `CapabilityGateway` (fail-closed), không được ủy quyền tự do.

#### C. Phân tích Ranh giới Bảo mật & Hạn chế của Goose
Goose sở hữu nhiều lớp bảo vệ, nhưng khi khảo sát sâu mã nguồn, bộc lộ các điểm hạn chế cốt tử:
- **Egress Inspector:** Khi nhận diện lệnh shell/web có kết nối ra ngoài, Goose chỉ ghi log `network egress detected` nhưng vẫn trả về `InspectionAction::Allow`.
- **Adversary Inspector & Malware Check:** Khi gọi mô hình LLM inspector hoặc kiểm tra dữ liệu OSV gặp lỗi mạng/HTTP, mã nguồn Goose chọn nhánh **`fail-open`** (tự động cho phép chạy tiếp).
- *Nguyên tắc Custos:* Custos tuyệt đối không chấp nhận fail-open. Mọi thẩm tra thẩm quyền khi lỗi hoặc không thể đánh giá đều phải **fail-closed** (từ chối thực thi và yêu cầu con người phê duyệt).

---

### 3.2. Nghiên cứu Kiến trúc & Logic của AgentGateway (A2A Protocol)

AgentGateway đóng vai trò là một tầng cổng giao thức (Protocol Gateway) cho phép các hệ thống tác tử độc lập (Multi-Agent Systems) giao tiếp, ủy thác công việc và trao đổi trạng thái một cách tin cậy:

1. **Chuẩn hóa Handoff Envelope (Phong bì Chuyển giao):**
   - Thay vì truyền toàn bộ lịch sử trò chuyện nguyên bản (tránh hiện tượng transcript explosion), AgentGateway chuẩn hóa gói thông tin bàn giao thành envelope có cấu trúc:
     - `task_id` & `correlation_id`: Định danh phiên làm việc và vết thực thi.
     - `objective`: Mục tiêu cô đọng của bước tiếp theo.
     - `completed_work`: Bản tóm tắt kết quả đã kiểm chứng từ worker trước.
     - `artifacts`: Danh sách file/tài nguyên đã tạo hoặc sửa đổi kèm hash sha256.
     - `constraints & budget`: Giới hạn token, thời gian và phạm vi quyền hạn.
2. **Cơ chế Khám phá Năng lực (Capability Discovery):**
   - Cho phép các agent đăng ký và công bố danh sách kỹ năng / công cụ mà mình sở hữu.
   - Hỗ trợ workflow router gửi task đến đúng chuyên gia (ví dụ: Code Worker, Research Worker, Test Runner).
3. **Định vị trong Cấu trúc Custos:**
   - Module `adapters/protocols/agentgateway/` sẽ đóng vai trò cầu nối: biên dịch giữa `ContinuationPacket` nội bộ của Custos và giao thức AgentGateway chuẩn hóa bên ngoài, mở ra khả năng cộng tác với các agent framework khác mà vẫn đảm bảo tính toàn vẹn dữ liệu.

---

### 3.3. So sánh Đối chiếu Kiến trúc: Goose vs. AgentGateway vs. Custos

| Tiêu chí | Goose (Block / AAIF) | AgentGateway (A2A) | Custos v4.0 (Đang phát triển) |
|---|---|---|---|
| **Triết lý Cốt lõi** | Trợ lý cá nhân tương tác (Conversation-centric agent) | Cổng giao tiếp & định tuyến tác tử (Protocol & Routing Hub) | Hệ điều hành tác tử cục bộ, bảo mật đa tầng (Local-first, Task-centric OS) |
| **Quản trị Tiến trình** | Single agent loop (có thử nghiệm subagents) | Định tuyến thông điệp giữa các agent độc lập | Multi-worker DAG Coordination với formal state machine |
| **Mô hình Trạng thái** | SQLite (`sessions`, `messages`, `usage_ledger`) | Stateless hoặc Session Envelope trung gian | Event-sourced CQRS (`tasks`, `spans`, `domain_events`, `outbox`) |
| **Chính sách An toàn** | Nhiều inspector nhưng thiên về **Fail-Open** | Ủy quyền bảo mật cho từng agent endpoint | Bắt buộc **Fail-Closed**, Single-use `ExecutionPermit`, OS Sandbox |
| **Giao tiếp Công cụ** | MCP Client (`rmcp`) + Local Shell | Định dạng Tool Call chuẩn hóa qua API | `CapabilityGateway` cách ly Seatbelt / Bubblewrap |

---

## 4. Bài học Rút ra & Định hướng Thiết kế cho Custos

1. **Học tập Cấu trúc State Machine của `goose-agent`:**  
   Thiết kế `crates/workflow-runtime` có thể tham khảo cách phân tách sạch sẽ giữa `machine` (logic điều phối vòng lặp) và `operations` (các bước cụ thể như tóm tắt, chuẩn bị prompt, gọi tool), giúp code dễ viết unit test và benchmark độc lập.
2. **Kiên định với Mô hình Fail-Closed & ExecutionPermit:**  
   Không thỏa hiệp bảo mật theo hướng fail-open như Goose. Mọi hành động của worker phát sinh hiệu ứng ổ đĩa, mạng hay shell bắt buộc phải thông qua `ExecutionPermit` được cấp từ Task Kernel.
3. **Tận dụng Handoff Protocol của AgentGateway:**  
   Ứng dụng cấu trúc envelope vào `ContinuationPacket` để đảm bảo chuyển giao ngữ cảnh sạch giữa các worker, giảm thiểu ô nhiễm prompt và đo lường được lượng token tiết kiệm.

---

## 5. Kế hoạch Cho Phần Tiếp theo của Tuần 02

1. **Hiện thực hóa Module Worker Lifecycle trong `crates/workflow-runtime`:**
   - Cài đặt state machine cho Worker với các trạng thái rõ ràng: `Created`, `Leased`, `Active`, `WaitingTool`, `Paused`, `Completed`, `Failed`.
2. **Thiết kế Schema Giao thức cho `adapters/protocols/agentgateway`:**
   - Định nghĩa struct Rust và schema Serde cho `HandoffEnvelope` và `AgentMessage`.
3. **Phối hợp Đồng bộ cùng Vi & Truong:**
   - Thảo luận cùng Vi về việc chuẩn hóa dữ liệu ngữ cảnh trong Handoff.
   - Thảo luận cùng Truong về cơ chế tích hợp Permit vào vòng đời worker khi dispatch tool calls.
