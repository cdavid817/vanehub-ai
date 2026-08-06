# Design: add-cli-memory-support

## Context

个性化能力第三阶段。Phase 1（`add-personalization-settings`）为 OnePiece 建了完整的记忆能力（`remember` 工具、压缩触发自动抽取、system prompt 注入、列表/删除/重置管理），明确把 CLI 扩展列为 Non-Goal。Phase 2（`add-cli-custom-instructions-injection`）证明"CLI 需要完全不同的机制"这条假设对自定义指令不成立——CLI 早就有现成的 Prompt Hook 注入管线。本变更针对记忆能力重新做了同样的调查（四路并行代码调查，均有 file:line 依据），结论是：读（注入）能复用同一条路，写（产生新记忆）不能直接复用，但有可行的新路径。

**读面的现状**（`memory_schema.rs`、`memory_repository.rs`）：`agent_memories` 表只有 `id/agent_id/folder/content/source/created_at/updated_at`，索引 `(agent_id, folder, created_at DESC)`。所有调用点（`resolve_system_prompt` 的 `memories.list`、`extract_memories` 的 `memories.save`、`remember` 工具的 `memories.save`、三个管理命令）严格按 `agent_id` 过滤。OnePiece 的 `agent_id` 是稳定字符串 `"onepiece"`，四个 CLI 各自也是稳定字符串——现状五个身份互不相通。

**写面的现状**（`api_process_adapter.rs`）：`maybe_compact`/`extract_memories` 是私有函数，只在 OnePiece 自己的 `execute()` 生成循环内被调用（工具轮次前、工具轮次后两处），由字符数阈值（`COMPACTION_TRIGGER_CHARACTERS = 60_000`）触发，不是每轮触发。`extract_memories` 内部调用 `summarize_turns` 做实际的模型请求，复用的是当次生成已经解析好、作为参数传入的 `api_key`/`model`/`client`/`wire_format`，自己不做独立凭据查找。`summarize_turns` 本身入参干净（wire_format/client/api_key/model/turns/instruction/cancelled），不依赖 `GenerationProcessRequest`/sink/工具循环，理论上可以在生成之外独立调用，但目前模块私有。CLI 路径（`process_adapter.rs`/`RuntimeAgentProcessAdapter`）完全没有引用 `AgentMemoryPort`/`AgentPersonalizationPort`，跟这套机制零共享。

**新触发点**（`service.rs`/`composite_process_gateway.rs`）：存在一个 API 和 CLI 共用的生成完成钩子——`GenerationEventHandler::completed()`，通过 `CompositeAgentProcessGateway` 统一接线（按 `launch.kind` 路由到不同 adapter，完成事件回调到同一个 sink 实例）。触发时完整回复文本（`state.response`）已经可用，CLI/API 一致——这是可以复用的、唯一的"CLI 一轮结束"观察点。

**凭据可用性判断**（`credential_aware_registry.rs`、`service.rs`、`api_credentials.rs`）：`ApiCredentialPort::fetch("onepiece")?.is_some()` 是干净的"OnePiece 现在能不能用"判断，等价于 `onepiece_provider_config()?.credential_present`。多 provider profile 切换最终都会把当前激活的凭据复制进这一个稳定的 `"onepiece"` key。

**"可选查找失败就降级"惯用法**在代码库里独立出现至少 5 次（`resolve_personalization_settings`、`resolve_tool_catalog`、`CredentialAwareAgentRegistry::decorate`、`tooling::skills` 的 `bound_skill_prompts_for_api_agent`、`execution_observability` 的 `CompositeExecutionTelemetry::export`），是成熟、可放心复用的既有约定。

**关于本文档的决策状态**：D1、D2、D7 最初是 explore 阶段给出的推荐方案。D1（共享池）已经用户明确确认（"共享池，所有cli共享一个全局记忆"）；D2、D7 已按本文档描述的方案完整实现并通过测试验证（详见任务 3-6），实现过程中未发现需要偏离这三条推荐的理由。三者现状均为**已确认**，不再是开放问题。

## Goals / Non-Goals

**Goals：**
- 记忆的读（注入）和写（产生新记忆）都覆盖到四个 CLI 包装型 Agent，不只是读。
- 存储模型改为主机级共享，呼应自定义指令"一次设置，主机上所有会话生效"的哲学。
- 不改动 OnePiece 现有的两个触发点（`remember` 工具、压缩触发抽取），新增的 CLI 写入路径完全独立、可关闭、失败不影响 CLI 消息发送。

**Non-Goals：** 见 proposal.md（抽取节流优化、记忆去重、向量检索）。

## Decisions

### D1（已确认）：存储模型改为主机级共享池，不再按 agent_id 过滤读取

```
现状：agent_id 是查询边界          本设计：agent_id 只做溯源，不做过滤
┌──────────┐                      ┌──────────┐
│ onepiece │──only sees own──▶    │ onepiece │──┐
├──────────┤                      ├──────────┤  │      ┌─────────────┐
│CLI × 4   │──only sees own──▶    │CLI × 4   │──┼─────▶│  共享记忆池  │
└──────────┘                      └──────────┘  │      └─────────────┘
```

`memory_repository.rs` 的查询方法去掉 `WHERE agent_id = ?1` 这个条件。**实现落地时的调整**：原计划保留 `list`（仅按 `folder` 过滤）与新增 `list_all_for_agent`（无条件返回全部）两个方法并存，实现时发现共享池模型下 `folder` 也不再是有效的查询边界（见 spec.md "Memories are visible across every workspace folder"），于是两者合并为单一的 `list_all()`（`AgentMemoryPort::list_all`，无任何过滤条件）,不再保留旧的 `list`/`list_all_for_agent` 命名。`save`/`delete_all` 的语义相应调整（写入仍然记录调用方的 `agent_id` 作为溯源；`delete_all`——原计划命名 `delete_all_for_agent`,实现时同样简化为无参数的 `delete_all`——现在语义上是"清空整个共享池"而不是"清空某个 agent 的",已在 UI 措辞和 spec 里明确这一点，见 D8）。`delete`（按 memory id 删单条）本来就不按 agent_id 过滤，不受影响。

**取舍**：另一个选项是保留按 agent_id 隔离，只是把"CLI 现在也能有自己独立的一份记忆"这件事做出来——改动小得多（只是把 `launch_kind == "api"` 的限制去掉），但跟自定义指令确立的"主机级、全局一份"的产品哲学不一致，且会让用户困惑"为什么自定义指令是共享的，记忆却要分开管理"。选择共享池。

### D2（已确认）：OnePiece 现有触发点完全不动，CLI 新增独立触发路径

不改 `maybe_compact`/`extract_memories`/`remember` 工具的任何现有代码。新增的 CLI 写入路径是一段独立的、只在 `agent.launch().kind_str() == "cli"` 时执行的新逻辑，复用同一个 `AgentMemoryPort`/新的抽取端口，但触发条件、触发点、调用方式都是全新的，跟 OnePiece 的两条触发点零交叉。风险最低，不会让本已在生产环境跑的 OnePiece 记忆机制因为这次改动而回归。

### D3：新增 `AgentMemoryExtractionPort`，把"独立发起一次抽取调用"做成可复用能力

```rust
// application/ports.rs
pub(crate) trait AgentMemoryExtractionPort: Send + Sync {
    fn extract(
        &self,
        exchange: &str,       // 这一轮对话的文本（用户消息 + CLI 最终回复）
        agent_id: &str,
        folder: Option<&str>,
    ) -> Result<Option<String>, AgentRuntimeApplicationError>;
}
```

实现方（infrastructure 层）内部：`credentials.fetch("onepiece")` → `config.provider_config("onepiece")` → `wire_format_for(...)` → 复用 `summarize_turns` 的 HTTP 调用逻辑（从 `api_process_adapter.rs` 抽出，改为 `pub(crate)` 或整体挪到一个双方都能访问的位置）。返回 `Ok(None)` 表示"这轮没有值得记住的内容"（镜像 `extract_memories` 现有的空结果语义）。凭据未配置或调用失败，由调用方（`service.rs` 的完成钩子）负责按 D2 的降级策略处理，端口本身只如实返回 `Err`。

### D4：抽取触发点——CLI 生成完成钩子，仅对 CLI-kind 生效

```
service.rs: GenerationEventHandler::completed()  ← API/CLI 共用的现有钩子
        │
        ├─ agent.launch().kind_str() != "cli" → 不做任何新事情（OnePiece 走它自己现有的机制）
        │
        └─ agent.launch().kind_str() == "cli"：
                │
                ▼
           ApiCredentialPort::fetch("onepiece")?.is_some()
                │
                ├─ false → 记 Warn（session.runtime.memory-extraction 或类似 category），跳过
                │
                └─ true → memory_extraction.extract(exchange, agent.id(), session.folder)
                                │
                                ├─ Ok(Some(content)) → memories.save(agent.id(), folder, content, MemorySource::Automatic)
                                ├─ Ok(None) → 无操作（这轮没有值得记住的）
                                └─ Err(error) → 记 Warn，跳过，不影响已经完成的 CLI 消息
```

这个钩子触发时 CLI 消息已经完成、已经交付给用户，抽取调用是纯异步的"事后加工"，失败不可能影响到已经展示给用户的内容。

### D5：CLI 消息发送路径新增记忆注入（读），顺序紧跟自定义指令之后

Phase 2 确立的 CLI effective prompt 组装：`[custom_instructions_block, prompt_hook_assembled]`。本变更插入记忆 section：`[custom_instructions_block, memory_block, prompt_hook_assembled]`——跟 OnePiece 现有顺序（`core → custom_instructions → skills → memory`，`agent-skill-injection` capability 的既定契约）保持相对次序一致的产品直觉：稳定的身份/偏好设定（自定义指令）优先级最高，会话派生的记忆其次，Prompt Hook 自己的动态内容和用户原始消息放最后。查询失败的降级策略镜像自定义指令：记 Warn，跳过记忆 section，不阻断发送。

### D6：读写解耦——凭据不可用只影响写，不影响读

D4 的凭据检查只发生在"要不要发起新的抽取调用"这一步。记忆注入（D5）只是查表，不需要任何凭据，未配置 OnePiece 的用户依然能在 CLI 里读到（其他 agent 产生的）已有记忆，只是不会再新增。这个不对称是有意的，不是遗漏。

### D7（已确认）：抽取节奏——先做"每轮都抽"，节流留给后续

不引入类似 OnePiece 压缩阈值那样的节流机制，CLI 每轮对话完成都触发一次抽取调用。**代价**：每条 CLI 消息完成后多一次模型调用，是持续的 API 开销。选择先做简单版本的理由：CLI 没有 VaneHub 可见的"压缩"信号可以复用（CLI 子进程自己管理上下文，对 VaneHub 不透明），构造一个全新的节流信号（比如按会话攒字符数）本身就是新工作，不确定值不值得在还没有真实使用数据之前投入。如果后续发现调用频率是真实问题，再加节流。

**实现落地时确认的细节**：CLI 抽取只受 `memoryEnabled` 主开关门控，不受 `memoryToolAssistedChatsEnabled` 子开关影响——该子开关的语义（"是否在有工具调用的会话中抽取"）是为 OnePiece 自己的压缩触发路径定义的，CLI 一侧没有"这轮有没有用到工具"这个可观察信号,套用同一个子开关没有意义。已同步更新 `personalization.memory.toolAssistedDesc` 文案明确这一点(任务 9),并修正了 spec.md 里一处曾错误暗示"两个开关都要看"的措辞(任务 10.1)。

### D8：管理面 UI 措辞需要更新，反映共享池语义

`agent-memory-section.tsx` 目前硬编码 `onePieceAgentId = "onepiece"` 读取/删除/重置记忆，i18n 文案里也没有提及"这是所有 agent 共享的"。存储模型改成共享池后（D1），这个面板实际展示/操作的是全部 agent 的记忆，`reset_agent_memories("onepiece")` 语义上会变成"清空所有 agent 的记忆"——如果不做任何 UI 调整，用户会误以为"重置"只清 OnePiece 的。至少需要：文案改为不特指 OnePiece（类似 Phase 2 对 `personalization.description` 的调整），重置按钮的二次确认文案需要更明确地说明"这会清空所有 agent（含 OnePiece 和四个 CLI）的记忆"。具体文案留给 tasks 阶段按 Phase 2 的既有模式（5 语言同步改）处理，不在这里逐条写死。

## Risks / Trade-offs

- **共享池的隐私/意外泄漏观感**——用户如果没意识到记忆现在是共享的，可能会对"CLI 里怎么知道我在 OnePiece 说过的话"感到意外。缓解：D8 的 UI 措辞调整 + 手动测试计划里加一条"确认共享行为符合预期"的用例。
- **CLI 每轮抽取的持续 API 开销（D7 的代价）**——已在 D7 讨论，接受此代价作为 MVP 范围，后续按真实数据决定要不要加节流。
- **共享池上线后的重复记忆**——同一件事可能被 OnePiece 和某个 CLI 各自抽取一次，产生内容相近的重复条目。去重明确列为 Non-Goal，不在本变更处理。
- **`AgentMemoryExtractionPort` 与现有 `extract_memories` 存在逻辑重叠**——两者都最终调用东西非常相似的 HTTP 请求逻辑（复用同一份 `summarize_turns`），但触发方式、凭据获取方式不同。需要在实现阶段确认代码复用的具体切法（抽公共函数 vs. 两条平行实现），不在设计阶段写死，留给任务阶段按实际代码形状决定。
- **排序依赖**——本变更假设 Phase 1、Phase 2 均已归档。`openspec validate` 不会拦这个（前两次变更已实测确认），但归档顺序必须是 Phase 1、Phase 2 都先于本变更。

## Migration Plan

纯增量，无破坏性 schema 变更：`agent_memories` 表结构不变（不需要新增列，`agent_id` 列继续写入，只是查询不再拿它做过滤条件）。行为变化仅限于"查询范围从单 agent 变成全部"，已有数据不需要回填或迁移脚本。新端口/新触发路径都是新增代码，不修改任何既有函数签名（除非任务阶段发现 `summarize_turns` 的复用需要调整其可见性）。

## Open Questions

无。D1、D2、D7 均已确认并按本文档描述实现（见上文各决策小节末尾的落地说明）；抽取节流、记忆去重、向量检索仍是明确的 Non-Goal（见 proposal.md），不在本变更范围内重新打开讨论。
