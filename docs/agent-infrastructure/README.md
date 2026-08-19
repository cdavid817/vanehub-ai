# Agent 基础设施技术文档

这一组文档讲的是 VaneHub AI 所依赖的**协议与技术本身**，不是 VaneHub AI 的实现。

三层文档的分工：

| 文档 | 回答的问题 | 读者 |
| --- | --- | --- |
| [用户指南](../user-guide/zh-CN/src/index.md) | 这个功能怎么用 | 使用者 |
| [开发者指南](../developer-guide/zh-CN/src/index.md) | VaneHub AI 内部**为什么这样实现** | 贡献者 |
| **本组文档** | 底层协议与技术**本身**是什么、规范怎么定的 | 需要做技术选型或实现集成层的人 |

举例：想知道怎么在界面里加一个 MCP 服务器，看[用户指南的 MCP 章节](../user-guide/zh-CN/src/mcp.md)；想知道 VaneHub AI 的 MCP 中继为什么只覆盖两个 CLI，看开发者指南；想知道 MCP 协议的传输层、生命周期和 2026-07-28 无状态化变更，看本组的 [MCP 技术架构](mcp-architecture.md)。

## 协议与接口

| 文档 | 内容 |
| --- | --- |
| [MCP 技术架构](mcp-architecture.md) | 协议模型与三角色架构、传输层、核心原语、协议生命周期（含 2026-07-28 无状态化变更）、扩展框架、授权与安全模型。以 2026-07-28 规范为基准并覆盖旧版差异 |
| [Function Calling 技术架构](function-calling-architecture.md) | 通用调用循环与约束解码原理、Anthropic Tool Use 与 OpenAI Function Calling 的 API 细节对比、并行调用与流式组装、结构化输出、多 Provider 适配层设计 |
| [LSP 技术架构](lsp-architecture.md) | 协议分层与生命周期、能力协商、文本同步模型、语言特性与工作区特性、版本演进，以及 LSP 在 AI 编码 Agent 架构中的角色 |
| [A2A 技术架构](a2a-architecture.md) | 数据模型（AgentCard / Task / Message / Artifact）、任务状态机、发现机制、三种异步更新通道、协议绑定、安全模型 |

## Agent 能力与编排

| 文档 | 内容 |
| --- | --- |
| [多 Agent 系统技术架构](multi-agent-architecture.md) | 编排拓扑与角色框架、通信与协调机制、上下文管理策略、执行隔离（Worktree/沙箱）、跨组织互操作、失败模式分类与评估方法 |
| [Agent Skills 技术架构](agent-skills-architecture.md) | 开放规范与文件格式、渐进式披露加载模型、触发与执行机制、与 MCP/Prompt/Subagent 的定位对比、编写与评估方法论、安全模型 |
| [内置 CLI 参数完全参考](builtin-cli-reference.md) | VaneHub AI 编排的五种 CLI 的参数完全参考，逐一覆盖调用形态、会话管理、模型选择、权限与沙箱、输出格式、配置注入，并给出宿主按统一任务模型向各 CLI 投影参数的映射矩阵 |

## 检索与代码理解

| 文档 | 内容 |
| --- | --- |
| [RAG 技术架构](rag-architecture.md) | 索引与检索管线、语义检索与关键字检索的原理与取舍、混合检索与重排序、进阶范式与评估方法 |
| [Tree-sitter 技术架构](tree-sitter-architecture.md) | GLR 增量解析原理、语法开发工具链与 ABI 机制、查询系统、语言注入，以及结构化代码切分、Repo Map、代码检索等应用模式 |

## 工程方法

| 文档 | 内容 |
| --- | --- |
| [OpenSpec 技术架构](openspec-architecture.md) | 规范驱动开发（SDD）的动机与知识模型、变更包工件链、opsx 命令族与生命周期、Delta 规格合并机制、多工具集成原理 |

## 注意事项

- **这些文档描述的是外部规范，不是 VaneHub AI 的承诺**。文中出现的协议能力不代表 VaneHub AI 已经实现了它——实现范围以用户指南和开发者指南为准。
- **协议在演进**。MCP、A2A 这类规范的版本差异在文中有标注，但以各自官方规范的最新版为准。
- **仅提供简体中文**。
