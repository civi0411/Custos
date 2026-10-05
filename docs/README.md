# Tài liệu Custos

Custos là local-first Agent Workspace cho Coding, Research và Assistant. Bộ docs phân biệt **kiến trúc đích**, **cách xây**, và **hiện trạng có bằng chứng**; tài liệu không thay kết quả build/test.

Định vị sản phẩm đầy đủ: **Supervised Agent Development Environment (SADE)**. [SADE design/supervision](architecture/sade-design-and-supervision.md) giải thích bản sắc, research rationale và economics; Workspace là bề mặt tương tác của cùng sản phẩm.

## Bắt đầu đọc

1. [Custos.md](../Custos.md): product, invariants, flows và subsystem decisions.
2. [Workspace và UI](architecture/agent-workspace-and-ui.md): experience ba miền, panes, worktrees, comparison, approvals và reconnect.
3. [Capabilities và skills](architecture/capability-catalog-and-skills.md): nơi đặt tính năng và dùng với model API/native agents.
4. [Kế hoạch tái cấu trúc SADE](development/workspace-restructuring-plan.md): source audit, cost core từ attempt đầu, module contracts, W0–W5, migration và acceptance gates.
5. [Catalog vật lý](development/codebase-architecture.md): tìm file hiện có; ghi chú gắn SHA là audit tại thời điểm đó.

## Theo chủ đề

- [Ba domain pack](architecture/domain-packs-and-workflows.md): jobs/artifacts/verification, Repo Intelligence, research AI/Data experiments và assistant effects/automation.
- [OI economics evaluation](../evals/oi/README.md): baseline, ablations, report fields và negative cases; design khác benchmark result.

- [Architecture](architecture/README.md): Task/kernel, authority/effects, evidence, context/memory, OI và protocol boundaries.
- [Development](development/README.md): code organization, delivery, testing và upstream reuse.
- [Reference](reference/README.md): vocabulary, naming, schema mapping và invariants.
- [Dev docs](../dev_docs/README.md): execution packets/RFC và phân công; không thay master decisions.

Master giữ WHY/WHAT; topic docs giữ HOW; catalog giữ WHERE. AGENTS.md là quy tắc làm việc trên repo, không phải runtime authorization. Code/tests mô tả reality, không tự biến implementation violation thành target policy. Thay quyết định thì sửa đúng topic và master/catalog liên quan, không tạo thêm một bản “final” cạnh tranh.

Research/vendor sources chỉ chứng minh kỹ thuật hoặc feature theo version của họ. Hiệu năng, safety và savings của Custos phải đo riêng. Demo/mock/stub phải ghi rõ; unsupported/unknown là trạng thái hợp lệ.
