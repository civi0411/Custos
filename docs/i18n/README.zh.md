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

## 四大专业领域工作台 (Vertical Workbenches)

Custos 为不同认知领域提供定制的交互模型、领域模型与验证引擎：

### 1. 编程工作台 (Coding Workbench - `workbenches/coding`)
- **三路执行工作区 (3-Path Workspaces)：** 严格隔离标准仓库 (canonical repo)、暂存分支 (staging fork) 与验证沙箱。
- **原子补丁包 (Atomic Patch Bundles)：** 在暂存前对结构化 Diff 强制执行自动语法校验与 Lint 检查。
- **工作树租约隔离 (Worktree Lease Isolation)：** 对同一工作树的并发写操作施加栅栏保护，杜绝竞态条件与数据丢失 (`ENV-02`)。

### 2. 科研工作台 (Research Workbench - `workbenches/research`)
- **论断溯源层级 (Claim Grounding Hierarchy)：** 确立四层认知严密性体系：`L0 未溯源`、`L1 文献支撑`、`L2 同行评审`、`L3 实证复现`。
- **引用覆盖率仲裁机 (Citation Coverage Oracle)：** 严格对照已发表论文的 DOI/文献，拒绝模型的自言自语式完成。
- **综合演练室与 Notebook 内核：** 将多来源文献调研转化为可检验的假设与可复现的实验批次，并与密码学 `EnvSnapshot` 锚定 (`ENV-04`)。

### 3. 助理工作台 (Assistant Workbench - `workbenches/assistant`)
- **发件箱审批队列 (Outbox Approval Queue)：** 对外发送通信与高危外部修改强制暂存在持久的发件箱中等待审批。
- **常备授权规则 (Standing Grants)：** 支持对常规例行任务进行基于策略的委托，同时对高危操作保留人工升级通道。
- **逻辑作用域锚定：** 直接锚定在项目的逻辑作用域上，不伪造任何多余的 Git 检出。

### 4. 量化工作台 (Quant Workbench - `workbenches/quant`)
- **市场数据确定性分区：** 将策略与确定性的历史数据分区锚定，彻底杜绝未来函数 (Look-ahead bias)。
- **物理离线沙箱回测 (Air-Gap)：** 回测期间强制切断网络连接 (`network_restricted = true`)，严防策略泄露或意外实盘报单 (`ENV-05`)。
- **Alpha 策略实证检验：** 在策略晋升前自动化评估夏普比率、索提诺比率及最大回撤指标。

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

## 智能体执行环境 (AEE) 与 7 大不变量门禁

自主智能体绝不允许在空虚、未锚定或不可审计的环境中运行 (`sade-foundations.md`)。Custos 形式化定义了 **智能体执行环境 (Agent Execution Environment - AEE)** 聚合，跨越纯领域模型 (`EnvironmentSpec`, `EnvironmentProfile`, `EnvironmentState`, `EnvSnapshot`, `EnvironmentAssurance`)、核心端口 (`EnvironmentPort`) 及运行时协调器。

在任何智能体任务修改状态前，系统将评估 7 大确定性不变量门禁：

| 不变量门禁 | 核心安全保证 | 失败防御机制 (Fail-Closed) |
|---|---|---|
| **ENV-01 (就绪态执行)** | 环境必须处于 `Ready` 就绪态（或已审计降级状态）。 | 初始化中、失败或已销毁状态一律拒绝状态修改。 |
| **ENV-02 (编程隔离树)** | Coding 视图严格要求隔离的 Git 工作树及明确分支。 | 共享未提交的脏工作树在未经人工豁免前拒绝执行。 |
| **ENV-03 (诚实隔离保证)** | 目标隔离保证绝不能超过宿主机实测具备的防护力 (Gate 4)。 | 宿主机仅提供 `ObserveOnly` 时严禁声称 `CustosMediated`，自动降级。 |
| **ENV-04 (科研语料绑定)** | Research 视图严格要求有效的本地文献语料目录及重现 Snapshot。 | 语料目录未锚定时拒绝运行 Notebook 单元代码。 |
| **ENV-05 (量化完全离线)** | 量化策略回测必须完全断网隔离 (`network_restricted = true`)。 | 检测到任何活动的外部网络 Socket 时立即终止执行。 |
| **ENV-06 (幂等资源回收)** | 环境注销操作可幂等重复执行；服务启动自动回收孤儿进程与 PTY。 | 重启时自动扫描并清理僵尸进程与无主工作树。 |
| **ENV-07 (透明降级原因)** | 任何向 `Degraded` 状态的迁移必须包含人类可审计的原因字符串。 | 拒绝匿名或未记录理由的功能降级。 |

---

## 工作区架构与分层拓扑

Custos 遵循六边形架构 (Hexagonal Architecture)，严格划分纯业务领域逻辑与底层物理副作用：

| 架构子系统 | 物理目录路径 | 核心职责 |
|---|---|---|
| **业务领域层 (Domain)** | `platform/domain` | 纯领域聚合标识、零 I/O 模型、AEE 特征规约及纯状态机。 |
| **核心契约层 (Core)** | `platform/core` | 任务调度服务、权限引擎、`EnvironmentPort` 契约及验收核验器。 |
| **持久存储层 (Storage)** | `platform/storage` | SQLite WAL 持久化、Outbox 模式、CAS `state_version` 状态流转与崩溃恢复。 |
| **人工智能层 (AI)** | `platform/ai` | 厂商中立的 `ModelPort` 契约、模型目录、流式 SSE 解析器与 Token 计量。 |
| **认知运行时 (Runtime)** | `platform/runtime` | S1/S2 认知循环、`EnvironmentCoordinator`、工作区生命周期及终端会话路由。 |
| **适配与沙箱层 (Adapters)** | `platform/adapters` | 操作系统沙箱 (Seatbelt/Bubblewrap)、MCP 客户端、Git 工作树与 PTY 终端。 |
| **组件与工作流 (Components)** | `platform/components` | 动态 Pack 注册中心、基础技能、能力描述符及按视图披露工具编译器。 |
| **助理工作台** | `workbenches/assistant` | 个人工作流助理、日历联系人、待办草稿及发件箱人工审批。 |
| **编程工作台** | `workbenches/coding` | 软件工程、三路工作区、结构化原子 Diff 及工作树租约保护。 |
| **科研工作台** | `workbenches/research` | 文献综合研判、四级溯源矩阵、ALCE 引用仲裁及 Notebook 计算。 |
| **量化工作台** | `workbenches/quant` | 量化策略开发、完全离线回测、Alpha 信号检验与行情分区管理。 |
| **宿主守护进程 (Daemon)** | `apps/daemon` | 唯一组装根节点 (Composition Root)、后台常驻服务与本地 JSON-RPC/HTTP API 调度。 |
| **客户端 SDK** | `sdk/client-rust` | Rust 客户端 SDK、本地 API 通信传输层与 ACP 交互结构体。 |
| **桌面端界面** | `ui/desktop` | 原生 Tauri 2 + Vite + React 19 桌面交互程序。 |
| **终端命令行** | `ui/cli` | 交互式终端命令行操作工具。 |

---

## 桌面端用户界面

Custos Desktop 采用 **Vite、React 19 和 Tauri 2** 构建：

- **三级架构体系：** 清晰层级导航 `项目 -> 工作区 -> 工作树 (Project -> Workspace -> Worktree)`。
- **Graphite JetBlack 设计令牌：** 高对比度现代暗色调主题，专为高强度长时间作业优化。
- **四大专用工作台 (Workbenches)：**
  - **编程 (Coding)：** 文件与变更、客观证据、工作树管理。
  - **科研 (Research)：** 综合演练室、科学发现、运行账本。
  - **助理 (Assistant)：** 沙箱浏览器、终端、远程集群、持久笔记。
  - **量化 (Quant)：** 策略分析图表、回测模拟器、Alpha 策略工作室。
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

所有提交的代码必须通过 `xtask` 自动化执行的 5 大强制质量门禁：

```bash
# 运行全部 5 大质量门禁 (Packs 契约, 桌面 UI 构建, Canvas Smoke 烟雾测试, 架构不变量, Cargo Check)
cargo run -p xtask -- gate

# 检查架构层级边界，严防 Kernel 与 Workbench 之间的逆向泄露
cargo run -p xtask -- check-boundaries

# 全量工作区 Clippy 静态检查 (0 警告容忍)
cargo clippy --workspace --all-targets -- -D warnings
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
