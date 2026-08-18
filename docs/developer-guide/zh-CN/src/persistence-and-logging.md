# 持久化与统一日志

## SQLite 所有权

SQLite 只能从 Rust 基础设施访问。迁移有一个全局顺序,但每个 schema 与 repository 都归属于某个 bounded context。外键引用并不授予一个 context 直接查询另一个 context 表的权限。

迁移变更要求:

- 一个带版本的迁移;
- clean-database 与升级路径的覆盖;
- 显式的行到领域映射;
- 与当前 fixture 兼容;
- 在生产 command 边界上不得使用 `unwrap()` 或 `expect()`。

## 日志

native 诊断与操作输出都流经统一日志服务。禁止 feature 专用的日志文件。

持久化的事件必须:

- 带有 `error`、`warn`、`info` 或 `debug` 语义;
- 在落盘前对凭据、token、用户内容、路径与命令敏感值进行脱敏;
- 关联长时间运行的操作,但不把原始 prompt 或 Agent 输出放进诊断通道;
- 在其所属的结果存储中保留页面可见的操作输出。

React 不能写本地日志文件。需要持久化的前端错误会越过服务边界上报到 native 日志 command。Web/mock 行为可以暴露页面可见的模拟日志,但不能声称具备 native 持久化。

执行可观测性关联规则由 `openspec/specs/agent-execution-observability/spec.md` 与 `openspec/specs/unified-log-management/spec.md` 约束;语义/日志存储的拆分作为 ADR-002 记录在 `src-tauri/ARCHITECTURE.md` 中。
