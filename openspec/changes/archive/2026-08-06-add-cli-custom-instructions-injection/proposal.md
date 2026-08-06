# Change: add-cli-custom-instructions-injection

## Why

`add-personalization-settings`（Phase 1）为 OnePiece 建了主机级自定义指令，四个 CLI 包装型 Agent（Claude Code / OpenCode / Codex CLI / Gemini CLI，即 `managedCliAgentIds`）当时被明确列为 Non-Goal，理由是"需要完全不同的注入机制"。

探索后发现这个理由不成立：这四个 CLI 已经有一条现成的、生产环境在跑的注入管线——`tooling::prompt_hooks` 的 `assemble_prompt_work`。每次给这四个 CLI 中任意一个发消息（`agent.launch().kind_str() == "cli"`），都会先跑一遍 Prompt Hook 组装：按 CLI 绑定过滤已发布/启用的 hook，渲染模板，拼接成一个"effective prompt"，这段文本才是真正写进 PTY、发给 CLI 子进程的内容。7 个内置 hook 里已经有一个不可禁用的"law-runtime-boundary"，证明"每轮把一段固定文本前置到用户消息前面"这个模式本身就是这套系统已经接受、且在用的设计。

所以本变更不是要建一套新的注入机制，而是把 Phase 1 已经建好的自定义指令设置，接进这条现成管线的下游一个新消费点。

## What Changes

- `AgentRuntimeApplicationPorts` 新增 `personalization: Arc<dyn AgentPersonalizationPort>` 字段（复用 Phase 1 已有的端口类型与 `RuntimeAgentPersonalizationAdapter`，bootstrap 里对同一个 `Arc` 多 clone 一份给这个新消费方，而不是新建端口）。
- CLI 消息发送路径（`AgentRuntimeApplicationService` 里 `agent.launch().kind_str() == "cli"` 分支）：在 `self.ports.prompts.assemble(...)` 之外，额外读取自定义指令设置，把渲染结果拼在 Prompt Hook 组装出的 effective prompt **前面**。
- 把自定义指令的格式化逻辑（style_rules 在前、about_user 在后那套规则）从 `infrastructure::api_process_adapter`（当前仅 OnePiece 私有）上移到 `application` 层（`PersonalizationSettings` 的方法或 application 内的共享函数），OnePiece 与 CLI 两条路径共用同一份格式化规则，不重复实现、不允许两条路径的文案漂移。
- 复用 Phase 1 已有的 `customInstructionsEnabled` 总开关——不新增开关，OnePiece 和四个 CLI 共用同一份"启用/禁用"状态，一次设置，四个 CLI + OnePiece 一起生效，符合"适用于本机所有会话"的原始诉求。
- Web/mock：已确认无需改动（调查结论见 design.md Open Questions）——`web-agent-client.ts` 的 `sendMessage` 是所有 agent 共用的同一个桩函数，回复内容是写死模板字符串，从未模拟过"实际发给模型/CLI 的提示词内容"这个概念，因此没有可观察行为需要新增模拟。
- 更新 `native-runtime-architecture` 的 "Native Prompt Hook pipeline" 相关 effective-prompt 组装契约，明确自定义指令与 Prompt Hook 管线的组合顺序。

## Non-Goals（本变更不做）

- 记忆能力扩展到 CLI Agent——Phase 1 的记忆功能（`remember` 工具、自动抽取、注入）设计上就只对 `launch_kind == "api"` 生效，这是三天前记忆功能上线时定的边界，本变更不改这条边界，也不评估要不要改。
- 给 Prompt Hooks 本身新增分类、新增模板变量、修改草稿/发布/回滚生命周期——完全不碰 `tooling::prompt_hooks` 内部实现，只在 `agent_runtime` 侧消费它已经产出的 effective prompt。
- 每个 CLI 的原生启动参数 / 托管上下文文件等"更原生"的注入方式（最初参考方案设想的 `ContextInjector` 三级降级）——评估后放弃：现有的文本前置机制已经统一覆盖四个 CLI 且在生产使用，没有证据表明需要更复杂的分级机制；如果未来某个 CLI 的原生机制被证明明显更优，再单独评估。
- "只在会话第一条消息注入一次"（`PromptHookStage::SessionInit` 语义）——现有 Prompt Hook 管线里这个 stage 字段目前没有被实际强制执行（每个 enabled+bound 的 hook 都是每轮重新渲染），本变更维持这个已有行为，不新建会话状态追踪机制。

## Capabilities

### Modified Capabilities
- `custom-instructions`：范围从"仅 OnePiece"扩大到"OnePiece + 四个 CLI 包装型 Agent"，新增 CLI 侧的组装/降级 requirement。
- `native-runtime-architecture`：Prompt Hook pipeline 产出的 effective prompt 与自定义指令的组合顺序纳入契约。

## Impact

- Affected specs: `custom-instructions`（修改）、`native-runtime-architecture`（修改）
- Affected code：
  - `src-tauri/src/contexts/agent_runtime/application/service.rs`（CLI 发送路径新增自定义指令拼接）
  - `src-tauri/src/contexts/agent_runtime/application/models.rs`（`PersonalizationSettings` 新增共享格式化方法）
  - `src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter.rs`（原有格式化函数改为委托给上移后的共享方法，OnePiece 路径行为不变）
  - `src-tauri/src/bootstrap/agent_runtime.rs`（`AgentRuntimeApplicationPorts` 新字段装配）
  - `src/services/web-agent-client.ts`（CLI-kind 模拟会话）
- **排序依赖（重要）**：本变更的 specs delta 是针对 `add-personalization-settings`（Phase 1）归档后主 specs 的状态写的（`custom-instructions` 届时才会作为 ADDED 合并进主 specs）。`openspec validate` 本身不会检查被 MODIFIED 的能力在主 specs 里是否已存在，所以此刻起草不会报错，但**归档顺序必须是 Phase 1 先于本变更**，否则合并结果未经验证、有风险。本变更不应在 Phase 1 归档之前被归档。
- 无新增 UI：复用 Phase 1 已经建好的「个性化」设置页与 `customInstructionsEnabled` 开关，用户侧无需学习新操作。
- 无破坏性变更：纯增量，四个 CLI 在自定义指令为空或禁用时行为与现在完全一致。
