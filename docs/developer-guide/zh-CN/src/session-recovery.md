# 会话恢复

受管理的生成是持久的：会话可以在崩溃、中断的 tool-use loop 或结构性不一致之后安全恢复，而不会丢失证据或重复执行工作。恢复状态**独立**于会话的生命周期状态进行追踪。

## 恢复状态与生命周期正交

每一个持久会话都带有一个恢复状态：`clean`、`reconciling`、`action_required` 或 `quarantined`：

- 一个 `failed` 会话，如果恢复状态为 `clean` 且没有活跃的 run，仍然接受新消息。
- 一个 `idle` 会话，如果恢复状态为 `action_required`，则在每一条受管理的提交路径上拒绝新的生成工作，直到某个允许的恢复动作成功为止。
- 一个 `quarantined` 会话——一种稳定的、不冒丢失证据风险就无法对账的结构性不一致——保持可读且可导出，但拒绝任何依赖该不一致状态的生成或变更。

## 持久执行标识与归属

每一个被接受的受管理生成都有一个稳定的 execution run id，与其会话和已持久化的消息相关联。在 provider 或 CLI 执行开始之前，该会话会原子地认领**至多一个**活跃的 execution run。在一个 run 活跃期间发起的竞争认领会被拒绝，且不会启动工作。

## 设计所在之处

本章用于引导贡献者。权威需求——恢复状态、持久执行标识与归属，以及允许的恢复动作——位于 spec 中。

- [openspec/specs/session-recovery](../../../../openspec/specs/session-recovery/spec.md)

会话持久性位于 `sessions` 限界上下文中；见 [Native bounded contexts](native-contexts.md)。
