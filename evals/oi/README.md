# Đánh giá OI và economics của Custos SADE

**Evaluation design, chưa phải benchmark report hoặc executable suite hoàn chỉnh.** [Refactor plan §16](../../docs/development/workspace-restructuring-plan.md#16-evaluation-và-release-gate-không-tự-chứng-minh) khóa release methodology; [domain blueprints](../../docs/architecture/domain-packs-and-workflows.md) khóa semantics. Không dùng worktree làm clean-merge guarantee, coverage score làm support oracle hoặc một latency constant làm SLO.

## 1. Ma trận jobs và oracle

| Miền | Jobs tối thiểu | Baseline công bằng | Outcome cần grade |
|---|---|---|---|
| Coding | Repo explain, bugfix, multi-file refactor | Cùng pinned model/harness, repo/dirty snapshot, tools và verifier | Source anchors; behavior invariants/regression; scope/base hash; test-diff integrity |
| Research | Source QA, compare/brief, small AI/Data experiment | Cùng corpus/data/version/environment và grading rubric | Locator/extraction/support riêng; conflicts/parse gaps; protocol/seed/metrics và interpretation |
| Assistant | Draft, exact send, recurring occurrence | Cùng contacts/account/timezone/payload/effect policy | Đúng identity/payload/time; receipt hoặc truthful unknown; duplicate-send/privacy/revoke |
| Cross-pack | Selected claims → spec → patch → redacted draft | Separate chats/manual handoff và Custos single flow | Criterion lineage, redaction/consent, no implicit grant transfer, total human effort |

Không baseline luôn chỉ là Claude Code: chọn runtime phù hợp job, pin exact model/harness/config. Harness-switch experiment phải báo context/cache transfer cost; không quy mọi khác biệt cho OI.

## 2. Ablation protocol

1. Freeze fixtures, grading, privacy/effect rules, hardware/toolchain/provider versions và quality margin theo slice **trước** chạy.
2. Chạy paired strong-single baseline → selective retrieval/cache → S1 assistance → OI topology. Đo từng lever riêng và combination, không cộng phần trăm savings độc lập.
3. Giữ verifier/acceptance tương đương; bounded trials gồm failed/retried/abstained Tasks. Không loại task khó khỏi mẫu số.
4. Split native/mediated, local/cloud, cold/warm cache; ledger leaf attempts với billed/estimated/unknown riêng. Thiếu native usage là unknown, không `$0`.
5. Report uncertainty/confidence intervals. Chỉ claim cost improvement khi quality noninferiority và safety gates liên quan đạt; samples thiếu → inconclusive/opt-in.

## 3. Report contract

Report giữ dataset/task IDs, source revisions, policy/route/pricing versions, actual worker/model/harness, assurance profile, trial count/seed, verifier version và artifacts. Metrics: accepted count/rate, per-criterion false pass/unknown/stale, billed total/failure spend/cost per accepted Task, tokens/cache/tool/compute usage, p50/p95 useful-output/end-to-end latency, approval waiting và human minutes riêng.

Nếu accepted count bằng 0, cost per accepted Task **undefined**; vẫn công bố total spend và failure outcomes. Child/parent usage không double-count. Savings relative to baseline không là số có nghĩa nếu usage baseline hoặc candidate unknown.

## 4. Negative fixtures và trạng thái hiện tại

Cases bắt buộc theo path: stale source/base hash, agent weakening tests, symlink/write conflict, local-only cloud fallback, wrong recipient, payload edit after approval, send timeout after delivery, duplicate occurrence, unknown launch, concurrent budget overage, replay/late settlement và crash/restart. Fixture pass không chứng minh sandbox trên mọi OS.

Baseline mechanics đã chạy: `cargo test -p custos-tests-e2e --test oi_engine_slice --test execution_spine_slice --offline` — **5 passed**. Executor OI hiện có simulated outputs/fixed usage; đây không phải cost/quality result của live workers. Wave W0 phải bổ sung real-attempt instrumentation, W4/W5 mới mở coordination/optimization claims. Không sửa tests chỉ để biến design này thành green report.
