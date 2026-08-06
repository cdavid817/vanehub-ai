# Tasks: add-personalization-settings

## 1. 研究与前置调查

- [x] 1.1 通读 `AgentSkillPort`/`RuntimeAgentSkillAdapter`（`skill_gateway.rs`）：确认模板形状——`#[derive(Clone)] struct RuntimeAgentSkillAdapter { skills: SkillApi }`，`new(skills: SkillApi)`，`impl AgentSkillPort for ...` 直接把另一个 context 的 `api` facade 包一层、把对方的错误类型 map 成 `AgentRuntimeApplicationError` 的对应变体。`AgentPersonalizationPort`/`RuntimeAgentPersonalizationAdapter` 照此结构复刻，持有字段换成 `desktop::api::DesktopSettingsApi`。
- [x] 1.2 通读 `desktop::api::DesktopSettingsApi::get_settings()`：返回 `Result<DesktopSettingsView, DesktopSettingsError>`，`DesktopSettingsView { settings: DesktopSettings, logging_policy: .. }`（`application/models.rs:45`）。`RuntimeAgentPersonalizationAdapter::settings()` 读 `view.settings`，通过 5 个新增的 `pub(crate)` getter（2.3 新增）取值，`DesktopSettingsError` 映射成 `AgentRuntimeApplicationError`。
- [x] 1.3 已确认（结论见 design.md D5）：检测信号取 `AgentMessage.tool_use: Vec<ToolUseBlock>`（`application/models.rs:411`），在 `execute()` 里 `history.recent_messages(...)` 取到 `recent` 后立即计算 `recent.iter().any(|m| !m.tool_use.is_empty())`，作为 `bool` 一路传进 `maybe_compact`/`extract_memories`——不解析 `turns: Vec<Value>` 的 provider-specific wire-format JSON，避免 `recent`/`turns` 索引不对齐的问题。
- [x] 1.4 已取出 `git show 4992aef:src/settings/pages/agents/agent-memory-panel.tsx` 的原始实现：`useQuery`/`useMutation` + `agentMemoriesQueryKey`、`listAgentMemories`/`deleteAgentMemory`、`window.confirm` 删除确认、`agents.memory.*` i18n key、按 content/source badge/folder/createdAt 渲染列表项。**确认该组件对应的 `agents.memory.*` i18n key 也已被一并删除**（`en.json` 全文搜索 `memory` 只剩一处无关的 `im.wechat.scanHint` 文案），8.5 需要新起 `personalization.memory.*` 命名空间，不是复用旧 key。

## 2. `desktop`：新增设置键（无 schema 迁移，`settings` 表已是 key/value 结构）

- [x] 2.1 `DesktopSettingKey` 新增 5 个变体：`CustomInstructionsAboutUser`、`CustomInstructionsStyleRules`、`CustomInstructionsEnabled`、`MemoryEnabled`、`MemoryToolAssistedChatsEnabled`（`domain/settings.rs`），`as_str`/`parse` 双向映射。
- [x] 2.2 `DesktopSettingMutation::parse_for_key` 新增对应分支：两个文本字段校验 ≤3000 Unicode 字符（超限返回 `DesktopSettingsDomainError::invalid`，镜像现有字段的校验风格）；三个布尔字段复用现有 `parse_bool`。
- [x] 2.3 `DesktopSettings` 聚合新增字段 + `apply()` 分支 + `defaults()`：`customInstructionsAboutUser`/`StyleRules` 默认空字符串，`customInstructionsEnabled`/`memoryEnabled`/`memoryToolAssistedChatsEnabled` 默认 `true`（design.md D8）。
- [x] 2.4 单元测试：扩展 `defaults_preserve_the_existing_native_settings_contract`（5 个新字段默认值）与 `setting_keys_and_values_keep_exact_storage_names_and_allowed_values`（5 个新 key 的 round-trip），新增 `custom_instructions_fields_enforce_the_character_limit`（3000 边界值通过、3001 拒绝、空字符串允许）。

## 3. `agent_runtime`：跨 context 端口

- [x] 3.1 新增 `PersonalizationSettings` 值对象 + `AgentPersonalizationPort` trait（`application/ports.rs`/`models.rs`，design.md D2 的确切签名——单方法 `settings()`，未拆分多方法）。追加了 `PersonalizationSettings::safe_fallback()`（`models.rs`），供跨 context 调用失败时降级用。
- [x] 3.2 新增 `RuntimeAgentPersonalizationAdapter`（`infrastructure/personalization_gateway.rs`），持有 `DesktopSettingsApi`（`Clone`，非 `Arc`包裹，与 `RuntimeAgentSkillAdapter`持有 `SkillApi`的方式一致），`settings()` 调用 `get_settings()` 并映射错误为 `AgentRuntimeApplicationError::Personalization`（新增的错误变体，同步修了 3 处因此新增而不再穷尽的 match：`commands/error.rs`、`application/coordination.rs`、`infrastructure/coordination_executor.rs`、`sessions/infrastructure/runtime_support.rs`，共 4 处）。
- [x] 3.3 bootstrap 装配：`AgentRuntimeDependencies` 新增 `desktop_settings: DesktopSettingsApi` 字段，`bootstrap/runtime.rs` 传入既有的 `desktop_settings_api.clone()`（该实例已在 `communications` 装配中被复用，`agent_runtime` 是新增的第二个跨 context 消费方），`assemble_agent_runtime_api` 构造 adapter 并作为新参数传入 `RuntimeAgentApiAdapter::new(...)`。
- [x] 3.4 未新增独立单元测试文件——`RuntimeAgentSkillAdapter`（本任务的直接模板）本身也没有专门测试（构造依赖 Tauri `AppHandle`，不便脱离 bootstrap 单测），保持与模板一致的覆盖策略；改为通过新增的 `NoopPersonalization` fake（`api_process_adapter.rs` 测试模块）间接覆盖 `AgentPersonalizationPort` 契约本身，贯穿 4.4/5.4 的所有新测试。

## 4. `agent_runtime`：`resolve_system_prompt` 扩展（自定义指令 section）

- [x] 4.1 新增 `format_custom_instructions_section(settings: &PersonalizationSettings) -> Option<String>`：禁用或两字段皆空 → `None`；否则按 `style_rules`（`### Response style`）在前、`about_user`（`### About the user`）在后拼装为 `## Custom Instructions` section，单侧字段为空时省略对应子标题。
- [x] 4.2 `resolve_system_prompt` 新增 `personalization: &dyn AgentPersonalizationPort` 参数（紧跟 `core_instructions` 之后，`skills` 之前），通过新增的共享辅助函数 `resolve_personalization_settings`（一次读取，失败记 `Warn` 日志并降级为 `PersonalizationSettings::safe_fallback()`，供 4.2/5.2/5.1 三处复用而非各自单独 fetch）取值；组装顺序变为 `[core_section, custom_instructions_section, skill_section, memory_section]`。
- [x] 4.3 `personalization` 依赖从 `RuntimeAgentApiAdapter` 结构体字段 → `new()` 构造参数 → `monitor_generation` clone → `run_generation` 参数 → `execute()` 参数，一路穿透到 `resolve_system_prompt`/`maybe_compact` 两个调用点（`execute()` 内部实际有两处 `maybe_compact` 调用——工具轮次循环前一次、循环内一次——两处都已接入）。
- [x] 4.4 测试（`resolve_system_prompt`/`format_custom_instructions_section` 相关）：`format_custom_instructions_section_orders_style_rules_before_about_user`、`_omits_the_section_when_disabled`、`_omits_the_section_when_both_fields_are_empty`、`_includes_only_the_non_empty_field`；`resolve_system_prompt_includes_custom_instructions_between_core_and_skills`（四来源组合场景，断言拼接顺序）；`resolve_system_prompt_falls_back_to_safe_defaults_when_personalization_lookup_fails`（其余 section 不受影响）。

## 5. `agent_runtime`：记忆开关接入（不改变 `agent_memories` 表结构）

- [x] 5.1 **实现位置与原计划不同**：没有改 `execute_tool_call`/`execute_remember` 的签名（那样会波及它们 12+ 处既有测试调用）。改为在 `execute()` 内唯一的真实调用点（工具执行循环里）前置判断：`tool_use.name == REMEMBER_TOOL_NAME && !settings.memory_enabled` 时直接构造"Memory is disabled"的拒绝结果，短路掉 `execute_tool_call`，从未触达 `AgentMemoryPort::save`。行为等价，`execute_tool_call`/`execute_remember` 签名与全部既有测试不受影响。
- [x] 5.2 `maybe_compact` 新增 `personalization`/`tool_assisted` 两个参数，`extract_memories` 调用前判断 `extraction_allowed = memory_enabled && (!tool_assisted || memory_tool_assisted_chats_enabled)`。`tool_assisted` 由 `execute()` 顶部一次性计算（见任务 1.3 结论），随 `maybe_compact` 每次调用传入。
- [x] 5.3 `resolve_system_prompt` 的 memory 分支：`!memory_enabled` 时 `memory_section` 直接短路为 `None`，不调用 `memories.list(...)`。
- [x] 5.4 测试：`remember_tool_call_is_rejected_without_persisting_when_memory_is_disabled`；`extract_memories`/`maybe_compact` 层面新增 `maybe_compact_skips_extraction_when_memory_is_disabled`、`maybe_compact_skips_extraction_for_a_tool_assisted_session_when_the_sub_toggle_is_off`、`maybe_compact_still_extracts_for_a_non_tool_assisted_session_when_the_sub_toggle_is_off`；`resolve_system_prompt_omits_memory_section_and_skips_the_lookup_when_memory_is_disabled`（用一个"调用即 panic"的 fake 断言仓储真的没被查询，而不仅是结果为空）。

  验证：`cargo test --lib api_process_adapter`

## 6. `agent_runtime`：记忆批量重置

- [x] 6.1 `AgentMemoryPort` 新增 `delete_all_for_agent(&self, agent_id: &str) -> Result<(), AgentRuntimeApplicationError>`。同步补全了全部 3 个既有实现者（`SqliteAgentMemoryRepository`、`application/tests.rs` 的 `FakeWorld`、`api_process_adapter.rs` 测试模块的 `FakeMemories`）。
- [x] 6.2 `SqliteAgentMemoryRepository` 实现：`DELETE FROM agent_memories WHERE agent_id = ?1`（物理删除，design.md D6）。
- [x] 6.3 `AgentRuntimeApplicationService`/`AgentRuntimeApi` 新增 `reset_agent_memories(agent_id)` facade 方法，镜像 `delete_agent_memory` 的委托模式。
- [x] 6.4 新 Tauri command `reset_agent_memories.rs`（`commands/agent_runtime/`），注册进 `commands/agent_runtime/mod.rs` 与 `commands/registry.rs`（按字母序插入正确位置）。
- [x] 6.5 仓储单元测试：`delete_all_for_agent_removes_every_memory_across_every_folder`、`delete_all_for_agent_does_not_affect_other_agents`（7 项 `memory_repository` 测试全部通过）。

## 7. 前端：服务层

- [x] 7.1 `src/types/settings.ts` 新增 5 个字段 + `customInstructionsFieldCharacterLimit` 常量；`settings-service.ts` 的 `defaultAppSettings`/`normalizeAppSettings` 新增字段默认值与校验（新增 `isValidCustomInstructionsField` 校验两文本字段 ≤3000 字符，三布尔字段走既有 `typeof` 模式）；`validateSettingValue` 无需改动（已是通用实现，自动复用 `normalizeAppSettings`）。未新增 `SettingsService` 接口方法。
- [x] 7.2 确认 `tauri-settings-client.ts`/`web-settings-client.ts` 均为通用 `key`/`value` 透传（无字段专属逻辑），无需改动；`web-settings-client.ts` 的 localStorage 默认值走 7.1 已更新的 `defaultAppSettings`。
- [x] 7.3 `src/services/agent-service.ts` 新增 `resetAgentMemories(agentId: string): Promise<void>`；`tauri-agent-client.ts` 实现（`invoke("reset_agent_memories", ...)`）；`web-agent-client.ts` mock 实现（清空该 agent 在 `webAgentMemories` 里的条目）。测试双（`test/render.tsx` 的 `createAgentServiceDouble`）基于 `Proxy`+`Partial<AgentService>`，新增接口方法无需同步改动。

## 8. 前端：「个性化」设置页

- [x] 8.1 `src/settings/settings-pages.ts`：`SettingsPageId` 新增 `"personalization"`，注册在 `agent-configurations` 之后、`skills` 之前（图标 `Sparkles`），lazy loader 指向新页面模块。同步更新 `settings-pages.test.ts`（页面总数 14→15，新增位置断言测试）。
- [x] 8.2 新增 `src/settings/pages/personalization-page.tsx`：`PageHeader` + 两个 section 组件（拆到 `pages/personalization/` 子目录，匹配 `pages/agents/`、`pages/mcp/` 等既有的子目录分文件惯例，单文件不超 300 行）。
- [x] 8.3 `pages/personalization/custom-instructions-section.tsx`：两个 `textarea`（回复风格 / 关于你）+ 实时字符计数（3000 上限，超限时 `onBlur` 不提交）+ 启用开关，直接走 `useSettings()` 的通用 `saveSetting`，未新增 provider 方法。
- [x] 8.4 `pages/personalization/agent-memory-section.tsx`：两个开关（`memoryEnabled`、`memoryToolAssistedChatsEnabled`，后者在总开关关闭时禁用置灰）+ 重建的列表（内容/来源徽章/文件夹/创建时间，参照 1.4 取出的原实现）+ 单条删除（`window.confirm`）+ 批量重置按钮（二次确认文案明确"不可撤销"）。Agent id 硬编码为 `"onepiece"`（本阶段唯一具备记忆能力的 Agent，不需要 Agent 选择器）。
- [x] 8.5 新增 `personalization.*` i18n key（未复用旧 `agents.memory.*`——已确认连同旧面板一起被删除），`settings.pages.personalization`/`settings.search.personalization`，写入全部 5 个已注册语言资源文件（`zh-CN`/`en`/`zh-TW`/`ja`/`ko`，`i18n-resource-parity.test.ts` 29 项全部通过）。

## 9. 前端测试

- [x] 9.1 `agent-memory-section.test.tsx`（7 项：空状态、列表+来源/文件夹渲染+删除确认、取消删除、重置确认、取消重置、总开关关闭时子开关禁用）+ `custom-instructions-section.test.tsx`（4 项：`onBlur` 保存、字符计数、超限不提交、启用开关联动）。两者都需要真实 `SettingsProvider`（`useSettings` 依赖），`renderWithAppProviders` 本身不含它，测试里额外包了一层。
- [x] 9.2 `web-agent-client.test.ts` 新增 `resetAgentMemories` 测试：清空指定 agent 的模拟记忆存储且不影响另一个 agent 的记忆。
- [x] 9.3 `settings-service.test.ts` 新增 3 项：默认值、5 个新键的 round-trip、超长字段回退为空。`tauri-settings-client.ts`/`web-settings-client.ts` 本身未变动（7.2 已确认为通用透传），未新增对应 client 测试。

## 10. 验证与收尾

- [x] 10.1 `cargo test`（等价于 `--manifest-path src-tauri/Cargo.toml`，在 `src-tauri/` 内执行）— 全量通过：lib 1254 passed（含 9 项既有 ignored）、architecture 12 passed、mcp_fixture_contracts 3 passed、mcp_relay_provider_invocations 3 passed，无回归。
- [x] 10.2 `cargo clippy --all-targets -- -D warnings` — 干净。修了一处新增 `resolve_system_prompt`（8 参数）触发的 `too_many_arguments`，补 `#[allow(...)]`（与文件里其它宽参数函数一致的既有处理方式）。
- [x] 10.3 `cargo check --lib --tests` — 干净；架构测试 `native_context_dependencies_point_inward` 通过，确认新的 `agent_runtime → desktop` 依赖未违反 DDD 分层规则，也没有引入其它越界依赖。另跑了 `cargo fmt`（发现 5 个文件有格式漂移，已自动修正）+ `cargo fmt --check` 复核为干净。
- [x] 10.4 `npm run test`（vitest）— 132 files / 543 tests 全部通过。过程中发现并修正两处：新组件用 `.toLocaleString()` 触发了 `i18n/format.test.ts` 的"不允许操作系统本地化兜底"守卫（改用 `formatAppDateTime`）；新增设置页把懒加载页面数从 14 变成 15，`settings-pages.test.ts` 与 `frontend-lazy-loading.test.ts` 两处硬编码计数都要同步改。
- [x] 10.5 `npm run lint` 与 `npx tsc --noEmit` — 均干净。
- [x] 10.6 `npm run build` — 干净，16 个懒加载 chunk 校验通过，主静态闭包 105.8 KiB gzip（既有的大 chunk 警告与本次改动无关）。
- [x] 10.7 `openspec validate --specs --strict` 与 `openspec validate add-personalization-settings --strict` — 全部通过（85 项主 specs + 本变更）。
- [ ] 10.8 **需要用户本机手动验证**：详细步骤见同目录下 `manual-test-plan.md`（18 项用例，覆盖基础健全性、设置持久化、自定义指令对生成行为的实际影响、记忆增删查、记忆双开关联动、错误提示、旧页面回归检查）。
- [ ] 10.9 归档：`openspec archive add-personalization-settings`，随后执行 `powershell -ExecutionPolicy Bypass -File scripts/Update-OpenSpecArchiveIndex.ps1`。**硬性要求（design.md Risks 已记录，`openspec verify` 独立核对时发现）：本变更归档后必须在同一操作序列内紧接着按顺序归档 `add-cli-custom-instructions-injection`、`add-cli-memory-support`，中间不能停顿提交** —— 本变更「Memory management」「Memory injection into the system prompt」断言的按 agent 隔离保证，已经被 `add-cli-memory-support` 已实现（未归档）的共享记忆池代码推翻，若归档后有间隔，main spec 会短暂断言一个当前代码已不成立的保证。

## 11. 实现完成后的深度代码审查（用户要求）与修复

- [x] 11.1 **修复**：`tool_assisted_session` 生成内失真（design.md Risks 已记录）。改 `execute()` 里该变量为 `mut`，工具轮次执行后若 `executed` 非空则置 `true`。新增 `tool_assisted_flag_reflects_a_tool_call_made_earlier_in_the_same_generation` 回归测试（`api_process_adapter.rs`），并通过临时禁用修复验证测试确实会失败，恢复后通过。
- [x] 11.2 **修复**：「个性化」页面完全没有保存/删除/重置失败的可见反馈。`PersonalizationPage` 新增顶层 `error` 展示（镜像 `basic-settings-page.tsx` 的既有模式）；`AgentMemorySection` 新增 `operationError`（合并 query + 两个 mutation 的 error，镜像 `onepiece-configuration-panel.tsx` 的既有模式）；顺带把 `handleDelete`/`handleReset` 从 `mutateAsync` + `await`（未捕获，存在 unhandled rejection 风险）改成 `.mutate()`（错误状态走 react-query 自身机制，不再需要手动 catch）。
- [x] 11.3 **修复**：字符数校验前端用 `String.length`（UTF-16 code unit）、后端用 `.chars().count()`（Unicode scalar value），代理对字符（如部分 emoji）会导致前端比后端更早拒绝合法输入。新增 `countCustomInstructionsCharacters`（`types/settings.ts`，`[...value].length` 按码点计数），`settings-service.ts`/`custom-instructions-section.tsx` 两处改用它。新增 emoji 场景回归测试（`settings-service.test.ts`）。
- [x] 11.4 验证：`cargo test` 全量重跑（1254 passed，另有 `tooling::mcp::infrastructure` 下少数测试在高并发/资源争抢下偶发超时，隔离单独重跑均通过——环境固有的计时敏感 flaky，与本次改动无关，非回归）；`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 干净；`npm run test`（544 passed，无 flaky）、`npm run lint`、`npx tsc --noEmit`、`npm run build` 全部干净。
- [x] 11.5 **修复**（`add-cli-custom-instructions-injection` 的 `openspec-verify-change` 交叉核对期间，为搭建个性化设置页 E2E 覆盖而读 `web-agent-client.ts` 时发现）：Web/mock 的 `sendMessage` 模拟里，`remember` 工具调用、自动抽取卡片、记忆注入卡片三处模拟事件完全不检查 `memoryEnabled`/`memoryToolAssistedChatsEnabled`，无条件触发——与自定义指令"mock 里天生不可观察"不同，这三个模拟事件在 mock 模式下是结构性可观察的（通过 `tool_use`/`rich_block` 事件流），关掉开关却看不出效果是真实的 mock/设置不一致。修复：`web-settings-client.ts` 导出 `readWebAppSettings`（原地把私有 `readStoredSettings` 改名导出，供其它 Web/mock client 复用，镜像已有的 `findWebSshConnection` 跨 client 导入先例）；`web-agent-client.ts` 的 `sendMessage` 读取一次设置，`remember` 工具调用与记忆注入卡片仅受 `memoryEnabled` 门控，自动抽取额外要求 `memoryToolAssistedChatsEnabled`（mock 里每个会话都会模拟工具调用，天然满足真实定义里"tool-assisted"的条件，因此这个门控是精确的，不是近似）。新增 2 项回归测试（`web-agent-client.test.ts`），并通过临时把两个开关读取值硬编码为 `true` 验证测试确实会失败，恢复后通过；测试文件本身跑在 Vitest 默认 `node` 环境（无 `window`），额外加了一个作用域仅限这两项测试的最小 `localStorage` 桩（`vi.stubGlobal` + `vi.unstubAllGlobals` 清理）。补了 `agent-cross-session-memory` spec 的对应 "Web runtime memory toggle parity" 新增要求。
- [x] 11.6 验证（11.5 之后）：`npx tsc --noEmit`、`npm run lint` 干净；`web-agent-client.test.ts` 单独跑 55 passed；`cargo check --lib --tests` 干净（本次是纯前端改动，用于确认没有意外影响 Rust 侧）；全量 `npm run test` 跑到 545/546（1 项 `session-skills-pane.test.tsx` 超时失败，与本次改动无关的文件，隔离单独重跑仍偶发超时——`tasklist` 确认当时机器上有非本会话发起的 cargo/rustc/node 进程在跑，环境资源争抢导致，不是回归；`custom-instructions-section.test.tsx` 同批合跑时也偶发一次不相关的断言错位，单独隔离跑 4/4 全过，同一根因）；`openspec validate add-personalization-settings --strict` 通过。
- [x] 11.7 **新增**：`tests/e2e/personalization-settings.spec.ts`（Playwright，4 项用例）——导航到「个性化」页并渲染两个 section；自定义指令 blur 后保存、刷新页面后仍持久化、超限内容被拒绝且不落盘；启用开关关闭时两个文本框置灰；记忆主开关关闭时工具辅助子开关跟着禁用。真实浏览器针对真实 dev server 跑通 4/4（`env -u all_proxy -u ALL_PROXY npx playwright test tests/e2e/personalization-settings.spec.ts`）。过程中发现并修了 2 处测试代码自身的 bug（非产品代码问题）：手算字符数错了一位；`beforeEach` 里 `page.addInitScript` 用 `setItem` 整体覆盖 `vanehub.appSettings`，而 `addInitScript` 在 `page.reload()` 时会重新执行，导致测试保存的自定义指令被刷新页面自己的初始化脚本冲掉——改成读现有值后 `{ ...existing, applicationLanguage: "en" }` 合并写入，不再覆盖同名 key 之外的字段。这份 E2E 覆盖了 `manual-test-plan.md` 里"基础健全性""设置持久化""字符上限"这几类用例里不依赖真实 CLI/API 凭据的部分，降低了 10.8 需要人工点击验证的范围（记忆增删查、真实 OnePiece 生成行为等仍需人工）。
- [x] 11.8 **修复**（`add-cli-memory-support` 的 `openspec verify` 期间发现，直接执行已安装的 `@fission-ai/openspec` 包的真实 `buildUpdatedSpec` 归档逻辑复现）：本变更自己的 `specs/agent-cross-session-memory/spec.md` 会导致 `openspec archive add-personalization-settings` 在真正执行时失败——`## MODIFIED Requirements` 把 `### Requirement: Web runtime memory parity` 改名成 `### Requirement: Web runtime memory toggle parity`，但没有声明 `## RENAMED Requirements`，归档器按"当前主 spec 里没有这个名字的 requirement"直接报错；且改名后的新场景集合里缺了当前主 spec 该 requirement 下的既有场景 `Web mock memory behaviors`，归档器有专门的"当前 spec 里存在、增量块里却找不到"防丢失检查，同样会拦下来。两处都不是 `openspec validate --strict` 能测出来的（validate 只查增量文件自身格式是否合法，不模拟真实合并）。修复：补上 `## RENAMED Requirements`（`FROM: Web runtime memory parity` / `TO: Web runtime memory toggle parity`）；把 `Web mock memory behaviors` 场景原样保留进 `MODIFIED` 块（内容依然成立，只是被后续两条更细的开关判定场景取代为更具体的特例，不冲突）。用同样直接调用 `buildUpdatedSpec` 的方式，模拟 Phase 1→2→3 三个变更依次真实归档，确认 9 次 spec 应用全部成功后才认为修复生效。
- [x] 11.9 **独立子代理执行 `openspec verify add-personalization-settings`**，发现并处理 3 类问题：(1) **归档顺序硬依赖未被本变更自己的文档记录**——本变更「Memory management」「Memory injection into the system prompt」断言的按 agent 隔离保证，已被同一工作树里已实现（未归档）的 `add-cli-memory-support` 共享记忆池代码推翻，若归档后有间隙会短暂断言失实的保证；已把这条硬性顺序要求写进本文件 10.9 和 design.md Risks，不是代码/spec 内容问题，是文档缺失。(2) **`specs/settings-center-ui/spec.md` 只 ADD 了「Personalization settings navigation」，design.md D7 承诺的"同步更新 UCD settings pages 已枚举导航列表"没有做**——补了一个 `## MODIFIED Requirements` 块，把 "Personalization" 插进主 SHALL 语句和 "Display UCD page set" 场景的 THEN 子句里（位置：Agent Configuration 之后、Skills 之前，跟 `settings-pages.ts` 实际顺序一致），并新增一条 AND 子句显式声明这个位置约束；第一版手动重建这个 MODIFIED 块时漏掉了一整个场景（"Check updates from About page"）且另一个场景内容被截短，靠跟主 spec 原文 diff 才发现，改成直接摘录原文再插入三处改动，diff 确认只有预期的 3 行变化；用真实 `buildUpdatedSpec` 确认 `settings-center-ui` 应用后是 `+1 ~1`，没有报错。(3) 子代理另外提出一条"i18n 文案宣称自定义指令支持 CLI Agent，但本变更自己的 proposal.md 把 CLI 支持列为 Non-Goal"的 WARNING——核实为**误报**：CLI 分支的自定义指令注入代码确实存在（`service.rs:1575-1638`，`agent.launch().kind_str() == "cli"` 分支），是 `add-cli-custom-instructions-injection`（同一工作树内已实现）加的，子代理搜索时找错了目录（找的是 `contexts/sessions` 而不是同一个 `contexts/agent_runtime/application/service.rs` 文件内的 CLI 分支），文案没有问题。子代理还顺带指出一个不阻塞本变更、跟本变更无关的既有小问题：`openspec/specs/agent-skill-injection/spec.md` 里 "Bounded Skill prompt assembly" 这个 requirement 重复出现了两次（第 71-80 行和 121-130 行，逐字节相同）——记录在此，留给后续单独清理，不在本变更范围内处理。
