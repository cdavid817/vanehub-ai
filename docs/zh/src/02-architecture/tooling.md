# 工具生态：MCP、Skills、扩展与 CLI 配置

> **集中配一次，按 Agent 下发**：MCP server、Skill、Prompt Hook、本地扩展与 SDK 依赖都在设置中心统一注册，再绑定到具体 Agent，而不是在每个 CLI 里各配一遍。

## 这一层解决什么问题

**`tooling` 是一个"元上下文"**——它下辖八个各自独立的子域（`src-tauri/src/contexts/tooling/`），每个都遵循同一套 `api.rs` + `application/` + `domain/` + `infrastructure/` 四层结构：

| 子域 | 职责 | 命令面 |
|---|---|---|
| `mcp` | MCP server 注册、连接、工具暴露与中继 | 大 |
| `skills` | Skill 注册、挂载、绑定与漂移检测 | 大 |
| `prompt_hooks` | 提示词生命周期钩子 | 中 |
| `extensions` | 本地 OCR / ASR / TTS 扩展 | 中 |
| `plugin_integrations` | 第三方插件集成 | 小 |
| `sdk` | 受管 SDK 依赖 | 中 |
| `cli` | CLI 安装检测、版本、冲突与生命周期 | 大 |
| `cli_config` / `cli_parameters.rs` | CLI 配置档案与启动参数 | 中 |

**八个子域合计 82 个命令文件**，是全仓命令面最大的上下文，详见 [限界上下文](bounded-contexts.md#tooling-是元上下文)。

## MCP 客户端管理

### 传输方式

**三种**（`tooling/mcp/domain/mod.rs:89-93` 的 `TransportType`）：

| 传输 | 必需字段 | 缺失时的错误 |
|---|---|---|
| `Stdio` | `command` | `MissingStdioCommand` |
| `Sse` | `url` | `MissingUrl` |
| `StreamableHttp` | `url` | `MissingUrl` |

名称非法报 `InvalidServerName`（`mod.rs:9-15`）。

### 作用域与状态

**作用域两种**（`mod.rs:115-118` 的 `Scope`）：`User`（用户级）与 `Project`（项目级）。

**连接状态四种**（`mod.rs:262-267` 的 `ConnectionStatus`）：`Connected`、`Disconnected`、`Error`、`Disabled`。

**失败分类六种**（`mod.rs:23-29` 的 `McpFailureCode`）：

| 分类 | 典型原因 |
|---|---|
| `Validation` | 配置写错 |
| `Spawn` | 子进程起不来 |
| `Timeout` | 超时 |
| `Cancelled` | 被取消 |
| `Protocol` | 协议层错误 |
| `UpstreamHttp` | 上游 HTTP 故障 |

**把"配置写错"与"上游挂了"分开**，排查时不必从一个笼统错误里猜原因。

### 中继

**MCP 中继让外部 CLI 也能用上统一注册的 server**。实现在 `mcp/infrastructure/`：`relay.rs`、`relay_streamable_http.rs`、`relay_legacy_sse.rs`、`relay_legacy_sse_session.rs`，**每种传输都配有独立的 `_tests.rs` 与 `_failure_tests.rs`**。

**受管中继当前只对两个 Agent 启用**：`claude-code` 与 `codex-cli`（`src-tauri/src/bootstrap/managed_mcp_relay.rs:110`），且两者的接入形态不同：前者写配置文件传 `--mcp-config`，后者传命令行覆盖项。

**中继链路传播 W3C `traceparent`**，因此 MCP 调用能并入同一条执行 trace。完整说明见 [MCP 集成](mcp-integration.md)。

底层依赖 `rmcp 3.0.1`。

## Skill 管理

### 作用域

**两种**（`skills/domain/identity.rs:22-25` 的 `SkillScope`）：`Global` 与 `Workspace`。工作区作用域用空字符串哨兵表示"无工作区"，这一约定被 [Agent 记忆](personalization.md#agent-记忆) 沿用。

### 漂移检测

**Skill 是文件系统上的实体，会被绕过界面修改**，因此需要漂移检测（`skills/domain/drift.rs`）。

**六种漂移问题**（`drift.rs:4-10` 的 `SkillDriftIssueType`）：

| 问题 | 含义 |
|---|---|
| `MissingSource` | 源文件不见了 |
| `MetadataChanged` | 元数据被改动 |
| `UnregisteredSource` | 存在未注册的源 |
| `MissingMount` | 挂载点缺失 |
| `Conflict` | 冲突 |
| `DeletedBuiltin` | 内置 Skill 被删除 |

**每条问题携带的信息**（`drift.rs:56-63` 的 `SkillDriftIssue`）：`skill_id`、`issue_type`、可选的 `agent_id` 与 `path`、以及一条 `&'static str` 消息。

**消息是静态字符串而非格式化文本**——六种问题各对应一句固定说明：

| 问题 | 消息 |
|---|---|
| `MissingSource` | `SKILL.md is missing` |
| `MetadataChanged` | `SKILL.md differs from the registry snapshot` |
| `MissingMount` | `Agent mount is missing` |
| `Conflict` | `Agent mount path is occupied by unmanaged content` |
| `UnregisteredSource` | `Skill source exists without a registry record` |
| `DeletedBuiltin` | `Built-in Skill is deleted and can be restored` |

#### 四条检测规则

**`detect_drift`（`drift.rs:65-136`）是一个纯函数**，输入是一次巡检快照，输出是问题列表——不碰文件系统，因此可以完全用构造的数据测试。

**规则一：源缺失时短路。**（`:69-78`）源文件不存在就 `continue`，**不再检查挂载**——源都没了，挂载状态没有诊断价值，报一堆下游问题只会淹没根因。

**规则二：哈希不等即元数据变更。**（`:79-89`）比对的是 `content_hash` 与注册时的 `expected_content_hash`。

**规则三：只有启用的 Skill 才查挂载。**（`:93`）

```rust,ignore
if skill.enabled {
    for binding in &skill.bindings { ... }
}
```

**停用的 Skill 本来就不该有挂载**，去检查它必然报一堆 `MissingMount`。测试名把这条与下一条一起钉住了：`disabled_skills_skip_mount_drift_and_deleted_builtins_are_global_only`（`drift.rs:242`）。

**规则四：`DeletedBuiltin` 只在全局作用域报。**（`:125`）

```rust,ignore
if inspection.location.scope == SkillScope::Global {
```

**内置 Skill 是全局概念**，在项目作用域下巡检时报「某个内置 Skill 被删了」没有意义——那不是这个项目能管的事。

#### 挂载观测有三态

（`drift.rs:13-18` 的 `SkillMountObservation`）

| 观测 | 处理 |
|---|---|
| `Managed` | 正常，`continue` |
| `Missing` | 报 `MissingMount` |
| `Conflict` | 报 `Conflict` |

**`Conflict` 与 `Missing` 分开是必要的**：路径空着和路径被**非受管内容**占着，处置完全不同——前者重新挂载即可，后者贸然覆盖会删掉用户自己的文件。

**挂载观测三种**（`drift.rs:14-18` 的 `SkillMountObservation`）：`Managed`、`Missing`、`Conflict`。

**源检查带内容哈希**（`drift.rs:28-31` 的 `SkillSourceInspection`）：`Present` 变体携带 `content_hash`，据此判断内容是否被改动，而不只是看文件在不在。

### 校验

领域错误覆盖了常见误用（`skills/domain/error.rs:4-10`）：`InvalidId`、`MissingMetadataFields`、`WorkspacePathRequired`、`CreateIdMismatch`、`UpdateIdChanged`、`InvalidMountPath(path)`。

**`UpdateIdChanged` 与 `CreateIdMismatch` 分开**：创建时 id 对不上、更新时试图改 id，是两类不同的错误。

## Prompt Hooks

### 绑定对象

**只能绑定到四个受管 CLI Agent**（`prompt_hooks/domain/binding.rs:5-10` 的 `ManagedCliAgentId`）：`ClaudeCode`、`CodexCli`、`GeminiCli`、`OpenCode`。

**`onepiece` 不在其中**——原生 Agent 有自己的核心指令与上下文构造机制，不通过 Prompt Hook 介入。

**绑定列表会去重**（`binding.rs:50-53` 的 `PromptHookBindings::new`，用 `HashSet` 判重）。

### 分类与阶段

**六种分类**（`domain/classification.rs:2-8` 的 `PromptHookCategory`）：

| 分类 | 用途取向 |
|---|---|
| `Bootstrap` | 启动引导 |
| `Callback` | 回调 |
| `Dynamic` | 动态内容 |
| `Law` | 规则约束 |
| `Navigation` | 导航 |
| `Routing` | 路由 |

**两个执行阶段**（`classification.rs:43-46` 的 `PromptHookStage`）：

| 阶段 | 时机 |
|---|---|
| `SessionInit` | 会话初始化时执行一次 |
| `PerTurn` | 每一轮都执行 |

**内置钩子定义带排序**（`domain/catalog.rs:4-10` 的 `BuiltinPromptHookDefinition`）：`id`、`name`、`description`、`category`、`stage`、`order`——`order` 决定同阶段内的执行顺序。

## 本地扩展

**扩展不是通用插件，而是本地多模态 AI 能力。**

**三种能力**（`extensions/domain/catalog.rs:2-6` 的 `ExtensionCapabilityId`）：

| 能力 | 含义 |
|---|---|
| `ocr` | 图像文字识别 |
| `asr` | 语音识别 |
| `tts` | 语音合成 |

**三个框架**（`catalog.rs:28-32` 的 `ExtensionFrameworkId`）：

| 框架 | 存储值 | 对应能力 |
|---|---|---|
| PaddleOCR | `paddleocr` | OCR |
| faster-whisper | `faster-whisper` | ASR |
| sherpa-onnx | `sherpa-onnx` | ASR / TTS |

**每个框架声明模型需求**（`catalog.rs:54-79` 的 `ExtensionModelRequirement` / `ExtensionRequirement` / `ExtensionFrameworkDefinition`）——安装时据此拉取所需模型文件。

**这解释了扩展管理为什么需要独立子域**：它涉及模型文件下载、磁盘占用与运行时依赖，比"装个插件"复杂得多。

## SDK 依赖管理

### 两个受管 SDK

**定义在 `sdk/domain/mod.rs:38-60` 的 `SDK_DEFINITIONS`**：

| 项 | Claude Code SDK | Codex SDK |
|---|---|---|
| id | `claude-sdk` | `codex-sdk` |
| npm 包 | `@anthropic-ai/claude-agent-sdk` | `@openai/codex-sdk` |
| 默认版本 | `0.2.88` | `0.117.0` |
| 伴随包 | `@anthropic-ai/sdk`、`@anthropic-ai/bedrock-sdk` | 无 |
| 备选版本 | `0.2.88` / `0.2.81` / `0.2.58` | `0.117.0` / `0.116.0` / `0.115.0` |
| 关联 provider | `anthropic`、`bedrock` | `openai` |

**每个 SDK 都带三个备选版本**——某个版本出问题时可以回退，而不是只能升不能降。

**与内置 Agent 的对应**：`claude-code` 与 `codex-cli` 在 Agent 种子中各自声明了 `managedSdkDependencyId`（`src/services/mock-agent-data.ts:8,29`）。

### 状态模型

`sdk/domain/mod.rs` 定义了一整套生命周期类型：`SdkInstallStatus`（`:69`）、`SdkStatus`（`:76`）、`SdkVersionSource`（`:118`）、`SdkVersionInfo`（`:124`）、`SdkUpdateInfo`（`:179`）、`SdkOperationType`（`:203`）、`SdkLifecycleAction`（`:211`）、`SdkLifecyclePlan`（`:217`）、`SdkOperationOutcome`（`:253`）。

**`SdkLifecyclePlan` 的存在说明升级不是单步操作**——它是一个先算好、再执行的计划。前端版本比较逻辑在 `src/services/sdk-versioning.ts`。

## 插件集成

第三方插件的集成配置（`tooling/plugin_integrations/`），完整四层子域，界面在 `src/settings/pages/plugin-integrations-page.tsx`，前端适配在 `src/services/plugin-integration-adapter.ts` 与 `plugin-integration-service.ts`。

## CLI 管理

### 检测与状态

**环境类型四种**（`tooling/cli/domain/mod.rs:62-67` 的 `EnvironmentType`）：`Windows`、`Macos`、`Linux`、`Unknown`。

**版本检查状态四种**（`mod.rs:70-75` 的 `VersionCheckStatus`）：

| 状态 | 含义 |
|---|---|
| `Unsupported` | 版本不受支持 |
| `NotDetected` | 未检测到 |
| `Succeeded` | 检查通过 |
| `Failed` | 检查失败 |

**`Unsupported` 与 `Failed` 分开**：装了但版本太老，和检查过程本身出错，是两回事。

**安装来源**（`mod.rs:78-89` 的 `InstallSource`）：`Npm`、`Winget`、`Desktop`、`Homebrew`、`Volta`、`Bun` 等——**同一个 CLI 可能同时来自多个来源，这正是冲突的根源**。

**冲突状态四种**（`mod.rs:91-96` 的 `ConflictState`）：

| 状态 | 含义 |
|---|---|
| `None` | 无冲突 |
| `Multiple` | 检测到多份安装 |
| `VersionMismatch` | 版本不一致 |
| **`RunnableMismatch`** | **实际会被执行的那份与预期不符** |

**`RunnableMismatch` 是最隐蔽的一种**：`PATH` 顺序决定了真正跑起来的是哪一份，可能不是你以为的那份。

**生命周期可管性**（`mod.rs:99-105` 的 `LifecycleEligibility`）：`Npm`、`Wget`、`Winget`、`Manual`、`Unavailable`——决定 VaneHub AI 能否代为安装升级，还是只能提示手工处理。

**操作类型三种**（`cli/application/models.rs:81-85` 的 `CliOperationType`）：`Refresh`、`Install`、`UpgradeAll`。

### 启动参数

**参数控件四种**（`tooling/cli_parameters.rs:30-35` 的 `CliParameterControl`）：`Enum`、`Boolean`、`MultiEnum`、`CustomText`。

**参数带风险标注**（`cli_parameters.rs:39-42` 的 `CliParameterRisk`）：`Normal` 与 `Warning`——危险开关在界面上会被显著标记。

**参数按启动场景区分**（`cli_parameters.rs:46-49` 的 `CliParameterLaunchScope`）：

| 场景 | 含义 |
|---|---|
| `Interactive` | 交互式终端启动 |
| `Chat` | 对话启动 |

**同一个 CLI 在两种场景下需要的参数并不相同**，因此参数必须带这个维度。

**权限模板会压过用户保存的参数选择**——见 [CLI 集成](cli-integration.md#差异吸收点一启动参数)。

## 界面入口与前端服务

| 要做的事 | 入口 |
|---|---|
| 注册 MCP server | 设置中心 → MCP 页（`src/settings/pages/mcp-page.tsx`） |
| 导入现有 MCP 配置 | MCP 页导入功能，逻辑见 `src/services/mcp-import.ts` |
| 校验 MCP 配置 | `src/services/mcp-validation.ts`、`mcp-tool-validation.ts` |
| 管理 Skill | 设置中心 → Skills 页（`skills-page.tsx`） |
| 配置 Prompt Hook | 设置中心 → Prompt Hooks 页（`prompt-hooks-page.tsx`） |
| 安装本地扩展 | 设置中心 → 扩展页（`extensions-page.tsx`） |
| 配置插件集成 | `plugin-integrations-page.tsx` |
| 管理 SDK 依赖 | `sdk-page.tsx` |
| 查看 CLI 安装状态 | `cli-installation-list.tsx`、`cli-environment-card.tsx` |
| 处理 CLI 冲突 | `cli-conflict-dialog.tsx` |
| 配置启动参数 | `cli-parameters-page.tsx` |
| CLI 配置档案 | `settings/pages/agents/cli-config-*.tsx` |

## 边界与限制

- **仅桌面可用** —— 所有子域都涉及进程启动、文件系统或 SQLite，Web/mock 模式下为模拟数据。
- **受管 MCP 中继只覆盖两个 Agent** —— 仅 `claude-code` 与 `codex-cli`；OpenCode 与 Gemini CLI 需各自配置。
- **Prompt Hook 不作用于 OnePiece** —— 只能绑定四个受管 CLI Agent。
- **漂移只报告不自动修复** —— 检测到问题时需人工确认处理方式。
- **CLI 生命周期管理受来源限制** —— `Manual` 或 `Unavailable` 的安装无法由 VaneHub AI 代为升级。
- **扩展需要下载模型文件** —— 占用磁盘且首次安装耗时。
- **受管 SDK 只有两个** —— OpenCode 与 Gemini CLI 没有对应的受管 SDK。
- **SDK 依赖门控与执行链路的关系需注意** —— 部分 Agent 的可用性判定与受管 SDK 安装相关，但实际执行不一定经过该包。
- **不改写 CLI 自身配置文件** —— 工具绑定通过启动参数与中继实现。

## 相关文档

- [MCP 集成](mcp-integration.md) —— 中继架构与私有目录
- [CLI 集成](cli-integration.md) —— 参数与权限模板的合成
- [权限审批](permissions-architecture.md) —— `mcp.tool` 动作的管辖
- [可观测性](observability-architecture.md) —— MCP 调用的 trace 传播
- [个性化](personalization.md) —— 专家角色绑定的 Skill
