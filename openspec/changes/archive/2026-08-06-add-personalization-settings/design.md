# Design: add-personalization-settings

## Context

这是「个性化」能力的第一阶段，范围收窄到 OnePiece——当前代码库里唯一具备 `resolve_system_prompt` 系统提示词组装管线的 Agent（`api_process_adapter.rs:769`，直接 HTTP 调用 Anthropic/OpenAI-compatible 接口，没有子进程、没有启动参数、没有 PTY）。四个 CLI 包装型 Agent（Claude Code / OpenCode / Codex CLI / Gemini CLI）走完全不同的子进程机制，是独立的后续阶段（见 proposal.md Non-Goals）。

记忆能力本身（`add-agent-cross-session-memory`，2026-08-01 归档）已完整实现：`agent_memories` 表、`remember` 工具（`AutoApprove`）、`maybe_compact` 触发时的 `extract_memories` 自动抽取、`resolve_system_prompt` 里的 `## Memory` section 注入、`list_agent_memories`/`delete_agent_memory` 命令。本阶段是在这套已跑通的机制上加控制面，不是重新实现。

今天的 `feat: add native OnePiece agent and provider configuration (#83)` 把承载记忆列表/删除的 `agent-memory-panel.tsx` 连同旧版多 Agent 设置页一起删除了——后端命令还在，前端入口没了。这是一个需要在本变更里顺带修复的真实回归，不是假设性风险。

最初有一份参考方案（`add-custom-instructions`，来自本机 Downloads 目录）为此设想了一个新的限界上下文 `personalization` 和一套面向"CLI 子进程"的 `ContextInjector` 三级降级机制。评估后发现它是为一个不存在的架构画的图——那套注入机制精确匹配的是 CLI 包装型 Agent 的问题（留给后续阶段复用其思路），但 OnePiece 根本不是子进程，套用会让 OnePiece 多走一层不必要的抽象，而真正的落点（`resolve_system_prompt` 里加一个 section）反而被漏掉。本设计保留该参考方案里站得住的产品决定（两字段、3000 字符上限、style_rules 优先于 about_user 的组装顺序），重新设计了架构落点。

## Goals / Non-Goals

**Goals：**
- 主机级自定义指令一次配置，对 OnePiece 所有会话生效，无需重启应用。
- 记忆的生成（`remember` 工具 + 自动抽取）与使用（系统提示词注入）可以被用户整体关闭，且能按"是否工具辅助会话"细分控制自动抽取。
- 记忆可以被批量重置，管理入口（列表/删除/重置）重新可达。
- 不引入新的限界上下文；复用 `desktop` 已有的设置聚合与 `agent_runtime` 已有的记忆聚合。

**Non-Goals：** 见 proposal.md（CLI Agent 注入、指令内容 LLM 校验、按 workspace/Agent 差异化、记忆向量检索）。

## Decisions

### D1: 自定义指令 + 记忆偏好开关归 `desktop` 所有，不新建限界上下文

`desktop` 已经有一个成熟、被广泛复用的 key/value 设置聚合（`DesktopSettings`/`DesktopSettingKey`/`DesktopSettingMutation`，`domain/settings.rs`），底层是一张纯 `settings(key TEXT, value TEXT)` 表（`sqlite_settings_repository.rs:26`：`SELECT key, value FROM settings`）——新增设置项**不需要任何 schema 迁移**，只需要给 `DesktopSettingKey` 加新变体。这和自定义指令/记忆开关"主机级、一台机器一份"的语义完全吻合。

否决新建 `personalization` context：这套设置没有独立的生命周期、没有独立的领域不变式需要保护，本质上就是几个新的 app 设置项；新建一个限界上下文只会复制 `desktop` 已经解决过的问题（key/value 校验、持久化、Web/mock 双实现），却拿不到额外的边界收益。

### D2: 新跨 context 端口 `AgentPersonalizationPort`（`agent_runtime → desktop`）

```rust
// src-tauri/src/contexts/agent_runtime/application/ports.rs
pub(crate) struct PersonalizationSettings {
    pub(crate) custom_instructions_about_user: String,
    pub(crate) custom_instructions_style_rules: String,
    pub(crate) custom_instructions_enabled: bool,
    pub(crate) memory_enabled: bool,
    pub(crate) memory_tool_assisted_chats_enabled: bool,
}

pub(crate) trait AgentPersonalizationPort: Send + Sync {
    fn settings(&self) -> Result<PersonalizationSettings, AgentRuntimeApplicationError>;
}
```

一次读取、五个字段，而不是拆成多个端口方法——底层就是 `desktop::api::DesktopSettingsApi::get_settings()` 的一次调用，拆分只会制造多次跨 context 调用而没有收益。实现方（`infrastructure` 里的 `RuntimeAgentPersonalizationAdapter`）直接持有 `desktop::api::DesktopSettingsApi`，镜像 `AgentSkillPort`/`RuntimeAgentSkillAdapter` 桥接 `tooling::skills` 的既有模式（`ports.rs:780` 附近注释："matching every other cross-context dependency in this module"）。

**这是 `agent_runtime` 第一次依赖 `desktop`**（此前二者之间没有调用关系）。方向上没有问题——`desktop` 不会反向依赖 `agent_runtime`——但这是依赖图里的新边，实现前需要在 PR/review 里显式确认，而不是当作既有模式的又一次复用来处理。

失败处理镜像 `resolve_system_prompt` 里 Skill/Memory 现有的降级哲学：读取失败 → 记一条 `Warn` 日志、当作"无个性化设置"处理，绝不让个性化设置的读取失败拖垮一次生成。

### D3: 系统提示词组装顺序——core → 自定义指令 → Skills → memory

`resolve_system_prompt` 现有顺序是 `[core_section, skill_section, memory_section]`（`api_process_adapter.rs:856`），`agent-skill-injection` capability 的 "Deterministic API system-prompt section ordering" requirement 是这个顺序的规范来源。新顺序改为 `[core_section, custom_instructions_section, skill_section, memory_section]`：

- Core 仍然最先——它是不可移除、不受用户控制的身份/安全边界（`onepiece-native-agent` 的 "Versioned OnePiece core instructions" requirement）。
- 自定义指令紧随其后——用户主动、稳定配置的偏好，优先级应该高于动态绑定的 Skills 和会话派生的 memory。
- Skills、memory 顺序不变。

`custom_instructions_section` 内部沿用参考方案的决定：`style_rules` 在前、`about_user` 在后（风格约束是对输出的横切要求，前置获得更高遵循优先级；关于你是背景事实，后置不受影响）。`customInstructionsEnabled = false` 或两个字段都为空时，整个 section 省略，和 Skill/Memory 现有的"空则省略"行为一致。

### D4: 记忆主开关是三件事的总闸，不是只影响生成

`memoryEnabled = false` 同时停用：`remember` 工具调度（`execute_tool_call`）、`maybe_compact` 触发的自动抽取（`extract_memories`）、`resolve_system_prompt` 里的 memory section 注入。对应你原始需求里的 2.1（"生成新记忆，并将其带入新聊天"这句话本身就绑定了生成和注入两件事）——不是只停止写入、留着旧记忆继续读，而是完全对称：关闭之后这次会话既不产生新记忆，也不使用旧记忆，行为上等价于这个 Agent 从未有过记忆能力。

默认值 `true`：三天前上线时记忆是无条件开启的，新开关默认开启才能保证现有用户升级后行为不变（纯增量，不是破坏性变更）。

### D5: 工具辅助会话子开关只影响自动抽取，不影响 `remember` 工具

`memoryToolAssistedChatsEnabled = false` 时，如果本次触发压缩的会话在被压缩的 turns 里包含过工具调用（shell/file/MCP 的 `ToolUseBlock`），`extract_memories` 直接跳过，不发起抽取调用。`remember` 工具调用不受此开关影响——用户显式说"记住……"是主动行为，语义上和被动的自动抽取不是一回事，不应该因为聊天里用过工具就被拦截。

**结论（任务 1.3 确认）**：检测信号取 `AgentMessage.tool_use: Vec<ToolUseBlock>`（`application/models.rs:411`），而不是 `maybe_compact`/`extract_memories` 操作的 `turns: &mut Vec<Value>`（`api_process_adapter.rs:944`，provider-specific wire-format JSON，Anthropic 与 OpenAI-compatible 两种格式里工具调用的表示方式不同，解析成本高且脆弱）。`execute()` 里 `history.recent_messages(...)` 取到的 `recent: Vec<AgentMessage>` 在转换成 `turns`（`wire_format.history_to_turns(&recent)`）之前就已经可用——一次性计算 `let tool_assisted = recent.iter().any(|message| !message.tool_use.is_empty());`，把这个 `bool` 一路传进 `maybe_compact`/`extract_memories`，不需要在两者之间做逐条对齐。

这也回避了另一个问题：`turns[..split_at]` 和 `recent` 之间不保证一一对应（`history_to_turns` 可能把一条 `AgentMessage` 展开成多个 turn），如果改成"只看被压缩的那一段"，需要先解决 `recent` 与 `turns` 的索引对齐问题；直接用"整段会话历史里是否出现过工具调用"作为信号，语义上也更贴合"工具辅助聊天"这个描述本身指的是会话的属性，而不是被压缩掉的那几条消息的属性，天然避开了对齐问题。

默认值 `true`：同 D4 的理由，保证现有用户升级后自动抽取行为不变。

### D6: 批量重置是物理删除，前端要求二次确认

`AgentMemoryPort` 新增 `delete_all_for_agent(agent_id) -> Result<(), _>`，`SqliteAgentMemoryRepository` 用一条 `DELETE FROM agent_memories WHERE agent_id = ?1` 实现（不是软删除/标记）。新 Tauri command `reset_agent_memories`。前端沿用这个代码库里 `ssh-connections-page.tsx`/`mcp-page.tsx` 已有的"`window.confirm` + mutation"删除确认模式，重置动作额外要求确认文案明确说明"不可撤销"。

### D7: UI 落点——新的独立「个性化」设置页，不塞进 Agent 配置页

`agent-configurations-page.tsx` 现在是"按 Agent 切标签"的结构（OnePiece / claude-code / opencode / codex-cli），主题是每个 Agent 的 provider/凭据配置。自定义指令和记忆开关是主机级、和"配置某一个 Agent 怎么连接哪个 provider"是不同维度的关注点——参考 ChatGPT Desktop 把 Personalization 和 Model/Connectors 分开设置的做法，新增一个独立的「个性化」设置页（`settings-pages.ts` 新增 `"personalization"` 页面 id），恢复的记忆管理面板也放在这里而不是原来的 Agent 详情列。

`settings-center-ui` 的 "UCD settings pages" requirement 需要同步更新已枚举的导航条目列表，并保持 "about 是最后一项" 这条既有不变式不被破坏——新条目放在 Agent 配置/Skills 附近（个性化设置和 Agent 行为强相关），About 仍然殿后。

### D8: 默认值一览

| 设置键 | 默认值 | 理由 |
|---|---|---|
| `customInstructionsAboutUser` | `""` | 新功能，无历史数据 |
| `customInstructionsStyleRules` | `""` | 同上 |
| `customInstructionsEnabled` | `true` | 内容为空时 section 本来就省略，开着也是零开销；开着可以让用户填完直接生效，不用多一步找开关 |
| `memoryEnabled` | `true` | 保留现有"始终开启"行为（D4） |
| `memoryToolAssistedChatsEnabled` | `true` | 保留现有"始终开启"行为（D5） |

## Risks / Trade-offs

- **~~`tool_assisted_session` 生成内失真~~（已修复）**——实现后自查发现的真实 bug，不是 D5 讨论过的"跨生成粗粒度"那种可接受简化：`execute()` 里 `tool_assisted_session` 最初只在生成开始时算一次，工具执行循环内第二次 `maybe_compact` 调用复用的还是这个旧值，导致"会话本次生成里刚发生的工具调用"侦测不到——一个从没用过工具的会话，如果本次生成自己调用了工具又恰好在同一次生成内触发压缩，子开关会被错误绕过。修复：`tool_assisted_session` 改为 `mut`，工具轮次执行后若 `executed` 非空就置 `true`，跨生成的"曾经用过工具就一直生效"这条粗粒度简化本身保留不变，只是同一次生成内的时序问题被修正了。`api_process_adapter.rs` 新增 `tool_assisted_flag_reflects_a_tool_call_made_earlier_in_the_same_generation` 回归测试，并通过临时撤销修复验证过该测试确实会失败。
- **新跨 context 依赖需要显式 review**——`agent_runtime → desktop` 是新边，虽然方向没问题，但要在实现 PR 里明确标注，避免以后有人误以为这是"一直都有"的既有模式。
- **三个 system-prompt 来源的字符预算叠加**——自定义指令（至多 2×3000 字符）+ Skills（16,000 字符聚合上限）+ Memory（4,000 字符预算）理论上限接近 25,000 字符。目前没有跨 section 的总预算控制，各 section 独立裁剪。暂不处理——现有 Skill/Memory 预算已经是独立控制，自定义指令只是加了一个数量级相近的新来源，不改变现有风险轮廓；如果实践中出现请求体过大的问题，再引入总预算裁剪。
- **工具辅助检测信号的粒度**——D5 确认后的信号是"整段拉取到的会话历史里是否出现过工具调用"，不是"只看这次被压缩掉的那一小段"。意味着一个会话只要用过一次工具，之后即便触发多次压缩，工具辅助子开关都会持续生效，不会随着旧 turns 被压缩掉而"失效"——这是比按 turns 精确对齐更宽松、但更稳定的行为，设计上认为可接受。
- **批量重置不可逆**——D6 用前端二次确认覆盖，没有软删除/回收站，和这个代码库里其它"删除"操作的既有严格程度一致（不是本变更引入的新风险等级）。
- **归档顺序是硬依赖，本变更自己的 spec 断言已经被后续变更的已实现代码推翻**（`openspec verify` 独立核对时发现）：本变更「Memory management」「Memory injection into the system prompt」两个 requirement 断言的按 agent 隔离保证（"reset 只影响一个 agent"、"注入的记忆限定在当前 agent"），已经被 `add-cli-memory-support`（共享记忆池，把隔离整体反转）实现的代码推翻——那份代码已经合并进同一个工作树、已经在跑，只是它自己的 spec delta 还没有被归档。`add-cli-memory-support` 自己的 spec delta 已经正确地用 MODIFIED 反转了这些保证，问题只在于：如果本变更单独归档、且归档后到 `add-cli-memory-support` 归档之间存在任何时间差，主 spec 会在那段时间里断言一个当前代码已经不成立的保证。**归档本变更时必须在同一操作序列内紧接着归档 `add-cli-memory-support`，不能中间停顿**（见 tasks.md 10.9）。

## Migration Plan

纯增量，无 SQLite schema 变更：
- `desktop` 侧新增 5 个 `DesktopSettingKey` 变体，读取不到时用 D8 的默认值，不需要迁移脚本、不需要回填。
- `agent_runtime` 侧 `agent_memories` 表结构不变，只新增一个 `DELETE` 查询方法，无需迁移。
- 回滚：移除新枚举变体和新 command 即可，不留孤儿数据、不影响 `settings`/`agent_memories` 表里其它既有数据。

## Open Questions

- ~~工具辅助会话的检测信号具体读哪个类型/字段~~——已在任务 1.3 确认，见 D5。
- 「个性化」设置页在导航列表里的精确位置（Agent 配置之后、Skills 之前，还是反过来）——UI 细节，实现时按现有视觉密度确定，不是架构决策。
