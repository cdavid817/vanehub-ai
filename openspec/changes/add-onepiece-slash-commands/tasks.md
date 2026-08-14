> 完整的分步实施计划（含每步的测试代码、实现代码与预期输出）见
> `docs/superpowers/plans/2026-08-14-onepiece-slash-commands-phase-1.md`。
> 下列任务与该计划的 Task 1-13 一一对应。

## 1. 命令内核（纯函数，无 React 依赖）

- [ ] 1.1 实现输入解析器 `src/services/slash-commands/parse-command.ts`：区分普通消息、`//` 字面转义与命令；命令限单行、名称须以字母开头，排除文件路径与多行粘贴
- [ ] 1.2 实现会话可用性谓词 `src/services/slash-commands/command-availability.ts`：`isOnePieceSession` 依据稳定 agent id 判定，`isMultiSeatCliSession` 供后续变更放宽时使用，`slashCommandsEnabled` 为唯一门控
- [ ] 1.3 定义共享类型 `src/services/slash-commands/types.ts`：`SlashCommand`、`CommandCapabilities`、`CommandContext`、`CommandOutcome`、`CommandOutput`；`appliesTo` 接收 capabilities 作为显式参数，保持纯函数
- [ ] 1.4 实现注册表查找 `src/services/slash-commands/command-registry.ts`：按名称与别名查找、按适用性过滤并排序

## 2. 命令定义

- [ ] 2.1 实现运行时切换命令 `runtime-commands.ts`（`/mode` `/reasoning` `/thinking` `/streaming` `/longcontext`），并建立 `command-catalog.ts` 汇总入口；确认不含 `/model` `/provider` `/agent`
- [ ] 2.2 实现会话与信息命令 `session-commands.ts`（`/export` `/stop` `/status` `/usage`）；`/usage` 经 `AgentService.getSessionUsageSummary` 取数，失败时呈现错误而非抛出
- [ ] 2.3 实现导航命令 `navigation-commands.ts`：destination 类（`/todo` `/plans` `/loops`）、八个工作区页签类，以及依赖 `capabilities.hasAssociatedPlan` 的 `/plan`
- [ ] 2.4 实现 `/help` `help-command.ts`：按当前会话的适用性列出命令，描述以翻译 key 形式传递

## 3. 界面表面

- [ ] 3.1 实现输出面板 `src/components/chat/SlashCommandOutput.tsx`：翻译标题与各条消息，`/help` 条目的描述参数需先翻译再插值；错误态带 `role="alert"`；可关闭
- [ ] 3.2 实现补全下拉 `src/components/chat/SlashCommandCompletion.tsx`：沿用 `SeatMentionCompletion` 的结构与无障碍属性

## 4. 调度与接线

- [ ] 4.1 实现调度 hook `use-slash-commands.ts`：`updateSuggestions` 无副作用供按键调用，`dispatch` 同步返回接管与否、异步落输出；未知命令与 handler 异常均转为错误输出并经服务边界上报
- [ ] 4.2 接入 `ChatInputBox.tsx`：新增四个可选 prop 渲染两个表面，既有调用点不受影响；确认文件仍低于 300 行
- [ ] 4.3 接入 `api-session-composer.tsx`：提交时先 dispatch，`//` 字面文本经 pending 标志跨渲染送出；失败上报复用 `createChatOperationFailureEvent` + notify + `reportClientLogEvent`
- [ ] 4.4 为 `SessionTabs` 增加 `requestedTabNonce` 并纳入 effect 依赖；在 `main-layout.tsx` 中持有页签请求状态并向 composer 传入 navigation 回调

## 5. 本地化

- [ ] 5.1 为 `slash.*` 全部键补齐 `en` / `ja` / `ko` / `zh-CN` / `zh-TW` 五个 locale 的文案；命令名保持英文不翻译
- [ ] 5.2 增加 locale parity 测试 `src/i18n/slash-command-locales.test.ts`，断言每个英文 `slash.` 键在五个 locale 中均存在且非空

## 6. 验证

- [ ] 6.1 补齐端到端用例 `tests/e2e/slash-commands.spec.ts`：`/help` 出面板且不发消息、未知命令报错、`/st` 出补全。运行时需清除 SOCKS5 代理变量并固定 `PLAYWRIGHT_PORT`
- [ ] 6.2 前端校验：`npm run lint:ci`、`npm run test`、`npm run build` 全部通过
- [ ] 6.3 原生侧校验（本变更未改 Rust，确认未被牵连）：`cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`、`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`、`cargo check --manifest-path src-tauri/Cargo.toml`
- [ ] 6.4 其余 CI 门槛：`npm run test:coverage`、`npm run contracts:check`、`npm run docs:check`
- [ ] 6.5 OpenSpec 校验：`openspec validate add-onepiece-slash-commands --strict` 与 `openspec validate --specs --strict`
- [ ] 6.6 确认 Web 适配器无需改动——本变更不新增 Tauri command，命令仅消费既有 `AgentService` 方法，`tauri-agent-client.ts` 与 `web-agent-client.ts` 保持一致
