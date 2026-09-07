# Design

## Context

动机见 `proposal.md - Why`。这里只列影响方案的现状约束。

`deletion_runtime.rs::quiesce`（89 行）是静止屏障的唯一适配器，按 deadline 预算依次停四类写入者：生成、工具审批等待者、后台命令、Shell。它已经通过 `published_agent_runtime()` 使用 `agent_runtime::api`，也通过 `workspaces()` 使用工作区能力，因此本变更不需要引入新的上下文依赖。

agent 终端子系统的现有接口是本设计的主要约束：

- `stop(StopAgentTerminalRequest { terminal_id })` — **按 terminal_id**，而删除协调器手上只有 `session_id`。
- `cleanup_idle(idle_after_seconds)` / `shutdown()` — 返回 `Vec<String>` 会话 id，说明端口内部掌握会话映射，但作用域是全局，会波及其他会话。
- **没有任何存活查询**，无法回答"这个会话还有终端活着吗"。

Shell 子系统已有正好对应的一对：`kill_shells_for_session(session_id)` 与 `live_session_shell_count(session_id)`。

## Goals / Non-Goals

**Goals**

- 会话删除在执行 Git 移除前，终止该会话的 agent 终端并确认其真实退出。
- 终端未能在预算内退出时，沿用既有阻塞语义：保留会话、阻止移除、不谎报成功。

**Non-Goals**

- 不改 `git worktree remove` 的无 `--force` 策略。该策略是刻意的安全设计，强删会把"进程仍在写入"变成静默的数据损坏。
- 不处理非本应用管理的外部进程。规范已明确对这类进程不宣称排他隔离。
- 不修 e2e 第 4 个用例。它的失败是第 3 个用例断言抛出后对话框未关的连带效应，第 3 个转绿后即消失；若未消失，说明存在独立缺陷，另行处理。
- 不改动 `Desktop Full Suite` 整体是否 opt-in 这一更大问题，只收敛本路径的闸门缺口。

## Decisions

### D1：在适配器而非协调器中扩展

`quiesce` 的实现放在 `deletion_runtime.rs`，协调器只通过 port 消费 `QuiescenceReport`。

**理由**：协调器是平台无关的策略层，"哪些算本应用管理的写入者"属于运行时装配知识。**备选**：给协调器加一个新 port。**否决**：`agent_runtime::api` 已是该适配器的依赖，新增抽象只会增加一层无收益的间接。

### D2：新增按会话的停止与存活查询，照搬 Shell 的形状

在 agent 终端服务与端口上新增两个方法，命名与语义对齐既有 Shell 版本：

- 按 `session_id` 停止该会话的全部终端
- 返回该会话仍存活的终端数

**理由**：`stop` 的 `terminal_id` 粒度在此处不可用，而 `cleanup_idle`/`shutdown` 会误伤其他会话的终端——那会违反规范中「keep 不得冻结同目录的其他会话」的约束。照搬 Shell 的方法对，读者不必学第二套心智模型。**备选**：让协调器先列出会话的 terminal_id 再逐个 stop。**否决**：把会话→终端的映射泄漏到调用方，且列举与停止之间存在竞态窗口。

### D3：轮询存活数确认退出，不采信 `stop` 的返回值

`stop` 返回 `bool` 表示请求是否被受理。规范对此有明确禁令：「取消请求受理 SHALL 不作为退出回执」。因此必须在同一 deadline 预算内轮询存活数直至归零。

**理由**：这正是本缺陷的形状——受理了停止请求，进程却还占着工作目录。采信受理回执会让修复只是把失败时机往后挪。**备选**：停止后固定 sleep 一段时间。**否决**：既不可靠又拖慢正常路径。

### D4：超时压入 `agent_terminal` 阻塞项

与 `generation` / `background_command` / `shell` 完全一致：压入阻塞项、`quiet=false`，由既有逻辑保留会话并阻止移除。

**理由**：不新增失败模式，UI 与重试入口无需改动。阻塞项名称对齐原生日志类别 `session.agent_terminal`，便于把失败与日志对上。

### D5：在 Shell 之前停终端

顺序放在后台命令之后、Shell 块之前。

**理由**：终端可能派生 shell；若先做 Shell 的严格关闭再停终端，终端可能在计数校验之后再拉起 shell，使检查失效。反向则不存在这个窗口。

### D6：把 `desktop-session-deletion` 纳入 CI 闸门

**理由**：该缺陷的失败模式是静默的——不丢数据、只是 worktree 残留并报 `needs_attention`，人不盯着就发现不了，而它已经这样进了 main。该层通过时约 2:44，加进 smoke 闸门的三平台成本可接受。**备选**：让 `Desktop Full Suite` 恒跑。**否决**：三平台 150 分钟预算，为一层付整套代价。

## Task 1.1 核实结论（2026-09-07 已查证，取代 D2 的部分前提）

读 `terminal_process.rs` 后确认三件事，前两件推翻了 D2「照搬 Shell 那对方法」的可行性：

1. **`stop()` 先摘注册表再 kill。** 顺序是 `terminals.remove(&session_id)` → `terminate_terminal_child`。任何基于注册表的存活计数在 `stop()` 返回后立刻为 0，不能作为退出证据。
2. **收尸有界且结果被丢弃。** `terminate_terminal_child` 以 `TERMINAL_REAP_BUDGET = 1500ms` 调 `reap_terminal_before`，写法是 `let _ = ...`。子进程仍存活时 `stop()` 一样返回 `Ok(true)`——正是规范禁止的「以受理充当退出回执」。
3. **PTY 子进程没有 job object 容器。** 终端经 `portable_pty::spawn_command` 启动，不走 `platform::process` 的 process-wrap 链，现成的 `windows_job::TerminateTreeJobObject` 够不着它。会话 Shell（`retained_shell_runtime.rs:545`）用同一条 spawn 路径，**同样没有容器**——该弱点非 agent 终端独有。

**对 D2 的修正**：放弃「停止 + 独立存活查询」两个方法的形状，改为**单个按会话的方法：请求停止，并在调用方给定的 deadline 内确认真实退出，返回是否已确认**。理由是第 1、2 条——注册表不能作为事实来源，且调用方（quiesce）持有的预算远大于固定的 1.5s。

**关于进程树范围**：第 3 条意味着 kill 直接子进程 `cmd.exe` 未必带走孙进程 `opencode.exe`，而占着 worktree 的是这两者。但现有证据**不足以**断定 `stop()` 杀不掉整棵树——当前 quiesce 从未调用过 `stop_agent_terminal`，终端存活只能证明"没人停它"，不能证明"停不掉"；ConPTY 主端关闭时附着进程通常会退出。因此先按最小正确修复实现并以 e2e 实测为判据，不为假想情况预先加设计。

**升级路径（仅在实测失败时启用）**：在 PTY spawn 处引入进程树容器，或按 pid 终止后代树；届时应同时覆盖 `retained_shell_runtime.rs`，因为两者共用同一条无容器的 spawn 路径。

## Risks / Trade-offs

- **孙进程可能在直接子进程被杀后存活并继续持有 worktree** → 见上面的升级路径；以 e2e 层的真实结果为判据，不以推测为准。
- **Windows 上 ConPTY 关闭已知会拖到超时**（本仓库已有记录：reader 要等 master 句柄释放才收到 EOF）→ 可能表现为删除比现在慢，最坏情况以 `agent_terminal` 阻塞项收尾。这仍严格优于现状：现状是无声地失败在 Git 上，改后是有名有姓的阻塞项加保留的会话。
- **终端停止会话正在用的终端** → 仅限被删除的会话，删除本身即蕴含停止其工作；且规范已要求如此。

## Migration Plan

无数据模型或 schema 变更，无迁移。回滚即还原该 commit，不留持久化痕迹。

## Open Questions

无。
