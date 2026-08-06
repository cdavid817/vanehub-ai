# Change: add-cli-memory-support

## Why

`add-personalization-settings`（Phase 1）把记忆能力（`remember` 工具、自动抽取、system prompt 注入、列表/删除/重置管理）限定为仅 OnePiece，理由是"扩展到 CLI Agent 需要完全不同的机制"。`add-cli-custom-instructions-injection`（Phase 2）已经证明这条理由对自定义指令不成立——CLI 早就有现成的 Prompt Hook 注入管线可以复用。这次针对记忆能力重新做了同样的调查：读（注入）确实可以复用 Phase 2 已经打通的 CLI effective prompt 组装点；写（产生新记忆）没有现成机制可以直接复用（OnePiece 的 `remember` 工具和压缩触发抽取都深度耦合在它自己的 API 工具调用循环里），但存在一条可行的新路径——`service.rs` 里已经有一个 API 和 CLI 共用的生成完成钩子（`GenerationEventHandler::completed()`），可以在这个点为 CLI 新增一条独立的抽取触发逻辑，复用 OnePiece 已配置的凭据发起一次独立的模型调用。

## What Changes

- **存储模型改为主机级共享池**：`agent_memories` 的读取（`list`/`list_all_for_agent`，含注入与管理面）不再按 `agent_id` 过滤；`agent_id` 列保留但仅作溯源用途，不再作为查询边界。效果：在 codex-cli 里产生的记忆，之后在 OnePiece 或 claude-code 里也能被注入和管理到——呼应自定义指令"一次设置，主机上所有会话生效"的哲学。这是一处真实的行为变化：升级后，此前彼此隔离的五个 agent 各自的记忆会互相可见，需要在 spec 里明确记录并在手动测试里验证。
- **CLI 消息发送路径新增记忆注入（读）**：复用 Phase 2 已经建立的 CLI 分支组装点（`service.rs` 的 `agent.launch().kind_str() == "cli"` 分支），在自定义指令与 Prompt Hook 组装结果之外新增记忆 section。查询失败时的降级策略镜像自定义指令：记 Warn 日志，跳过记忆 section，不阻断 CLI 消息发送。
- **CLI 生成完成后新增独立的记忆抽取路径（写）**：在共用的 `GenerationEventHandler::completed()` 钩子里，当 `agent.launch().kind_str() == "cli"` 时新增一条独立触发逻辑：检查 `ApiCredentialPort::fetch("onepiece")?.is_some()`，未配置则记 Warn 日志、跳过（不阻断，CLI 消息本身已经完成，这一步纯粹是异步的记忆产出）；已配置则发起一次独立的、复用 OnePiece 已配置凭据的模型调用，从这轮 CLI 对话中抽取值得记住的内容，写入共享记忆池（`source = automatic`）。
- **新增可独立调用的记忆抽取能力**：现有 `extract_memories`/`summarize_turns`（`api_process_adapter.rs`）是被动接收 OnePiece 生成过程中已解析好的凭据/client 作为参数，不能直接在生成之外复用。新增一个由 `agent_runtime` application 层定义、infrastructure 层实现的端口（承载"给一段文本，独立发起一次模型调用做记忆抽取"这个能力），内部复用 `summarize_turns` 的 HTTP 调用逻辑，但改为主动查找凭据而非被动接收。
- **OnePiece 现有机制不变**：`remember` 工具、压缩触发抽取（`maybe_compact`/`extract_memories`，字符阈值 60,000）、`resolve_system_prompt` 里的注入逻辑全部保持原样——这次改动只是让"读"面向所有 agent 开放（通过上面的存储模型改动自动生效，OnePiece 侧代码不用改），"写"面新增一条 CLI 专属的独立触发路径，不修改、不复用、不干扰 OnePiece 已有的两个触发点。
- **管理面（列表/删除/重置）自动获得跨 agent 可见性**：现有的 `list_agent_memories`/`delete_agent_memory`/`reset_agent_memories` 命令因为存储模型改动而自动变成"看到的是共享池"，前端「个性化」设置页当前硬编码 `agent_id = "onepiece"` 展示记忆列表——是否需要在 UI 上调整措辞（比如从"OnePiece 的记忆"改成"本机记忆"）留给 design.md 讨论。

## Non-Goals（本变更不做）

- CLI 抽取节奏的精细节流（比如"内容太短跳过""攒够字符量再抽"）——先做"每轮都抽"的简单版本，节流留给后续按实际使用情况评估。
- 记忆内容的去重/合并——共享池上线后，同一件事可能被不同 agent 各自抽取一次而重复入库，去重机制不在本变更范围。
- 记忆的向量检索或语义排序——沿用现有的按 `created_at` 倒序 + 字符预算裁剪。

## Capabilities

### Modified Capabilities
- `agent-cross-session-memory`：存储与查询模型从"按 agent_id 隔离"改为"主机级共享池"；新增 CLI 侧的记忆注入（读）与独立抽取触发（写）两类 requirement。
- `native-runtime-architecture`：新增记忆 section 在 CLI effective prompt 组装里的位置契约，以及"CLI 生成完成后触发独立抽取调用"这一新运行时行为的契约。

## Impact

- Affected specs: `agent-cross-session-memory`（修改）、`native-runtime-architecture`（修改）
- Affected code：
  - `src-tauri/src/contexts/agent_runtime/infrastructure/memory_repository.rs`（查询合并为单一的 `list_all()`，不再按 `agent_id`/`folder` 过滤；详见 design.md D1 的实现落地说明）
  - `src-tauri/src/contexts/agent_runtime/application/ports.rs`（新增记忆抽取端口）
  - `src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter.rs`（抽出可复用的抽取调用逻辑；OnePiece 现有触发点不变）
  - `src-tauri/src/contexts/agent_runtime/application/service.rs`（CLI 分支新增记忆注入；`GenerationEventHandler::completed()` 新增 CLI 场景下的抽取触发）
  - `src-tauri/src/bootstrap/agent_runtime.rs`（新端口装配）
  - `src/settings/pages/personalization/agent-memory-section.tsx`（视具体 UI 措辞决定是否需要改动，留给 design.md）
- **排序依赖**：本变更建立在 Phase 1（`add-personalization-settings`）与 Phase 2（`add-cli-custom-instructions-injection`）已归档的假设上——存储模型改动直接作用于 Phase 1 建的表，CLI 记忆读取直接依赖 Phase 2 建的 CLI effective prompt 组装点。本变更不应在前两者归档之前被归档。
- 无新增 UI 结构：复用 Phase 1 已有的记忆管理面板，具体文案是否需要更新留给 design.md。
