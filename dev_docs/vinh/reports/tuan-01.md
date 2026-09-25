# Báo cáo Tiến độ Tuần 01 — Project Onboarding, 9router Analysis & AI Runtime Architecture Blueprint

> **Người thực hiện:** Vinh (Vinh)  
> **Vai trò:** Agent Systems & Coordination Research Engineer  
> **Chu kỳ:** Sprint 1 — The Local Kernel, CLI Experience & First Vertical Slice (`repo_explain`)  
> **Thời gian:** 21/09/2026 – 24/09/2026  
> **Nhánh phát triển:** `vinh` ➔ `dev`  

---

## 1. Tổng quan Mục tiêu & Kết quả Tuần 01

Trong tuần khởi động đầu tiên của dự án Custos, vai trò Agent Systems & Coordination Research Engineer tập trung vào 3 trọng tâm chiến lược:

1. **Đọc hiểu Dự án & Thiết lập Ranh giới Kiến trúc (Custos Onboarding):** Nghiên cứu toàn diện Master Agent Protocol (`AGENTS.md`), Blueprint hệ thống (`docs/canonical-specification.md`), và ma trận trách nhiệm (`dev_docs/README.md`). Định hình rõ ranh giới hệ thống: điều phối đa tác tử dưới góc độ kỹ thuật phần mềm phân tán (Systems Software Engineering), bảo toàn tính bất biến (invariants) của `crates/workflow-runtime`, tuyệt đối không can thiệp vào logic prompt hay heuristic nhận thức thuộc quyền của Vi, và không vi phạm thẩm quyền bảo mật của Truong.
2. **Nghiên cứu Chuyên sâu Hệ thống 9router & RTK (Rust Token Killer):** Phân tích cơ chế hoạt động của 9router cùng công nghệ RTK (Rust Token Killer) chuyên trách cắt giảm và tối ưu hóa triệt để lượng token tiêu thụ, kỹ thuật chuẩn hóa và chuyển tiếp request (Request Forwarding / Translation) đến các LLM provider đa dạng (OpenAI, Anthropic, Gemini, Groq, local models), và cơ chế chuyển đổi dự phòng 3 tầng (3-Tier Fallback Resilience).
3. **Phác thảo Bản thiết kế Kiến trúc Điều phối Đầu tiên:** Vận dụng các triết lý cốt lõi từ 9router kết hợp với yêu cầu local-first của Custos để hoàn thiện bản đặc tả kiến trúc tổng thể. Bản thiết kế giải quyết dứt điểm sự tách biệt giữa **Workflow Router** (điều phối topology tác tử/DAG) và **Model Router** (lựa chọn mô hình theo capability và ngân sách), cấu trúc bộ nhớ ngữ cảnh 5 lớp (L0–L4), và chuẩn hóa Provider Adapter đa tầng (API và CLI transport).

---

## 2. Bảng tổng hợp Pull Requests & Commits trong Tuần

| PR / Commit | Thời gian | Mô tả công việc & Thay đổi chính | Trạng thái |
|---|---|---|---|
| **PR #15** | 2026-09-23 | **Tích hợp Frame UI Mascot cho CLI (`apps/custos-cli`):** Đóng góp bộ khung ảnh đồ họa mascot cú tuyết, phối hợp cùng nhánh `truong` hoàn thiện pipeline hiển thị CLI UI ban đầu. | Merged |
| **Commit `f05f16d`** | 2026-09-23 | Thêm tài nguyên ảnh UI frame mascot vào `custos-cli`. | Merged (PR #15) |
| **Commit `5df3650`** | 2026-09-23 | Thiết lập cấu hình đồng tác giả (Co-authored-by) và chuẩn hóa branch tracking. | Merged (PR #17) |
| **PR #17** | 2026-09-23 | Đồng bộ hóa nhánh `vinh` với các cập nhật cốt lõi từ `dev`. | Merged |
| **Commit `8bedb6a`** | 2026-09-23 | Cập nhật đồng bộ hóa metadata và quyền tác giả. | Merged (PR #18) |
| **PR #18** | 2026-09-23 | Merge hoàn tất đồng bộ branch governance lên `dev`. | Merged |
| **Architecture Blueprint** | 2026-09-24 | **Phác thảo Thiết kế Custos:** Hoàn thành tài liệu phác thảo thiết kế kiến trúc toàn diện gồm 44 mục, mô hình hóa 7 plane, phân tách Workflow Router vs Model Router, và tích hợp cơ chế 3-tier fallback của 9router. | Không merge chỉ review |

---

## 3. Chi tiết các Hạng mục Đã Thực hiện & Thay đổi Cụ thể

### 3.1. Nghiên cứu & Nắm bắt Kiến trúc Custos
- **Ranh giới Phân định Quyền hạn (Boundary Matrix):**
  - Xác lập nguyên tắc tiếp cận: Điều phối tác tử là bài toán kỹ thuật hệ thống đồng thời (concurrency, lifecycle, scheduling, backpressure), không phải bài toán AI persona.
  - Phối hợp với Vi: Vi nắm quyền định nghĩa nội dung nhận thức (`Cognitive Specification`, `ContextRecipe`, prompt logic). Vinh hiện thực hóa cơ chế thực thi (`Systems Implementation`, worker state machine, channel topologies, token budget enforcement).
  - Phối hợp với Truong: Truong quản lý tính bền vững (`persistence-sqlite`), giấy phép thẩm quyền (`ExecutionPermit`), và cách ly sandbox (`Seatbelt/Bubblewrap`). Vinh điều phối worker thực thi thông qua Kernel, không bao giờ tự ý ghi nhận trạng thái Task hoặc bypass sandbox.
- **Rà soát Invariants của `crates/workflow-runtime`:**
  - Zero I/O trực tiếp không qua Kernel hoặc Gateway.
  - Tách bạch rõ 5 module con dự kiến: `worker_lifecycle/`, `coordination/`, `scheduler/`, `handoff/`, `recovery/`.
  - Mọi thao tác async phải tôn trọng `tokio_util::sync::CancellationToken` và xử lý graceful shutdown.

### 3.2. Nghiên cứu Chuyên sâu Kiến trúc 9router & Công nghệ RTK (Rust Token Killer)
Nghiên cứu tài liệu và mã nguồn tham chiếu của 9router nhằm chắt lọc các giải pháp định tuyến và tối ưu hóa tài nguyên LLM hiệu năng cao:
- **Công nghệ RTK (Rust Token Killer) — Cắt giảm & Tối ưu Hóa Token Triệt để:**
  - **Bản chất RTK:** RTK (Rust Token Killer) là engine tối ưu hóa token viết bằng ngôn ngữ Rust với tốc độ xử lý cận 0ms (`sub-millisecond latency`), có nhiệm vụ thanh lọc, nén và cắt giảm lượng token tiêu thụ trước khi payload được gửi tới các LLM provider.
  - **Cơ chế cắt tỉa token:**
    - Tự động làm sạch prompt, loại bỏ các ký tự vô nghĩa, khoảng trắng thừa và cú pháp định dạng dư thừa.
    - Cắt lọc các system instruction lặp lại và loại bỏ triệt để hiện tượng ô nhiễm lịch sử hội thoại (`transcript pollution`).
    - Áp dụng kỹ thuật cắt tỉa ngữ cảnh thông minh (`smart context pruning` và `sliding window`), giữ lại các thông tin có giá trị lập luận cốt lõi và loại bỏ dữ liệu ngoại vi không cần thiết.
  - **Lợi ích kinh tế & hiệu năng:** Giúp cắt giảm từ 20% đến 40% lượng token tiêu thụ cho mỗi tác vụ, giảm chi phí vận hành API, ngăn chặn nguy cơ vượt quá cửa sổ ngữ cảnh (`context window overflow`), và giảm thời gian trễ mạng.
- **Cơ chế Chuyển tiếp Request & Ánh xạ Tham số (Request Forwarding & Parameter Mapping):**
  - **Chuẩn hóa Request Schema:** 9router chuẩn hóa request từ các client khác nhau theo một schema trung gian chung (`Canonical Request Schema`) trước khi phân giải tới endpoint mục tiêu.
  - **Ánh xạ tham số đa nền tảng (`Parameter Mapping`):** Tự động chuyển đổi các tham số tương ứng giữa các nhà cung cấp (ví dụ: `max_tokens` của OpenAI ➔ `max_output_tokens` của Anthropic/Gemini, xử lý `temperature`, `top_p`, `stop_sequences`).
  - **Quản lý kết nối & Streaming:** Tối ưu hóa pool kết nối HTTP/2 keep-alive, chuyển tiếp luồng dữ liệu phản hồi thời gian thực qua Server-Sent Events (SSE).
  - **Bảo mật Header:** Quản lý xác thực an toàn đa tài khoản, đảm bảo không rò rỉ API key vào hệ thống log (`Zero-leakage telemetry`).
- **Quản lý Hàng đợi & Giới hạn Tốc độ (Rate-Limit & Quota Governor):**
  - Theo dõi trạng thái sức khỏe định kỳ (`health probes`) và hạn mức RPM/TPM còn lại của từng provider account.
  - Giảm thiểu tình trạng thắt nút cổ chai khi nhiều tác tử đồng thời gửi request.

### 3.3. Cơ chế Phân giải Định tuyến Đa tầng & 3-Tier Fallback Resilience
Áp dụng trực tiếp bài học từ 9router để xây dựng chiến lược phục hồi 3 tầng khi gọi mô hình:
1. **Tier 1 — Hard Constraints Filtering (Lọc Ràng buộc Cứng):**
   - Loại bỏ ngay lập tức các provider đang down hoặc gặp sự cố quota.
   - Kiểm tra các điều kiện tiên quyết của tác vụ: yêu cầu thị giác (`vision`), hỗ trợ gọi công cụ (`tool_calling`), định dạng đầu ra có cấu trúc (`structured_output`), và độ dài context window (ví dụ: loại bỏ model < 200k tokens nếu tài liệu repo lớn).
2. **Tier 2 — Heuristic Scoring & Ranking (Chấm điểm & Xếp hạng Ứng viên):**
   - Đánh giá các model thỏa mãn Tier 1 theo thang điểm trọng số: `CodingScore`, `ReasoningScore`, `AvgLatency`, `CostPer1kTokens`, và `ReliabilityHistory`.
   - Sinh danh sách ưu tiên (Candidate Priority List) tối ưu hóa giữa chất lượng đầu ra và chi phí token.
3. **Tier 3 — Dynamic Fallback & Graceful Escalation (Dự phòng Động & Nâng bậc):**
   - Khi request gặp lỗi `429 Too Many Requests`, `503 Service Unavailable`, hoặc timeout, hệ thống tự động fallback tức thì sang model tiếp theo trong danh sách mà không ngắt quãng phiên làm việc của user.
   - Hỗ trợ cơ chế **Dynamic Downgrade / Upgrade**: Đối với các tác vụ trích xuất đơn giản của worker phụ, chuyển sang cheap model để tiết kiệm ngân sách; đối với tác vụ gặp lỗi kiểm thử 2 lần liên tiếp (`test fails × 2`), tự động nâng bậc lên strong reasoning model.

### 3.4. Phác thảo Thiết kế Kiến trúc Custos v1
Tài liệu hóa chi tiết kiến trúc điều phối runtime toàn diện gồm 44 chương mục, đặt nền móng cho việc hiện thực hóa `crates/workflow-runtime`:
- **Kiến trúc Tổng thể 7 Mặt phẳng (7-Plane Architecture):**
  - `Experience Plane`: CLI/TUI, Local Web App, VS Code Extension.
  - `Local Core Daemon`: `custosd` chịu trách nhiệm quản trị phiên, API Gateway (`/v1/*`, `/api/*`, `/events`).
  - `Decision Plane`: Tách biệt hoàn toàn **Workflow Router** (chọn chiến lược phối hợp tác tử) và **Model Router** (chọn LLM provider theo 9router).
  - `Context Plane`: Cấu trúc bộ nhớ phân cấp 5 tầng (L0: Hot Buffer, L1: Session, L2: Project, L3: User, L4: Semantic Archive) cùng cơ chế Provider Session Binding.
  - `Execution Plane`: Engine thực thi DAG, quản lý vòng đời tác tử đơn lẻ (Single-Agent) hoặc nhóm tác tử (Multi-Agent: Supervisor, Parallel Workers, Planner + Workers + Synthesizer).
  - `Provider Plane`: Tích hợp Provider Registry, Model Capability Metadata, và Provider Adapters hỗ trợ song song hai loại transport: **API Provider** (HTTPS) và **CLI Provider** (stdio/PTY cho Claude CLI, Codex CLI, Gemini CLI).
  - `Observability / Economics Plane`: Thu thập trace, phân bổ chi phí token từng worker, giám sát độ trễ và tỷ lệ cache hit.
- **Chuẩn hóa 7 Mẫu Workflow Định sẵn (Workflow Archetypes W0–W6):**
  - `W0: DIRECT` — Hỏi đáp trực tiếp, không dùng công cụ.
  - `W1: TOOL AGENT` — Vòng lặp ReAct đơn cho tác vụ tra cứu, chạy tool cục bộ.
  - `W2: PLAN & EXECUTE` — Planner sinh kế hoạch tuần tự cho Executor.
  - `W3: PARALLEL EXPERTS` — Chia nhỏ bài toán cho nhiều worker chuyên biệt chạy song song và tổng hợp kết quả qua Synthesizer.
  - `W4: CODE` — Quy trình lập trình khép kín: Analyze ➔ Plan ➔ Edit ➔ Test ➔ Review ➔ Commit.
  - `W5: RESEARCH` — Thu thập ngữ cảnh sâu, tổng hợp tài liệu đa nguồn.
  - `W6: HIGH CONFIDENCE` — Chạy song song nhiều giải pháp độc lập, tranh luận (Debate) và bỏ phiếu (Voting) để chọn kết quả tối ưu.

### 3.5. Đóng góp Khởi tạo Nền tảng CLI UI Subsystem
- Phối hợp chặt chẽ với Truong trên nhánh `custos-cli` (PR #15, Commit `f05f16d`):
  - Tham gia tích hợp bộ ảnh và frame UI cho linh vật Custos Snowy Owl.
  - Hỗ trợ kiểm thử hiển thị ban đầu trên terminal, đồng bộ hóa các định nghĩa trạng thái vòng đời tác vụ phục vụ cho các bản render tiến trình trực quan sau này.

---

## 4. Kết quả Bàn giao & Xác minh (Deliverables & Verification)

- **Tài liệu Thiết kế Kiến trúc Hoàn chỉnh:**  
  Bản đặc tả Custos hoàn thành với đầy đủ sơ đồ ASCII kiến trúc tổng thể, mô hình dữ liệu (Database Schema sơ bộ cho SQLite), giao diện TypeScript/Rust mẫu cho Provider Adapter, và phân tích luồng xử lý thực tế end-to-end.
- **Đồng bộ Nhánh & Tuân thủ Chuẩn Workspace:**
  - Nhánh `vinh` được đồng bộ sạch sẽ với `dev` qua PR #17 và PR #18, không phát sinh xung đột.
  - Kiểm tra `cargo test --workspace` trên môi trường local: Đạt 100% tests PASS, zero warning trên các crates hiện hữu.
  - Toàn bộ tài liệu tuân thủ quy chuẩn hệ thống, chuẩn hóa thuật ngữ tiếng Anh chuyên ngành cho các hợp đồng giao tiếp (Contracts).

---

## 5. Kế hoạch & Mục tiêu Tuần 02

1. **Khởi tạo & Cấu trúc Hóa `crates/workflow-runtime`:**
   - Xây dựng module `worker_lifecycle`: Định nghĩa formal state machine cho Worker (`Created`, `Leased`, `Active`, `WaitingTool`, `Paused`, `Completed`, `Failed`, `Cancelled`).
   - Thiết lập các kênh Tokio MPSC/Broadcast cho điều phối sự kiện tác tử nội bộ.
2. **Hiện thực hóa Hợp đồng Trao đổi Ngữ cảnh (`Structured Handoff Protocol`):**
   - Phối hợp cùng Vi chuẩn hóa struct `ContinuationPacket` và `HandoffEnvelope`.
   - Đảm bảo việc chuyển giao công việc giữa các worker không làm phình đại diện tích token (eliminate transcript pollution) và giữ nguyên vết bằng chứng xác thực.
3. **Hiện thực hóa Provider Adapter & Bộ lọc Định tuyến Kế thừa 9router (kèm RTK):**
   - Xây dựng trait `ProviderAdapter` trong Rust hỗ trợ streaming response.
   - Tích hợp pipeline RTK (Rust Token Killer) để thanh lọc và nén ngữ cảnh trước khi dispatch request, tối ưu chi phí token.
   - Hiện thực hóa bộ lọc 3 tầng (Tier 1 Constraints, Tier 2 Heuristic, Tier 3 Fallback) cho module Model Routing bên trong workflow runtime.
4. **Phối hợp Tích hợp Hệ thống cùng Truong & Vi:**
   - Kết nối worker task execution với `AuthorityEngine` và `CapabilityGateway` của Truong để đảm bảo an toàn sandbox.
   - Sẵn sàng tích hợp luồng điều phối tác tử vào bài toán Vertical Slice đầu tiên (`repo_explain`).
