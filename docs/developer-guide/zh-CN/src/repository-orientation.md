# 仓库导览

同一套 React 应用运行在两个运行时适配器之后:

```text
React components
      |
frontend service interfaces
      |
      +-- Web/mock adapters
      |
      +-- Tauri adapters --> Rust commands --> bounded contexts --> SQLite / CLI / OS
```

重要根目录:

| 路径 | 职责 |
| --- | --- |
| `src/components`, `src/main-layout`, `src/settings` | React 展示与交互 |
| `src/services` | 前端运行时无关的契约与适配器 |
| `src/types`, `src/contracts` | 与传输无关的 TypeScript 契约 |
| `src-tauri/src/commands` | 薄的 Tauri 命令与 DTO 映射边界 |
| `src-tauri/src/contexts` | native 领域、应用与基础设施归属 |
| `src-tauri/src/platform` | 共享的平台适配器,例如数据库、进程与日志 |
| `openspec/specs` | 已确认的行为需求 |
| `openspec/changes` | 活跃与已归档的变更证据 |
| `tests/e2e` | Playwright 用户可见的回归路径 |

从 `AGENTS.md` 和 `openspec/project.md` 开始。它们是规范性贡献者规则,优先于本指南中的解释性示例。

详细的 native 模块清单维护在 [`src-tauri/ARCHITECTURE.md`](../../reference/native-architecture.md) 与仓库源码中。组装好的指南将该已签入的 Markdown 作为参考副本复制,因此它不会与仓库文件发生漂移。
