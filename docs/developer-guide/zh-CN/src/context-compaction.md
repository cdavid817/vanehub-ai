# 上下文压缩

一次生成所累积的回合以**字符数累加值**衡量,而非按 provider 上报的 token 数。当运行总数超过固定阈值时,运行时会在发送下一个请求之前进行压缩。这一设计刻意避免依赖 provider 实际上报的 token 数来决定何时压缩。

## 压缩触发时机

- 低于阈值 → 请求原样发出。
- 会话的对话历史本身就超过阈值 → 在该生成首个请求之前进行压缩。
- 工具调用循环(tool-use loop)中累积的回合(工具调用结果)把总数推过阈值 → 在该循环的下一个请求之前进行压缩。

## 摘要式压缩

当压缩触发时,运行时会按原文保留固定数量的最近回合,并用一个携带模型生成摘要的合成回合替换所有更早的回合。摘要调用是一次针对保留窗口之前回合的单次 provider 调用;该摘要调用不声明任何工具。

## 设计所在

本章用于为贡献者定向。权威需求——字符数触发与摘要式压缩——位于 spec 中。

- [openspec/specs/agent-context-compaction](../../../../openspec/specs/agent-context-compaction/spec.md)

压缩运行在 `agent_runtime` 限界上下文内,与 [Tool registry and execution](tool-registry.md) 中描述的工具调用循环位于同一路径。
