# 跨会话记忆

存储的记忆是一个共享的、主机级单池，由每一个 Agent 共享——OnePiece 和所有 CLI 包装的 Agent（`claude-code`、`codex-cli`、`gemini-cli`、`opencode`、`antigravity-cli`）一视同仁。它们不按产生记忆的 Agent 或 workspace 文件夹来划分作用域。这是 `retrieval` 限界上下文的持久化部分；检索/搜索路径见 [Retrieval and vector search](retrieval.md)。

## 共享的主机级记忆池

当记忆在一个带有 workspace 文件夹的会话中被保存时，产生该记忆的 Agent id 和那个 workspace 文件夹会作为**来源元数据**记录在存储记录上，而不作为注入、列出或管理的过滤条件。其后果：

- 在某个 Agent 下保存的记忆，对其他所有 Agent 的生成和管理视图都可见，就如同它们自己产生的一样。
- 在没有 workspace 文件夹的会话中保存的记忆仍会被存入共享池（不记录文件夹，也不拒绝），并且可从任何 workspace 或无 workspace 的情况下读取、注入和管理。
- Agent id 和 workspace 文件夹仅作为来源元数据；检索不按它们过滤，也不将它们作为检索工具的输入暴露。

## 保存记忆

- **OnePiece** 在其自身的 API tool-calling loop 中暴露一个保存记忆的工具。当记忆启用开关打开时，该工具会被自动批准——它无需用户确认即可立即持久化。
- **CLI 包装的 Agent** 不暴露该工具，因为 VaneHub 不控制 CLI 包装 Agent 自身的内部工具系统。它们通过单独的自动抽取机制产生记忆，该机制由其各自的需求约束。

## 设计所在之处

本章用于引导贡献者。权威需求位于 spec 中。

- [openspec/specs/agent-cross-session-memory](../../../../openspec/specs/agent-cross-session-memory/spec.md) —— 共享池、来源元数据和保存路径。
- [openspec/specs/retrieval-vector-search](../../../../openspec/specs/retrieval-vector-search/spec.md) —— 检索工具与降级。

记忆持久化与检索位于 `retrieval` 限界上下文中；见 [Native bounded contexts](native-contexts.md)。
