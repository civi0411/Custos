# CUSTOS — BẢN PHÂN CÔNG NHIỆM VỤ & MA TRẬN TRÁCH NHIỆM CHUẨN (RACI)
## (Team Work Allocation & Responsibility-Driven Blueprint)

> **Mã tài liệu:** DEV-TEAM-02 (Revised)
> **Phân loại:** Quy ước vận hành bắt buộc & Hợp đồng phân quyền kỹ thuật  
> **Cơ sở kiến trúc tối cao:** Tuân thủ tuyệt đối [`Custos.md`](../Custos.md).  
> **Lưu ý Quan Trọng:** `Custos.md` mô tả **ĐÍCH ĐẾN** của kiến trúc (Aspirational Target), không phải hiện trạng codebase. Việc phân chia dưới đây là con đường đi đến đích, chia theo Trách nhiệm thay vì theo Crate vật lý.
> **Hội đồng phê duyệt:** Vĩ (Chief Architect & AI/Data Lead), Trường (Storage & OS Lead), Vinh (Runtime & Integration Lead)

---

## 1. Triết Lý Thiết Kế Trách Nhiệm (6 Nguyên Tắc Cốt Lõi)

Kiến trúc và phân công lao động của Custos được thiết kế lại dựa trên 6 nguyên tắc bất di bất dịch nhằm xóa bỏ rào cản phụ thuộc giữa các thành viên:

1. **Thiết kế theo Trách nhiệm (Responsibility-based), không theo Crate:** Code có thể được di chuyển, đổi tên hoặc tách crate khi cần, nhưng trách nhiệm cốt lõi (Ai làm AI, Ai làm Hệ thống) thì tuyệt đối không thay đổi.
2. **Hợp đồng đi trước (Contract-first):** Bắt buộc đóng băng **6 Ports (Rust Traits)** và **4 Schemas (JSON/Struct)** trước mọi nỗ lực implementation. Mọi người code theo contract, không ai phải chờ đợi ai.
3. **Cắt lát dọc (Vertical slice), không làm theo tầng ngang:** Mỗi giai đoạn (Gate) phải chạy end-to-end xuyên suốt các tầng. Không có khái niệm "Làm xong tầng 0 rồi mới lên tầng 1".
4. **Phát triển theo Cổng nghiệm thu (Gate-driven):** Lộ trình phát triển đi theo 8 Gates (từ A $\rightarrow$ OPT) đã định nghĩa trong `Custos.md`, không phải chạy theo Sprint vô định.
5. **Mock-first cho AI, InMemory cho SE:** Vĩ có trách nhiệm cung cấp `MockS1`/`MockS2` trả về JSON tĩnh. Trường cung cấp `InMemoryStorage`. Vinh cung cấp `MockSandbox`. Phát triển song song và độc lập tuyệt đối.
6. **Tự động hóa Documentation Triad:** Mọi thay đổi kiến trúc phải cập nhật đồng thời `Custos.md`, `docs/`, và `codebase-architecture.md`. CI sẽ tự động fail nếu vi phạm.

---

## 2. Phân Bổ Trọng Lượng & Sơ Đồ Kiến Trúc (40/30/30)

| Thành viên | Tỷ trọng | Lĩnh vực (Camp) | Triết lý |
|---|---|---|---|
| **VĨ (Chief Architect)** | **~40%** | **Cognitive, Contracts & Data** | Giữ vai trò quyết định kiến trúc, không nhất thiết code nhiều LOC hơn. Sở hữu hợp đồng 6 Ports, AI (S1/S2), Context Compiler, Evidence và Doc Triad. |
| **TRƯỜNG (SE 1)** | **~30%** | **Storage & OS Sandbox** | Hạ tầng hệ thống, SQLite WAL, CAS, Isolation Sandbox, và bảo chứng giao dịch T1–T5. |
| **VINH (SE 2)** | **~30%** | **Runtime, UI & Integration** | Điều phối vòng đời Task, Workflow loop, MCP client, CLI/SDK và các Harness adapters. |

```text
                                 KIẾN TRÚC TẦNG 11 CRATES VÀ QUYỀN SỞ HỮU
                                 (Responsibility-Based Allocation)
                                 
  [ TẦNG 5: CLIENT & UX ]       crates/custos-cli         crates/custos-sdk         ──► VINH
                                        │                         │
                                        ▼                         ▼
  [ TẦNG 4: COMPOSITION ROOT ]              crates/custos-daemon                    ──► VĨ
                                        │                         │
                     ┌──────────────────┴─────────────────────────┴──────────────────┐
                     ▼                                                               ▼
  [ TẦNG 3: RUNTIME & PACKS ]   crates/custos-bridge     crates/custos-runtime      crates/custos-packs
                                (VINH)                   (VINH)                     (VĨ)
                                     │                        │                          │
                     ┌───────────────┴────────────────────────┼──────────────────────────┘
                     ▼                                        ▼
  [ TẦNG 2: INFRA & ADAPTERS ]  crates/custos-persistence crates/custos-adapters   crates/custos-provider
                                (TRƯỜNG)                  (TRƯỜNG & VINH)          (VĨ)
                                     │                        │                          │
                     ┌───────────────┴────────────────────────┴──────────────────────────┘
                     ▼
  [ TẦNG 1: PURE KERNEL ]                    crates/custos-core                             ──► VĨ
                                                      │
                                                      ▼
  [ TẦNG 0: ZERO-I/O DOMAIN ]                crates/custos-domain                           ──► VĨ
```

*(Lưu ý: Context Compiler được Vĩ quản lý trực tiếp, dù nằm ở `custos-runtime` hay tách riêng, quyền kiểm soát logic vẫn thuộc về Vĩ).*

---

## 3. Ma Trận RACI Chi Tiết

**R (Responsible):** Người viết code, test và bảo trì.  
**A (Accountable):** Người chịu trách nhiệm duyệt kiến trúc cuối cùng (Chốt Contract).  
**C (Consulted):** Người cần được hỏi ý kiến trước khi đổi contract/schema.  
**I (Informed):** Người nhận thông báo sau khi thay đổi.

| Phân hệ / Crate | Nội dung | R | A | C |
|---|---|:---:|:---:|:---:|
| **6 Ports + 4 Schemas** | Ký kết hợp đồng Interface Freeze | **Vĩ** | **Vĩ** | Trường, Vinh |
| `custos-domain` | Zero-I/O Purity (Domain Event, Value Object) | **Vĩ** | **Vĩ** | Trường, Vinh |
| `custos-core` | Task FSM, Authority Engine, Budget Governor | **Vĩ** | **Vĩ** | Trường, Vinh |
| `custos-persistence` | SQLite WAL, CAS, Outbox, FTS5 | **Trường**| **Trường**| Vĩ, Vinh |
| `custos-provider` | Trait ModelProvider, Token Streaming, Cost | **Vĩ** | **Vĩ** | Vinh |
| `custos-adapters` (Sandbox)| Seatbelt (macOS), Bubblewrap (Linux) | **Trường**| **Trường**| Vĩ |
| `custos-adapters` (MCP) | MCP Client, Harness (Claude/Codex) | **Vinh** | **Vinh** | Vĩ |
| `custos-runtime` | Workflow loop, Lease, Cancel, Checkpoint | **Vinh** | **Vinh** | Vĩ |
| Context Compiler | 8-step pipeline, Semantic Cache | **Vĩ** | **Vĩ** | Vinh |
| Evidence Engine | Fact/Extraction/Semantic validation | **Vĩ** | **Vĩ** | Trường |
| `custos-packs` | Engineering, Research (FIRE), Assistant | **Vĩ** | **Vĩ** | Vinh |
| `custos-bridge` | Session-Task binding, Idempotency | **Vinh** | **Vinh** | Vĩ, Trường |
| `custos-daemon` | Cấu hình `bootstrap.rs`, ráp nối 6 Ports | **Vĩ** | **Vĩ** | Trường, Vinh |
| `custos-cli` & `custos-sdk`| IPC client, TUI, CLI output, Diff preview | **Vinh** | **Vinh** | Vĩ |
| **Doc Triad & Evals** | `Custos.md`, SWE-bench, Red-teaming | **Vĩ** | **Vĩ** | Trường, Vinh |
| **CI/CD & Chaos** | Crash testing T1-T5, `deny.toml` | **Trường**| **Trường**| Vĩ |

---

## 4. Thỏa Thuận "Đóng Băng" 6 Ports & 4 Schemas (Sprint 0)

Mọi hoạt động phát triển phải dừng lại cho đến khi 6 Ports và 4 Schemas dưới đây được viết bằng mã Rust và chốt hạ (Interface Freeze Agreement). Bất kỳ thay đổi nào sau Sprint 0 phải đi qua quy trình RFC (Request for Comments).

### 4.1. Sáu Ports (Rust Traits) Bắt Buộc

| # | Interface | Owner (A) | Reference (Custos.md) | Nơi Implement |
|---|---|---|---|---|
| 1 | **KernelPort** | Vĩ | §3.2, §4.2 | `custos-core` |
| 2 | **StoragePort** (`TaskRepo`, `CasStorage`, `OutboxPort`)| Trường | §3.10, §4.3 | `custos-persistence` |
| 3 | **SandboxPort** (`SandboxDriver`) | Trường | §8.4, §10.3 | `custos-adapters/sandbox` |
| 4 | **ModelPort** (`ModelProvider`) | Vĩ | §7.7, §14.4 | `custos-provider` + adapters |
| 5 | **WorkflowPort** (`WorkflowExecutionPort`) | Vinh | §3.3, §3.8 | `custos-runtime` |
| 6 | **MemoryPort** | Vĩ | §9.3 | `custos-core` + persistence |

### 4.2. Bốn Schemas Cốt Lõi (JSON Schema / Rust Structs)

| # | Schema | Owner | Nơi dùng & Mục đích |
|---|---|---|---|
| 1 | **ContextPack** | Vĩ | Output của Context Compiler truyền vào đầu vào của Model. |
| 2 | **ActionIntent + Permit + Receipt** | Vĩ | Luồng xác thực: Authority $\rightarrow$ Gateway $\rightarrow$ Evidence. |
| 3 | **EvidenceRecord** | Vĩ + Trường | Chứng nhận hoàn thành: Fact, Extraction, Semantic. |
| 4 | **ContinuationPacket** | Vĩ | Phục vụ Resume và Checkpointing tiến trình làm việc. |

---

## 5. Ranh Giới Giao Dịch Bền Vững (T1–T5 Transaction Boundaries)

Trường chịu trách nhiệm đảm bảo hệ thống phục hồi an toàn tuyệt đối nếu sập nguồn (SIGKILL) tại bất kỳ điểm nào.

- **T1: Task Inception:** (Trường) Lưu Task record + OutboxMessage trong 1 SQLite TX (fsync).
- **T2: Step Planning:** (Vĩ) Thu thập AST, tạo ActionIntent thuần RAM. Đảm bảo Budget.
- **T3: Authority Minting:** (Vĩ & Trường) Đúc ExecutionPermit. Ghi Audit log xuống đĩa.
- **T4: Sandboxed Execution:** (Trường) Cô lập tiến trình, lấy ExecutionReceipt (Hash files).
- **T5: Completion Gate:** (Vĩ & Trường) Vĩ thẩm định REAL. Trường commit TaskSucceeded + EvidenceRecord + giải phóng Budget trong 1 TX cuối.

---

## 6. Lộ Trình 8 Gates (Từ A đến OPT)
*(Tổng thời lượng dự kiến: 40–52 tuần / 10–12 tháng cho 3 người. Không có đường tắt.)*

| Gate | Đích đến End-to-End (E2E) | Vĩ (Cognitive/Data) | Trường (Storage/OS) | Vinh (Runtime/UI) | Thời gian |
|---|---|---|---|---|---|
| **A (Local E2E)** | CLI $\rightarrow$ Daemon $\rightarrow$ Local Model $\rightarrow$ `repo_explain` | Context Compiler minimal + S1/S2 stub + Evidence Fact | SQLite WAL + CAS + Task FSM + T1-T5 skeleton | CLI + Daemon + Local API + worktree read-only | 6-8 tuần |
| **B (Workspace)** | PatchBundle + Sandbox test | Semantic verifier + OI route cơ bản | Sandbox (Seatbelt/Bwrap) + crash test T1-T5 | Workflow loop + cancel + checkpoint | 6-8 tuần |
| **C (Crash)** | Bắn SIGKILL giữa T3-EXT $\rightarrow$ `Uncertain` | Evidence LTL + Reconciliation logic | Outbox durable + Reconciliation protocol | Recovery UI + audit T3 | 4 tuần |
| **D (Taint)** | 100% injection bị chặn đứng | Taint semantics + Context isolation + Secret redaction | — | MCP fail-closed + red-team fixtures | 4 tuần |
| **E (SWE-bench)**| SWE-bench Lite subset pass | OI routing đầy đủ + Engineering Pack | — | **1 harness adapter** (Claude Code/Codex) | 8-12 tuần |
| **F (Research)** | Tái lập 1 paper + Reproducibility | Research Pack + FIRE + DatasetCard | — | Experiment runner + Batch API | 6-8 tuần |
| **G (Memory)** | LongMemEval temporal consistency | Temporal facts + MemoryPort + ContinuationPacket| — | Memory runtime + FTS5 recall | 4 tuần |
| **OPT (Cost)** | `CostPerAcceptedTask` giảm $\ge$ 20%| OI economics + cache + cascade + compaction | — | Usage telemetry T2 | 4-6 tuần |

---

## 7. Quy Trình Vận Hành & Lời Thề Kỹ Thuật (Red Lines)

1. **Gate-driven, Không Sprint-driven:** Roadmap đi từ Gate A đến Gate OPT. Gate trước chưa hoàn hảo E2E thì tuyệt đối không sang Gate sau.
2. **MCP/A2A Fail-closed:** Nếu adapter chưa thỏa mãn Conformance Gate, client bắt buộc fail-closed. Không claim `custos-mediated` khi chưa có test thật.
3. **1 Harness Trước:** Vinh chỉ làm duy nhất 1 Harness Adapter (VD: Claude Code) để đi qua Gate E, tuyệt đối không làm 5 cái cùng lúc.
4. **Cam kết Review 24h & RFC:** Đổi hợp đồng phải có RFC. Review code trong 24h. Vĩ giữ quyền phủ quyết cuối cùng (Veto) dựa trên Custos.md.

---

## 8. Base Linkage & Khung Testbed Độc Lập Cho AI/Data Lead

Để Vĩ phát triển chuyên sâu phần AI mà không phụ thuộc vào hạ tầng SE:
- **Base Linkage:** Luồng dữ liệu 7 khớp nối kết nối CLI/UI $\rightarrow$ Kernel $\rightarrow$ Context Compiler $\rightarrow$ ModelPort $\rightarrow$ S1/S2 $\rightarrow$ Authority $\rightarrow$ Sandbox $\rightarrow$ Evidence $\rightarrow$ Storage.
- **In-Memory Harness (`CustosAiTestbed`):** Khung mock in-memory trong RAM cho phép Vĩ chạy TDD unit test và integration test cho Context Compiler, S1/S2 và Evidence Verifier độc lập với tiến độ SQLite/Sandbox.
- **Tài liệu tham chiếu chi tiết:** Xem `ARCHITECTURAL_BASE_LINKAGE_AND_AI_DEV_PLAN.md`.

