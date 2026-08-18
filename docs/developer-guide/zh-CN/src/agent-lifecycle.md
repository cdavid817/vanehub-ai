# Agent 生命周期与 provider 运行时

本章覆盖运行时如何在不引入应用层 provider 身份分支的前提下,将一个稳定的 Agent id 解析为具体的 provider 契约,以及 Agent 从注册表条目到可被启动所经过的解析、能力声明与可用性评估。

## 稳定的 provider 解析

Agent 运行时通过一个 **provider registry** 来解析受支持的内置 CLI 运行时行为,该 registry 以 Agent registry 条目的稳定 id 为键。与 provider 无关的应用与 Session 模块不会根据 provider 身份分支来选择行为。一个没有兼容 provider 注册的 Agent id 会返回一个分类好的 `unsupported-provider` 错误,且不会回退到其他 provider。

## Provider 元数据与能力

每个注册的 provider 各自声明经过校验的元数据、就绪前提与受支持的运行时能力(interaction、resume、structured-output、terminal、usage、permission、model-selection、reasoning),独立于显示名称匹配或调用方推断。provider 未声明的能力不会被静默假设为存在。

## 从注册表到启动的流程

一个 Agent 从注册表条目到被启动,要经过起源分类、运行时解析、能力声明与可用性评估若干阶段。下图展示从注册表条目到启动的主干流程。

```mermaid
flowchart TD
    A["注册表中的 Agent 条目"] --> B{"AgentOrigin"}
    B -- "Builtin 内置" --> C["目录内置条目"]
    B -- "User" --> D["用户条目"]
    C --> E{"AgentRuntimeKind / LaunchKind"}
    D --> E
    E -- "NativeDesktop" --> F["原生桌面运行时<br/>例:OnePiece"]
    E -- "Cli" --> G["外部 CLI 运行时<br/>例:五个 CLI"]
    F --> H["稳定 provider 解析<br/>按稳定 id,不按显示名"]
    G --> H
    H --> I{"provider 注册存在?"}
    I -- "否" --> J["unsupported-provider 错误<br/>不回退其他 provider"]
    I -- "是" --> K["能力声明<br/>capability_tags + 元数据"]
    K --> L["可用性评估 AvailabilityAssessment"]
    L --> M{"AgentAvailability"}
    M -- "Available" --> N["可选 / 就绪"]
    M -- "NeedsAuthentication" --> O["不可选,需认证"]
    M -- "Unavailable" --> P["不可选,附原因"]
    M -- "Unknown" --> Q["未声明,状态未知"]
    N --> R["工作流选择 / 启动"]
```

### 起源与运行时形态

- **稳定的 agent id** —— 每个 Agent 以一个持久 id 标识,在该 Agent 参与的所有会话中保持不变。id 是 provider 解析、Loop 定义(Worker/Verifier id)等所有引用的键,而非显示名。
- **`AgentOrigin`** —— 内置 Agent(`Builtin`)由目录支持;`User` 标记的 Agent 是注册表中的用户条目。
- **`AgentRuntimeKind` / `LaunchKind`** —— 区分运行时形态:`NativeDesktop`(原生桌面运行时)与 `Cli`(外部 CLI 运行时),以及其他形态。

### provider 解析与能力

- **provider 解析稳定性** —— 运行时通过以 Agent registry 条目稳定 id 为键的 **provider registry** 解析受支持的内置 CLI 运行时行为,与 provider 无关的应用与 Session 模块不会按 provider 身份分支选择行为。
- **无回退** —— 一个没有兼容 provider 注册的 Agent id 返回分类好的 `unsupported-provider` 错误,且不回退到其他 provider。
- **能力声明** —— 每个注册 provider 各自声明经过校验的元数据、就绪前提与受支持的运行时能力(interaction、resume、structured-output、terminal、usage、permission、model-selection、reasoning),独立于显示名称匹配或调用方推断。provider 未声明的能力不会被静默假设为存在。

### 可用性状态与选择

`AgentAvailability` 由 `AvailabilityAssessment::assess()` 综合托管 SDK 依赖状态(`ManagedSdkStatus`)与可执行文件状态(`ExecutableStatus`)得出。

| 状态 | 含义 | 可选性 |
| --- | --- | --- |
| `Available` | 托管 SDK(若需要)已安装且可执行文件在 PATH | 可选,可进入会话 |
| `NeedsAuthentication` | 需要补充认证 | 不可选 |
| `Unavailable` | 托管 SDK 缺失/未识别或可执行文件不在 PATH,附原因 | 不可选 |
| `Unknown` | 未声明可执行文件 | 状态未知 |

`ensure_selectable()` / `ensure_session_selectable()` 在选择前做两道闸:先查 `AgentAvailability`,不可用直接拒绝并带原因;再查该 Agent 是否声明了目标 `InteractionMode`,不支持则拒绝。

### 内置 Agent

- **OnePiece** —— `builtin` + `NativeDesktop`,稳定 id `onepiece`;使用由目录支持的专用 provider **Profile** 操作,允许配置多个独立受保护的 provider/endpoint/model 组合与一个显式活跃 Profile。provider、endpoint 类型、interface format 与 Base URL 都从所选内置目录条目解析。
- **五个 CLI** —— `claude-code`、`codex-cli`、`gemini-cli`、`opencode`、`antigravity-cli` 均为 `builtin` + `Cli`,由内置 provider registry 解析运行时行为。

## 关键类型与常量

下表汇总 Agent 生命周期与 provider 运行时的核心类型、常量与错误码,供实现时快速查阅。权威语义仍以本节前文与规范为准。

### 起源与运行时形态

- `AgentOrigin` 枚举 —— `Builtin`(由内置目录支持)与 `User`(注册表中的用户条目)。
- `AgentRuntimeKind` / `LaunchKind` —— `NativeDesktop`(原生桌面运行时,如 OnePiece)与 `Cli`(外部 CLI 运行时,如五个 CLI)。

### 稳定 agent id

每个 Agent 以一个持久 id 标识,在该 Agent 参与的所有会话中保持不变。id 是 provider 解析、Loop 定义(Worker/Verifier id)等所有引用的键,而非显示名。

### provider 解析稳定性

运行时通过以 Agent registry 条目稳定 id 为键的 **provider registry** 解析受支持的内置 CLI 运行时行为。与 provider 无关的应用与 Session 模块不会按 provider 身份分支选择行为。

- **无回退** —— 没有兼容 provider 注册的 Agent id 返回分类好的 `unsupported-provider` 错误,且不回退到其他 provider。

### 能力声明

每个注册 provider 各自声明经过校验的元数据、就绪前提与 `capability_tags`,外加受支持的运行时能力:

- `interaction`、`resume`、`structured-output`、`terminal`、`usage`、`permission`、`model-selection`、`reasoning`。

provider 未声明的能力不会被静默假设为存在。

### 可用性状态

`AgentAvailability` 由 `AvailabilityAssessment::assess()` 综合托管 SDK 依赖状态(`ManagedSdkStatus`)与可执行文件状态(`ExecutableStatus`)得出,四状态:

- `Available` —— 可选,可进入会话
- `NeedsAuthentication` —— 不可选,需补充认证
- `Unavailable` —— 不可选,附原因
- `Unknown` —— 未声明可执行文件,状态未知

### 选择闸

`ensure_selectable()` / `ensure_session_selectable()` 在选择前做两道闸:

1. 先查 `AgentAvailability`,不可用直接拒绝并带原因
2. 再查该 Agent 是否声明了目标 `InteractionMode`,不支持则拒绝

## 单 Agent 会话的运行时形态

单 Agent 会话里,选定的 Agent 按 `AgentRuntimeKind` 走不同的运行时路径:

| 维度 | 内置 CLI Agent(`Cli`) | OnePiece 原生 Agent(`NativeDesktop`) |
| --- | --- | --- |
| 进程 | VaneHub 启动并管理 CLI 子进程(经 Agent Terminal / PTY),真正的代码生成由 CLI 完成 | 不启动外部进程,直接在应用内通过 HTTP 调用 active Profile 配置的 provider |
| 认证 | 由各 CLI 自行管理,VaneHub 不保存其凭据 | API Key 由 VaneHub 保存(Profile scoped 凭据) |
| Skill | 经统一 Skill 体系,覆盖层治理后注入 | 经 `AgentSkillPort` 消费生效视图:eager 注入 system prompt,on-demand 经固定只读工具加载 |
| MCP | Claude Code/Codex CLI 走中继;其余各自配置 | native 工具目录直接纳入可见且 active 的 MCP 工具 |
| 可观测性 | CLI 内部黑盒,链路只到边界 | 原生保真度,工具调用可逐层展开 |

CLI Agent 的 PTY 与启动详见[终端与 PTY 运行时](terminal-runtime.md);OnePiece 的 provider 调用、上下文组装与工具循环详见[OnePiece native Agent](onepiece-native-agent.md)。两者的 Skill 与 MCP 统一管理架构见[Skill 管理](skill-management.md)与[MCP 工具与客户端](mcp-tools.md)。

## 设计所在

本章用于为贡献者定向。权威的需求位于规范中。

- [openspec/specs/agent-lifecycle-management](../../../../openspec/specs/agent-lifecycle-management/spec.md)
- [openspec/specs/agent-provider-runtime](../../../../openspec/specs/agent-provider-runtime/spec.md)
- [openspec/specs/agent-switching](../../../../openspec/specs/agent-switching/spec.md)

native 执行路径位于 `agent_runtime` bounded context 中;见 [Native bounded context](native-contexts.md)。
