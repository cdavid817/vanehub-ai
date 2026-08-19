<div align="center">

[English](README.md)
· **简体中文**
· [日本語](README.ja.md)

</div>

<!-- docs-section:hero -->

# VaneHub AI

<p align="center">
  <img src="public/icon-512.png" alt="VaneHub AI 应用图标" width="160" />
</p>

通过统一 React 界面和明确的 Web/mock、Tauri runtime 边界管理 AI Coding Agent 的桌面优先工作台。

<!-- docs-fact:project-version value:0.1.0-preview.1 -->
<!-- docs-fact:tauri-major value:2.x -->
<!-- docs-fact:react-major value:19.x -->

[![Version](https://img.shields.io/badge/version-0.1.0--preview.1-blue.svg)](package.json)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8DB.svg)](src-tauri/Cargo.toml)
[![React](https://img.shields.io/badge/React-19.x-61DAFB.svg)](package.json)
[![CI](https://github.com/cdavid817/vanehub-ai/actions/workflows/ci.yml/badge.svg)](https://github.com/cdavid817/vanehub-ai/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-green.svg)](LICENSE)

<!-- docs-section:overview -->

## 项目简介

VaneHub AI 把 Claude Code、OpenCode、Codex CLI、Gemini CLI 和 Antigravity CLI 汇集到统一桌面工作台中。它管理 CLI 可用性、会话、终端执行、项目与 worktree、设置、工具、可观测性和桌面集成，同时避免 React 组件直接依赖 native API。

### 支持的 CLI

装一个就能开始，不必五个都装。

| Agent | 提供方 | 命令 | 模型族 | 应用内安装 | 第三方模型端点 |
| --- | --- | --- | --- | --- | --- |
| Claude Code | Anthropic | `claude` | Anthropic | ✅ `@anthropic-ai/claude-code` | ✅ |
| Codex CLI | OpenAI | `codex` | OpenAI | ✅ `@openai/codex` | ✅ |
| OpenCode | OpenCode（开源） | `opencode` | 未知 | ✅ `opencode-ai` | ✅ |
| Gemini CLI | Google | `gemini` | Google | ✅ `@google/gemini-cli` | ⚠️ 端点可改，但目录中只有官方预设 |
| Antigravity CLI | Google | `agy` | Google | ❌ 无 npm 包，走官方安装脚本 | ❌ 只接受 Google 登录 |

- 应用内安装指能否在设置 → CLI 管理里由 VaneHub AI 代为安装与升级——它只走 npm，Homebrew、winget、scoop 装的那份需要你用原来的来源升级。
- 第三方模型端点指能否在设置 → Agent 配置里把该 CLI 指向 DeepSeek、OpenRouter 一类兼容端点。**各家的官方订阅登录（OAuth）一律在终端里完成**，VaneHub AI 不代管。
- OpenCode 的模型族是「未知」而非漏填：它驱动的是你自己配置的任意模型，没有固定归属，「要求评审来自不同模型族」这类策略对它不生效。
- Gemini CLI 正在被 Antigravity CLI 取代，Google 自 2026-06-18 起对个人/免费账号逐步停用它。
- 不想装任何 CLI，可以直接用内置的原生 API Agent OnePiece——它通过 HTTP 调模型，完全在应用内运行，详见下面的使用者指南。

### 支持的模型提供商

内置 25 家提供商的配置模板，同时供 OnePiece 和三个 CLI Agent 使用；目录之外可填自定义兼容端点。

| 类别 | 提供商 |
| --- | --- |
| 官方 | Anthropic、OpenAI |
| 聚合与云平台 | OpenRouter、SiliconFlow、Alibaba Bailian、Volcengine Ark、Together AI、Fireworks AI、NVIDIA NIM、ModelScope、PPIO、Qiniu AI |
| 模型厂商 | DeepSeek、Zhipu GLM、Kimi / Moonshot、xAI、Mistral AI、MiniMax、MiniMax Global、StepFun、Baichuan AI、Xiaomi MiMo、Z.AI |
| 推理加速 | Groq、Cerebras |

**一家提供商能配给哪个 Agent，取决于它提供的端点协议**：Anthropic Messages 的 16 家可配 Claude Code，OpenAI Chat Completions 的 24 家可配 Codex CLI 与 OpenCode。

完整目录、各家图标、端点协议、默认模型与 API Key 申请入口见[内置模型提供商目录](docs/model-providers.md)。

<!-- docs-section:download -->

## 下载

预构建的桌面安装包发布在 [Releases 页面](https://github.com/cdavid817/vanehub-ai/releases)：Windows `.exe` 安装器、macOS `.dmg`，以及 Linux 的 `.deb` 与 AppImage。不发布 `.msi` 和 `.rpm`。

当前构建是未签名的预览版。Windows 与 macOS 在运行前会给出警告，各平台的处理步骤记录在 release notes 中。安装前请用发布的 `SHA256SUMS` 校验下载文件。

<!-- docs-section:documentation -->

## 文档

<!-- docs-locale-guides -->

### 使用者指南

| 主题 | 入口 |
| --- | --- |
| 快速开始 | [五步走完，从装 CLI 到在工作区里干活](docs/user-guide/zh-CN/src/quick-start.md) |
| 基础配置 | [界面语言、主题、字号、默认权限模板、开机自启、网络代理、数据目录、日志目录](docs/user-guide/zh-CN/src/user-interface.md#基础配置) |
| 用户界面总览 | [主布局、导航、面板切换、会话/对话/工作区标签页/信息面板](docs/user-guide/zh-CN/src/user-interface.md) |
| 会话列表 | [分组/搜索/筛选/批量/拖拽、右键菜单、专注模式](docs/user-guide/zh-CN/src/user-interface.md#会话列表) |
| 浮动助手 | [独立浮窗会话、状态徽章、主操作菜单](docs/user-guide/zh-CN/src/user-interface.md#浮动助手) |
| 循环中心 | [Loop 运行控件、验证命令、时间线](docs/user-guide/zh-CN/src/loop-engineering.md) |
| 计划中心 | [任务图生成、审批约束、运行与验收](docs/user-guide/zh-CN/src/user-interface.md#计划中心) |
| 目标中心 | [把分散的执行体归到一处追踪](docs/user-guide/zh-CN/src/goal-management.md) |
| 任务看板 | [任务的看板视图与状态流转](docs/user-guide/zh-CN/src/todo-board.md) |
| Agent 评测 | [同题对照跑多个 Agent，比通过率、token 与耗时](docs/user-guide/zh-CN/src/evaluation.md) |
| 通知中心 | [铃铛、未读数、全部已读、清除](docs/user-guide/zh-CN/src/user-interface.md#通知) |
| 系统托盘 | [显示/隐藏主窗口、开机自启、通知联动](docs/user-guide/zh-CN/src/user-interface.md#系统托盘) |
| CLI 管理、安装与认证 | [两种安装方式、认证、安装检测、冲突诊断与升级](docs/user-guide/zh-CN/src/getting-started.md) |
| 多 Agent 群聊 | [席位、`@` 交接、轮次边界](docs/user-guide/zh-CN/src/multi-agent-workflow.md) |
| Git Worktree | [并行改同一仓库而不互相踩踏](docs/user-guide/zh-CN/src/worktree.md) |
| 代码评审 | [评审中心与评审流程](docs/user-guide/zh-CN/src/code-review.md) |
| 斜杠命令 | [会话内的命令入口](docs/user-guide/zh-CN/src/slash-commands.md) |
| 记忆与上下文 | [跨会话记忆与上下文压缩](docs/user-guide/zh-CN/src/memory-and-context.md) |
| 代码索引 | [工作区代码索引与隐私边界](docs/user-guide/zh-CN/src/code-indexing.md) |
| LSP 代码智能 | [实时语言服务器与信任模型](docs/user-guide/zh-CN/src/lsp-code-intelligence.md) |
| OnePiece 原生 Agent | [不装 CLI 直接用的内置 API Agent](docs/user-guide/zh-CN/src/native-agent.md) |
| 定时任务 | [定时任务与用量统计](docs/user-guide/zh-CN/src/automation.md) |
| 远程工作区与 SSH 连接 | [SSH 工作区、保存的连接与 IM 接入](docs/user-guide/zh-CN/src/remote-and-im.md) |
| CLI 参数 | [按 CLI Agent 配置启动参数，含各 CLI 参数速查](docs/user-guide/zh-CN/src/tooling.md#cli-参数) |
| 扩展能力 | [本地扩展安装/启用/禁用](docs/user-guide/zh-CN/src/tooling.md#扩展能力) |
| 插件集成 | [内置产品集成与就绪检测](docs/user-guide/zh-CN/src/plugin-integration.md) |
| MCP 服务器 | [MCP server 配置与按 Agent 绑定](docs/user-guide/zh-CN/src/mcp.md) |
| Agent 配置 | [按 Agent 配置 provider、端点与模型](docs/user-guide/zh-CN/src/tooling.md#agent-配置) |
| 专家角色 | [角色字段、职责与评审策略](docs/user-guide/zh-CN/src/expert-roles.md) |
| Agent 权限策略 | [Agent 权限策略与审批模板配置](docs/user-guide/zh-CN/src/permissions.md) |
| 个性化 | [Custom Instructions 与跨会话记忆](docs/user-guide/zh-CN/src/personalization.md) |
| Skill 管理 | [Skill 安装与绑定](docs/user-guide/zh-CN/src/skill-management.md) |
| Prompt Hook | [钩子管理](docs/user-guide/zh-CN/src/prompt-hooks.md) |
| IM 能力 | [IM 连接器配置](docs/user-guide/zh-CN/src/remote-and-im.md#im-连接器) |
| 执行可观测性 | [执行追踪与日志采集策略](docs/user-guide/zh-CN/src/observability.md) |
| 使用统计 | [Token 用量统计](docs/user-guide/zh-CN/src/automation.md) |
| 关于 | [版本、更新检查、changelog、仓库链接](docs/user-guide/zh-CN/src/app-updates.md) |
| 故障排查 | [出错了先看这里、日志在哪](docs/user-guide/zh-CN/src/troubleshooting.md) |
| 反馈问题 | [选对入口、要填什么、提交前怎么脱敏](docs/user-guide/zh-CN/src/reporting-issues.md) |

### 开发者指南

| 主题 | 入口 |
| --- | --- |
| 仓库结构 | [仓库布局与模块归属、各限界上下文职责](docs/developer-guide/zh-CN/src/repository-orientation.md) |
| 运行时边界 | [前端服务边界、Web/mock 与 Tauri 适配器](docs/developer-guide/zh-CN/src/runtime-boundaries.md) |
| 限界上下文 | [21 个 native bounded context 各自拥有什么](docs/developer-guide/zh-CN/src/native-contexts.md) |
| Agent 生命周期与 provider 运行时 | [注册 Agent 编辑、稳定 provider 解析、能力声明](docs/developer-guide/zh-CN/src/agent-lifecycle.md) |
| 终端与 PTY 运行时 | [会话级 Agent Terminal、自动启动/附着、远程终端](docs/developer-guide/zh-CN/src/terminal-runtime.md) |
| 工具注册表与执行 | [固定原生工具目录、按 interface_format 翻译、多轮工具循环](docs/developer-guide/zh-CN/src/tool-registry.md) |
| 权限模型 | [统一决策点、显式 Deny 优先、审批代理、CLI flag 投影、Claude Code 钩子桥](docs/developer-guide/zh-CN/src/permission-model.md) |
| 上下文压缩 | [字符计数触发、摘要式压缩、保留近期轮次](docs/developer-guide/zh-CN/src/context-compaction.md) |
| 检索与向量搜索 | [主机级共享记忆池、workspace 代码索引、优雅降级](docs/developer-guide/zh-CN/src/retrieval.md) |
| Tree-sitter 代码索引 | [语法解析、bounded chunk、符号元数据、grammar 版本与脱敏](docs/developer-guide/zh-CN/src/tree-sitter-code-indexing.md) |
| 跨会话记忆 | [主机级共享池、provenance 元数据、OnePiece 工具与 CLI 自动提取](docs/developer-guide/zh-CN/src/cross-session-memory.md) |
| 会话恢复 | [恢复状态与生命周期正交、持久化执行身份与所有权](docs/developer-guide/zh-CN/src/session-recovery.md) |
| OnePiece 原生 Agent | [内置 API Agent 身份、Profile 生命周期与 provider 目录](docs/developer-guide/zh-CN/src/onepiece-native-agent.md) |
| 多 Agent 群聊 | [席位模型、中途增减、轮次路由与持久化 presence](docs/developer-guide/zh-CN/src/multi-agent-group-chat.md) |
| Skill 管理 | [双 scope、SKILL.md 契约、漂移与内建播种/对账](docs/developer-guide/zh-CN/src/skill-management.md) |
| MCP 工具与客户端 | [传输与配置模型、原生工具目录中的 MCP 工具](docs/developer-guide/zh-CN/src/mcp-tools.md) |
| IM 连接器 | [五种内建连接器、首版直发消息范围、入站路由](docs/developer-guide/zh-CN/src/im-connectors.md) |
| Loop 与 Plan 运行时 | [持久化 Loop 定义、拓扑感知串行子任务调度、Worker/Verifier 信任](docs/developer-guide/zh-CN/src/loop-and-plan-runtime.md) |
| Token 用量统计 | [上报 token 与估算字符分离、时间范围、per-Agent 拆分](docs/developer-guide/zh-CN/src/usage-statistics.md) |
| LSP 代码智能 | [会话内 LSP 集成实现](docs/developer-guide/zh-CN/src/lsp-code-intelligence.md) |
| 持久化与日志 | [SQLite 所有权与统一脱敏日志](docs/developer-guide/zh-CN/src/persistence-and-logging.md) |
| 测试与发布 | [测试、打包与发布流程](docs/developer-guide/zh-CN/src/testing-and-release.md) |
| OpenSpec 工作流 | [提案→设计→delta spec→任务→校验→归档的变更流程](docs/developer-guide/zh-CN/src/openspec-workflow.md) |
| Native API 参考 | [Rustdoc 生成的内部契约与所有权文档](docs/developer-guide/zh-CN/src/native-api-reference.md) |
| 架构决策 | [仓库结构与模块导览、限界上下文与调用关系](docs/developer-guide/zh-CN/src/repository-orientation.md) |

用户指南提供英文与简体中文两种语言。日文、繁体中文、韩文仅作为应用界面资源语言交付，不提供对应的用户指南。

<!-- /docs-locale-guides -->

<!-- docs-section:architecture -->

## 架构

```mermaid
flowchart LR
  UI[React UI] --> Service[Frontend service interfaces]
  Service --> Web[Web/mock adapters]
  Service --> Tauri[Tauri adapters]
  Tauri --> Commands[Rust commands]
  Commands --> Contexts[Native bounded contexts]
  Contexts --> SQLite[(SQLite)]
  Contexts --> CLI[Agent CLIs]
```

React 组件调用 `src/services/` 中的服务。Tauri 专属 `invoke()` 调用仅位于 frontend Tauri adapter；SQLite、CLI 进程、文件系统访问与桌面生命周期行为位于 Rust。

<!-- docs-section:quick-start -->

## 从源码运行

<!-- docs-fact:node-minimum value:22+ -->

前置要求：Node.js 22+、npm、stable Rust，以及当前平台的 [Tauri 前置依赖](https://v2.tauri.app/start/prerequisites/)。

平台 linker 要求、release profile 行为、worktree 缓存建议及构建测量结果参见[原生构建性能指南](docs/build-performance.md)。

```powershell
npm ci
```

运行 Web/mock 预览：

```powershell
npm run dev -- --host 127.0.0.1
```

运行桌面应用：

```powershell
$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"
npm run tauri -- dev
```

Web/mock 是确定性的浏览器模拟，不代表真实发生了本地 CLI 执行、SQLite 持久化、文件修改或操作系统 side effect。

<!-- docs-section:development -->

## 开发

提交变更前，请逐字运行 AGENTS.md「校验命令」一节中的全部命令；该清单是与 CI 对齐的唯一真源。

新功能和架构调整必须在实现前创建 OpenSpec proposal。项目规则见 [AGENTS.md](AGENTS.md) 与 [openspec/project.md](openspec/project.md)。

### Agent 基础设施技术文档

| 主题 | 入口 |
| --- | --- |
| MCP | [协议模型与三角色架构、传输层、核心原语、生命周期、授权与安全模型](docs/agent-infrastructure/mcp-architecture.md) |
| Function Calling | [调用循环与约束解码、Anthropic 与 OpenAI 的 API 差异、并行调用与流式组装、结构化输出](docs/agent-infrastructure/function-calling-architecture.md) |
| LSP | [协议分层与生命周期、能力协商、文本同步模型、语言与工作区特性](docs/agent-infrastructure/lsp-architecture.md) |
| A2A | [AgentCard/Task/Message/Artifact 数据模型、任务状态机、发现机制、异步更新通道](docs/agent-infrastructure/a2a-architecture.md) |
| 多 Agent 系统 | [编排拓扑与角色框架、通信与协调、上下文管理、执行隔离、失败模式与评估](docs/agent-infrastructure/multi-agent-architecture.md) |
| Agent Skills | [开放规范与文件格式、渐进式披露加载、触发与执行、与 MCP/Prompt 的定位对比](docs/agent-infrastructure/agent-skills-architecture.md) |
| AI 编码 CLI 参数完全参考 | [五种 CLI 的参数族逐一覆盖，以及宿主向各 CLI 投影参数的映射矩阵](docs/agent-infrastructure/builtin-cli-reference.md) |
| RAG | [索引与检索管线、语义与关键字检索取舍、混合检索与重排序、评估方法](docs/agent-infrastructure/rag-architecture.md) |
| Tree-sitter | [GLR 增量解析、语法工具链与 ABI、查询系统、结构化代码切分与 Repo Map](docs/agent-infrastructure/tree-sitter-architecture.md) |
| OpenSpec | [规范驱动开发的知识模型、变更包工件链、opsx 命令族、Delta 规格合并](docs/agent-infrastructure/openspec-architecture.md) |

参考：[Native 架构清单](src-tauri/ARCHITECTURE.md) · [贡献指南](CONTRIBUTING.md) · [原生构建性能](docs/build-performance.md) · [发布签名](docs/release-signing.md)

构建 mdBook 指南与 Rustdoc Reference：

```powershell
npm run docs:check
npm run docs:test
npm run docs:build
```

文档构建需要 `docs/toolchain.json` 中固定的 mdBook 版本。

<!-- docs-section:roadmap -->

## 路线图

已实现行为和当前 contract 记录在 [OpenSpec 主规范](openspec/specs/)中。近期产品方向包括自定义 Agent、插件市场和扩展的本地 OCR/语音能力。

<!-- docs-section:contributing -->

## 贡献

开始变更前请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。涉及行为变更时，应同步文档、两个 frontend runtime adapter、native contract、测试与 OpenSpec 工件。

<!-- docs-section:license -->

## License

本项目采用 Apache License 2.0，详见 [LICENSE](LICENSE)。
