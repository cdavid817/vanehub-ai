## Why

OnePiece 会话里每一次改执行模式、切推理深度、导出会话、跳到待办看板，都要离开输入框去点工具栏或侧边栏。用户已经在键盘上了，却必须换到鼠标才能完成这些操作。斜杠命令把这些既有能力变成键盘可达的入口，且不需要新增任何后端能力。

现在做的另一个理由是拦截点唯一：`api-session-composer.tsx` 是 `ChatInputBox` 的唯一生产消费者，此刻引入一层调度的成本远低于将来聊天入口分化之后。

## What Changes

- 新增前端斜杠命令运行时：解析器、可用性谓词、命令注册表、调度 hook。输入以 `/` 开头且匹配已注册命令时由前端执行，**不发送给模型**
- 输入框新增两个表面：`/` 触发的命令补全下拉，以及输入框上方承载命令输出的可关闭临时面板
- 命令输出存于前端独立状态，不写入消息流。原因是 `sendMessage.onSuccess` 会 invalidate `["messages", sessionId]` 触发重新拉取，注入消息缓存的本地条目活不过一轮对话
- 未知命令**不转发给模型**，以错误形式提示；`//` 前缀转义为字面 `/`，保证用户仍能发送真正以斜杠开头的文本
- 第一版命令集限于零后端改动的既有能力：`/mode` `/thinking` `/streaming` `/longcontext` `/export` `/status` `/usage` `/help`，以及导航类 `/plan` `/plans` `/loops` `/todo` 与八个工作区页签
- **移除** `/reasoning` 与 `/stop`（最终评审后删除，理由见下）：`/reasoning` 对这些会话必然是空操作——OnePiece 模型 `supportsReasoning: false` 使 `config.reasoningDepth` 恒为 `undefined`，工具栏也不渲染对应选择器供用户对照；`/stop` 的成功路径不可达——流式生成时输入框会整体撤下提交入口（`canSubmit` 恒假，Send 换成 Stop），命令永远无法在 `isStreaming` 为真时执行，唯一可达结果是报错
- 作用域限于 OnePiece 会话（`agentId === "onepiece"`），由单一谓词模块门控。多席位 CLI 会话的接入是后续变更，架构上只需放宽该谓词
- **不提供** `/model` `/provider` `/agent`：`api-session-composer.tsx` 无条件传入 `lockRuntimeIdentity`，`ButtonArea` 据此禁用了对应选择器，命令绕开该锁会与既有产品决策冲突
- **不包含** `/clear` 与 `/compact`：两者需要新的后端能力（删除消息并重置上下文、持久化的手动压缩），单独成后续变更

## Capabilities

### New Capabilities

- `slash-command-runtime`: 定义斜杠命令的输入解析、会话可用性门控、命令注册与查找、调度语义、输出呈现与失败上报。

### Modified Capabilities

- `chat-experience`: 「Chat input submits user messages」需要收窄——在启用斜杠命令的会话中，命令形态的输入由前端接管，不构成一条用户消息。

## Impact

- **Web/桌面双运行时**：本变更纯前端，不新增 Tauri command，`tauri-agent-client.ts` 与 `web-agent-client.ts` 均无需改动，两个适配器天然保持一致
- **前端**：新增 `src/services/slash-commands/`（解析、可用性、注册表、命令定义、调度 hook）与两个 `src/components/chat/` 组件；`ChatInputBox.tsx`、`api-session-composer.tsx`、`session-tabs.tsx`、`main-layout.tsx` 各有小幅接线改动
- **服务边界**：命令通过既有的 `AgentService` 方法（`exportSession`、`getSessionUsageSummary`）与 `MainLayoutModel` 回调工作，组件不直接调用 Tauri `invoke()`
- **`use-main-layout-model.ts` 零改动**：该文件 298/300 行且不在 `max-lines` 豁免清单内，因此调度层落在 composer 而非会话模型
- **i18n**：新增 `slash.*` 键，须覆盖 `en` / `ja` / `ko` / `zh-CN` / `zh-TW` 五个 locale，由 parity 测试强制
- **无新依赖**，无数据库迁移，无 Rust 侧改动
