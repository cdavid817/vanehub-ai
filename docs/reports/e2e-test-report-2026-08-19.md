# VaneHub AI 端到端验证报告（2026-08-19）

被测提交：`27a2ba3b`（main）。平台：Windows 11 Pro 26200，`x86_64-pc-windows-msvc`。

本轮共发现 6 个缺陷，全部修复并附回归测试；另有 2 项确认为负载敏感、未能复现，如实列为未修复项。

## 1. 测试矩阵

### 1.1 构建健康度

| 项 | 方法 | 结果 | 证据 |
| --- | --- | --- | --- |
| Rust 工作区类型检查 | `cargo check --workspace --all-targets` | PASS | exit 0 |
| Rust lint | `cargo clippy --workspace --all-targets -- -D warnings` | PASS | exit 0，改动后复跑仍 exit 0 |
| Rust 格式 | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASS | exit 0 |
| 生产代码 panic 快捷方式 | `npm run native:panic:check` | PASS | exit 0，本轮未新增 `unwrap`/`expect` |
| TypeScript 类型检查 | `npx tsc --noEmit` | PASS | exit 0 |
| 前端 lint | `npm run lint:ci` | PASS | exit 0 |
| 前端构建 | `npm run build` | PASS | 1m18s；`Verified 16 lazy frontend chunks; main static closure 131.5 KiB gzip` |

### 1.2 单元 / 集成测试

| 项 | 方法 | 结果 | 证据 |
| --- | --- | --- | --- |
| 前端单测 | `npm run test`（vitest） | PASS | 286 files / 1301 tests；修复前 1 failed |
| 原生库单测 | `cargo test --lib` | PASS | 3537 passed / 0 failed / 15 ignored，277s |
| 架构守卫 | `cargo test --test architecture` | PASS | 41 passed |
| MCP fixture 契约 | `cargo test --test mcp_fixture_contracts` | PASS | 3 passed |
| MCP relay provider | `cargo test --test mcp_relay_provider_invocations` | PASS | 3 passed |
| 权限 hook crate | `cargo test -p vanehub-permission-hook` | PASS | 15 passed |
| 前后端契约 | `npm run contracts:check` | PASS | exit 0 |
| 文档闸门 | `npm run docs:check` | PASS | exit 0 |
| 规范校验 | `openspec validate --specs --strict` | PASS | exit 0 |
| node 工具链单测 | `sidecar:unit:test`、`desktop:unit:test`、`version:unit:test`、`coverage:policy:test`、`performance:unit:test`、`docs:unit:test` | PASS | 6 个套件全部 exit 0 |
| 浏览器 E2E | `npx playwright test` | PASS | 156 passed，9.2m（修复前 154/2，见 §3 BUG-4、BUG-6） |

### 1.3 核心链路（代码审查 + 最小复现）

| 链路 | 方法 | 结果 |
| --- | --- | --- |
| PTY 会话生命周期 | 审查 `terminal_process.rs`、`portable_pty.rs`、`managed_child.rs`；对两处缺陷写最小复现单测 | **发现 2 个缺陷**（BUG-2、BUG-3） |
| 多 CLI 适配 | 审查 `providers/invocation.rs` 全部 5 个 CLI 的 argv 构造与 `providers/output.rs` 解析器 | 未发现缺陷；`providers/tests.rs` 已按 `agent × template` 全组合校验 fixture |
| PDP/PEP 三层拦截 | 审查协议回调（`hook_bridge_server.rs`）、配置编译（`invocation.rs` 策略投影）、决策点（`evaluation_service.rs`） | 未发现缺陷，见 §2 |
| SQLite 持久化与迁移 | 校验迁移账本、连接 PRAGMA | 未发现缺陷，见 §2 |
| git worktree 隔离 | 审查 `subagent_worktree.rs`，对回退清理路径补测试 | 疑似残留**未复现**，见 §4 |

### 1.4 边界与异常

| 场景 | 结论 | 依据 |
| --- | --- | --- |
| CLI 二进制缺失 | 安全 | `availability.rs` 返回类型化 `ExecutableStatus::Missing`，不 panic；`invalid_shell_executable_fails_to_spawn` 已覆盖 spawn 失败 |
| 子进程崩溃 | 安全 | 退出监视线程将非零退出映射为 `AgentTerminalState::Failed` 并回收；`reap_terminal_without_holding_lock` 已规避 wait 持锁死锁 |
| SQLite 锁冲突 | 安全 | WAL + `busy_timeout` + `synchronous=FULL` + `foreign_keys=ON` + 连接池；已有测试覆盖「写入期间同连接读不被锁死」 |
| 非 UTF-8 输出 | 安全 | `take_decodable_utf8` 保留不完整尾字节、对真正非法字节走有损解码不卡死；3 条单测覆盖 |
| 并发会话 | **发现缺陷** | BUG-3：注册表锁跨阻塞 PTY 调用 |

## 2. 未发现缺陷的链路（审查结论）

- **权限三层**。协议回调层仅绑定 loopback、校验 Bearer token，未映射的工具直接 deny；决策点 `evaluate` 对任何内部错误 fail-closed 到 `Ask`，MCP 来源动作有不可放宽的 `Ask` 下限；配置编译层对 5 个 CLI 各自投影策略模板，且刻意避开 `--dangerously-skip-permissions` 一类绕过开关，`gemini-cli` 的 `standard` 还额外强制写入 `--approval-mode default`，防止落回用户自己的 `settings.json`。Runner 授权用 `witness` + `revalidate` 防 TOCTOU，指纹材料以 `\0` 分隔避免字段拼接碰撞。
- **迁移账本**。79 条迁移，版本号与名称均无重复、连续无空洞；启动时 `assert_migration_history_is_dense` 会拒绝有空洞的历史，版本号碰撞由 `migration_sequence_matches_expected` 在测试期拦截。

## 3. Bug 清单

| 编号 | 级别 | 现象 | 根因 | 修复 commit | 回归验证 |
| --- | --- | --- | --- | --- | --- |
| BUG-1 | P2 | `mission-control` 单测断言 `[role='tablist']` 得到 `null`，仅在全量 `vitest run` 下失败，单跑 3/3 通过 | 测试同步在了错误的信号上：`getMissionControlRun` 在 click 同步阶段即被 spy 记录，而 tablist 依赖 await 解析后的 React 提交 | `887626e1` | `npm run test` 全量通过；并在并发 clippy 负载下复跑仍 1301/1301 |
| BUG-2 | P2 | 交互式 Agent 终端可能捕获不到 provider 会话 id，导致会话无法 resume、用量数据缺失，而终端显示一切正常 | 读循环对「解码后无可显示文本」的 read 提前 `continue`，跳过了 `ProviderOutputFramer`；PTY 可能只返回多字节字符的头 1–2 字节，这些字节被永久丢弃，使会话标记行变成非法 UTF-8 被 `unwrap_or_default()` 吞掉 | `d8fb8787` | 新增 `a_read_that_decodes_to_no_text_still_reaches_the_provider_framer`，修复前红（`left: []` vs `right: ["claude-session"]`），修复后绿 |
| BUG-3 | P1 | 一个停止读取 stdin 的 CLI 会冻结整个 Agent 终端子系统：其他终端无法输入/resize/attach，且**该终端自身也无法被停止** | `input()`/`resize()` 持有 `terminals` 注册表锁跨越阻塞的 PTY 写入；`stop()` 需要同一把锁，因此取消路径也被堵死 | `5f8a0f63` | 新增 `a_blocked_terminal_writer_does_not_hold_the_registry_lock`；`terminal_process` 全部 17 条测试通过 |
| BUG-4 | P1（开发/CI） | `npx playwright test` 每轮的头 1–2 个 spec 必定超时在 `page.goto("/")`，页面停在静态 `Starting...` 外壳，伪装成确定性 UI 回归 | `vite.config.ts` 的 `server.watch.ignored` 覆盖了 `src-tauri`/`target`/文档目录，唯独漏了 `.claude`——本仓库的嵌套 worktree 都在那里，各自带 `node_modules` 与 `target`。`test.exclude` 早已因同一原因排除 `.claude`，watcher 这一半被漏掉 | `6251035b` | 两次必挂的 spec 对随后 6/6 通过，该文件对耗时从 4.2m 降至 2.4m |
| BUG-5 | P2 | `shutdown_runs_processes_concurrently_and_forces_unresponsive_trees` 在全量 `cargo test` 下 `graceful == 0`（期望 1），单跑必过 | 900ms 绝对 deadline 需覆盖两次 `node` fixture 启动加一次优雅关闭往返；与另外 3500 条测试同机竞争时不可达。该测试自身注释已为「墙钟上限」写过同样的推理，却没应用到 deadline 本身 | `68a3558e` | 放宽到 5s；判别力不变——串行实现无论预算多大都会先耗尽在 `lsp-hang` 上 |
| BUG-6 | P2 | 修掉 BUG-4 后，156 条 E2E 仍有排序最前的 1 条超时在首次导航 | Playwright 的 `webServer.url` 探测只证明服务器能应答 HTTP（`index.html` 立即返回），并不代表 Vite 已完成模块图转译；两个 worker 同时启动，共同抢这次冷转译。已确认是位置相关而非用例相关：单文件 `--workers=1` 运行时首条失败、其后两条走同一导航 helper 的用例通过 | `7fb88736` | 新增 `tests/e2e/global-setup.ts` 串行预热一次导航；全量从 155/1 变为 **156 passed**，耗时 11.8m → 9.2m |

级别口径：P0 = 数据损坏或主链路不可用；P1 = 核心功能在可达场景下失效且无自愈；P2 = 局部降级、误报或开发体验缺陷。

## 4. 未修复项

| 项 | 现象 | 为何未修 | 建议 |
| --- | --- | --- | --- |
| `real_playwright_worker_bounds_page_operations_handoff_and_artifact_bytes` | 一次全量 `cargo test` 中报 `worker shutdown: ShutdownFailed` | 仅观测到 1 次。此后两轮干净全量运行（3535、3537 passed）均未复现，单跑亦通过 | 与 BUG-5 同属「围绕 Node 子进程的固定超时」形态，但失败发生在生产侧的关闭预算内而非测试常量上。建议先在关闭路径加分阶段耗时日志取证，再决定是放宽预算还是修关闭逻辑——只凭一次观测盲目放宽会掩盖真实的进程泄漏 |
| `initialize_timeout_forces_bounded_process_tree_cleanup_without_cancellation` | 同一轮中 cleanup 阶段 `Failed`（期望 `Succeeded`） | 同上，未复现 | 同上 |
| `slash-commands.spec.ts:87 › /logs opens the logs workspace tab` | 一轮全量 E2E 中断言 `aria-selected` 为 `false`（期望 `true`），排在第 135/156 位 | 仅观测到 1 次。隔离下 2/2 通过，其前后两轮全量运行（含最终 156 passed 的确认轮）均未复现 | 与上面两条同属负载敏感，但这条是行为断言而非超时，更值得追。建议下次复现时抓 trace 比对 `/logs` 命令派发与 tab 选中之间是否存在未被等待的状态提交 |
| 桌面端 E2E（`npm run test:desktop`） | 未运行 | 本轮未改动桌面运行时、Tauri 启动链路或 IPC 行为，按 `AGENTS.md` 不属必跑项；且该命令会真实构建并启动桌面客户端，本机结果不可外推到其他平台 | 合入前由 CI 的 `Desktop Smoke` 在三平台原生 runner 上覆盖 |
| `npm run test:coverage` | 未运行 | CI 用它替代 `npm run test` 并附覆盖率门槛；本轮以 `npm run test` 取得功能结论 | 由 CI 执行 |
| 疑似 worktree 残留 | 无 | **调查后证伪**：`worktree remove --force` 在目录已被外部删除时仍能正确注销，回退路径无残留 | 已补一条通过的覆盖测试固化该不变量（`f37bebb0`） |

### 范围说明

任务书列出的「Checkpoint 与 git worktree 隔离」中，**本仓库不存在会话级 checkpoint 功能**。代码中 `checkpoint` 一词仅出现在两处无关语义：IM 连接器的轮询位点（`im-connector-management`）与技能文件系统事务的回滚点。git worktree 隔离确实存在，由 `subagent_worktree.rs` 提供，已按「创建 / 捕获 / 清理不残留」审查并补测。

## 5. 新增测试用例

| 用例 | 位置 | 防止的回归 |
| --- | --- | --- |
| `a_read_that_decodes_to_no_text_still_reaches_the_provider_framer` | `src-tauri/src/contexts/agent_runtime/infrastructure/terminal_process.rs` | PTY 读循环丢字节导致会话标记解析失败。刻意走生产的 `ProviderOutputFramer`，而非既有测试所用的 `#[cfg(test)]` 版 `drain_complete_lines` |
| `a_blocked_terminal_writer_does_not_hold_the_registry_lock` | 同上 | 注册表锁跨阻塞 PTY 调用；对齐 workspace shell 运行时同名不变量 |
| `a_reap_that_falls_back_to_the_filesystem_leaves_no_administrative_record` | `src-tauri/src/contexts/agent_runtime/infrastructure/subagent_worktree_tests.rs` | worktree 目录已消失时的回退清理留下 prunable 注册项 |
| `tests/e2e/global-setup.ts` | Playwright 全局预热 | 首次导航与冷转译竞争，使排序最前的 spec 误报为 UI 回归 |

`mission-control.test.tsx` 的断言由同步改为 `waitFor`，未新增用例。

**红-绿验证程度**（避免高估这些测试的证明力）：

- BUG-1、BUG-2、BUG-4、BUG-6 走完了完整的红-绿：修复前均能稳定复现失败（BUG-2 的红是 `left: []` vs `right: ["claude-session"]`；BUG-4、BUG-6 各自连续失败 2 次以上），修复后转绿。
- BUG-3 只有绿。修复前的结构把 writer 直接放在注册表锁保护的 struct 里，无法在不引入完整 runtime 脚手架的前提下写出行为级红灯；该用例的作用是钉住修复后的不变量，缺陷本身的依据是代码路径分析加上 workspace shell 运行时中同一失效模式的既有修复与注释。
- BUG-5 的红只观测到 1 次且不可稳定复现，属于放宽超时而非修正逻辑。

## 6. 环境注意事项

本机在验证期间触发了两处非产品问题，记录以免后续误判：

- `cargo test --workspace` 一度以 `error: failed to write query cache ... 磁盘空间不足 (os error 112)` 中断。D 盘 98% 占用，`target/debug/incremental` 独占 8.1 GB。后续以 `CARGO_INCREMENTAL=0` 完成，未删除任何文件。
- 本机同时有 4 个 node 开发服务器在监听，`npx playwright test` 首次选用的端口已被占用而直接失败。E2E 需显式钉一个已确认空闲的 `PLAYWRIGHT_PORT`。
