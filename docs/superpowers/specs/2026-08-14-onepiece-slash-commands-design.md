# OnePiece 原生 Agent 斜杠命令设计

日期：2026-08-14
状态：设计已确认，待起 OpenSpec proposal

## 目标

在 VaneHub AI 的聊天输入框中引入**应用级斜杠命令**：用户输入 `/xxx` 后由前端拦截并执行，不发送给模型。第一版只对 OnePiece 原生 Agent 会话开放，但架构必须让多席位 CLI 会话在后续版本以低成本接入。

这批命令与底层 CLI（Claude Code、Codex 等）自带的斜杠命令无关，VaneHub 不发现、不转发、不管理它们。

## 现状与约束

### 拦截点

`src/session-workspace/api-session-composer.tsx` 是 `ChatInputBox` 唯一的生产消费者，它把 `onSubmit` 接到 `use-main-layout-model.ts` 的 `submit()`。OnePiece 会话的全部用户输入都经过这一个函数。

拦截**放在 composer 而不是 model**，原因有二：

1. `use-main-layout-model.ts` 是 298 行，`max-lines` 硬上限为 300 且该文件不在 `eslint.config.js` 的技术债豁免清单里，没有增长空间
2. 命令层属于 UI 关注点，不应污染核心会话模型

`main-layout.tsx` 虽在豁免清单内，但已达 460 行（清单注释仍写着 341），同样不适合承载新逻辑。

### 会话类型判据

`InteractionMode` 为 `"browser" | "native-desktop" | "cli" | "api"`。相关判据：

- OnePiece 会话：`session.agentId === "onepiece"`，该字面量已是既定判据（`useChatConfig.ts:10,104,115`、`ButtonArea.tsx:88`）
- 多席位 CLI 会话：`interactionMode === "cli"` 且 `activeSeatsFromSession(session).length > 1`

注意 `ApiSessionComposer` **不是** OnePiece 专属的。`main-layout.tsx:214` 的渲染条件是 `interactionMode === "api" || 多席位`，多席位 CLI 会话也走同一个 composer。因此作用域必须按 `agentId` 门控，不能按 composer 划分。

### 运行时开关的既有锁

`api-session-composer.tsx:23` 无条件传入 `lockRuntimeIdentity`，而 `ButtonArea.tsx:79,104` 用它禁用了 `ProviderSelect` 与 `ModelSelect`。API 会话中切换模型与 provider 是产品上刻意锁死的行为。

因此 `/model` 与 `/provider` **不做**——斜杠命令绕开 UI 的锁就是缺陷。`/agent` 同样不做：`useChatConfig.ts:115` 在 OnePiece 会话中把候选代理过滤成只剩自身，没有可切换的对象。

### 上下文压缩的现状

压缩实现在 `src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter.rs`：

- 阈值 `COMPACTION_TRIGGER_CHARACTERS = 60_000` 字符（第 78 行）
- 保留最近 `COMPACTION_KEEP_RECENT_TURNS = 6` 轮（第 81 行）
- 触发点在第 1941 行，改写即将发给 provider 的 `turns` 向量

关键性质：**压缩是请求作用域的，不落库**。存储的消息原封不动，每次生成重新压一遍，系统中不存在"已压缩状态"这个概念。手动 `/compact` 因此需要引入新的持久化概念。

### 数据库外键

`messages` 表仅有 `session_id` 外键指向 `sessions`（`migrations.rs:1102`）。反向引用只有一处：

```sql
-- usage_accounting.rs:50
FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE SET NULL
```

`model_invocations` 的用量记录按 `session_id` 挂靠，删除消息只会把 `message_id` 置空。所以 `/clear` 不会破坏会话的 token 成本记录，只丢失逐条消息的归因。`model_invocations.purpose` 枚举中已含 `'context-compaction'`，手动压缩的开销天然进账。

## 架构

### 新增模块

`src/services/slash-commands/`：

| 文件 | 职责 |
| --- | --- |
| `command-registry.ts` | 命令定义表 |
| `parse-command.ts` | 解析 `/name arg…`，纯函数无依赖 |
| `command-availability.ts` | 唯一的门控真源 |
| `command-context.ts` | 定义 handler 可访问的能力 |
| `use-slash-commands.ts` | hook：持有输出面板状态，暴露 `tryDispatch(draft): boolean` |

新增组件：

- `src/components/chat/SlashCommandCompletion.tsx`：`/` 触发的补全下拉，沿用 `SeatMentionCompletion` 的既有模式
- `src/components/chat/SlashCommandOutput.tsx`：输入框上方的可关闭临时面板

### 扩展缝

门控收敛到一个模块、一个谓词，不散落到各处：

```ts
// command-availability.ts
export const isOnePiece = (s: Session) => s.agentId === "onepiece";
export const isMultiSeatCli = (s: Session) =>
  s.interactionMode === "cli" && activeSeatsFromSession(s).length > 1;

export const slashCommandsEnabled = (s: Session) => isOnePiece(s);  // v1 到此为止
```

每条命令自带 `appliesTo`：

```ts
type SlashCommand = {
  name: string;
  aliases?: string[];
  category: "session" | "runtime" | "navigation" | "info";
  appliesTo: (session: Session) => boolean;   // 扩展缝
  run: (ctx: CommandContext, args: string[]) => Promise<CommandOutcome>;
};
```

后续放开多席位 CLI 会话 = 把 `slashCommandsEnabled` 改成 `isOnePiece(s) || isMultiSeatCli(s)`，再逐条调整 `appliesTo`。dispatch、补全、输出面板均无需改动。

这不是为将来预付成本：`/clear` 与 `/compact` 本就必须按会话类型分流（压缩逻辑位于 `api_process_adapter.rs`，CLI 会话不走这条路径），该谓词当前即为必需。

### 数据流

用户按下 Enter → composer 先调 `tryDispatch(draft)` → 命中则执行 handler、清空 draft 并返回，模型完全不感知；未命中则原样走 `model.submit()`。

补全下拉在 `value` 匹配 `/^\/(\S*)$/` 时渲染。该正则与既有的 `@` 补全（`(?:^|\s)@([^\s@]*)$`，`ChatInputBox.tsx:70`）互不干扰。

### 命令输出的归属

命令输出存于独立的 React state，渲染成输入框上方的可关闭临时面板，**不进入消息流**。

原因：`sendMessage.onSuccess` 会调用 `invalidateRuntime()`（`use-main-layout-model.ts:155`），它 invalidate `["messages", activeSessionId]` 触发从后端重新拉取。注入消息缓存的本地系统消息会在用户下一次发消息后被后端数据冲掉，活不过一轮对话。

代价是刷新即失，且引入一种新的 UI 形态。收益是零后端改动，且 `/help` 这类工具性输出不会污染对话历史与导出结果。

需要区分两个概念：上述"命令输出"指 `/help`、`/usage`、`/status` 面向用户的即时反馈，不落库；而 `/compact` 触发的压缩通知 rich block 是**会话事件**，由后端在压缩发生时写入，遵循既有 spec 的「Visible compaction notice」要求，与本节无关。

## 命令清单

| 类别 | 命令 | 依托 | 后端成本 |
| --- | --- | --- | --- |
| 会话 | `/clear` | 新后端能力，需二次确认 | 大 |
| 会话 | `/compact` | 新后端能力 | 大 |
| 会话 | `/export [md\|json]` | `model.exportSession` | 无 |
| 运行时 | `/mode <mode>` | `chatConfig.setSessionExecutionMode` | 无 |
| 运行时 | `/reasoning <low\|medium\|high>` | `chatConfig.setReasoningDepth` | 无 |
| 运行时 | `/thinking [on\|off]` | `chatConfig.setThinking` | 无 |
| 运行时 | `/streaming [on\|off]` | `chatConfig.setStreaming` | 无 |
| 运行时 | `/longcontext [on\|off]` | `chatConfig.setLongContext` | 无 |
| 导航 | `/plan` | `onOpenPlan`，仅当存在关联计划运行 | 无 |
| 导航 | `/todo` `/plans` `/loops` | `setDestination`（`main-layout.tsx:114`） | 无 |
| 导航 | `/logs` `/files` `/changes` `/documents` `/terminal` `/shell` `/traces` `/report` | `SessionTabId` 激活 | 无 |
| 信息 | `/help` | 注册表自省，按 `appliesTo` 过滤 | 无 |
| 信息 | `/usage` | `agentService.getSessionUsageSummary` | 无 |
| 信息 | `/status` | 当前 mode / reasoning / thinking / streaming / 席位 | 无 |
| 其他 | `/stop` | `model.stop()` | 无 |

导航命令涉及两条正交的轴：`destination`（`"sessions" | "loops" | "plans" | "todo-board"`）与 `SessionTabId`（`chat | changes | documents | files | terminal | shell | logs | traces | report`）。两者的 state 都在 `main-layout.tsx`，而 `ApiSessionComposer` 已在从那里接收 `onOpenPlan`，navigation 回调沿用同一条既有通路即可。

`/plan` 与 `/plans` 是两条不同的命令，须在 `/help` 中明确区分：`/plan`（单数）打开当前会话**关联的那次计划运行**，仅当 `chatConfig.associatedPlanRun` 存在时可用；`/plans`（复数）切换到全局的计划中心 destination，始终可用。

`/todo`、`/plans`、`/loops` 会切走 destination，从而隐藏聊天区（`main-layout.tsx:271`）。这是导航命令的预期行为。

## 后端

### `/clear`

新增 Tauri command `clear_session_messages(session_id)`：

- `DELETE FROM messages WHERE session_id = ?`
- 外键自动把 `model_invocations.message_id` 置 NULL，用量记录保留
- 一并清除该会话的压缩记录
- 会话本身、分类、工作区、配置均不受影响

`AgentService` 接口、`tauri-agent-client.ts`、`web-agent-client.ts` 三处同步实现。前端需二次确认。

### `/compact`

采用**非破坏性**方案。新增表：

```sql
CREATE TABLE IF NOT EXISTS session_context_compactions (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    summary TEXT NOT NULL,
    up_to_message_id TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (up_to_message_id) REFERENCES messages(id) ON DELETE SET NULL
);
```

新增 Tauri command `compact_session_context(session_id)`：读取会话 turns，复用 `api_process_adapter.rs` 中现成的摘要调用（绕过 `should_compact` 的 60k 阈值），写入一条压缩记录，并发出压缩通知 rich block。

请求组装路径改为：先读最新压缩记录，`turns = [摘要 turn, ...该记录之后的消息]`。原有的自动压缩仍在此基础上叠加，两者共存。

不去破坏性重写 `messages` 表，理由有三：

1. 用户预期 `/compact` 是省 token，不是删聊天记录
2. 既有 spec 的「Visible compaction notice」已确立"压缩是可见事件"而非"历史消失"的语义
3. 可逆——将来若要加 `/uncompact`，只需删除记录

### 迁移版本号风险

新表需占用一个 `migrations.rs` 版本号。本仓库多 worktree 并行开发时撞过迁移版本号，症状是启动时报 "no such table"。实现时必须先确认号段未被其他分支占用。

## 错误处理

- 未知命令**不转发给模型**，输出面板提示"未知命令，试试 `/help`"。静默转发会让用户误以为消息已发出
- 参数非法（如 `/mode nonsense`）时列出该命令的合法取值
- `appliesTo` 为 false 的命令不出现在补全中；硬输入则提示不适用于当前会话类型
- 后端命令失败走既有的 `reportChatFailure` 通路（notify + `settingsService.reportClientLogEvent`），与 `use-main-layout-model.ts:66` 保持一致
- `isStreaming` 时禁用 `/clear` 与 `/compact`，提示先停止生成
- `//` 转义为字面 `/`，使用户能发送真正以斜杠开头的文本

## 测试

| 层 | 内容 |
| --- | --- |
| `parse-command.test.ts` | 空参数、多空格、`//` 转义、大小写 |
| `command-registry.test.ts` | 每条命令的 `appliesTo` 矩阵（OnePiece / 多席位 CLI / 单席位 CLI） |
| `use-slash-commands.test.tsx` | dispatch 命中与否、输出面板、错误路径 |
| `SlashCommandCompletion.test.tsx` | 补全过滤，且与 `@` 补全互不干扰 |
| Rust | `clear_session_messages` 的外键行为、`compact_session_context` 的写入与请求组装读取 |
| Playwright | `/help` 出面板、`/mode` 改变工具栏、`/clear` 二次确认后消息清空 |
| i18n | 五 locale（en / ja / ko / zh-CN / zh-TW）key 完整性，沿用 `builtin-tool-locales.test.ts` 的写法 |

命令名保持英文不翻译，描述与输出文案需覆盖全部五个 locale。

## 实施切分建议

本设计跨越两类差异很大的工作，建议拆成两个 OpenSpec change 顺序落地：

1. **斜杠命令框架与零后端命令**：注册表、解析、dispatch、补全、输出面板，以及清单中全部标注"后端成本：无"的命令。纯前端，可独立交付并验证交互手感
2. **`/clear` 与 `/compact` 的后端能力**：新 Tauri command、新表与迁移、请求组装路径改造。依赖第 1 项的框架已就位

第 2 项中 `/compact` 的持久化压缩会改动 OnePiece 运行时的请求组装路径，风险高于其余全部工作之和，单独成 change 便于回滚与验证。

## 明确不做

- `/model`、`/provider`、`/agent`：与 `lockRuntimeIdentity` 的既有产品决策冲突
- 底层 CLI 自带斜杠命令的发现、补全与转发
- 用户自定义命令的 CRUD 管理页
- 命令输出的持久化与导出
