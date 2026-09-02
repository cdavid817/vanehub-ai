# Agent 基础设施技术文档

本目录讲的是 VaneHub AI 依赖或可能集成的**外部协议、通用架构模式与工程方法**本身。

它不承担以下职责：

- **不是 VaneHub AI 已实现能力的承诺**——协议具备某项能力，不代表本项目实现了它；
- 不替代[使用者指南](../user-guide/zh-CN/src/index.md)（怎么用某个功能）；
- 不替代[开发者指南](../developer-guide/zh-CN/src/index.md)（本项目内部为什么这样实现）；
- 不保存本项目易变的运行时常量、provider 数量、CLI 参数或平台兼容矩阵。

判断某项能力在 VaneHub AI 里是否已交付，看上面两套指南、[OpenSpec 主规范](../../openspec/specs/)和生成参考，不看这里。仅提供简体中文。

## 协议

Agent 宿主与外部系统之间的互操作规范，各有独立的版本与治理方。

| 主题 | 适用范围 | 文档基线 |
| --- | --- | --- |
| [MCP](mcp-architecture.md) | Agent 宿主与工具/资源/提示词服务之间的互操作 | 协议模型与三角色架构、传输层、核心原语、生命周期、授权与安全模型 |
| [Function Calling](function-calling-architecture.md) | 模型向应用表达结构化工具调用意图 | 调用循环与约束解码、Anthropic 与 OpenAI 的 API 差异、并行调用与流式组装、结构化输出 |
| [LSP](lsp-architecture.md) | 编辑器/Agent 宿主与语言服务器之间的代码智能协议 | 协议分层与生命周期、能力协商、文本同步模型、语言与工作区特性 |
| [A2A](a2a-architecture.md) | 跨 Agent、跨进程或跨组织的任务委派与协作 | AgentCard/Task/Message/Artifact 数据模型、任务状态机、发现机制、异步更新通道 |

## 通用架构模式

不属于任何一份协议规范的工程模式——可以照着做，但没有哪家规范强制。

| 主题 | 适用范围 |
| --- | --- |
| [多 Agent 系统](multi-agent-architecture.md) | 编排拓扑与角色框架、通信与协调、上下文管理、执行隔离、失败模式与评估 |
| [Agent Skills](agent-skills-architecture.md) | 开放规范与文件格式、渐进式披露加载、触发与执行、与 MCP/Prompt 的定位对比 |
| [RAG](rag-architecture.md) | 索引与检索管线、语义与关键字检索取舍、混合检索与重排序、评估方法 |
| [Tree-sitter](tree-sitter-architecture.md) | GLR 增量解析、语法工具链与 ABI、查询系统、结构化代码切分与 Repo Map |

## 工程方法

| 主题 | 适用范围 |
| --- | --- |
| [OpenSpec](openspec-architecture.md) | 规范驱动开发的知识模型、变更包工件链、opsx 命令族、Delta 规格合并 |

OpenSpec 是一套具体工具与工程方法，不是 Agent 互操作协议，所以与 MCP、LSP、A2A 分开列。

## 不在本目录的：项目生成参考

CLI 参数相关的三份文档是 **VaneHub AI 的项目参考**，不是外部基础设施教程，已迁至 [`docs/reference/cli/`](../reference/cli/)：

- [内置 CLI 参数完全参考](../reference/cli/builtin-cli-reference.md)
- [CLI 参数矩阵](../reference/cli/parameter-matrix.md)（由权威注册表生成，不要手工编辑）
- [CLI 参数注册表维护流程](../reference/cli/maintenance.md)

## 写作与维护规则

本目录的每条事实分四类，写的时候要能分清自己在写哪一类：

1. **规范要求**——上游规范明确保证的行为，必须能指到官方规范的对应章节；
2. **常见实现**——生态里普遍这么做但规范并不强制，要写明「常见做法」而不是「协议规定」；
3. **架构建议**——本文作者的工程判断，要写成「建议」，不得伪装成协议要求；
4. **本项目适用性**——为什么与 VaneHub AI 相关，只链接到开发者指南，不在这里自行承诺实现状态。

其余约束：

- 版本、日期与厂商支持情况必须有上游官方来源，并在正文中写明核对日期；易变的生态数据超过半年未核对就应当复核或删除。
- 不用博客二手总结替代规范原文。
- 不把某一家厂商的实现机制概括成跨厂商的通用事实。
- 不在本目录复制 VaneHub AI 的 provider 数、工具数、语言数、CLI 参数与运行时常量——那些属于生成参考或开发者指南。
