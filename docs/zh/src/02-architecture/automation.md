# 自动化与洞察：定时任务、操作跟踪、通知与用量

> **把重复的事交给调度器，把长跑的事变得可见，把花掉的 token 变成可查的账**。

## 这一层解决什么问题

**这一组能力围绕"不用盯着"展开**：定时任务按周期自动触发，长时操作有排队与状态，完成后通知你，事后能查用量。

## 能力与运行时边界

| 能力 | 说明 | 运行时 |
|---|---|---|
| 定时任务 | 五种频率的周期性触发 | **仅桌面** |
| 任务启停 | 单个任务的启用开关 | **仅桌面** |
| 到期扫描 | 按当前时间取出到期任务 | **仅桌面** |
| 任务状态流转 | running / succeeded / failed 标记 | **仅桌面** |
| 长时操作跟踪 | 五类操作的排队、状态与逐行日志 | **仅桌面** |
| 通知中心 | 四类通知，支持全局与会话作用域 | 桌面 / Web |
| 用量采集 | 四个 CLI 各自的用量摄取，四维 token | **仅桌面** |
| 幂等重复采集 | 可反复调用而不重复计数 | **仅桌面** |
| 用量趋势 | 按 Agent 拆分的消耗趋势 | **仅桌面** |
| 桌面集成 | 悬浮助手、托盘、开机自启、后台生命周期 | **仅桌面** |

## 定时任务

### 频率模型

**五种频率**（`src-tauri/src/commands/sessions/dto.rs:105-123` 的 `ScheduledTaskFrequency`）：

| 频率 | 参数 |
|---|---|
| `Minutes` | `interval`（分钟间隔） |
| `Hours` | `interval`（小时间隔） |
| `Daily` | `time_of_day` |
| `Weekly` | `weekday` + `time_of_day` |
| `Monthly` | `day_of_month` + `time_of_day` |

**间隔必须为正**——`interval <= 0` 直接返回 `invalid_frequency`（`sessions/infrastructure/scheduled_tasks.rs:208-210`）。

**序列化用 `#[serde(tag = "kind", rename_all = "camelCase")]`**（`dto.rs:103-104`），即前端拿到的是带 `kind` 判别字段的联合类型。

### 为什么在 sessions 上下文

**定时任务实现在 `sessions` 上下文而非 `operations`**（`sessions/infrastructure/scheduled_tasks.rs`）——因为任务的本质是"按时创建一个会话执行"，它的产物是会话。

**核心函数**：

| 函数 | 行号 | 职责 |
|---|---|---|
| `list_scheduled_tasks` | `:26` | 列出任务 |
| `create_scheduled_task` | `:49` | 创建 |
| `set_scheduled_task_enabled` | `:112` | 启停 |
| `delete_scheduled_task` | `:136` | 删除 |
| `compute_next_run` | `:203` | 计算下次运行时间 |
| `due_tasks` | `:245` | 取出到期任务 |
| `mark_task_running` | `:270` | 标记开始 |
| `mark_task_succeeded` | `:277` | 标记成功 |
| `mark_task_failed` | `:292` | 标记失败 |

### 时区处理

**这是一处值得注意的分工**：

| 函数 | 时间类型 | 原因 |
|---|---|---|
| `compute_next_run` | `DateTime<Local>` | "每天早上 9 点"是用户本地时间概念 |
| `due_tasks` | `DateTime<Utc>` | 到期判定需要绝对时间，避免夏令时切换时重复或漏跑 |

**用本地时区算下一次、用 UTC 判到期**——两者各自用对了时间类型。

**但存储只有一种**（`scheduled_tasks.rs:241`）：

```rust,ignore
Ok(next.with_timezone(&Utc).to_rfc3339())
```

`compute_next_run` 收 `DateTime<Local>`、算完立刻转成 UTC 的 RFC3339 字符串再落库。**本地时区只存在于计算过程中，不进数据库**——这样换时区或跨夏令时都不会让已排期的任务错位。

### 频率校验在计算入口

（`scheduled_tasks.rs:203-241`）

| 频率 | 校验 |
|---|---|
| `Minutes { interval }` | `interval > 0` |
| `Hours { interval }` | `interval > 0` |
| `Daily { time_of_day }` | 时间可解析 |
| `Weekly { weekday, .. }` | `weekday ∈ 0..=6` |
| `Monthly { day_of_month, .. }` | `day_of_month ∈ 1..=31` |

**非法值直接返回错误而不是钳制**——`interval = 0` 会导致下次运行时间等于当前时间，任务会疯跑。

### 「只补最近一次」的根源是一个列

**到期扫描非常简单**（`scheduled_tasks.rs:245-268`）：

```sql
SELECT ... FROM scheduled_tasks
WHERE enabled = 1 AND next_run_at <= ?1
ORDER BY next_run_at ASC
```

**每个任务在表里只有一个 `next_run_at`**。应用关闭三天、任务是每天一次，重启后这一行仍然只有一个过期的 `next_run_at` 值——**扫描出一条，跑一次，然后重算下一次**。

**中间错过的两次没有任何地方记录，因此无法补**。这不是取舍后放弃，而是这个数据模型的直接后果：要补齐全部错过的运行，得存一个待办队列而不是一个时间戳。

测试名把这件事说明白了：`due_scan_returns_one_backfill_candidate_for_missed_task`（`scheduled_tasks.rs:491`）——**one** candidate。

**用户侧的表述**是「重启后补上错过的运行，且只补最近一次」，对应的就是这里。

由定时任务触发的执行在追踪中标记为 `ExecutionSource::Scheduled { task_id }`，见 [可观测性](observability-architecture.md#执行身份与关联)。

## 长时操作跟踪

**五类操作**（`src-tauri/src/contexts/operations/domain/operation.rs:6-12` 的 `OperationKind`）：

| 类别 | 典型场景 |
|---|---|
| `Sdk` | 受管 SDK 安装 / 升级 |
| `Mcp` | MCP server 连接 |
| `Agent` | Agent 相关长操作 |
| `Workspace` | 工作区操作 |
| `Extension` | 本地扩展安装（含模型下载） |

**五种状态**（`operation.rs:16-22` 的 `OperationStatus`）：`Queued`、`Running`、`Succeeded`、`Failed`、`Cancelled`。

**日志逐行记录**（`operation.rs:26-30` 的 `OperationLogEntry`）：每条带 `operation_id`、`line`、`timestamp`。

**这满足统一日志规范的一条要求**——SDK/CLI/任务类操作的输出必须**同时**保留页面内展示与统一日志目录写入，见 [可观测性](observability-architecture.md#写日志时必须遵守的四条)。

**序列化为 camelCase**（`operation.rs:24-25`），前端服务在 `src/services/operation-service.ts`，契约在 `src/contracts/operation.ts`。

## 通知

**四种类型**（`src/notifications/notification-types.ts:1`）：`success`、`error`、`warning`、`info`。

**两种作用域**（`notification-types.ts:3-5` 的 `NotificationScope`）：

| 作用域 | 形态 |
|---|---|
| 全局 | `{ kind: "global" }` |
| 会话级 | `{ kind: "session"; sessionId: string }` |

**会话级通知让"某个会话出错了"不会淹没在全局提示流里**——切到那个会话时才是最相关的上下文。

**toast 有三态**（`:7` 的 `NotificationToastState`）：`visible`、`exiting`、`hidden`——`exiting` 单独一态是为了播放退场动画而不立即卸载。

**模块划分**（`src/notifications/`）：

| 文件 | 职责 |
|---|---|
| `notification-provider.tsx` | 上下文提供 |
| `notification-center.tsx` | 通知中心列表 |
| `notification-toast-viewport.tsx` | toast 展示 |
| `notification-reducer.ts` | 状态归约（纯逻辑，可单测） |
| `notification-types.ts` | 类型定义 |

## 用量统计

### 四维 token

**每条用量记录四个维度**（`agent_runtime/infrastructure/terminal_usage_ingestion.rs:18-23` 的 `TerminalUsageTotals`）：

| 维度 | 含义 |
|---|---|
| `input_tokens` | 输入 |
| `output_tokens` | 输出 |
| `cache_read_tokens` | **缓存读取** |
| `cache_creation_tokens` | **缓存创建** |

**分开记缓存的两个方向很有用**：`cache_creation` 是一次性成本，`cache_read` 是节省下来的开销，两者对比才能判断提示词缓存是否划算。

### 四条独立的摄取路径

**每个 CLI 报告用量的方式不同，因此各有一条路径**（`terminal_usage_ingestion.rs`）：

| 函数 | 行号 | 对应 Agent |
|---|---|---|
| `ingest_claude_terminal_usage` | `:29` | `claude-code` |
| `ingest_opencode_terminal_usage` | `:66` | `opencode` |
| `ingest_codex_terminal_usage` | `:116` | `codex-cli` |
| `ingest_gemini_terminal_usage` | `:164` | `gemini-cli` |

**这里刻意没有抽象**：四个函数各自处理各自的格式。抽象一个"通用用量解析器"会把四种互不相干的格式硬塞进一个形状。代价是新增 CLI 必须新增一条。

### 幂等设计

**Claude 侧的摄取注释说明了调用契约**（`terminal_usage_ingestion.rs:25-28`）：

> 读取 claude-code 的会话 JSONL，并在 `message_id` 下 upsert 一条聚合用量记录。**可以安全地反复调用**（例如终端打开期间每几秒一次、退出时再来一次）：共享的 message 状态跨重启可恢复，且只在存在非零用量时才创建，**因此每次调用更新的都是同一行**。

**这个设计让采集策略变得简单**：不需要精确的"只采一次"逻辑，定时轮询 + 退出时补一次即可，重复调用不会重复计数。

**Claude 的用量按项目目录组织**，另有 `claude_project_dir_name(cwd)` 做目录名推导（`:292`）；`load_terminal_usage_message_id`（`:199`）负责恢复已有的 message 关联。

### 界面

设置中心 → 用量统计页（`src/settings/pages/usage-statistics-page.tsx`），子组件在 `src/settings/pages/usage/`：

| 组件 | 内容 |
|---|---|
| `usage-summary.tsx` | 汇总 |
| `usage-trend.tsx` | 趋势 |
| `usage-agent-breakdown.tsx` | 按 Agent 拆分 |
| `usage-controls.tsx` | 筛选控件 |
| `usage-status.tsx` | 状态 |
| `usage-format.ts` | 数值格式化 |
| `usage-query.ts` | 查询逻辑 |
| **`usage-accounting-note.tsx`** | **口径说明** |

**口径说明单独成一个组件**，说明这套数据存在需要向用户交代的前提——用量来自各 CLI 的自述，不是 VaneHub AI 独立计量的结果。

## 桌面集成

`desktop` 上下文（`src-tauri/src/contexts/desktop/`）通过 10 个端口承载桌面能力（`application/ports.rs:11-76`）：

| 端口 | 行号 | 职责 |
|---|---|---|
| `DesktopSettingsRepository` | `:11` | 设置持久化 |
| `DesktopClockPort` | `:27` | 时钟 |
| `DesktopNetworkProxyPort` | `:31` | 网络代理 |
| `DesktopLogDirectoryPort` | `:38` | 日志目录 |
| `DesktopStartupPort` | `:44` | 开机自启 |
| `DesktopLocalePort` | `:48` | 语言 |
| `DesktopDirectoryPort` | `:52` | 目录 |
| `DesktopNodeInfoPort` | `:62` | Node 信息 |
| `DesktopNetworkProxyActionsPort` | `:67` | 代理动作 |
| `DesktopClientLoggingPort` | `:76` | 前端日志上报 |

**生命周期端口另在** `application/lifecycle/ports.rs:4,11`：`DesktopLifecyclePort` 与 `DesktopShutdownPort`。

**对应界面**：

| 能力 | 入口 |
|---|---|
| 悬浮助手 | `src/floating-assistant/`、设置 `floating-assistant-settings-section.tsx` |
| 开机自启 | `startup-settings-section.tsx`（依赖 `tauri-plugin-autostart 2.5.1`） |
| 网络代理 | `network-proxy-section.tsx` |
| 数据管理 | `data-management-section.tsx` |
| 文件夹打开器 | `folder-openers-section.tsx` |

托盘由 Tauri 的 `tray-icon` feature 提供（`src-tauri/Cargo.toml`）。

## 界面入口与前端服务

### 创建定时任务

主界面活动栏（`src/main-layout/workspace-activity-bar.tsx`）打开定时任务对话框（`scheduled-tasks-dialog.tsx`），选择频率与要执行的内容。

### 查看用量

设置中心 → 用量统计页，用筛选控件按时间范围与 Agent 查看。**注意看口径说明**。

### 管理通知

通知中心查看历史；toast 用于即时提醒。会话级通知在对应会话内展示。

## 边界与限制

- **定时任务仅桌面可用** —— 依赖原生调度与 SQLite。
- **应用需在运行状态** —— 定时任务由应用内调度器驱动，完全退出后不会触发；后台生命周期行为见 `desktop-background-lifecycle` 能力。
- **用量数据来自各 CLI 自述** —— VaneHub AI 不独立计量 token，口径以各 CLI 报告为准。
- **四条摄取路径彼此独立** —— 某个 CLI 改变输出格式只影响它自己那条，但新增 CLI 需要新增实现。
- **OnePiece 不在终端用量摄取范围内** —— 四条路径都是 CLI 终端向的；原生 Agent 的用量走另外的记录路径。
- **通知在 Web/mock 下仅为应用内展示** —— 不产生操作系统级通知。

## 相关文档

- [可观测性](observability-architecture.md) —— 执行来源与统一日志
- [工具生态](tooling.md) —— SDK / MCP / 扩展操作的来源
- [会话管理](sessions.md) —— 定时任务创建的会话
- [限界上下文](bounded-contexts.md) —— `operations` 与 `desktop` 的职责
