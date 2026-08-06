# Change: add-personalization-settings

## Why

VaneHub AI 当前没有"个性化"能力：OnePiece 每次会话都从核心指令 + Skills + 记忆冷启动，用户的语言偏好、身份背景、回复风格约束无法一次性配置、长期生效。参考 ChatGPT Desktop 的 Personalization 设置，需要提供**主机级自定义指令**（对标 Custom Instructions）与**记忆的用户可控性**（对标 Manage Memories）。

记忆的采集/保留/整合本身三天前已经上线（`add-agent-cross-session-memory`，2026-08-01 归档）：`remember` 工具、基于压缩触发的自动抽取、系统提示词注入全部已实现（`api_process_adapter.rs`）。但它缺三样东西：用户没有开关能关闭它、没有区分"工具辅助会话"是否要生成记忆、只能逐条删除没有一键重置。更紧迫的是，承载记忆列表/删除的设置面板（`agent-memory-panel.tsx`）在今天的 OnePiece 整合提交（`feat: add native OnePiece agent and provider configuration #83`）中，作为旧版多 Agent 设置页的一部分被一并删除——后端命令（`list_agent_memories`/`delete_agent_memory`）仍然完整可用，但用户现在完全没有入口查看或管理自己的记忆。

## What Changes

- 新增主机级**自定义指令**：`about_user`（关于你）+ `style_rules`（回复风格）两个独立文本字段，各 ≤3000 字符，一个总启用开关。存储复用 `desktop` 限界上下文已有的 `DesktopSettings` key/value 聚合（`src-tauri/src/contexts/desktop/domain/settings.rs`），不新建表、不新建限界上下文。
- 新增跨 context 只读端口 `AgentPersonalizationPort`（`agent_runtime → desktop`，一条新的依赖边，镜像 `AgentSkillPort` 桥接 `tooling` 的既有模式），在生成时读取自定义指令与记忆偏好开关。
- 扩展 `resolve_system_prompt`（`api_process_adapter.rs:769`）：在 core instructions 之后、Skills 之前插入自定义指令 section；`agent-skill-injection` capability 的 "Deterministic API system-prompt section ordering" requirement 同步更新组装顺序。
- 新增记忆主开关 `memoryEnabled`：关闭时 `remember` 工具调用、自动抽取、系统提示词注入三者一起停用。默认值为 `true`（保留当前"始终开启"的行为），不影响已有用户。
- 新增记忆子开关 `memoryToolAssistedChatsEnabled`：仅当会话内已发生工具调用（shell/file/MCP）时，控制是否允许*自动抽取*为该次压缩生成记忆；显式 `remember` 工具调用不受影响——用户主动要求记住的内容，不因为聊天里用过工具就被拦截。
- 新增记忆批量重置：`AgentMemoryPort` 新增 `delete_all_for_agent`，新增 Tauri command，供设置页"重置记忆"按钮使用。
- 恢复并重建记忆管理 UI：列表 + 单条删除（已有能力）+ 批量重置（新增），迁移到新设置页。
- 新增独立设置页「个性化」（`settings-center-ui` 新增导航条目），承载自定义指令表单 + 记忆三个开关 + 记忆管理列表。
- Web/mock 侧为以上全部行为提供确定性模拟，保持与 desktop 运行时的服务契约一致。

## Non-Goals（本 change 不做）

- **四个 CLI 包装型 Agent（Claude Code / OpenCode / Codex CLI / Gemini CLI，即 `managedCliAgentIds`）的指令注入**——它们通过子进程 + PTY 运行，不经过 `resolve_system_prompt`，需要完全不同的机制（启动参数 / 托管上下文文件 / 首条消息前缀降级）。这是一块独立、量级相当的工作，放到后续变更里做，不与 OnePiece 这条"扩展现有管线"的轻量路径混在一起。
- 自定义指令内容的 LLM 校验或改写。
- 按 workspace / 按 Agent 差异化自定义指令（首版仅主机级全局；这与记忆现有的按 agent + folder 隔离是两回事，互不影响）。
- 记忆的向量检索 / 语义相关性排序（`add-agent-cross-session-memory` 已有的 Non-Goal，仍然成立）。

## Capabilities

### New Capabilities
- `custom-instructions`：OnePiece 的主机级自定义指令——配置持久化、系统提示词组装、启用开关。

### Modified Capabilities
- `agent-cross-session-memory`：记忆主开关、工具辅助会话子开关、批量重置、管理 UI 修复。
- `app-settings`：`DesktopSettings` 新增自定义指令与记忆偏好相关键。
- `agent-skill-injection`：系统提示词 section 组装顺序纳入自定义指令。
- `settings-center-ui`：新增「个性化」设置导航条目。

## Impact

- Affected specs: `custom-instructions`（新增），`agent-cross-session-memory`、`app-settings`、`agent-skill-injection`、`settings-center-ui`（修改）
- Affected code：
  - `src-tauri/src/contexts/desktop/domain/settings.rs`、`infrastructure/sqlite_settings_repository.rs`：新增设置键
  - `src-tauri/src/contexts/agent_runtime/application/ports.rs`：新增 `AgentPersonalizationPort`
  - `src-tauri/src/contexts/agent_runtime/infrastructure/`：新增跨 context 桥接适配器；`api_process_adapter.rs` 的 `resolve_system_prompt` / `execute_tool_call` / `maybe_compact` 调用路径接入开关判断
  - `src-tauri/src/contexts/agent_runtime/infrastructure/memory_repository.rs`、`application/ports.rs`：新增 `delete_all_for_agent`
  - `src-tauri/src/commands/agent_runtime/`：新增 `reset_agent_memories` command
  - `src/settings/settings-pages.ts`，新增 `src/settings/pages/personalization-page.tsx` 及子组件
  - `src/services/settings-service.ts`、`src/services/agent-service.ts` 及对应 tauri/web 实现
  - `src/i18n/locales/{zh-CN,en,zh-TW,ja,ko}.json`
- 风险：`agent_runtime` 首次依赖 `desktop`——此前二者之间没有跨 context 调用，需要在实现前确认方向正确（`desktop` 不反向依赖 `agent_runtime`）且不引入循环。已有行为的默认值经过选择以保证不破坏现有正在使用记忆功能的用户（详见 design.md）。
