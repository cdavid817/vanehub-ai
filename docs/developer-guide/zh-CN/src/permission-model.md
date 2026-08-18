# 权限模型

每个受控操作——无论是来自 native API Agent 的 tool-use 循环的请求,还是经由 Claude Code permission-hook 桥接转发——都通过单一决策点进行评估。CLI 发起的调用没有独立的决策引擎。

## 统一决策模型

评估将一个 `(principal, action, resource)` 三元组解析为 `Allow`、`Deny` 或 `Ask` 三者之一。一个 principal 仅由**稳定的 agent id** 唯一标识——每个 Agent 一个持久的 principal,在该 Agent 参与的每一个会话中都保持不变。会话 id 与 generation id 是每次评估的上下文,不属于 principal 身份的一部分。因此,Agent 以与其其他会话相同的 principal 与策略分配参与新会话,而非使用会话作用域的分配。

未匹配的操作(没有策略匹配 principal/action/resource)解析为 `Ask`,绝不解析为 `Allow`。

## 解析顺序:显式 Deny 优先

相互冲突的策略匹配按显式 `Deny` 优先于显式 `Allow`、显式 `Allow` 优先于默认 `Ask` 的方式解析。

## 审批代理

待处理的审批请求以 native 运行时为唯一真源持有,与是否收到关于它们的任何前端事件无关。遗漏的前端事件不会让一次生成静默等待:前端通过事件推送新的待处理审批,**并**在挂载/重连时拉取完整待处理列表进行对账。一个待处理审批以一个批准/拒绝决定连同 `Once`、`Session`、`Project` 或 `Global` 之一的内存作用域一并解决。

## CLI 启动参数投影

对于 `gemini-cli`、`codex-cli` 和 `opencode`,只要 Agent 的 Agent Terminal 以交互方式启动,Agent principal 所分配的策略模板(`readonly`、`standard`、`trusted` 或 `yolo`)就会被投影为该工具自身的 native 审批/沙箱启动参数。仅使用目录合法、不可绕过的参数值——不会引入任何原始的绕过 flag(例如名称中包含 "dangerously" 的 flag)来达成某个模板的行为。`trusted` 和 `yolo` 投影为相同的启动参数。

## Claude Code permission-hook 桥接

来自 hook 包装器的 `PreToolUse` 请求被转换为一个 `Action`/`Resource` 对,并通过与 native API Agent 相同的 `evaluate()`/`ApprovalBroker` 管线解析。该 hook 仅匹配 `Bash`、`Edit`、`Write`、`Read`、`Glob`、`Grep` 以及 MCP 工具名(`mcp__*`),将它们映射到 `shell.exec`/`file.write`/`file.read`/`mcp.tool`;任何其他工具(例如 `WebFetch`)都不会被拦截,Claude Code 的 native 行为不受影响。`Ask` 解析会在既有的 `ApprovalCard` UI 中创建一个待处理审批,并挂起 HTTP 响应,直到人工决定或超时扫描。

## 设计所在之处

本章用于为贡献者定位。权威需求位于规范之中。

- [openspec/specs/permissions-core](../../../../openspec/specs/permissions-core/spec.md) —— 统一决策模型与解析顺序。
- [openspec/specs/permissions-approval](../../../../openspec/specs/permissions-approval/spec.md) —— 审批代理、待处理状态与内存作用域。
- [openspec/specs/cli-agent-permission-launch-flags](../../../../openspec/specs/cli-agent-permission-launch-flags/spec.md) —— CLI 启动参数投影。
- [openspec/specs/claude-code-permission-hook](../../../../openspec/specs/claude-code-permission-hook/spec.md) —— Claude Code `PreToolUse` 桥接。

权限评估位于 `agent_runtime` 限界上下文中;参见 [Native 限界上下文](native-contexts.md)。
