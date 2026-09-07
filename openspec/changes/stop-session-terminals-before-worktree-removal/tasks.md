## 1. 核实退出确认强度（先做，结论决定第 2 组的范围）

- [x] 1.1 阅读 `src-tauri/src/contexts/agent_runtime/infrastructure/terminal_process.rs`，确认终端被 stop 后是从注册表摘除即算结束，还是等到真实 OS 进程退出；把结论写进 design.md，不要停留在口头。
      **结论**：先摘注册表再 kill；收尸预算固定 1500ms 且结果被 `let _ =` 丢弃；子进程存活时 `stop()` 仍返回 `Ok(true)`。注册表不可作为退出证据。已写入 design.md「Task 1.1 核实结论」并据此修正 D2。
- [x] 1.2 按修正后的 D2 实现真实退出确认（不再新增独立存活查询——注册表已被证明不可信）。

## 2. 按会话的停止并确认退出

- [x] 2.1 在 agent 终端端口与 `AgentTerminalApplicationService` 上新增单个方法：按 `session_id` 停止该会话的终端，并在**调用方给定的 deadline** 内确认真实退出，返回是否已确认。不得改动现有按 `terminal_id` 的 `stop`（其固定 1500ms 预算服务于退出路径，语义不同）。
- [x] 2.2 在 `agent_runtime::api` 上暴露该方法，供 `deletion_runtime.rs` 通过既有的 `published_agent_runtime()` 调用；不新增上下文依赖。
- [x] 2.3 补单元测试：只影响目标会话的终端，其他会话的终端不受影响（对应规范中 keep 不得冻结同目录其他会话的约束）；以及子进程未退出时返回未确认而非 true。

## 3. 接入静止屏障

- [x] 3.1 在 `deletion_runtime.rs::quiesce` 的后台命令之后、Shell 块之前加入终端停止（见 design.md D5 的顺序理由）。
- [x] 3.2 在同一 `remaining()` deadline 预算内轮询存活数直至归零；**不得**采信 stop 的返回值作为退出回执。
- [x] 3.3 超时未归零时压入 `agent_terminal` 阻塞项，名称对齐原生日志类别 `session.agent_terminal`；不新增失败模式。
- [x] 3.4 在 `src-tauri/src/contexts/sessions/application/deletion/tests.rs` 增加用例：终端未退出时 `quiet=false`、会话保留、不进入 Git 移除。

## 4. 端到端验证

- [x] 4.1 运行 `npm run test:desktop:session-deletion`（单层模式不自建，需先 `npm run test:desktop:build`，且必须经 npm 启动）；4 个用例全绿。不修改 `tests/desktop/specs-session-deletion/session-deletion.e2e.mjs`——它就是验收标准。
- [x] 4.2 确认第 4 个用例「deletes a project session without offering or touching its directory」随第 3 个转绿而恢复；若仍失败，说明是独立缺陷，单独记录不要顺手改测试。
- [x] 4.3 运行结束后检查 `%TEMP%` 下不再残留 `vanehub-deletion-*-feature-*` 空目录——这是缺陷仍在的直接痕迹。
- [x] 4.4 运行 `npm run test:desktop` 全量 10 层，确认本改动没有破坏其他层。rc=0，10/10 层通过（43 分钟）；session-deletion 层 36 秒通过。session-shell 层 1 次重试后通过，属既有偶发。

## 5. 收敛 CI 闸门

- [x] 5.1 把 `desktop-session-deletion` 纳入 CI 恒跑的层集合（见 design.md D6），使该路径不再只能靠人工全量运行发现。
- [x] 5.2 在 `.github/workflows/ci.yml` 相应位置写明纳入理由：失败模式静默、不丢数据，仅表现为 worktree 残留与 `needs_attention`。

## 6. 修正失实记录

- [x] 6.1 将 `openspec/changes/add-session-worktree-cleanup/tasks.md` 的任务 5.3 改回未完成，并注明 quiesce barrier 当时未覆盖 agent 终端；不要静默改写描述来掩盖。

## 7. 校验命令

- [x] 7.1 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 7.2 `cargo check --workspace`
- [x] 7.3 `cargo clippy --workspace --all-targets -- -D warnings`
- [x] 7.4 `npm run native:panic:check`
- [x] 7.5 `cargo test --workspace` — rc=0，6717 passed / 0 failed / 15 ignored（860s，空闲机器）。此前一轮的 4 个 folder_opener 失败已按跨平台方式修好；再前一轮出现的 2 个 `platform::git` locale 失败与一次阻塞经空闲机器定向复跑证明是负载所致（4 个 git 测试 0.13s 全过），非代码问题。
- [x] 7.6 `npm run lint:ci`、`npm run test`、`npm run build`。**原假设已作废**：本变更改了 `scripts/test-desktop.mjs` 与 `scripts/desktop-orchestrator.node-test.mjs`，eslint 覆盖二者，三条都要跑，不能以"不动前端"跳过。
- [x] 7.9 `npm run desktop:unit:test`——改了编排脚本必须跑；其中有断言 CI 闸门层集合的守卫。
- [x] 7.7 `npm run architecture:check`——本变更触及 Tauri 边界与上下文间调用，必须跑
- [x] 7.8 `openspec validate stop-session-terminals-before-worktree-removal --strict` 与 `openspec validate --specs --strict`
