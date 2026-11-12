# Tài liệu Custos

Custos là local-first Agent Workspace & Execution Platform cho Coding, Research, Assistant và Quant. Bộ docs phân biệt **kiến trúc đích**, **cách xây**, và **hiện trạng có bằng chứng**; tài liệu không thay kết quả build/test.

Định vị sản phẩm đầy đủ: **Supervised Agent Development Environment (SADE)**. [SADE design/supervision](architecture/sade-design-and-supervision.md) giải thích bản sắc, research rationale và economics; Workspace là bề mặt tương tác của cùng sản phẩm.

## Bắt đầu đọc

1. [Custos.md](../Custos.md): product, invariants, flows và subsystem decisions.
2. [Agent Execution Environment (AEE)](architecture/agent-environment.md): mô hình môi trường thực thi của agent, honest assurance và 7 cổng bất biến `ENV-01..ENV-07`.
3. [Domain Ownership & Boundary Ledger](architecture/domain-ownership.md): ranh giới sở hữu kiểu miền và nguyên tắc Zero-Kernel Leakage.
4. [Workspace và UI](architecture/agent-workspace-and-ui.md): experience 4 bàn làm việc chuyên biệt, panes, worktrees, approvals và reconnect.
5. [Capabilities và skills](architecture/capability-catalog-and-skills.md): nơi đặt tính năng và dùng với model API/native agents.
6. [Kế hoạch tái cấu trúc SADE](development/workspace-restructuring-plan.md): source audit, cost core từ attempt đầu, module contracts, W0–W5, migration và acceptance gates.
7. [Catalog vật lý](development/codebase-architecture.md): tìm file hiện có; ghi chú gắn SHA là audit tại thời điểm đó.

## Theo chủ đề

- [Bốn domain workbench](architecture/domain-packs-and-workflows.md): jobs/artifacts/verification, scoped code reading, research literature/experiments, assistant outbox và quant backtest.
- [OI economics evaluation](../evals/oi/README.md): baseline, ablations, report fields và negative cases; design khác benchmark result.

- [Architecture](architecture/README.md): Task/kernel, AEE, authority/effects, evidence, context/memory, OI và protocol boundaries.
- [Development](development/README.md): code organization, delivery, testing và upstream reuse.
- [Reference](reference/README.md): vocabulary, naming, schema mapping và invariants.
- [Dev docs](development/legacy-team/README.md): execution packets/RFC và phân công; không thay master decisions.

Master giữ WHY/WHAT; topic docs giữ HOW; catalog giữ WHERE. AGENTS.md là quy tắc làm việc trên repo, không phải runtime authorization. Code/tests mô tả reality, không tự biến implementation violation thành target policy. Thay quyết định thì sửa đúng topic và master/catalog liên quan, không tạo thêm một bản “final” cạnh tranh.

Research/vendor sources chỉ chứng minh kỹ thuật hoặc feature theo version của họ. Hiệu năng, safety và savings của Custos phải đo riêng. Demo/mock/stub phải ghi rõ; unsupported/unknown là trạng thái hợp lệ.
