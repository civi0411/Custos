<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../assets/banner-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="../assets/banner.png">
  <img alt="Custos" src="../assets/banner.png" width="100%">
</picture>

[ EN ](../../README.md) · [ VI ](README.vi.md) · [ DE ](README.de.md) · [ ZH ](README.zh.md) · [ JA ](README.ja.md) · [ KO ](README.ko.md) · [ ES ](README.es.md)

</div>

> **受监督智能体开发环境 (SADE) — 本地优先、实证绑定、人类主导。**

Custos 是一个面向自主智能体（Agent）的本地优先开发与执行环境，覆盖软件工程、科学研究及个人工作流管理。与简单的 LLM 包装器或不透明的多智能体系统不同，Custos 强制执行确定性状态持久化、经密码学审计的能力网关、溯源上下文编译，并在任何任务被确认为完成前，必须提供客观的实证成果。

---

## 核心痛点与定位

当今的智能体系统虽然具备出色的生成能力，但在架构上存在根本性缺陷：

- **易失性上下文丢失：** 上下文被禁锢在不同厂商的封闭孤岛中，在工具切换或会话重启时丢失。
- **未经验证的完成断言：** 盲目信任模型的自我完成声明，缺乏客观验证或可复现的实证支持。
- **不受控的外部副作用：** 文件修改、网络请求、系统命令等操作在没有严格能力门禁和审计日志的情况下直接执行。
- **脆弱的状态管理：** 系统崩溃、速率限制或上下文溢出会导致进行中的任务完全损毁，缺乏确定性的恢复机制。
- **人类掌控力的削弱：** 不透明的多智能体黑盒循环削弱了人类的监督权，剥夺了关键节点的决策控制。

Custos 采用实证绑定的运行机制解决上述问题，保障人类的主权地位，并对智能体的所有外部行为进行严格监管。

---

## 三大不可妥协的支柱

| 支柱 | 核心原则 | 架构保障 |
|---|---|---|
| **P1 — 任务作为一等公民** | 任务独立于交互会话和网络连接而持久存在 | 基于 SQLite WAL 的持久存储、确定性崩溃恢复以及幂等命令账本。 |
| **P2 — 提议 ≠ 决策 ≠ 副作用** | 模型提议；内核许可；网关执行 | 不受信任的 AI 模型仅发起 `ActionIntent`；内核签发单次使用的 `ExecutionPermit`；沙箱通过 Outbox 严格执行能力门禁。 |
| **P3 — 基于实证的完成判定** | 无客观证据则绝不认可完成 | 验收条件必须有独立的客观实体产物（Diff 补丁、测试运行追踪、引用审计）背书；杜绝虚假完成。 |

---

## 运行机制

```mermaid
flowchart LR
    Human[人类操作员] --> Session[工作区会话]
    Session --> Task[持久化任务]
    Task --> Context[上下文编译]
    Context --> Worker[模型 / Harness Worker]
    Worker --> Intent[操作意图 ActionIntent]
    Intent --> Gate[权限门禁与许可]
    Gate --> Effect[沙箱内受控副作用]
    Effect --> Evidence[客观实证 Evidence]
    Evidence --> Outcome[经检验的完成态]
```

---

## 三大专业领域视图

Custos 为不同认知领域提供定制的交互模型与验证引擎：

### 1. 工程视图 (Engineering Lens - 编程工作台)
- **三路执行工作区 (3-Path Workspaces)：** 严格隔离标准仓库 (canonical repo)、暂存分支 (staging fork) 与验证沙箱。
- **原子补丁包 (Atomic Patch Bundles)：** 在暂存前对结构化 Diff 强制执行自动语法校验与 Lint 检查。
- **工作树租约隔离 (Worktree Lease Isolation)：** 对同一工作树的并发写操作施加栅栏保护，杜绝竞态条件与数据丢失。

### 2. 研究视图 (Research Lens - 科研工作台)
- **论断溯源层级 (Claim Grounding Hierarchy)：** 确立四层认知严密性体系：`L0 未溯源`、`L1 文献支撑`、`L2 同行评审`、`L3 实证复现`。
- **引用覆盖率仲裁机 (Citation Coverage Oracle)：** 严格对照已发表论文的 DOI/文献，拒绝模型的自言自语式完成。
- **综合演练室 (Synthesis Studio)：** 将多来源文献调研转化为可检验的假设与可复现的实验批次。

### 3. 助理视图 (Assistant Lens - 个人助理与通用工作台)
- **发件箱审批队列 (Outbox Approval Queue)：** 对外发送通信与高危外部修改强制暂存在持久的发件箱中等待审批。
- **常备授权规则 (Standing Grants)：** 支持对常规例行任务进行基于策略的委托，同时对高危操作保留人工升级通道。
- **全集成操作环境：** 统一安全接入沙箱浏览器、PTY 终端、远程服务器机群与持久化笔记。

---

## 10 条不可逾越的系统不变量

1. **INV-01 (本地优先主权):** 所有状态、契约、任务和遥测日志均驻留在本地存储中。
2. **INV-02 (零虚幻原子性):** 任何状态变更必须先提交至 SQLite，方可向外部调度分发动作。
3. **INV-03 (副作用前置许可):** 任何外部变更在没有密码学绑定的 `ExecutionPermit` 时严禁执行。
4. **INV-04 (执行者身份强绑定):** 操作者身份直接绑定自经过身份验证的传输层，绝不信任载荷自声明。
5. **INV-05 (实证重于模型断言):** 任务验收必须依赖经过验证的实体产物，严禁直接信任模型的完成自述。
6. **INV-06 (界面能力真实性):** 当后台服务缺失或降级时，前端界面必须立即进入关闭防御状态（`unavailable`/`degraded`）。
7. **INV-07 (工作树租约隔离):** 针对同一工作树的并发写操作实施强隔离栅栏，杜绝数据损坏。
8. **INV-08 (资源余量边界限制):** 所有进程执行、日志行缓冲和 Token 剪裁均严格遵循字节配额限制。
9. **INV-09 (全链路审计传输):** 所有外部变更均必须通过具备摘要校验与防重放机制的幂等 Command Ledger。
10. **INV-10 (人类最高统治权):** 人类操作员在任务生命周期的任何阶段均保留无限制的暂停、转向或终止权利。

---

## 工作区 Crates 与架构拓扑

Custos 严格划分纯业务领域逻辑与外部副作用，划分为以下分层 Cargo 包：

| 分层 | Crate 路径 | 包名称 | 核心职责 |
|---|---|---|---|
| **Layer 0 (Domain)** | `crates/custos-domain` | `custos-domain` | 纯领域实体、数据模型、状态机与零 I/O 不变量。 |
| **Layer 1 (Core)** | `crates/custos-core` | `custos-core` | 任务编排调度、权限仲裁引擎与完成态验证器。 |
| | `crates/custos-persistence` | `custos-persistence` | SQLite WAL 持久化、Outbox 模式、CAS `state_version` 状态流转与崩溃恢复。 |
| | `crates/custos-provider` | `custos-provider` | 厂商中立的 `ModelPort` 契约、流式 SSE 解析器与 Token 计量器。 |
| **Layer 2 (Runtime & Adapters)** | `crates/custos-bridge` | `custos-bridge` | 会话至任务晋升 (Session-to-Task)、运行中转向 (Steering) 与 IPC 路由。 |
| | `crates/custos-runtime` | `custos-runtime` | S1/S2 认知循环、上下文压缩 (RTK)、分层记忆系统与执行规划器。 |
| | `crates/custos-adapters` | `custos-adapters` | 操作系统沙箱、MCP 客户端适配器、本地推理 Sidecar、Git 工作树与协议转换器。 |
| **Layer 3 (Domain Packs)** | `crates/custos-packs` | `custos-packs` | 领域工作流：工程包 (Engineering)、科研包 (Research) 与助理包 (Assistant)。 |
| **Layer 4 (Composition & Hosts)** | `crates/custos-daemon` | `custos-daemon` | 唯一组装根节点 (Composition Root)、后台守护服务与本地 HTTP/JSON-RPC。 |
| | `crates/custos-sdk` | `custos-sdk` | 客户端数据传输对象 (DTO)、UniFFI 绑定与 ACP 传输类型。 |
| | `crates/custos-app/cli` | `custos-app-cli` | 基于 `ratatui` 构建的终端操作界面。 |
| | `crates/custos-app/desktop` | `custos-app-desktop` | 原生 Tauri 2 宿主外壳，集成 `ui/desktop` 前端。 |

---

## 桌面端用户界面

Custos Desktop 采用 **Vite、React 19 和 Tauri 2** 构建：

- **三级架构体系：** 清晰层级导航 `项目 -> 工作区 -> 工作树 (Project -> Workspace -> Worktree)`。
- **Graphite JetBlack 设计令牌：** 高对比度现代暗色调主题，专为高强度长时间作业优化。
- **专用工作台 (Workbenches)：**
  - **工程 (Engineering)：** 文件与变更、客观证据、工作树管理。
  - **科研 (Research)：** 综合演练室、科学发现、运行账本。
  - **助理与通用 (Assistant & Shared)：** 沙箱浏览器、终端、远程集群、持久笔记。
- **零 Mock 策略 (Zero-Mock Policy)：** 界面所有指标直连守护进程真实遥测，服务异常时安全降级。

---

## 协议中心 (Hubs) 与原生 Harness 监督

Custos 守护进程通过 6 大核心协议中心充当系统互联枢纽：

- **Model Hub：** 在本地推理引擎 (GGUF, llama.cpp) 与云端模型 (Anthropic, OpenAI, Gemini, Bedrock) 之间动态路由并自动容灾。
- **Tool Hub：** 统一治理 Model Context Protocol (MCP) 客户端与受沙箱保护的系统工具。
- **Agent Hub：** 协调智能体间通信协议 (A2A) 与智能体交互协议 (ACP)。
- **Storage Hub：** 统一管理 SQLite 持久层、Blob 大文件与结构化索引。
- **Event Hub：** 高吞吐分发系统审计遥测与生命周期事件。
- **Operator Hub：** 为 CLI 和桌面端提供经过身份认证的本地安全端点。

### 原生 Harness 协同监督
Custos 包装并监管主流编程 Harness，不侵入其内部核心逻辑：
- **Claude Code：** Stdio 子进程隔离与命令安全包装。
- **Codex：** 沙箱化代码补全与测试自动化执行。
- **Cursor：** 编辑器缓冲区同步与差异补丁安全应用。
- **Antigravity：** 智能体编程运行时协调与任务状态同步。

---

## 质量门禁与验证

所有提交的代码必须无条件通过强制质量门禁：

```bash
# 1. Rust Clippy 零警告零错误硬门禁
cargo clippy --workspace --all-targets -- -D warnings

# 2. 全量工作区测试套件 (>1,670 单元/集成测试，100% 通过)
cargo test --workspace --all-targets

# 3. 桌面端前端类型检查与打包构建
cd ui/desktop && npm run build
```

---

## 文档与参考

- **系统架构权威规范：** [Custos.md](../../Custos.md)
- **智能体与开发者准则：** [AGENTS.md](../../AGENTS.md)
- **桌面端 UI 设计规范：** [.agents/rules/desktop_ui_architecture_standards.md](../../.agents/rules/desktop_ui_architecture_standards.md)
- **Git PR 工作流规范：** [.agents/rules/git_pr_workflow.md](../../.agents/rules/git_pr_workflow.md)
- **国际化文档总览：** [docs/i18n/README.md](README.md)

---

## 开源许可

Custos 基于 **GNU AGPL-3.0-or-later** 许可证开源分发。许可证包含严格的商标条款以保护 Custos 品牌，并明令禁止将其封装为闭源商业 SaaS 服务。详情参阅 [LICENSE](../../LICENSE)。
