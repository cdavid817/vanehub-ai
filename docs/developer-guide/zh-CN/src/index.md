# VaneHub AI 开发者指南

本指南是为参与 VaneHub AI 开发的贡献者整理的入口文档。它解释了所有权与集成边界;源代码、OpenSpec 主规范以及生成的 Rustdoc 仍然是权威的细节来源。

当你需要回答以下问题时,请使用本指南:

- 一处前端或 native 改动应该放在哪里?
- 桌面端的哪些运行时行为是真实的,哪些是在 Web 预览中模拟的?
- 每个 bounded context 各自拥有哪些数据、流程和日志?
- 变更是如何被规约、验证、打包和发布的?

## 章节

| 章节 | 覆盖内容 |
| --- | --- |
| [仓库导览](repository-orientation.md) | 前端、native 与规范工作各自落在何处 |
| [运行时与服务边界](runtime-boundaries.md) | 服务层,以及桌面端上哪些行为是真实的 |
| [Native bounded context](native-contexts.md) | 每个 Rust context 各自拥有什么 |
| [持久化与统一日志](persistence-and-logging.md) | SQLite、迁移以及脱敏规则 |
| [测试、打包与发布](testing-and-release.md) | 关卡、覆盖率门槛与打包目标 |
| [OpenSpec 工作流](openspec-workflow.md) | 如何提出、应用与归档一个变更 |
| [Native API 参考](native-api-reference.md) | 由 Rust `//!` 与 `///` 文档生成 |

参考章节是生成产物,故意与本叙事指南分开维护。

## 本仓库中的其他文档

这些文档不在本指南的章节列表中,但属于仓库文档的一部分。

| 文档 | 覆盖内容 |
| --- | --- |
| [CLI Agent 全局配置](../../../cli-agent-global-configuration.md) | Claude Code、OpenCode 与 Codex CLI 的用户级 provider profile,以及为何保存一个 profile 永远不会改变当前活跃的 Agent 或 Session |
| [Native 构建性能](../../../build-performance.md) | 各平台链接器要求、release profile 行为与实测构建证据 |
| [发布签名](../../../release-signing.md) | 已发布产物的签名与验证链 |

### 时间点快照

**这些是快照,而非持续维护的叙事。** 它们描述的是其所标注修订版本当时的系统状态,其中的 `文件:行号` 引用锚定到该修订版本——也正是在这些地方最可能发生漂移。阅读它们是为了了解某个子系统当初的形态,而要了解当前状态请参考上面的章节和规范。

| 文档 | 编写时所基于的 |
| --- | --- |
| [VaneHub AI 技术架构深度解析](../../../VaneHub-AI-技术架构深度解析.md)(简体中文) | Commit `bb3d28d8`,2026-08 |

## 文档状态

本指南记录的是 `main` 分支的架构。某项功能并不会仅仅因为存在某个服务或 native command 就被视为已交付给用户;它还必须存在用户可见的路径以及相应的验证证据。
