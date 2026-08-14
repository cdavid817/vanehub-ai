## Context

动机见 `proposal.md` 的「Why」。以下只列塑造了技术方案的既有约束——每一条都是从代码中核实的，不是假设：

- `src/session-workspace/api-session-composer.tsx` 是 `ChatInputBox` 唯一的生产消费者，它把 `onSubmit` 接到 `use-main-layout-model.ts:249` 的 `submit()`
- `use-main-layout-model.ts` 是 298 行，`max-lines` 上限 300 且该文件**不在** `eslint.config.js` 的豁免清单内
- `main-layout.tsx` 是 460 行，在豁免清单内但已远超注释所记的 341，不适合承载新逻辑
- `sendMessage.onSuccess` 调用 `invalidateRuntime()`（`use-main-layout-model.ts:155`），它 invalidate `["messages", activeSessionId]` 并触发从后端重新拉取
- `api-session-composer.tsx:23` 无条件传入 `lockRuntimeIdentity`，`ButtonArea.tsx:79,104` 据此禁用 `ProviderSelect` 与 `ModelSelect`
- `ApiSessionComposer` 的渲染条件是 `interactionMode === "api" || 多席位`（`main-layout.tsx:214`），因此它并非 OnePiece 专属
- `SessionTabs` 的页签请求由 `useEffect([requestedTab, sessionId])`（`session-tabs.tsx:85-89`）消费
- `ChatInputBox.tsx:70` 已有 `@` 补全，其正则为 `(?:^|\s)@([^\s@]*)$`

## Goals / Non-Goals

**Goals:**

- 命令层可被独立测试，其核心判定不依赖 React、i18n 或服务调用
- 作用域放宽到多席位 CLI 会话时，改动收敛在一个谓词与各命令的适用条件，dispatch、补全、输出面板均不需改
- 不新增 Tauri command，从而不触发「两个 client 必须同步改」的架构约束

**Non-Goals:**

- 不设计 `/clear` 与 `/compact`，两者需要新后端能力，属后续变更
- 不设计用户自定义命令的持久化与管理界面
- 不设计命令的键盘导航（补全下拉的上下键选择）——第一版只支持点击选择

## Decisions

### 拦截点放在 composer，而非会话模型

`submit()` 是唯一发送路径，直觉上应在那里拦截。但 `use-main-layout-model.ts` 只剩 2 行预算且无豁免，任何有意义的改动都会撞上 `max-lines`。更重要的是命令层是 UI 关注点，把它塞进核心会话模型会让该模型承担两种职责。

`ApiSessionComposer` 只有 43 行，把 `onSubmit` 包一层是成本最低且职责最正的位置。

**备选**：给 `submit()` 增加一个可选的 `beforeSubmit` 钩子。否决——仍要改那个文件，且引入了一个只有一个使用者的抽象。

### 命令输出不进入消息流

`invalidateRuntime()` 在每次发送成功后重新拉取消息列表，任何注入 React Query 缓存的本地条目都会在用户下一轮对话时被后端数据覆盖。`/help` 的输出打完字活不过一轮，这不是可接受的行为。

因此命令输出存于 composer 持有的独立 state，渲染成输入框上方的可关闭面板。

**备选 A**：新增 `appendSystemMessage` 后端能力把输出落库。否决——为 `/help` 这类瞬时输出付出新 Tauri command、两个 client 同步改与每条一行数据库的代价，收益不成立。

**备选 B**：注入缓存并在每次 invalidate 后重新注入。否决——与框架对抗，且需要在会话模型里挂钩子。

### 适用性判定是纯函数，能力以参数传入

`/plan` 只在会话存在关联计划运行时可用，而这个事实不在 session 行上。最初的写法是在命令模块里放一个模块级布尔量、由 hook 在渲染期写入——那同时引入了模块级可变状态、渲染期副作用和测试间相互污染三个问题。

改为 `appliesTo(session, capabilities)`，`capabilities` 由调度层组装后显式传入。判定恢复为其参数的纯函数，测试无需置位与复位。

**备选**：把导航上下文塞进 session 对象。否决——污染了一个被约 148 处代码读取的核心类型。

### 键入与提交是两个入口

补全需要随每次按键刷新，执行只能发生在提交时。若共用一个入口，键入 `/mode execute` 会在 `/m`、`/mo`、`/mod`…每一步都执行一次命令。

因此 `updateSuggestions(draft)` 由 `onChange` 调用且**无副作用**，`dispatch(draft)` 由 `onSubmit` 调用且是唯一允许产生副作用的入口。

### 判定同步、执行异步

`dispatch` 必须同步回答「这条输入要不要放行给 `model.submit()`」，否则会出现命令与消息双发。但 `/usage` 需要 await 服务调用。

解法是 `dispatch` 同步返回接管与否，命令的 promise 在其后 resolve 时把输出设入 state。

### `//` 转义与跨渲染送出

未知命令不转发给模型（见 spec），因此需要转义才能发送真正以 `/` 开头的散文。`//foo` 解析为字面 `/foo`。

但 `model.submit()` 读的是 `model.draft`，改写 draft 与提交之间必须隔一次渲染。用一个 pending 标志加 `useEffect` 完成，而不是寄望于同一 tick 内读到新值。

### 页签请求需要 nonce

`SessionTabs` 的 `useEffect` 依赖 `[requestedTab, sessionId]`，同一个页签连续请求两次不会重新触发。用户敲 `/logs`、手动切回 chat、再敲 `/logs` 时第二次将无效。

因此 `SessionTabs` 新增可选的 `requestedTabNonce` 并纳入依赖数组，由 main-layout 在每次命令导航时递增。

### 不提供 `/model`、`/provider`、`/agent`

`lockRuntimeIdentity` 在 API 会话中禁用了模型与 provider 选择器，这是刻意的产品决策。命令绕开 UI 的锁属于缺陷而非功能。`/agent` 则因 `useChatConfig.ts:115` 在 OnePiece 会话中把候选过滤成只剩自身，没有可切换的对象。

## Risks / Trade-offs

- **命令输出刷新即失** → 这是有意的取舍。`/help`、`/status`、`/usage` 都是瞬时查询，重新执行的成本接近于零；持久化它们反而会污染对话历史与导出结果
- **`/help me understand X` 会被当成 `/help` 执行** → 已接受。命令形态限定为单行且名称后需空白，已排除路径与多行粘贴；剩余的误判面是散文恰好以已注册命令名开头，概率低且用户可用 `//` 转义
- **导航命令切走 destination 会隐藏聊天区**（`main-layout.tsx:271`）→ 这是导航的预期行为，非缺陷
- **`ApiSessionComposer` 同时服务多席位 CLI 会话** → 若门控写错会让命令在 CLI 会话意外生效。缓解：门控收敛在单一谓词模块，并有覆盖三种会话类型的适用性矩阵测试
- **新增两个浮层与既有 `@` 补全争夺同一空间** → 两者的触发正则互斥（`/` 需在行首且为整个输入，`@` 需前置空白或行首且不占满输入），但同屏叠放仍需在实现时确认层级

## Migration Plan

无数据迁移、无 schema 变更、无新依赖。纯增量的前端能力：未启用会话与未匹配命令的行为与当前完全一致。

回滚方式是撤销提交——命令层不写入任何持久化状态，不存在需要清理的残留。

## Open Questions

- 补全下拉的键盘导航（上下键 + Enter 选中）已按 Non-Goals 排除在本变更之外。待实现完成、能实际试用手感后，再决定它是值得单独立项的后续变更，还是点击选择已经够用。这个判断需要可运行的界面才能做出，且无论结论如何都不影响本变更的 specs、方案与任务拆分。
