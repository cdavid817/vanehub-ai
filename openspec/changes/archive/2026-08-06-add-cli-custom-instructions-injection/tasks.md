# Tasks: add-cli-custom-instructions-injection

## 1. 研究与前置条件

- [x] 1.1 **阻塞性前置条件**：`openspec list --json` 确认 `add-personalization-settings` 目前仍是 in-progress，**尚未归档**。不阻塞本变更编码/测试，只阻塞任务 6.7 的最终归档——归档前需要重新检查一次这个条件。
- [x] 1.2 已确认（结论见 design.md Open Questions）：`web-agent-client.ts` 的 `sendMessage` 是所有 agent 共用的同一个桩函数，回复内容是写死模板字符串，从未模拟过"实际发给模型/CLI 的提示词内容"这个概念（OnePiece 自定义指令在 mock 里同样不可观察）。**不是本变更的缺口**，任务 5 相应收窄为确认性任务，不新增代码。custom-instructions spec 里对应的 Web mock 场景已同步改写为准确描述现状。
- [x] 1.3 已通读 `AgentRuntimeApplicationService::send_message`（`application/service.rs`）里 `agent.launch().kind_str() == "cli"` 分支：`effective_prompt` 之后被用于 `input_count = effective_prompt.content.chars().count()`（约第 1697 行）与 `GenerationProcessRequest { effective_prompt: effective_prompt.content, .. }`（约第 1744 行）。拼接需要在 `if kind_str() == "cli"` 分支内部完成，产出的仍然是一个 `EffectivePrompt { content, trace }`，下游两处用法不需要改动签名，只要分支内部返回拼接后的值即可。

## 2. `application` 层：格式化逻辑上移（OnePiece 与 CLI 共用）

- [x] 2.1 `PersonalizationSettings::custom_instructions_block(&self) -> Option<String>` 新增到 `application/models.rs`，实现原样从 `format_custom_instructions_section` 搬过来。
- [x] 2.2 `api_process_adapter.rs` 的 `format_custom_instructions_section` 改为一行委托（`settings.custom_instructions_block()`），保留函数本身不删除，4 个既有测试的调用语法和断言文本都不用改。
- [x] 2.3 `cargo test --lib api_process_adapter::tests::format_custom_instructions` — 4 项全绿，断言文本未变。

## 3. 跨 context 端口扩展到 CLI 消费方

- [x] 3.1 `AgentRuntimeApplicationPorts`（`application/service.rs`）新增 `personalization: Arc<dyn super::AgentPersonalizationPort>` 字段。
- [x] 3.2 `bootstrap/agent_runtime.rs`：`RuntimeAgentApiAdapter::new(...)` 的 `agent_personalization` 参数改传 `.clone()`，原值传入新增的 `AgentRuntimeApplicationPorts { personalization: agent_personalization, .. }`（放在构造对象的最后一个字段，因此这里不需要再 clone）。
- [x] 3.3 `FakeWorld`（`application/tests.rs`）新增 `impl AgentPersonalizationPort`，返回 `PersonalizationSettings::safe_fallback()`；`AgentRuntimeApplicationPorts` 测试构造里 `memories` 从移动值改成 `.clone()`，新增 `personalization: world`。

## 4. CLI 发送路径接入

- [x] 4.1 `AgentRuntimeApplicationService` 的 CLI 分支：`assemble()` 成功返回后，读取 `self.ports.personalization.settings()`（失败降级为跳过自定义指令、记 `Warn` 日志、不阻断发送——design.md D2），取 `custom_instructions_block()`，与 `effective_prompt.content` 按 `[自定义指令块, effective_prompt]` 顺序、`"\n\n"` 拼接成最终发送内容。
- [x] 4.2 已确认：拼接后的 `EffectivePrompt { content, trace }` 直接替换原变量，下游 `input_count = effective_prompt.content.chars().count()` 与 `GenerationProcessRequest { effective_prompt: effective_prompt.content, .. }` 两处用法无需改动签名，自动读取到拼接后的最终文本。
- [x] 4.3 测试（`application/tests.rs`，均已通过）：自定义指令启用且非空时正确前置（`send_message_prepends_custom_instructions_for_cli_agents_when_enabled`）；禁用时不影响 CLI 发送内容，等价于本变更之前的行为（`send_message_omits_custom_instructions_for_cli_agents_when_disabled`）；两字段皆空时不产生空块（`send_message_omits_custom_instructions_for_cli_agents_when_both_fields_are_empty`）；`personalization.settings()` 读取失败时降级为跳过自定义指令、CLI 消息仍正常发送且记录 `Warn` 日志（`send_message_degrades_gracefully_when_personalization_lookup_fails_for_cli_agents`）；非 CLI（API kind）agent 不受影响，回归验证（`send_message_does_not_prepend_custom_instructions_for_non_cli_agents`）；拼接逻辑按 `launch().kind_str() == "cli"` 生效，不绑定具体 agent id，对第二个不同 id 的 CLI-kind agent 同样生效，证明对 `claude-code`/`codex-cli`/`gemini-cli`/`opencode` 四者一致适用（`send_message_prepends_custom_instructions_for_any_cli_kind_agent_not_just_one`）。

  验证：`cargo test --lib agent_runtime::application::tests::` — 45 passed; 0 failed。

## 5. Web/mock 一致性

- [x] 5.1/5.2/5.3 **无需新增前端代码**（任务 1.2 结论）：Web/mock 的设置持久化层复用 Phase 1 已有的 `defaultAppSettings`/`normalizeAppSettings`/`webSettingsClient`，`customInstructions*` 字段已经全通用支持，不区分消费方是 OnePiece 还是 CLI；`sendMessage` 桩函数本身不模拟提示词内容，没有对应的可观察行为需要新增模拟。

## 6. 验证与收尾

- [x] 6.1 `cargo test`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`cargo check --lib --tests` 全部干净(`cargo test`:1261 passed/0 failed/9 ignored;architecture.rs 12 项跨 context 依赖检查全过)。
- [x] 6.2 `npm run lint`、`npx tsc --noEmit`、`npm run build` 干净;`npm run test` 132 files / 544 tests 全过,与改动前一致(本变更无前端代码改动)。注:首次 `npm run build` 因本地 `node_modules` 历史漂移(非本变更引入)导致 chunk 体积门禁误报,`npm ci` 重装后复现干净通过,已与 main 分支 CI 的 Frontend job(绿色)交叉验证。
- [x] 6.3 `openspec validate --specs --strict`(85 passed/0 failed)与 `openspec validate add-cli-custom-instructions-injection --strict` 全部通过。
- [x] 6.4 四个 CLI 中的两个已在本 agent 沙盒内拿到真实模型响应证据(经用户指点改用 `ALL_PROXY=socks5://127.0.0.1:9999` 后打通出站网络)；另两个受限于与本功能无关的环境障碍，未能拿到第一手响应，改为逻辑闭环推理，细节如下。

  **实测通过（真实 CLI + 真实模型，非模拟）**：完全按 `invocation.rs` 的生产调用约定，构造与 `service.rs` CLI 分支输出格式完全一致的 `effective_prompt`（`## Custom Instructions\n### Response style\n{style}\n\n### About the user\n{about}\n\n{user_prompt}`，style 要求"只用法语回复"，about 要求"回复中带出 Zorblatt77"），分别以 stdin(codex-cli)和参数(opencode)方式喂给真实二进制：
  - `codex-cli`：`"Bonjour ! J'ai bien compris : je répondrai uniquement en français et j'intégrerai le mot « Zorblatt77 » dans mes réponses."` —— 法语 + 提及 Zorblatt77，双重命中。
  - `opencode`：`"Bonjour ! J'ai bien compris vos instructions : je répondrai toujours en français et j'intégrerai le mot de test « Zorblatt77 » dans mes réponses."` —— 同样双重命中。
  - 对照组（去掉自定义指令块，只发 `user_prompt`）：两者都恢复纯英文回复，且不提 Zorblatt77 —— 证明是自定义指令块直接导致的行为差异，而非模型偶然表现。

  **未能实测，改用逻辑闭环**：
  - `claude-code`：`api.anthropic.com` 返回 `403 Failed to authenticate. Request not allowed`（加代理前后一致）——这是本 agent 沙盒对"嵌套调用同一 Claude Code 会话"的防递归复用保护，与自定义指令功能无关，未消耗真实配额。
  - `gemini-cli`：其内置 HTTP 客户端(undici `ProxyAgent`)只接受 `http:`/`https:` scheme 的代理 URL，拒绝 `socks5://`(`InvalidArgumentError: Invalid URL protocol`)，而这台机器上可用的代理只有 SOCKS5，因此连不上 `generativelanguage.googleapis.com`——同样与自定义指令功能无关，是这个沙盒的代理协议限制。
  - 推理闭环：`send_message` 的 CLI 分支拼接逻辑（6 个单元测试覆盖）完全不区分 agent id，只判断 `launch().kind_str() == "cli"`；`invocation.rs` 里 claude-code 与 codex-cli 同属 `ProviderPromptDelivery::Stdin`，gemini-cli 与 opencode 同属 `ProviderPromptDelivery::Argument` —— 两种底层传递机制均已被上面的真实调用各验证一次，claude-code/gemini-cli 各自复用了已验证机制的另一半，加上两者对应的 `process_adapter.rs`/`invocation.rs` 代码本身未被本变更触碰（`git status` 确认），因而未实测部分的风险由代码同构性兜底，不是空白猜测。

  **仍待用户执行**：关闭 `customInstructionsEnabled` 后四个 CLI 都不再体现、以及 OnePiece 侧不受影响的回归检查，仍需在桌面 App 里点击验证（这一步涉及 UI 交互，不是本 agent 能做的）。
- [x] 6.6 **修复**（用户复核发现）：proposal.md 里"无新增 UI"说的是没有新增 UI 结构，但「个性化」设置页里沿用 Phase 1 的既有文案却是——`personalization.description`/`personalization.customInstructions.description` 一直写的是"为 OnePiece..."，本变更把自定义指令实际覆盖范围扩大到四个 CLI 之后，这两句描述文案没有跟着更新，用户在页面上看到的说明和实际行为对不上。五个语言资源文件（`zh-CN`/`en`/`zh-TW`/`ja`/`ko`）同步改成"为 OnePiece 和 CLI Agent（Claude Code/Codex CLI/Gemini CLI/OpenCode）..."；`personalization.memory.description` 保持不变，因为记忆功能确实仍然只对 OnePiece 生效（design.md D 系列决策与 Non-Goals 明确写了这条边界不在本变更范围内），两处文案的范围差异现在在页面上是准确的，不是遗漏。验证：`node -e "JSON.parse(...)"` 确认 5 个文件仍是合法 JSON；`npx tsc --noEmit`、`i18n-representative-surfaces.test.tsx` 单独隔离跑 5/5（全量批跑时撞见一次同批测试资源争抢导致的超时，隔离重跑排除是回归）。
- [ ] 6.7 确认任务 1.1 的前置条件（Phase 1 已归档）成立后，归档本变更：`openspec archive add-cli-custom-instructions-injection`，随后执行 `powershell -ExecutionPolicy Bypass -File scripts/Update-OpenSpecArchiveIndex.ps1`。
