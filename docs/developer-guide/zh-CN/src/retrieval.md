# 检索与向量搜索

`retrieval` 限界上下文拥有两套相互独立的搜索:一个宿主级跨会话**内存**池(向量 + FTS),以及一个按工作区划分的**代码索引**(Tree-sitter + FTS + 向量)。两者都会优雅降级,绝不会因为搜索错误而导致一次生成失败。

## 共享的宿主级内存池

检索搜索的宿主级内存池,与基于新近度的内存注入所取用的池相同(`agent-memory-shared-pool`)。召回**不**受 agent id 或工作区文件夹限制。agent id 与工作区文件夹仅作为来源元数据记录在索引行上,不作为召回工具的输入暴露:

- 在另一个 agent 下保存的内存,可从任意 agent 的会话中召回。
- 召回绝不会返回内存注入已放入系统提示的严格子集。
- 召回工具的输入 schema 恰好只暴露 `query` 和 `limit`——没有 agent id、文件夹或作用域参数,因为共享池没有可供模型指名的切片。

## 优雅降级

检索失败绝不会导致一次生成失败。工具返回一个描述不可用状态的成功结果:

- 搜索期间嵌入 provider 不可达 → 仅关键词结果,标记为 `degraded: keyword_only`。
- FTS5 查询失败 → 仅向量结果,标记为 `degraded: vector_only`。
- 两条路径都执行且都没有命中 → 返回空结果列表,而非错误。

## 工作区代码索引

持久化的代码索引按工作区划分作用域:工作区身份、文件清单、代码块、符号、向量与有界的本地审计记录。native worker 执行元数据优先的对账,仅读取或解析新增或变更的文件。Tree-sitter 语法、分块查询与脱敏策略共享一个版本标记。工作区代码嵌入受一个绑定到工作区 id、generation、provider 配置与模型的显式确认门控。FTS 保持按工作区划分作用域,并在确认之前可用;来自其他工作区或模型的向量永远不会成为候选。

## 设计所在之处

本章用于为贡献者定位。权威需求位于规范之中。

- [openspec/specs/retrieval-vector-search](../../../../openspec/specs/retrieval-vector-search/spec.md) —— 共享内存池、召回工具、降级。
- [openspec/specs/workspace-code-indexing](../../../../openspec/specs/workspace-code-indexing/spec.md) —— 工作区代码索引、对账、嵌入确认。

`retrieval` 限界上下文在 [Native 限界上下文](native-contexts.md)中描述。
