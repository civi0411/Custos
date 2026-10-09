<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../assets/banner-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="../assets/banner.png">
  <img alt="Custos" src="../assets/banner.png" width="100%">
</picture>

[ EN ](../../README.md) · [ VI ](README.vi.md) · [ DE ](README.de.md) · [ ZH ](README.zh.md) · [ JA ](README.ja.md) · [ KO ](README.ko.md) · [ ES ](README.es.md)

</div>

**本地优先、由人类治理、具备可实证成果的智能体运行时环境。**

Custos 将瞬态的人工智能对话转化为受治理的持久化任务，具备明确意图、有界执行、可恢复状态以及基于客观实证的成果。它在本地运行，横跨本地模型、云端提供商和外部编程工具套件（Harness）。

---

## 核心动因

当前的智能体工具虽然功能强大，但普遍存在根本性的架构缺陷：

- **瞬态上下文丢失：** 上下文被困在孤立的厂商孤岛中，一旦更换会话或切换工具就会丢失。
- **未经实证的模型断言：** 模型关于任务完成的自我声明被直接信任，缺乏客观且独立的验证证据。
- **失控的外部副作用：** 外部变更（文件修改、网络请求、系统命令）在缺乏严格能力门禁和审计日志的情况下直接执行。
- **脆弱的执行状态：** 系统崩溃、速率限制和上下文溢出会中断进行中的工作，且无法进行确定性恢复。
- **人类主权的侵蚀：** 晦涩的多智能体结构削弱了人类监督，剥夺了对重大决策的审批控制权。

Custos 通过引入本地优先、与实证绑定的运行模型彻底解决这些问题：人类主权至上，所有智能体动作均受到严格审计。

---

## 基础概念

Custos 严格划分系统运行实体之间的界限：

- **会话 (Session)：** 瞬态交互通道（命令行、IDE 套件、桌面界面、对话窗口）。
- **任务 (Task)：** 包含意图、策略、状态与验收标准的不可变持久化单元。
- **运行尝试 (Run)：** 活跃任务生命周期内的一次独立执行尝试。
- **动作意图 (Action Intent)：** 由模型工作器提议的外部变更操作。
- **执行许可凭证 (Execution Permit)：** 由权限引擎签发的具备密码学绑定的单次使用授权凭证。
- **副作用 (Effect)：** 仅在持有有效许可凭证时在沙箱中执行的外部变更。
- **客观实证 (Evidence)：** 独立于模型自我声明、客观可验证的成果证明产物。

```mermaid
flowchart LR
    Human[人类主权者] --> Session[会话通道]
    Session --> Task[持久化任务]
    Task --> Context[上下文组装]
    Context --> Worker[模型 / Harness 工作器]
    Worker --> Intent[动作意图]
    Intent --> Gate[权限网关]
    Gate --> Effect[沙箱副作用]
    Effect --> Evidence[客观实证]
    Evidence --> Outcome[实证检验成果]
```

---

## 系统架构

Custos 运行于三个清晰隔离的信任区域（Trust Zones）：

1. **非信任区 (Untrusted Zone)：** 外部模型、第三方智能体工作器、远程网络端点以及未经校验的用户原始输入。
2. **受监督区 (Supervised Zone)：** 上下文编译、提示词合成、语义路由以及瞬态执行规划。
3. **可信内核 (Trusted Kernel)：** 基于 SQLite 预写日志（WAL）的持久化存储、权限引擎、能力网关以及完成度校验器。

### 工作区 Crate 组成

代码仓库由 11 个职责明确的 Rust Crate 构成：

| 分层 | Crate | 核心职责 |
|---|---|---|
| **领域层 (Domain)** | `custos-domain` | 纯领域标识、实体模型、状态机与系统不变式。 |
| **核心层 (Core)** | `custos-core` | 任务编排服务、权限引擎与完成度判定策略。 |
| | `custos-persistence` | SQLite 数据库迁移、持久化仓储与故障恢复。 |
| **运行时层 (Runtime)** | `custos-runtime` | 会话生命周期、工作器进程、认知系统与分层记忆。 |
| | `custos-provider` | 厂商中立的模型契约、流式传输协议与分词器。 |
| **桥接与适配层** | `custos-bridge` | 会话至任务的提升以及本地 API 分发。 |
| | `custos-adapters` | 模型、能力工具与执行沙箱的具体实现。 |
| **领域总成 (Packs)** | `custos-packs` | 专业工作流总成：工程总成、研究总成与助理总成。 |
| **守护进程与客户端** | `custos-daemon` | 唯一的生产组合根节点与后台守护进程服务。 |
| | `custos-sdk` | 客户端数据传输对象（DTO）与传输层抽象。 |
| | `custos-cli` | 面向操作员的轻量级命令行交互界面。 |

---

## 协议中枢与 Harness 兼容性

Custos 本地守护进程通过六个专用中枢（Hub）充当核心互操作网关：

- **Model Hub：** 在本地引擎与云端推理提供商之间路由请求，具备弹性回退能力。
- **Tool Hub：** 治理 Model Context Protocol (MCP) 服务端与外部执行工具。
- **Agent Hub：** 管理智能体间通信 (A2A) 与 Agent Communication Protocol (ACP) 工作流。
- **Storage Hub：** 协调 SQLite 持久化、二进制大对象（Blob）存储与结构化索引缓存。
- **Event Hub：** 分发高吞吐量审计日志与系统生命周期事件。
- **Operator Hub：** 为操作员前端与终端会话提供安全的本地端点。

### Harness 适配器

Custos 可以在不破坏安全边界的前提下无缝对接主流外部开发环境：

- **Claude Code：** 基于 stdio 的子进程隔离与命令封装。
- **Codex：** 沙箱化代码补全与测试用例执行。
- **Cursor：** 编辑器缓冲区同步与安全差异合并。
- **Antigravity：** 智能体编程运行时协调与任务状态同步。

---

## 领域专用总成

Custos 针对特定专业领域提供预置工作流总成：

- **工程总成 (Engineering Pack)：** 自主软件工程工作流，包含原子补丁总成、多工作器错误定位以及三路径执行工作区（标准仓、暂存分支、验证沙箱）。
- **研究总成 (Research Pack)：** 面向科学与技术探索的工作流，重点涵盖可复现性总成、迭代式事实检验 (FIRE)、数据集审计与实验追踪。
- **助理总成 (Assistant Pack)：** 个人工作流管理，支持多层级自主权、日程与通信治理以及荷载稳定性保障。

---

## 系统不变式

1. **本地优先主权：** 所有任务数据、状态流转、上下文产物与审计日志均完整保存在本地存储介质。
2. **零无凭证副作用：** 任何智能体或模型在未获取有效执行许可凭证前，严禁修改文件系统、执行系统进程或发起网络请求。
3. **实证先于断言：** 任务绝不可仅凭模型的自我声明即判定为完成；必须附带客观独立的实证依据。
4. **持久化状态事务：** 状态变更在外部操作派发前必须持久化落盘，杜绝异常崩溃造成的模糊未决状态。
5. **人类主权至上：** 人类操作员在任务生命周期的任何节点，均享有无条件暂停、审查、修改或终止任务的最高权限。

---

## 构建与测试验证

构建并验证工作区完整性：

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace --all-targets
```

---

## 文档与规范索引

- **系统架构规范：** [Custos.md](../../Custos.md) (单一事实来源 - SSOT)
- **智能体与协作准则：** [AGENTS.md](../../AGENTS.md)
- **文档导航目录：** [docs/README.md](../README.md)
- **多语言本地化索引：** [docs/i18n/README.md](README.md)

---

## 许可证

仓库根目录的 [LICENSE](../../LICENSE) 目前为 MIT 许可证。Goose 衍生代码的来源与署名要求正在[上游源码清单](../development/upstream-source-map.md)中核查；不能仅凭根目录许可证判断所有衍生文件的条款。
