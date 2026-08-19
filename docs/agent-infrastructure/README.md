# Agent 基础设施技术文档

讲的是 VaneHub AI 依赖的**协议与技术本身**，不是 VaneHub AI 的实现。怎么用某个功能见[用户指南](../user-guide/zh-CN/src/index.md)，VaneHub AI 内部为什么这样实现见[开发者指南](../developer-guide/zh-CN/src/index.md)。

| 主题 | 入口 |
| --- | --- |
| MCP | [协议模型与三角色架构、传输层、核心原语、生命周期、授权与安全模型](mcp-architecture.md) |
| Function Calling | [调用循环与约束解码、Anthropic 与 OpenAI 的 API 差异、并行调用与流式组装、结构化输出](function-calling-architecture.md) |
| LSP | [协议分层与生命周期、能力协商、文本同步模型、语言与工作区特性](lsp-architecture.md) |
| A2A | [AgentCard/Task/Message/Artifact 数据模型、任务状态机、发现机制、异步更新通道](a2a-architecture.md) |
| 多 Agent 系统 | [编排拓扑与角色框架、通信与协调、上下文管理、执行隔离、失败模式与评估](multi-agent-architecture.md) |
| Agent Skills | [开放规范与文件格式、渐进式披露加载、触发与执行、与 MCP/Prompt 的定位对比](agent-skills-architecture.md) |
| AI 编码 CLI 参数完全参考 | [五种 CLI 的参数族逐一覆盖，以及宿主向各 CLI 投影参数的映射矩阵](builtin-cli-reference.md) |
| RAG | [索引与检索管线、语义与关键字检索取舍、混合检索与重排序、评估方法](rag-architecture.md) |
| Tree-sitter | [GLR 增量解析、语法工具链与 ABI、查询系统、结构化代码切分与 Repo Map](tree-sitter-architecture.md) |
| OpenSpec | [规范驱动开发的知识模型、变更包工件链、opsx 命令族、Delta 规格合并](openspec-architecture.md) |

**这里描述的是外部规范，不是 VaneHub AI 的承诺**——文中出现的协议能力不代表已经实现，实现范围以上面两套指南为准。仅提供简体中文。
