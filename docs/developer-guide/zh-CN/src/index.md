# VaneHub AI 开发者指南

本指南面向参与 VaneHub AI 开发的贡献者，解释**所有权与集成边界**。源代码、OpenSpec 主规范以及生成的 Rustdoc 仍然是细节的权威来源。

需要回答下面这类问题时用它：

- 一处前端或 native 改动应该放在哪里？
- 桌面端的哪些运行时行为是真实的，哪些是在 Web 预览中模拟的？
- 每个限界上下文各自拥有哪些数据、流程和日志？
- 变更是如何被规约、验证、打包和发布的？

## 先读这三章

不熟悉这个代码库的话，按顺序读完这三章再看别的——它们决定你后面每一章能不能读懂。

| 章节 | 覆盖内容 |
| --- | --- |
| [仓库结构与模块导览](repository-orientation.md) | 前端、native 与规范工作各自落在何处 |
| [运行时与服务边界](runtime-boundaries.md) | 服务层，以及桌面端上哪些行为是真实的 |
| [Native 限界上下文](native-contexts.md) | 21 个上下文各自拥有什么，以及它们之间怎么说话 |

## Agent 执行

| 章节 | 覆盖内容 |
| --- | --- |
| [Agent 生命周期与 provider 运行时](agent-lifecycle.md) | 注册 Agent 编辑、稳定 provider 解析、能力声明 |
| [OnePiece native Agent](onepiece-native-agent.md) | 内置 API Agent 身份、Profile 生命周期与 provider 目录 |
| [OnePiece 内置工具](onepiece-builtin-tools.md) | 扩展原生工具集的发布门控、依赖与回退触发条件 |
| [CLI 生命周期与全局配置](cli-lifecycle.md) | CLI 检测、冲突判定、升级资格，以及改写各 CLI 配置文件的约束 |
| [终端与 PTY 运行时](terminal-runtime.md) | 会话级 Agent Terminal、自动启动与附着、远程终端 |
| [SSH 连接与远程运行时](ssh-connections.md) | 主机密钥信任、有界字段校验、远程通道事件与连接池限额 |
| [Tool registry 与执行](tool-registry.md) | 固定原生工具目录、按 interface_format 翻译、多轮工具循环 |
| [扩展工具上下文](extended-tool-contexts.md) | 沙箱执行、浏览器、联网研究与产物存储，以及各自的门控与隔离约束 |
| [CLI 委派与 ChangeSet 管线](cli-delegation.md) | 隔离执行、ChangeSet 捕获与封存、一次性精确应用与回滚胶囊 |
| [多 Agent 群聊](multi-agent-group-chat.md) | 席位模型、中途增减、轮次路由与持久化 presence |
| [Loop 运行时与会话 Plan 模式](loop-and-plan-runtime.md) | 持久化 Loop 执行与 OnePiece 会话内的只读 Plan 模式 |
| [目标与任务看板](goals-and-work-board.md) | 目标状态机与派生验收、看板的幂等对账 |
| [会话恢复](session-recovery.md) | 恢复状态与生命周期正交、持久化执行身份与所有权 |

## 上下文与检索

| 章节 | 覆盖内容 |
| --- | --- |
| [上下文压缩](context-compaction.md) | Token-aware 触发与字符回退、摘要式压缩、冷却与熔断 |
| [跨会话记忆](cross-session-memory.md) | 主机级共享池、provenance 元数据、自动提取 |
| [检索与向量搜索](retrieval.md) | 记忆池与工作区代码索引、优雅降级 |
| [Tree-sitter 代码索引](tree-sitter-code-indexing.md) | 语法解析、bounded chunk、符号元数据、grammar 版本与脱敏 |
| [LSP 代码智能](lsp-code-intelligence.md) | 会话内 LSP 集成、工作区信任与能力协商 |

## 工具与扩展

| 章节 | 覆盖内容 |
| --- | --- |
| [Skill 管理](skill-management.md) | 双 scope、SKILL.md 契约、漂移与内建播种/对账 |
| [有效 Skill 运行时](effective-skill-runtime.md) | 运行时如何把多个来源解析成一份生效的 Skill 集 |
| [Skill 覆盖层治理](skill-overlay-governance.md) | 覆盖层的优先级、冲突处理与治理规则 |
| [Skill 演进证据](skill-evolution-evidence.md) | 归因依据与可用范围分级、信号分类、脱敏与加密存储 |
| [MCP 工具与客户端](mcp-tools.md) | 传输与配置模型、原生工具目录中的 MCP 工具 |
| [IM connector](im-connectors.md) | 内建连接器、直发消息范围与入站路由 |

## 策略与可观测

| 章节 | 覆盖内容 |
| --- | --- |
| [权限模型](permission-model.md) | 统一决策点、显式 Deny 优先、审批代理、CLI flag 投影、Claude Code 钩子桥 |
| [执行可观测性与 Agent 评测](execution-observability.md) | run/span/时间线、四档保真度与脱敏上限，以及评测竞技场的判定规则 |
| [持久化与统一日志](persistence-and-logging.md) | SQLite、迁移以及脱敏规则 |
| [使用统计](usage-statistics.md) | 上报 token 与估算字符分离、时间范围、per-Agent 拆分 |

## 工程流程

| 章节 | 覆盖内容 |
| --- | --- |
| [OpenSpec 工作流](openspec-workflow.md) | 如何提出、应用与归档一个变更 |
| [测试、打包与发布](testing-and-release.md) | 关卡、覆盖率门槛与打包目标 |

## 参考

| 章节 | 覆盖内容 |
| --- | --- |
| [Native API 参考](native-api-reference.md) | 由 Rust `//!` 与 `///` 文档生成 |

参考章节是生成产物，故意与本叙事指南分开维护。

## 本仓库中的其他文档

这些不在本指南的章节列表中，但属于仓库文档的一部分。

| 文档 | 覆盖内容 |
| --- | --- |
| [CLI Agent 全局配置](../../../cli-agent-global-configuration.md) | 五个 CLI Agent 的用户级 provider profile，以及为何保存一个 profile 永远不会改变当前活跃的 Agent 或 Session |
| [内置模型提供商目录](../../../model-providers.md) | 25 家提供商的端点协议、默认模型与凭据存放方式 |
| [Agent 基础设施技术文档](../../../agent-infrastructure/README.md) | MCP、LSP、Function Calling、RAG 等**协议与技术本身**，不是 VaneHub AI 的实现 |
| [Native 构建性能](../../../build-performance.md) | 各平台链接器要求、release profile 行为与实测构建证据 |
| [发布签名](../../../release-signing.md) | 已发布产物的签名与验证链 |
| [桌面端发布验证](../../../desktop-release-verification.md) | 一次桌面端发布在发布前必须逐平台通过的验证流程 |
| [运行时性能预算](../../../runtime-performance-budgets.md) | 已声明的运行时预算,以及针对它们的回归如何被报告 |
| [CLI Agent 全局配置](../../../cli-agent-global-configuration.md) | VaneHub AI 如何写入各 CLI 自己的全局配置,以及测试如何隔离它 |
| [统一工作台 UI 重构基线](../../../ui-redesign/baseline.md) | `redesign-unified-workbench-ui` 的里程碑 0 "改动前"证据:校验命令结果、已知缺陷、已确认的运行时架构事实,以及截图与 Fixture 引用 |
| [统一工作台设计系统参考](../../../ui-redesign/design-system.md) | `redesign-unified-workbench-ui` 在 `src/styles.css` 中新增的语义 Token、表面层级、文本/截断规则,以及各行/卡片的元数据预算 |
| [统一工作台视觉 Token 审计](../../../ui-redesign/token-audit.md) | 支撑上述 Token 的任务 2.1 硬编码色值/圆角/高度/阴影审计,以及哪些项被有意延后到后续里程碑及原因 |

### Provider SDK

Provider SDK 文档位于 `docs/provider-sdk/`,是第三方 provider 插件要实现的契约。`openspec/specs/provider-plugin-sdk` 要求它们存在于该位置。

| 文档 | 讲什么 |
| --- | --- |
| [Provider 契约](../../../provider-sdk/contract.md) | provider 要实现的接口,以及它必须维持的保证 |
| [Manifest](../../../provider-sdk/manifest.md) | manifest schema、必填字段与版本兼容性 |
| [示例 provider](../../../provider-sdk/example-provider.md) | 一个仅供测试的参考实现,端到端走一遍 |
| [一致性测试](../../../provider-sdk/conformance-testing.md) | provider 提交前要跑的一致性流程 |
| [安全规则](../../../provider-sdk/security-rules.md) | provider 插件运行时所受的限制 |

### 时间点快照

**这些是快照，而非持续维护的叙事。** 它们描述的是其所标注修订版本当时的系统状态，其中的 `文件:行号` 引用锚定到该修订版本——也正是在这些地方最可能发生漂移。读它们是为了了解某个子系统当初的形态；要了解当前状态请看上面的章节和规范。

| 文档 | 编写时所基于的 |
| --- | --- |
| [VaneHub AI 技术架构深度解析](../../../VaneHub-AI-技术架构深度解析.md) | Commit `bb3d28d8`，2026-08 |

## 文档状态

本指南记录的是 `main` 分支的架构。**某项功能并不会仅仅因为存在某个服务或 native command 就被视为已交付给用户**；它还必须存在用户可见的路径以及相应的验证证据。

[Native 限界上下文](native-contexts.md)那张地图与 `src-tauri/src/contexts/` 由 `npm run docs:links:check` 强制对齐：新增一个上下文却不在地图里加一行，校验会直接失败。
