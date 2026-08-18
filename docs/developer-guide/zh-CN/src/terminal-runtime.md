# 终端与 PTY 运行时

单 Agent CLI 会话运行在一个会话级 Agent Terminal 内：这是一个由 native 运行时拥有、以 PTY 为底座的 CLI 进程，通过 frontend Agent service 边界暴露给 React。React 组件绝不直接调用 Tauri command 来管理终端生命周期。

## 会话级、单 Agent

Agent Terminal 面向未归档的单 Agent CLI 会话。对已归档会话发起的终端启动请求会被拒绝，不会启动任何 CLI 进程，并返回一个简洁的、可向用户展示的失败信息。

## 自动启动与附着

当单 Agent 会话被创建或选中后，UI 会自动为该会话请求 Agent Terminal 启动——没有单独的启动按钮。如果选中的会话已经有一个活跃的 retained Agent Terminal 进程，UI 会附着到已有的终端流上，而不是为同一会话再启动一个重复的 CLI 进程。

## 远程终端

远程 SSH workspace 暴露其自身的远程终端运行时路径；本地 PTY 的归属模型不会原样延伸到远程会话。远程 workspace 的工作流见用户指南，`workspaces`/`sessions` 的归属划分见 [Native bounded contexts](native-contexts.md)。

## 本地 PTY 实现

本地 Agent 终端基于 `portable-pty` 库。核心结构 `ManagedAgentTerminal`(`agent_runtime/infrastructure/terminal_process.rs`)持有 `master: Box<dyn MasterPty>`、`writer`、`child` 与一个有界转录缓冲 `BoundedTextBuffer`。注册表以 **session_id 为键**映射到 `ManagedAgentTerminal`,即"会话级单 Agent 终端"的归属。

- **有界转录缓冲** `BoundedTextBuffer` —— `{chunks: VecDeque, bytes, max_bytes}`;`RETAINED_TERMINAL_TRANSCRIPT_BYTES = 1MB`,超限从队头按 **UTF-8 字符边界** 裁切;`snapshot()` 拼接全文供附着时重放。
- **读缓冲** `TERMINAL_READ_BUFFER_BYTES = 64KB` —— 大缓冲聚拢突发输出、减少 IPC 事件数;`take_decodable_utf8` 处理跨读分裂的 UTF-8。
- **Shell 类型** `AgentTerminalShell`(WindowsPowerShell / WindowsCmd / UnixDefault)—— Windows 优先 `powershell.exe`,否则 `cmd.exe`;Unix 用 `$SHELL` 或 `/bin/sh`。
- **wrapper 脚本** `generate_agent_terminal_wrapper` —— 生成 `.ps1`/`.cmd`/`.sh` 包裹脚本,设置 UTF-8、进入会话目录、`exec` 目标 CLI;`validate_token` 拒绝空与 NUL;`redacted_command` 用于日志。
- **终端尺寸** —— rows clamp `1..=200`、cols clamp `1..=500`。

### 自动启动与附着

`open_or_attach` 先查注册表,按 session_id 命中即视为 retained 终端——刷新 `last_active_at`、发 `State{Running}` 事件、把**存量转录作为 Output 事件重放**(不必重启 CLI)。否则走新开流程:校验 provider `Terminal`/`Resume` 能力、生成 invocation 与 wrapper、`openpty`、spawn、注册。

前端在 tab 挂载时即 `openAgentTerminal({rows, cols})`;`sessionActivationKey` 变化且无 terminalId 且状态为 stopped/failed 时自动重连。

### 归档与只读拒绝

`terminal_service.rs` 在 `open_or_attach` 入口拒绝:已归档会话 → `Validation("Archived sessions cannot start Agent terminals.")`;只读(verifier)会话 → `PolicyDenied{action:"open-terminal"}`;非 `Cli` 交互模式 → `UnsupportedInteractionMode`。

### 并发与死锁防护

阻塞 I/O 不在注册表锁内执行。`reap_terminal_without_holding_lock` 用 `try_wait()` 50ms 轮询、短锁持有,避免 reader 线程与 `stop()` 的 kill 互相死锁;`terminate_terminal_child` 锁内 kill、解锁后 reap。独立 usage 轮询线程 250ms tick、5s 间隔,经 `AtomicBool alive` 停止并 join。

### 空闲回收

后台每 60s 调用 `cleanup_idle_agent_terminals`,`AGENT_TERMINAL_IDLE_TIMEOUT_SECONDS = 2 小时` 的空闲 Agent 终端被回收。

## 远程终端(SSH)

远程终端走 **russh 库**在 SSH 会话上请求的远程 PTY,与本地 `portable-pty` openpty 完全不同:`channel_open_session` → `request_pty(true, "xterm-256color", cols, rows, 0, 0, &[])` → `request_shell(true)`。远程 shell 传输池有独立的容量与 idle 限制(`remote_terminal_limits.rs`)。

## 终端输出捕获

Agent 终端只保留内存有界转录(1MB);持久化的 Terminal 捕获走 workspaces 的捕获服务,两者分离。

- **有界捕获队列** `BoundedCaptureQueue` —— `TERMINAL_CAPTURE_QUEUE_CHUNKS=256`、`TERMINAL_CAPTURE_CHUNK_BYTES=32KB`、`TERMINAL_CAPTURE_BATCH_CHUNKS=32`;满则 `pop_front` 并置 `dropped=true`。
- **缺口标记** —— `drain_batch` 若曾丢弃,先输出一条 `source: Gap`、`content: "[capture gap]"` 的缺口标记,再排空——不静默丢数据。
- **保留与容量** —— `TERMINAL_CAPTURE_RETENTION_DAYS=30`、`TERMINAL_CAPTURE_CAPACITY_BYTES=512MB`;`enforce_capacity` 按最早块循环删除直到总量 ≤ 容量。
- **持久化表** `terminal_output_chunks` —— `UNIQUE(stream_id, sequence)`,带 FTS5 trigram 全文索引;`source IN ('pty','quick-command','gap')`。
- **单块上限** —— `output_chunk.rs` 超 `32KB` 报 `TooLarge`,并剥离 ESC 控制字符。
- **远程池常量** —— `REMOTE_TERMINAL_POOL_CAPACITY=8`、`REMOTE_TERMINAL_IDLE_TIMEOUT_SECONDS=300`、`CONNECT_TIMEOUT=15s`、`KEEPALIVE=30s`。

## 设计所在之处

本章用于引导贡献者。权威需求位于 spec 中。

- [openspec/specs/agent-terminal-runtime](../../../../openspec/specs/agent-terminal-runtime/spec.md)
- [openspec/specs/remote-terminal-runtime](../../../../openspec/specs/remote-terminal-runtime/spec.md)
- [openspec/specs/session-shell](../../../../openspec/specs/session-shell/spec.md)

PTY 与 shell 运行时位于 `workspaces` 和 `sessions` 限界上下文中；见 [Native bounded contexts](native-contexts.md)。
