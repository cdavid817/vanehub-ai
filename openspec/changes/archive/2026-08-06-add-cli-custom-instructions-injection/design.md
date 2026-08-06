# Design: add-cli-custom-instructions-injection

## Context

个性化能力第二阶段。第一阶段（`add-personalization-settings`）把 OnePiece 的 Non-Goal 里说"四个 CLI 包装型 Agent 需要完全不同的注入机制,放到后续变更"——本设计的核心发现是这个假设不成立。

四个 CLI（Claude Code / OpenCode / Codex CLI / Gemini CLI）已经有一条现成、生产环境在跑的注入管线：

```
用户在 CLI 会话里发一条消息
        │
        ▼
agent.launch().kind_str() == "cli"？ ──否──▶ 原样发送（OnePiece，不受本变更影响）
        │ 是
        ▼
EffectivePromptGateway::assemble(agent_id, session_id, user_prompt)
        │
        ▼
tooling::prompt_hooks::assemble_prompt_work()：
  遍历所有 Prompt Hook（内置 7 个 + 用户自建）
  ├─ 未发布草稿 → 跳过　├─ 禁用 → 跳过　├─ 未绑定这个 CLI → 跳过
  └─ 其余：渲染模板（{{agent_id}}/{{current_time}}/{{sample_input}} 等变量）
        │
        ▼
[hook1 渲染结果, hook2 渲染结果, ..., 用户原始消息] 用 "\n\n" 拼接
        │
        ▼
GenerationProcessRequest.effective_prompt → 写进 PTY，真正发给 CLI 子进程
```

`native-runtime-architecture` spec 里已经有权威描述（"Native Prompt Hook pipeline" requirement）：`assemble` 产出"one effective prompt for the provider invocation builder"，provider-specific 的启动参数/stdin 交付本身仍归 provider invocation builder 所有——也就是说 Prompt Hook 管线自己也不直接碰 CLI 的启动参数，走的是"把内容并入用户输入文本"这唯一一种交付方式，四个 CLI 统一对待。这跟最初参考方案设想的"每个 CLI 走自己的原生机制"完全是两回事——现状是四个 CLI 早就统一走同一条低成本路径，而且已经被内置的"law-runtime-boundary"（不可禁用）验证过在生产里管用。

## Goals / Non-Goals

见 proposal.md。核心目标：把 Phase 1 的自定义指令设置接进这条已有管线,不新建注入机制,不碰 Prompt Hooks 内部。

## Decisions

### D1: 复用 Prompt Hook 的文本前置交付方式，不新建 `ContextInjector`

已与用户确认（方案 B）。四个 CLI 已经统一通过"把内容拼进发给 CLI 的文本"这一种方式接收 Prompt Hook 内容，自定义指令走同一种交付方式，不新增机制、不区分 CLI 类型。

### D2: 自定义指令的组合发生在 `agent_runtime` 侧，在 `assemble()` 调用之外——不把自定义指令存成一个 Prompt Hook

两个候选方案的取舍（已与用户讨论）：

| | 存成系统管理的 Prompt Hook | `agent_runtime` 侧独立拼接（选定） |
|---|---|---|
| 复用度 | 最高，不用碰 `assemble` 调用点 | 需要在 `service.rs` 加几行 |
| 内容生命周期匹配度 | 差——Prompt Hook 是 draft/publish/版本回滚的精心编写内容模型，用户在设置页改个"关于你"要走这一整套显然别扭 | 好——直接读直接生效，跟 `saveSetting` 的即时语义一致 |
| 管理 UI 干扰 | 需要在 Prompt Hook 管理列表里特殊隐藏这个系统条目 | 无——不出现在 Prompt Hook 管理页 |
| 对 `{{sample_input}}` 等模板变量的影响 | 无（如果实现得当） | 无——自定义指令在 `assemble()` 返回之后再拼接，不影响 Prompt Hook 自己看到的"用户原始消息" |

选择独立拼接：`AgentRuntimeApplicationPorts` 新增 `personalization` 端口字段（复用 Phase 1 的 `AgentPersonalizationPort`/`RuntimeAgentPersonalizationAdapter`），在 CLI 发送路径里，`assemble()` 返回后，把渲染好的自定义指令块拼在 `effective_prompt.content` **前面**：

```rust
let custom_instructions = personalization.settings()
    .ok()
    .and_then(|settings| settings.custom_instructions_block()); // D3 共享方法
let final_prompt = [custom_instructions, Some(effective_prompt.content)]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n\n");
```

失败处理镜像 Phase 1 既有哲学：`personalization.settings()` 读取失败 → 记 `Warn` 日志、当作没配置，不阻断消息发送（CLI 消息发送比 OnePiece 生成更不能容忍"因为个性化设置读取失败就整体失败"）。

### D3: 格式化逻辑上移到 `application` 层，OnePiece 与 CLI 共用一份

`format_custom_instructions_section`（style_rules 在前、about_user 在后、`## Custom Instructions` 标题）目前是 `infrastructure::api_process_adapter` 的私有函数。`application::service.rs`（CLI 路径）不能依赖 `infrastructure`（依赖方向只能反过来），所以这份逻辑需要挪到两边都能触达的层——`application::models.rs`，作为 `PersonalizationSettings` 的方法（如 `custom_instructions_block(&self) -> Option<String>`）。`api_process_adapter.rs` 里原来的私有函数改成委托给这个方法，OnePiece 侧行为、既有测试断言的确切文本不变。

**沿用同一套 markdown 格式，不为 CLI 单独设计文案**：现有 7 个内置 Prompt Hook 的模板都是朴素自然语言（"Respect the active VaneHub runtime..."），没有 markdown 标题；自定义指令用 `## Custom Instructions` / `### Response style` / `### About the user` 这种结构化标题风格，跟 Prompt Hook 的行文习惯不完全一致。但四个 CLI 都是面向代码/文档场景的编程助手，处理 markdown 结构化输入是其常态能力，不构成理解障碍；维护两套格式化规则（OnePiece 一套、CLI 一套）换来的一致性收益不值得为了风格统一而付出双重维护成本。沿用同一份格式化方法。

### D4: 组装顺序——自定义指令在 Prompt Hook 组装结果之前

```
[自定义指令块（可选）, Prompt Hook 组装出的 effective_prompt（含用户原始消息）]
```

CLI 场景没有 OnePiece 的 "core instructions" 那一层（OnePiece 场景顺序是 core → 自定义指令 → skills → memory）；自定义指令是这里语义上最"底层"的身份/偏好设定，排最前，Prompt Hook 自己的 law/static/dynamic/navigation/routing 内容以及用户原始消息跟在后面。

### D5: 每轮重复注入，不做"只在会话首条消息注入一次"

已与用户确认。理由：
1. VaneHub 不掌控四个 CLI 子进程内部如何维护自己的上下文，每轮重新注入不依赖"CLI 自己记得住上一轮内容"这个假设。
2. `PromptHookStage::SessionInit` 这个字段目前在 `assemble_prompt_work` 里完全没有被实际读取/强制执行——所有 enabled+bound 的 hook（不分 stage）都是每轮重新渲染。做"只注入一次"需要新建一套会话级状态追踪，这是货真价实的新工作，且会让自定义指令的行为与现有 7 个内置 hook 的实际行为不一致，制造两套心智模型。跟随现状,不新增范围。

### D6: 复用 Phase 1 的 `customInstructionsEnabled` 开关，不新增

一次配置，OnePiece + 四个 CLI 一起生效/一起关闭，匹配"适用于本机所有会话"的原始诉求，也避免用户要分别为 OnePiece 和"所有 CLI"两处开关操心。

## Risks / Trade-offs

- **每轮重复注入的 token/成本代价**——跟现有 7 个内置 hook（含不可禁用的 law-runtime-boundary）风险轮廓相同，不是本变更新引入的问题；自定义指令上限 2×3000=6000 字符，用户可见、可控（关掉开关或清空内容）。
- **排序依赖**——本变更的 specs delta 假设 Phase 1 已归档合并进主 specs。`openspec validate` 不会拦这个（已实测确认），但归档脚本在"MODIFIED 一个尚未在主 specs 里的能力"这种情形下的实际行为没有验证过，不应该冒险——必须保证 Phase 1 先于本变更归档。
- **Web/mock 覆盖范围未探明**——`web-agent-client.ts` 当前 CLI-kind 模拟会话是否已经模拟 Prompt Hook 组装本身还没确认（`sendMessage` 里没搜到明显的 assembly 模拟痕迹，但代码量大，需要任务阶段仔细读一遍再下结论）。如果目前压根没模拟 Prompt Hook 组装，那么"自定义指令的 Web mock"这部分范围会比预想大一点——需要先把 Prompt Hook 组装本身的 mock 补上，还是只加自定义指令这一层 mock、property Prompt Hook 部分维持现状，留给任务阶段研究后决定。

## Migration Plan

纯增量：新端口字段、新拼接逻辑、格式化方法搬家（行为不变，只是可见性从 `infrastructure` 私有变成 `application` 内可共享）。没有 schema 变更（自定义指令的存储早在 Phase 1 就建好了，本变更只是多一个读取方）。四个 CLI 在 `customInstructionsEnabled` 关闭或内容为空时，行为与现在完全一致。

## Open Questions

- ~~`web-agent-client.ts` 的 CLI-kind 模拟会话现在到底有没有模拟 Prompt Hook 组装~~——已在任务 1.2 确认：**没有**。`sendMessage` 是所有 agent（不分 CLI/API）共用的同一个桩函数，回复内容是写死的模板字符串（`` `Mock ${session.agentId} response: I received "..."` ``），从未模拟过"实际发给模型/CLI 的文本内容"这个概念——OnePiece 自己的自定义指令在 mock 里同样不会让回复内容产生可观察差异。这不是本变更引入的新缺口，是这套 mock 一直以来的既有边界（它模拟的是"事件序列"如 tool_use/rich_block，不模拟"提示词内容"）。

  **结论**：本变更在前端没有可观察行为需要新增 mock。`add-personalization-settings` 里写的"Web mock 要模拟 CLI 侧的自定义指令前置行为"这条 spec 场景已经在本文件同步修正——原场景对应的行为不存在实现的意义（没有任何东西可观察地"没模拟"，因为压根没有可观察的提示词内容分支）。任务 5 相应收窄为"确认无需新增前端代码"，不是实现空白。
