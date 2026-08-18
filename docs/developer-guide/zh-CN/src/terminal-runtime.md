# 终端与 PTY 运行时

单 Agent CLI 会话运行在一个会话级 Agent Terminal 内：这是一个由 native 运行时拥有、以 PTY 为底座的 CLI 进程，通过 frontend Agent service 边界暴露给 React。React 组件绝不直接调用 Tauri command 来管理终端生命周期。

## 会话级、单 Agent

Agent Terminal 面向未归档的单 Agent CLI 会话。对已归档会话发起的终端启动请求会被拒绝，不会启动任何 CLI 进程，并返回一个简洁的、可向用户展示的失败信息。

## 自动启动与附着

当单 Agent 会话被创建或选中后，UI 会自动为该会话请求 Agent Terminal 启动——没有单独的启动按钮。如果选中的会话已经有一个活跃的 retained Agent Terminal 进程，UI 会附着到已有的终端流上，而不是为同一会话再启动一个重复的 CLI 进程。

## 远程终端

远程 SSH workspace 暴露其自身的远程终端运行时路径；本地 PTY 的归属模型不会原样延伸到远程会话。远程 workspace 的工作流见用户指南，`workspaces`/`sessions` 的归属划分见 [Native bounded contexts](native-contexts.md)。

## 设计所在之处

本章用于引导贡献者。权威需求位于 spec 中。

- [openspec/specs/agent-terminal-runtime](../../../../openspec/specs/agent-terminal-runtime/spec.md)
- [openspec/specs/remote-terminal-runtime](../../../../openspec/specs/remote-terminal-runtime/spec.md)
- [openspec/specs/session-shell](../../../../openspec/specs/session-shell/spec.md)

PTY 与 shell 运行时位于 `workspaces` 和 `sessions` 限界上下文中；见 [Native bounded contexts](native-contexts.md)。
