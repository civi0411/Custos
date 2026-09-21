# Architecture Decision Records (ADRs)

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần VI (§39.3) & Phần VII (§59) Canonical Specification

Hệ thống lưu trữ các quyết định kiến trúc của Custos tuân theo định dạng chuẩn [ADR (Architectural Decision Records)](https://adr.github.io/). Mỗi bản ghi phản ánh một quyết định trọng yếu, bối cảnh ra quyết định, các lựa chọn thay thế đã xem xét, và hệ quả kỹ thuật.

---

## Danh Mục Quyết Định Kiến Trúc (ADR Index)

| Mã ADR | Tiêu đề quyết định | Trạng thái | Lĩnh vực |
|---|---|---|---|
| **ADR-0001** | Product category and Task as core unit | Accepted | Product / Core |
| **ADR-0002** | Local daemon and trust boundary | Accepted | Architecture / Security |
| **ADR-0003** | SQLite event/outbox persistence | Accepted | Storage / Persistence |
| **ADR-0004** | Typed internal communication | Accepted | Communication |
| **ADR-0005** | MCP only at tool/resource boundaries | Accepted | Gateway / Tools |
| **ADR-0006** | ProviderPort and capability probes | Accepted | Provider / Interop |
| **ADR-0007** | Domain Packs and ephemeral workers | Accepted | Agent / Execution |
| **ADR-0008** | Worktree mutation isolation | Accepted | Execution / Git |
| **ADR-0009** | Exact-payload approvals | Accepted | Security / HITL |
| **ADR-0010** | ContextPack and provenance | Accepted | Context / Memory |
| **ADR-0011** | Evidence-based completion | Accepted | Verification |
| **ADR-0012** | Rust/TypeScript/Python split | Accepted | Codebase / Stack |
| **ADR-0013** | Cognitive Control Fabric | Accepted | Cognitive / Architecture |
| **ADR-0014** | RDC protocol (Request-Decision-Challenge) | Accepted | Cognitive / Protocols |
| **ADR-0015** | JudgmentPort vs DeliberationPort | Accepted | Cognitive / Interfaces |
| **ADR-0016** | Confidence is not authority | Accepted | Security / Principles |
| **ADR-0017** | Versioned Question Registry | Accepted | Cognitive / Evaluation |
| **ADR-0018** | Calibration profiles | Accepted | Cognitive / Tuning |
| **ADR-0019** | Jev egress/local fallback | Accepted | Cognitive / Adapters |
| **ADR-0020** | Reflexive challenge | Accepted | Cognitive / Safety |
| **ADR-0021** | Human Attention Packet | Accepted | UX / HITL |
| **ADR-0022** | Decision Ledger | Accepted | Audit / Persistence |
| **ADR-0023** | Memory promotion/invalidation | Accepted | Memory / Knowledge |
| **ADR-0024** | Artifact-addressed large payloads (CAS) | Accepted | Storage / Performance |
| **ADR-0025** | A2A deferred to federation horizon | Accepted | Architecture / Scope |
| **ADR-0026** | System One is a pluggable capability, not a vendor model | Accepted | Cognitive / Extensibility |
| **ADR-0027** | Custos-owned semantic core | Accepted | Architecture / Core |
| **ADR-0028** | OSS adoption levels and no-copy default | Accepted | Development / Governance |
| **ADR-0029** | Minimal SQLite workflow instead of external engine for MVP | Accepted | Storage / MVP |
| **ADR-0030** | Official MCP SDK without protocol mesh in MVP | Accepted | Gateway / MCP |
| **ADR-0031** | Provider sessions are not generic model calls | Accepted | Provider / Sessions |
| **ADR-0032** | Cedar assists authorization but does not own grants | Accepted | Security / Auth |
| **ADR-0033** | Derived vector/graph indexes are non-canonical | Accepted | Storage / Indexes |
| **ADR-0034** | Tiered sandbox backends (macOS Seatbelt / Linux bubblewrap) | Accepted | Security / Sandbox |
| **ADR-0035** | Shadow onboarding for judgment backends | Accepted | Cognitive / Testing |
| **ADR-0036** | VS Code first, desktop shell later | Accepted | UX / Clients |
| **ADR-0037** | Research/Personal packs after engineering gate | Accepted | Roadmap / Scope |

---

## Mẫu Cấu Trúc Bản Ghi Quyết Định (ADR Template)

Mỗi file ADR chi tiết khi được khởi tạo trong thư mục `docs/adr/ADR-xxxx.md` sẽ tuân theo khuôn mẫu:

```markdown
# ADR-xxxx: [Tiêu đề quyết định]

## Bối cảnh & Vấn đề (Context & Problem Statement)
Mô tả tình huống kỹ thuật và lý do cần đưa ra quyết định này.

## Quyết định đã chọn (Decision Outcome)
Quyết định cụ thể được chấp thuận và phương án kiến trúc được áp dụng.

## Các lựa chọn đã xem xét (Considered Options)
- Phương án A: [Ưu điểm / Nhược điểm]
- Phương án B: [Ưu điểm / Nhược điểm]

## Hệ quả tích cực & Tiêu cực (Consequences)
- Tích cực: Những lợi ích thu được.
- Tiêu cực / Rủi ro: Các gánh nặng kỹ thuật hoặc hạn chế phát sinh cần xử lý.

## Bằng chứng xác minh (Verification & References)
Các bài kiểm tra, PR hoặc benchmark kiểm chứng quyết định này.
```
