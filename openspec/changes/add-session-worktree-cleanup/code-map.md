# 源码定位与证据基线

- 核查日期：2026-09-05。
- 基线：`main@d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5`。
- 本表区分已读取的基线事实与设计提出的新模块；实施前对当前 checkout 复核。
- 未访问你的本机数据库、会话或实际 worktree，也未在真实仓库执行删除。

## 已有源码与规范

| 引用 | 路径 | 已观察到的用途/行为 |
| --- | --- | --- |
| R01 | [AGENTS.md](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/AGENTS.md) | 仓库规范：React 19、npm、service 隔离、300 行、workspace 检验与桌面测试要求。 |
| R02 | [openspec/config.yaml](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/openspec/config.yaml) | schema: spec-driven；proposal/design/specs/tasks 的格式及 runtime 边界约束。 |
| R03 | [openspec/specs/session-management/spec.md](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/openspec/specs/session-management/spec.md) | 现有 Session mutation operations 需求及五个原场景，本 change 完整保留后追加删除约束。 |
| R04 | [openspec/specs/project-worktree-management/spec.md](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/openspec/specs/project-worktree-management/spec.md) | 普通会话 worktree 创建及 Loop review retention；本 change 不改 Loop 保留需求。 |
| R05 | [src/services/session-lifecycle-service.ts](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/src/services/session-lifecycle-service.ts) | deleteSession(sessionId): Promise<void>；没有目录清理策略。 |
| R06 | [src/main-layout/session-sidebar.tsx](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/src/main-layout/session-sidebar.tsx) | confirmDelete 调用 onBatchDelete 后 exitBatch；批量反馈接入点。 |
| R07 | [src/main-layout/main-layout.tsx](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/src/main-layout/main-layout.tsx) | 将 model.deleteSessions 传给 onBatchDelete；统一删除入口接线位置。 |
| R08 | [src-tauri/src/commands/sessions/delete_session.rs](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/src-tauri/src/commands/sessions/delete_session.rs) | 旧命令只有 session_id；调用后发布 active-session-changed(None)，须改成真实状态事件。 |
| R09 | [src-tauri/src/contexts/sessions/application/service.rs](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/src-tauri/src/contexts/sessions/application/service.rs) | 普通删除先 stop_session_activity 再 transactions.delete_session；普通创建先准备 worktree 后创建 session record。 |
| R10 | [src-tauri/src/contexts/sessions/infrastructure/transactions.rs](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/src-tauri/src/contexts/sessions/infrastructure/transactions.rs) | DELETE sessions 与 clear_active 的 SQLite 事务；不负责 Git。 |
| R11 | [src-tauri/src/contexts/sessions/infrastructure/runtime_support.rs](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/src-tauri/src/contexts/sessions/infrastructure/runtime_support.rs) | stop_generation、回收后台命令、kill_shells_for_session；不等于已验证所有写入者完全静止。 |
| R12 | [src-tauri/src/contexts/workspaces/application/service.rs](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/src-tauri/src/contexts/workspaces/application/service.rs) | 普通与 Loop worktree 创建服务；新增清理应在 workspaces 边界扩展。 |
| R13 | [src-tauri/src/contexts/workspaces/infrastructure/git.rs](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/src-tauri/src/contexts/workspaces/infrastructure/git.rs) | 现有 GitAdapter、worktree add、诊断与 timeout；清理可复用边界但不能复用不存在的安全 remove。 |
| R14 | [src-tauri/src/platform/filesystem/mod.rs](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/src-tauri/src/platform/filesystem/mod.rs) | sibling_worktree_target 及 Windows 展示路径规范化；不能把显示规范化等同删除身份认证。 |
| R15 | [src-tauri/src/contexts/agent_runtime/infrastructure/subagent_worktree.rs](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/src-tauri/src/contexts/agent_runtime/infrastructure/subagent_worktree.rs) | ChildWorktree 的 Drop 有临时资源强制清理；不得用于普通会话 worktree。 |
| R16 | [src/services/web-session-lifecycle-client.ts](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/src/services/web-session-lifecycle-client.ts) | Web/mock 会话与 worktree 元数据模拟；没有真实磁盘清理。 |
| R17 | [src/services/runtime-adapter.ts](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/src/services/runtime-adapter.ts) | web-http 未提供 adapter 时显式报错，不允许为了本能力回退 mock。 |
| R18 | [src/services/runtime-agent-client.ts](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/src/services/runtime-agent-client.ts) | Tauri 与 Web/mock service 的组合入口。 |
| R19 | [docs/user-guide/zh-CN/src/worktree.md](https://github.com/cdavid817/vanehub-ai/blob/d6e1d6ff2f89e0e58c984ef817b1ec41f08b63f5/docs/user-guide/zh-CN/src/worktree.md) | 路径/分支与 Loop 保留说明；新增普通会话删除说明的用户文档位置。 |

## 2026-09-05 当前 checkout 复核结果

实施 checkout：`worktree-main-20260905`，基于 `main@d6e1d6ff`，工作区干净。逐项复核后与基线的差异如下；未列出的条目与基线一致。

| 主题 | 复核结果 |
| --- | --- |
| 迁移顶号 | `EXPECTED_MIGRATIONS` 顶号为 111（`permission-grant-canonical-identity`）。本变更使用 112 `managed-worktree-resources`（workspaces 拥有）与 113 `session-deletion-operations`（sessions 拥有）。 |
| 跨进程锁 | 仓库无共享 platform 原语；仅 `contexts/personalization/infrastructure/memory_directory_lock.rs` 用 `std::fs::File::try_lock` 实现上下文私有锁。跨上下文 import 违反架构规则，因此本变更在 `platform/filesystem/advisory_lock.rs` 新增通用 OS 文件锁，供 workspaces 的 use gate 使用。 |
| 单实例 | 未使用 tauri single-instance 插件，多实例仲裁是真实需求。 |
| 定时任务引用 | `scheduled_tasks` 表没有工作目录列，只有 `latest_run_session_id`；定时任务对 worktree 的引用经由其运行会话，而不是独立路径绑定。设计中「禁用但仍保存路径的任务」在当前 schema 下不存在，引用检查以会话为准。 |
| Loop 引用 | `loop_runs.worktree_path` 与 `sessions.loop_run_id` 存在；Loop worktree 通过 `loop_runs` 行识别，本能力只把它们作为 external/loop 来源排除。 |
| 删除入口 | 单条：`session-context-panel.tsx` 内联 modal（非 ApplicationDialog）→ `main-layout.tsx` `onDelete={model.deleteSession}`；批量：`session-sidebar.tsx` `confirmDelete` 调 `onBatchDelete` 后立即 `exitBatch`；`use-main-layout-model.ts` 用 `Promise.allSettled` 逐个调 `agentService.deleteSession`。搜索结果与归档列表复用同一 card 与右键菜单，没有独立入口。 |
| 旧命令 | `commands/sessions/delete_session.rs` 调 `SessionsApi::delete` 后无条件 `emit_active_session_changed(None)`；`SessionsApi::delete` 是唯一调用者。 |
| 停止边界 | `AgentSessionRuntimeAdapter::stop_session_activity` = `stop_generation` + `reap_background_commands` + `kill_shells_for_session`（后者 close 未完成时返回 `SESSION_CLEANUP_INCOMPLETE` Conflict）。`reap_background_commands` 只设置 kill 标志，不等待退出；Agent 终端由 `terminal_service.stop` 单独管理。 |
| Git 能力 | 本机 git 2.43.0；`worktree list --porcelain -z`、`rev-parse --path-format=absolute --git-common-dir`、`status --porcelain=v1 -z --ignored=traditional` 均可用。`platform/git/mod.rs` `GitAdapter` 已固定 `LC_ALL=C`，超时由 `ProcessError::TimedOut` 表达并会 kill 子进程。 |
| 主规范重叠 | `session-management` 已有 `UI-driven multi-session deletion`（要求 UI 逐个 id 调用服务），本 delta 新增 MODIFIED 块并保留 4 个原场景标题；`System session mutation refusal` 由 `SessionId::parse` 在领域层拒绝 `system-activity-v1-` 前缀，本能力沿用。 |
| 前端行数预算 | `main-layout.tsx` 514/528、`session-sidebar.tsx` 300/300、`tauri-agent-client.ts` 1202/1215。删除对话框与状态机全部放入 `src/main-layout/session-deletion/` 新文件。 |
| 对话框原语 | `components/ui/application-dialog.tsx` 提供 focus trap、Esc、`data-dialog-autofocus`、`closeDisabled`、`footer`。 |
| 操作设施 | `OperationsApi::start/complete/fail` 提供 OperationTask；删除 journal 独立于 operations 表，不受其保留策略影响。 |
| 后台执行 | `commands/sessions/background.rs` 用 `tauri::async_runtime::spawn_blocking` 承载会话创建，删除执行沿用同一模式。 |

## 当前分支重新勘探要求

执行前重新查找所有 `deleteSession`、`delete_session`、`deleteSessions`、`onBatchDelete`、`stop_session_activity`、`worktree_path` 与真实有效 folder 的使用者。上述部分文件会超过工具单次输出长度，必须读取相关函数完整范围，不根据文件开头推断后续逻辑。

现有未归档 change 目录已核对，没有本包同名的 `add-session-worktree-cleanup`。这不证明其他 change 内容完全无重叠；实施时仍需检查会话、工作区 UI、运行恢复和用户体验相关 change。

当前远程查询基准包含 main 与 UI 重构等分支，但本包不依赖 dev 分支存在。实施基于用户实际打开的目标 checkout，不自动选择或切换 main/dev。

## 新增模块属于设计，不是现有事实

design.md 中的 SessionDeletionCoordinator、ManagedWorktree、PreviewSessionDeletionInput、WorkspaceUseGatePort 和建议表名均为待实现设计。若仓库已有同等设施应复用；代码地图没有证明它们现在已存在。

## 公开原始参考

- [Git worktree 官方手册](https://git-scm.com/docs/git-worktree)：linked/main 区分、非 force remove、登记、locked、prune 与 NUL 输出。
- [Git status 官方手册](https://git-scm.com/docs/git-status)：tracked/staged/untracked/ignored 的独立检查及 porcelain 输出。
- [Git rev-parse 官方手册](https://git-scm.com/docs/git-rev-parse)：实际 root、git-dir 和 common-dir 解析。
- [OpenSpec conventions](https://github.com/Fission-AI/OpenSpec/blob/main/openspec/specs/openspec-conventions/spec.md)：delta 操作段落、完整修改需求、Scenario 结构。
- [OpenSpec CLI](https://github.com/Fission-AI/OpenSpec/blob/main/docs/cli.md)：change 与主 specs 校验是不同命令。

引用用于解释设计依据；实现的规范义务以本 change 的 specs 为准。Git 行为应在仓库支持的版本上用临时仓库复验，不用在线手册的最新功能悄悄抬高最低依赖版本。
