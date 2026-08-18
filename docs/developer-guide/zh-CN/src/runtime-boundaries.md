# 运行时与服务边界

React 组件依赖带类型的 frontend service。它们禁止直接导入 Tauri `invoke()`、打开 SQLite、启动 CLI 或直接访问本地文件系统。

## 桌面路径

1. 组件调用一个 service 接口。
2. Tauri frontend adapter 将请求映射到一个已声明的 command。
3. 薄薄的 Rust command 校验并映射 transport DTO。
4. 拥有该能力的 native application service 通过注入的 port 执行用例。
5. Infrastructure adapter 执行 SQLite、进程、文件系统、网络或 OS 相关工作。

可能较慢的工作会在完成前返回一个操作标识，并通过 operations 边界暴露进度。

## Web/mock 路径

Web adapter 以确定性的内存态实现同一套 frontend 契约。它可以为 UI 开发模拟执行与时序，但禁止声称本地进程已运行、SQLite 已变更或某个操作系统动作已发生。

## 新增能力

- 先扩展与运行时无关的 service 接口。
- 当 UI 消费该能力时，同时实现 Tauri adapter 和 Web/mock adapter。
- 将 provider 特定的启动行为保留在 Agent Runtime infrastructure 之后。
- 对用户可见的错误保持本地化，native 诊断写入统一的脱敏日志管道。

TypeScript 模型契约生成的决策（`ts-rs`）记录为 `src-tauri/ARCHITECTURE.md` 中的 ADR-005。早期的单 CLI chat 运行时叙事已被多 Agent group chat 运行时（`openspec/specs/multi-agent-group-chat/`）取代。
