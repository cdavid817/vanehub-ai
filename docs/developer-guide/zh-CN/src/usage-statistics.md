# 使用统计

VaneHub 为 VaneHub 管理的助手响应记录逐响应的使用量，并在设置中心对其汇总。不存在外部计费集成；这是第一版的本地使用量核算。

## 上报 token 与估算字符

两类数据被严格区分：

- **上报 token** —— fresh-input、output、cache-read、cache-creation 和 total，取自 provider 上报的使用量。上报的 total 等于上述四类之和。
- **估算字符** —— input、output 和 total，在 provider 上报使用量不可用时由字符计数推导而来。估算字符绝不会加到任何上报 token 总数中。

统计还会返回上报/估算/总计的已计数响应数、已计数会话数、每日趋势点、按稳定 Agent id 索引的每 Agent 分项行，以及由上报使用量支撑的已计数响应百分比。没有记录的时间范围会返回零值总计和空数组，而不是失败。

## 时间范围

支持的范围包括：今天、最近七天、最近三十天和全部时间，基于当前运行时的用户本地日历计算。

## 设计所在之处

本章用于引导贡献者。权威需求位于 spec 中。

- [openspec/specs/usage-statistics](../../../../openspec/specs/usage-statistics/spec.md)

使用量持久化位于 `sessions` 限界上下文中；见 [Native bounded contexts](native-contexts.md)。
