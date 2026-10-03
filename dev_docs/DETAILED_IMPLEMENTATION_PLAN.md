# CUSTOS DETAILED IMPLEMENTATION PLAN (GATE-DRIVEN)

> **Mã tài liệu:** DEV-PLAN-03 (Execution Spine First)  
> **Trạng thái:** Sẵn sàng thực thi (Actionable)  
> **Nguyên tắc cốt lõi:** Không xây "Siêu Agent OI" từ đầu. Bắt buộc hoàn thiện đường trục thực thi (Execution Spine) có thể quan sát, phục hồi được sau crash, sau đó mới gắn OI vào để ra quyết định dựa trên dữ liệu thật.

Tài liệu này chia nhỏ lộ trình kiến trúc đích thành 5 Pull Requests (PRs) đầu tiên. Mỗi PR có đầu ra kiểm chứng được (Measurable Gates) và ánh xạ trực tiếp tới các file cần sửa.

---

## PR 1: Execution Spine Đọc-Only (Gate 1 & 2)
**Mục tiêu:** Chứng minh một chu kỳ sống trọn vẹn: `Daemon -> Task -> start_run -> context scanner -> ModelPort -> Outcome` mà không có side-effects (ghi đĩa).

### Phân công & File cần sửa:
1. **Refactor Data Contracts (`custos-domain` - Vĩ)**
   - Mở file: `crates/custos-domain/src/task.rs`.
   - Cập nhật `TaskContract` thành `TaskContractRevision`.
   - Bổ sung struct `CriterionSpec` (gồm `criterion_id`, `pack`, `rubric`, `required_evidence`).
   - Cập nhật `Run`, `WorkerRun`, `NodeAttempt` có định danh rõ ràng.
2. **Refactor WorkflowPort (`custos-core` - Vĩ/Trường)**
   - Mở file: `crates/custos-core/src/contracts/workflow.rs`.
   - Đổi signature của `start_run` thành nhận `StartRunCommand` và trả về `RunHandle` (không trả về trực tiếp `WorkerRun`).
   - Bổ sung `request_cancel` (trả về `CancelReceipt`) và `resume` (trả về `RunHandle`).
3. **Implement TaskRuntime (`custos-runtime` - Vinh)**
   - Mở file: `crates/custos-runtime/src/workflow/task_runtime.rs`.
   - Viết ruột cho `start_run`: Đọc trạng thái từ Storage, khởi tạo `RunHandle`, kết nối tới `FakeProvider` (ModelPort).
4. **Viết End-to-End Test (Gate 2 Verification - Vinh/Trường)**
   - Mở file: `tests/testkit/tests/vertical_slice.rs`.
   - Viết test: Gửi lệnh qua Local API $\rightarrow$ Daemon gọi `WorkflowPort` $\rightarrow$ Fake Worker chạy đọc repo $\rightarrow$ Sinh Outcome $\rightarrow$ Test pass. *(Không dùng mock dispatcher)*.

---

## PR 2: Durable Effect Spine (Gate 3)
**Mục tiêu:** Bất kỳ external effect nào cũng phải thông qua cơ chế `Intent -> Exact Permit -> Durable Attempt (Outbox) -> Effect -> Receipt/Uncertain -> Evidence`. Chịu được `SIGKILL` tại mọi cửa sổ.

### Phân công & File cần sửa:
1. **Hoàn thiện Effect Schema (`custos-domain` - Vĩ)**
   - Cập nhật `EffectAttempt` state enum: `Pending`, `InFlight`, `Succeeded`, `Failed`, `Uncertain`.
   - Bổ sung `IdempotencyKey` vào `ActionIntent` và `ExecutionPermit`.
2. **Xây dựng Outbox & Ledger (`custos-persistence` - Trường)**
   - Thêm bảng SQLite để lưu `EffectAttempt`.
   - Đảm bảo logic: Write Intent $\rightarrow$ Commit SQLite $\rightarrow$ Call Adapter.
3. **Cơ chế Reconcile (Phục hồi sau Crash) (`custos-core` - Trường)**
   - Khi khởi động Daemon, quét các `EffectAttempt` đang `InFlight`.
   - Đánh dấu chúng là `Uncertain`. Gọi adapter kiểm tra trạng thái thực tế dựa trên `IdempotencyKey` trước khi cho phép Retry.
4. **Cập nhật Capability Gateway (`custos-adapters` - Vinh)**
   - Adapter Sandbox chỉ nhận đúng `ExecutionPermit` đã được ký bởi Kernel. Từ chối mọi payload bị can thiệp (Tampered payload).

---

## PR 3: Native Harness Adapter Đầu Tiên (Gate 4)
**Mục tiêu:** Chọn **1** native coding agent (VD: Claude Code hoặc Codex), bọc nó vào `AgentRuntimePort` với Profile năng lực trung thực, không giả lập model.

### Phân công & File cần sửa:
1. **Định nghĩa HarnessProfile (`custos-provider` - Vĩ)**
   - Mở file: `crates/custos-provider/src/port.rs`.
   - Định nghĩa `trait AgentRuntimePort`. Phân biệt nó với `ModelPort`.
   - Bổ sung `HarnessProfile` khai báo mức độ hỗ trợ: chặn tool thật không, có xài worktree riêng không, có hỗ trợ cancel không.
2. **Implement Claude Code / Codex Adapter (`custos-adapters` - Vinh)**
   - Viết sub-process wrapper quản lý vòng đời (start, attach, steer, cancel).
   - Bắt và chuẩn hóa (normalize) log event của agent thành dạng mà Custos có thể hiểu.
3. **Kiểm duyệt Native Bypass (Gate 4 Verification - Trường)**
   - Viết test thử nghiệm: Nếu agent tự gọi một bash command bằng native shell của nó (bypass Custos Sandbox), Custos phải dán nhãn `observe-only` hoặc `provider-governed` thay vì `custos-mediated`.

---

## PR 4: Graph Runtime & Scheduler (Gate 5)
**Mục tiêu:** Loại bỏ job scheduler hẹn giờ cũ. Đưa `WorkflowCompiler` và `ReadyFrontierScheduler` vào hoạt động.

### Phân công & File cần sửa:
1. **Xóa bỏ Scheduler cũ (`custos-runtime` - Vinh)**
   - Xóa bỏ hoặc thay thế cấu trúc `WorkflowScheduler` (xử lý job theo thời gian).
2. **Implement Ready-Frontier Scheduler (`custos-runtime` - Vinh)**
   - Chỉ trigger worker khi các Node phụ thuộc đã hoàn thành và nhả Artifact hợp lệ.
   - Quản lý `Lease Epoch` để tránh tranh chấp worker.
3. **Implement Workflow Compiler (`custos-runtime` - Vĩ/Vinh)**
   - Kiểm tra DAG: không có vòng lặp (cycle), không write conflict, tuân thủ privacy scope.
   - Đầu ra của Compiler là một `WorkflowRevision` có thể thực thi được.

---

## PR 5: OI D0 Decision Record (Gate 7)
**Mục tiêu:** Đưa não bộ OI sơ khai vào hoạt động. Không sinh topo phức tạp, chỉ làm một việc duy nhất: Trích xuất `DecisionSnapshot`, luôn đề xuất Native Baseline, và ghi `DecisionRecord` để đo lường.

### Phân công & File cần sửa:
1. **Trích xuất DecisionSnapshot (`custos-core` - Vĩ)**
   - Viết hàm thu thập thông tin: Budget còn lại, Pending Effects, Task revision hiện tại.
2. **Dumb OI Planner (`custos-runtime` - Vĩ)**
   - Viết logic nhận `DecisionSnapshot`, trả về `StrategyProposal` mặc định là dùng 1 Native Worker (Claude 3.5).
3. **Lưu trữ DecisionRecord (`custos-persistence` - Trường)**
   - Lưu trữ quyết định của OI vào Ledger để phục vụ quá trình Meta-Offline đánh giá sau này.
   - Phân tích chi phí overhead của Custos (Độ trễ S1, I/O ghi đĩa).

---
*Tiến độ: Tuân thủ quy tắc 4 Mắt (Four-Eyes Principle) và cập nhật Doc Triad trước khi merge mỗi PR.*
