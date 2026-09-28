# CUSTOS — BẢN ĐẶC TẢ KIẾN TRÚC ĐÍCH TOÀN DIỆN (HYBRID AGENT WORKBENCH)

> **Authority notice (2026-09-28):** This Vietnamese document is preserved as
> a non-normative research and implementation companion. It is not an agent
> instruction file, an accepted shared contract, or implementation evidence.
> Current authority is defined by [`docs/README.md`](../README.md), with
> [`ARCH-REF-01`](reference-architecture.md) as the target architecture and
> [`docs/status/`](../status/README.md) as dated implementation evidence. When
> this document conflicts with those sources, the active authority wins.

**Mã tài liệu:** ARCH-MASTER-2026
**Trạng thái:** Historical research and implementation companion; non-normative
**Ngày phê duyệt:** 2026-09-27
**Phân quyền dự án:**
- **Lead / AI & Data Core Architect:** Vĩ (Lead)
- **Platform & Core Domain SE:** Trương
- **Runtime, Security & Infrastructure SE:** Vinh

---

## TỔNG QUAN CHIẾN LƯỢC: CUSTOS — PROOF-CARRYING AGENT WORKBENCH

Custos là môi trường làm việc tác tử cục bộ (Local-First Proof-Carrying Agent Workbench). Trong đó, **CUSTOS là trung tâm kiến trúc tối cao**, tích hợp lai tạo (Hybrid) với Goose để kế thừa sức mạnh thực thi, đồng thời sử dụng bộ công cụ hỗ trợ Nexus nội bộ để tối ưu hóa việc phân tích mã nguồn:

1. **CUSTOS — KHUNG XƯƠNG SỐNG BỀN VỮNG & BẢO CHỨNG (DURABLE & PROOF-CARRYING KERNEL)**:
   - **Task Kernel**: Coi Task (không phải Session) là đơn vị công việc chính thức, có Epoch, State Machine và Revision.
   - **Authority Gateway & OS Sandbox**: Giam giữ mọi tác vụ can thiệp hệ thống vào macOS Seatbelt hoặc Linux Bubblewrap, cấp phép qua `ExecutionPermit`.
   - **Evidence Mesh**: Nghiệm thu công việc dựa trên bằng chứng vật lý (`CitationVerifier`, `CommandExitCodeVerifier`, SHA-256 artifacts).
   - **Durable State**: Lưu trữ nguyên tử trên SQLite WAL, đảm bảo khôi phục 100% không mất mát dữ liệu khi sập nguồn hoặc restart (`kill -9`).

2. **GOOSE — ĐẦU MÁY THỰC THI & GIAO DIỆN LAI TẠO (HYBRID EXECUTION & ACP SURFACE)**:
   - Cung cấp Agent Loop đa bước đã được kiểm chứng qua hàng vạn giờ sử dụng (`custos-agent/src/machine.rs`).
   - Cung cấp giao thức ACP (Agent Client Protocol) giúp kết nối mượt mà với Goose Desktop, CLI, và VSCode Extension.
   - Cung cấp cơ chế phân giải streaming model (OpenAI, Anthropic, Databricks, Ollama) và hệ sinh thái công cụ MCP.

3. **NEXUS — CÔNG CỤ HỖ TRỢ NỘI BỘ (INTERNAL REPO INTELLIGENCE UTILITY)**:
   - Là bộ công cụ phân tích AST và Call Graph nội bộ (`tools/repo_intelligent`) được build để **hỗ trợ AI và dev hiểu sâu từng ngóc ngách của repo mà không tốn token**.
   - Cung cấp dữ liệu chỉ mục cho `custos-context` để đóng gói ContextPack siêu nhẹ.

---

# PHẦN I: NĂM TRỤ CỘT BẤT BIẾN & TÁM ĐIỀU LUẬN KỸ THUẬT (INVARIANTS)

## 1. Năm Trụ Cột Bất Biến (The 5 Pillars)

* **Trụ cột 1 — Task là đơn vị công việc (Task-Centric Work Unit)**:
  Mọi cam kết thay đổi hệ thống, sửa code, chạy thử nghiệm phải thuộc về một `Task`. Session chỉ là kênh giao tiếp. Task sống độc lập, bền vững qua crash, restart, và thay đổi model.
* **Trụ cột 2 — Proof phải đi kèm kết quả (Proof-Carrying Outcome)**:
  Không bao giờ tin lời model tự xưng "Tôi đã làm xong". Kết quả chỉ được công nhận khi có `EvidenceBundle` chứa các bằng chứng kiểm tra được (Exit code = 0, Dòng trích dẫn tồn tại thực, Hash khớp).
* **Trụ cột 3 — Authority phải được hòa giải (Mediated Authority & Sandboxing)**:
  Tuyệt đối không có lệnh hệ thống nào được chạy trực tiếp trên shell của host. Mọi hành động (`ActionIntent`) phải qua `GatewayTool`, được cấp `ExecutionPermit` và chạy cách ly trong Sandbox.
* **Trụ cột 4 — Durable Execution là mặc định (Durable by Default)**:
  Mọi bước chuyển trạng thái đều được ghi nhật ký (Event Journal) vào SQLite WAL trước khi phát tác vụ tiếp theo. Khởi động lại sau sự cố phải tiếp tục chạy từ checkpoint, không lặp lại tool đã xong.
* **Trụ cột 5 — Con người nắm quyền tối thượng (Human Sovereignty & Trust Precedence)**:
  Thứ bậc tin cậy: `Chính sách con người (Human Policy) > Hợp đồng Task (Contract) > Cấu hình tin cậy > Đề xuất của Model > Dữ liệu bên ngoài`. AI không bao giờ được tự cấp thêm quyền.

---

## 2. Tám Điều Luận Kỹ Thuật (Core Invariants I1 – I8)

* **Invariant I1 (Task Transition Validity)**:
  Chỉ cho phép các bước chuyển trạng thái hợp lệ:
  `Draft ➔ Queued ➔ Running ➔ Blocked | Succeeded | Failed | Cancelled` và `Blocked ➔ Running | Cancelled`. Mọi chuyển đổi sai luật bị chặn lập tức.
* **Invariant I2 (Evidence Binding)**:
  Mọi `Evidence` phải gắn với một `CriterionId` cụ thể trong Contract. Mọi tiêu chí bắt buộc (`required: true`) phải có `VerificationClaim` đạt trạng thái `Pass`.
* **Invariant I3 (Effect Authorization)**:
  Mọi effect ra đĩa/mạng phải có `Permit` hợp lệ tại thời điểm dispatch. `permit.payload_hash == action.payload_hash`.
* **Invariant I4 (Unknown Handling)**:
  Trạng thái `Unknown` KHÔNG PHẢI là `Failed`, cũng KHÔNG PHẢI là `Succeeded`. Khi gặp `Unknown`, cấm tự động retry mù quáng; phải hòa giải (reconcile) hoặc hỏi con người.
* **Invariant I5 (Single Writer Rule)**:
  Chỉ duy nhất `custos-kernel` có quyền ghi vào `TaskState`. Chỉ duy nhất `custos-session` có quyền ghi vào `SessionState`. Chỉ duy nhất `custos-security/evidence` có quyền ghi `EvidenceState`.
* **Invariant I6 (Trust Precedence)**:
  Tầng dưới không thể ghi đè quy tắc của tầng trên. Model không thể tự nới lỏng ngân sách token hay hạ chuẩn tiêu chí nghiệm thu.
* **Invariant I7 (Delegation Narrowing)**:
  Quyền hạn của tác vụ con (sub-task) luôn là tập con (`⊆`) quyền hạn của tác vụ cha.
* **Invariant I8 (Secret Isolation)**:
  Khóa bí mật (API Keys, Token) chỉ lưu trong OS Keychain; tuyệt đối không bao giờ xuất hiện trong Prompt gửi ra ngoài, không lọt vào Log và không lọt vào Artifacts.

---

# PHẦN II: KIẾN TRÚC PHÂN LỚP TOÀN DIỆN & BẢN ĐỒ 46 CRATES

Hệ thống được tổ chức thành 6 tầng vật lý sạch sẽ theo chuẩn Lục Giác (Hexagonal Architecture):

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. APP & CLIENT LAYER (Cổng giao tiếp & Ứng dụng)                           │
│    • custos-cli              • custos-daemon          • custos-local-api   │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. RUNTIME ORCHESTRATION LAYER (Động cơ điều phối & Nhận thức)              │
│    • custos-session          • custos-workflow        • custos-cognitive   │
│    • custos-context          • custos-security        • custos-agent       │
│    • custos-context-management                        • custos-memory      │
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. CORE DOMAIN & PORTS LAYER (Trái tim nghiệp vụ & Khế ước)                 │
│    • custos-domain (Pure)    • custos-kernel          • custos-bridge      │
│    • custos-provider-sdk     • custos-provider-types  • custos-sdk-types   │
├─────────────────────────────────────────────────────────────────────────────┤
│ 4. INFRASTRUCTURE & PERSISTENCE LAYER (Lưu trữ bền vững & Quan sát)         │
│    • custos-persistence (SQLite WAL, Repositories, Migrations)              │
│    • custos-observability (OpenTelemetry Tracing, Metrics)                 │
├─────────────────────────────────────────────────────────────────────────────┤
│ 5. ADAPTERS & DRIVERS LAYER (Môi trường bên ngoài & Sandbox)                │
│    • custos-adapters-mcp     • custos-mcp             • custos-providers   │
│    • sandboxes/macos-seatbelt                         • sandboxes/linux-bwrap
│    • judgments/contracts     • judgments/rules        • judgments/onnx     │
│    • custos-download-manager                                                │
├─────────────────────────────────────────────────────────────────────────────┤
│ 6. DOMAIN PACKS LAYER (Gói nghiệp vụ chuyên ngành)                          │
│    • domain-pack-sdk         • custos-packs-engineering                     │
│    • custos-packs-research   • custos-packs-assistant • evals/              │
└─────────────────────────────────────────────────────────────────────────────┘
                                      ▲
                                      │ (Đọc trực tiếp SQLite AST 0.1ms)
┌─────────────────────────────────────┴───────────────────────────────────────┐
│ 7. SUBSYSTEM: REPO INTELLIGENCE (BỘ CÔNG CỤ HỖ TRỢ NEXUS LENS)              │
│    • tools/repo_intelligent (Tree-sitter AST, Call Graph, Semantic Index)   │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

# PHẦN III: RANH GIỚI KẾ THỪA GOOSE (ADR-COMPAT-01: GOOSE HYBRID BOUNDARY)

Để 2 bạn SE (Trường & Vinh) và Lead không dẫm chân vào mã rác hoặc hiểu sai triết lý, đây là quy chuẩn phân loại di sản Goose:

| Phân loại | Thành phần cụ thể | Hướng xử lý trong Custos |
|---|---|---|
| **VÀNG MƯỜI (Tái sử dụng 100%)** | `custos-agent/src/machine.rs` (Loop, cancellation, turn limit)<br>`custos-providers/` (SSE, stream chunk parser)<br>`custos-adapters-mcp/` (Stdio transport framing) | **Bảo tồn**: Giữ nguyên sức mạnh thực thi, tích hợp vào Daemon theo mô hình **Yield Mode**. |
| **ĐỘC DƯỢC (Cấm dùng, Phải thay thế)** | - Coi Session là nguồn sự thật duy nhất<br>- Chạy raw shell trực tiếp không cô lập<br>- Model tự xưng "I'm done"<br>- Nhồi nhét full transcript làm memory | **Thay thế hoàn toàn bằng Custos**: Bắt buộc dùng `custos-kernel`, bọc qua `GatewayTool` + `Seatbelt Sandbox`, nghiệm thu qua `CompletionGate` + `EvidenceBundle`. |
| **RÁC / MÃ DƯ THỪA (Cách ly tuyệt đối)** | `crates/runtime/custos-engine/` (~125 tệp mã nguồn fork cũ từ Goose desktop/UI) | **Đóng băng**: Cấm bật crate này vào Cargo workspace. Không sửa, không refactor. Chỉ bóc tách riêng lẻ khi thực sự cần. |
| **TƯƠNG THÍCH HAI CHIỀU (Dual-Read)** | Cấu hình `GOOSE_*`, đường dẫn `.goose` | **Tương thích ngược**: Custos đọc `CUSTOS_*` trước, nếu không có mới fallback đọc `GOOSE_*`. Ghi mới vào `~/.custos`. |

---

# PHẦN IV: NĂM LUỒNG HOẠT ĐỘNG CƠ SỞ (THE 5 BASE FLOWS)

### Luồng 1: Fast Path (Interactive Vibe / Chat)
- **Mục đích**: Hỏi đáp, khám phá nhanh, phản hồi <1s.
- **Thực thi**: Người dùng gửi prompt ➔ `custos-session` ghi nhận vào `SessionJournal` ➔ Tính `promotion_score` ➔ Gọi `custos-context` lấy tóm tắt nhẹ ➔ S1 Fast Model trả lời stream.
- **Quy tắc**: Không sinh Task, không cấp Lease, không khóa tài nguyên.

### Luồng 2: Promotion & Bridge Transaction (Thăng Cấp An Toàn)
- **Mục đích**: Chuyển cuộc trò chuyện thành công việc có bảo chứng khi xuất hiện yêu cầu sửa code hoặc thao tác phức tạp (`promotion_score > 0.8`).
- **Thực thi**: `BridgeService::promote(session_id, contract)` ➔ Gọi `TaskService::execute_create` ➔ Tạo Task ở trạng thái `Draft` (Epoch 0) ➔ Lưu `SessionTaskBinding(session_id, task_id)` vào SQLite.
- **Quy tắc V2**: **Session của Goose VẪN GIỮ TRẠNG THÁI ACTIVE**, người dùng tiếp tục chat bình thường trong khi Task chạy ngầm.

### Luồng 3: Durable Task Execution Spine (Xương Sống Tự Hành)
- **Bước 1 (Context Compilation)**: `custos-context` mở trực tiếp `nexus_ast.db` của Nexus đọc AST & Call Graph (độ trễ 0.1ms), đóng gói `ContextPack` (<4k tokens).
- **Bước 2 (Cognitive Routing)**: `custos-cognitive` định tuyến: Tier 0 (AST deterministic, 0 token) ➔ S1 (Fast) ➔ S2 (Deliberate Planning). Sinh `ActionProposal`.
- **Bước 3 (Authority & Sandbox)**: `GatewayTool` chặn hành động, kiểm tra quyền, cấp `ExecutionPermit` (scoped path) ➔ Chạy trong Sandbox (Seatbelt macOS / Bubblewrap Linux) ➔ Nhận `ToolResult`.
- **Bước 4 (Evidence Gathering & Journaling)**: Đưa kết quả qua `EvidencePipeline` ➔ Chạy Verifiers (`Citation`, `ExitCode`, `ExactMatch`) ➔ Lưu `Span` và `ContinuationPacket` vào SQLite WAL.

### Luồng 4: Acceptance & Completion Gate (Nghiệm Thu Chứng Thực)
- **Mục đích**: Khóa chặt cửa ra, chỉ cho phép Task hoàn thành khi có bằng chứng thật.
- **Thực thi**: `TaskService::execute_complete` ➔ `CompletionGate::can_complete(&task, &claims)`.
- **Quy tắc**: 100% các tiêu chí bắt buộc trong hợp đồng (`required criteria`) phải có ít nhất 1 `VerificationClaim` đạt `Pass`. Nếu thiếu hoặc fail ➔ Chuyển Task sang `Waiting` hoặc yêu cầu sửa lỗi.

### Luồng 5: Crash Recovery & Lossless Resume (Phục Hồi Sự Cố)
- **Mục đích**: Khôi phục nguyên vẹn trạng thái khi máy bị tắt đột ngột (`kill -9`).
- **Thực thi**: Khi daemon khởi động lại ➔ Quét SQLite tìm Task `Running` có Lease hết hạn ➔ `TaskReducer::replay(events)` tái lập chính xác `TaskState` ➔ Đọc `ContinuationPacket` gần nhất ➔ Tiếp tục chạy từ bước dở dang mà **không chạy lại các lệnh tool đã hoàn thành trước đó**.

---

# PHẦN V: MA TRẬN PHÂN BỔ 2 TRỤC TÁC CHIẾN (SE SPINE VS AI/DATA CORE)

Để đội ngũ 3 người vận hành với tốc độ cao nhất mà không bị block lẫn nhau:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ TRỤC 1: TRƯỜNG & VINH (SE TEAM — PLATFORM & RUNTIME SPINE)                  │
│ • Trường: Lõi Nghiệp Vụ, Máy Trạng Thái, Event Reducer, Completion Gate     │
│   (custos-domain, custos-kernel, custos-session, custos-bridge, workflow)   │
│ • Vinh: Hạ Tầng Bền Vững, Cổng Thẩm Quyền, Sandbox OS, Daemon & CLI         │
│   (custos-persistence, custos-security/authority, sandboxes/, daemon, cli)  │
├─────────────────────────────────────────────────────────────────────────────┤
│ TRỤC 2: LEAD / VĨ (AI & DATA CORE ARCHITECT)                                │
│ • Lõi Tri Thức Repo: Subsystem Nexus (Tree-sitter AST, Call Graph SQLite)   │
│ • Ngữ Cảnh Token: custos-context (Direct In-Process Reader, ContextPack)    │
│ • Động Cơ Nhận Thức: custos-cognitive (Tier 0 / S1 / S2 Router, BudgetGuard)│
│ • Lưới Thẩm Định: custos-security/evidence (5 Verifiers Pipeline)           │
│ • Tiêu Chuẩn Nghiệp Vụ: domain-packs, prompt blueprints, evals/             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

# PHẦN VI: KẾ HOẠCH TRIỂN KHAI 4 GIAI ĐOẠN (MASTER ACTION PLAN)

| Mã | Hạng mục công việc | Người phụ trách | Crate mục tiêu | Tiêu chuẩn hoàn thành (DoD) |
|---|---|---|---|---|
| **#01** | **Khóa Chặt Frozen Core & 8 Invariants** | **Lead (Bạn)** | `custos-domain`<br>`custos-kernel` | Structs & 8 Invariants viết bằng Rust, `cargo check` PASS 100%. |
| **#02** | **Session SQLite & Non-kill Bridge** | **Trường & Vinh** | `custos-session`<br>`custos-bridge` | Session lưu SQLite; Promote xong session vẫn `Active` chat tiếp được. |
| **#03** | **CompletionGate nối EvidenceClaims** | **Trường** | `custos-kernel` | Không có bằng chứng Pass ➔ Chặn đứng lệnh CompleteTask. |
| **#04** | **ExecutionPermit & Sandbox macOS/Linux** | **Vinh** | `custos-adapters-mcp`<br>`sandboxes/` | Xóa substring check; tool chạy bị giam trong Seatbelt/Bubblewrap. |
| **#05** | **Daemon Background Worker Loop** | **Trường & Vinh** | `custos-daemon` | Daemon tự động kéo Task từ Queued sang Running và thực thi. |
| **#06** | **Nexus Direct In-Process SQLite Reader** | **Lead (Bạn)** | `custos-context`<br>`tools/repo_intelligent` | Đọc bảng AST/Graph từ `nexus_ast.db` với độ trễ 0.1ms, 0 token lãng phí. |
| **#07** | **Token-Aware Context Compiler** | **Lead (Bạn)** | `custos-context` | Nén và đóng gói ContextPack chuẩn mực dưới 4k tokens. |
| **#08** | **Cognitive Tiering Hub (Tier 0/S1/S2)** | **Lead (Bạn)** | `custos-cognitive` | Định tuyến linh hoạt: 40% Tier 0, 45% S1, 15% S2; Failsafe switching. |
| **#09** | **Evidence Mesh (Bộ 5 Verifiers)** | **Lead (Bạn)** | `custos-security` | Đăng ký đủ Citation, ExitCode, ExactMatch, Hash, Semantic Evaluator. |
| **#10** | **E2E Integration & Chaos Crash Test** | **Cả đội (All)** | `tests/e2e/` | Chạy xanh `repo_explain_slice` và pass test `kill -9` khôi phục dữ liệu. |

---

# PHẦN VII: BỘ KHUNG CAM KẾT & TIÊU CHUẨN XONG VIỆC (DEFINITION OF DONE)

Một tính năng chỉ được coi là HOÀN THÀNH khi thỏa mãn đồng thời 4 điều kiện:
1. **Compiles Clean**: `cargo check --workspace` không có lỗi.
2. **Contract Conformance**: Không phá vỡ bất kỳ Invariant nào trong 8 điều luận (I1–I8).
3. **Automated Evidence**: Có Unit test hoặc Contract test chứng minh hành vi.
4. **Integration Green**: Bài test tích hợp luồng [repo_explain_slice.rs](file:///Users/mac/Project/AgentHub/Custos/tests/e2e/tests/repo_explain_slice.rs) chạy xanh từ đầu đến cuối.

> *"Custos là môi trường vận hành công việc có AI hỗ trợ — không phải chatbot, không phải agent framework thông thường. Bản đặc tả này là bộ luật bất biến. Chúng ta xây dựng trên đó, mở rộng trên đó, nhưng không bao giờ phá vỡ nền móng này."*
