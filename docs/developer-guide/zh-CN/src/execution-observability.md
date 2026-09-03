# 执行可观测性

执行链路怎么被记录、怎么分层、哪些节点是可见保真度、哪些到边界为止。

Agent 评测竞技场跑在同一套 Operation 生命周期上，但它是另一个问题域，见[评测运行时](evaluation-runtime.md)。

链路与评测共享同一条原则：**只记录能证实的东西，并如实声明自己知道多少**。

## 执行链路

### 四个核心类型

| 类型 | 是什么 |
| --- | --- |
| `ExecutionRun` | 一次可观测的执行，持有 trace id 与状态 |
| `ExecutionSpan` | run 内的一段，名称上限 128 字符 |
| `ExecutionEvent` | span 上的时点事件 |
| `ExecutionTimeline` | 供界面展开的时间线视图 |

`ExecutionStatus` 六态：`Accepted`、`Running`，以及四个终态 `Succeeded`、`Failed`、`Cancelled`、`Incomplete`——`is_terminal()` 判定的就是后四个。

**`Incomplete` 是一个终态，不是中间态**。它表示这次执行结束了但链路没记全，与「失败」区分开：失败是执行的结论，不完整是观测的结论。

### 保真度：链路自己声明知道多少

`ExecutionFidelity` 四档，是这个上下文最重要的设计：

| 保真度 | 含义 |
| --- | --- |
| `Native` | 运行时自己产生的一手记录 |
| `Proxied` | 经由中继观察到的 |
| `Inferred` | 从可得信号推断出来的 |
| `Opaque` | 这一段发生了什么无从得知 |

**为什么必须有 `Opaque`**：外部 CLI Agent 是黑盒，VaneHub 启动进程、采集输出，但看不到它内部的工具调用。如果把这种情况画成一个看似完整的 span 树，读者会以为自己看到了全部。声明 `Opaque` 是在说「这里确实有一段，但我不知道里面是什么」——这比补一个编造的节点诚实，也比干脆不画有用。

OnePiece 走 native API，其工具调用是 `Native` 保真度、可逐层展开；这正是[原生 Agent](onepiece-native-agent.md)相对外部 CLI 的可观测性优势。

### 采集策略与脱敏

`CapturePolicy` 只有两档：`MetadataOnly` 与 `RedactedContent`。**没有「原始内容」这一档**——即使切到最详细的采集，内容也是脱敏后的。

属性有硬上限，超限即拒绝而非截断：

| 限额 | 值 |
| --- | --- |
| 每组属性数量 | **32** |
| 属性键长度 | **128** 字符 |
| 属性值长度 | **256** 字符 |

类型是 `SafeAttributes` / `SafeAttributeValue`——**「安全」写进了类型名里**，构造时就校验，而不是落盘前再过一遍脱敏。想往链路里塞一段任意长的文本，编译期就过不去。

### 执行来源

`ExecutionSource` 区分三种发起方：`Desktop`、`InstantMessage { connector_id }`、`Scheduled { task_id }`。IM 与定时任务带上各自的标识，所以「这次执行是谁触发的」在链路里是一等信息，而不是靠时间戳猜。

## 与其他上下文的关系

- 统一日志与脱敏规则见[统一日志](unified-logging.md)。**链路与日志可以按 `runId`/`traceId`/`spanId` 关联**——`AgentRuntimeLoggingAdapter::record` 会把这三个字段写进日志条目的 `context`。只有外部 CLI 的内部行为、旧记录或上下文缺失的降级路径才需要退回按时间对齐。
- Operation 生命周期由 `operations` 拥有。
- 用户侧界面见用户指南的可观测性一章。

## 设计所在

本章用于为贡献者定向，权威需求位于 `openspec/specs` 下对应能力的主规范中。
