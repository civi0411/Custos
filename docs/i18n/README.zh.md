<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../assets/banner-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="../assets/banner.png">
  <img alt="Custos" src="../assets/banner.png" width="100%">
</picture>

[ EN ](../../README.md) · [ VI ](README.vi.md) · [ DE ](README.de.md) · [ ZH ](README.zh.md) · [ JA ](README.ja.md) · [ KO ](README.ko.md) · [ ES ](README.es.md)

</div>

# Custos SADE

> **受监督智能体开发环境 (SADE) — 本地优先、实证绑定、人类主导。**

Custos 是一个面向自主智能体（Agent）的本地优先开发与执行环境，覆盖编程辅助、科学研究及个人工作流管理。与简单的 LLM 包装器或不透明的多智能体系统不同，Custos 强制执行严格的人类监督、确定性的状态持久化、经密码学审计的能力网关，并在任何任务被确认为完成前，必须提供客观的实证成果。

---

## 三大不可妥协的支柱

| 支柱 | 核心原则 | 架构保障 |
|---|---|---|
| **P1 — 任务作为一等公民** | 任务独立于交互会话和网络连接而持久存在 | 基于 SQLite WAL 的持久存储、确定性崩溃恢复以及防重放幂等命令账本。 |
| **P2 — 提议 ≠ 决策 ≠ 副作用** | 模型提议；内核许可；网关执行 | AI 模型仅发起 `ActionIntent`；内核签发单次使用的 `ExecutionPermit`；沙箱严格执行能力门禁。 |
| **P3 — 基于实证的完成判定** | 无客观证据则绝不认可完成 | 验收条件必须有独立的实体产物（Diff 补丁、测试运行追踪、引用审计）背书；杜绝一切虚假完成。 |

---

## 10 条不可逾越的系统不变量

1. **INV-01 (本地优先主权):** 所有的状态、契约、任务和遥测日志均驻留在本地存储介质中。
2. **INV-02 (零虚幻原子性):** 任何状态变更必须先提交至 SQLite，方可向外部调度分发动作。
3. **INV-03 (副作用前置许可):** 任何外部变更操作在没有有效的 `ExecutionPermit` 时严禁执行。
4. **INV-04 (执行者身份强绑定):** 操作者身份直接绑定自经过身份验证的传输层，绝不信任载荷自声明。
5. **INV-05 (实证重于模型断言):** 任务验收必须依赖经过验证的实体产物，严禁直接信任模型的完成自述。
6. **INV-06 (界面能力真实性):** 当后台服务缺失或降级时，前端界面必须立即进入关闭防御状态（`unavailable`/`degraded`）。
7. **INV-07 (工作树租约隔离):** 针对同一工作树（Worktree）的并发写操作实施强隔离栅栏，杜绝数据损坏。
8. **INV-08 (资源余量边界限制):** 所有进程执行、日志行缓冲和 Token 剪裁均严格遵循字节配额限制。
9. **INV-09 (全链路传输审计):** 所有系统变更均须经过命令账本并验证摘要哈希，严格防止重复执行。
10. **INV-10 (人类最高统治权):** 操作人员在任务生命周期的任何阶段，均保有无条件暂停、纠偏或中止任务的权力。

---

## 运行机制

```mermaid
flowchart LR
    Human[人类操作者] --> Session[交互会话 / 工作台]
    Session --> Task[持久化任务]
    Task --> Context[上下文组装]
    Context --> Worker[模型 / Harness 工作器]
    Worker --> Intent[动作提议 (Action Intent)]
    Intent --> Gate[权限网关与许可凭证]
    Gate --> Effect[沙箱副作用执行]
    Effect --> Evidence[客观实证产物]
    Evidence --> Outcome[经检验的完成成果]
```

---

## 系统架构：11 个规范的 Rust Crates

Custos 在 11 个分层的 Rust Crate 中严格隔离纯逻辑与外部副作用：

| 分层 | Crate | 核心职责 |
|---|---|---|
| **Layer 0 (Domain)** | `custos-domain` | 纯领域实体模型、状态机规范及零 I/O 系统不变量。 |
| **Layer 1 (Core)** | `custos-core` | 任务编排服务、权限引擎以及完成判定验证器。 |
| | `custos-persistence` | SQLite WAL 存储层、Outbox 模式、CAS 状态迁移及崩溃恢复。 |
| | `custos-provider` | 跨提供商模型契约、流式 SSE 解析器及 Token 计算模块。 |
| **Layer 2 (Runtime & Bridge)** | `custos-bridge` | 会话至任务晋升机制、RPC 路由以及本地 API 分发调度。 |
| | `custos-runtime` | 智能体认知循环、上下文无损压缩 (RTK)、记忆分层及执行规划器。 |
| | `custos-adapters` | 操作系统沙箱、MCP 客户端适配器、本地推理 Sidecar 及协议转换器。 |
| **Layer 3 (Domain Packs)** | `custos-packs` | 领域业务工作流包：工程开发包 (Engineering)、科研包 (Research) 与助手包 (Assistant)。 |
| **Layer 4 (Entry & Distribution)** | `custos-daemon` | 唯一的组合根（Composition Root）、后台守护进程及本地 HTTP/JSON-RPC 服务。 |
| | `custos-sdk` | 客户端数据传输对象 (DTO)、UniFFI 跨语言绑定及传输客户端。 |
| | `custos-cli` | 基于终端界面的操作工具 (`ratatui`)。 |

---

## 桌面用户界面 (Desktop UI)

Custos 桌面端由 **Vite、React 19 与 Tauri 2** 驱动：

- **三级层次结构:** 直观导航体系：`项目 -> 工作空间 -> 工作树 (Project -> Workspace -> Worktree)`。
- **Graphite JetBlack 设计系统:** 现代高对比度暗黑设计，专为长时间沉浸式开发深度优化。
- **专业工作台 (Workbenches):**
  - **工程工作台 (Engineering):** 文件与变更审查、实证查看器与工作树管理器。
  - **科研工作台 (Research):** 综合研究室 (Synthesis Studio)、科学发现看板及执行记录总账。
  - **助手与共享工作台 (Assistant & Shared):** 内置浏览器、终端、远程集群与便签笔记本。
- **零假数据政策 (Zero-Mock Policy):** 界面显示的所有指标均来自 Daemon 真实遥测，服务不可用时明确显示警告。

---

## 协议中心 (Hubs) 与外部工具套件集成

Custos 守护进程作为核心互操作网关，提供 6 大功能中心：

- **Model Hub:** 智能调度本地模型（GGUF、llama.cpp）与云端大模型（Anthropic、OpenAI、Gemini、Bedrock），支持无缝容灾切换。
- **Tool Hub:** 治理模型上下文协议 (MCP) 服务端与沙箱系统工具。
- **Agent Hub:** 协调智能体对智能体通信协议 (A2A) 与智能体交互协议 (ACP)。
- **Storage Hub:** 管理 SQLite WAL 事务存储、Blob 二进制存储及结构化索引缓存。
- **Event Hub:** 高吞吐分发审计追踪事件及全生命周期状态更新。
- **Operator Hub:** 为命令行工具及桌面界面提供经认证的本地端点。

### 外部编码套件适配器
Custos 在不破坏外部工具内部机制的前提下提供统一的监督沙箱：
- **Claude Code:** Stdio 子进程隔离与安全命令包装。
- **Codex:** 沙箱化代码补全与测试自动化执行。
- **Cursor:** 编辑器缓冲区实时同步与差分应用。
- **Antigravity:** 智能体编程运行时协调与任务一致性同步。

---

## 质量门禁与验证

所有提交必须通过严格的质量门禁：

```bash
# 1. 零警告 Clippy 阻断性门禁 (Rust)
cargo clippy --workspace --all-targets -- -D warnings

# 2. 全量测试套件 (>1,670 个测试，100% 通过)
cargo test --workspace --all-targets

# 3. 桌面端前端类型检查与打包构建
cd ui/desktop && npm run build
```

---

## 文档与参考链接

- **系统架构权威规范:** [Custos.md](../../Custos.md)
- **智能体与开发者准则:** [AGENTS.md](../../AGENTS.md)
- **桌面 UI 架构标准:** [.agents/rules/desktop_ui_architecture_standards.md](../../.agents/rules/desktop_ui_architecture_standards.md)
- **Git PR 工作流规范:** [.agents/rules/git_pr_workflow.md](../../.agents/rules/git_pr_workflow.md)
- **文档中心目录:** [docs/README.md](../README.md)

---

## 许可证

Custos 采用 **GNU AGPL-3.0-or-later** 许可证开源发布。包含严格的商标保护条款，禁止闭源的商业 SaaS 封装。详见 [LICENSE](../../LICENSE)。
