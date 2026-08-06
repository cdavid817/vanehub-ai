# Tasks: add-cli-memory-support

## 1. 研究与前置条件

- [x] 1.1 确认：`openspec list --json` 显示 `add-personalization-settings`（50/52）与 `add-cli-custom-instructions-injection`（18/19）均未归档，但都不阻塞本变更编码/测试，只阻塞最终归档（本任务本身的 done 标记的是"已确认状态并记录"，不是"已满足前置条件"——归档前仍需重新检查一次任务 11.5）。
- [x] 1.2 **决定**：不整体搬迁，原地放宽可见性。`summarize_turns`/`wire_format_for`/`WireFormat`/`MEMORY_EXTRACTION_INSTRUCTION`/`REQUEST_TIMEOUT` 都改成 `pub(crate)`（`WireFormat` 的字段保持私有——新增网关只需要把这个类型当不透明值透传，不需要构造/解构它的字段），新增的抽取网关文件通过 `use super::api_process_adapter::{...}` 直接调用。`blocking_http_client` 已经是 `pub(crate)`（`platform::network::proxy.rs:97`），不需要改。额外发现：`extract_memories` 现有逻辑是把 `summarize_turns` 的返回值按行拆分、每个非空行各自存一条记忆——新抽取路径的调用方（`service.rs` 新的 CLI 完成钩子）需要复刻同一个"按行拆分"逻辑，不是端口本身内置这个语义。
- [x] 1.3 已确认 `web-agent-client.ts` 现状：`hadExistingMemoriesForScope`（约 3131-3132 行）判断逻辑还是旧的按 `agentId`+`folder` 严格匹配，需要在任务 10 一并改成"共享池里有没有任何记忆"；这个"记忆注入"事件本身已经是不分 agent kind 无条件触发的（不需要额外加 CLI 分支）。"抽取"事件（约 3100-3129 行）和"remember 工具调用"事件（约 3263-3286 行）都在 `if (agent?.launch.kind === "api")` 分支内，且抽取事件额外绑定在"长对话触发压缩"这个模拟条件上——CLI 需要一条独立的新分支（每轮完成都模拟一次抽取，不绑定压缩条件），remember 工具调用没有 CLI 等价物（不需要新增，CLI 只有抽取，没有显式保存）。

## 2. 存储层：`agent_memories` 查询模型改造（不改表结构）

- [x] 2.1 `memory_repository.rs`：`list(agent_id, folder)` 与 `list_all_for_agent(agent_id)` 合并为单一 `list_all() -> Vec<AgentMemory>`，SQL 去掉 `WHERE agent_id = ?1`（`list` 原有的 `folder` 过滤一并去掉，改为纯 `ORDER BY created_at DESC`）。`save` 签名不变，继续记录 `agent_id`/`folder` 作溯源。`delete_all_for_agent(agent_id)` 改为 `delete_all() -> Result<(), _>`，SQL 变成无条件 `DELETE FROM agent_memories`。`delete(memory_id)` 不变。
- [x] 2.2 **决定**：新增一条独立的、带版本号的迁移（`schema_migrations` version 42,`agent-memory-shared-pool`），不是回头改写已经跑过的 `apply_memory_schema`（那是 version 31,已迁移过的库不会再执行它）。新函数 `apply_memory_shared_pool_schema`：`DROP INDEX IF EXISTS idx_agent_memories_scope` + `CREATE INDEX IF NOT EXISTS idx_agent_memories_recency ON agent_memories(created_at DESC)`,注册进 `migrations.rs` 的 `apply_migration(conn, 42, ...)`。
- [x] 2.3 单元测试：`list_all_spans_every_agent_and_every_folder`（构造 my-agent/other-agent 两个 agent 的记忆，确认都被返回）；`delete_all_removes_every_memory_across_every_agent`；`save_records_provenance_and_list_all_returns_it`（溯源字段不丢）；`shared_pool_schema_is_idempotent_and_replaces_the_scoped_index`（新迁移幂等，旧索引确实被删、新索引确实被建）。全部通过。

## 3. `AgentMemoryPort` 端口签名简化

- [x] 3.1 `application/ports.rs`：`AgentMemoryPort` trait 方法改为 `save(agent_id, folder, content, source)`（不变）、`list_all() -> Result<Vec<AgentMemory>, _>`（替换原 `list`+`list_all_for_agent`）、`delete(memory_id)`（不变）、`delete_all() -> Result<(), _>`（替换 `delete_all_for_agent`）。
- [x] 3.2 全部既有调用点已更新：`resolve_system_prompt` 的 `memories.list(agent_id, folder)` → `memories.list_all()`；`extract_memories`/`remember` 工具的 `memories.save` 调用签名不变；`commands/agent_runtime/list_agent_memories.rs`/`reset_agent_memories.rs` 两个命令改用新签名（见任务 7）。
- [x] 3.3 `application/tests.rs` 的 `FakeWorld`、`api_process_adapter.rs` 测试模块的 `FakeMemories`/`PanicsOnListMemories`：均已同步实现新签名，`cargo check --lib --tests` 干净，`cargo test --lib memory` 20 项全过、无回归（含 `resolve_system_prompt_omits_memory_section_and_skips_the_lookup_when_memory_is_disabled`，`PanicsOnListMemories` 的 panic 断言点从 `list` 移到 `list_all`，仍然精确验证"关闭时不查库"）。

## 4. 新增独立的记忆抽取能力

- [x] 4.1 新增 `AgentMemoryExtractionPort`（`application/ports.rs`）：`fn extract(&self, exchange: &str) -> Result<Option<String>, AgentRuntimeApplicationError>`。**跟最初设计有一处出入**：去掉了 `agent_id`/`folder` 两个参数——抽取调用本身不需要它们（只需要 OnePiece 凭据 + 文本），它们只在保存时才用得上，而保存这一步按设计放在调用方（`service.rs`），调用方已经有 `agent.id()`/`session.folder` 可用，不需要经这个端口传一趟。
- [x] 4.2 新增 `RuntimeAgentMemoryExtractionAdapter`（`infrastructure/memory_extraction_gateway.rs`）：`credentials.fetch("onepiece")` → 缺失返回 `AgentRuntimeApplicationError::Credential`；`config.provider_config("onepiece")` → 缺失同样返回 `Credential`；`wire_format_for(...)` 失败或 `summarize_turns` 调用失败 → 返回 `AgentRuntimeApplicationError::Memory`（复用既有错误枚举的两个变体做区分，不新增错误类型）。`summarize_turns`/`wire_format_for`/`MEMORY_EXTRACTION_INSTRUCTION`/`REQUEST_TIMEOUT`/`WireFormat` 按任务 1.2 的决定改成 `pub(crate)`，原地复用。
- [x] 4.3 单元测试（7 项，均通过）：凭据缺失 → `Credential` 错误；provider 未配置 → `Credential` 错误；调用成功且有内容 → `Ok(Some(content))`；调用成功但没有值得记住的内容 → `Ok(None)`；调用本身失败（HTTP 500）→ `Memory` 错误；专门验证两类错误互不混淆的判别性测试。用了一个精简版本地 HTTP fixture（没有跨文件复用 `api_process_adapter.rs` 测试模块内的私有 fixture helper，那些函数没有导出，跨文件共享需要额外改动，权衡后选择在新文件里自带一份小而完整的 fixture）。

## 5. CLI 发送路径：记忆注入（读）

- [x] 5.1 `application/service.rs` 的 CLI 分支：`personalization.settings()` 只查一次（同时供自定义指令与记忆开关判断使用，查询失败统一降级为 `PersonalizationSettings::safe_fallback()`，不是两处各查各的），在自定义指令拼接之后、Prompt Hook 组装结果之前新增记忆 section。读取 `self.ports.memories.list_all()`，`memory_enabled` 关闭或结果为空则跳过（不产生空 section）。格式化逻辑复用 `application::format_memory_section`（任务 5 顺带把这个函数从 `api_process_adapter.rs` 移到 `application/models.rs`，OnePiece 侧改成一行委托，镜像自定义指令格式化逻辑上移的先例）。
- [x] 5.2 查询失败时的降级：记 Warn 日志（category `session.runtime.memory`），跳过记忆 section，不阻断 CLI 消息发送。
- [x] 5.3 测试（6 项，均通过）：记忆非空且开关开启时正确插入（`send_message_prepends_memory_for_cli_agents_when_enabled_and_present`）；开关关闭时不影响 CLI 发送内容（`send_message_omits_memory_for_cli_agents_when_disabled`）；记忆池为空时不产生空 section（`send_message_omits_memory_for_cli_agents_when_the_pool_is_empty`）；`memories.list_all()` 失败时降级、CLI 消息仍正常发送且记录 Warn 日志（`send_message_degrades_gracefully_when_memory_lookup_fails_for_cli_agents`）；完整验证 `[custom_instructions, memory, prompt_hook_assembled]` 三段顺序（`send_message_orders_memory_after_custom_instructions_and_before_prompt_hook_output_for_cli_agents`）。**没有**额外写"跨多个 CLI agent id 都适用"的重复测试——这条注入逻辑跟自定义指令共用同一个只按 `launch().kind_str() == "cli"` 判断的分支，Phase 2 的 `send_message_prepends_custom_instructions_for_any_cli_kind_agent_not_just_one` 已经证明这个分支本身不绑定具体 agent id，再写一遍是重复覆盖。

## 6. CLI 完成钩子：记忆抽取触发（写）

- [x] 6.1 `GenerationEventHandler`/`GenerationEventHandlerInput` 新增三个字段：`is_cli_kind: bool`（构造时算好 `agent.launch().kind_str() == "cli"`，不在 `completed()` 里重新判断）、`folder: Option<String>`、`user_prompt: String`（构造点在 `send_message_internal` 里，`prompt`/`session.folder`/`agent.launch()` 当时都在作用域内）。`complete_claimed` 末尾（已完成消息交付之后）新增 `if self.is_cli_kind { self.extract_and_save_memory(&response); }`，新增私有方法 `extract_and_save_memory`：查 `personalization.settings().memory_enabled`（关闭则直接返回），拼 `exchange = "User: {user_prompt}\n\nAssistant: {response}"`，调用 `self.ports.memory_extraction.extract(&exchange)`。`AgentRuntimeApplicationPorts` 新增 `memory_extraction: Arc<dyn AgentMemoryExtractionPort>` 字段，`bootstrap/agent_runtime.rs` 装配新的 `RuntimeAgentMemoryExtractionAdapter::new(api_credentials, repository)`（复用既有的 `api_credentials`/`repository` 实例，不新建）。
- [x] 6.2 降级路径按错误类型分流：`Err(Credential(_))` → 记 Warn,措辞"OnePiece 没有可用凭据,跳过";`Err(Memory(_))`（含 `wire_format_for`/HTTP 调用失败）→ 记 Warn,措辞"抽取调用失败,跳过";两者都用新增的 `record_memory_extraction_log` 辅助方法,category 固定为 `session.runtime.memory-extraction`（区别于任务 5 读取失败用的 `session.runtime.memory`,这两个是不同的失败面,分开更容易在日志里区分"读注入失败"和"写抽取失败"）。`Ok(Some(content))` 按行拆分,每个非空行单独 `memories.save(...)`（镜像 `extract_memories` 现有的"一行一条记忆"约定，没有复用 OnePiece 那份代码，因为触发路径整个不一样，只是约定一致）；`Ok(None)` 无操作。
- [x] 6.3 **确认**：抽取调用被放在 `complete_claimed` 的最后——`message_completions.deliver(...)`、`events.publish(AgentEvent::MessageCompleted)`、生命周期更新等所有用户可见的完成动作都已经在它之前执行完。抽取调用本身仍是同步的（在同一个后台监控线程上跑，不是另起线程/异步任务），所以严格来说"这次 completed() 调用整体耗时"会变长，但这不影响任何用户可见的完成时机——`monitor_generation` 是每个生成独立起的后台线程（`process_adapter.rs` 里 `thread::spawn`），拖慢的只是这一个线程自己的收尾时间，不阻塞其它会话、不阻塞 UI。视为满足"不阻塞交付"的实质要求，技术原因已如实记录，不引入额外的异步任务调度。
- [x] 6.4 测试（5 项，均通过）：开关开启且凭据存在时触发抽取并正确保存记忆，`exchange` 参数里能看到用户消息内容（`generation_completed_triggers_memory_extraction_for_cli_agents_when_enabled_and_credential_available`）；开关关闭时不触发（`generation_completed_skips_memory_extraction_for_cli_agents_when_memory_is_disabled`）；凭据不可用时不触发但记录 Warn 且不影响已完成消息（`generation_completed_degrades_gracefully_without_a_usable_onepiece_credential`）；抽取调用本身失败时同样降级（`generation_completed_degrades_gracefully_when_the_extraction_call_itself_fails`）；OnePiece（API-kind）生成完成时不触发这条新逻辑，回归验证（`generation_completed_does_not_trigger_memory_extraction_for_non_cli_agents`）。测试通过直接从 `world.generation_sinks` 取出构造好的 sink、手动 `sink.handle(GenerationProcessEvent::Completed(None))` 来驱动完成路径，这是这个文件里驱动生成完成的既有测试模式。

## 7. Tauri command 层：管理命令签名简化

- [x] 7.1 `commands/agent_runtime/list_agent_memories.rs`：命令签名去掉 `agent_id` 参数（**决定**：保留命令外部名字 `list_agent_memories` 不变，只去参数——契约测试明确叫"keep_stable_names"，命令改名的连带影响面比这次改动本身要大，不属于本次范围），改为无参调用 `api.list_all_memories()`。
- [x] 7.2 `commands/agent_runtime/reset_agent_memories.rs`：同样保留命令名，去掉 `agent_id` 参数，改为无参调用 `api.reset_all_memories()`。`delete_agent_memory`（按 memory id）不受影响。
- [x] 7.3 `AgentRuntimeApi`/`AgentRuntimeApplicationService` 对应的 facade 方法已改签名并改名（`list_agent_memories`→`list_all_memories`、`reset_agent_memories`→`reset_all_memories`，纯 Rust 内部命名，不影响 Tauri command 外部契约）。
- [x] 7.4 任务 11 统一跑 `contract_tests` 确认命令注册契约测试仍然通过——`contract_tests::agent_runtime_command_registration_and_frontend_invokes_keep_stable_names` 在全量 `cargo test` 中通过，符合预期（只检查名字匹配，参数减少不受影响）。

## 8. 前端：service 接口同步

- [x] 8.1 `src/services/agent-service.ts`：`listAgentMemories(agentId)`/`resetAgentMemories(agentId)` 改为无参 `listAllMemories()`/`resetAllMemories()`（保留 `deleteAgentMemory(memoryId)` 不变）。
- [x] 8.2 `tauri-agent-client.ts`：对应 `invoke(...)` 调用去掉 `agentId` 参数。
- [x] 8.3 `web-agent-client.ts`：`listAgentMemories`/`resetAgentMemories` 的 mock 实现改为操作全部 `webAgentMemories`（不再按 `agentId` 过滤/清空）。
- [x] 8.4 `test/render.tsx` 的 `createAgentServiceDouble`（Proxy 模式）：确认接口签名变化后测试双不需要额外改动（Proxy 对新签名天然兼容）——已核实 `render.tsx` 不含任何硬编码方法名引用，验证符合预期，无需改动。

## 9. 前端：UI 文案更新

- [x] 9.1 `agent-memory-section.tsx`：去掉硬编码的 `onePieceAgentId = "onepiece"`，改为调用无参的 `listAllMemories()`/`resetAllMemories()`。记忆列表每一项展示改为包含"来源 agent"信息（图标或文字标签，复用 `memory.agentId` 溯源字段），让用户能区分某条记忆是哪个 agent 产生的——已用 `<Badge tone="muted">{memory.agentId}</Badge>` 实现。
- [x] 9.2 i18n 文案（5 语言：zh-CN/en/zh-TW/ja/ko）：`personalization.memory.description`、重置按钮的二次确认文案（`personalization.memory.confirmReset`）需要明确"这会清空所有 agent（含 OnePiece 和四个 CLI）共享的记忆"，不能再读起来像"只清 OnePiece 的"。参考 Phase 2 处理 `personalization.description`/`personalization.customInstructions.description` 的同类改法。已确认 zh-CN 文案："删除所有已保存的记忆（OnePiece 和全部 CLI Agent 共享）？此操作不可撤销。"，其余 4 语言同步。
- [x] 9.3 前端测试：`agent-memory-section.test.tsx` 同步更新——不再需要按 agentId mock 服务调用参数；新增"记忆项展示来源 agent"的渲染断言（`onepiece`/`codex-cli` 双 agent 混合列表，断言两个 agentId 徽章均渲染）。

## 10. Web/mock：CLI 侧记忆事件模拟

- [x] 10.1 `web-agent-client.ts` 的 `sendMessage`：为 CLI-kind 模拟会话（当前只有 `shell`/MCP 工具事件）新增记忆相关模拟——生成完成后，若 `memoryEnabled` 开启,模拟一次"抽取"事件（`rich_block`，镜像 API-kind 现有的 "Memory extracted" 卡片），并把结果写入 `webAgentMemories`（`agentId` 记录为该 CLI 的 id，`source: "automatic"`）。实现时发现并修正了 spec.md 里一处与已实现的 Rust 后端（任务 6）、tasks.md 本条、以及已定稿的 `toolAssistedDesc` 文案不一致的措辞（"both applicable toggles allow it"）——CLI 抽取只看 `memoryEnabled` 主开关，不受 `memoryToolAssistedChatsEnabled` 子开关影响，且不设字符数门槛（每轮都抽,对应 D3 MVP）,已同步更新 `specs/agent-cross-session-memory/spec.md` 的三处相关场景。
- [x] 10.2 CLI-kind 会话若 `webAgentMemories` 里已有内容（不论来源 agent），比照现有判断方式（去掉 agent/folder 过滤，改成"池子里有没有东西"，变量重命名为 `hadExistingMemories`），模拟一次"Memory applied"事件。确认此事件本就不区分 agent kind 触发,只需修正判断条件本身,不需要新增 CLI 专属分支。
- [x] 10.3 测试：`web-agent-client.test.ts` 新增 4 个用例——CLI-kind 抽取事件在 `memoryEnabled` 开启时产生且不需要长度门槛、`memoryEnabled` 关闭时不产生、`memoryToolAssistedChatsEnabled` 关闭时依然产生（证明不受子开关影响）、API agent 存的记忆之后在 CLI 会话里能看到"Memory applied"事件（共享池在 mock 层可观察）。顺带修复了一个因 CLI 抽取卡片现在默认触发而断言过时的既有测试（`stores messages and emits mock streaming events`，codex-cli 场景的 richBlocks 从 `["card","checklist"]` 变为 `["card","card","checklist"]`）。59/59 通过，`npx tsc --noEmit` 干净。

## 11. 测试与验证

- [x] 11.1 `cargo fmt --check`、`cargo check --lib --tests`、`cargo clippy --all-targets -- -D warnings` 全部干净（fmt 首次运行发现本阶段新写的三个文件未格式化，已用 `cargo fmt` 自动修正，纯机械改动）。`cargo test` 首次全量运行 1274 passed / 3 failed，3 个失败全部在 `contexts::tooling::mcp::infrastructure::relay*`（`http_routing_forwards_json_rpc_and_refuses_redirects` 等），与本变更的 `agent_runtime`/personalization 代码无关——已核对是本仓库已记录在案的已知 flaky 测试族（socket 时序竞争）；隔离单线程重跑其中两个（`relay_stdio::tests::*`）立即通过；`relay::tests::http_routing_forwards_json_rpc_and_refuses_redirects` 连续 4 次单独重跑仍 3 次失败、1 次通过，且每次失败的具体报错都不一样，确认是测试自身内部的竞态而非本次改动引入的回归（已补充记录到相关 memory）。额外单独跑 `cargo test --lib memory` 35/35 全过，`contract_tests::agent_runtime_command_registration_and_frontend_invokes_keep_stable_names` 通过（对应任务 7.4）。此次验证过程中还发现并清理了一个本会话（压缩前）遗留的、卡在 `relay_streamable_http` flaky 测试上超过一小时未退出的僵尸测试进程，它一直占用同一个可执行文件路径导致后续两轮 `cargo test` 都在链接阶段报 "permission denied"——终止该进程后问题消失，与本次代码改动无关。
- [x] 11.2 `npm run lint`、`npx tsc --noEmit`、`npm run test`（550/550，132 个测试文件）、`npm run build` 全部干净。`npm run build` 首次因 chunk-size 门禁失败（`runtime-floating-assistant-client` 808 KiB 超 700 KiB 预算）；核对 main 分支自己的 CI（`gh run view`）确认 Frontend job 当前是绿的，判断是本 worktree 的 `node_modules` 漂移（已知的、之前在其他 worktree 出现过的环境问题，这次没有留下 `pnpm-lock.yaml` 痕迹，是新变种，已补充记录到相关 memory），`npm ci` 后重建，chunk 降到 571 KiB，通过。
- [x] 11.3 `openspec validate --specs --strict`（85/85 通过）与 `openspec validate add-cli-memory-support --strict`（通过）。
- [x] 11.4 **修复**（用户执行 `openspec verify` 后触发的独立子代理核对；子代理直接执行已安装的 `@fission-ai/openspec` 包的真实 `buildUpdatedSpec` 归档逻辑复现，而不是只跑 `openspec validate --strict`——后者不模拟真实合并，测不出这类问题）：本变更 `specs/agent-cross-session-memory/spec.md` 里 3 处 MODIFIED 场景会导致真实归档失败。(1) "Memory scoping"：把当前主 spec 的 3 个既有场景名（`Memory scoped to agent and folder`、`Memory scoped to agent only when no folder is available`、`Memories do not cross agents`）整体换成了 3 个全新场景名，触发归档器的"当前 spec 场景在增量块里找不到"防丢失检查；(2) "Memory management" 同理，`List an agent's memories`/`Reset all of an agent's memories`/`Reset is scoped to one agent` 被换名或直接丢弃；(3) "Web runtime memory toggle parity" 的场景名跟 `add-personalization-settings` 归档后将产生的既有场景名对不上（还发现 `add-personalization-settings` 自己的同一个 requirement 改名也没声明 `RENAMED`，一并在那边的 tasks.md 11.8 修了）。修复方式统一：保留每一个当前存在的旧场景名作为标题，把内容改写成如实描述新的共享池行为（包括几处行为整体反转的场景，如"Memories do not cross agents"标题下老实写明"reversing the isolation this scenario previously guaranteed..."），需要的新增场景（如 CLI 专属的抽取模拟）作为额外场景保留，不占用旧名额。修复后用一个临时脚本直接调用真实 `buildUpdatedSpec`，模拟 Phase 1→2→3 三个变更依次真实归档到一份 scratch spec 树上，9 次 spec 应用全部成功，才认为修复生效（而不是只看 `openspec validate` 变绿）。顺带修了一处代码审查中发现的无关小问题：`api_process_adapter.rs:917` 的 `scoped_memories` 变量名是共享池改造前的残留命名，改回 `memories`。
- [x] 11.5 **用户本机手动验证已完成**：按 `manual-test-plan.md` 的 4 个场景（跨 agent 记忆共享可见、CLI 对话产生新记忆、无 OnePiece 凭据时优雅降级、重置确认文案）验证。
- [ ] 11.6 归档前重新确认任务 1.1 的前置条件（Phase 1、Phase 2 均已归档）成立，执行 `openspec archive add-cli-memory-support`，随后执行 `powershell -ExecutionPolicy Bypass -File scripts/Update-OpenSpecArchiveIndex.ps1`。
