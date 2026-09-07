# Stop session agent terminals before removing the worktree

## Why

删除会话并勾选「同时移除 worktree」时，Git 移除在 Windows 上必定失败，操作以 `needs_attention` 收场而不是 `succeeded`。

根因是删除前的静止屏障漏掉了一类写入者。`deletion_runtime.rs` 的 `quiesce` 只停四类：生成、工具审批等待者、后台命令、Shell。CLI 模式的 **agent 终端**不属于其中任何一类——它是 `vanehub-ai.exe` 直接派生的 `cmd.exe`，包装脚本首行就是 `cd /d <会话 worktree>`，因此该 worktree 是这个活进程的当前工作目录。Windows 拒绝删除任何活进程的当前工作目录，`git worktree remove` 于是返回 255 与 `failed to delete <path> Permission denied`，运行时如实映射为 `Refused`。

这不是规范缺口。`add-session-worktree-cleanup` 的「Quiescence before session deletion」已经写明「会话删除 SHALL 停止并等待本应用管理的生成、**CLI**、后台命令、Shell 及相关工作区句柄释放」——agent 终端正是其中的 CLI。**规范是对的，实现没做到。** 该变更的任务 5.3 声称已实现 quiesce barrier 并已勾选，但对 agent 终端不成立。

现在做，是因为它已随 #278 进入 main，且没有任何 CI 闸门会发现它：`Desktop Smoke` 在 `CI=true` 下只跑 core smoke 契约，覆盖此路径的 `desktop-session-deletion` 层属于 `Desktop Full Suite`，需要 `desktop-full-suite` 标签才触发。本地全量运行 6/6 确定性复现。

## What Changes

- 在 `quiesce` 静止屏障中加入 agent 终端：请求停止，并在同一个 deadline 预算内等待其真实退出。
- 超时未退出时压入 `agent_terminal` 阻塞项，沿用既有语义——保留会话记录、阻止 Git 移除、不谎报成功。这与现有 `generation` / `background_command` / `shell` 阻塞项的处理方式一致，不新增失败模式。
- 修正 `add-session-worktree-cleanup` 任务 5.3 的完成状态，使其不再声称已覆盖 CLI。
- 关闭 CI 闸门缺口，使这一层不再只能靠人工全量运行发现。

不含破坏性变更。不新增 UI、状态库或依赖。

## Capabilities

### New Capabilities

无。

### Modified Capabilities

无。本变更设置 `skip_specs: true`。

规范层面的行为**没有**改变：应停止 CLI 的要求已由 `add-session-worktree-cleanup` 的「Quiescence before session deletion」规定，本变更只是让实现满足它。刻意不重复声明一份 delta——该 capability（`session-deletion-operations`）尚未同步进 `openspec/specs/`，两个未归档变更同时重建同一条需求会让其中一份 delta 失效。该需求的归属与同步仍由 `add-session-worktree-cleanup` 负责。

## Impact

**运行时范围：仅桌面端。** Web/mock adapter 不派生进程，不受影响。

- `src-tauri/src/contexts/sessions/infrastructure/deletion_runtime.rs` — `quiesce` 增加 agent 终端停止与有界等待。这是唯一需要改的生产代码路径。
- 不跨越新的上下文边界：该文件已通过 `published_agent_runtime()` 依赖 `agent_runtime::api`（用于 `stop_generation` 与 `reap_background_commands_and_wait`），`stop_agent_terminal` 就在同一个 API 上。前后端隔离与运行时适配器边界均不受影响。
- `src-tauri/src/contexts/sessions/application/deletion/tests.rs` — 补充 agent 终端未退出即阻止移除的用例。
- `tests/desktop/specs-session-deletion/session-deletion.e2e.mjs` — 既有用例即验收标准，不需修改；本变更使其转绿。
- `.github/workflows/ci.yml` — 收敛闸门缺口。
- `openspec/changes/add-session-worktree-cleanup/tasks.md` — 修正 5.3 的失实勾选。
