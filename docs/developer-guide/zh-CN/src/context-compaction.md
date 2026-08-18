# 上下文压缩

上下文压缩由一个**版本化的 Token-aware 决策**驱动:当有可验证的模型容量与 Token 计量证据时,以 Token 占用达阈值为权威触发;当该证据不可用或分析失败时,回退到固定的字符计数触发。运行时在发送下一个请求之前进行压缩,且不会臆造容量或 Token 值。

## 触发优先级:Token-aware 主,字符回退

`select_authoritative_compaction`(`agent_runtime/domain/context_compaction_control.rs`)按以下优先级决定是否压缩:

- **Token-aware(主路径)** —— 当 Token 计量证据充分时,按版本化阈值(`context_window_tokens - reserve - buffer`)判定;`CompactionTriggerSource::TokenAware`。
- **字符回退** —— 当 Token 证据不足(`should_compact = None`)时,用固定字符计数阈值判定;`CompactionTriggerSource::CharacterFallback`。

运行时还会记录 token 决策与字符决策的"分歧"(disagreement),供观测。spec 明确要求:Token-aware 生产决策是权威触发,字符计数是兼容回退,二者不可颠倒。

## 压缩触发时机

- 触发判定为否 → 请求原样发出。
- 会话对话历史本身已超阈值 → 在该生成的首请求之前压缩。
- 工具调用循环(tool-use loop)中累积的回合把总数推过阈值 → 在该循环的下一个请求之前压缩,避免循环中途上下文无限膨胀。

## 摘要式压缩

当压缩触发时,运行时按原文保留固定数量的最近回合,并用一个携带模型生成摘要的合成回合替换所有更早的回合。摘要调用是一次针对保留窗口之前回合的**单次 provider 调用,且不声明任何工具**——摘要本身不会触发新的工具循环。

## 压缩控制与冷却

自动压缩受 `AutomaticCompactionState` 与 `AutomaticCompactionMode` 控制(`context_compaction_control.rs`):

- **`AUTOMATIC_COMPACTION_COOLDOWN_CHARACTERS = 8192`** —— 这是**冷却阈值**(不是触发阈值):距上次成功压缩的字符增长须超过此值才允许再次压缩,避免频繁压缩。`growth_since_success < 8192` 时以 `CompactionBypassReason::Cooldown` 旁路。
- **`AutomaticCompactionMode`** —— 默认自动压缩;`Suppressed` 模式下即使超阈值也不自动压缩,由调用方(如长时 tool-use loop 的上层)接管时机。
- **`AutomaticCompactionState`** —— 记录 `user_preference_enabled`、`last_success_characters`、`consecutive_failures`、`circuit_open`;连续失败达 `AUTOMATIC_COMPACTION_FAILURE_LIMIT`(2)后熔断旁路。
- **`CompactionBypassReason`** —— `RequestSuppressed`、`UserPreferenceSuppressed`、`Cooldown`、`CircuitOpen`。
- **`AUTOMATIC_COMPACTION_POLICY_VERSION = "onepiece-automatic-compaction-control-v1"`** —— 版本化决策标识。

## 关键类型与字段归属

注意字段归属,不要混淆:

- `compaction_triggered: bool` 属于 `ContextEvidenceManifest`(`context_engine.rs`),记录某次生成是否触发过压缩。
- `reserved_recent_turns: u64` 属于 `ContextBudget`(`context_engine.rs`),OnePiece 路径取值 `12_288`;它是上下文预算里"保留最近若干回合不被压缩"的配置,不在 `AutomaticCompactionState` 上。

## 设计所在

本章用于为贡献者定向。权威需求——Token-aware 触发与字符回退——位于 spec 中。

- [openspec/specs/agent-context-compaction](../../../../openspec/specs/agent-context-compaction/spec.md)
- [openspec/specs/agent-context-compaction-control](../../../../openspec/specs/agent-context-compaction-control/spec.md)

压缩运行在 `agent_runtime` 限界上下文内,与 [Tool registry and execution](tool-registry.md) 中描述的工具调用循环位于同一路径。
