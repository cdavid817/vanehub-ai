# VaneHub AI 架构与实现（中文）

> **本文档集只讲一件事：VaneHub AI 是怎么实现的**。分层怎么切、11 个限界上下文各管什么、四个 CLI 的差异在哪吸收、权限判定走哪四步、Span 怎么串起来。
>
> **想知道怎么用**，去[用户指南](../user/zh-CN/index.html)——那里是任务导向的操作步骤，带截图。两套文档不重复：那套讲怎么用，这套讲怎么实现。

## 直接开始

| 你的处境 | 从这里读 |
|---|---|
| 第一次接触这个代码库 | [架构总览](02-architecture/README.md) —— 分层全貌、三种运行时、功能↔上下文对照表 |
| 要在本地跑起来 | [开发环境搭建](03-development/setup.md) —— 装环境、跑校验、避开已知陷阱 |
| 要改某个功能，但不知道改哪 | [架构总览 · 功能与限界上下文的对应](02-architecture/README.md#功能与限界上下文的对应) |
| 要接一个新的 CLI Agent | [CLI 集成](02-architecture/cli-integration.md) —— 六个差异吸收点与十处需要改的地方 |
| 提交前想确认会被什么拦下来 | [五层约束体系](03-development/constraints.md) |

## 全部文档

**概览**

| 文档 | 内容 |
|---|---|
| [项目定位与用途](01-overview.md) | 定位、痛点对照、目标场景、5 个内置 Agent、平台与语言、演进方向 |

**架构 · 基础**——先读这三篇，其余各篇都建立在它们之上

| 文档 | 内容 |
|---|---|
| [架构总览](02-architecture/README.md) | 分层全貌、三种运行时、启动装配、**用 syn 做的架构测试**、功能↔上下文对照、**几处容易误解的地方** |
| [技术栈与选型理由](02-architecture/tech-stack.md) | 版本固定三档策略、各层依赖与理由、构建配置 |
| [十一个限界上下文](02-architecture/bounded-contexts.md) | 职责与命令面、`tooling` 元上下文、**双向依赖如何不成环** |
| [端口与适配器](02-architecture/ports-and-adapters.md) | 四层结构、端口粒度、时钟与 id 也是端口、测试替身 |

**架构 · 横切机制**——跨多个上下文的公共设施

| 文档 | 内容 |
|---|---|
| [CLI 集成](02-architecture/cli-integration.md) | **六个差异吸收点**、四条测试定义的行为、各 CLI 特例汇总 |
| [进程管理与 PTY](02-architecture/process-and-pty.md) | 两条进程路径、Job Object、包装脚本、**流式 UTF-8 解码** |
| [权限架构](02-architecture/permissions-architecture.md) | **四步判定顺序与失败关闭**、五个受管动作、模板规则、Principal 生命周期、钩子桥接 |
| [可观测性架构](02-architecture/observability-architecture.md) | 四级 Span、保真度、traceparent 传播、**两级脱敏**、统一日志的四条硬约束 |
| [数据层](02-architecture/data-layer.md) | 连接池、迁移机制、**版本号冲突陷阱**、70 张表清单 |
| [前端架构](02-architecture/frontend.md) | services 三件套、运行时选择、错误归一化、纯逻辑分离、**i18n 三道守卫** |
| [MCP 集成](02-architecture/mcp-integration.md) | rmcp、**应用二进制重入充当中继**、按 Agent 分派、私有目录 |

**架构 · 功能域实现**——单个能力从领域模型到落库的完整链路

| 文档 | 内容 |
|---|---|
| [会话](02-architecture/sessions.md) | 会话模型、标识校验、聊天配置与推理深度钳制、分类、归档保护、9 个标签页 |
| [多 Agent 群聊](02-architecture/group-chat.md) | 席位、句柄派生、**交接解析的五条防御**、三种交回意图、模型族判定 |
| [Loop 工程化运行时](02-architecture/loop-engineering.md) | 七态五阶段、判定优先级、十二种终止原因、**三维指纹无进展检测**、启动恢复 |
| [个性化](02-architecture/personalization.md) | Custom Instructions、共享记忆池、**OnePiece 代做提取**、专家角色与评审策略 |
| [原生 API Agent](02-architecture/native-agent.md) | 25 家 provider、调用构造、两层记忆、**记忆池混合检索与 RRF 融合** |
| [工具生态](02-architecture/tooling.md) | MCP、Skills 漂移检测、Prompt Hooks、**OCR/ASR/TTS 扩展**、两个受管 SDK、CLI 冲突 |
| [项目与工作区](02-architecture/workspaces.md) | 项目探测、worktree、**两层路径防逃逸**、shell、输出捕获与容量常量 |
| [远程与 IM](02-architecture/remote-and-im.md) | SSH 与 **TOFU 主机密钥**、连接池、五个 IM 连接器、七态生命周期、字段级密级 |
| [自动化与洞察](02-architecture/automation.md) | 定时任务与时区分工、长时操作、通知作用域、**四维 token 与幂等采集**、桌面集成 |

**开发**

| 文档 | 内容 |
|---|---|
| [开发环境搭建](03-development/setup.md) | 前置要求、校验命令、覆盖率门槛、E2E 端口复用坑、**迁移版本号冲突** |
| [OpenSpec 工作流](03-development/openspec-workflow.md) | 三阶段、工件组合分布、归档治理、从归档反查历史、**规范落后于代码时怎么办** |
| [五层 AI 协作约束体系](03-development/constraints.md) | 宪法 → OpenSpec → Skills → Hooks → CI，**以及这套体系拦不住什么** |

## 几个高频问题的直达入口

| 问题 | 去这里 |
|---|---|
| 为什么我的 CLI Agent 没有记忆？ | [个性化 · 提取时机与执行者](02-architecture/personalization.md#提取时机与执行者) |
| `Trusted` 和 `Yolo` 有什么区别？ | [权限架构 · 模板规则的实际内容](02-architecture/permissions-architecture.md#模板规则的实际内容) |
| 检索能搜到我的项目代码吗？ | [原生 API Agent · 检索的对象](02-architecture/native-agent.md#检索的对象是记忆池不是项目代码) |
| 启动报 `no such table`？ | [开发环境搭建 · 迁移版本号冲突](03-development/setup.md#迁移版本号冲突) |
| `AgentAdapter` trait 在哪？ | [CLI 集成 · 先澄清一个常见误解](02-architecture/cli-integration.md#先澄清一个常见误解) |
| 终端输出为什么少了一段？ | [项目与工作区 · 终端输出捕获](02-architecture/workspaces.md#终端输出捕获) |
| 加一个新 CLI 要改哪些地方？ | [CLI 集成 · 加一个新 CLI](02-architecture/cli-integration.md#加一个新-cli-需要动哪些地方) |

## 与仓库其他文档的关系

| 文档 | 语言 | 定位 |
|---|---|---|
| **本文档集**（`docs/zh/`） | 中文 | **架构与实现**：怎么设计的、为什么这么设计 |
| [用户指南](../user/zh-CN/index.html) | 英文 / 简体中文 | **怎么用**：任务导向的操作步骤，含截图 |
| [开发者指南](../developer/index.html) | 英文 | 原生边界、API 参考、发布流程 |
| 根 `README` | 英文 / 简体中文 / 日本語 | 项目门面与快速上手 |

**英文版**：本文档集当前仅有中文，英文版为后续工作。

## 编写约定

贡献时请保持一致：

| 约定 | 说明 |
|---|---|
| **结论先行** | 每节第一句给结论，再展开细节 |
| **有据可查** | 所有模块名、trait 名、表结构均来自实际代码或 OpenSpec 归档；引用代码时给出 `路径:行号` |
| **不臆测** | 找不到依据的内容标注 `TODO: 待确认`，不编造 |
| **写"为什么"** | 优先记录设计动机与取舍，而非代码翻译 |
| **技术名词保留英文** | trait、Span、PTY、worktree 等不强行翻译 |
| **多用表格** | 对比与清单优先用表格表达 |
| **Mermaid 内嵌** | 图表用 Mermaid 源码嵌入，不用外链图片 |
| **每篇都有「边界与限制」** | 讲清什么做不到，与讲清什么能做同样重要 |
| **相对路径互链** | 文档间用相对路径引用 |
| **句末标点放在加粗外** | 写 `**结论**。下一句`，不要写 `**结论。**下一句`——后者在 CommonMark 里不构成合法的闭合定界符，`docs:check` 会拦下来 |

**改完文档后必须跑**：

```bash
npm run docs:check
```

它包含链接校验（`scripts/validate-docs.mjs`），覆盖整个 `docs/` 目录——**断链会直接让 CI 的 documentation job 失败**。
