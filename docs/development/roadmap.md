# Lộ Trình Phát Triển 16 Tuần (16-Week Delivery Roadmap)

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần XIV (§37-38) & Phần VII (§56) Canonical Specification

Lộ trình triển khai của Custos được thiết kế theo phương pháp **Vertical Slices (Lát cắt dọc)**: mỗi giai đoạn đều tạo ra một hệ thống có thể chạy được từ đầu đến cuối (*End-to-End*), không xây dựng các tầng trừu tượng nằm chờ.

---

## 1. Bốn Giai Đoạn Triển Khai (The 4 Phases)

### Phase 0 — Tuần 1–2: Architecture Runway
- Thiết lập Cargo Monorepo, CI với `cargo clippy`, `cargo test`, `cargo deny`.
- Định nghĩa các kiểu thực thể cốt lõi (`TaskId`, `TaskContract`, `ActionProposal`, `ExecutionPermit`, `VerificationReceipt`).
- Xây dựng Fake Provider và Fake Tool phục vụ kiểm thử.
- Viết 10 ADR đầu tiên (ADR-0001 đến ADR-0010).
- Xây dựng bộ test crash harness ban đầu.

### Phase 1 — Tuần 3–5: Durable Local Kernel
- Triển khai SQLite schema, migrations và WAL mode.
- Xây dựng Task & Step State Machine với Event Sourcing.
- Triển khai mô hình giao dịch atomic: Append Event + Projection + Outbox trong 1 transaction.
- Triển khai Content-Addressable Storage (CAS) cho artifacts.
- CLI tối thiểu: `custos run`, `custos status`, `custos pause`, `custos resume`.
- Vượt qua bài kiểm tra sống sót qua sự cố (*Kill/Restart Test Matrix*).

### Phase 2 — Tuần 6–8: Read-Only Coding & Repo Intelligence
- Tích hợp Git snapshot và cô lập môi trường.
- Tích hợp `ripgrep` và `tree-sitter` trích xuất symbols nhanh chóng.
- Xây dựng bộ biên soạn `ContextPack` có chấm điểm liên quan và kiểm soát token budget.
- Adapter ProviderPort đầu tiên (kết nối trực tiếp với OpenAI Codex hoặc Claude).
- Hoàn thành tính năng giải thích mã nguồn có trích dẫn (*Repo Explanation with Citation*).

### Phase 3 — Tuần 9–12: Controlled Mutation & Engineering Pack v1
- Triển khai `Capability Gateway` và sandboxing native (macOS Seatbelt / Linux bwrap).
- Tích hợp cơ chế cô lập Git Worktree cho từng Task/Run.
- Hiện thực hóa cơ chế phê duyệt chính xác (*Exact-Payload Human Approval*).
- Bộ kiểm tra độc lập (*Verifier Runner*) cho `cargo test`, `pytest`, `ruff`.
- Xuất gói `Verifiable Outcome Bundle` hoàn chỉnh.
- Adapter Provider thứ hai và cơ chế chuyển đổi an toàn qua `ContinuationPacket`.

### Phase 4 — Tuần 13–16: Cognitive Control Fabric (System One Beta)
- Triển khai `JudgmentPort` và Versioned Question Registry.
- Tích hợp Rules Engine và Local SLM (ONNX/llama.cpp) cho System One.
- Tích hợp adapter TypeSafe Jev ở chế độ shadow/advisory.
- Ba gói câu hỏi định chuẩn: Lọc ngữ cảnh, Phân loại rủi ro, Phản biện hoàn thành.
- Xuất báo cáo hiệu chuẩn (*Calibration Report*) đo lường tỉ lệ tiết kiệm chi phí so với baseline.

---

## 2. Ma Trận Nghiệm Thu 15 Tiêu Chí (Acceptance Matrix)

| # | Tiêu chí nghiệm thu | Chỉ số mục tiêu | Phương pháp kiểm tra |
|---|---|---|---|
| 1 | Sống sót sau `kill -9` | 100% Resume thành công | Crash Test Harness tự động |
| 2 | Sửa lỗi bug-fix thực tế | Đi trọn vẹn từ Goal -> Verified Patch | Chạy trên 20 bài tập SWE-bench |
| 3 | Thoát khỏi Gateway | 0 vi phạm (Zero bypass) | Sandbox audit logs & seccomp traps |
| 4 | Thu hồi phê duyệt | Vô hiệu hóa ngay khi sửa payload | Test Approval Invalidation |
| 5 | Bảo vệ nhánh Git chính | 0 thay đổi trên user working branch | Kiểm tra hash commit trên HEAD |
| 6 | Trích dẫn mã nguồn | 100% trích dẫn gắn snapshot commit | Code Citation Verification |
| 7 | Giới hạn ngân sách token | Không vượt trần cam kết ($< +1\%$) | Token accounting unit tests |
| 8 | Lỗi provider mạng | Task tự về trạng thái an toàn | Mock 429/500 chaos injection |
| 9 | Gói Outcome Bundle | Đầy đủ diff, hash, receipts, cost | Bundle Schema Validator |
| 10 | Phục hồi cơ sở dữ liệu | Restore thành công 100% sau crash | SQLite Online Backup Verification |
| 11 | Tiết kiệm độ trễ System One | Giảm $\ge 60\%$ latency phán đoán | Benchmark so với LLM Deliberation |
| 12 | Tiết kiệm chi phí API | Giảm $\ge 50\%$ token cho khâu routing | Shadow Evaluation Report |
| 13 | Thời gian khởi động CLI | $< 30	ext{ ms}$ | Benchmark `hyperfine custos --version` |
| 14 | Mức tiêu tốn RAM khi idle | $< 50	ext{ MB}$ | OS Process Memory Monitoring |
| 15 | Không rò rỉ Secrets | 0 khóa API xuất hiện trong logs/DB | Secret Scanning CI Gate |
