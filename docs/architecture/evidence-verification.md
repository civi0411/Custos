# Bằng Chứng & Xác Minh (Evidence & Verification Architecture)

> **Status:** Canonical Baseline v4.0  
> **Source:** Phần IV (§13-14) & Phần V (§24) Canonical Specification

Nguyên tắc cốt lõi của Custos là: **Không chấp nhận lời khẳng định chay của AI; mọi kết quả hoàn thành đều bắt buộc phải mang theo bằng chứng kiểm định khách quan.**

---

## 1. Định Nghĩa Hình Thức: Evidence-Carrying Action (ECA)

Một hành động mang theo bằng chứng $ECA$ được định nghĩa bằng bộ 6 thành phần hình thức:

$$ECA = \langle Intent, Proposal, Authority, Execution, Evidence, Verification angle$$

Trong đó:
- $Intent$: Ý định của con người hoặc mục tiêu của subtask.
- $Proposal$: Kế hoạch thực thi do worker đề xuất.
- $Authority$: Thẩm quyền hợp lệ thông qua `ExecutionPermit`.
- $Execution$: Quá trình chạy trong sandbox sinh ra artifacts và nhật ký.
- $Evidence$: Tập hợp các chứng cứ khách quan thu được từ môi trường thực tế.
- $Verification$: Kết quả đánh giá độc lập của bộ kiểm tra (*Verifier*) xác nhận $Evidence \models Intent$.

---

## 2. Sáu Tầng Bằng Chứng (The 6 Evidence Classes)

Custos phân loại bằng chứng thành 6 cấp độ từ thấp đến cao:

| Lớp | Tên lớp | Loại bằng chứng | Ví dụ thực tế |
|---|---|---|---|
| **L0** | **Static Evidence** | Kiểm tra cú pháp, định dạng tĩnh | AST parse sạch, Linter không báo lỗi, Schema JSON hợp lệ. |
| **L1** | **Execution Evidence** | Biên nhận chạy lệnh hệ thống | Exit code $= 0$, nhật ký stdout/stderr sạch, không timeout. |
| **L2** | **Deterministic Test** | Kiểm thử tự động độc lập | `cargo test` pass 100%, coverage đạt ngưỡng cam kết. |
| **L3** | **Environmental Evidence** | Kiểm chứng trạng thái môi trường | File hash trùng khớp, Git worktree sạch, dịch vụ trả về HTTP 200. |
| **L4** | **Human Attestation** | Xác nhận tường minh của người dùng | Con người xem trước diff và bấm chấp thuận bàn giao. |
| **L5** | **Cryptographic Proof** | Chứng chỉ số và bất biến mật mã | Chữ ký số Ed25519 của Kernel, Content Hash trên CAS. |

---

## 3. Gói Kết Quả Được Kiểm Minh (Verifiable Outcome Bundle)

Khi một Task hoàn tất, runtime không trả về một đoạn text đơn thuần mà đóng gói thành một **Verifiable Outcome Bundle** có cấu trúc chuẩn YAML/JSON:

```yaml
bundle_version: "custos.outcome.v1"
task_id: "tsk_01J8N6Z8K9M0P1Q2R3S4T5U6V7"
completed_at: "2026-09-21T21:30:00Z"

contract_fulfillment:
  intent: "Fix issue #102: JWT token expiration handling"
  status: "FULLY_SATISFIED"
  completion_gate_passed: true

artifacts:
  - path: "crates/auth/src/token.rs"
    content_hash: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    diff_stat: "+24 -6 lines"

evidence_ledger:
  - class: "L0_STATIC"
    tool: "cargo-clippy"
    exit_code: 0
    receipt_id: "rcpt_clippy_01"
  - class: "L2_DETERMINISTIC_TEST"
    tool: "cargo-test"
    tests_run: 32
    tests_passed: 32
    receipt_id: "rcpt_test_02"
    log_artifact_hash: "sha256:8f434346648f6b96df89dda901c5176b10a6d83961dd3c1ac88b59b2dc327aa4"

cost_accounting:
  total_tokens: 14250
  total_cost_usd: 0.0428
  wall_clock_seconds: 18.4

signatures:
  kernel_signature: "ed25519:3b9ac97d519b93821a..."
```
