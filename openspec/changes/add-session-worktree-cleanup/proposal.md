# 为删除会话增加可选、安全、可恢复的 worktree 清理

## Why

用户在创建会话时可以让 VaneHub AI 真实创建 Git worktree，但现有普通会话删除链路只停止会话活动并删除会话数据，工作目录与分支继续保留。默认保留成果是合理的；缺口在于删除时没有清晰的资源选择、风险预检查和后续结果说明。

本变更落实用户已经确定的交互：删除会话时弹出统一确认框，提供默认不勾选的「同时删除关联的 worktree」。它不是自动清理策略，也不是把删除聊天记录解释为删除项目。

源码证据基于 `main@d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5`，定位见 [code-map.md](code-map.md)。该 SHA 是研究基准，不是强制实施分支。Claude Code 必须先对当前 checkout 重新核实路径和规范；不得擅自切换、重置或覆盖用户工作。

## What Changes

- 单条删除、归档列表删除、搜索结果删除和批量删除，共用一个预检查与确认流程。
- 默认仅删除会话；普通项目、主 worktree 和远程工作区不会出现可执行的目录删除入口。
- 对可验证由应用创建的普通会话 worktree，提供 `keep` / `remove-safe` 两种策略。第一版始终保留 Git 分支，不提供强制删除、自动提交、自动暂存、自动合并或自动 prune。
- 后端只读预检查真实 Git 身份、归属证据、改动、忽略文件、共享引用和运行活动；执行前重新核验。读取失败或结果不完整不能等同于安全。
- 忽略文件单独提示并绑定本次快照确认；有已跟踪修改、暂存修改、冲突或非忽略的未跟踪文件时禁止清理。
- worktree 以独立、轻量的资源记录追踪。新创建资源记录来源；历史资源只有取得完整可验证证据才迁为可信，不能根据名称前缀推定所有权。
- 删除操作写入持久化日志和阶段状态，处理重复请求、停止失败、Git 失败、部分成功及重启恢复；Git 副作用不假装受 SQLite 事务回滚。
- 批量删除按真实 worktree 身份去重、按资源分组；一个分组失败不伪装为整批成功，也不静默改成只删除会话。
- Desktop/Tauri 提供真实磁盘行为；Web/mock 提供明确标识的模拟契约，不声称释放了本机空间。
- 补充用户指南、错误文案、无障碍交互、跨平台真实 Git 测试和桌面集成验收。

## Capabilities

### New Capabilities

- `session-deletion-operations`: 会话删除预览、持久化协调、幂等、引用门禁、部分完成与恢复。

### Modified Capabilities

- `session-management`: 删除确认、默认保留、单条/批量入口一致性、异步结果及适配器契约。
- `project-worktree-management`: 普通会话 worktree 来源追踪、真实身份检查、保守清理与保留资源状态。

## Scope

### In scope

普通本地会话的 worktree 删除选择及安全执行；其最低必要依赖是资源归属、引用协调和可恢复操作。创建路径只补充来源记录与删除门禁接入，不在本变更重做会话创建体验。

### Non-goals

- 不建设完整 worktree 管理中心、不自动扫描全盘、不做定时或 TTL 垃圾回收。
- 不改变 Loop worktree 的审查保留策略、不复用 OnePiece 子 Agent 的临时 worktree 强制回收逻辑。
- 不删除远程 SSH 目录、不删除外部 worktree、不迁移或修复未知 Git 元数据。
- 不删除任何分支、提交、stash、tag 或远端资源。
- 不顺带修复创建框失焦重置、子目录命名、分支名称输入等独立问题。历史异常路径在此处拒绝清理，相关创建问题另起 change。
- 不升级 React、引入新的状态管理库、重构全局 OperationStatus 或擅自重排迁移号。
- 不声称应用门禁能阻止外部编辑器、其他 Git 客户端或恶意本机进程。

## Impact

**Desktop runtime:** 通过现有 React service → Tauri adapter → commands → application service → ports/adapters 分层接入；Git、SQLite、进程停止和路径核验全部在 Rust 侧。阻塞 Git 和进程等待放入受管理阻塞工作线程，不阻塞 Tauri 命令主线程。

**Web runtime:** 补全相同 DTO、预览/执行/重试 API 和可控模拟场景，保留 `web-http` 缺失 adapter 时的显式失败；不回退伪造真实删除成功。

**Database:** 增量迁移增加轻量 worktree 来源/状态、删除操作及分组数据；优先复用已有可满足约束的设施。迁移不得执行文件系统删除，资源与恢复记录不得跟随会话外键级联丢失。

**Compatibility:** 旧 `deleteSession(sessionId): Promise<void>` 可作为仅保留 worktree 的适配包装器，仍必须经过相同会话删除互斥与停止边界。所有用户可见入口迁到显式确认 API。无需兼容独立旧客户端，但不能给内部调用偷偷引入磁盘副作用。

## Acceptance

完成 [tasks.md](tasks.md) 与 [acceptance-tests.md](acceptance-tests.md)，逐条映射 delta 场景。只有在真实临时 Git 仓库验证目录、Git 登记与分支保留，并有恢复/竞态测试后，才能标记实现完成。Web/mock 截图不构成真实文件系统删除的证据。

## Delivery State

本包是待实现的 OpenSpec change；全部实施任务保持未勾选。包结构自检与真实项目验证分开记录，见 [verification.md](verification.md)。不要把收到设计包视为代码已实现或三平台已验证。
