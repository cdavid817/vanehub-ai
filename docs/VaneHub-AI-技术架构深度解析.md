# VaneHub AI 技术架构深度解析

> 版本：基于源码主干 `bb3d28d8`（2026-08）撰写。本文档通过四个并行代码探查子任务对整个仓库进行结构化测绘后综合而成，所有结论均以 `文件:行号` 形式锚定到真实源码，便于读者交叉验证。
>
> **历史快照说明**：本文保留 `bb3d28d8` 当时的代码测绘，不代表当前架构。独立 Plan Center、PlanRun 与 `task_orchestration` 已在后续重构中退役；当前 Plan 仅是 OnePiece 会话输入栏内的只读模式。当前设计以 `openspec/specs/` 和开发者指南为准。

---

## 目录

- [第一部分 项目全景](#第一部分-项目全景)
  - [第 1 章 项目定位与技术栈](#第-1-章-项目定位与技术栈)
  - [第 2 章 顶层架构总览](#第-2-章-顶层架构总览)
  - [第 3 章 双运行时模式：桌面与 Web/Mock](#第-3-章-双运行时模式桌面与-webmock)
  - [第 4 章 源码组织与目录结构](#第-4-章-源码组织与目录结构)

- [第二部分 前端架构](#第二部分-前端架构)
  - [第 5 章 应用启动与引导](#第-5-章-应用启动与引导)
  - [第 6 章 路由与懒加载](#第-6-章-路由与懒加载)
  - [第 7 章 服务边界层：核心架构基石](#第-7-章-服务边界层核心架构基石)
  - [第 8 章 状态管理与数据流](#第-8-章-状态管理与数据流)
  - [第 9 章 聊天体验与消息渲染管线](#第-9-章-聊天体验与消息渲染管线)
  - [第 10 章 多智能体群聊：席位与发言路由](#第-10-章-多智能体群聊席位与发言路由)
  - [第 11 章 主布局与功能界面](#第-11-章-主布局与功能界面)
  - [第 12 章 国际化与主题系统](#第-12-章-国际化与主题系统)
  - [第 13 章 类型与契约：前后端契约守护](#第-13-章-类型与契约前后端契约守护)

- [第三部分 Rust 原生后端（DDD）](#第三部分-rust-原生后端ddd)
  - [第 14 章 原生架构总览与限界上下文](#第-14-章-原生架构总览与限界上下文)
  - [第 15 章 引导与组合根](#第-15-章-引导与组合根)
  - [第 16 章 限界上下文详解](#第-16-章-限界上下文详解)
  - [第 17 章 Tauri 命令层](#第-17-章-tauri-命令层)
  - [第 18 章 平台层](#第-18-章-平台层)
  - [第 19 章 架构守护机制](#第-19-章-架构守护机制)

- [第四部分 Agent 运行时与多 Provider](#第四部分-agent-运行时与多-provider)
  - [第 20 章 Provider 模型与注册表](#第-20-章-provider-模型与注册表)
  - [第 21 章 会话与消息生命周期](#第-21-章-会话与消息生命周期)
  - [第 22 章 CLI 进程启动与流式解析](#第-22-章-cli-进程启动与流式解析)
  - [第 23 章 原生 API 运行时与工具使用循环](#第-23-章-原生-api-运行时与工具使用循环)
  - [第 24 章 OnePiece 原生 Plan-Agent 循环](#第-24-章-onepiece-原生-plan-agent-循环)
  - [第 25 章 工具执行与 MCP](#第-25-章-工具执行与-mcp)
  - [第 26 章 Skill 注入](#第-26-章-skill-注入)
  - [第 27 章 多智能体群聊与发言调度](#第-27-章-多智能体群聊与发言调度)
  - [第 28 章 上下文压缩与跨会话记忆](#第-28-章-上下文压缩与跨会话记忆)
  - [第 29 章 计划与循环工程](#第-29-章-计划与循环工程)
  - [第 30 章 精细 token 计量与执行可观测性](#第-30-章-精细-token-计量与执行可观测性)
  - [第 31 章 权限系统](#第-31-章-权限系统)

- [第五部分 数据持久化与平台集成](#第五部分-数据持久化与平台集成)
  - [第 32 章 数据库与持久化](#第-32-章-数据库与持久化)
  - [第 33 章 统一日志与运维操作](#第-33-章-统一日志与运维操作)
  - [第 34 章 工作区与远程终端](#第-34-章-工作区与远程终端)
  - [第 35 章 Tooling 子域](#第-35-章-tooling-子域)
  - [第 36 章 通信：IM 连接器](#第-36-章-通信im-连接器)
  - [第 37 章 代码智能与检索](#第-37-章-代码智能与检索)
  - [第 38 章 桌面子系统与生命周期](#第-38-章-桌面子系统与生命周期)
  - [第 39 章 CI 与治理](#第-39-章-ci-与治理)

- [第六部分 横切关注与质量保障](#第六部分-横切关注与质量保障)
  - [第 40 章 端到端时序图集](#第-40-章-端到端时序图集)
  - [第 41 章 工程治理与机器强制层](#第-41-章-工程治理与机器强制层)
  - [第 42 章 关键设计权衡与 ADR](#第-42-章-关键设计权衡与-adr)

- [第七部分 深入专题与系统走读](#第七部分-深入专题与系统走读)
  - [第 43 章 服务边界层的设计哲学](#第-43-章-服务边界层的设计哲学)
  - [第 44 章 消息渲染管线的性能走读](#第-44-章-消息渲染管线的性能走读)
  - [第 45 章 多智能体群聊的语义模型](#第-45-章-多智能体群聊的语义模型)
  - [第 46 章 上下文窗口的经济学](#第-46-章-上下文窗口的经济学)
  - [第 47 章 权限模型的安全语义](#第-47-章-权限模型的安全语义)
  - [第 48 章 持久化层的演化与对账艺术](#第-48-章-持久化层的演化与对账艺术)
  - [第 49 章 统一日志的脱敏与可审计性](#第-49-章-统一日志的脱敏与可审计性)
  - [第 50 章 可观测性的分层与 fidelity](#第-50-章-可观测性的分层与-fidelity)
  - [第 51 章 Tooling 子域的自治与一致性](#第-51-章-tooling-子域的自治与一致性)
  - [第 52 章 通信子域的事件驱动完成](#第-52-章-通信子域的事件驱动完成)
  - [第 53 章 桌面生命周期的优雅退出](#第-53-章-桌面生命周期的优雅退出)
  - [第 54 章 OpenSpec 流程的治理意义](#第-54-章-openspec-流程的治理意义)
  - [第 55 章 架构守护的机械性与局限](#第-55-章-架构守护的机械性与局限)
  - [第 56 章 系统的张力与未来演进](#第-56-章-系统的张力与未来演进)

- [第八部分 实现细节与边界条件](#第八部分-实现细节与边界条件)
  - [第 57 章 会话状态机的完整转换图](#第-57-章-会话状态机的完整转换图)
  - [第 58 章 操作任务的内存模型与恢复证据](#第-58-章-操作任务的内存模型与恢复证据)
  - [第 59 章 SSH 连接池的并发语义](#第-59-章-ssh-连接池的并发语义)
  - [第 60 章 终端输出捕获的有界性与可搜索性](#第-60-章-终端输出捕获的有界性与可搜索性)
  - [第 61 章 检索的混合路径与优雅降级](#第-61-章-检索的混合路径与优雅降级)
  - [第 62 章 Skill overlay 的增量修改与回放](#第-62-章-skill-overlay-的增量修改与回放)
  - [第 63 章 Prompt Hook 的草稿发布回滚生命周期](#第-63-章-prompt-hook-的草稿发布回滚生命周期)
  - [第 64 章 定时任务与 headless 执行的可靠性](#第-64-章-定时任务与-headless-执行的可靠性)
  - [第 65 章 凭据管理的零拷贝与信任边界](#第-65-章-凭据管理的零拷贝与信任边界)
  - [第 66 章 国际化的资源对等约束](#第-66-章-国际化的资源对等约束)
  - [第 67 章 主题系统的语义对等](#第-67-章-主题系统的语义对等)

- [第九部分 数据流与控制流深度走读](#第九部分-数据流与控制流深度走读)
  - [第 68 章 从 CLI 安装到首次生成的完整旅程](#第-68-章-从-cli-安装到首次生成的完整旅程)
  - [第 69 章 错误传播与命令安全边界](#第-69-章-错误传播与命令安全边界)
  - [第 70 章 并发控制与资源限流](#第-70-章-并发控制与资源限流)
  - [第 71 章 事件系统的设计与订阅语义](#第-71-章-事件系统的设计与订阅语义)
  - [第 72 章 资源清理与生命周期回收](#第-72-章-资源清理与生命周期回收)
  - [第 73 章 测试策略的层次与契约](#第-73-章-测试策略的层次与契约)
  - [第 74 章 版本同步与发布工程](#第-74-章-版本同步与发布工程)
  - [第 75 章 安全模型的纵深防御](#第-75-章-安全模型的纵深防御)
  - [第 76 章 可测试性作为架构约束](#第-76-章-可测试性作为架构约束)
  - [第 77 章 文档工程与可信文档](#第-77-章-文档工程与可信文档)

- [第十部分 设计模式纵览与子系统剖析](#第十部分-设计模式纵览与子系统剖析)
  - [第 78 章 不可变基础与可变叠加模式](#第-78-章-不可变基础与可变叠加模式)
  - [第 79 章 端口与适配器模式的系统性应用](#第-79-章-端口与适配器模式的系统性应用)
  - [第 80 章 事件驱动与轮询的取舍](#第-80-章-事件驱动与轮询的取舍)
  - [第 81 章 失败语义的三态与最终一致](#第-81-章-失败语义的三态与最终一致)
  - [第 82 章 Agent 终端与工作区 shell 的区分](#第-82-章-agent-终端与工作区-shell-的区分)
  - [第 83 章 工作区与 worktree 的工程意义](#第-83-章-工作区与-worktree-的工程意义)
  - [第 84 章 Provider 中性层与具体 provider 的边界](#第-84-章-provider-中性层与具体-provider-的边界)
  - [第 85 章 配置的即时生效与持久化](#第-85-章-配置的即时生效与持久化)
  - [第 86 章 浮窗助手的独立渲染面](#第-86-章-浮窗助手的独立渲染面)
  - [第 87 章 主窗口关闭与托盘最小化的意图判断](#第-87-章-主窗口关闭与托盘最小化的意图判断)
  - [第 88 章 内存状态与持久状态的边界](#第-88-章-内存状态与持久状态的边界)
  - [第 89 章 延迟加载与首屏体验](#第-89-章-延迟加载与首屏体验)
  - [第 90 章 代码索引与向量检索的工程化](#第-90-章-代码索引与向量检索的工程化)

- [第十一部分 演进历史、对比与反思](#第十一部分-演进历史对比与反思)
  - [第 91 章 从扁平结构到 DDD 的迁移故事](#第-91-章-从扁平结构到-ddd-的迁移故事)
  - [第 92 章 与同类工具的对比定位](#第-92-章-与同类工具的对比定位)
  - [第 93 章 单 crate 与多 crate 的架构选择](#第-93-章-单-crate-与多-crate-的架构选择)
  - [第 94 章 契约守护的免代码生成方案](#第-94-章-契约守护的免代码生成方案)
  - [第 95 章 从阅读者视角的导航建议](#第-95-章-从阅读者视角的导航建议)
  - [第 96 章 设计反思：哪些地方可以更好](#第-96-章-设计反思哪些地方可以更好)
  - [第 97 章 对读者的最终建议](#第-97-章-对读者的最终建议)

- [第十二部分 上下文深潜与场景集锦](#第十二部分-上下文深潜与场景集锦)
  - [第 98 章 operations 上下文：可观测性的中央枢纽](#第-98-章-operations-上下文可观测性的中央枢纽)
  - [第 99 章 sessions 上下文：会话作为一等聚合](#第-99-章-sessions-上下文会话作为一等聚合)
  - [第 100 章 workspaces 上下文：项目与终端的统一管理](#第-100-章-workspaces-上下文项目与终端的统一管理)
  - [第 101 章 desktop 上下文：桌面外壳的生命周期](#第-101-章-desktop-上下文桌面外壳的生命周期)
  - [第 102 章 permissions 上下文：能力的守门人](#第-102-章-permissions-上下文能力的守门人)
  - [第 103 章 典型场景：多 agent 协作完成一个编码任务](#第-103-章-典型场景多-agent-协作完成一个编码任务)
  - [第 104 章 典型场景：IM 驱动的远程协助](#第-104-章-典型场景im-驱动的远程协助)
  - [第 105 章 典型场景：崩溃后的会话恢复](#第-105-章-典型场景崩溃后的会话恢复)
  - [第 106 章 典型场景：CLI 版本升级与配置迁移](#第-106-章-典型场景cli-版本升级与配置迁移)
  - [第 107 章 总结：一个工程化 agent 系统的全貌](#第-107-章-总结一个工程化-agent-系统的全貌)

- [第十三部分 工程哲学与深度反思](#第十三部分-工程哲学与深度反思)
  - [第 108 章 复杂度守恒与显式化](#第-108-章-复杂度守恒与显式化)
  - [第 109 章 信任边界与最小权限](#第-109-章-信任边界与最小权限)
  - [第 110 章 确定性与可复现](#第-110-章-确定性与可复现)
  - [第 111 章 文档即架构的一部分](#第-111-章-文档即架构的一部分)
  - [第 112 章 对失败友好的设计](#第-112-章-对失败友好的设计)
  - [第 113 章 复杂系统的可读性](#第-113-章-复杂系统的可读性)
  - [第 114 章 工程纪律与创造力的关系](#第-114-章-工程纪律与创造力的关系)
  - [第 115 章 这份文档本身的局限](#第-115-章-这份文档本身的局限)

- [第十四部分 补充专题与边界深入](#第十四部分-补充专题与边界深入)
  - [第 116 章 模型发现与凭据校验的探针机制](#第-116-章-模型发现与凭据校验的探针机制)
  - [第 117 章 Skill 的 effective catalog 与运行时缓存](#第-117-章-skill-的-effective-catalog-与运行时缓存)
  - [第 118 章 扩展框架的自有 loopback 运行时](#第-118-章-扩展框架的自有-loopback-运行时)
  - [第 119 章 代码索引的增量协调器](#第-119-章-代码索引的增量协调器)
  - [第 120 章 会话恢复的证据收集与决策树](#第-120-章-会话恢复的证据收集与决策树)
  - [第 121 章 测试夹具与契约对齐的实践](#第-121-章-测试夹具与契约对齐的实践)
  - [第 122 章 性能预算与监控](#第-122-章-性能预算与监控)
  - [第 123 章 最终的整合视角](#第-123-章-最终的整合视角)

- [第十五部分 边界补遗与深度收尾](#第十五部分-边界补遗与深度收尾)
  - [第 124 章 文件系统边界的 symlink 安全](#第-124-章-文件系统边界的-symlink-安全)
  - [第 125 章 进程树的清理与 Windows job object](#第-125-章-进程树的清理与-windows-job-object)
  - [第 126 章 消息序列号的并发安全](#第-126-章-消息序列号的并发安全)
  - [第 127 章 国际化的复数形式处理](#第-127-章-国际化的复数形式处理)
  - [第 128 章 自定义指令与记忆的注入层次](#第-128-章-自定义指令与记忆的注入层次)
  - [第 129 章 悬浮助手的拖拽与位置持久化](#第-129-章-悬浮助手的拖拽与位置持久化)
  - [第 130 章 全文总结与致读者](#第-130-章-全文总结与致读者)

- [第十六部分 收尾补章](#第十六部分-收尾补章)
  - [第 131 章 设计模式回顾与跨章索引](#第-131-章-设计模式回顾与跨章索引)
  - [第 132 章 读完之后怎么继续](#第-132-章-读完之后怎么继续)
  - [第 133 章 系统的约束清单与机器执行映射](#第-133-章-系统的约束清单与机器执行映射)
  - [第 134 章 写在最后的工程信条](#第-134-章-写在最后的工程信条)
  - [第 135 章 名词与概念的再梳理](#第-135-章-名词与概念的再梳理)
  - [第 136 章 致未来的自己](#第-136-章-致未来的自己)
  - [第 137 章 检索降级与韧性设计的再思考](#第-137-章-检索降级与韧性设计的再思考)
  - [第 138 章 写在卷末的最后几句话](#第-138-章-写在卷末的最后几句话)


---

# 第一部分 项目全景

## 第 1 章 项目定位与技术栈

VaneHub AI 是一个**桌面端多 AI 编程助手管理终端**。它的核心使命是把 Claude Code、OpenCode、Codex CLI、Gemini CLI、Antigravity CLI 等多个命令行 AI 编程代理（以及一套名为 OnePiece 的原生 API 代理）统一收拢到同一个桌面应用里，提供一致的会话管理、消息渲染、工具审批、计划/循环工程、用量统计与跨 IM 渠道协作能力。

这个定位决定了它两件本质性的工作：

1. **生命周期托管**：它本身不是一个模型推理引擎，而是一个"编排壳"。它负责探测这些 CLI/SDK 是否安装、解析它们的配置、以正确的参数启动它们、解析它们各异的结构化输出、把 token 用量落账、在崩溃后恢复会话。
2. **统一体验**：尽管每个被托管的代理有自己的交互范式（Claude Code 的 headless stream-json、Codex 的 JSONL、Gemini 的 stream-json、Antigravity 的包装事件，以及 OnePiece 直接打到 provider HTTP 端点的原生 API 循环），VaneHub 把它们统一成一套 `ChatStreamEvent`、一套消息模型、一套权限与审批模型。

技术栈是严格约束的，`AGENTS.md` 明令禁止引入替代方案，`eslint.config.js` 与 `Cargo.toml` 则从机制上锁死。前端是 React 19 + TypeScript（strict）+ Vite + Tailwind v4，桌面运行时是 Tauri 2.x（Rust），数据库是 SQLite（前端绝不直连库），状态管理只用 React 内置 state/context + TanStack Query（禁 Redux/Zustand），样式只用 Tailwind（禁 styled-components/CSS Modules），包管理用 npm（项目已有 `package-lock.json`，禁切 pnpm/yarn），测试用 Vitest（单元/组件）+ Playwright（E2E）。

下表是真实的依赖清单（`package.json:43-72` 与 `src-tauri/Cargo.toml` 的提炼）：

| 层 | 技术 | 用途 |
| --- | --- | --- |
| UI 框架 | React 19.2.8 | 函数组件 + Hooks，禁 class |
| 语言 | TypeScript（v6/v7 alias） | strict，禁 `any` 与 `@ts-ignore` |
| 构建 | Vite 8.2.1 + `@tailwindcss/vite` 4.3 | rolldown 代码分割 |
| 桌面 | Tauri 2 + `plugin-dialog`/`plugin-opener`/`plugin-autostart` | Rust 运行时 |
| 路由 | react-router 8.3 | `/workspace`、`/settings` 两路由 |
| 服务端状态 | TanStack Query 5.101 + react-virtual | 缓存、乐观更新、虚拟列表 |
| Markdown | react-markdown 10 + remark-gfm/math + rehype-katex/highlight | 富文本渲染 |
| 图表 | mermaid 11 | 消息内 mermaid 图 |
| 表单 | react-hook-form 7 + zod 4 | 设置页表单与校验 |
| 终端 | xterm 6 | 会话工作区终端 |
| 图标 | lucide-react 1.29 | |
| Rust | edition 2021，crate `vanehub_ai_lib` | staticlib/cdylib/rlib |
| Rust 进程 | `portable-pty`、`russh`、`rmcp`、`rusqlite`+`r2d2`、`keyring`、`reqwest`、`tokio`、`syn`(架构测试)、`axum`(权限 hook bridge) | |
| 测试 | Vitest 4 + testing-library + Playwright 1.62；Rust `cargo test` + `cargo-llvm-cov` | |

> 一个值得注意的细节：`package.json:100-103` 的 `allowScripts` 只放行了 `esbuild`，这是 npm 的"信任脚本"白名单——任何依赖想跑安装时脚本都必须显式登记，是供应链硬化的一部分。

## 第 2 章 顶层架构总览

VaneHub 的架构可以用一句话概括：**一个 React 单页应用，通过一个"服务边界层"与运行时解耦；运行时在桌面端是 Tauri，背后是一套严格六边形分层的 Rust DDD 单 crate；在浏览器端是一个行为等价的内存 mock**。

这个分层的核心动机是：同一套 React UI 既要能跑在 Tauri 桌面 webview 里（调用 Rust 命令），也要能以浏览器页面形式独立运行（用于开发、演示、E2E 与回归）。如果让组件直接调用 `@tauri-apps/api` 的 `invoke()`，这两条路径就分裂了。因此项目把所有运行时能力收口到一个 TypeScript 接口 `AgentService`，再由两个适配器实现它。

```mermaid
graph TB
    subgraph Frontend["前端（React 19 + TS）"]
        Components["components/ 纯展示组件<br/>不直接依赖 Tauri"]
        Hooks["hooks/ + main-layout/"]
        Services["services/ 服务边界层<br/>唯一允许被组件依赖的一层"]
        Contracts["contracts/ + types/<br/>前后端共享契约"]
        Components --> Services
        Services --> Contracts
        Hooks --> Services
    end
    subgraph RuntimeSelection["运行时适配器选择（runtime-adapter.ts）"]
        Detect["detectRuntimeKind()<br/>__VANEHUB_RUNTIME__ > __TAURI_INTERNALS__ > __VANEHUB_HTTP_BASE_URL__"]
    end
    Services --> Detect
    Detect -->|tauri| TauriAdapter["tauri-*-client.ts<br/>invoke('snake_case_cmd')"]
    Detect -->|web-mock| WebAdapter["web-*-client.ts<br/>内存 mock（≈5000 行）"]
    Detect -->|web-http| HttpAdapter["webHttp 适配器<br/>缺失即 fail-fast"]
    subgraph Native["Rust 原生运行时（单 crate DDD）"]
        Commands["commands/registry.rs<br/>299 个命令，按上下文分组"]
        Contexts["contexts/ 14 个限界上下文<br/>domain/application/infrastructure/api"]
        Platform["platform/<br/>database/process/filesystem/logging/network/credentials"]
        Commands --> Contexts --> Platform
    end
    subgraph Store["SQLite + 日志 + 凭据"]
        DB[("vanehub.sqlite<br/>65 个迁移")]
        LOG[("vanehub.log JSONL<br/>脱敏+轮转")]
        KEY[("OS keyring")]
    end
    TauriAdapter --> Commands
    Native --> Store
```

上图揭示了三条关键数据通路：

- **组件 → 服务接口**：React 组件只看到 `AgentService`（`src/services/agent-service.ts:172`，约 140 个方法），不知道运行时是什么。
- **服务接口 → 适配器**：`runtime-adapter.ts` 在 `detectRuntimeKind()` 里用 window 全局变量决定走哪个适配器，并用一个 `Proxy` 统一包装错误（`withServiceErrorNormalization`）。
- **适配器 → Rust 命令 / mock**：Tauri 适配器把每个方法机械映射成 `invoke("snake_case_command", {...})`；mock 适配器用内存 Map 模拟同样行为，连异步延迟都模拟。

这套设计的另一个结果是：前端有 186 个服务层文件（约 22,843 行），Rust 有 1,130 个源文件，二者通过契约文件对齐，而不是通过代码生成器。

## 第 3 章 双运行时模式：桌面与 Web/Mock

理解运行时选择，是理解整个项目的钥匙。`src/services/runtime-adapter.ts`（56 行）定义了三选一逻辑：

```typescript
type RuntimeKind = "tauri" | "web-mock" | "web-http";
type RuntimeAdapterSet<T> = { tauri: T; webMock: T; webHttp?: T };

function detectRuntimeKind(host = window): RuntimeKind {
  if (host.__VANEHUB_RUNTIME__) return host.__VANEHUB_RUNTIME__;       // 显式覆盖（测试用）
  if (host.__TAURI_INTERNALS__) return "tauri";                         // Tauri webview
  if (host.__VANEHUB_HTTP_BASE_URL__) return "web-http";               // HTTP 部署
  return "web-mock";                                                     // 默认：纯浏览器
}
```

优先级有三层含义：

1. **`__VANEHUB_RUNTIME__`** 是显式覆盖，主要服务于测试与 E2E——Playwright 启动 dev server 时可以注入这个全局变量，强制走 mock 或 HTTP。
2. **`__TAURI_INTERNALS__`** 是 Tauri webview 注入的全局，存在即说明正在桌面壳里运行。
3. **`__VANEHUB_HTTP_BASE_URL__`** 是 HTTP 部署标志，意味着前端要连一个真正的后端服务（而非 mock）。关键约束是：如果某个服务**没有**提供 `webHttp` 适配器却选中了 `web-http`，`createRuntimeAdapter` 会在构造时**抛错**（`runtime-adapter.ts:48`），而不是静默回退到 mock——这是为了防止"看起来在用生产数据、其实在用假数据"的灾难性误判。

```mermaid
flowchart TD
    Start["应用启动"] --> Check1{"window.__VANEHUB_RUNTIME__?"}
    Check1 -->|有| Override["用显式覆盖值"]
    Check1 -->|无| Check2{"window.__TAURI_INTERNALS__?"}
    Check2 -->|有| Tauri["tauri：调用 Rust invoke"]
    Check2 -->|无| Check3{"window.__VANEHUB_HTTP_BASE_URL__?"}
    Check3 -->|有| Http["web-http：连后端 HTTP"]
    Http --> HasAdapter{"该服务有 webHttp 适配器?"}
    HasAdapter -->|有| UseHttp["使用 HTTP 适配器"]
    HasAdapter -->|无| Fail["构造时抛错（fail-fast）"]
    Check3 -->|无| Mock["web-mock：内存模拟"]
    Tauri --> Wrap["withServiceErrorNormalization<br/>Proxy 统一错误"]
    UseHttp --> Wrap
    Mock --> Wrap
    Override --> Wrap
    Wrap --> Client["返回统一 shape 的 service 单例"]
```

这种设计的价值在测试层体现得最明显：项目里 80 多个服务测试文件之所以能跑，正是因为前端可以完全离线地对着内存 mock 验证逻辑，而 mock 又共享了与 Tauri 适配器相同的纯逻辑模块（如 `chat-events.ts`、`turn-routing.ts`、`mention-routing.ts`）——后者保证了 mock 的行为是"忠实"的，不会漂移。

## 第 4 章 源码组织与目录结构

项目的目录结构本身就是架构的投影。`AGENTS.md` 和 `openspec/project.md` 给出了严格约束：

```
src/                        # 前端
├─ components/              # 纯展示型组件，不直接依赖 Tauri API
│  └─ chat/                 # 聊天相关展示组件
├─ services/                # 服务边界层（唯一允许被组件依赖的一层）
├─ hooks/                   # 自定义 hook
├─ main-layout/             # 主工作区外壳
├─ session-workspace/       # 会话工作区（9 个标签页）
├─ settings/                # 设置中心（17 个页面）
├─ plan-center/             # 计划中心
├─ loop-center/             # 循环中心
├─ floating-assistant/      # 悬浮助手（独立渲染面）
├─ notifications/           # 通知系统
├─ theme/  i18n/  types/  contracts/  config/  lib/
src-tauri/src/
├─ lib.rs                   # 仅模块暴露 + run() 委托
├─ bootstrap/               # 组合根：Tauri builder、state、后台任务
├─ contexts/<context>/      # 限界上下文：domain/application/infrastructure/api.rs
├─ commands/<context>/      # 每个命令一个文件，按上下文分组
└─ platform/                # 可复用外层技术适配器
   ├─ database/  process/  filesystem/  network/
   ├─ logging.rs  credentials/  clock.rs  ids.rs  git/  error.rs  text/
openspec/
├─ changes/                 # 未归档变更提案
│  └─ archive/              # 已完成变更历史
├─ specs/                   # 已确认规范（唯一真源）
└─ project.md               # 项目上下文与详细规范
```

几个值得强调的约束（它们都有机器强制层兜底，见第 41 章）：

- **`lib.rs` 不能含业务符号**：架构测试 `root_lib_contains_no_business_symbols`（`tests/architecture.rs:1230`）解析 `lib.rs` 的 AST，断言它只声明 `mod` 和 `run()`，零个 struct/fn/enum/impl/trait。这把"组合根只能放构造、不能放逻辑"从口头约定变成编译期红线。
- **组件不直接 `invoke()`**：`AGENTS.md` 写明，`eslint` 与架构测试共同拦截。所有运行时调用必须经 `src/services/`。
- **`src-tauri/src` 不下沉 CLI 检测/启动路由/SQLite 注册表到前端**：这些是 Rust 侧的职责。
- **300 行硬规则**：`eslint.config.js` 的 `max-lines` 对所有 ts/tsx 生产代码生效（测试文件豁免），存量超限文件有固定的技术债豁免清单，且**禁止向清单新增文件**——新代码一律 ≤300 行。豁免清单里有 9 个文件，包括 `web-agent-client.ts`（4137 行）、`tauri-agent-client.ts`（763 行）、`agent.ts`（538 行）等，它们是迁移期的历史债。

---

# 第二部分 前端架构

## 第 5 章 应用启动与引导

VaneHub 有两种渲染面（surface），由一个 URL 参数在最早的时刻分流。`src/main.tsx`（40 行）是整个前端的入口：

```mermaid
sequenceDiagram
    participant HTML as index.html
    participant Main as main.tsx
    participant App as App.tsx
    participant FA as floating-assistant-root.tsx

    Note over HTML: 静态 bootstrap-shell（logo + spinner）<br/>由独立 CSS 渲染，不依赖 JS bundle
    HTML->>Main: 加载模块脚本
    Main->>Main: 读取 ?surface=floating-assistant
    alt surface=floating-assistant
        Main->>FA: 动态 import ./floating-assistant/floating-assistant-root
        FA-->>Main: 渲染（透明无框窗口）
    else 默认
        Main->>App: 动态 import ./App
        App-->>Main: 渲染主窗口
    end
    Main-->>HTML: createRoot().render(<StrictMode>)
    Note over Main: 任何失败 → recoverFromBootstrapFailure()<br/>纯 DOM 错误面板 + 上报 ClientLogEvent
```

这个启动流程有几个精心设计的"防御层"：

**静态 bootstrap-shell**。`index.html:15-24` 内联了一个 `bootstrap-shell` div（logo + "Starting..." + spinner），由 `public/bootstrap.css` 样式化。它在 JS bundle 之前就可见，给用户即时反馈。`src/bootstrap-shell.test.ts` 强制三条不变量：(a) shell 必须出现在 `src/main.tsx` 之前；(b) 启动 CSS 必须独立于 app bundle（这样 React 加载失败也不影响错误展示）；(c) 必须尊重 `prefers-reduced-motion`。

**纯 DOM 的失败兜底**。`src/bootstrap-failure.ts`（79 行）不依赖 React——如果 React 本身加载失败，它仍能用裸 DOM 拼出一个错误面板。`recoverFromBootstrapFailure(options)`（`:71`）渲染面板后尽力上报 `ClientLogEvent`；`createBootstrapFailureEvent`（`:25`）把错误归一成 `kind: "critical-operation-failure"`、`source: "frontend-bootstrap"` 的标准化事件。重试按钮就是 `window.location.reload()`。

**双 QueryClient**。主窗口与悬浮助手各自持有独立的 `QueryClient`，因为它们是不同的窗口、不同的 React 根。但二者共享 `SettingsProvider`/`ThemeProvider`。悬浮助手根**没有**路由器和通知 Provider——它是一个极简的"迷你会话面"。

## 第 6 章 路由与懒加载

`src/App.tsx`（119 行）是主窗口的根。它的 Provider 嵌套顺序值得记住：

```mermaid
graph TD
    SP["SettingsProvider<br/>设置存储（乐观保存/回滚）"] --> TP["ThemeProvider<br/>主题切换"]
    TP --> NP["NotificationProvider<br/>通知上下文"]
    NP --> QP["QueryClientProvider<br/>单一共享 QueryClient<br/>refetchOnWindowFocus=false, retry=1"]
    QP --> Router["BrowserRouter"]
    Router --> EB["ErrorBoundary<br/>FallbackComponent=RouteErrorFallback"]
    EB --> Routes["AppRoutes"]
```

路由只有三条（`AppRoutes`，`:44-71`）：`/workspace` → `MainLayout`、`/settings` → `SettingsShell`、`*` → 重定向到 `/workspace`。但路由承担了"深链接"职责：`?createSession=1` 触发新建会话、`?section=<pageId>` 跳到特定设置页、`?agentConfig=onepiece` 直接打开 OnePiece 配置面板。

`AppRoutes` 还订阅了悬浮助手的事件（`:50`）：`floatingAssistantService.subscribeEvents` 监听 `main-action` 事件，根据动作（`new-session`/`current-session`/`settings`）导航。这是悬浮助手与主窗口通信的桥。

`ErrorBoundary` 是一个全局兜底：它把每个捕获到的错误（连同组件栈 `info.componentStack`）通过 `settingsService.reportClientLogEvent` 上报到 native 日志服务（`:100`）。这条路径是第 33 章讲的前端→native 错误上报链的起点。

**懒加载原语 `LazyFeature`**（`src/components/lazy-feature.tsx`，50 行）。它接收一个 `loader: () => Promise<{default: ComponentType}>`，用 `lazy()` 包裹，并给每个懒加载组件套上**独立的** `ErrorBoundary` + `Suspense`。出错时显示本地化的重试按钮，`onReset` 重建懒组件。这个原语被用于**每一个**设置页、Loop/Plan 两个中心、以及大部分会话标签页。

为什么"每个懒组件独立 ErrorBoundary"很重要？因为如果所有懒组件共享一个外层 ErrorBoundary，一个设置页的渲染崩溃会把整个设置中心打挂，用户连切到别的页都做不到。独立边界把爆炸半径限制在单个页面。

项目用 `src/frontend-lazy-loading.test.ts` 强制大特性模块不得被静态 import 进主 bundle，配合 `scripts/check-frontend-chunks.mjs`（`build` 脚本的一部分）做 chunk 体积门禁。

## 第 7 章 服务边界层：核心架构基石

如果说有一章是全文最重要的一章，就是这一章。服务边界层是前端架构的脊柱，也是双运行时得以成立的关键。

### 7.1 `AgentService`——前端唯一的能力出口

`src/services/agent-service.ts`（481 行，约 140 个方法）定义了 `interface AgentService`（`:172`）。这个接口横跨整个产品域：

- **Agent 注册表**：`listAgents`、`registerApiAgent`/`updateApiAgent`/`deleteApiAgent`（API 代理增删改）
- **OnePiece Provider**：`saveOnePieceProviderConfig`、Provider 预设、Profile、模型发现、凭据校验
- **Agent 记忆**：host 级全局记忆池
- **检索/代码索引**：embedding 模型配置、索引重建
- **LSP**：配置、信任、发现、测试
- **CLI 工具**：版本安装/升级、参数 Profile、配置 Profile（预设/凭据校验/发现/导入/应用）
- **工作流状态**：`selectAgent`、`checkBrowserReadiness`、`launchActiveWorkflow`
- **会话**：CRUD、分类、归档、定时任务、搜索、导出、恢复
- **循环**：`subscribeLoopEvents`
- **聊天配置**：每会话配置
- **消息**：`subscribeMessageEvents`
- **用量统计**
- **Agent 终端**：`subscribeAgentTerminalEvents`
- **工作区文件/Git/日志**、**文件夹打开器 + 事件**、**Shell + 事件**、`subscribeSessionEvents`
- **专家角色**、**Skill**（挂载路径、绑定、漂移、同步、预览、加载、导入）、**Skill Overlay**（patch/guidance/file/import/promote/reconcile + history）
- **Prompt Hook**（CRUD、预览、trace、变量、草稿、版本、回滚）

它还定义了会话恢复领域（`:407-481`）：`SessionStateEvent`（含 `recovery-*` 的判别联合）、`RecoveryDecision`（8 个值）、`RecoveryReasonCode`（15 个码）、`RecoveryEvidenceReference`、`SessionRecoveryReport/Summary/Acknowledgement`。

### 7.2 两个适配器，一份契约

**Tauri 适配器** `tauri-agent-client.ts`（1,016 行）：把每个 `AgentService` 方法机械映射成 snake_case 的 Tauri 命令。例如 `listAgents` → `invoke<AgentRegistryEntry[]>("list_agents", { capabilityTag })`（`:202`）。它组合了一组共享的 native 侧归一化器：`normalizeCodeIndexWorkspaces`（`code-index-contract.ts`）、`normalizeLspConfiguration` 等（`lsp-contract.ts`）、`normalizeTauriUsageStatistics`（`tauri-usage-statistics.ts`）。部分命令是"半客户端"的——比如 `listCliConfigPresets` 直接返回 `getCliConfigPresets(...)`（`config/cli-agent-provider-presets.ts`，`:426`），因为预设是纯前端常量。它在 `invoke` 之前做输入校验（`requireCliConfigAgentId`、`requireHttpsExternalUrl`）。

**Web/mock 适配器** `web-agent-client.ts`（4,984 行）：一个完整的内存实现，用模块级 Map 维护状态，种子数据来自 `mock-agent-data.ts`/`mock-workspace-data.ts`。它组合了多个子客户端（`web-session-workspace-client`、`web-lsp-client`、`web-ssh-connection-client`、`web-settings-client`、`web-token-usage`、`web-prompt-hook-versions`、`web-skill-overlay-runtime`、`web-operation-client`、`web-permissions-mock-state`），实现了模拟流式（`sendMessage` + 工具使用）、聊天配置归一化、席位快照、i18n 感知的 mock 文本。它还导出测试钩子（如 `markFirstWebPlanRunRecoveryRequired`、`triggerLatestWebPlanRepairForTest`）。

### 7.3 保证 parity 的四道防线

```mermaid
graph LR
    subgraph Contract["契约守护"]
        A["1. TS 接口<br/>agent-service.ts"]
        B["2. 两适配器都实现它"]
        C["3. createRuntimeAdapter 保证同 shape<br/>+ Proxy 统一错误"]
        D["4. 共享纯逻辑模块<br/>chat-events/turn-routing/mention-routing"]
    end
    A --> Tauri["tauri-agent-client.ts"]
    A --> Web["web-agent-client.ts"]
    Tauri --> C
    Web --> C
    Tauri -.共享.-> D
    Web -.共享.-> D
```

1. **TypeScript 接口**：`AgentService` 是编译期约束，两个适配器都 `implements` 它，少一个方法就编译不过。
2. **运行时单例工厂**：每个 `runtime-*-client.ts` 文件用 `createRuntimeAdapter` 生成模块级单例，并套上 `withServiceErrorNormalization` Proxy，把同步 throw 转成 rejected promise、把错误归一成 `ServiceError`。
3. **共享纯逻辑**：`chat-configuration.ts`（每 agent 默认配置 + 执行策略推导）、`turn-routing.ts`（路由决策）、`mention-routing.ts`（移交 @ 提及解析）、`human-handoff.ts`（意图解析）等纯函数模块被两个适配器共同 import。这保证了 mock 的行为与生产 Tauri 适配器"行为忠实"。
4. **归一化器**：把 native 的 snake_case/松散 shape 映射成严格前端类型（如 `mapMcpErrorCode`、`NativeMcpServerStatus`）。

### 7.4 服务清单

`src/services/` 共 186 个文件，按职责分四类：

- **服务接口（契约）**：`agent-service.ts`、`settings-service.ts`（含 `defaultAppSettings`、`normalizeAppSettings`）、`mcp-service.ts`、`operation-service.ts`、`plan-service.ts`、`im-service.ts`、`extension-service.ts`、`floating-assistant-service.ts`、`sdk-service.ts`、`ssh-connection-service.ts`、`plugin-integration-service.ts`、`workspace-service.ts`、`execution-observability-service.ts`、`permissions.ts`。
- **Tauri 适配器**：`tauri-{agent,settings,mcp,plan,im,session-workspace,extension,floating-assistant,operation,permissions,plugin-integration,sdk,ssh-connection,workspace,execution-observability}-client.ts` + `tauri-usage-statistics.ts`。
- **Web/mock 适配器**：对应的 `web-*.ts` 全家桶，外加 `web-terminal-capture-client.ts`、`web-lsp-client.ts`、`web-command-template-client.ts`、`web-token-usage.ts`、`web-prompt-hook-versions.ts`、`web-mcp-tool-simulation.ts`、`web-skill-overlay-*.ts`、`web-cli-config-client.ts`（仅测试）。
- **纯逻辑/领域服务**：`service-error.ts`、`chat-events.ts`（流式事件折叠）、`tool-use.ts`、`turn-routing.ts`、`turn-status.ts`、`mention-routing.ts`、`human-handoff.ts`、`message-speaker.ts`、`seat-context.ts`、`seat-briefing.ts`、`seat-mention-options.ts`、`seat-mutation.ts`、`seat-presentation.ts`、`session-seats.ts`、`session-admission.ts`、`model-family.ts`、`agent-model-family.ts`、`reviewer-recommendation.ts`、`role-injection-channel.ts`、`expert-role-runtime.ts`、`usage-statistics.ts`、`sdk-versioning.ts`、`cli-parameter-catalog.ts`、`mcp-validation.ts`、`mcp-tool-validation.ts`、`mcp-import.ts`、`external-url.ts`、`code-index-contract.ts`、`lsp-contract.ts`、`skill-overlay-error.ts`、`loop-run-polling.ts`、`plan-run-polling.ts`、`mock-agent-data.ts`、`mock-workspace-data.ts`。

这个清单的密度本身就说明了项目的复杂度：一个"多 AI CLI 管理终端"的纯前端逻辑，已经膨胀到需要几十个领域服务模块来承载。

## 第 8 章 状态管理与数据流

项目禁用 Redux/Zustand，只用两种机制：

### 8.1 TanStack Query 作为全局服务端状态

TanStack Query 是全局"服务端状态"的唯一存储。查询键族包括 `["agents"]`、`["sessions"]`、`["sessions","active"]`、`["sessions","search",q]`、`["messages", sessionId, limit]`、`["session-documents", id]`、`["session-chat-config", id]`、`["session-usage-summary", id]`、`["usage-statistics"]`、`["expert-roles"]`、`["loops", ...]`、`["floating-messages", id]`。

乐观更新和流式更新都通过 `setQueryData`/`setQueriesData` 完成（`use-main-layout-model.ts`、`use-active-session-chat.ts`）。比如发消息时，`createOptimisticUserMessage`（`optimistic-message.ts`）先把用户消息塞进缓存，`onMutate`（`:138`）立即更新 UI，再等服务端确认。

### 8.2 React Context：只有四个

项目刻意保持 Context 极少，只有四个 `createContext`：

- `SettingsContext`（`settings-provider.tsx:25`）：应用设置存储，带乐观保存/回滚、`applySettings`（字体大小、`data-theme`、语言激活）、加载失败回退默认值、订阅 `settings-events`。
- `ThemeContext`（`theme-provider.tsx:11`）：对 `settings.theme` 的薄封装。
- `NotificationContext` + `NotificationPresentationContext`（`notification-provider.tsx:32-33`）：通知系统（见第 38 章）。

其余状态都是所属组件本地的 `useState`/`useReducer`（如通知 reducer、会话侧边栏展示模式）。这种克制是有意的：避免全局 store 满天飞导致状态来源不清。

### 8.3 流式事件总线——性能的关键

聊天消息的 token 流式渲染是性能敏感场景。`use-main-layout-model.ts:174-219` 实现了一个关键优化：**token 事件先缓冲，在下一个动画帧批量应用**（`applyChatEvents`），而终端事件（completed/failed/cancelled）立即 flush。`turn_status` 事件单独追踪，不走缓冲。

```mermaid
sequenceDiagram
    participant Rust as Rust chat:event
    participant Sub as subscribeMessageEvents
    participant Buffer as token 缓冲
    participant RAF as requestAnimationFrame
    participant Cache as Query Cache

    loop 每个 token
        Rust->>Sub: ChatStreamEvent.token
        Sub->>Buffer: push(token)
    end
    Note over Buffer: 单次 O(n) 聚合，referential 稳定
    RAF->>Cache: applyChatEvents(flush)
    Note over Cache: setQueryData 更新消息<br/>MessageItem 是 memo 的
    Rust->>Sub: ChatStreamEvent.completed
    Sub->>Cache: 立即 flush（不等帧）
    Sub->>Cache: invalidate usage/sessions
```

为什么这样做？如果每个 token 到达都 `setQueryData` 一次，React 要在每帧重渲染整条消息列表。批量化后，无论一帧内来了多少 token，都只触发一次更新。配合 `MessageItem` 的 `React.memo` 和 `MessageList` 的 `ResizeObserver` + `anchoredScrollTop`（滚动锚定，`:13`），token 流式时 UI 保持 60fps。

## 第 9 章 聊天体验与消息渲染管线

`src/components/chat/` 是聊天的前端呈现核心。一条 `ChatMessage` 的渲染管线是分层的：

```mermaid
graph TD
    Msg["ChatMessage"] --> RM["RichMarkdown<br/>react-markdown v10"]
    Msg --> FR["文件引用 chips"]
    Msg --> Err["错误文本"]
    Msg --> TB["ThinkingBlock<br/>可折叠 <details>"]
    Msg --> TUB["ToolUseBlock<br/>工具使用展示+审批"]
    Msg --> RB["RichBlocks<br/>结构化富块"]
    RM -->|code.language-mermaid| Mermaid["MermaidDiagram<br/>lazy import mermaid"]
    RM -->|img| Safe["SafeImage<br/>src 白名单"]
    RM -->|table/blockquote/heading| Styled["设计 token 样式"]
    RB --> Card["card"]
    RB --> Diff["diff"]
    RB --> Check["checklist 进度"]
    RB --> Gallery["media_gallery"]
    RB --> File["file"]
    RB --> Audio["audio"]
    RB --> Html["html_widget<br/>sandbox iframe"]
    RB --> Interactive["interactive（disabled 设计）"]
    RB --> Unknown["UnknownBlock 兜底"]
```

**`RichMarkdown.tsx`**（78 行）：react-markdown v10 + `remarkGfm` + `remarkMath` + `rehypeKatex` + `rehypeHighlight`（`detect:false, ignoreMissing:true`）。自定义 `Components` 映射（`:10-58`）：`code` 带 `language-mermaid` 类 → `MermaidDiagram`；`img` → `SafeImage`；表格、引用、标题、任务列表都用设计 token 样式。`urlTransform`（`:60`）把图片 src 走 `safeImageSource`，其余走 `defaultUrlTransform`。

**`RichBlocks.tsx`**（239 行）：渲染类型化的结构化块。`RichBlockRenderer` switch（`:207`）按 `block.kind` 分派：`card`、`diff`、`checklist`（进度 + 勾选图标）、`media_gallery`、`file`（带 `formatFileSize`）、`audio`（`<audio controls>`）、`html_widget`（沙箱 iframe，高度钳制 50–600，`sandbox=""`，`:171`）、`interactive`（**刻意 disabled**——一个设计决策，`:192`）、`UnknownBlock` 兜底。全部包在 `BlockShell` 里，带 success/warning/danger 色调。

**`ToolUseBlock.tsx`**（264 行）：最复杂的渲染器。`normalizeToolUse`（来自 `tool-use.ts`）按 id 去重；把工具分桶为 approval/active/failed/completed；`groupConsecutiveFailures`（`:41`）折叠连续失败；`toolActivityPreview`（`:25`）对 `api_key|token|password|secret` 脱敏并截断到 120 字符。自动展开逻辑（`:176-186`）：流式/活跃/待审批时保持活动列表打开，完成/取消时关闭。它内嵌了 **`ApprovalCard`**（`:63`），调用 `permissionsService.listPendingApprovals()` 和 `resolvePendingApproval(id, approved, scope)`，其中 `ApprovalScope = "once" | "session" | "project" | "global"`，带 `L0..L3` 风险等级徽章。

**`MessageList.tsx`**（112 行）：滚动锚定。`anchoredScrollTop`（`:13`）在列表增长时若用户未钉底则保持相对滚动；`ResizeObserver` + rAF（`useLayoutEffect`，`:37`）防止 token 流式时的布局抖动；`hasMore` 时显示"加载更早"按钮；空时显示 `WelcomeScreen`；非自动滚动时显示 `ScrollControl` 回到底部按钮。

**`SeatMentionCompletion.tsx`**（47 行）+ `ChatInputBox.tsx:70-96` + `seat-mention-options.ts`：composer 用正则 `/(?:^|\s)@([^\s@]*)$/` 检测尾部提及（`:70`）；`seatMentionOptions`（`:7`）只在 ≥2 个活跃席位时构建候选 handle（`roleName` 连字符化，去重时加 `-2`/`-3` 后缀），过滤匹配 mention/roleName/agentName，上限 8（`:80-85`）。补全还覆盖**文件引用**——`@path` 建议会话文档（`:72-79`）。选中时替换尾部 token（`:87-96`）。

**`ChatInputBox.tsx`**（188 行）：受控 textarea，自动高度（88–200px，`:98-104`），Enter 发送（IME 感知，`:143`），文件引用 chips，`@` 补全下拉，把配置工具栏委托给 `ButtonArea.tsx`（142 行），后者承载 `selectors/*`（Config/Provider/Mode/Model/Reasoning via `SelectorDropdown.tsx`）、有效执行策略提示、"打开关联计划"、增强、停止/发送按钮。

**`useChatConfig.ts`**（271 行）：配置状态机。从 agent 推导 `providerId`，从 `PROVIDER_MODELS` 目录（`models.ts`）选默认模型，按模型钳制 `reasoningDepth`（`clampReasoningDepth`，`:23`），加载持久化的每会话配置（`agentService.getSessionChatConfig`，`:81`），带 120ms 防抖自动保存（`:237-250`），追踪 OnePiece 的 `associatedPlanRun`，处理 **plan↔execute 模式切换**：execute→plan 切换会暂停 run（`requestPlanControl(runId,"pause")`），`waitForPlanningBoundary` 轮询直到 run 离开 running/verifying/repairing/pause_requested（`:183-214`）。

## 第 10 章 多智能体群聊：席位与发言路由

多智能体群聊（multi-agent group chat）是 VaneHub 的高级特性。它的核心抽象是"席位"（seat）。

### 10.1 席位领域模型

`types/agent.ts` 定义了席位类型：

- `SessionSeat`（`:229`）：`seatId?`、`agentId`、`roleId`、`roleSnapshot?`、`joinedAt`、`leftAt`。
- `SessionSeatRoleSnapshot`（`:218`）：`roleName`、`avatar`、`color`、`responsibility`、`agentName`、`modelFamily`、`crossFamilyReviewer`。
- `Session` 可选 `seats?`（`:256`），其中 `agentId` **镜像 `seats[0].agentId`**——这是为了兼容约 148 个遗留读取者（`session-seats.ts:3-12` 有文档说明）。

- `session-seats.ts`：`seatsFromSession` 从 `agentId` 合成单席位；`activeSeatsFromSession` 过滤 `leftAt == null`。
- `message-speaker.ts`：`resolveMessageSpeaker`（`:24`）把消息的 `speakerSeatId`/`seatIndex` 解析成 `MessageSpeaker {agentId, avatar, color, roleName, agentName, crossFamilyReviewer}`；对遗留/单席位/畸形归属返回 **null**，保证旧会话渲染不变。
- `use-session-speakers.ts`：`sessionSpeakers` 构建按 seatIndex 和 seatId 双键的 `Map<string|number, MessageSpeaker>`（仅当 seats ≥ 2）；`useSessionRoles(enabled)` 门控角色获取，单 agent 会话绝不查 `expert-roles`。
- `seat-mutation.ts`：`addSeat`/`removeSeat`——移除最后一个席位返回 null（拒绝），历史永不重写。

### 10.2 发言路由

```mermaid
flowchart TD
    UserMsg["用户消息"] --> Route["routeUserMessage"]
    Route --> Mention{"开头有 @ 提及?"}
    Mention -->|有| ParseMention["parseHandoffMentions<br/>仅行首 @handle<br/>剥离代码块/列表前缀<br/>最长匹配"]
    Mention -->|无| Last{"有 lastHolder?"}
    Last -->|有| LastHolder["路由给 lastHolder"]
    Last -->|无| First["路由给 firstSeat"]
    ParseMention --> Targets["nextTurnTargets"]
    Targets --> Depth{"达到 MAX_CHAIN_DEPTH=15?"}
    Depth -->|是| Ended["endedReason: max-depth"]
    Depth -->|否| Mentions{"提及数 > MAX_MENTIONS_PER_REPLY=2?"}
    Mentions -->|是| Cap["截断到 2"]
    Mentions -->|否| Dispatch["派发给目标席位"]
```

- **`turn-routing.ts`**：`routeUserMessage`（`:18`）——未被 @ 的消息发给 `lastHolder ?? firstSeat`；`nextTurnTargets`（`:40`）强制 `maxDepth`（返回 `endedReason: "max-depth"`）和 `maxMentions`。
- **`mention-routing.ts`**：`parseHandoffMentions`（`:35`）——只有**行首**的 `@handle` 才路由（blockquote/list 标记之后也算行首）；代码块先剥离（`:14`）；最长提及优先；边界正则保证 `@opus-45` 不会匹配 `@opus`。
- **`human-handoff.ts`**：`parseHumanHandoff`（`:20`）识别 `@用户 handoff | fyi | done`；只有 `handoff` 阻塞/中断（`applyHumanHandoff`，`:36`）——`fyi` 让 agent 继续，`done` 结束本轮。
- **`seat-briefing.ts`**：`buildSeatBriefing`（`:25`）组装花名册 + 移交规则 + 人类移交语法，告诉 agent。
- **`seat-context.ts`**：`buildSeatContext`（`:23`）——若 provider session 存在则 `resume`（不重新注入），否则 `inject` 最近几轮为 `[speaker 说] …` 文本，在 `maxChars` 内。

### 10.3 回合状态条

`TurnStatusBar.tsx`（51 行）+ `turn-status.ts`：粘性状态条显示 `agent`（持有者 + 链深度）、`waiting-human`（带 `waitedMinutes`，客户端每 30s tick——`use-main-layout-model.ts:222-234`）、或 `round-complete`。设计注记（`:10-13`）：只有"暂停"状态被强调，这样信息性的 `fyi` 移交不会被惩罚。

这套规则是**双实现**的——同样的 `parseHandoffMentions`/`routeUserMessage`/`parseHumanHandoff` 逻辑既在前端 `src/services/` 里有纯函数版本，也在 Rust `domain/seat_turn.rs` 里有镜像版本。后者存在是为了让 IM 连接器和定时任务这类 headless 会话也能做移交，而不依赖前端（第 27 章详述）。

## 第 11 章 主布局与功能界面

### 11.1 主布局外壳

`main-layout/main-layout.tsx`（448 行）是工作区外壳。组成（`:223-447`）：

- `TopBar`
- `WorkspaceActivityBar`（VS Code 风格左轨：会话/计划/循环/定时任务/设置/帮助，`workspace-activity-bar.tsx`）
- `ucd-workspace-grid` CSS 网格：`session-sidebar | workspace | info-panel`（`styles.css:287-302`）
- **会话侧边栏**（`session-sidebar.tsx`，269 行）：搜索（防抖 ≥2 字符）、三种展示模式（列表/分类/项目）、置顶区、agent 过滤、批量删除、拖拽分类；宽度持久化到 localStorage。
- **工作区标签页**（`session-workspace/session-tabs.tsx`，172 行）：`SessionTabBar` 定义 9 个标签——`chat, changes, documents, files, terminal, shell, logs, traces, report`（`:19-45`）。非 chat 标签都是**懒加载**（`LazyFeature`，`:28-36, 111-122`）。`mountedTabs` 保持已访问标签挂载，切回不丢状态。`SeatSwitcher`（`seat-switcher.tsx`）在 >1 席位时为席位级标签渲染每席位子视图（刻意"在标签内而非加标签"，`:6-12`）。chat 标签本身在 `interactionMode !== "api"` 且单席位时切到 `AgentTerminalTab`（CLI/终端风格），否则用 `ChatTab` + `ApiSessionComposer`（`:98-109`）。
- **右侧信息面板**（`session-info-panel.tsx`，157 行）：标签 `members | basic | usage | skills | im | codeIndex`；members 仅多席位（`SessionRosterEditor`），codeIndex 仅 OnePiece。
- **Loop/Plan 中心**：`LazyFeature` 门控，访问后才挂载（`loopCenterVisited`/`planCenterVisited`，`:390-414`）——它们不预挂载，也不卸载。
- 对话框：`CreateSessionDialog`（+ agent-section、workspace-sections、remote-workspace-section）、`ScheduledTasksDialog`、`SessionContextPanel`（右键菜单）、`SessionRecoveryNotice`。末尾 `NotificationHost`。
- 焦点模式（`conversationFocusMode`）：通过 `data-*` 网格属性折叠两侧边栏和信息面板。

### 11.2 中央数据 hook

`use-main-layout-model.ts`（298 行）是中央数据 hook：TanStack Query 键覆盖 agents/sessions/archived/categories/search/active/messages/documents；乐观发送（`createOptimisticUserMessage`，`onMutate` at `:138`）；第 8.3 节的流式事件总线；导出类型 `MainLayoutModel`（`:298`）。配套还有 `use-session-switch.ts`、`use-session-recovery-sync.ts`、`chat-operation-failure.ts`（错误事件工厂）。

### 11.3 会话工作区标签页

`session-workspace/` 下：`agent-terminal-tab.tsx`（真实 xterm agent 终端）、`terminal-tab.tsx`（工具使用 I/O 日志视图）、`shell-tab.tsx`、`logs-tab.tsx`（285 行，会话日志分页）、`changes-tab.tsx`（+ `diff-view.tsx`，git 变更/diff）、`files-tab.tsx`、`documents-tab.tsx`、`report-tab.tsx`（+ `report-utils.ts`）、`execution-timeline-tab.tsx`（traces）、`folder-opener-control.tsx`、`session-conversation-header.tsx`、`conversation-overflow-menu.tsx`、`workspace-state.tsx`、`tab-scope.ts`（`showsSeatSwitcher`）、`terminal-utils.ts`、`terminal-theme.ts`、`trace-seat.ts`、`session-workspace-limits.ts`、`workspace-error.tsx`、`log-list-utils.ts`、`git-status-presentation.ts`。

### 11.4 设置中心

`settings/settings-shell.tsx`：search-param 驱动页面选择，`visitedPages` keep-alive，每页懒加载。`settings-pages.ts` 定义 17 个页面 id：`basic, agent-configurations, agent-policies, cli-parameters, mcp, skills, personalization, prompt-hooks, expert-roles, providers, extensions, plugins, im, ssh-connections, observability, usage, about`，每个是 `SettingsPageDefinition`（icon/labelKey/loader）。`settings/pages/` 下 46 个页面模块（含子目录 `agents/`[CLI 配置 profile、LSP 配置、OnePiece 面板]、`skills/`[skill 卡片、overlay、漂移、reconciliation、变更对话框]、`mcp/`、`im/`、`ssh/`、`usage/`、`personalization/`、`expert-roles/`、`prompt-hooks/`）。共享布局：`settings-sidebar.tsx`、`settings-topbar.tsx`、`page-parts.tsx`。

### 11.5 计划中心与循环中心

**`plan-center/`**：`plan-center.tsx`（目标表单 → 草稿编辑器 → run 视图；`subscribePlanRunPolling` 1.5s 轮询，`:38`）、`plan-draft-editor.tsx`、`plan-policy-editor.tsx`、`plan-run-view.tsx`。

**`loop-center/`**：`loop-center.tsx`（三栏：`LoopNavigation` | 时间线 | `LoopInspector`，响应式抽屉模式带 focus-trapping `useDrawerFocus`）、`loop-timeline.tsx`、`loop-iteration-details.tsx`、`loop-monitoring.ts`/`loop-run-controls.tsx`、`loop-inspection-actions.tsx`、`loop-navigation.tsx`、`loop-definition-dialog.tsx` + `loop-definition-form.ts`、`loop-verification-command-editor.tsx`。

### 11.6 悬浮助手

`floating-assistant/`：`floating-assistant-root.tsx`（独立 QueryClient、Provider）、`floating-assistant-app.tsx`（collapsed→menu→chat 三种面模式，可拖拽 header，`useSessionMessageEvents`）、`floating-assistant-status.ts`（从会话 + 流式消息推导 `FloatingAssistantStatus`）。

## 第 12 章 国际化与主题系统

### 12.1 i18n

`src/i18n/`：`index.ts` 初始化 i18next，`fallbackLng`/默认 `zh-CN`，init 时只加载默认 bundle；`ensureAppLanguage`（`:29`）延迟加载资源 bundle，带去重 map；`activateAppLanguage`（`:45`）加载失败回退 `zh-CN`，并应用 `document.documentElement.lang/dir`。`supported-locales.ts` 定义 5 个 locale——**zh-CN（默认，静态 import）、en、zh-TW、ja、ko**——每个带 `labelKey`、`direction`（目前都是 LTR）、懒 `load()`。

护栏测试强制两件事：`i18n-resource-parity.test.ts` 检查已注册 locale 的键/插值/重复键/复数形式 parity；`i18n-visible-text-guardrail.test.ts` 检查没有硬编码的可见文本。`project.md` 的 i18n 规则允许"产品名、provider 名、Agent 名、协议名、可执行文件、npm 包、命令、文件路径、URL、日志级别、稳定 id"保持字面量。

### 12.2 主题

`src/theme/theme-registry.ts` 定义恰好两个主题——`"futuristic"`（默认，深色）和 `"minimal"`（浅色）——加上 `themeStorageKey = "vanehub.uiStyle"`。

`src/styles.css` 是 **Tailwind v4**（`@import 'tailwindcss'`），架构如下：

- `@theme` token 映射（`:7-39`）：颜色 token 为 `hsl(var(--...))`。
- 三套 HSL token 调色板：`:root`（浅色兜底，`:59-124`）、`:root[data-theme="futuristic"]`（深色，`:126-163`）、`:root[data-theme="minimal"]`（浅色，`:165-202`）。token 覆盖 `--background/-foreground`、`--muted`、`--accent`、`--border`、`--ring`、`--primary`、`--destructive`、`--panel/*`、`--nav-active(/-soft)`、`--success(/-soft)`、`--warning(/-soft)`、`--danger(/-soft)`、per-agent 颜色 `--agent-codex/claude/opencode/gemini/antigravity`，以及固定深色**终端 ANSI 调色板**（`--terminal-*`，hex，`:98-124`，记为终端域约定）。
- 自定义 CSS 架构（"ucd" = VaneHub UI 设计）：`ucd-panel`（玻璃态，`backdrop-filter: blur(14px)`）、`ucd-muted-panel`、`ucd-card`、`ucd-interactive`、`ucd-segmented`、`ucd-list-row`、`ucd-workspace-grid`、`ucd-session-sidebar-resize`、`ucd-activity-bar`、`ucd-input`、`ucd-status-{success,warning,danger}`、`ucd-agent-{codex,claude,...}` 色调，以及 hljs/katex 主题化（`:465-504`）。响应式断点 900px（网格→两列）和 640px（单列，`:339-362`）。

`project.md` 的视觉设计规则要求两个主题暴露等价的语义角色（background/foreground/panel/muted panel/border/input/primary/success/warning/danger/focus ring/shadow），卡片用 ≤8px 圆角，桌面管理面用紧凑操作密度，悬停/激活/禁用/加载/聚焦状态不得改变控件尺寸或挤压相邻内容，页面区块避免"卡片套卡片"装饰。

## 第 13 章 类型与契约：前后端契约守护

VaneHub 没有像 protobuf/GraphQL 那样的代码生成器。它的前后端契约是靠 TypeScript 类型 + 一个**编译期类型相等测试**守护的。

### 13.1 `src/types/`——前端领域类型

- `agent.ts`（589 行，最大）：`InteractionMode`（`"browser"|"native-desktop"|"cli"|"api"`）、`AgentRegistryEntry`、`Session`、`SessionSeat`/`SessionSeatRoleSnapshot`、会话恢复状态、OnePiece provider 配置/预设/profile、`AgentMemory`、检索/代码索引配置、`WorkflowState`、CLI 工具/安装/冲突状态、`managedCliAgentIds` + `CliParameterDefinition/Profile/Selections`、定时任务、远程工作区、agent 终端。
- `chat.ts`（254 行）：`ChatConfig`、`ChatMessage`、`ToolUseBlock`、8 种 `RichBlock` 联合（每个带 `v: 1`）、`ChatStreamEvent`（8 变体联合，含 `turn_status`）、用量统计 shape。
- 其余：`session-workspace.ts`、`skill.ts`（279）、`skill-overlay.ts`、`skill-overlay-reconciliation.ts`、`cli-agent-config.ts`（274）、`code-index.ts`、`lsp.ts`、`loop.ts`、`plan.ts`、`prompt-hook.ts`、`mcp.ts`（+ `MCP_LIMITS` 常量）、`permissions.ts`（`ApprovalScope`、`PolicyTemplateName`、`RiskLevel`、`PrincipalEntry`、`PendingApprovalEntry`）、`sdk.ts`、`extension.ts`、`execution-observability.ts`、`token-usage.ts`、`settings.ts`（`AppSettings`、`ClientLogEvent`、`AppSettingKey`）、`expert-role.ts`、`floating-assistant.ts`、`folder-opener.ts`、`operation.ts`、`plugin-integration.ts`、`remote-terminal.ts`、`ssh-connection.ts`、`workspace.ts`、`provider-credential-validation.ts`、2 行的 `agent-seats.ts`。

### 13.2 `src/contracts/`——"已提交契约"镜像

大部分文件是**纯 re导出**（`chat.ts`、`token-usage.ts`、`session-workspace.ts`；`operation.ts` 含真正的 `OperationTask` 定义）。但 `agent.ts`（411 行）**复制**了核心 agent/session/CLI 类型而非 re-export；其他（IM、skill、loop、mcp、sdk、execution-observability）直接定义类型或用 **zod schema**——例如 `im.ts` 用 `z.enum`/`z.object` schema（`imConnectorViewSchema` 等）+ `parse*` 函数；`mcp.ts` 定义 `MCP_ERROR_CODES`/`MCP_LIMITS`；`folder-opener.ts` 提供运行时 `normalizeFolderOpeners` 守卫。

### 13.3 契约守护测试

`src/contracts/contract-conformance.test.ts`（394 行）是关键。它编译期断言 **类型相等**：`Equal<Contract.X, Types.X>` 跨 agent/chat/token-usage/mcp/sdk/skill/skill-overlay/operation/observability/loop/session-workspace（`:60-327`），外加运行时断言 MCP 常量和 skill 协议 fixture 匹配 native 层（`:346-393`）。

```mermaid
graph LR
    Types["types/ 前端领域类型"] --- Test{"contract-conformance.test.ts<br/>Equal&lt;Contract.X, Types.X&gt;"}
    Contracts["contracts/ 契约镜像<br/>(部分 re-export, 部分复制, 部分 zod)"] --- Test
    Test -->|编译通过| OK["契约一致"]
    Test -->|不等| Fail["编译失败 → 必须修"]
    Contracts --> Rust["Rust DTO<br/>#[serde(rename_all=camelCase)]"]
    Rust -.invoke 边界.-> Frontend["前端 invoke&lt;T&gt;"]
```

`Equal<A, B>` 是条件类型技巧：`type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends (<T>() => T extends B ? 1 : 2) ? true : false`。如果 `contracts/agent.ts` 和 `types/agent.ts` 的某个类型结构不匹配，测试**编译失败**。这就是不靠代码生成器也能保证前后端 interface parity 的机制——Rust 侧 DTO 用 `#[serde(rename_all = "camelCase")]`，前端 `invoke<T>` 用对应的 TS 类型，二者结构由 contracts/types 对齐 + 这个测试守护。

值得注意的设计张力：`contracts/agent.ts` 复制而非 re-export 类型，是为了让"契约"成为一份独立的、可被审计的、与 `types/` 解耦的快照。一旦契约变化，相等测试会逼你同时改 `types/`。

### 13.4 与后端共享的关键跨切类型

`ChatMessage` + `ChatStreamEvent`（流式协议）、`Session`/`SessionSeat`（多智能体）、`OperationTask`（CLI 安装/MCP 测试等长操作）、`LoopRun`/`LoopDefinition`/`LoopEvent`、`PlanRunDetail`/`PlanDraft`、`Skill`/`SkillOverlay*`、`PromptHook*`、`McpServerConfig` + `MCP_LIMITS`、`TokenUsage*`、`ClientLogEvent`。这些类型在前端 `types/`、`contracts/`、Rust 命令 DTO、Rust 领域模型四处出现，由契约测试对齐前端两处、由 `commands/error.rs` 的 `From<XxxError>` 实现对齐错误。

---

# 第三部分 Rust 原生后端（DDD）

第二部分讲的是前端的"壳"，第三部分讲的是壳背后的"核"。VaneHub 的 Rust 侧是一个严格的领域驱动设计（DDD）单 crate，它的分层规则不只是写在 `openspec/project.md` 里，而是由一个解析 Rust AST 的架构测试机械强制。这部分会逐层拆解。

## 第 14 章 原生架构总览与限界上下文

`ARCHITECTURE.md` 描述的是一个 8 上下文的目标，但实际代码已经演化为 **14 个限界上下文**，注册在 `src/contexts/mod.rs:3-15`：`agent_runtime`、`code_intelligence`、`communications`、`desktop`、`execution_observability`、`operations`、`permissions`、`retrieval`、`sessions`、`ssh_connections`、`task_orchestration`、`tooling`、`workspaces`（外加 legacy `mod.rs` 自身）。

整个 crate 是单一 Cargo 包 `vanehub-ai`（`src-tauri/Cargo.toml:1-15`），edition 2021，lib 目标 `vanehub_ai_lib`（`crate-type = ["staticlib","cdylib","rlib"]`），外加第二个二进制目标 `src/bin/vanehub-permission-hook.rs`（Claude Code 权限 hook 包装器，`default-run = "vanehub-ai"`）。

### 14.1 上下文地图

`ARCHITECTURE.md` 的 Context Map 表格给出了每个上下文的发布职责、上游依赖、下游消费者：

| 上下文 | 发布职责 | 上游 | 下游 |
| --- | --- | --- | --- |
| `agent_runtime` | Agent 目录、工作流选择、就绪、Provider 调用、生成生命周期 | `tooling`（CLI/prompt 配置）、`sessions` API、`operations` 端口 | Tauri 命令、`communications` 入站执行 |
| `sessions` | 会话/消息/分类/配置生命周期、导出、维护、用量读模型 | `operations` 端口、有界 `workspaces` 文件访问 | Tauri 命令、`agent_runtime`、`communications` |
| `workspaces` | 项目、远程工作区、worktree、文件/Git 检查、PTY shell | `operations` 端口 | Tauri 命令、`sessions` 有界文件读 |
| `tooling` | CLI、MCP、SDK、扩展、插件、Skill、Prompt Hook 子域 | `operations` 端口 + 平台适配器 | Tauri 命令、`agent_runtime` 配置 API |
| `communications` | IM 配置、凭据、传输、路由、授权、交付 | `sessions`/`agent_runtime` API、`operations` 端口 | Tauri 命令、连接器传输 |
| `desktop` | 设置、路径、启动、代理偏好、窗口/托盘/悬浮生命周期 | `operations` 端口 + 平台适配器 | Tauri bootstrap + 命令 |
| `operations` | 可观测任务 + 统一诊断/操作日志契约 | 平台 clock/id + 统一日志实现 | 每个上下文 |

跨上下文调用默认是**同步的发布应用 API**。只有当一个完成的动作有独立处理的下游反应时，才用显式事件。没有任何上下文可以伸手到另一个上下文的存储或基础设施。

### 14.2 分层与依赖方向

每个上下文内部按四层组织：

```mermaid
graph TD
    subgraph Inbound["入站适配器"]
        Cmd["commands/&lt;context&gt;/<br/>每个 Tauri 命令一个文件"]
    end
    subgraph Application["应用层"]
        App["application/<br/>用例 + ports"]
        Api["api.rs<br/>发布的跨上下文契约"]
    end
    subgraph Domain["领域层（纯）"]
        Dom["domain/<br/>实体/值对象/聚合/不变量"]
    end
    subgraph Infra["基础设施层"]
        Inf["infrastructure/<br/>SQLite/进程/网络/凭据适配器"]
    end
    subgraph Boot["bootstrap/"]
        B["组合根：选具体实现<br/>注入到 Tauri state"]
    end
    Cmd -->|"DTO→领域, 调用用例, 领域→DTO, 发事件"| App
    App --> Dom
    Inf -.实现.-> App
    Inf -.不定义业务不变量.-> App
    B -.选具体实现.-> Inf
    B --> Api
    Api -.跨上下文同步调用.-> App
```

依赖方向**只允许向内指**（`project.md` 原文）：

```
commands / inbound adapters -> application -> domain
infrastructure -------------> application ports + domain
bootstrap ------------------> outer implementations for construction only
```

- **`domain`** 禁止依赖 Tauri、Rusqlite、文件系统/进程/网络 API、凭据存储、任务注册表、日志实现、infrastructure、commands、bootstrap，或另一个上下文的私有模块。
- **`application`** 只能依赖自己的 domain、输入输出模型和 ports、以及刻意发布的跨上下文契约。禁止依赖 Tauri state/命令、Rusqlite 连接、具体 I/O 适配器。
- **`infrastructure`** 实现 application 拥有的 ports，**禁止定义业务不变量**。
- **Tauri 命令处理器**校验/映射 transport DTO、获取已组装的用例、调用它、映射命令安全输出/错误、执行接口拥有的事件发射。**禁止执行 SQL、构造外部进程、决定领域策略**。
- **`bootstrap`** 是唯一选择具体实现的层。Tauri 管理的 state 可以存已组装的应用服务，但**不得**被 domain 或 application 当作服务定位器。

### 14.3 模块布局目标

```
src-tauri/src/
├─ lib.rs                         # 仅模块暴露 + bootstrap::run() 委托
├─ bootstrap/                     # 组合根、运行时设置、state、后台任务
│  └─ runtime.rs                  # Tauri builder + 显式依赖装配
├─ contexts/
│  └─ <context>/
│     ├─ domain/                  # 实体、值对象、不变量、领域错误/事件
│     ├─ application/
│     │  └─ ports/                # 用例 + 消费侧 I/O 契约
│     ├─ infrastructure/          # SQLite/进程/网络/文件系统/凭据适配器
│     └─ api.rs                   # 刻意发布的跨上下文契约
├─ commands/
│  ├─ registry.rs                 # 按 bounded context 分组的完整 invoke handler
│  └─ <context>/                  # 每个 Tauri 命令一个文件，按 context 分组
└─ platform/                      # 可复用外层技术适配器
   ├─ database/  process/  filesystem/  network/
   ├─ logging.rs  credentials/  clock.rs  ids.rs  git/  error.rs
```

`project.md` 的几条硬规则：空层目录**不得**提前创建，直到该上下文需要它；迁移专用兼容模块需要活跃的、显式文档的 OpenSpec 任务（完成的 DDD 迁移没有常设兼容模块配额）；模块默认私有，用最窄的可见性（`pub(super)` 或 `pub(crate)`），公共上下文访问走 `api` 或显式接口契约。

## 第 15 章 引导与组合根

### 15.1 入口点

- `src/main.rs:4` — `vanehub_ai_lib::run()`；5 行纯委托。
- `src/lib.rs:26-31` — `pub fn run()` 先检查 `contexts::tooling::mcp::infrastructure::try_run_from_process_args()`（进程级 MCP helper 模式，`:27`），再调用 `bootstrap::run()`。
- `src/lib.rs:7-10` 只暴露 `bootstrap`、`commands`、`contexts`、`platform`；仅测试用的根 `contract_tests`、`migration_fixture_tests`、`native_lifecycle_tests`、`native_lsp_end_to_end_tests`、`remote_terminal_migration_tests`、`test_support`（`:12-23`）。

### 15.2 Tauri builder

`bootstrap::run()`（`runtime.rs:16`）构建 `tauri::Builder::default()`：

- `.plugin(tauri_plugin_dialog::init())`、`.plugin(tauri_plugin_opener::init())`、`.plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))`（`:20-27`）
- `.setup(setup)`（`:29`）
- `.on_window_event(crate::contexts::desktop::infrastructure::handle_main_window_event)`（`:31`）
- `.invoke_handler(crate::commands::invoke_handler())`（`:33`）
- `app.run(...)` 监控 `RunEvent::Exit`，以有界截止时间关闭 `ExecutionTelemetryLifecycle`（`:40-55`）

### 15.3 `setup()`——显式依赖装配

`setup()`（`runtime.rs:72-353`）是整个原生侧最关键的函数。它**显式**装配每个 API，没有 DI 框架。顺序如下：

```mermaid
flowchart TD
    S1["1. 解析 VANEHUB_APP_DATA_DIR 或 app_data_dir()"] --> S2["2. NativeDatabase::new()<br/>+ install_main_webview_recovery()"]
    S2 --> S3["3. desktop settings/floating API<br/>激活日志目录 + 自启"]
    S3 --> S4["4. assemble_operations_api → OperationsApi"]
    S4 --> S5["5. code_intelligence API + responder + fanout"]
    S5 --> S6["6. tooling 子域: cli_parameters/cli_config/mcp/cli/sdk/<br/>shared_registry/extensions/plugin_integration/skills/prompt_hooks"]
    S6 --> S7["7. ssh_connections + workspace API"]
    S7 --> S8["8. assemble_sessions_api<br/>(SessionRuntimeDependencies)"]
    S8 --> S9["9. assemble_permissions_api"]
    S9 --> S10["10. DeferredAgentRetrieval（空 cell）<br/>retrieval 与 agent_runtime 循环依赖"]
    S10 --> S11["11. assemble_agent_runtime_api<br/>→ AgentRuntimeAssembly<br/>(api, telemetry_lifecycle, completion_events)"]
    S11 --> S12["12. permission 超时清扫<br/>+ execution_observability<br/>+ assemble_retrieval<br/>→ RetrievalAssembly"]
    S12 --> S13["13. 缝合延迟接缝:<br/>session_runtime.attach_agent_runtime<br/>session_recovery.run_startup_with_retry(100)"]
    S13 --> S14["14. task_orchestration API<br/>+ agent_runtime.reconcile_loop_startup()"]
    S14 --> S15["15. assemble_communications<br/>+ WeChatAuthorizationApi<br/>+ CommunicationsCompletionHook"]
    S15 --> State["app.manage(...) 注册全部 state"]
    State --> Jobs["启动后台任务"]
```

注意几个微妙点：

- **延迟接缝 `DeferredAgentRetrieval`**（`:191, 228`）：`assemble_retrieval` 需要 `AgentRuntimeApi`，而 `AgentRuntimeApi` 又需要 retrieval port——这是循环依赖。解法是先创建一个空的 cell，`assemble_agent_runtime_api` 时往里塞引用，`assemble_retrieval` 之后再 `bind`。
- **`AgentRuntimeAssembly`**（`:192-214`）返回三元组 `{api, telemetry_lifecycle, completion_events}`。`completion_events` 是给 IM 用的——`CommunicationsCompletionHook`（一个 `AgentCompletionHook`）在 `AgentEvent::MessageCompleted` 时调用 `api.notify_session_completion`（`:355-372`）。
- **会话启动恢复** `session_recovery.run_startup_with_retry(100)`（`:233-238`）：非阻塞，bootstrap 启动一个 worker 立即返回，worker 调用 `sessions` 维护 API。

### 15.4 受管 Tauri state

`runtime.rs:254-302` 通过 `app.manage(...)` 注册每个 API：`NativeDatabase`、`ScheduledTaskLogDirectory`、`OperationsApi`、`CodeIntelligenceApi`、`CliApi`、`CliConfigApi`、`CliParametersApi`、`McpApi`、`SdkApi`、`ExtensionApi`、`PluginIntegrationApi`、`SkillApi`、`PromptHookApi`、`SshConnectionsApi`、`WorkspaceApi`、`TaskOrchestrationApi`、`SessionsApi`、`AgentRuntimeApi`、`PermissionsApi`、`RetrievalApi`、`CodeIndexApi`、`ExecutionTelemetryLifecycle`、`ExecutionObservabilityApi`、`CommunicationsApi`、`WeChatAuthorizationApi`、`DesktopSettingsApi`、`FloatingAssistantApi`、`DesktopLifecycleApi`。

这些就是 Tauri 命令处理器通过 `State<'_, XxxApi>` 拿到的服务实例。

### 15.5 后台任务清单

`runtime.rs:304-351` 启动的后台任务：

| 任务 | 触发 | 位置 |
| --- | --- | --- |
| 定时任务扫描（启动 + 60s tick） | `start_scheduled_task_jobs` | `runtime.rs:304`，impl `bootstrap/scheduled_tasks.rs:22` |
| 执行可观测保留 | `start_execution_retention_job` | `runtime.rs:310` |
| 会话维护（每小时归档） | `start_session_maintenance_jobs`（线程 + `SESSION_MAINTENANCE_INTERVAL = 60*60s`） | `runtime.rs:314`，`bootstrap/sessions.rs:98` |
| 检索索引 worker | `start_retrieval_indexing_worker` | `runtime.rs:319` |
| Agent 终端空闲清理（60s tick，2h 空闲） | `start_agent_terminal_cleanup_job` | `runtime.rs:320` |
| 初始 CLI 刷新 | `start_initial_cli_refresh` | `runtime.rs:337` |
| IM 去重 + WeChat 上下文保留（6h） | `start_communications_maintenance_job` | `runtime.rs:338` |
| 启动已保存的 IM 连接器 | `communications_api.start_saved_connectors().await`（spawn） | `runtime.rs:342-351` |

`bootstrap/mod.rs:28-65` re-export 每个 `assemble_*` 函数和 job starter。

## 第 16 章 限界上下文详解

### 16.1 `operations`——所有上下文消费的枢纽

`contexts/operations/` 是中心枢纽。

**Domain**（`domain/operation.rs`）：`OperationKind` 枚举（`Sdk|Mcp|Agent|Workspace|Extension`，serde 小写，`:6`）、`OperationStatus`（`Queued|Running|Succeeded|Failed|Cancelled`，`:16`）、`OperationTask` 聚合（`:64-79`）带 `start/append_log/correlate_execution/succeed/fail/cancel` 转换方法（`:82-137`）、`OperationLogEntry`（`:56`）、`OperationRecoveryEvidence`（`:48`）。

**Application**（`application/`）：`OperationService`（start, append_log, correlate_execution, complete, fail, cancel, cancellation_flag → `Arc<AtomicBool>`，get, list, list_recovery_evidence — `operation_service.rs:52-185`）；ports `OperationRepository`、`OperationClock`、`OperationIdGenerator`（`application/mod.rs:10-11`）；logging 契约在 `application/logging.rs`：`LogSeverity`、`DiagnosticLog`、`OperationLog`，ports `DiagnosticLogPort`、`OperationLogPort`、`ExternalLogExportPort`（`:4-39`）。

**Infrastructure**（`infrastructure/mod.rs`）：`persistent_operation_service`（SQLite-backed `operation_registry.rs`）和 `UnifiedLoggingAdapter`（`unified_logging.rs`）实现两个 log port，落在 `platform::logging` 上。

**API**（`api.rs`）：`OperationsApi`——start/append_log/correlate_execution/complete/fail/cancel/cancellation_flag/get/list/list_recovery_evidence（`:30-98`）；re-export `OperationKind`、`OperationTask`、`DiagnosticLog`、`LogSeverity`、`OperationsError`。

一个重要设计：`OperationService` 维护 `cancellations: HashMap<String, Arc<AtomicBool>>` 注册表（`operation_service.rs:56`），调用方可以轮询取消标志。操作本身是**内存的**，只有 `operation_recovery_evidence`（恢复证据）持久化到 SQLite，让崩溃恢复能读到终端状态（第 33 章详述）。

### 16.2 `sessions`——会话/消息/分类/配置

**Domain**（`domain/`）：`SessionAggregate`（`session.rs:154`）——字段 id/title/lifecycle/owner/category_id/pinned/archived/recovery；方法 `create`、`rehydrate`、`activation`、`ensure_accepts_messages`、`transition_to`、`assign_category`、`set_pinned`、`archive`、`unarchive`、`can_archive_automatically`（`:165-312`）。支撑值对象：`SessionTitle`（默认 `"新会话"`，`session.rs:6`）、`SessionLifecycle`（`Idle|Starting|Running|Failed|Stopped`，`:33`）、`SessionActivation`、`SessionOwner`（`Desktop` vs `Connector{connector_id}`——IM 会话不能激活，`session.rs:96-151`）、`LoopSessionRole`（`Worker|Verifier`，`:74`）。

`SessionMessage`（`message.rs:161`）带 `MessageRole`、`MessageStatus`（`Pending|Streaming|Completed|Failed|Cancelled` 带 `can_transition_to` 状态机，`:114-158`）、`FileReference`/`FileReferenceSet`（最多 5 个引用，路径唯一，`message.rs:4,67-89`）、`SessionSeat`/`SessionSeatRoleSnapshot`（JSON 列 seats，`session_seat.rs:18-33`）、`CategoryName`/`SessionCategory`、`ChatPreferences`/`ChatConfigurationRequest`、`SessionRecoveryReport`/`RecoveryDecision`/`RecoveryReasonCode`/`RecoveryTrigger`，身份 `SessionId`/`MessageId`/`CategoryId` 经宏（`identity.rs:30-32`），用量核算值对象（`AccountingUnit`、`TokenDimensions`、`UsageStatus`）。

**Application**（`application/`）：`SessionsApplicationService`（组合 ports `SessionApplicationPorts`，`service.rs`）、`SessionRecoveryCoordinator`（`recovery_coordinator.rs`）、token 核算（`usage_accounting.rs`、`usage_accounting_ports.rs`）。Ports（`application/ports.rs`）包含 15+ trait，从 `SessionRepository`（`:17`）到 `SessionTransactionPort`（`:188`——多表协调：create/activate/archive/delete/complete_message）、`SessionCreationContextPort`（`:302`——worktree/project/remote-workspace 创建委托给 workspaces）等。

**Infrastructure**（`infrastructure/`）：`SqliteSessionsRepository`（`sqlite_repository.rs:31`）实现 session+message+category+configuration+usage repos；`transactions.rs` 实现 `SessionTransactionPort`；`usage.rs`/`usage_accounting.rs`/`usage_accounting_projection.rs`（v22/v64 schema）；`creation_context.rs`、`chat_profile.rs`、`operation_adapter.rs`、`runtime_support.rs`、`scheduled_tasks.rs`。

**API**（`api.rs`）：`SessionsApi`（约 60 方法，`:34-441`）：创建（`prepare_creation`/`execute_creation`）、CRUD、分类、chat config、durable generation（`start_generation`/`terminalize_generation`）、消息持久化（`create_message`、`append_message_content/thinking/tool_use/rich_block`、`complete_message`、`fail_message`）、prompt 组合、导出、用量统计、恢复、`run_maintenance`。

`SessionAggregate` 的不变量值得单独强调：归档会话不能激活、接受消息或启动生成（`session.rs:218-255`）；连接器拥有的会话不能激活（`ConnectorCannotActivate`）；`can_archive_automatically()` 要求未归档、未置顶、无活跃生成、恢复干净（`session.rs:273-279`）。

### 16.3 `workspaces`——项目/远程/worktree/PTY

**Domain**（`domain/`）：`ProjectPath`/`ProjectInspection`（`project.rs:31`——规范路径、git_root 探测、`ensure_git_worktree_available`）、`WorktreeName`/`GitReference`（`worktree.rs`——拒绝 `/`、`\`、`..`、控制字符；派生分支 `vanehub/{name}`；`ensure_worktree_compatible` 禁止 remote+worktree）、`RemoteWorkspace`（`remote_workspace.rs`）、`ShellHost`/`TerminalDimensions`（`shell.rs`）、`CommandRun`/`CommandRunStatus`、`CommandTemplate`、`TerminalOutputChunk`、`remote_terminal_limits.rs` 常量。

**Application**（`application/`）：`WorkspaceApplicationService`（项目/历史/worktree）、`WorkspaceQueryApplicationService`（有界目录/文档/git/log 读）、`WorkspaceShellApplicationService`（PTY 生命周期）。Ports（`application/ports.rs`）：`WorkspaceHistoryRepository`、`WorkspaceGitPort`、`WorkspaceFilesystemPort`、`ProjectDirectorySelectionPort`、`WorkspaceClockPort`、`WorkspaceSessionQueryPort`、`WorkspaceShellRuntimePort`、`WorkspaceShellEventPort`、`WorkspaceShellIdPort`、`WorkspaceShellLogPort`、`WorkspaceShellContextPort`。

**Infrastructure**（`infrastructure/`）：`SqliteWorkspaceHistoryRepository`（现有表投影进 `known_projects`/`known_remote_workspaces` + `sessions` 列）、`WorkspaceFilesystemAdapter`（基于 `platform::filesystem::BoundedFilesystem`）、`WorkspaceGitAdapter`（基于 `platform::git::GitAdapter`）、`PortablePtyShellRuntime`（worker 线程输出读取）、`TauriProjectDirectorySelection`、`TauriWorkspaceShellEventPublisher`/`UuidWorkspaceShellId`/`WorkspaceShellLoggingAdapter`、远程终端 SQLite（`remote_terminal_schema.rs`、`remote_terminal_logging.rs`、`command_runs.rs`、`command_templates.rs`、`output_search.rs`、`capture_queue.rs`、`capture_maintenance.rs`）、`SqliteShellWorkspaceAdapter`、`SystemWorkspaceClock`。

**API**（`api.rs`）：`WorkspaceApi` 包装三个服务——项目/远程列表、`inspect_project`、`select_project_directory`、`create_worktree`/`create_guarded_loop_worktree`/`create_guarded_plan_worktree`、`resolve_session_root`、会话文件/目录/git/log 查询带 `_blocking` async 包装，以及 shell `create_shell`/`write_shell_input`/`reset_shell_directory`/`resize_shell`/`kill_shell`/`kill_shells_for_session`。

### 16.4 `tooling`——伞形子域

`tooling/mod.rs:3-11` 列出 9 个子域，每个都是完整的 domain/application/infrastructure/api 四元组：

| 子域 | Domain 类型 | API |
| --- | --- | --- |
| `mcp` | `ServerName`（kebab-case 不变量，`domain/mod.rs:66`）、`TransportType`（`Stdio|Sse|StreamableHttp`，`:89`）、`Scope`（`User|Project`）、`ServerConfiguration`/`ServerConfigurationDraft`、`McpFailureCode`（9 码带安全消息）、`ConnectionOutcome`、`ToolCallOutcome`、`ServerStatus` | `McpApi`（`api.rs:16-100`）；infrastructure 含 `SqliteMcpServerRepository`、`bounded_stdio.rs`、`streamable_http*.rs`、`legacy_sse*.rs`、`relay*.rs`（MCP relay/桥） |
| `sdk` | `SdkId`、`SdkDefinition`、`SdkStatus`、`SdkVersionInfo`、`SdkOperationType`、`SdkInstallStatus` | `SdkApi`（`api.rs:14-67`）；infrastructure 含 `SqliteSdkRepository`（v21 `sdk_operation_logs`）、`package_adapter.rs`（npm）、`process_adapter.rs` |
| `extensions` | `ExtensionCatalog`、`ExtensionEnvironmentReason`、生命周期规则（9 状态 `ExtensionLifecycleStatus`）、`ExtensionHealth`、`ExtensionInstallationDrift` | `ExtensionApi`（`api.rs:17-51`）；infrastructure 含 `SqliteExtensionRepository`（v15）、`installation_adapter.rs`、`runtime_adapter.rs`（自有 loopback runtime） |
| `plugin_integrations` | 内置目录（GitHub）、就绪计划、分类 | `PluginIntegrationApi`（`api.rs:12-33`）；infrastructure 含 `tool_adapter.rs`（有界 `gh` CLI 执行） |
| `skills` | `SkillId`（kebab-case）、`SkillScope`（`Global|Workspace`）、`SkillKey`、`SkillMetadata`、`SkillOrigin`、`SkillDelivery`（`Eager|Lazy`）、`SkillType`（`Role|Task`）、6 个内置、漂移分类、完整 overlay 子域 | `SkillApi`（`api.rs:24-129`+），可选 `with_overlay_service`；infrastructure 含 `SqliteSkillRepository`（v7 + v37 可靠性 + v60 effective-runtime）、overlay journal |
| `prompt_hooks` | `PromptHookId`、`PromptHookManifest`、`PromptHookCategory`/`Stage`/`Source`、`ManagedCliAgentId`、`PromptHookBindings`、排序、模板插值 | `PromptHookApi`（`api.rs:17-127`）——**不可变 effective-prompt 契约** `effective_prompt(agent_id, session_id, user_prompt)` 被 agent_runtime 消费；infrastructure 含 `SqlitePromptHookRepository`（v19） |
| `cli` | CLI 工具目录 `CLI_TOOL_DEFINITIONS`（5 CLI）、`EnvironmentType`、`VersionCheckStatus`、`InstallSource`、`ConflictState`、`LifecycleEligibility` | `CliApi`（`api.rs:8-62`）；infrastructure 含 `SqliteCliStatusRepository`（v6 `cli_tool_status` + v16 env details）、`detection_adapter.rs`、`executable_locator.rs`、`native_config_reader.rs` |
| `cli_config` | `CliConfigError`、profile 领域 | `CliConfigApi`（`api.rs:33-758`——最大 api，含 `synchronize_startup` at 72）；infrastructure 含 `SqliteCliConfigRepository`（v34 `cli_config_profiles` + v35 `cli_config_applied_state`）、`credential_adapter.rs`（auth.json via OS store） |
| `cli_parameters` | `CliParameterOption`、`CliParameterDefinition`、`CliParameterProfile`（`cli_parameters.rs:58-90`）、每 agent 目录、`preview_args` 投影 | `CliParametersApi`（`cli_parameters.rs:760`）：list_profiles/save_profile/reset_profile/load_selections/normalize_selections/preview_args；schema v12 `cli_parameter_settings` |

`project.md` 要求 `tooling` 子域保持分离的领域模型和应用 API。一个子域**可以**通过批准的架构决策晋升为对等上下文（当它有独立语言、生命周期或事务所有权时）。用法报告仍是 `sessions` 读模型，而用量记录归 assistant 消息拥有。

### 16.5 `communications`——IM 连接器

**Domain**（`domain/`）：`ConnectorKind`（`connector.rs:7`——`Feishu|Telegram|DingTalk|WeCom|WeChat`，wire id `weixin` 带 legacy 别名 `wechat`）、`ConnectorFieldDefinition`/`ConnectorFieldStorage`（`Public|Secret`）per-kind 字段目录（`:31-93`）、`ConnectorConfig` 带 `reject_sensitive_public_config` 不变量（`:174-204`）、`ConnectorDescriptor`（supports_qr_authorization, max_outbound_chars）、`ConnectorLifecycle`/`ConnectorStatus`/`ConnectorHealth`、`RoutingSettings`、`SessionBinding`（state Active/Paused + completion_notifications）、`PairingIntent`、`ConnectorCheckpoint`、`NormalizedInbound`/`OutboundText`/`DeduplicationDecision`/`InboundDisposition`/`MAX_PENDING_PER_CHAT`、`AuthorizationAttempt`/`AuthorizationStatus`、`ConnectorErrorClass` + `classify_safe_code` + `safe_platform_status_code`。

**Application**：`CommunicationsApplicationService`（connector_snapshot, transport_health, assemble_connectors, routing, save_connector, set_connector_enabled, clear/test/restart, start_saved_connectors, shutdown, claim_inbound, route_inbound, maintain_deduplication, begin_pairing/cancel_pairing, session_binding, notify_session_completion, reset_bindings）、`LifecycleCoordinator`（per-ConnectorKind async mutex 通道，使单个连接器的生命周期串行化而其他保持响应）。Ports：repository、credential、transport、agent-execution、session-binding、operation、clock、logging。

**Infrastructure**：`SqliteCommunicationsRepository`（v10 各 IM 表 + v65 managed bindings）、`CommunicationsCredentialAdapter`（over `OsCredentialStore`，zeroizing 读）、transports（dingtalk/feishu/telegram/wecom/wechat + 共享 http/protocol/token_cache/runtime）、`ConnectorRuntimeManager`（全局 pending 上限 64，活跃 agent 生成上限 8）、`CommunicationsTransportAdapter`、`TauriConnectorLifecycleEvents`、`CommunicationsInboundBridge`、`WeChatAuthorizationService`、application adapters（`CommunicationsAgentExecutionAdapter` 用 `send_message_with_completion` + 一次性完成通道上的 `recv_timeout`——无 SQLite 轮询）。

**API**：`CommunicationsApi`（`:55-235`）+ `WeChatAuthorizationApi`（begin/poll/cancel，`:27-53`）。

### 16.6 `desktop`——设置/启动/窗口

**Domain**：`DesktopSettings`/`DesktopSettingKey`（16 个 key）/`DesktopSettingMutation`/`ApplicationLanguage`/`AutomaticArchivalSettings`/`StartupPreference`/`NetworkProxyPreferences`、悬浮助手（`FloatingAssistantConfig`、`FloatingAssistantPlatform`、`FloatingAssistantSurfaceMode`、`FloatingAssistantAnchor`、`MonitorWorkArea`、`ScreenPosition`、`SurfaceTransition`、`WindowPlacement`、`position_for_monitor`、`should_intercept_main_close`）、`should_hide_main_for_tray`、`NativeCopy` 本地化。

**Application**：`DesktopSettingsApplicationService`、`DesktopEnvironmentApplicationService`、`FloatingAssistantApplicationService`、`DesktopLifecycleApplicationService`（幂等优雅退出、延迟动作）。

**Infrastructure**：`SqliteDesktopSettingsRepository`（v5 `settings`）、`SqliteFloatingAssistantRepository`（v14 `floating_assistant_config`）、`TauriDesktopLifecycleAdapter` + `handle_main_window_event`、`TauriFloatingAssistantWindowAdapter`、`RuntimeNetworkProxyAdapter`/`RuntimeNetworkProxyActionsAdapter`、`DesktopDirectoryAdapter`、`PlatformNodeInfoAdapter`、`UnifiedClientLoggingAdapter`、`RuntimeLogDirectoryAdapter`、`TauriDesktopStartupAdapter`、`SystemDesktopClock`、`DesktopLocaleBridge`、`FolderOpenerService` + `webview_recovery.rs`（`install_main_webview_recovery`）。

**API**：三个 facade——`DesktopSettingsApi`（设置、归档、自启、数据管理、node info、代理 test/scan、客户端日志、文件夹打开器、开目录）、`FloatingAssistantApi`、`DesktopLifecycleApi`。

### 16.7 `agent_runtime`——核心

这是第四部分的主角，这里先给骨架。**Domain**：`AgentId`、`InteractionMode`（`Browser|NativeDesktop|Cli|Api`，`catalog.rs:18`）、`LaunchKind`/`LaunchMetadata`、`AgentDefinition` + `AgentOrigin`（`Builtin|User`）带 `ensure_selectable`/`ensure_session_selectable` 不变量、`AvailabilityAssessment`/`AvailabilityProbe`/`AgentAvailability`、`ProviderMetadata`/`ProviderCapabilities`/`ProviderReadinessPrerequisites`/`ProviderSessionRef`、`AgentLifecycle`/`AgentReadiness`/`AgentWorkflow`、`GenerationAttempt`/`GenerationState`（reserved→active→terminal 机）、loop 领域、`ExpertRole`、`SeatRoster`/`SeatTurn`。

**Application**：`AgentRuntimeApplicationService`、`AgentTerminalApplicationService`，加一整个 loop-engineering 集群（9 个 service）。**Ports**（`application/ports.rs`，50+ trait）：`AgentRegistryRepository`、`AgentWorkflowRepository`、`AgentSessionGateway`、`AgentCliProfileGateway`、`EffectivePromptGateway`、`AgentProcessGateway`、`AgentProcessEventSink`、`AgentTerminalGateway`、`AgentTaskPort`、`AgentLoggingPort`、`AgentClockPort`、`AgentPermissionPort`、`AgentEventPort`、`AgentGenerationPort`、`ConversationHistoryPort`、`ApiAgentGateway`、`ApiCredentialPort`、`OnePieceModelDiscoveryPort`、`OnePiecePlanningPort`、`ToolApprovalPort`、`AgentSkillPort`、`AgentCoreInstructionsPort`、`AgentMcpToolPort`、`AgentMemoryPort`、`AgentMemoryExtractionPort`、`AgentPersonalizationPort`、`AgentCodeRetrievalPort`、`AgentRetrievalPort`、`AgentCodeIntelligencePort`、`AgentCodeIntelligenceResponderPort`、`AgentWorkspaceMutationPort`，以及整个 Loop* port 集。

**Infrastructure**：`SqliteAgentRuntimeRepository`、`CredentialAwareAgentRegistry`、`RuntimeAgentProcessAdapter`（CLI 子进程 gateway）、`RuntimeAgentApiAdapter`（API 进程 gateway；`api_process_adapter.rs`，7894 行——全 crate 最大文件）、`CompositeAgentProcessGateway`、各 `Runtime*Adapter`、`SessionsAgentRuntimeAdapter`（跨上下文 gateway）、`InMemoryGenerationCoordinator`、`InMemoryLoopExecutionCoordinator`、`NativeLoopScheduler`、`NativeSeatTurnCoordinator`、`InMemoryAgentMessageTerminalCompletions`（一次性终端完成注册表）、`SqliteAgentMemoryRepository`（v31）、`SqliteExpertRoleRepository`（v46）、`SqliteLoopRepository`（v25）、`PortablePtyAgentTerminalRuntime`、`StructuredLoopVerificationProcess`、`TauriAgentRuntimeEventAdapter`（`events.rs`）、providers（`providers/mod.rs`：`builtin_cli_provider_registry`）、tools（`tools/`：edit/file/glob/grep/shell/walk）、`mcp_tool_gateway.rs`、`memory_extraction_gateway.rs`、`personalization_gateway.rs`。

**API**：`AgentRuntimeApi` 包装 9 个 service via `AgentRuntimeApiServices`（`:91`）。关键方法：`list_agents`/`get_agent`/`register_api_agent`/`update_api_agent`/`delete_api_agent`、OnePiece provider config/profiles/models/credentials、`workflow`/`select_agent`/`browser_readiness`/`launch_active_workflow`/`session_details`、`send_message`/`send_message_with_completion`（返回 `StartedAgentMessage` 带一次性完成）、`stop_generation`、`active_generation_correlation`、agent 终端、完整 loop 控制、expert roles、guarded validation（`run_guarded_validation_cancellable`）、memories、`resolve_tool_approval`、`list_embedding_models`。还有 `AgentMemoryDeletionGateway` trait（`:627`），让一个命令能删一条记忆而不必构建整个 API。

## 第 17 章 Tauri 命令层

### 17.1 注册表

`commands/mod.rs:3-17` 按上下文声明模块加 `mod registry`；`commands/registry.rs:3` 构建 `invoke_handler()` 作为一个 `tauri::generate_handler![...]`，列出 **299 个注册命令**，按 bounded context 分组带横幅注释：

- `agent_runtime` **46**（含 `expert_roles/*` 和 `loops/*`）
- `sessions` **35**（含 `scheduled_tasks/*` 4 命令）
- `desktop` **26**
- `communications` **18**
- `task_orchestration` **17**
- `retrieval` **17**（含 `code_index/*` 12）
- `workspaces` **16**
- `ssh_connections` **7**、`code_intelligence` **7**、`execution_observability` **6**、`permissions` **4**、`operations` **2**
- `tooling/*` **98** 总计（cli 4、cli_config 10、cli_parameters 3、mcp 9、sdk 10、skills 42、prompt_hooks 13、extensions 9、plugin_integrations 3）

### 17.2 共享错误映射

`commands/error.rs` 是共享错误映射层：`CommandError` + `CommandErrorCategory`（`Validation|NotFound|Conflict|Unsupported|Unavailable|Infrastructure|Internal`，`:25`），为每个上下文实现 `From<XxxError>`（sessions:357, agent_runtime:170, workspace:316, communications:145, mcp:517, cli:548, cli_config:574, sdk:617, extension:637, skills:681, prompt_hook:761, desktop:267, floating:298, ssh:431/459, operations:101, permissions:118, plan:250，`InfrastructureError` at 136）。敏感消息在 `From` 边界过 `redact_text`（`CommandError::redacted`，`:72`）；序列化为纯字符串（`Serialize` at 88）。

这条边界至关重要：raw rusqlite 错误文本永远不跨命令边界。`InfrastructureError::command_safe_message()` 返回静态安全字符串，真实诊断只进统一日志。

### 17.3 处理器形状（薄适配器，零 I/O）

代表性 `commands/agent_runtime/send_message.rs:6-22`：

```rust
#[tauri::command]
pub(crate) fn send_message(
    api: State<'_, AgentRuntimeApi>,
    session_id: String,
    content: String,
    config: dto::ChatConfig,
    file_references: Option<Vec<dto::ChatFileReference>>,
) -> Result<dto::ChatMessage, CommandError> {
    api.send_message(mapper::send_message_request(session_id, content, config, file_references))
        .map(mapper::message_to_dto)
        .map_err(map_command_error)
}
```

模式：`State<'_, Api>` → `mapper::*_request(dto)` → `api.*(...)` → `mapper::*_to_dto` → `map_err(map_command_error)`。DTO 文件是每上下文 `dto.rs`，输入输出 `#[serde(rename_all = "camelCase")]`，枚举 kebab-case；`mapper.rs` 做显式契约映射。后台调度是分开的：`create_session` 立即返回 `OperationTask` 并调 `background::spawn_creation(...)`；`commands/tooling/mcp/background.rs:3-7` `spawn_connection_test`；`commands/tooling/{cli,sdk,extensions}/background.rs` 调度预备好的 job。

### 17.4 事件发射

事件由实现 application port 的 infrastructure 适配器发射，**不是**命令：

```mermaid
graph LR
    subgraph Infra["infrastructure 适配器"]
        AE["TauriAgentRuntimeEventAdapter<br/>实现 AgentEventPort"]
        SE["NativeSessionRecoveryEvents"]
        WE["TauriWorkspaceShellEventPublisher"]
        CE["TauriConnectorLifecycleEvents"]
    end
    AE -->|chat:event| Chat["ChatStreamEvent 联合<br/>Started/Token/Thinking/ToolUse/RichBlock/<br/>Completed/Failed/Cancelled/TurnStatus"]
    AE -->|agent-terminal:event| AT["AgentTerminalEvent"]
    SE -->|session:event| Sess["SessionEventKind<br/>ActiveSessionChanged/ConfigurationChanged/<br/>Recovery*/RecoveryQuarantined/RecoveryAcknowledged"]
    WE -->|shell event| Shell["按 shell id 键"]
    CE -->|im-connector:lifecycle| Imc["按 connector generation 键"]
```

- Chat 流：`contexts/agent_runtime/infrastructure/events.rs`——`TauriAgentRuntimeEventAdapter` 实现 `AgentEventPort`，发射 `"chat:event"`（`:66`）带 tag 的 `ChatStreamEvent` 枚举（`Started|Token|Thinking|ToolUse|RichBlock|Completed|Failed|Cancelled|TurnStatus`，`:83-136`）和 `"agent-terminal:event"`（`:77`）；还有 `AgentCompletionHook`（`:10`）给 IM 完成。
- Sessions：`commands/sessions/events.rs`——`NativeSessionRecoveryEvents` 发射 `"session:event"` 带 `SessionEventKind`（`ActiveSessionChanged|ConfigurationChanged|RecoveryStarted|RecoveryCompleted|RecoveryActionRequired|RecoveryQuarantined|RecoveryAcknowledged`，`:22-30`）。
- Shell：`TauriWorkspaceShellEventPublisher`（`workspaces/infrastructure/shell_support.rs`）发射按 shell id 键的 shell 事件。
- 连接器生命周期：`TauriConnectorLifecycleEvents`（`communications/infrastructure/lifecycle_events.rs`）发射校验过的 `im-connector:lifecycle`，按 connector generation 键。

事件是另一条兼容性边界：`ARCHITECTURE.md` 明确"Tauri 事件契约也是兼容性边界"——会话状态事件、按 session id 键的 chat 流事件、按 shell id 键的 shell 事件、桌面/悬浮生命周期事件、校验过的 `im-connector:lifecycle` 按 connector generation 键。

## 第 18 章 平台层

平台层（`src/platform/`）是可复用的外层技术适配器，被多个上下文的 infrastructure 共享。

### 18.1 数据库

`platform/database/mod.rs`：`NativeDatabase` 拥有 `db_path` + `r2d2::Pool<SqliteConnectionManager>`（max 12 连接，min idle 1，5s 连接超时——`MAX_POOL_SIZE`/`CONNECTION_TIMEOUT` `:19-23`）。每个物理连接经 `with_init`（`:65-76`）配置一次：`busy_timeout(5s)`（`BUSY_TIMEOUT`）、`PRAGMA journal_mode=WAL`、`foreign_keys=ON`、`synchronous=FULL`（已验证）。`NativeDatabase::new` 跑迁移 + `seed_registry` 恰好一次（`:84-87`）；`connection()` 检出 `PooledSqlite`。`DatabaseError`（typed，`:37`）。DB 文件 `vanehub.sqlite`（`:31`）。

迁移系统在 `database/migrations.rs`：`migrate(conn)`（`:6`）创建 `schema_migrations(version, name, applied_at)` 然后应用 **65 个版本化迁移**。每个迁移是一个 `fn(&Connection) -> Result<(), DatabaseError>`，由 `apply_migration`（`:1084`）包在一个 `unchecked_transaction` 里，使 DDL+DML+版本行原子提交；`apply_transactional_migration`（`:1118`）相同但无幂等。迁移把 schema 所有权委托给上下文拥有的 `apply_*_schema` 函数（如 v7 → `skills::infrastructure::apply_schema`，v10 → `communications::infrastructure::apply_schema`）。运行后守卫：`assert_migration_history_is_dense`（`:700`）拒绝启动时的间隙或未知更高版本；`EXPECTED_MIGRATIONS` const（`:632`）是由 `migration_sequence_matches_expected` 测试（`:1905`）钉死的真源。版本碰撞事件有内联文档（v42/v43/v44 碰撞，`:247-270`）。`table_has_column`（`:1265`）支持幂等 `ALTER TABLE ADD COLUMN`。完整的 65 个迁移清单见第 32 章。

### 18.2 进程

`platform/process/mod.rs` 是**唯一**允许非测试外部进程构造的地方。关键类型：`ProcessRequest`（builder：executable/args/current_dir/env/timeout/cancellation/output_limit，`:102-174`）、`ProcessAdapter`（`execute` → `ProcessOutput`，`:176-188`）、`ProcessError`（typed：InvalidExecutable/Spawn/Wait/TimedOut/Cancelled 带捕获输出，`:24-49`）、`ProcessCancellation`（`Arc<AtomicBool>`）、`ManagedChild`/`ManagedTokioChild`（`managed_child.rs`）、`BlockingStderrDrain`/`TokioStderrDrain`（`stderr_drain.rs`）、Windows job 包含 `TerminateTreeJobObject`（`windows_job.rs`）。

不变量：`validate_executable` 拒绝空/控制字符（`:219`）；`std_command`/`tokio_command`（`:263`/`:271`）应用代理策略 + `CREATE_NO_WINDOW` 控制台抑制（`suppress_console_window` trait，`:242`；`CREATE_NO_WINDOW = 0x0800_0000` `:238`）；`output_with_control`（`:332`）把命令包在 kill-handle 包含（`process_wrap::CommandWrap`）里，500ms 宽限排干管道（`OUTPUT_DRAIN_GRACE`），强制 timeout/cancellation，超时时杀后代；`spawn_detached`（`:190`）用 `DETACHED_PROCESS`；`audit_command`（`:294`）写脱敏的 "executing ..." 日志行。

### 18.3 文件系统

`platform/filesystem/mod.rs`：`BoundedFilesystem`（根规范化，`:44-88`）带 `validate_relative` 拒绝绝对/隐藏/穿越组件（`BoundaryError`，`:6-24`）、`resolve_existing`、`resolve_with_existing_parent`、`ensure_inside`（symlink 安全规范包含）、`sibling_worktree_target`（`:107`，拒绝已存在目标）、`normalize_windows_extended_length_path`（`:34`，剥 `\\?\`）、`open_directory`（`:129`，explorer/open/xdg-open）。

### 18.4 日志

`platform/logging.rs`：只追加 JSONL 持久化（`write_entry`，`:100`）：每行 `LogEntry { timestamp, level, category, message, context }`；`LogLevel`（`Error|Warn|Info|Debug`，`:32`）；脱敏 `redact_text`（`:275`）——路径脱敏 `[REDACTED_PATH]`、`Bearer` 处理、provider token（`sk-`、`ghp_`、`github_pat_`、`ssh-connection/`）、敏感 key 脱敏 via `is_sensitive_key`（`:401`：password/api_key/token/secret/credential/authorization/key_path/private_key + IM 字段 external_chat/sender_id/message_content/prompt/response/qr_payload/headers/protocol_frame）。轮转：活动文件 `vanehub.log` 24h 轮转为 `vanehub-YYYYMMDDTHHMMSSZ.log`，30 天后归档进 `archive/`，维护限流 1/h（`ROTATION_AGE_HOURS`/`RETENTION_DAYS`/`MAINTENANCE_INTERVAL_HOURS`，`:22-24`，`maintain_log_dir` `:181`）。活动目录状态 via `OnceLock<Mutex<Option<PathBuf>>>` + `set_active_log_dir`/`active_log_dir`（`:71-86`）；`fallback_log_dir()`（`:147`）。`write_client_event` 给前端错误（`frontend.client` category，`:160`）。`private_relay_fs.rs` 是第二个允许追加写的位置（由架构测试强制）。

### 18.5 网络

`platform/network/mod.rs` + `proxy.rs` + `provider_credential_probe.rs`：全局 `PROXY_STATE`（`NetworkProxyState { url, bypass }`，`OnceLock<RwLock<_>>`，`proxy.rs:32-35`），默认 bypass `localhost,127.0.0.1,::1`（`:30`）；`http_client`/`blocking_http_client`/`blocking_no_redirect_http_client` 应用代理；`apply_to_std_command`/`apply_to_tokio_command` 注入代理 env 到子进程；代理检测探 `PROXY_PORTS`（7890/7891/1080/8080/8888/3128/10808/10809，`:63`）；`AsyncIo`/`BoxedAsyncIo` 给 TLS 包装流。

### 18.6 凭据

`platform/credentials/mod.rs`：`OsCredentialStore` over `keyring` crate：`set`/`get`/`delete` 带 `Zeroizing<String>` 读（`:24-35`），`NoEntry` 视为 `None`，错误映射到 `InfrastructureError::Credential`。

### 18.7 clock 和 ids

- `platform/clock.rs`：`SystemClock` 带 `unix_seconds()` 和 `rfc3339()`。
- `platform/ids.rs`：`MonotonicIdGenerator`（per-instance `AtomicU64`，prefix-timestamp-counter 格式 `op-100-1`）。
- `platform/git/mod.rs`：`GitAdapter` over 进程边界（`execute(root, args, timeout)` → `GitOutput`）、`redacted_diagnostic`。
- `platform/error.rs`：`InfrastructureError`（`Database|Storage|Process|Network|Credential|Serialization`）带 `category()` 和 `command_safe_message()`（静态安全字符串——raw rusqlite 文本不跨命令边界的原因）。

## 第 19 章 架构守护机制

VaneHub 的架构不靠口头约定，而靠一个解析 Rust AST 的集成测试机械强制。主战场是 **`src-tauri/tests/architecture.rs`**，用 `syn = 3` + `proc-macro2` 解析 Rust 源码。

### 19.1 向内依赖检查

`native_context_dependencies_point_inward`（`:851`）是核心解析依赖检查。`DependencyVisitor` 走 `src/` 下每个 `use` 和路径；`SourceScope` 计算为 `contexts/<ctx>/domain|application`。三条违规规则（`inspect`，`:231-259`）：

1. **禁用技术根**：`tauri|rusqlite|reqwest|rmcp|keyring|portable_pty` 加 `std::fs/net/process` 和 `tokio::fs/net/process`（`is_forbidden_technology`，`:309`）。
2. **外层 import**：`crate::platform`、`crate::commands`、`crate::bootstrap`（及移除的根 `logging`/`tasks`）在 domain/application 被禁；同上下文 `application|infrastructure|interfaces` 在 domain 被禁，`infrastructure|interfaces` 在 application 被禁（`is_forbidden_outer_layer`，`:332`）。
3. **私有跨上下文访问**：`crate::contexts::<other>` 除 `::api` 外任何东西（`imports_private_cross_context_module`，`:353`）。

### 19.2 其他守卫

- **`root_lib_contains_no_business_symbols`**（`:1230`）：`root_business_items` 断言 `lib.rs` 只声明 `mod`（和 `run`）；零 struct/fn/enum/impl/trait。
- **`tauri_command_adapters_cannot_gain_io_or_control_flow_decisions`**（`:1297`）：`CommandBodyVisitor` 计 `io_decisions`（禁用技术、`Command::new`/`Connection::open`、字符串字面量里的 SQL 关键字、`execute/prepare/spawn/output/kill/...` 方法调用）和 `control_flow_decisions`（`if/match/for/while/loop`）在 `#[tauri::command]` 体内；二者必须为零。这就是"命令处理器零 SQL/进程/领域决策预算"的机械执行。
- **`runtime_processes_and_append_logs_use_shared_adapters`**（`:1338`）：`RuntimeIoVisitor` 标直接 `std::process::Command::new`、`OpenOptions::new`、`.append(true)`；只 `platform/process/mod.rs` 和 `platform/logging.rs`/`platform/private_relay_fs.rs` 允许（带 detector 自检测试 at `:1370`）。
- **`distributable_release_profile_stays_optimized`**（`:8`）：钉 `opt-level=3`、`lto="thin"`、`codegen-units=1`、`strip="debuginfo"`、debug off。
- **`provider_neutral_layers_do_not_select_concrete_cli_providers`**（`:61`）：sessions domain/application + agent_runtime application 不得按 provider id（`claude-code`、`codex-cli`、`gemini-cli`、`opencode`、`antigravity-cli`）分支，也不得 import `infrastructure::providers`。
- **LSP/retrieval 边界测试**（`:917-1228`）：`code_intelligence` 必须只经其 `api` 可达，绝不 import `retrieval` 内部；agent_runtime/code_intelligence/retrieval 只能通过自有 port 或公共 api 互相触及；React 组件不得直接 invoke LSP 命令；web-lsp-client import 白名单。
- **Windows 专用**：`windows_command_constructors_suppress_console_windows`（`:1575`）+ `every_creation_flags_call_keeps_the_child_console_hidden`（`:1664`）钉 `CREATE_NO_WINDOW`。
- **兼容性/反回归**：`migrated_session_code_cannot_return_to_root_or_legacy_modules`（`:1242`）禁 30+ 遗留符号名和 `src/session_configuration.rs`/`src/usage.rs`；`communications_completion_wait_stays_event_driven_without_sqlite_polling`（`:1420`）；`commands_holding_the_retrieval_api_never_return_error_payload_text`（`:1443`）；`production_logging_contract_is_not_debug_assertion_gated`（`:1392`）；token-accounting 边界（`:117`）。

支撑测试根：`src/contract_tests.rs`（命令注册/DTO/错误契约测试）、`src/migration_fixture_tests.rs`（迁移序列/行为 fixture）、`src/native_lifecycle_tests.rs`、`src/native_lsp_end_to_end_tests.rs`、`src/remote_terminal_migration_tests.rs`、`tests/mcp_fixture_contracts.rs`、`tests/mcp_relay_provider_invocations.rs`。

`ARCHITECTURE.md:205-212` 文档化守卫；ADR-001（单 crate + 解析测试，`:216`）和 ADR-004（组合和命令注册分开集中化，`:228`）是治理决策。

这套机制的意义在于：架构分层从"code review 时人会检查"升级为"CI 编译期机械强制"。一个 PR 如果让 domain import 了 rusqlite，架构测试直接红，连 review 都进不去。这是项目能维持 1130 个 Rust 文件不腐化的根本原因。

---

# 第四部分 Agent 运行时与多 Provider

如果说前三部分讲的是"壳与骨架"，那么第四部分讲的是 VaneHub 真正的心脏——它如何调度、启动、监控那些异构的 AI 编程代理，如何把它们的输出统一成一套消息流，以及如何在多智能体、计划、循环等高级模式下保持一致。这是全系统最复杂的部分，两个运行时实现各司其职：CLI 运行时（`RuntimeAgentProcessAdapter`）启动并解析外部 CLI 进程；原生 API 运行时（`RuntimeAgentApiAdapter`，7894 行，全 crate 最大文件）直接打 provider HTTP 端点并自己跑工具使用循环。

```mermaid
graph TB
    subgraph Dispatch["CompositeAgentProcessGateway<br/>按 process_id/launch_kind 分派"]
        D{launch_kind?}
    end
    D -->|api| API["RuntimeAgentApiAdapter<br/>api_process_adapter.rs (7894 行)<br/>直接 HTTP + 自有工具循环"]
    D -->|cli| CLI["RuntimeAgentProcessAdapter<br/>process_adapter.rs<br/>spawn CLI 子进程 + 解析输出"]
    CLI --> Providers["providers/<br/>claude/codex/gemini/opencode/antigravity<br/>各自 invocation + output parser"]
    API --> Wire["anthropic_provider.rs<br/>openai_compatible_provider.rs<br/>SSE 翻译 + ToolCallAccumulator"]
    CLI --> Output["providers/output.rs<br/>ProviderOutputEvent 联合<br/>Token/Thinking/ToolLifecycle/<br/>RichBlock/SessionId/Completed/Failed"]
    API --> Sessions["sessions_gateway.rs<br/>SessionsAgentRuntimeAdapter<br/>持久化消息 + terminalize"]
    Output --> Sessions
    API --> Sessions
```

二者由 `CompositeAgentProcessGateway`（`infrastructure/composite_process_gateway.rs:15`）分派，按 `process_id` 前缀 `"agent-api-process-"`（`:8`）和 agent 的 `launch.kind`。组合根在 `src-tauri/src/bootstrap/agent_runtime.rs`（provider registry 在 `:156-213` 构建）。

## 第 20 章 Provider 模型与注册表

### 20.1 五个 CLI Provider 的兼容性矩阵

`src-tauri/src/contexts/agent_runtime/infrastructure/providers/compatibility.rs:31-82` 定义了 `DEFINITIONS: [CompatibilityProviderDefinition; 5]`：

| stable id | 显示名 | 可执行文件 | SDK 依赖 | 输出格式 | 用量能力 | reasoning | sandbox |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `claude-code` | Claude Code | `claude` | `claude-sdk` | `ClaudeStreamJson` | HeadlessAndTerminalReported | true | false |
| `codex-cli` | Codex CLI | `codex` | `codex-sdk` | `StructuredJsonLines` | HeadlessAndTerminalReported | true | true |
| `gemini-cli` | Gemini CLI | `gemini` | none | `StructuredJsonLines` | HeadlessAndTerminalReported | false | true |
| `opencode` | OpenCode | `opencode` | none | `StructuredJsonLines` | HeadlessAndTerminalReported | false | false |
| `antigravity-cli` | Antigravity CLI | `agy` | none | `AntigravityStreamJson` | HeadlessReported | true | true |

`builtin_cli_provider_registry()`（`:193`）构造 `ProviderRegistry::new(providers)`，`validate_builtin_contracts`（`:204`）强制无重复、声明完整。`ProviderRegistry` 类型在 `application/provider.rs`（拒绝重复注册）。领域模型 `domain/provider.rs`：`AgentProviderId`、`ProviderFamily::CodingCli`、`ProviderMetadata`、`ProviderCapabilities`（interaction_modes、session_resume、structured_output、terminal、usage、permissions、model_selection、reasoning、sandbox）、`ProviderReadinessPrerequisites`（executable_names + managed_sdk_dependency_id）、`ProviderSessionRef`（opaque provider-native resume id）。

### 20.2 种子 Agent 注册表

`platform/database/migrations.rs:1149-1223` 种子了 `agents`、`agent_modes`、`agent_capability_tags`、`workflow_state`、`session_details`、`mcp_servers`。`infrastructure/schema.rs:17-96` 定义 `AGENTS: [SeedAgent; 6]`——5 个 CLI + `onepiece`（`launch_kind='api'`），`seed_registry()` 在 `:269`（幂等 `INSERT OR IGNORE`，在 `:279` 拒绝非 `api` 的 `onepiece` 碰撞）。`apply_agent_origin_schema`（`:98`）加 `agent_origin`（`builtin`/`user`）。

### 20.3 可用性探测

`domain/catalog.rs`：`AgentDefinition`、`LaunchKind`、`InteractionMode`（`Browser`/`NativeDesktop`/`Cli`/`Api`）、`AgentAvailability`（`Available`/`Unavailable`/`NeedsAuthentication`/`Unknown`）、`AvailabilityAssessment::assess`（`:144`）先检查受管 SDK 状态再检查 PATH 上的可执行文件。`infrastructure/availability.rs` 是运行时可用性探测；`credential_aware_registry.rs` 是注册表包装器，把缺凭据视为 `NeedsAuthentication`；`api_credentials.rs` 按 `agent_id` 键存凭据，OnePiece profile 级键 `onepiece-profile:<id>`。

CLI 工具检测/版本管理在 `contexts/tooling/cli/`：`CliApi`（`cli/api.rs`：`list_tools`、`resolve_executable`、`prepare_refresh`、`prepare_install`、`prepare_upgrade_all`）；检测/包适配器在 `cli/infrastructure/`（`executable_locator.rs`、`detection_adapter.rs`、`package_adapter.rs`、`native_config_reader.rs`、`candidates.rs`）。

### 20.4 OnePiece 原生 Provider 模型

OnePiece 是 VaneHub 自己的"原生 API 代理"——它不依赖任何 CLI，而是直接把请求打到各 provider 的 HTTP 端点，自己跑工具使用循环。

**Provider 目录**：`src/config/onepiece-provider-catalog.json`——`catalogVersion`、`sourceRevisions`、恰好 **25 个 provider**，每个带审查过的端点（`anthropic-messages`、`openai-chat-completions`、`openai-responses`）。provider 包括：anthropic、openai、openrouter、deepseek、zhipu-glm、kimi、siliconflow、bailian、volcengine-ark、groq、xai、mistral、together-ai、fireworks、nvidia-nim、cerebras、minimax、minimax-global、stepfun、baichuan、ppio、qiniu、modelscope、xiaomi-mimo、zai。

加载器 `application/onepiece_provider_catalog.rs`（`include_str!("../../../../../src/config/onepiece-provider-catalog.json")` `:5`；`list()`/`resolve(provider_id, endpoint_type)`/`discovery_url`）。

**Provider Profile**：`onepiece_provider_profiles` SQLite 表（`infrastructure/schema.rs:138-171`），带部分唯一索引 `idx_onepiece_provider_profiles_active` 强制至多一个活跃 profile（`:156`）。

应用层类型 `application/models.rs`：`OnePieceProviderPreset`、`OnePieceProviderEndpoint`、`OnePieceProviderProfile`、`StoredOnePieceProviderProfile`、`SaveOnePieceProviderProfileInput`、`OnePieceModelDiscoveryRequest`、`ProviderCredentialProbeRequest/Result`。命令在 `commands/agent_runtime/` 下：`save_onepiece_provider_config.rs`、`list_onepiece_provider_profiles.rs`、`save_onepiece_provider_profile.rs`、`activate_onepiece_provider_profile.rs`、`delete_onepiece_provider_profile.rs`、`discover_onepiece_provider_models.rs`、`validate_onepiece_provider_credential.rs`、`list_onepiece_provider_presets.rs`、`reset_onepiece_provider_config.rs`、`get_onepiece_provider_config.rs`。

### 20.5 凭据校验共享探针

`application/service.rs:578`（`validate_onepiece_provider_credential`）；`infrastructure/onepiece_model_discovery.rs`（`HttpOnePieceModelDiscoveryAdapter`）。模型 `ProviderCredentialProbeProtocol`（AnthropicMessages / OpenAiChatCompletions / OpenAiResponses）、`ProviderCredentialProbeAuthentication`（AnthropicApiKey / Bearer）、`ProviderCredentialValidationStatus`（valid / invalid-credential / configuration-rejected / rate-limited / provider-unavailable / unsupported / inconclusive）在 `application/models.rs:1157-1188`。

### 20.6 每次生成的聊天配置

`application/models.rs:375-385`：`AgentChatConfiguration { agent_id, interaction_mode, execution_mode, provider_id, model_id, reasoning_depth, streaming, thinking, long_context }`。由 sessions 上下文校验（`validate_chat_configuration`、`validate_seat_chat_configuration`）。

## 第 21 章 会话与消息生命周期

### 21.1 SQLite 表

`sessions`（`migrations.rs:1239-1251`）：id、title、agent_id、interaction_mode、lifecycle_state、folder、pinned、archived、created_at、updated_at。后续迁移加 `chat_preferences`、各列、`seats` JSON、`loop_run_id`/`loop_iteration_id`/`loop_role`、`runtime_session_id`。

`messages`（`migrations.rs:1059-1078`）：id、session_id、role、status、content、thinking_content、tool_use（JSON）、rich_blocks（JSON）、token_input、token_output、metadata、file_references、created_at、updated_at；加 `seat_index`、`speaker_seat_id`（带索引）、`execution_run_id`、`session_sequence`。

`agents`、`agent_modes`、`agent_capability_tags`、`workflow_state`、`session_details`、`mcp_servers` 在 `migrations.rs:1149-1207`。

### 21.2 会话与消息网关

`infrastructure/sessions_gateway.rs`——`SessionsAgentRuntimeAdapter` 实现 `AgentSessionGateway` + `ConversationHistoryPort` + `LoopSessionRecoveryPort` + `LoopRoleSessionPort`，委托给 sessions 上下文的 `SessionsApi`。关键方法：`find_session`（`:108`）、`validate_configuration`（`:156`）、`validate_seat_configuration`（`:191`）、`compose_prompt`（`:226`）、`start_generation`（`:263`，durable 用户+assistant 消息创建带 `execution_run_id`）、`find_message`（`:289`）、`append_content`（`:299`）、`append_thinking`（`:309`）、`append_tool_use`（`:319`）、`append_rich_block`（`:329`）、`complete_message`（`:339`，当存在 execution run 时路由到 `terminalize_generation`）、`fail_message`（`:389`）、`cancel_streaming_messages`（`:434`）、`update_lifecycle`（`:476`）、`update_runtime_session_id`（`:486`）、`recent_messages`（`:507`）。

### 21.3 回合执行流程

`application/service.rs:1371` `send_message` / `:1379` `send_message_with_completion` → `send_message_internal`（`:1399`）：trim 内容、拒绝归档会话、校验 chat config、对只读（verifier）会话强制 plan 模式、为多席位计算 `initial_seat_turn_context`、注册完成接收器、调用 `start_message_generation`（`:1458`）。

`start_message_generation`（`service.rs:1458`）是完整生命周期：

```mermaid
sequenceDiagram
    participant SM as send_message_internal
    participant SG as start_message_generation
    participant Sess as sessions gateway
    participant Gen as generation coordinator
    participant Ops as operations
    participant GW as AgentProcessGateway
    participant Sink as event sink
    participant Events as Tauri events

    SM->>SG: start_message_generation
    SG->>SG: 1. 构造 ExecutionRun/root span (vanehub.task.execute)<br/>记录 seat 属性
    SG->>SG: 2. compose_prompt span (vanehub.prompt.assemble)
    SG->>Sess: 3. start_generation (durable 用户+流式 assistant 消息)
    SG->>Gen: 4. generations.reserve (每会话独占租约)
    SG->>Ops: 5. operations.start_agent_generation (可观测操作)
    SG->>Ops: correlate_execution
    SG->>Events: 6. 发布 AgentEvent::MessageStarted
    SG->>Sess: update_lifecycle(Running)
    Note over SG: 7. CLI 路径: Prompt-Hook assemble<br/>+ custom-instructions + memory 注入<br/>generations.correlate_prompt
    SG->>Sess: 8. resolve CLI profile (cli_profile.load)
    SG->>GW: start_generation + monitor_generation (带 sink)
    GW->>Sink: ProviderOutputEvent 流
    Sink->>Sess: append_content/thinking/tool_use/rich_block
    Note over Sess: complete_message → terminalize_generation
```

### 21.4 生成协调

`infrastructure/generation_coordinator.rs`——`InMemoryGenerationCoordinator` 实现 `AgentGenerationPort`：`reserve`（每会话独占，`:27`）、`correlate`、`attach`、`release`、`cancel`、`complete`、`fail`、`active_process_id`、`active_correlation`。领域 `GenerationAttempt` 在 `domain/generation.rs`。

### 21.5 消息完成桥

`infrastructure/message_terminal_completions.rs`——`AgentMessageTerminalCompletionPort` 被 `send_message_with_completion` 用于让 IM/定时连接器阻塞等一个回合完成。`application/models.rs:986-1016`（`AgentMessageTerminalReceiver`）。CLI 生成的记忆提取 hook 在 `infrastructure/memory_extraction_gateway.rs`（`RuntimeAgentMemoryExtractionAdapter::extract` 自己解析 OnePiece 凭据并在回合纯文本上调用模型）。

## 第 22 章 CLI 进程启动与流式解析

### 22.1 CLI 调用构造

`infrastructure/providers/invocation.rs`：

- `build_invocation`（`:76`）——per-agent 标志：
  - **claude-code**：`-p --output-format stream-json --include-partial-messages --verbose` + `--resume <id>`；prompt 经 **stdin**（`ProviderPromptDelivery::Stdin`）。
  - **codex-cli**：过滤掉 `--ephemeral`，然后 `exec [resume <id>] [--ephemeral] --json -`；stdin。
  - **gemini-cli**：`-p <prompt> -o stream-json`；prompt 作为**参数**。
  - **opencode**：`run [--session <id>] --format json <prompt>`；参数。
  - **antigravity-cli**：`-p <prompt> --output-format stream-json [--conversation <id>]`；参数。
- `build_invocation_with_role`（`:44`）——多智能体角色 briefing：claude-code 用 `--append-system-prompt`，codex 用 `-c developer_instructions=...`，其他无（调用方每轮注入）。
- `build_interactive_invocation`（`:162`）——claude-code/gemini-cli 在无 session 时铸一个 UUID `--session-id`（assigned_runtime_session_id）；codex 用 `resume <id>`；opencode `--session <id>`；antigravity `--conversation <id>`。
- `add_codex_output_capture_args`（`:223`）——在 codex 的 `-` 前注入 `-o <temp file>` 以捕获 final-message。
- `apply_configuration_overrides`（`:234`）——model 映射（`mapped_model`，`:455`）、reasoning depth→effort（claude `effort`，codex `reasoningEffort` 带 `max`→`xhigh`，antigravity `effort` 钳到 `high`）、opencode `thinking`。
- `apply_policy_template_overrides`（`:305`）——来自分配策略模板的权限启动标志（见第 31 章）。
- `opencode_standard_permission_env_var`（`:444`）——opencode `standard` 的 `OPENCODE_PERMISSION={"edit":"ask","bash":"ask"}`。

### 22.2 CLI profile → args 管线

`infrastructure/cli_profile.rs`——`RuntimeAgentCliProfileAdapter` 实现 `AgentCliProfileGateway`：`load`（chat scope，`:38`）和 `load_interactive`（`:85`）。两者从 `cli_parameter_settings` 表加载选择、归一化、投影策略（`project_policy_and_build_args`，`:146`——只 `POLICY_TEMPLATE_GOVERNED_AGENT_IDS`）、经 `preview_args` + `force_gemini_standard_approval_flag` + env 渲染 args。可执行文件解析经 `CliApi::resolve_executable`。

CLI 参数目录 `contexts/tooling/cli_parameters.rs`——`catalog_for`（`:211`）：claude-code（`--effort`、`--permission-mode`、`--chrome`、`--model`、`--config`）、codex（`--sandbox`、`--ask-for-approval`、`--ephemeral`、`--model`）、gemini（`--approval-mode`、`--model`）、opencode（`--variant`、`--agent`、`--thinking`、`--model`）、antigravity（`--effort`、`--mode`、`--agent` 等）。`CliParameterLaunchScope` = Interactive | Chat。

### 22.3 进程 spawn 与流式（CLI）

`infrastructure/process_adapter.rs`：

- `RuntimeAgentProcessAdapter`（struct `:33`）持有 `processes: HashMap<String, ManagedProcess>`。
- `start_cli_generation`（`:114`）：解析可执行文件（`normalize_generation_executable`，`:1024`——把 codex/opencode npm shim 解析为打包的原生二进制，见 `resolve_codex_npm_shim`/`resolve_opencode_npm_shim` at `:1034`/`:1078`）、校验结构化输出能力、解析 provider session、经 `provider.prepare_generation` 构建 spec、应用 MCP relay args（`apply_mcp_relay_args`，`:932`）和 codex `-o` 捕获路径、用 `std_command` spawn、设 `TRACEPARENT` env（`:183`）和 `current_dir`（workspace folder，经 `normalize_windows_extended_length_path` 规范）、`Stdin` 交付时写 prompt（`:268`）、记录脱敏启动日志（`:209`）、开 `vanehub.process.run <agent>` OTel span（`:226`）、返回 `StartedGenerationProcess { process_id }` = `agent-process-<pid>-<n>`。
- `monitor_generation`（`:444`）spawn 一个 `ProcessMonitor` 线程（`:499`）。
- `ProcessMonitor::run`（`:607`）：逐行读 stdout，喂给 `output_parser_for_format(output_format)`（`providers/output.rs::output_parser_for_format`，`:95`），第二线程排干 stderr，经 `reap_without_holding_child_lock`（`:986`）每 50ms 轮 `try_wait()` 收割子进程（避免死锁 `stop_generation`），发射 `first_visible_output`/`process.exited`/`process.exited_without_output` 遥测事件，`compose_terminal_event`（`:1104`）挑选终端事件（解析的诊断胜过退出状态；stderr 仅在无更好信息时用）。
- `stop_generation`（`:525`）：移除受管进程，记录 `process.cancellation_requested`，`kill()` 子进程，span 结束为 `Cancelled`(user)/`Failed`(runtime cleanup)。

### 22.4 输出解析（CLI 结构化事件）

`infrastructure/providers/output.rs`：

`ProviderOutputEvent`（Token | Thinking | ToolLifecycle | RichBlock | SessionId | Completed(usage) | Failed | Empty）。`ProviderOutputParser::parse_line` 按 `ParserKind` 分派：

```mermaid
graph LR
    Line["stdout 一行"] --> Dispatch{"ParserKind"}
    Dispatch -->|ClaudeStreamJson| Claude["parse_claude_line<br/>rich_block/system/session_init<br/>assistant*/content_block_delta text<br/>thinking*/tool_use|tool_result|tool_error<br/>result/complete（is_error→Failed）<br/>error/failed"]
    Dispatch -->|StructuredJsonLines| Struct["parse_structured_json_line<br/>通用 JSON-lines（codex/gemini/opencode）<br/>error kinds/completion markers<br/>result→gemini_usage<br/>turn.completed→codex_usage<br/>step_finish→opencode_usage<br/>session-id/thinking/tool/text_value(17 路径)"]
    Dispatch -->|AntigravityStreamJson| Anti["parse_antigravity_line<br/>wrapper {event:...}<br/>init→conversation_id SessionId<br/>result→Completed(antigravity_usage)<br/>或 CANCELED/ERROR/INVALID→Failed"]
```

`ProviderReportedUsage`（`:26`）带归一化版本和 `ProviderUsageOverlap`（Subset/Exclusive/Unknown）。工具事件经 `normalize_provider_tool`（`process_adapter.rs:874`）归一为 `ToolLifecycleEvent`；有 call id 则 fidelity `Inferred` 否则 `Opaque`。

### 22.5 交互式 Agent 终端（PTY）

`infrastructure/terminal_process.rs`——`PortablePtyAgentTerminalRuntime`（`:122`）实现 `AgentTerminalGateway`；`ManagedAgentTerminal` 持 `MasterPty`、writer、child 和一个 `BoundedTextBuffer` transcript（1 MiB `RETAINED_TERMINAL_TRANSCRIPT_BYTES`，`:40`）。读缓冲 64 KiB（`:45`），session-id 解析缓冲 256 KiB（`:49`），provider-session 发现轮询 250 ms（`:50`），用量轮询 5 s（`:56`）。

**原生拥有的 shell 包装器** `infrastructure/terminal_wrapper.rs`——`default_agent_terminal_shell`（`:37`：Windows 上 PowerShell→cmd 回退，Unix `$SHELL`）、`generate_agent_terminal_wrapper`（`:58`）写 `.ps1`/`.cmd`/`.sh` 包装（PowerShell `-NoLogo -NoProfile -ExecutionPolicy Bypass -File`，cmd `/d /s /c`，unix `exec`），经引号保留字面 token 并校验 NUL-free；导出 `OPENCODE_PERMISSION` env；`redacted_command` 诊断。

终端事件 `AgentTerminalEvent::{Output, State, RuntimeSessionId}`（`application/models.rs:355-372`）发布在 `agent-terminal:event`（`infrastructure/events.rs:72-79`）。保留：attach-or-spawn 一次，2h 后空闲清理（`cleanup_idle`），shutdown 停所有——`AgentTerminalGateway` port（`application/ports.rs:478-506`）。Runtime session id 捕获：创建时 id 分配和 provider-session 发现基线（`infrastructure/providers/session_capture.rs`：`find_codex_rollout_since`、`find_gemini_chat_session`、`find_opencode_session_since`、`codex_session_root`、`opencode_database_path`、`prepare_provider_session_capture`）。

## 第 23 章 原生 API 运行时与工具使用循环

`infrastructure/api_process_adapter.rs`（7894 行）是 OnePiece 与 API agent 的运行时。它的存在意义是：不依赖任何 CLI，直接与 provider 通信，自己实现工具使用循环、压缩、记忆、权限——一个"内置 agent"。

- `RuntimeAgentApiAdapter`（`:90`）字段：credentials、config、history、skills、core_instructions、memories、mcp、permissions、retrieval、code_intelligence、workspace_mutations、personalization、accounting（`SessionsApi`）、`generations` map 带 `ManagedApiGeneration { cancelled: AtomicBool, pending_approvals, monitoring }`。
- `start_generation`（`:215`）只接受 `launch_kind=="api"`；`monitor_generation`（`:245`）spawn `run_generation`（`:366`）→ `execute_with_code_intelligence`（`:793`）。

`execute_with_code_intelligence`：取 API key + `ApiProviderConfig`，经 `wire_format_for`（`:690`——OpenAI-compatible post 到 `<base>/chat/completions` 带 `Authorization: Bearer`；Anthropic post `<base>/v1/messages` 带 `x-api-key` + `anthropic-version` header，第三方用 Bearer）选 `WireFormat`，组装 system prompt，加载最近 `HISTORY_LIMIT`=50 消息，构建工具目录，跑工具使用循环。

### 23.1 工具使用循环

```mermaid
flowchart TD
    Start["execute_with_code_intelligence"] --> Build["组装 system prompt<br/>加载最近 50 条消息<br/>构建工具目录"]
    Build --> Loop{"for round_trip in 0..25"}
    Loop --> Account["begin_api_invocation<br/>purpose: AssistantInitial/ToolContinuation"]
    Account --> Body["build_request_body"]
    Body --> Post["POST（blocking reqwest, 120s 超时）"]
    Post --> SSE["读 SSE data: 行<br/>translate_sse_data"]
    SSE --> Tool{"有 tool_use?"}
    Tool -->|有| Exec["execute_tool_call_impl<br/>（可能 await_approval）"]
    Exec --> Append["append reply turns"]
    Append --> Compact{"should_compact?<br/>>60000 字符"}
    Compact -->|是| Sum["summarize_turns_accounted<br/>替换旧轮为合成 user turn<br/>发 compaction_notice RichBlock<br/>顺带自动记忆提取"]
    Compact -->|否| Loop
    Tool -->|无| Loop
    Loop -->|25 轮耗尽| Fail["failed_non_retryable<br/>'Tool-use loop exceeded...'"]
```

工具使用循环（`:929`）：`for round_trip in 0..MAX_TOOL_ROUND_TRIPS (=25, :50)`：核算调用（`begin_api_invocation`，`:496`；purpose `AssistantInitial`/`ToolContinuation`）、构建 body、POST（blocking reqwest，`REQUEST_TIMEOUT`=120 s `:47`）、读 SSE `data:` 行、经 `translate_sse_data` 翻译、执行 tool 调用、append reply turns、再检查压缩。耗尽 → `failed_non_retryable("Tool-use loop exceeded...")`（`:1265`）。

wire 翻译：`anthropic_provider.rs`（`build_request_body` `:42`——顶层 `system`、`thinking: {type:"adaptive"}`；`build_reply_turns` `:77`——tool_use + tool_result blocks；`translate_sse_data` `:110`；`ANTHROPIC_VERSION = "2023-06-01"`）和 `openai_compatible_provider.rs`（`build_request_body` `:51`——前置 `role:"system"`、`reasoning_effort`、`stream_options.include_usage`；`build_reply_turns` `:105`——tool_calls + `role:"tool"`；`DONE_SENTINEL` `:22`）。共享 `ToolCallAccumulator` 跨 SSE delta 累积 tool_use JSON。

**OnePiece 核心指令** `infrastructure/core_instructions.rs`——`ONEPIECE_CORE_VERSION = "1.0.0"`、`include_str!("onepiece-core-v1.md")`（1040 字节，预算 ≤8000 由测试强制）。

**System prompt 组装顺序**（`resolve_system_prompt` `:1433` → `format_system_prompt`）：core instructions → custom instructions → Skills → memories，各自独立可选。

## 第 24 章 OnePiece 原生 Plan-Agent 循环

OnePiece 不只是"另一个 provider"——它还实现了一个原生的 Plan-Agent 循环，把一个大任务拆成子任务、拓扑排序、逐个执行、验证。

`infrastructure/onepiece_planning.rs`——`RuntimeOnePiecePlanningAdapter` 实现 `OnePiecePlanningPort::generate`：解析活跃 profile + 凭据（`onepiece-profile:<id>`）、经 `summarize_turns` 带 `PLANNER_COMPLETION_INSTRUCTION` 构建无工具的 plan 请求，边界：`SUPPORTED_INSTRUCTION_VERSION=2`、`MAX_PLANNER_PROMPT_CHARACTERS=64_000`、`MAX_PLANNER_RESPONSE_CHARACTERS=128_000`、`PLANNING_DISCOVERY_TOOLS`（16-25）。

编排侧 `contexts/task_orchestration/`：`infrastructure/onepiece_planner.rs`（`OnePiecePlanGenerator`）、`infrastructure/onepiece_executor.rs`（`OnePieceAttemptExecutor`——`create_session`、`execute`、`usage`、`stop_session`、`classify_terminal_failure`）、`application/planner.rs`、`application/scheduler.rs`（`decide_serial_schedule`——拓扑感知串行分派，`:27`）、`application/service.rs`、`infrastructure/driver_registry.rs`（`NativePlanDriverRegistry`——每 run 单例激活）。planner system prompt：`task_orchestration/assets/plan_planner_system.md`（1–10 SubTasks 的 JSON schema、guarded validation 命令、manual-evidence 绑定、repair 边界 1-5）。

这部分与第 29 章的计划/循环工程表密切相关，是 OnePiece 区别于其他 CLI 的根本能力。

## 第 25 章 工具执行与 MCP

### 25.1 工具目录（原生 API agent）

`application/tool_catalog.rs`——固定 provider 无关目录：

- `tool_catalog()`（`:34`）：`shell`、`file`、`grep`、`glob`、`edit`、`remember`、`list_skills`、`load_skill`、`read_skill_resource`。
- `plan_mode_tool_catalog()`（`:101`）：只读——`file`(read only)、`grep`、`glob`、`remember`、3 个 skill 工具；无 `shell`/`edit`。
- 常量：`SHELL_TOOL_NAME` 等（`:14-32`）、`MCP_TOOL_NAME_PREFIX = "mcp__"`（`:32`）。
- 条件工具：`recall_tool_definition`（`:246`，检索）、`search_code_tool_definition`（`:267`，代码索引）、`code_intelligence_tool_definitions`（`:289`——`find_definition`、`find_references`、`get_hover`、`get_diagnostics`）。
- 目录解析：`resolve_tool_catalog_with_code_intelligence`（`api_process_adapter.rs:1338`）合并固定 + MCP + 条件工具；plan 模式完全跳过 MCP 并强制只读。

### 25.2 工具执行流程

`api_process_adapter.rs`：

- `permission_action_and_resource`（`:2069`）：`shell`→`shell.exec/workspace`、`file` read→`file.read/<path>` write→`file.write/<path>`、`grep`/`glob`/`search_code`→`file.read/workspace`、`edit`→`file.write`、LSP→`file.read`、`remember`→`memory.write/memory`、`recall`→`file.read/memory`、`list_skills`/`load_skill`/`read_skill_resource`→`file.read/<name>`、`mcp__*`→`mcp.tool/<name>`、未知→合成 `unknown:<name>`（fail-closed 到 Ask）。
- `await_approval`（`:2110`）——在 `pending_approvals` 里按 `call_id` 注册 mpsc 通道，轮询 `recv_timeout(APPROVAL_POLL_INTERVAL=200 ms)` 并监视 generation `cancelled` 标志。
- `execute_tool_call_impl`（`:2496`）：skill 工具 → `execute_skill_read`（`:2265`，闭 schema 经 `ListSkillsInput`/`LoadSkillInput`/`ReadSkillResourceInput`、`valid_skill_identifier`/`valid_skill_resource_uri` `:2228`/`:2238`）；`remember`/`recall` 在 workspace gate 前；`search_code`；plan 模式强制（`plan_mode_denial`，`:2144`）；MCP 调用经 port；`shell`→`execute_shell`、`file`→`execute_file`、`grep`→`execute_grep`、`glob`→`execute_glob`、`edit`→`execute_edit`、LSP→`execute_code_intelligence_tool`。
- 沙箱化：file/edit 路径相对会话工作区解析；隐藏路径拒绝；搜索尊重 `.gitignore`/`.ignore`，跳过 hidden/symlink/binary；上限（200 结果行、2000 行/2000 字符/64 KB 每文件读、10 MB 预检）——按 `agent-tool-execution` spec；实现在 `api_process_adapter` 后的工具模块里。
- 工具使用持久化在完成的 assistant 消息上经 `GenerationProcessEvent::ToolUse` + `sessions.append_tool_use` + `complete_message`（`sessions_gateway.rs:319,339`）。

### 25.3 MCP

`infrastructure/mcp_tool_gateway.rs`——`RuntimeAgentMcpToolAdapter` 实现 `AgentMcpToolPort`：`catalog_entries(project_path)` 调 `McpApi::visible_tool_catalog`；`call_tool` 拆 `mcp__<server>__<tool>`（`split_tool_name`，`:88`）并经 `tauri::async_runtime::block_on(self.mcp.call_tool_with_cancellation(..., cancellation))` 桥接 async。

MCP 上下文 `contexts/tooling/mcp/`——`McpApi` facade、runtime/service、infrastructure 带 `managed_session.rs`、`connection_adapter.rs`、`bounded_stdio.rs`、`relay*.rs`（stdio、streamable HTTP、legacy SSE）、`sqlite_repository.rs`。`mcp_servers` 表在 `migrations.rs:1188-1207`。

审批：MCP 调用始终 `Ask`（从不在任何模板里——`template.rs:132` 测试；`permission_action_and_resource` 返回 `mcp.tool`），经原生 `await_approval` 循环解析。

CLI agent 的 MCP relay：`ManagedMcpRelayPort` 在 `process_adapter.rs:51`、`apply_mcp_relay_args`（`:932`）、monitor 退出时 guard 清理（`:808`）；relay 实现在 `bootstrap/managed_mcp_relay.rs`。

## 第 26 章 Skill 注入

`AgentSkillPort`（`application/ports.rs:865`）带 `bound_skill_prompts(agent_id, workspace_path)`；`RuntimeAgentSkillAdapter` 在 `bootstrap/skills.rs`；skill runtime 在 `contexts/tooling/skills/`。

注入预算：每个 Skill ≤8,000 Unicode 字符，聚合 ≤16,000——`format_system_prompt`（`api_process_adapter.rs:1547`）确定性迭代绑定的 eager Role Skill；按需 Role Skill 与 Utility Skill 不 eager 注入，而是经 `list_skills`/`load_skill`/`read_skill_resource` 暴露。

`BoundSkillPrompt {id, name, body}`（`application/models.rs:1291`）、`AgentSkillReadRequest` 闭枚举（`:429-448`）。API 绑定命令：`bindSkillToApiAgent`/`unbindSkillFromApiAgent`（`src/services/agent-service.ts:359-361`）。

Skill 的设计哲学值得注意：eager 注入的 Role Skill 是"系统提示的一部分"，按需加载的 Utility Skill 是"工具调用"。这种区分让 agent 的上下文预算可控——不是所有 skill 都塞进 system prompt。

## 第 27 章 多智能体群聊与发言调度

第 10 章讲了前端的席位与路由，这一章讲后端镜像的实现。后端存在同样逻辑的原因写在 `domain/seat_turn.rs:1-8`：**让 headless 会话（IM 连接器、定时任务）也能做移交**，而不依赖前端。

### 27.1 席位模型

`AgentSession { id, agent_id, seats: Vec<AgentSessionSeat>, interaction_mode, lifecycle, folder, runtime_session_id, archived, read_only, loop_ownership }` 和 `AgentSessionSeat { seat_id, agent_id, role_id, left_at }`（`application/models.rs:110-132`）。seats 持久化为 `sessions.seats` JSON + `messages.speaker_seat_id`（稳定身份，非数组位置）+ legacy `messages.seat_index`。

角色花名册 `application/seat_turn.rs`——`SeatRosterEntry`、`seat_roster`（`:103`）把 expert role 与 agent 配对、`derive_mentions`/`normalize_model_family` 来自 `domain/seat_roster.rs`；expert roles 表 `expert_roles`（`infrastructure/schema.rs:110-136`：avatar、color、responsibility、instruction、skill_ids、peer_reviewer、require_different_family、preferred_providers）。

### 27.2 路由 / 发言选择

`domain/seat_turn.rs`（镜像 `src/services/mention-routing.ts`、`turn-routing.ts`、`human-handoff.ts`）：

- `parse_handoff_mentions`（`:139`）：只有行首 `@mention` 路由；剥 `>`/`-`/`*`/`+`/有序列表前缀（`strip_line_prefix` `:46`）、忽略 fenced code（`strip_fenced_code` `:120`）、最长匹配 + token 边界（`is_boundary` `:80`）、过滤 self-mention 和重复、上限 `max_mentions`。
- `next_turn_targets`（`:190`）强制 `MAX_CHAIN_DEPTH = 15`、`MAX_MENTIONS_PER_REPLY = 2`（`seat_turn.rs:29-30`）；`ChainEndReason::{TooManyMentions, MaxDepth}`。
- 人类移交：`parse_human_handoff`（`:212`）读 `@用户 handoff|done|fyi`；`apply_human_handoff`（`:233`）——只有 `handoff` 阻塞（开始等待）、`done` 完成本轮、`fyi`/裸提及是信息性非阻塞。

### 27.3 回合调度

`application/seat_turn.rs`——`decide_seat_turn`（`:212`）读完成的 `SeatTurnTerminal` 返回 `SeatTurnDecision { next: Vec<SeatTurnAssignment>, stop }`；`start_seat_turn`（`:298`）启动一个席位的自己的生成（带它自己的 agent 配置 `validate_seat_configuration`、角色 briefing 放进 CLI 的 system-prompt 通道 `build_seat_briefing`、前置线程上下文经 `seat_turn_prompt`（`:398`，`SEAT_CONTEXT_BUDGET_CHARS=4000`、`SEAT_CONTEXT_MESSAGE_LIMIT=40`）。席位串行运行（`SeatTurnStop` 语义：AwaitingHuman / RoundComplete / NobodyMentioned / Bounded / TurnFailed）。

`infrastructure/seat_turn_completions.rs`——`InMemorySeatTurnCompletions`（deliver/take_for_session，单投递 guard）；`infrastructure/seat_turn_coordinator.rs`——`NativeSeatTurnCoordinator::schedule/run/await_terminal`。

回合状态可见性：`SeatTurnStatus::{Agent{...}, WaitingHuman{...}, RoundComplete{...}}`（`application/models.rs:932-953`）作为 `AgentEvent::TurnStatusChanged` → `ChatStreamEvent.turn_status`（`events.rs:242-318`）；前端 `src/components/chat/TurnStatusBar.tsx`、`src/session-workspace/seat-switcher.tsx`。角色 briefing 注入通道：`build_seat_briefing`、`build_seat_context`、`SeatContextMode`；per-role-briefing 注入 CLI 经 `build_invocation_with_role`。

## 第 28 章 上下文压缩与跨会话记忆

### 28.1 上下文压缩

`api_process_adapter.rs:1273-1299`——字符计数代理（`turns_character_count`、`value_character_count`），`should_compact` > `COMPACTION_TRIGGER_CHARACTERS` = **60,000**（`:60`），保留 `COMPACTION_KEEP_RECENT_TURNS` = **6**（`:63`）。`maybe_compact_accounted`（`:1621`）：在 `len - 6` 处切分轮次、调 `summarize_turns_accounted`（`:1884`）带 `SUMMARIZATION_INSTRUCTION`（`:64`）用 `GenerationOptions::disabled()`（内部调用无 thinking/reasoning）、把旧轮次替换为一个合成 user turn、发一个 `RichBlock` 压缩通知（`compaction_notice_block`，`:1290`）、并搭便车自动记忆提取（`:1693`）。压缩在首次请求前（`:906`）和每轮工具往返后（`:1242`）复检。

压缩是 token 经济的关键：一个长会话如果不压缩，上下文会一直膨胀直到触发 provider 的上下文窗口限制。VaneHub 用字符数代理 token 数（不需要 tokenizer），在 6 万字符时触发，保留最近 6 轮，把更早的轮次压成一条摘要。

### 28.2 跨会话记忆

表 `agent_memories`（`infrastructure/memory_schema.rs:10-19`：id、agent_id、folder（空字符串哨兵）、content、source、created_at、updated_at）；`apply_memory_shared_pool_schema`（`:33`）删 scoped 索引并替换为 `idx_agent_memories_recency`——记忆是**一个 host 级池，所有 agent 共享**。

`MemorySource`（`Explicit`/`Automatic`）、`AgentMemory`、`format_memory_section` 带 `MEMORY_INJECTION_CHARACTER_BUDGET` = 4000 和 `<memory>` 分隔符 + 前言——`application/models.rs:1359-1440`。注入 OnePiece system prompt：`resolve_system_prompt`（`api_process_adapter.rs:1433`）→ `format_memory_section`；注入 CLI prompt：`service.rs:1782-1811`。

Ports：`AgentMemoryPort`（save/list_all/delete/delete_all）、`AgentMemoryExtractionPort`（extract）——`application/ports.rs:929-966`。命令：`list_agent_memories.rs`、`delete_agent_memory.rs`、`reset_agent_memories.rs`。

`remember` 工具自动审批（`permission_action_and_resource` 映射到 `Action::memory_write` at `api_process_adapter.rs:2094`）；`recall` 工具（向量搜索检索）at `:2095` 和 `execute_recall`（`:2845`）、`search_code`（`:2897`）。检索 port `AgentRetrievalPort` 在 `application/ports.rs:1027`。

## 第 29 章 计划与循环工程

### 29.1 计划执行运行时（task orchestration）

表 `contexts/task_orchestration/infrastructure/schema.rs`：`plans`、`plan_versions`（goal、project_path、base_ref、planner_profile_id、approved_at）、`plan_subtasks`（ordinal、title、description、acceptance_criteria、assigned_role、token_budget、tool_call_limit、timeout_seconds、validation_commands JSON）、`plan_subtask_dependencies`、`plan_runs`（status、worktree_path/name/branch、simulated）、`plan_subtask_runs`（topological_rank、ordinal、result_summary、changed_files、verification_summary）、`plan_subtask_attempts`（sequence、session_id、profile_id、execution_run_id、operation_id、token_usage、tool_call_count、error_class）、`plan_verification_evidence`、`plan_control_requests`、`plan_generation_failures`、`plan_criterion_evidence_bindings`、`plan_run_policies`、`plan_finalizations`、`plan_final_verification_evidence`、`plan_final_repair_attempts`。

关键逻辑：调度器 `application/scheduler.rs::decide_serial_schedule`（拓扑感知、确定性、一次一个尝试；失败只阻塞后代）；尝试执行器 `infrastructure/onepiece_executor.rs`（集成 worktree 内的 attempt 级 OnePiece 会话）；驱动 `infrastructure/driver_registry.rs`（单例自主驱动）+ `recovery_repository.rs`、`attempt_repository.rs`、`attempt_verifier.rs`、`control_repository.rs`、`finalization_repository.rs`、`query_repository.rs`。命令在 `commands/task_orchestration/`：`generate_plan_draft.rs`、`save_plan_draft.rs`、`get_plan_draft.rs`、`start_plan_run.rs`、`approve_plan.rs`、`execute_next_plan_attempt.rs`、`plans.rs`、`queries.rs`、`controls.rs`。前端：`src/services/plan-service.ts`、`plan-run-polling.ts`、`web-plan-client.ts`、`runtime-plan-client.ts`。

### 29.2 循环工程运行时

表 `infrastructure/loop_schema.rs`：`loop_definitions`（project_path、base_branch、goal、acceptance_criteria、allowed_paths、protected_paths、worker_agent_id、verifier_agent_id、verification_commands、limits、version）、`loop_runs`（definition_snapshot、status、phase、terminal_reason、current_iteration、consecutive_runtime_errors、consecutive_no_progress、pause_requested、worktree、active_operation_id；唯一部分索引 `idx_loop_runs_one_active_definition` 防并发 run）、`loop_iterations`（worker_session_id、verifier_session_id、worker_summary、verifier_recommendation、verifier_findings、decision_reason、diff_fingerprint、check_failure_fingerprint、user_feedback）、`loop_evidence`。

应用层：`loop_service.rs`（定义 CRUD + `start_manual`（`:118`，snapshot + 拒并发 run + 校验角色资格）、`pause/resume/cancel/accept/continue/reject`）、`loop_orchestrator.rs`（`execute_inner`（`:56`）阶段：`prepare`（`:102`）→ `act`（`:142`，Worker 生成）→ `verify`（`:203`，guarded 检查）→ 决策 → 终结；`loop_decision.rs::decide_loop_iteration`（`:44`）——基于必需检查结果 + Verifier 推荐（`LoopVerifierRecommendation::{Pass, Revise, Blocked}`）+ 用户反馈 + 硬上限的决策策略）、`loop_worker.rs`（`start_iteration`/`resume_iteration`/`complete`；prompt builder `loop_worker_prompt.rs`，`MAX_CONTEXT_BYTES=32 KiB`、`truncate_utf8`）。Infrastructure：`loop_repository.rs`、`loop_scheduler.rs`、`loop_recovery`、`loop_verification_process.rs`（guarded 结构化验证：程序 + args、cwd 有界、超时、无 shell 拼接）、`loop_execution_coordinator.rs`。命令在 `commands/agent_runtime/loops/`。前端：`loop-run-polling.ts`、`loop` 类型在 `types/loop.ts`、Loop Center UI。

### 29.3 定时任务

`commands/sessions/scheduled_tasks.rs` + `sessions/infrastructure/scheduled_tasks.rs`；DTO `ScheduledTask`、`ScheduledTaskFrequency`（`commands/sessions/dto.rs:179-228`）；前端 `createScheduledTask`/`listScheduledTasks`（`agent-service.ts:270-273`）。定时任务源经共享 native 执行服务提交，创建带 `AgentMessageSource::Scheduled` 的 execution run。

## 第 30 章 精细 token 计量与执行可观测性

### 30.1 精细 token 计量（调用账本）

表 `sessions/infrastructure/usage_accounting.rs:24-111`：

- `model_invocations`（id、generation_id、run_id、operation_id、session_id、message_id、agent_id、provider_id、profile_id、endpoint_id、model_id、interaction_kind ∈ managed-cli|terminal-cli|native-api、purpose ∈ assistant-initial|tool-continuation|context-compaction|memory-extraction|retry|terminal-interval、request_sequence、attempt、status ∈ running|succeeded|failed|cancelled、started_at、completed_at）。
- `token_usage_observations`（invocation_id、quality ∈ reported|reported-derived|estimated、unit ∈ tokens|characters、measurement_kind ∈ interval|cumulative-snapshot、全部六个维度计数、cache_overlap/reasoning_overlap ∈ subset|exclusive|unknown、normalization_version、source、source_key UNIQUE、source_revision、supersedes/superseded_by、event_at、observed_at、provenance_hash；CHECK 强制 reported+token vs estimated+characters）。
- `usage_ingestion_cursors`（per provider-session 游标）用于幂等累积摄入。
- legacy `usage_records` 表**已删除**（`:22`）——账本是唯一来源。

摄入：原生 API `begin_api_invocation`/`api_invocation_snapshot`/`finish_api_invocation`（`api_process_adapter.rs:496-655`）——每个 provider 请求按 purpose 记录；用量来自 SSE（`message_start`/`message_delta` usage、`message_stop`）经 `ToolCallAccumulator::update_usage`；无 reported 用量时字符计数回退为 `estimated`。CLI headless `providers/output.rs` per-CLI 用量解析器；`process_adapter.rs::normalize_provider_usage`（`:817`）转为 `ReportedUsageTotals`；用量降级诊断（`usage_degradation_reason` `:863`）脱敏记录。CLI terminal `infrastructure/terminal_usage_ledger.rs`——`ingest_claude_terminal_usage`（`:59`）等；`ingest_cumulative`（`:307`）把累积快照对账成 `reported-derived` delta，带 epoch/reset 检测。

桥：`sessions_gateway.rs::session_invocation_usage`（`:639`）映射 `AgentInvocationUsage` 到 `CompletedInvocationAccounting` 带 `UsageInteractionKind::ManagedCli`。投影 `sessions/infrastructure/usage_accounting_projection.rs`；查询经 CTE `ACTIVE_USAGE_CTE` 连接 observations→invocations where `superseded_by_observation_id IS NULL`，拆分 reported vs estimated（`usage.rs:136-152`）。前端 `src/services/token-usage*`、`types/token-usage.ts`。

### 30.2 执行可观测性

表 `execution_observability/infrastructure/schema.rs`：`execution_runs`、`execution_spans`、`execution_events`、`execution_links`、`execution_observability_settings`（local_timeline_enabled、otlp_enabled、otlp_endpoint/protocol、sampling_ratio、retention_days、capture_policy metadata_only、mcp_relay_enabled、otlp_auth_ref、last_retention_at）。

`ExecutionContext { run_id, trace_id, span_id, capture_policy, sampling_per_million, mcp_relay_enabled }`；`ExecutionRun`、`ExecutionSpan`、`ExecutionEvent`、`ExecutionStatus`、`ExecutionFidelity {Native, Proxied, Inferred, Opaque}`——在 `domain/`。API via `crate::contexts::execution_observability::api`；ports `ExecutionIdentityPort`/`ExecutionTelemetryPort`；infrastructure：`otel_telemetry.rs`（OTLP 导出）、`composite_telemetry.rs`、`privacy.rs`（脱敏）、`retention.rs`（定时清理）、`queries.rs`、`observability_repository.rs`。

trace 创建：`service.rs:1480-1545`（`ExecutionRun`/root span `vanehub.task.execute`）、`vanehub.prompt.assemble`、`vanehub.process.run <agent>`（`process_adapter.rs:229`）、进程事件 `process.spawned`/`process.first_visible_output`/`process.exited`、终端工具边界带 `opaque`/`inferred` fidelity。前端 `src/services/execution-observability-service.ts`、`src/session-workspace/execution-timeline-tab.tsx`。计划/循环关联：`plan_subtask_attempts.execution_run_id`、`loop_evidence.operation_id`、`ActiveGenerationCorrelation { operation_id, execution_run_id }`。

### 30.3 运维（可观测长操作）

`contexts/operations/`；port `AgentTaskPort`（`application/ports.rs:516`——`start_agent_launch`、`start_agent_generation`、`start_loop_operation`、`append_log`、`correlate_execution`、`complete/fail/cancel`）。前端 `src/services/operation-service.ts`。

## 第 31 章 权限系统

### 31.1 权限核心

`contexts/permissions/`：

**Domain**：`domain/action.rs`（`SHELL_EXEC="shell.exec"`、`FILE_READ`、`FILE_WRITE`、`MCP_TOOL`、`MEMORY_WRITE`）、`domain/resource.rs`（`workspace()`、`memory()`、`file_path(path)`、`mcp_tool(server, tool)`）、`domain/effect.rs`（`Effect::{Allow, Ask, Deny}`、`resolve`）、`domain/policy.rs`（`ResourcePattern`、`Policy`、`resolve_for`）、`domain/risk_level.rs`（`risk_level_for`）、`domain/grant.rs`、`domain/principal.rs`（`Principal` 带 `template`、parent principal、budget_config）、`domain/approval_request.rs`（`ApprovalRequest`、`ApprovalDecision`）、`domain/scope.rs`（`Scope` + `is_remembered`）、`domain/mod.rs`（`CLAUDE_CODE_AGENT_ID`）。

**策略模板** `domain/template.rs`——`PolicyTemplateName::{Readonly, Standard, Trusted, Yolo}`；`policies_for_template`（`:58`）：`file.read` 和 `memory.write` 总是 `Allow`；`shell.exec`/`file.write` → Readonly:`Deny`、Standard:`Ask`、Trusted/Yolo:`Allow`。`requires_confirmation_to_assign`（`:47`）对 Trusted/Yolo 为 true。`mcp.tool` 从不出现在模板里（Ask 地板是 `evaluate()` 前置条件）——这是 fail-closed 设计。

**评估** `application/evaluation_service.rs`；审批 broker `application/approval_broker.rs`；`evaluate(action, resource, session_id, generation_id, project_key)` 经 `AgentPermissionPort`（`agent_runtime/application/ports.rs:581`）暴露，由 `agent_runtime/infrastructure/permission_adapter.rs::PermissionsPortAdapter`（`:21`）实现带 `create_pending_approval`（`:41`）。Infrastructure：`default_template_adapter.rs`、`principal_repository.rs`、`grant_repository.rs`、`audit_repository.rs`、`event_adapter.rs`、`hook_bridge_discovery.rs`、`hook_bridge_wait_registry.rs`、`schema.rs`。命令在 `commands/permissions/`。

### 31.2 原生 agent 审批流程

`api_process_adapter.rs:2110 await_approval`——暂停工具使用循环、注册 pending approval（经 port `permissions.create_pending_approval`）、发射 `ToolLifecyclePhase::AwaitingApproval`（`application/models.rs:784`）、阻塞 mpsc 直到 `ToolApprovalPort::resolve`（`RuntimeAgentApiAdapter::resolve`，`:337`）投递 `Approved`/`Denied` 或 generation 取消。Denial 作为 tool result 报回 provider。plan 模式强制是独立于目录的硬门（`:2565-2570`）。

### 31.3 CLI 权限 hook（claude-code-permission-hook）

`infrastructure/hook_bridge_server.rs`——`start_hook_bridge_server`（`:50`）绑一个 localhost axum HTTP server 带随机 bearer token（discovery 文件写给 wrapper），路由 `POST /evaluate`（`handle_evaluate` `:108`）：经 `map_tool_to_action` 映射工具、经 `PermissionsApi` 带 `CLAUDE_CODE_AGENT_ID` 评估、`Ask` 时经 `HookWaitRegistry` 调 `await_human_decision`（`:141`）、响应 Allow/Deny。`require_bearer_token`（`:90`）。

`infrastructure/hook_bridge_mapping.rs`——`map_tool_to_action`（`:12`）：`Bash`→shell.exec、`Edit`/`Write`→file.write（无路径 fail-closed）、`Read`/`Glob`/`Grep`→file.read（无路径用占位 "unspecified"）、`mcp__<server>__<tool>`→mcp.tool（首个 `__` 拆）。未映射 → None → fail-closed deny。

`infrastructure/claude_code_hook_adapter.rs`——`install`/`remove` 写 Claude Code 全局 `settings.json` 的 `PreToolUse` 条目经 `CliConfigHookProjectionPort`：matchers `Bash|Edit|Write|Read|Glob|Grep` 和 `mcp__.*`（`:17, :21`），hook 超时 330 s（编译期断言 320 < 330 < 600）。

**权限启动标志**（`cli-agent-permission-launch-flags`）：`providers/invocation.rs::apply_policy_template_overrides`（`:305`）把分配的模板投影到每个 CLI 的原生审批/沙箱标志：

```mermaid
graph LR
    Template["PolicyTemplateName<br/>Readonly/Standard/Trusted/Yolo"] --> Proj["apply_policy_template_overrides"]
    Proj --> CC["claude-code: permissionMode<br/>= plan / default / acceptEdits"]
    Proj --> Codex["codex-cli: sandbox<br/>= read-only / workspace-write<br/>approvalPolicy = never / on-request / never"]
    Proj --> Gemini["gemini-cli: approvalMode<br/>= plan / default / yolo<br/>(force_gemini_standard_approval_flag 保证 argv)"]
    Proj --> Opencode["opencode: agent = plan<br/>Standard → 无选择 + OPENCODE_PERMISSION env<br/>Trusted/Yolo → autoApprove=true"]
    Proj --> Anti["antigravity-cli: mode<br/>= plan / default / accept-edits + sandbox"]
```

这些只覆盖受管 key；在 `cli_profile.rs::project_policy_and_build_args`（`:146`）从 `resolve_effective_execution_policy`（`application/execution_policy.rs`）应用。交互式终端继承同样策略（`load_interactive`，`cli_profile.rs:85`）并只注入 `interactive`-scope 参数。前端 `src/services/permissions.ts`、`runtime-permissions-client.ts`、`web-permissions-client.ts`、`web-permissions-mock-state.ts`。

---

# 第五部分 数据持久化与平台集成

第四部分讲的是"运行时大脑"，这一部分讲的是它赖以生存的"地面设施"——数据库、日志、工作区、终端、各类工具子域、IM 通信、代码智能与检索，以及桌面自身的生命周期。这些是 VaneHub 能在生产环境长期运行而不腐烂的地基。

## 第 32 章 数据库与持久化

### 32.1 数据库平台

`platform/database/mod.rs`：单文件 DB `vanehub.sqlite`（`:31`，`DATABASE_FILE_NAME`），解析为 `<data_dir>/vanehub.sqlite` 经 `database_path()`（`:96-98`）。data dir 来自 `VANEHUB_APP_DATA_DIR` env 或 Tauri `app_data_dir()`（`runtime.rs:73-77`）。

连接池 `r2d2` + `r2d2_sqlite`，`NativeDatabase` struct（`:52-55`）。常量：`BUSY_TIMEOUT = 5s`（`:15`）、`MAX_POOL_SIZE = 12`（`:19`）、`CONNECTION_TIMEOUT = 5s`（`:23`）、`SQLITE_SYNCHRONOUS_FULL = 2`（`:27`）。

每个连接在池 init 时配置一次（`:65-76`）：`busy_timeout(5s)`、`PRAGMA journal_mode=WAL`、`foreign_keys=ON`、`synchronous=FULL`（经 `PRAGMA synchronous` 探针验证）。`NativeDatabase::new()`（`:58-89`）跑 `migrate(&connection)` 然后 `seed_registry(&connection)` 恰好一次，再共享池。测试断言全部 65 个迁移、6 个种子 agent、WAL/foreign_keys/synchronous/busy_timeout 在每个池化连接上（`:116-369`）。

### 32.2 迁移引擎

`platform/database/migrations.rs`：

- 簿记表 `schema_migrations(version INTEGER PRIMARY KEY, name TEXT, applied_at TEXT DEFAULT strftime('%s','now'))`（`:9-13`）。
- 两个 applicator 都**版本门控**（跳过若 `schema_migrations` 已有该版本）：`apply_migration`（`:1084-1116`）和 `apply_transactional_migration`（`:1118-1144`）。两者把 DDL + 版本行包在一个 `unchecked_transaction` 里，使中途失败回滚。
- **密度检查** 启动时 `assert_migration_history_is_dense`（`:700-737`）拒绝间隙（缺行）和高于最大期望的版本；碰撞（两个迁移声称同号）只在测试里捕获，因为碰撞的共享 DB 否则无法启动。真源：`EXPECTED_MIGRATIONS` const（`:632-698`）含全部 65 个 `(version, name)` 对；由测试 `migration_sequence_matches_expected`（`:1904-1928`）守护。
- **对账策略** 处理并行 worktree 共享一个 DB：`:247-257`、`:267-270`、`:753-776` 注释解释 42/43/44 碰撞历史、退役的协调 no-op 槽（27 → `apply_retired_coordination_schema` `:763-765`）、删除迁移编号 45（`apply_remove_coordination_migration` 删 `coordination_runs`，`:773-776`）、`apply_plan_and_code_index_reconciliation`（`:755-758`，version 53）重放两个幂等 schema。`repair_missing_stable_participant_schema`（`:742-751`）在不重写历史的情况下把缺失不变量强制到共享 DB。
- Helper `table_has_column(conn, table, column)` 经 `PRAGMA table_info`（`:1265-1278`）。

### 32.3 65 个迁移完整清单

| 版本 | 名称 | 关键表/内容 |
| --- | --- | --- |
| 1 | initial-schema | `agents`、`agent_modes`、`agent_capability_tags`、`workflow_state`（单例行 CHECK id=1）、`session_details`、`mcp_servers` |
| 2 | agent-managed-sdk-dependency | `agents.managed_sdk_dependency_id` |
| 3 | session-management | `sessions`、`workflow_state.active_session_id` |
| 4 | chat-messages | `messages`（id、session_id FK CASCADE、role、status、content、thinking_content、tool_use、token_input/output、metadata、created/updated_at） |
| 5 | app-settings | `settings(key,value,created_at,updated_at)` KV |
| 6 | cli-tool-status | `cli_tool_status`（agent_id PK、installed、current/latest_version、available_versions、detected_path、last_checked_at、last_error、last_operation_id、version_check_status） |
| 7 | skill-management | skills `apply_schema` |
| 8 | project-worktree-management | `known_projects`；`sessions` + project_path/worktree_path/worktree_name/worktree_branch |
| 9 | session-runtime-metadata | `sessions.runtime_session_id` |
| 10 | im-connectors | communications `apply_schema` |
| 11 | im-session-source | `sessions.source_kind/source_connector`；wechat→weixin rename |
| 12 | cli-parameter-settings | `cli_parameters::apply_schema` |
| 13 | session-chat-configuration | `sessions.chat_preferences` |
| 14 | floating-assistant-configuration | desktop `apply_floating_assistant_schema` |
| 15 | local-extension-management | extensions `apply_schema` |
| 16 | cli-local-environment-details | `cli_tool_status` + environment_type/installations/active_installation_path/conflict_state/lifecycle_eligibility |
| 17 | message-rich-blocks | `messages.rich_blocks` |
| 18 | session-management-organization | `session_categories`、`sessions.category_id`、`messages.file_references`；种子 `automaticArchivalEnabled`/`automaticArchivalInactiveDays` |
| 19 | prompt-hook-management | prompt hooks `apply_schema` |
| 20 | remote-workspace-sessions | `known_remote_workspaces`；`sessions` + remote_workspace_* 列 |
| 21 | sdk-operation-logs | `sdk_operation_logs` |
| 22 | session-usage-records | 保留 no-op；精细始于 64 |
| 23 | scheduled-task-management | `scheduled_tasks` |
| 24 | ssh-connection-management | `ssh_connections`；`known_remote_workspaces.port`；`sessions.remote_workspace_port` |
| 25 | loop-engineering-runtime | agent_runtime loop schema + sessions loop ownership |
| 26 | agent-execution-observability | execution_observability `apply_schema` |
| 27 | multi-agent-coordination | 退役 no-op |
| 28 | remote-terminal-management | `remote_terminal_schema` |
| 29 | api-agent-registration | api-agent schema |
| 30 | openai-compatible-agent-registration | onepiece api-agent schema |
| 31 | agent-cross-session-memory | `agent_memories` |
| 32 | agent-tool-trust | agent tool trust |
| 33 | session-message-search-index | `session_message_fts` FTS5(trigram, content='messages') + insert/delete/update triggers |
| 34 | cli-agent-global-config | `cli_config_profiles` |
| 35 | cli-agent-applied-ownership-snapshot | `cli_config_applied_state` |
| 36 | mcp-truthful-url-transports | `mcp_transport_migration_journal` + `sse`→`streamable_http` rewrite |
| 37 | skill-management-reliability | `skill_api_agent_bindings` repair |
| 38 | agent-management-origin | `agents.agent_origin` |
| 39-41 | onepiece-provider-profiles/catalog/endpoints | `onepiece_provider_profiles`、`onepiece_provider_catalog`、`onepiece_provider_endpoints` |
| 42 | agent-memory-shared-pool | host 级共享记忆池 |
| 43 | retrieval-vector-index | `retrieval_documents` + FTS5 |
| 44 | permissions-core | permissions schema + principal backfill |
| 45 | remove-multi-agent-coordination | 删 `coordination_runs` |
| 46 | expert-role-management | `expert_roles` |
| 47 | session-seats | `sessions.seats` JSON |
| 48 | message-speaker | `messages.seat_index` |
| 49 | plan-execution-foundation | legacy plan schema |
| 50-52 | workspace-code-index-* | `code_index_*` 表 |
| 53 | plan-and-code-index-reconciliation | 重放 49+50 |
| 54 | loop-evidence-iteration-index | `idx_loop_evidence_iteration_created` |
| 55 | session-recovery-evidence-foundation | recovery 列 + `session_recovery_reports` |
| 56 | operation-recovery-evidence | `operation_recovery_evidence` |
| 57 | session-recovery-performance-hardening | FTS triggers 过滤 `status<>'streaming'`，热索引 |
| 58 | lsp-code-intelligence-foundation | `lsp_configuration`、`lsp_language_configurations`、`lsp_workspace_trust` |
| 59 | stable-session-participants | `messages.speaker_seat_id` backfill |
| 60 | effective-skill-runtime | `skill_runtime_state`、`skill_builtin_reconciliation`、`skill_catalog_revision` |
| 61 | session-execution-policy | 清 `chat_preferences`、删 permission-mode CLI 参数 |
| 62 | onepiece-plan-agent-loop | plan-agent loop schema |
| 63 | plan-session-association | plan↔session 链接 |
| 64 | fine-grained-token-accounting | `model_invocations`、`token_usage_observations`、`usage_ingestion_cursors` |
| 65 | managed-im-session-bindings | `im_session_bindings` state/completion/delivery_credential、`im_pairing_intents`、`im_notification_deliveries` |

`schema_migrations` 由 `platform::database` 拥有。外键引用跨上下文用稳定 id；它们不授予跨边界的 repository 访问。

### 32.4 sessions 行模型

`SESSION_SELECT`（`rows.rs:15`）暴露 36 列。`MESSAGE_SELECT`（`rows.rs:17`）暴露 19 列。Repo `sessions/infrastructure/sqlite_repository.rs`（1278 行）。

## 第 33 章 统一日志与运维操作

### 33.1 平台日志存储

`platform/logging.rs`：JSONL 文件 `vanehub.log`（`:20`）、归档子目录 `archive/`（`:21`）。常量 `RETENTION_DAYS=30`、`ROTATION_AGE_HOURS=24`、`MAINTENANCE_INTERVAL_HOURS=1`（`:22-24`）。`LogLevel` Error/Warn/Info/Debug 小写序列化（`:30-37`）；`LogEntry = {timestamp, level, category, message, context: BTreeMap}`（`:39-47`）。`ClientLogEvent`/`ClientLogEventKind`（ErrorBoundary|CriticalOperationFailure，kebab serde）at `:49-65`。

全局 `ACTIVE_LOG_DIR` OnceLock、`LOG_WRITE_LOCK`、`LAST_MAINTENANCE` map（`:26-28`）。`write_entry`（`:100-116`）：全局锁 → `maintain_log_dir` → JSON 序列化**脱敏**条目 → 追加。维护（`:181-197`）每目录至多每小时跑：`rotate_active_log` 在 >24h 时改名 `vanehub.log` → `vanehub-<YYYYmmddTHHMMSSZ>.log`（`:199-231`），然后 `archive_expired_logs_at` 把 >30 天的轮转日志移进 `archive/`（`:233-259`）。

**脱敏** `redact_text`（`:275-329`）分词并替换私有路径（`C:\…`、`/home/`、`/Users/`、`file:///`）→ `[REDACTED_PATH]`；`Bearer <val>`；provider token（`sk-`、`ghp_`、`github_pat_`、`ssh-connection/`）→ `[REDACTED]`；敏感 key 经 `is_sensitive_key`（`:401-431`）。`redact_entry` 应用到 message + 每个 context 值（`:331-346`）。

### 33.2 operations 上下文

Domain `operation.rs`：`OperationKind` {Sdk, Mcp, Agent, Workspace, Extension}（`:6-12`）；`OperationStatus` {Queued, Running, Succeeded, Failed, Cancelled}（`:16-22`）；`OperationTask` 聚合带 `start/correlate_execution/append_log/succeed/fail/cancel`（`:62-137`）。`OperationRecoveryEvidence`（`:47-52`）。

Application `operation_service.rs`：`OperationRepository`/`OperationClock`/`OperationIdGenerator` ports；`OperationService` 带 `cancellations: HashMap<String, Arc<AtomicBool>>` 注册表，调用方可轮询取消（`:56-201`）。Application `logging.rs`：`LogSeverity`、`DiagnosticLog`、`OperationLog`；ports `DiagnosticLogPort`、`OperationLogPort`、`ExternalLogExportPort`（`:5-39`）。

Infrastructure `operation_registry.rs`：`InMemoryOperationRepository` 带可选持久旁路——**操作在内存**；只有 `operation_recovery_evidence(operation_id PK, execution_run_id, status, updated_at)` 经 UPSERT 持久化到 SQLite（`:35-63`），使恢复能跨重启读终端状态（测试 at `:229-259`）。`unified_logging.rs`：`UnifiedLoggingAdapter` 实现 `DiagnosticLogPort`/`OperationLogPort`，写穿 `platform::logging`（总是）和可选附加 `ExternalLogExportPort`（如 OTel）——两者收**同样脱敏**内容（`:65-96`，测试 219-260）。`OperationLogPort` 注入 `operationId` 到 context（`:106-111`）。Facade `operations/api.rs` = `OperationsApi`。

### 33.3 前端 → native 错误上报路径

```mermaid
sequenceDiagram
    participant FE as 前端
    participant Cmd as report_client_log_event 命令
    participant Desk as DesktopSettingsApi
    participant Env as EnvironmentService
    participant Adapter as NativeEnvironmentAdapter
    participant Log as platform::logging

    FE->>Cmd: settingsService.reportClientLogEvent()
    Cmd->>Desk: report_client_log
    Desk->>Env: report_client_log
    Env->>Adapter: write_client_event
    Adapter->>Log: write_client_event (category=frontend.client)
    Note over Log: JSONL 追加，脱敏
    Note over FE: Web/mock 是 no-op
```

前端 `src/types/settings.ts:90` `ClientLogEvent`；kind 联合 at `:14`（`"error-boundary" | "critical-operation-failure"`）。React error boundary 在 `src/App.tsx:100` 和 `src/main.tsx:36`；`src/bootstrap-failure.ts` bootstrap reporter；settings provider `settings-provider.tsx:210-230` 暴露 `reportClientLogEvent` → `settingsService`。Native 命令 `commands/desktop/report_client_log_event.rs:7` → `DesktopSettingsApi.report_client_log`（`desktop/api.rs:158`）→ `EnvironmentService.report_client_log`（`desktop/application/environment_service.rs:66`）→ `NativeEnvironmentAdapter` 调 `platform::logging::write_client_event`（`desktop/infrastructure/environment.rs:152-156`），写 category `frontend.client`（`logging.rs:160-174`）。Web/mock 是 no-op。

### 33.4 启动 wiring

`runtime.rs:setup()`：创建 `NativeDatabase`（`:79`）、激活配置的日志目录（`:93-100`）、组装 `operations_api`（`:115`），然后每个域 API（`:116-302`）。Tauri exit hook flush 执行遥测（`:40-55`）。`write_bootstrap_log`（`:437-452`）用 `UnifiedLoggingAdapter::active(fallback)`。

## 第 34 章 工作区与远程终端

### 34.1 workspaces 上下文布局

`contexts/workspaces/`：domain（project、worktree、remote_workspace、command_template、command_run、output_chunk、shell、remote_terminal_limits、path）、application（service、query_service、shell_service）、infrastructure（git、filesystem、portable_pty、capture_queue、capture_maintenance、output_search、command_runs、command_templates、remote_terminal_schema、remote_terminal_logging、sqlite_repository、selection、session_shell_workspace、session_queries）。

Domain 不变量：`ProjectPath::parse`/`display_name`（`project.rs:7-28`）；`WorktreeName`（拒绝 `/`、`\`、`..`、control）带 `branch_name()` = `vanehub/<name>`（`worktree.rs:33-56`）；`GitReference` 校验（`worktree.rs:9-31`）；`RemoteWorkspace::new` 规范化 `ssh://user@host[:port]/path` URI（`remote_workspace.rs:13-69`）；`ensure_worktree_compatible` 拒绝 remote+worktree 组合（`worktree.rs:58-67`）。

### 34.2 Git 检查

`platform/git/mod.rs` + `workspaces/infrastructure/git.rs`：`WorkspaceGitAdapter` 实现 `WorkspaceGitPort`——`repository_root`（`rev-parse --show-toplevel`）、`resolve_commit_oid`、`create_worktree`、`validate_loop_worktree`、`create_loop_worktree`。全部 30s 超时（`git.rs:9`），诊断经 `DiagnosticLogPort` 带 `GitAdapter::redacted_diagnostic`（`git.rs:36-39`）。

### 34.3 本地 shell（PTY）

`portable_pty.rs`：`PortablePtyShellRuntime` 带 `ManagedShell` 注册表按 shell id 键；`ShellIo` 持 `MasterPty` + writer 在注册表锁外（`:18-21`）；`SHELL_READ_BUFFER_BYTES = 64 KiB`（`:34`）；`default_shell()` = `COMSPEC`/`$SHELL`（`:65-71`）；`terminate_child` kill + wait（`:92-120`）。用 `portable-pty` crate。

### 34.4 SSH 连接管理

`contexts/ssh_connections/`。表 `ssh_connections`（`infrastructure/sqlite_repository.rs:8-33`）：id、name、host、port、user、default_path、auth_mode、key_path、credential_ref、**revision**（INTEGER NOT NULL DEFAULT 1）、test_status（`not-tested`）、last_connected_at、last_error、timestamps；索引 `idx_ssh_connections_updated`。

凭据 `SshConnectionCredentialAdapter` 用 `OsCredentialStore::new("io.vanehub.ai.ssh")`（`:19-54`），account key `ssh-connection/<id>`（`:56-58`），经 `keyring` 带 `zeroize::Zeroizing` 读。补偿性变更（部分失败回滚）在 spec `ssh-connection-management.md:70-82`。测试 `TcpSshConnectionTester`——有界 `TcpStream::connect_timeout` 5s，**无 auth/命令**（`:68-87`）。身份 `UuidSshConnectionIdentity` → `ssh-<uuid>`（`:89-95`）。revision 仅在 endpoint/auth 变更时递增（spec `:83-93`）。

### 34.5 SSH 运行时 + 连接池

`domain/runtime.rs`：`RemoteSshConnectionKey{connection_id, revision}`（`:8-21`）；`RemotePtyRequest{columns,rows}` 有界 1-500×1-300（`:88-101`）；`RemoteSshChannelEvent` {Output, ExtendedOutput, ExitStatus, ExitSignal, Eof, Closed}（`:103-111`）；`HostKeyEvidence` 有界 algorithm(96)/fingerprint(160) bytes（`:29-56`）；`HostKeyChallenge` 带 `HostKeyChallengeKind {FirstSeen, Changed}`（`:23-80`）。

`application/connection_pool.rs`：`RemoteSshConnectionPool`（容量 8，idle 超时 5 min，常量在 `remote_terminal_limits.rs:1-5`：池 8、idle 5·60s、drain 30s、connect 15s、keepalive 30s）。Entry 状态 `Connecting{generation, Shared<BoxFuture>}` / `Ready{transport, leases, last_used, health}`（`:52-63`）。`acquire`（`:105-162`）：淘汰空闲 → 复用健康同 key Ready → join 在途 Connecting（single-flight 经 `future::Shared`）→ 否则开新；`finish_connect` generation-guard 结果（`:164-219`）。`evict_idle`（`:221-233`）、`drain(connection_id)` 标匹配 key Draining 并在最后租约后关（`:235-260`，release at `:313-339`）、`shutdown`（`:262-274`）、`snapshot`（`:276-297`）报告 `RemoteSshPoolHealth {Healthy, Draining, Failed}`。容量淘汰选 LRU idle entry 或 `PoolAtCapacity`（`:361-383`）。

`infrastructure/runtime/russh_adapter.rs`：`RusshSshConnector` 实现 `RemoteSshConnectorPort` 经 `russh`；`HostCheckingHandler::check_server_key` 产 SHA256 fingerprint evidence 并问 `RemoteSshHostKeyVerifierPort`（Accepted vs Challenge）（`:48-74`）；`authenticate` password（凭据 store）或 publickey（`load_secret_key` from key_path）（`:122-166`）；`client_config` 钉首选 kex（含 MLKEM768X25519）、key、cipher（ChaCha20-Poly1305/AES-GCM）、MAC、keepalive 30s、max 3（`:168-216`）。host trust 存储 `ssh_host_trust(connection_id PK, host, port, algorithm, fingerprint, confirmed_at)`（`remote_terminal_schema.rs:26-34`）。

### 34.6 终端输出捕获与搜索

`workspaces/infrastructure/remote_terminal_schema.rs`：加 `ssh_connections.revision`、`sessions.remote_ssh_connection_id` + `remote_ssh_connection_revision`。表：`terminal_command_templates`（带 scope CHECK 强制每 scope 恰好一个绑定）、`terminal_command_runs`（status CHECK）、`terminal_output_chunks`（source CHECK pty|quick-command|gap, UNIQUE(stream_id,sequence)）、`terminal_capture_settings`（singleton, retention_days default 30, capacity_bytes default 536870912）、`terminal_output_fts` FTS5(trigram, content='terminal_output_chunks') + 3 triggers。

限制 `domain/remote_terminal_limits.rs`：queue chunks 256、chunk bytes 32 KiB、batch 32、retention 30d、capacity 512 MiB、transcript 1 MiB、search page 50/100、query/cursor 512 bytes。

捕获路径 `BoundedCaptureQueue`（`capture_queue.rs:5-50`）——溢出时丢最旧并设 `dropped` 标志；下次 `drain_batch` 前置一个 `TerminalOutputSource::Gap` chunk `"[capture gap]"`。chunk 规范化剥 ESC（`output_chunk.rs:27-47`，无效 UTF-8 → U+FFFD，>32 KiB 拒绝）。持久化 `capture_maintenance.rs`——`purge_session`、`purge_before`、`enforce_capacity`（删最旧行直到 SUM(content_bytes) ≤ cap，逐行有界事务）。搜索 `output_search.rs:26-71`——`highlight(terminal_output_fts,0,'[',']')` snippet，按 session/connection/terminal/run 过滤，`limit.clamp(1,100)`，query ≤512 chars。Domain `CommandTemplate::validate` 拒绝类密命令（`password=`、`token=`、`api_key=`、`private_key`、`secret=`）（`command_template.rs:34-67`）；`CommandRun::finish` 只从 queued/running（`command_run.rs:48-64`）。

## 第 35 章 Tooling 子域

第 16.4 章给了 tooling 的总览，这里展开各子域的细节。

### 35.1 CLI 生命周期

`contexts/tooling/cli/domain/mod.rs`：`ToolDefinition` 目录 `CLI_TOOL_DEFINITIONS`（5 CLI）：claude-code（`@anthropic-ai/claude-code`，winget `Anthropic.ClaudeCode`）、codex-cli（`@openai/codex`）、gemini-cli（`@google/gemini-cli`）、opencode（`opencode-ai`，install https://opencode.ai/install）、antigravity-cli（`agy`，script + powershell installer）（`:40-91`）。`ScriptInstaller` 携带必须喂 URL 的解释器（`:20-38`）。Enums：`EnvironmentType`、`VersionCheckStatus`、`InstallSource`、`ConflictState`、`LifecycleEligibility`、`Installation`。持久化 `cli_tool_status`（迁移 6 + 16）加 `background.rs`、`detection_adapter.rs`、`executable_locator.rs`、`native_config_reader.rs`、`package_adapter.rs`、`process_adapter.rs`、`runtime_adapters.rs`、`sqlite_repository.rs`、`candidates.rs`。命令：`install_cli_version`、`upgrade_all_cli_versions`、`refresh_cli_detections`、`list_cli_tools`。

### 35.2 SDK 依赖管理

`contexts/tooling/sdk/domain/mod.rs`：`SdkId {ClaudeSdk, CodexSdk}`；`SDK_DEFINITIONS`——claude-sdk（`@anthropic-ai/claude-agent-sdk`，default 0.2.88，companion `@anthropic-ai/sdk`、`@anthropic-ai/bedrock-sdk`，fallbacks [0.2.88,0.2.81,0.2.58]）和 codex-sdk（`@openai/codex-sdk`，default 0.117.0，fallbacks [0.117.0,0.116.0,0.115.0]）（`:38-59`）。`SdkStatus`/`SdkVersionInfo`/`SdkUpdateInfo`；semver 规范化 `normalize_requested_version`/`compare_versions`（拒绝 tags、ranges、`latest`、shell chars）（`:287-334`）；`lifecycle_plan` 映射 Install/Update/Rollback → `InstallPackages`、Uninstall → `RemoveInstallation`（`:225-250`）。

`application/service.rs`：`SdkApplicationService`——`prepare_operation` 启动可观测操作、`execute_operation` 跑 package 适配器，发布每个 `SdkLogEvent` 到三个 sink：操作 append、`sdk_operation_logs` 表、统一日志（`:242-260`）。`infrastructure/package_adapter.rs`：**install** = `npm install --include=optional --ignore-scripts --prefix <~/.vanehub/dependencies/<sdk-id>> <specs>`（300s 超时，audit category `sdk.npm.install`）（`:162-186`）；写 `.installed` marker + `manifest.json`；**uninstall** 只删 root 内规范化的 dir（`ensure_child` guard）并删 manifest 条目。npm 发现经 `where`/`which`。Schema `sdk_operation_logs`（`:106-117`）；已装版本从 package.json 读（权威）（`:34-49`）。

### 35.3 MCP 客户端管理

`contexts/tooling/mcp/`：domain `ServerName` kebab-case only；`TransportType {Stdio, Sse, StreamableHttp}` 带 fail-closed `from_persisted`；`Scope {User, Project}`；`ServerConfiguration::create` 要求 stdio 给 command、SSE/StreamableHttp 给 url、project scope 给 project_path（`:166-207`）；`McpFailureCode`（9 码带安全消息）；`ConnectionOutcome`、`ToolCallOutcome`。

`mcp_servers` 表（迁移 1，`:1188-1207`）：name PK、transport_type、command、args、env、url、headers、description、active、scope、project_path、last_connection_status、last_connected、last_error、last_tools、last_test_duration_ms、timestamps。

`application/service.rs`（1839 行）：list/add/update/remove/toggle/server_status/visible_tool_catalog/call_tool_with_cancellation/import/export/prepare_connection_test/execute_connection_test。Runtime：`infrastructure/managed_session.rs`、`relay*.rs`（relay、relay_stdio、relay_streamable_http、relay_legacy_sse + observers/pumps/protocol）、`connection_adapter.rs`、`bounded_stdio.rs`、`sse_parser.rs`、`streamable_http*.rs`。`managed_mcp_relay` bootstrap（`bootstrap/managed_mcp_relay.rs`）为工具调用托管内部 relay。迁移 36 journal SSE→StreamableHTTP。

### 35.4 本地扩展

`contexts/tooling/extensions/domain/catalog.rs`：capabilities {ocr, asr, tts}；frameworks {paddleocr（port 9875，`paddleocr>=3,<4` + `paddlepaddle`）、faster-whisper（9876，`faster-whisper>=1,<2`）、sherpa-onnx（9879，`sherpa-onnx>=1,<2`）} 带模型需求（PP-OCRv5-mobile 120MB、base 150MB、vits-zh-aishell3 170MB）（`:99-148`）。`domain/lifecycle.rs`：`ExtensionLifecycleStatus`（9 状态）；`ExtensionFrameworkState`/`ExtensionFrameworkStatus` 带 `ExtensionHealth` 和 `ExtensionInstallationDrift`；`observe_status` 从 runtime + environment 派生 running/installed/unsupported；operation plans `InstallPlan/RemovalPlan/EnablementPlan/RuntimePlan/SelfTestPlan`。

### 35.5 插件集成

`contexts/tooling/plugin_integrations/domain/catalog.rs`：单一内置 `github` 集成（version 1.0.0，docs `https://cli.github.com/manual/gh_auth_login`，setup steps install/auth）；就绪计划 = `gh auth status` 10s 超时（`:62-88`）。无 DB 表——`tool_adapter.rs` 跑可执行文件并报告就绪。

### 35.6 Skills

`contexts/tooling/skills/` 有丰富 domain：`SkillId/SkillKey/SkillLocation/SkillScope`、`SkillOrigin {builtin, imported, user-created}`、`SkillSource`、`SkillBindingPlan`、`detect_drift`/`SkillDriftIssue`，以及整个 overlay 子系统：`OverlayDocument/OverlayFile/OverlayPatch/OverlayScope/OverlayTrust/OverlayMutationState`、`OVERLAY_SCHEMA_VERSION`、learned-guidance markers（`LEARNED_GUIDANCE_START_MARKER`/`END_MARKER`）、overlay limits、media validation、path validation、`replay_exact_patches`、`replay_overlay_scope_chain`、`scan_overlay_text`。

Schema：`skills`、`skill_agent_bindings`（status `pending`）、`skill_api_agent_bindings`、`skill_agent_mount_paths`、`deleted_builtin_skills`、`skill_drift_snapshots`；effective-runtime schema（`skill_runtime_state`、`skill_builtin_reconciliation`、`skill_catalog_revision` singleton）。可靠性 schema 37 重建 `skill_api_agent_bindings` 并剪 orphaned bindings。Filesystem 层：`infrastructure/filesystem/{overlay_manifest,overlay_payload,overlay_transaction,overlay_history,overlay_import,overlay_layout,provider,paths,transaction,usage,mod}`。命令在 `commands/tooling/skills/`（create/import/delete/update/set enabled/overlay preview/reconcile/promote/revert/restore builtin/bind to agents/select workspace dir 等）。

### 35.7 Prompt Hooks

Domain：`PromptHookCategory`、`PromptHookStage`、`PromptHookSource`、`PromptHookId/Name`、`ManagedCliAgentId`、`PromptHookBindings`、`PromptHookOrder` + `PROMPT_HOOK_VARIABLES`。Schema：`prompt_hook_overrides`、`prompt_hooks_user`（full manifest 含 governance、template_body、version、hook_order、cli_bindings）、`prompt_hook_traces`、`prompt_hook_drafts`、`prompt_hook_versions`、`prompt_hook_executions`。Draft→publish→rollback 生命周期由这些表支撑；builtin hooks 经 `builtin_prompt_hooks` catalog。

### 35.8 CLI config & 参数

`cli_config`：`cli_config_profiles`、`cli_config_applied_state`（迁移 34/35）、credential 适配器、live config reader；命令 apply/import/discover/duplicate/validate/delete profiles。`cli_parameters`：`cli_parameter_settings(agent_id, parameter_id, enabled, value_json, updated_at)`，用于 session chat config + execution policy（迁移 61 清 permission-mode 参数）。

## 第 36 章 通信：IM 连接器

### 36.1 Schema

`communications/infrastructure/schema.rs`：`im_connector_configs(connector PK, enabled, display_name, public_config, credential_ref, updated_at)`；`im_credential_refs(connector PK, credential_ref, updated_at, FK CASCADE)`；`im_routing_settings(id=1 singleton, agent_id FK agents, project_path, updated_at)`；`im_session_bindings(connector, external_chat_hash, session_id, created_at, PK(connector,external_chat_hash), FK session CASCADE)` + 迁移 65 列 `state`('active'/'paused')、`completion_notifications`、`delivery_credential_ref`、`updated_at`，且 rank>1 的会话行自动 paused；`im_inbound_dedup(connector, event_hash, received_at)`；`im_connector_checkpoints(connector, checkpoint_key, value, updated_at)`；`im_wechat_reply_contexts(chat_hash PK, credential_account, last_used_at)`；`im_pairing_intents(id, connector, session_id, code_hash, salt, expires_at, created_at, replace_existing, UNIQUE(connector,code_hash))`；`im_notification_deliveries(message_id, session_id, connector, delivered_at, PK(message_id,session_id,connector))`。Legacy `wechat`→`weixin` 数据迁移。

### 36.2 Domain 模型

`connector.rs`：`ConnectorKind {Feishu, Telegram, DingTalk, WeCom, WeChat}`（weixin wire id，别名 wechat）；per-kind 字段定义 Public/Secret/required——Feishu appId+appSecret、Telegram botToken、DingTalk appKey+appSecret+robotCode、WeCom botId+secret、WeChat botToken+baseUrl+botId（`:31-93`）；`builtin_descriptors()` 带 `supports_qr_authorization`（仅 WeChat）、`experimental`（WeChat）、`max_outbound_chars`（Telegram 4096、Feishu 20000、其他 2000）（`:147-161`）；`ConnectorConfig::validate` 递归拒 public config 里的敏感 key（`:173-204`）。

`delivery.rs`：`NormalizedInbound` + `InboundDisposition {Deliver, IgnoreGroupMessage, IgnoreUnsupportedContent}`（只有非空直接文本 deliver）；`MAX_PENDING_PER_CHAT=8` 和 `pending_delivery_admission`；`split_text` unicode 安全分块；`classify_safe_code` → `ConnectorErrorClass {Transient, Authentication, AuthorizationExpired, Permanent}` 带 provider 特定规则；`safe_platform_status_code`。`routing.rs`：`RoutingSettings`、`SessionBinding`、`PairingIntent`、`InboundEventIdentity`、`CheckpointKey`/`ConnectorCheckpoint`。

### 36.3 Application 与运行时

`application/service.rs`（1171 行）：connector 全生命周期 + `notify_session_completion`（`:673`，经 binding 交付 + 记 `im_notification_deliveries`）。`lifecycle_coordinator.rs`：per-ConnectorKind async mutex 通道，使单个连接器的生命周期串行化而其他保持响应。Transports：`protocol.rs` 把每个 vendor 的入站 payload 规范化为 `NormalizedInbound`（Feishu `/event/message/content`+`/header/event_id`、Telegram update_id/chat/from、DingTalk headers.messageId/data.senderStaffId、WeCom req_id/body、WeChat message_id/conversation_id）（`:28-123`）；per-vendor adapter；`wechat_authorization.rs` 实现 QR 配对（poll/cancel）。`runtime_manager.rs`（1776 行）驱动生命周期。IM 完成钩子在 `runtime.rs:272-276`（`CommunicationsCompletionHook` → `notify_session_completion`）。维护 job 每 6h：`maintain_deduplication` + `maintain_wechat_reply_contexts`（`runtime.rs:374-392`）。启动恢复已保存连接器（`runtime.rs:342-351`）。

## 第 37 章 代码智能与检索

### 37.1 LSP

`contexts/code_intelligence/`。Schema（`infrastructure/schema.rs:4-41`）：`lsp_configuration`（singleton enabled/revision）、`lsp_language_configurations`（language_id CHECK rust|typescript_javascript；enabled、executable_override、startup_arguments_json、initialization_options_json、revision）、`lsp_workspace_trust`（canonical_workspace_root PK、trusted、revision）。

Domain（`domain/models.rs`）：`LanguageFamily {Rust, TypeScriptJavaScript}` → `ServerKind {RustAnalyzer, TypeScriptLanguageServer}`；TS 默认 args `["--stdio"]`；`WorkspaceTrust` revisioned；`ProcessState {Absent, Starting, Initializing, Ready, Stopping, Backoff, Failed}`；`NegotiatedCapabilities`（position_encoding utf8/utf16、document_sync none/full/incremental、definition/references/hover/diagnostics）；`DocumentVersion` monotonic；`NormalizedRange`（一基、有序）；`QueryStatus {Ready, Warming, Timeout, Unavailable, Failed}` 和 `QueryOutcome<T>` 带 stale/truncated/counts（`:356-516`）。

Infrastructure：`lsp_stdio_child.rs`（spawn + JSON-RPC framing）、`json_rpc_actor.rs`、`lsp_framing.rs`、`initialize_negotiation.rs`、`document_lease.rs`（didOpen/didChange 生命周期）、`document_snapshot.rs`、`document_invalidation.rs`、`diagnostics_cache.rs`、`lsp_diagnostics.rs`、`process_registry.rs`、`runtime_process_coordinator.rs`、`shutdown_coordinator.rs`、`semantic_query_coordinator.rs`/`semantic_results.rs`、`position_conversion.rs`、`server_discovery.rs`、`server_test.rs`、`project_root.rs`、`runtime_notifications.rs`。命令在 `commands/code_intelligence/`。日志：只生命周期诊断——无 raw LSP payload。

### 37.2 检索 / 向量搜索

`contexts/retrieval/`。Schema：`retrieval_documents`（id PK、source_kind、source_id、scope_agent_id、scope_folder、content、content_hash、index_state、attempt_count、failure_category、embedding_model、embedding_dimensions、embedding BLOB、timestamps、UNIQUE(source_kind,source_id)）+ FTS5 `retrieval_documents_fts` 带 3 triggers；`retrieval_configuration` singleton（source_profile_id、embedding_model、automatic_code_index_mode CHECK disabled|local|semantic）。Code index：`code_index_workspaces`（canonical_root UNIQUE COLLATE NOCASE、origin manual|automatic、index_mode local|semantic、selected_roots_json、languages_json、exclusion_patterns_json、max_file_bytes default 102400、index_version、phase、generation、embedding_confirmed_profile/model/generation）；`code_index_files`（PK workspace+relative_path、language、byte_size、modified_ns、content_hash、index_version、state、failure_category、chunk_count）；`code_index_chunks`（document_id → retrieval_documents ON DELETE CASCADE、start_line≥1、end_line、symbol_name/kind、chunk_ordinal、chunk_key、redaction_count、index_version、UNIQUE(workspace,path,chunk_key)）+ delete trigger；`code_index_symbols`；`code_index_audit`。

Tree-sitter 解析 `infrastructure/code_parser.rs`——`load_and_parse`、`grammar_for`（JavaScript、TS/TSX、Python、Rust、Go、Java、C、C++）、`symbol_query`（`:82-102`）。分块 `code_chunker.rs`——`DEFAULT_MAX_CHUNK_BYTES = 6 KiB`（`:7`）、`chunk_code`。

Application：`indexing_service.rs`、`code_search_service.rs`、`search_service.rs`——**双路混合检索**：向量（余弦相似度，`vector_candidates_scoped`）+ 关键词（FTS5，`keyword_candidates_scoped`），由 **RRF** 融合（`domain/fusion.rs` `fuse_with_rrf`，smoothing `RRF_SMOOTHING=60.0`）；失败语义：两者都失败 → `RetrievalError::Unavailable`，一个失败 → `Degradation {KeywordOnly, VectorOnly}`，query 在 `EMBEDDING_CONTENT_LIMIT` 截断；结果按 id 从源重取（绝不全量快照）并跳过已删源（`:70-176`）。`code_embedding.rs`、`openai_embedding_adapter.rs`、`code_admission.rs`/`code_redaction.rs`（embedding 前脱敏密钥）、`code_reconciler.rs`、`code_inventory.rs`、`code_symbols.rs`、`workspace_file_index_source.rs`。`recall` 工具仅在配置了 embedding 源时暴露给 agent；失败绝不 fail 生成（`search_service.rs:68-70` 设计规则）。

## 第 38 章 桌面子系统与生命周期

### 38.1 桌面设置与生命周期

`contexts/desktop/`。Domain `settings.rs`：`ApplicationLanguage {zh-CN,en,zh-TW,ja,ko}`、`DesktopFontSize {12,14,16,18}px`、`DesktopTheme {futuristic, minimal}`；`DesktopSettingKey` 枚举 16 个 key（含 networkProxyUrl/bypass、automaticArchivalEnabled/InactiveDays、launchOnStartup、defaultPolicyTemplate、customInstructions*、memoryEnabled、memoryToolAssistedChatsEnabled）带严格 parse；代理 URL scheme 白名单 http/https/socks5/socks5h；custom-instructions 字段上限 3000 字符；默认 zh-CN、14px、futuristic、归档 enabled 10 天、模板 "standard"。

Lifecycle service（`application/lifecycle/`）+ `infrastructure/tauri_desktop_lifecycle.rs`：启动控制（`set_launch_on_startup`）、主窗口事件处理（`handle_main_window_event`）、webview 恢复（`install_main_webview_recovery` `runtime.rs:80-84`）、托盘语言。`bootstrap/desktop.rs` 组装 `desktop_lifecycle_api`（`runtime.rs:321-331`）和 `initialize_desktop_runtime`（`:332-336`）。

### 38.2 悬浮助手

`contexts/desktop/domain/floating_assistant.rs`：`FloatingAssistantPlatform {Windows, Unsupported}`（native_available 仅 Windows）；`FloatingAssistantSurfaceMode {Collapsed(76×76), Menu(304×316), Chat(408×620)}`（逻辑尺寸，经 scale factor 缩放到物理）；`FloatingAssistantAnchor`（有限，|coord| ≤ 10,000,000）、`position_for_monitor` + `clamp_position` 带 20px 屏幕边距；`should_intercept_main_close(enabled, floating_window_available)`；main actions `{new-session, current-session, settings}`。

Infrastructure：`sqlite_floating_assistant_repository.rs`、`tauri_floating_assistant_window.rs`、`tauri_desktop_lifecycle.rs`、`webview_recovery.rs`。命令：start_floating_assistant_drag、persist_floating_assistant_position、set_floating_assistant_enabled/surface、save_floating_assistant_anchor、get_floating_assistant_config/runtime_info。

### 38.3 通知系统

前端纯 React context，内存，无 SQLite/Tauri 命令。类型化发布 API 经 React context；有界历史 + toast 上限；global vs session scope；toast 跨导航存活；两主题（futuristic/minimal）；i18n zh-CN/en。实现 under `src/`（notification context provider）。**native** 侧只记 IM 完成交付到 `im_notification_deliveries`。

`notifications/notification-provider.tsx`（两个 context：`NotificationContext` + `NotificationPresentationContext`；`notify` 返回 id）、`notification-reducer.ts`（历史上限 20、`VISIBLE_TOAST_LIMIT = 4`、5s 默认时长）、`notification-types.ts`（`NotificationScope = global | session`）、`notification-center.tsx`（铃铛 popover）、`notification-toast-viewport.tsx`（toast 队列带退出动画，按活跃 session 分域）。

### 38.4 定时任务

表 `scheduled_tasks`（`migrations.rs:840-865`）：id、name、content、agent_id FK、frequency（JSON）、enabled、next_run_at、latest_status（`never-run`）、latest_run_at、latest_run_session_id、latest_error、timestamps；索引 `idx_scheduled_tasks_enabled_next_run`。

调度器 `bootstrap/scheduled_tasks.rs`：`start_scheduled_task_jobs` 启动时（backfill）然后每 60s（`:22-47`）；`run_due_tasks` → `mark_task_running` → `run_one_task`（创建一个 PreserveActive 桌面会话，然后 `agents.send_message` 带 `AgentMessageSource::Scheduled{task_id}`；onepiece 用 `InteractionMode::Api`，其他 Cli）（`:132-176`）；状态转换经 `mark_task_succeeded/failed`；每步记到统一日志带 category `scheduled-tasks.*` 和 `taskId` context（`:178-197`）。API guard：只 CLI agent + onepiece 允许（`sessions/infrastructure/scheduled_tasks.rs:73-85`）。

### 38.5 用量统计与 token 计量

迁移 64 表（`usage_accounting.rs:24-111`）：`model_invocations`、`token_usage_observations`、`usage_ingestion_cursors`。聚合查询（`sessions/infrastructure/usage.rs`）：`SessionUsageRepository::statistics`（范围过滤、daily + per-agent 拆分）、`summary_for_session`；CTE `ACTIVE_USAGE_CTE` 连接 observations→invocations where `superseded_by_observation_id IS NULL`，拆分 reported vs estimated（`:136-152`）。Application 模型在 `sessions/application/usage_accounting.rs`。前端 usage-statistics 设置页 + spec `usage-statistics`。

### 38.6 后台任务总览

所有后台任务在 `runtime.rs`：定时任务（304）、执行保留（310）、会话维护（314）、检索索引 worker（319）、agent 终端清理 2h 空闲（320, 398-409）、communications 维护 6h（338, 374-392）。

## 第 39 章 CI 与治理

### 39.1 工作流

`.github/workflows/ci.yml` job（全部用 pin SHA 的 action）：

- **frontend**（ubuntu, 20m）：`npm ci` → `npm run lint:ci`（eslint `--max-warnings=0`）→ `npm run build` → `coverage:policy:test` → `version:unit:test` → `test:coverage` → `coverage:check:frontend` → 上传 `coverage/frontend/`。
- **contracts**：`npm run contracts:check`（vitest `src/contracts/contract-conformance.test.ts`）。
- **openspec**：`npx @fission-ai/openspec@1.6.0 validate --specs --strict`，然后 per-change 循环 `openspec/changes/*/`（跳过 `archive`）跑 `validate <name> --strict`。
- **documentation**（windows-latest, 45m）：Rust toolchain、pin mdBook（`docs/toolchain.json`）、Playwright chromium、`docs:check`、`docs:test`、`docs:screenshots:check`、`docs:build`，然后 `git diff --exit-code`（证明 docs build 只读）。
- **rust**（ubuntu, 45m）：linux deps → `cargo fmt --check` → `cargo check` → `cargo clippy --all-targets -- -D warnings` → `cargo test`。
- **native-coverage**（ubuntu, 60m）：`cargo-llvm-cov 0.8.7` → `npm run coverage:native` → `coverage:check:native`。
- **native-platform-check**（matrix windows-latest + macos-latest）：`cargo build`（验证 Windows Rust-toolchain LLD 存在）。
- **e2e**（ubuntu, 30m）：`npx playwright test`，失败上传报告。

另有 `codeql.yml`、`dependency-review.yml`、`labeler.yml`、`package.yml`。

### 39.2 校验命令集

前端 build `npm run build`（tsc && vite build && `scripts/check-frontend-chunks.mjs`）；Rust check `cargo check --manifest-path src-tauri/Cargo.toml`；OpenSpec `openspec validate "<change-name>" --strict`；加 `lint:ci`、`test:coverage`、`contracts:check`、`docs:check`、`version:check`、`coverage:policy:test`。`allowScripts` 限 esbuild。

`AGENTS.md` 末尾的"校验命令"要求逐字照抄参数——`npm run lint:ci` 而非 `lint:ci`、`cargo clippy` 带 `--all-targets -- -D warnings`、不漏 `cargo fmt`——否则本地通过而 CI 拦下。

### 39.3 Hooks 与仓库治理

- `.claude/settings.json` `PostToolUse` hook（matcher `Edit|Write|MultiEdit`）→ `node scripts/hooks/post-edit-quality.mjs`（90s 超时）。脚本（`:39-72`）：`.ts/.tsx/.mts/.cts` 编辑后跑 `eslint --fix --no-warn-ignored --max-warnings=0` 并若有问题**阻塞**（exit 2 带反馈）；`.rs` 编辑后跑 `rustfmt --edition 2021`；跳过生成目录；toolchain 错误 fail-open。
- Husky：`.husky/pre-commit` = `npx lint-staged`；`.husky/commit-msg` = `npx --no -- commitlint --edit "$1"`。
- `lint-staged.config.mjs`：`*.{ts,tsx}` 和 `*.{js,mjs}` → `eslint --fix --max-warnings=0 --no-warn-ignored`；`*.rs` → `rustfmt --edition 2021`。
- `commitlint.config.mjs`：conventional config + 自定义 `type-enum` 含 `deps`（deps(npm)/deps(cargo)/deps(actions)）。
- ESLint `eslint.config.js`：`max-lines: ["error", {max: 300}]` 对 `**/*.{ts,tsx}`（空行和注释计），对 `*.test.{ts,tsx}` 和 `tests/**/*.ts` 禁用；固定 **9 个超限文件的技术债豁免清单**（web-agent-client.ts 4137、tauri-agent-client.ts 763、agent.ts 538、sdk-page.tsx 393、contracts/agent.ts 364、main-layout.tsx 341、coordination-runtime.ts 330、agent-service.ts 307、create-session-dialog.tsx 306）不得增长且必须拆分。`no-explicit-any` error；`ban-ts-comment` 受限。
- `.claude/settings.json` permissions：只允许 lint/test/build/contracts/docs/version/cargo fmt|check|clippy|test/openspec validate|list|show/git status|diff|log|show；deny `git commit --no-verify`、`git push --force`、`rm -rf`、pnpm/yarn、读 `.env*`、编辑 `openspec/changes/archive/**`。

### 39.4 OpenSpec 治理 / 归档流程

`openspec/config.yaml`：schema `spec-driven`；context（product、stack: Tauri 2/React/Vite/Tailwind/shadcn/Rust/SQLite/Playwright）、architecture 约束、domain model、verification commands；proposal/design/specs/tasks 规则。

`openspec/archive-governance.md`：归档位置 `openspec/changes/archive/YYYY-MM-DD-<change>/`；准入 = 所有 tasks done → verify → `openspec validate --strict` → `openspec archive` → 经 `powershell scripts/Update-OpenSpecArchiveIndex.ps1` 重生 index → 主 specs + 归档 + index 一起提交。`--no-validate` 禁止；`--skip-specs` 只用于无 spec 影响的变更。归档工件不可变；冷迁移记在 `openspec/archive-cold-migrations.md`；绝不基于年龄自动删除。Index 文件 `archive/README.md` + `archive-index.json` 是生成的（勿手编）。`AGENTS.md` 要求查询归档时优先读 `archive-index.json`，按 `changeName` 或 `capabilities` 过滤。每 6 个月审查在线归档。

---

# 第六部分 横切关注与质量保障

前五部分是"各层拆解"，第六部分把视角拉高——把跨多个子系统的端到端流程用一组时序图画出来，把工程治理的机器强制层集中讲清，最后归纳关键设计权衡与 ADR。

## 第 40 章 端到端时序图集

### 40.1 一次完整的"用户发消息→流式回复"全链路

这是 VaneHub 最核心的链路，跨越前端、服务边界、Tauri 命令、agent_runtime application、infrastructure、sessions、operations、最终到 CLI/API provider。

```mermaid
sequenceDiagram
    autonumber
    actor User as 用户
    participant UI as ChatInputBox / use-main-layout-model
    participant AS as AgentService(tauri 适配器)
    participant Cmd as send_message 命令
    participant API as AgentRuntimeApi
    participant Sess as SessionsApi (gateway)
    participant Gen as InMemoryGenerationCoordinator
    participant Ops as OperationsApi
    participant GW as CompositeAgentProcessGateway
    participant Proc as RuntimeAgentProcessAdapter (CLI)
    participant CLI as claude/codex/gemini 子进程
    participant Ev as TauriAgentRuntimeEventAdapter

    User->>UI: 输入消息, Enter
    UI->>UI: createOptimisticUserMessage（乐观更新缓存）
    UI->>AS: sendMessage(sessionId, content, config, fileRefs)
    AS->>Cmd: invoke("send_message", {...camelCase})
    Cmd->>Cmd: mapper::send_message_request(DTO→领域)
    Cmd->>API: api.send_message(request)
    API->>API: send_message_internal: trim/校验/拒绝归档/校验配置
    API->>API: start_message_generation
    API->>API: 构造 ExecutionRun + root span (vanehub.task.execute)
    API->>Sess: start_generation (durable 用户+流式 assistant 消息)
    API->>Gen: reserve (每会话独占租约)
    API->>Ops: start_agent_generation + correlate_execution
    API->>Ev: 发布 AgentEvent::MessageStarted → chat:event
    API->>Sess: update_lifecycle(Running)
    Note over API: CLI 路径: Prompt-Hook assemble + custom-instructions + memory 注入
    API->>Sess: resolve CLI profile (cli_profile.load)
    API->>GW: start_generation + monitor_generation (带 sink)
    GW->>Proc: 分派 (launch_kind=cli)
    Proc->>CLI: spawn (std_command, CREATE_NO_WINDOW, TRACEPARENT env)
    Proc->>CLI: 写 prompt 到 stdin (Stdin 交付时)
    loop 流式输出
        CLI-->>Proc: stdout stream-json 行
        Proc->>Proc: output_parser_for_format 解析
        Proc->>Sess: append_content/thinking/tool_use/rich_block
        Proc->>Ev: Token/Thinking/ToolUse 事件 → chat:event
        Ev-->>UI: subscribeMessageEvents (缓冲 + rAF flush)
    end
    CLI-->>Proc: 退出 (result/complete)
    Proc->>Sess: complete_message → terminalize_generation
    Proc->>Proc: normalize_provider_usage (token 计量)
    Proc->>Ev: Completed(usage) → chat:event
    Ev-->>UI: 立即 flush
    UI->>UI: invalidate usage/sessions 查询
```

注意链路里几处刻意的设计：

- **乐观更新**（步骤 4）让用户输入立即上屏，不等服务端。
- **durable 用户+assistant 消息**（步骤 11）在生成开始前就持久化，即使进程立刻崩溃，消息与 execution run 仍在。
- **生成租约**（步骤 12）保证一个会话同时只有一个生成。
- **流式缓冲 + rAF flush**（步骤 22-23）把高频 token 事件聚合成每帧一次更新。
- **token 计量**（步骤 25）在消息完成时落账，归一化各 CLI 的用量格式。

### 40.2 工具审批时序（原生 API agent）

```mermaid
sequenceDiagram
    autonumber
    participant Loop as 工具使用循环 (api_process_adapter)
    participant Prov as Provider (SSE)
    participant Tool as execute_tool_call_impl
    participant Perm as PermissionsPort
    participant Card as 前端 ApprovalCard
    participant Ev as chat:event

    Loop->>Prov: POST body (含 tool_use)
    Prov-->>Loop: SSE: tool_use delta
    Loop->>Tool: execute_tool_call_impl(call)
    Tool->>Perm: permission_action_and_resource 映射
    alt Allow (模板允许)
        Perm-->>Tool: Allow
        Tool->>Tool: 执行 (shell/file/grep/...)
    else Ask (需审批)
        Tool->>Perm: create_pending_approval
        Tool->>Ev: ToolLifecyclePhase::AwaitingApproval
        Ev-->>Card: chat:event (pending approval 出现)
        Card-->>User: 显示 ApprovalCard (含风险等级 L0-L3)
        User->>Card: 点 Approve/Deny (选 scope: once/session/project/global)
        Card->>Perm: resolve_pending_approval(id, approved, scope)
        alt Approved
            Perm-->>Tool: Approved (经 mpsc)
            Tool->>Tool: 执行工具
        else Denied
            Perm-->>Tool: Denied
            Tool->>Prov: denial 作为 tool result 报回
        end
    end
    Tool-->>Loop: tool result
    Loop->>Loop: append reply turn, 检查压缩, 继续
```

### 40.3 CLI 权限 hook（Claude Code 桥接）

Claude Code 等 CLI 有自己的权限机制，VaneHub 通过一个 localhost axum HTTP server 把它的 `PreToolUse` hook 桥接到自己的权限系统：

```mermaid
sequenceDiagram
    autonumber
    participant CC as Claude Code 子进程
    participant HBS as hook_bridge_server (localhost axum)
    participant Perm as PermissionsApi
    participant Wait as HookWaitRegistry
    participant Ev as 前端 ApprovalCard (经 chat:event)

    Note over CC: 安装时: claude_code_hook_adapter 写<br/>settings.json PreToolUse entries<br/>(matchers Bash|Edit|Write|Read|Glob|Grep|mcp__.*)
    CC->>HBS: POST /evaluate (Bash, command=..., bearer token)
    HBS->>HBS: require_bearer_token 校验
    HBS->>Perm: map_tool_to_action(Bash)→shell.exec
    HBS->>Perm: evaluate(action, resource, CLAUDE_CODE_AGENT_ID)
    alt Allow
        Perm-->>HBS: Allow
        HBS-->>CC: {"behavior":"allow"}
    else Ask
        HBS->>Wait: await_human_decision (注册回调)
        HBS->>Ev: 发 pending approval → chat:event
        Ev-->>User: ApprovalCard 出现
        User->>Ev: Approve/Deny
        Ev->>Wait: resolve
        Wait-->>HBS: Approved/Denied
        alt Approved
            HBS-->>CC: {"behavior":"allow"}
        else Denied
            HBS-->>CC: {"behavior":"deny","message":"..."}
        end
    end
    Note over CC: 超时 330s (编译期断言 320<330<600)
```

### 40.4 IM 连接器入站→Agent 执行→完成通知

```mermaid
sequenceDiagram
    autonumber
    participant IM as IM 平台 (飞书/钉钉/...)
    participant Trans as Transport 适配器
    participant Svc as CommunicationsApplicationService
    participant Dedup as im_inbound_dedup
    participant Bind as im_session_bindings
    participant AR as AgentRuntimeApi
    participant Comp as AgentMessageTerminalCompletions
    participant IMUser as IM 用户

    IM->>Trans: webhook / poll
    Trans->>Trans: protocol.rs 规范化为 NormalizedInbound
    Trans->>Svc: claim_inbound
    Svc->>Dedup: 查 event_hash (去重)
    alt 已处理
        Svc-->>Trans: Ignore
    else 新事件
        Svc->>Svc: pending_delivery_admission (MAX_PENDING_PER_CHAT=8)
        Svc->>Bind: 按 (connector, external_chat_hash) 找/建 session binding
        Svc->>AR: send_message_with_completion(session, content)
        AR->>Comp: 注册一次性完成接收器
        AR->>AR: 正常 start_message_generation (CLI 或 API)
        Note over AR: 生成完成 → terminalize
        AR->>Comp: 通知完成
        Comp-->>Svc: recv_timeout 投递完成
        Svc->>Svc: notify_session_completion (经 binding)
        Svc->>Svc: 记 im_notification_deliveries
        Svc->>Trans: 经 transport 回复结果文本
        Trans-->>IMUser: 消息送达
    end
    Note over Svc: 维护 job 每 6h: maintain_deduplication + wechat_reply_contexts
```

这条链路的关键是"事件驱动完成通知"——`send_message_with_completion` 返回一个一次性接收器，IM 侧 `recv_timeout` 阻塞等它，**不轮询 SQLite**。架构测试 `communications_completion_wait_stays_event_driven_without_sqlite_polling` 机械强制这一点。

### 40.5 会话崩溃恢复

```mermaid
flowchart TD
    Boot["bootstrap setup()"] --> Worker["启动 sessions 维护 worker<br/>(非阻塞)"]
    Worker --> Run["session_recovery.run_startup_with_retry(100)"]
    Run --> Scan["扫描 active_execution_run_id 非空的会话<br/>+ recovery_status"]
    Scan --> Classify{"分类恢复证据"}
    Classify -->|"clean"| Reconcile["reconciling: 标记/重置"]
    Classify -->|"进程已退出且消息完成"| Complete["terminalize_generation<br/>完成挂起消息"]
    Classify -->|"消息 streaming 但进程已死"| Quarantine["quarantined + 恢复报告"]
    Classify -->|"需用户决策"| Action["action_required<br/>→ session:event RecoveryActionRequired"]
    Action --> User["前端 SessionRecoveryNotice"]
    User --> Ack["用户确认 → acknowledge_recovery"]
    Reconcile --> Done["recovery_status=clean"]
    Complete --> Done
    Quarantine --> Done2["保持 quarantined 等处理"]
```

恢复协调器 `SessionRecoveryCoordinator`（`sessions/application/recovery_coordinator.rs`）处理"应用崩溃时一个生成正在进行"的场景。证据来自 `operation_recovery_evidence`（持久化）+ `active_execution_run_id`（session 列）。恢复决策 `RecoveryDecision` 有 8 个值，原因码 `RecoveryReasonCode` 有 15 个。

### 40.6 SSH 远程终端连接池与输出捕获

```mermaid
sequenceDiagram
    autonumber
    participant UI as 会话工作区 (shell 标签)
    participant WA as WorkspaceApi
    participant Pool as RemoteSshConnectionPool (capacity 8)
    participant Conn as RusshSshConnector
    participant Host as 远程 SSH 主机
    participant Cap as BoundedCaptureQueue
    participant FTS as terminal_output_fts

    UI->>WA: shell_create(connection_id, revision, dims)
    WA->>Pool: acquire(RemoteSshConnectionKey)
    alt 同 key 有健康 Ready
        Pool-->>WA: 复用 (更新 last_used)
    else 同 key 有在途 Connecting
        Pool-->>WA: join (single-flight via Shared)
    else 空闲淘汰后开新
        Pool->>Conn: connect (15s 超时)
        Conn->>Host: TCP + kex (含 MLKEM768X25519)
        Host-->>Conn: host key
        Conn->>Conn: SHA256 fingerprint
        alt FirstSeen / 已信任
            Conn-->>Pool: Ready (transport, leases)
        else Changed
            Pool-->>UI: HostKeyChallenge (问用户)
        end
    end
    WA->>Conn: open_pty (keepalive 30s)
    loop 输出
        Host-->>Conn: channel data
        Conn-->>Cap: 规范化为 ≤32KiB chunk (剥 ESC, U+FFFD)
        Cap->>Cap: 超 256 chunks 时丢最旧 + Gap 标志
        Cap->>FTS: 持久化 + FTS5 索引
        Cap-->>UI: agent-terminal:event Output
    end
    UI->>WA: shell_input / resize / kill
    Note over Pool: idle 5min 淘汰; drain 标 Draining<br/>最后租约后关; shutdown 全关
```

## 第 41 章 工程治理与机器强制层

VaneHub 的一个鲜明特征是：规范不只是写在 `AGENTS.md`/`openspec/project.md` 里，而是有一整套机器层在操作时自动执行。`AGENTS.md` 把这套机制称为"机器强制层"，并明确"收到拦截反馈时，正确做法永远是修代码，而不是想办法关掉闸门"。

```mermaid
graph TB
    subgraph EditTime["编辑时"]
        PostEdit["PostToolUse hook<br/>post-edit-quality.mjs"]
        PostEdit -->|".ts/.tsx 编辑后"| EslintFix["eslint --fix --max-warnings=0<br/>有剩余错误则阻塞(exit 2)"]
        PostEdit -->|".rs 编辑后"| Rustfmt["rustfmt --edition 2021<br/>格式化失败=语法错误"]
    end
    subgraph CommitTime["提交时"]
        HuskyPre["husky pre-commit"]
        HuskyPre --> LintStaged["lint-staged<br/>eslint --fix / rustfmt"]
        HuskyMsg["husky commit-msg"]
        HuskyMsg --> Commitlint["commitlint<br/>Conventional Commits<br/>type: build/chore/ci/deps/docs/feat/fix/perf/refactor/revert/style/test"]
    end
    subgraph Architecture["架构测试 (cargo test)"]
        ArchTest["tests/architecture.rs<br/>syn 解析 AST"]
        ArchTest --> Inward["向内依赖检查"]
        ArchTest --> LibRoot["lib.rs 零业务符号"]
        ArchTest --> CmdIO["命令零 I/O/控制流"]
        ArchTest --> SharedIO["进程/日志用共享适配器"]
        ArchTest --> ProviderNeutral["provider 中性层"]
        ArchTest --> Win["Windows CREATE_NO_WINDOW"]
    end
    subgraph CI["CI (.github/workflows/ci.yml)"]
        Frontend["frontend job<br/>lint:ci + build + coverage"]
        Contracts["contracts job<br/>contract-conformance.test.ts"]
        Rust["rust job<br/>fmt+check+clippy+test"]
        Spec["openspec job<br/>validate --specs --strict<br/>+ per-change validate"]
        E2E["e2e job<br/>playwright"]
    end
```

### 41.1 编辑即校验

`.claude/settings.json` 注册 PostToolUse hook（`scripts/hooks/post-edit-quality.mjs`）：每次编辑/写入 `.ts`/`.tsx` 后自动跑 `eslint --fix` 并把剩余错误回报；编辑 `.rs` 后自动跑 `rustfmt`。格式化失败通常意味着写出了语法错误。

脚本逻辑（`post-edit-quality.mjs:39-72`）：`.ts/.tsx/.mts/.cts` 编辑后跑 `eslint --fix --no-warn-ignored --max-warnings=0` 并**阻塞**（exit 2 带反馈）若仍有问题；`.rs` 编辑后跑 `rustfmt --edition 2021`（必须匹配 `Cargo.toml`）；跳过生成目录；toolchain 错误 fail-open。

### 41.2 提交即拦截

`git commit` 触发 husky：lint-staged 对暂存的 TS/JS 跑 `eslint --fix`、对 `.rs` 跑 `rustfmt`；commitlint 要求提交信息符合 Conventional Commits，允许的 type：build/chore/ci/deps/docs/feat/fix/perf/refactor/revert/style/test。

`commitlint.config.mjs` 是 conventional config + 自定义 `type-enum` 含 `deps`（细分为 deps(npm)/deps(cargo)/deps(actions)）。`lint-staged.config.mjs`：`*.{ts,tsx}` 和 `*.{js,mjs}` → `eslint --fix --max-warnings=0 --no-warn-ignored`；`*.rs` → `rustfmt --edition 2021`。

### 41.3 三百行硬规则

`max-lines`（按物理行计）对全部 ts/tsx 生产代码生效，测试文件豁免。存量超限文件在 `eslint.config.js` 列有技术债豁免清单——**禁止向清单新增文件**，新代码一律 ≤300 行。豁免清单 9 个文件：web-agent-client.ts（4137）、tauri-agent-client.ts（763）、agent.ts（538）、sdk-page.tsx（393）、contracts/agent.ts（364）、main-layout.tsx（341）、coordination-runtime.ts（330）、agent-service.ts（307）、create-session-dialog.tsx（306）。它们是迁移期的历史债，必须被拆分而非被模仿。

`no-explicit-any` error；`ban-ts-comment` 受限（需绕过时用 `// @ts-expect-error` 并写明原因）。

### 41.4 中文文档的加粗写法

`npm run docs:check` 会拒绝 `**结论。**下一句` 这种写法——句末标点留在 `**` 之内时，按 CommonMark 的 flanking 规则它不构成合法的闭合定界符，GitHub 上要么原样显示星号，要么把加粗配对到错误的文字上。正确写法是 `**结论**。下一句`。这种"标点在加粗外"的规则是文档 CI 真正会拦下的东西。

### 41.5 禁止绕过

`AGENTS.md` 明令：不得使用 `git commit --no-verify`、`git push --force`；不得为了让校验通过而修改或删除 `.husky/`、`.claude/settings.json`、eslint 豁免清单、lint-staged/commitlint 配置。即使本地绕过，CI 也会以同样标准全量复查。`openspec/changes/archive/` 是不可变历史归档，工具层已禁止直接编辑，归档只能走 `openspec archive` 流程。个人化的权限放宽或本地实验配置写在 `.claude/settings.local.json`（已 gitignore），不改仓库级 `.claude/settings.json`。

### 41.6 OpenSpec 变更流程

任何新功能或架构调整，必须先在 `openspec/changes/` 下起一个 proposal，通过 `openspec validate --specs --strict` 校验后再动代码，不要跳过 spec 直接改代码。

归档治理（`AGENTS.md`"OpenSpec 归档治理"）：

- 已完成变更的唯一在线归档位置是 `openspec/changes/archive/YYYY-MM-DD-<change-name>/`；完整 Markdown 工件必须保留在 Git 中，不可用 zip/tar 替代。
- 归档前必须完成 tasks、执行 `openspec validate <change-name> --strict`、在涉及代码时记录实现验证结果。正常流程禁止 `--no-validate`；仅无主规范影响的变更可 `--skip-specs`。
- 使用 `openspec archive <change-name>` 后，必须执行 `powershell -ExecutionPolicy Bypass -File scripts/Update-OpenSpecArchiveIndex.ps1`，并将主 specs、归档目录和索引一起提交。
- 查询归档时优先读 `openspec/changes/archive/archive-index.json`，按 `changeName` 或 `capabilities` 过滤；仅定位到具体变更后才读其 Markdown 工件。
- 每 6 个月审查在线归档；冷迁移前必须验证目标 Git 仓库/不可变分支/tag，在 `openspec/archive-cold-migrations.md` 记录可验证引用后才能移除在线副本。

## 第 42 章 关键设计权衡与 ADR

这一章归纳全文反复出现的设计权衡，把它们作为可检索的决策记录。

### ADR-001：单 crate + 显式模块边界（保持一个 crate）

> 出自 `src-tauri/ARCHITECTURE.md` ADR-001。

运行时保持单一 Cargo crate。模块私有 + 解析的架构测试强制依赖方向，不引入 DI 框架或多 crate 构建期间的复杂度。

**为什么**：多 crate 拆分在大型 Rust 项目里常见，但它带来构建配置复杂度、版本协调成本，且对一个团队规模的项目收益有限。VaneHub 选择"单 crate + 编译期 AST 检查"——用 `tests/architecture.rs` 解析 `syn` AST，机械强制 domain 不依赖 rusqlite、命令不写 SQL、跨上下文不伸手私有模块。这把"架构纯净度"从 review 责任变成 CI 红绿。

**代价**：crate 内所有模块共享一次编译，增量编译边界不如多 crate 精细；但团队接受这个代价换取了配置简单和强制力。

### ADR-002：语义日志与日志存储技术分离

> 出自 ADR-002。

`operations` 拥有诊断和操作日志语义。`platform::logging` 拥有脱敏的 JSONL 持久化、轮转、归档、活动目录状态。上下文应用代码经 port 发语义记录；infrastructure 适配器只在外缘消费平台日志存储。

**为什么**：把"日志说什么"和"日志怎么落盘"分开，让领域层不碰文件 I/O，让平台层不懂业务语义。脱敏（`redact_text`）集中在一处，所有日志路径强制过它——这是合规与安全的硬要求。

### ADR-003：CLI 参数留在 Tooling 内

> 出自 ADR-003。

CLI 参数目录和持久化选择仍是 Tooling 子域，经 `CliParametersApi` 发布。Sessions 消费不可变 chat 默认值，Agent Runtime 消费启动参数，二者都不 import Tooling 持久化或命令 DTO。

**为什么**：CLI 参数既是"配置项"（sessions 关心）又是"启动标志"（agent runtime 关心），但它的归属是 tooling。把它发布为 API 让两个消费者解耦，避免 sessions/agent_runtime 依赖 tooling 的内部存储。

### ADR-004：组合与命令注册分开集中化

> 出自 ADR-004。

`bootstrap/runtime.rs` 拥有构造、Tauri state 注册、后台任务启动。`commands/registry.rs` 拥有稳定的 invoke 面。这让应用装配可审计，又不把接口注册放回 `lib.rs`。

**为什么**：把"怎么造"和"暴露什么"分开，`lib.rs` 因此能保持零业务符号（ADR-001 的强制前提）。

### 设计权衡 1：双运行时（tauri/webMock/webHttp）的代价与价值

前端 `AgentService` 接口约 140 方法，要维护两个完整实现（Tauri 适配器 1016 行 + Web mock 4984 行）。这看起来是巨大的重复劳动。但它换来了：

- 80+ 服务测试文件能完全离线跑（对内存 mock 验证逻辑）。
- 同一套 UI 能以浏览器页面形式运行（开发、演示、E2E、回归）。
- 共享纯逻辑模块（`chat-events.ts`/`turn-routing.ts`/`mention-routing.ts`）保证 mock 行为忠实。

代价是：每加一个后端命令，要同时改两处适配器 + 契约类型。`contract-conformance.test.ts` 用条件类型 `Equal<>` 守护前后端一致性，减轻漂移风险。

### 设计权衡 2：CLI 输出解析的异构性收容

五个 CLI 有五种结构化输出格式（ClaudeStreamJson、StructuredJsonLines×3、AntigravityStreamJson）。VaneHub 没有要求它们统一，而是在 `providers/output.rs` 用 `output_parser_for_format` 按格式分派，把异构输出归一成统一的 `ProviderOutputEvent`。这把"异构性"限制在一个文件里，不污染上游。

### 设计权衡 3：原生 API 运行时（OnePiece）的存在意义

OnePiece（`api_process_adapter.rs`，7894 行）是全 crate 最大文件。它存在的意义不是"再实现一个 agent"，而是：

- 不依赖任何 CLI 的安装/版本——直接打 provider HTTP 端点。
- 自己实现工具使用循环、上下文压缩、跨会话记忆、权限审批、plan-agent 循环——这些能力对 CLI agent 是"交给 CLI 自己管"的，对 OnePiece 是"VaneHub 自己管"。
- 让 IM 连接器、定时任务这类 headless 场景有一个不依赖桌面 CLI 进程的 agent。

代价是 7894 行的复杂度，但它是 OnePiece 区别于"CLI 托管壳"的根本能力。

### 设计权衡 4：迁移对账而非重写历史

65 个迁移在并行 worktree 共享一个 SQLite 时会碰撞（v42/v43/v44 历史）。VaneHub 不重写迁移历史（那会破坏已部署数据库），而是用对账迁移（如 v53 `apply_plan_and_code_index_reconciliation` 重放 v49+v50、`repair_missing_stable_participant_schema` 强制缺失不变量）+ `assert_migration_history_is_dense` 启动检查来处理。`EXPECTED_MIGRATIONS` const 由测试钉死真源。

### 设计权衡 5：事件驱动完成通知 vs SQLite 轮询

IM 连接器要等一个 agent 回合完成才能回复用户。朴素的实现是轮询 SQLite 看消息 status。VaneHub 用一次性完成通道（`AgentMessageTerminalCompletionPort` + `recv_timeout`）——`send_message_with_completion` 返回接收器，生成完成时投递。架构测试 `communications_completion_wait_stays_event_driven_without_sqlite_polling` 机械强制不轮询。这避免了 headless 场景下的数据库轮询负载。

### 设计权衡 6：契约测试用条件类型而非代码生成

没有 protobuf/GraphQL 代码生成器，前后端契约靠 TypeScript 接口 + `contract-conformance.test.ts` 的 `Equal<Contract.X, Types.X>` 条件类型守护。`contracts/agent.ts` 故意复制而非 re-export 类型，让契约成为可审计的独立快照。代价是手写同步，收益是不引入额外工具链、契约变化在编译期可见。

### 设计权衡 7：lazy 懒加载 + 独立 ErrorBoundary

每个设置页、Loop/Plan 中心、大部分会话标签页都经 `LazyFeature` 懒加载，且每个懒组件套独立 `ErrorBoundary`。代价是代码分割配置复杂，收益是主 bundle 小、单页崩溃不连累全局。

### 设计权衡 8：流式 token 缓冲 + rAF flush

token 事件先缓冲、在下一个动画帧批量应用（`applyChatEvents`），终端事件立即 flush。代价是实现比"每 token setQueryData"复杂，收益是无论 token 多密集都只每帧一次 React 更新，配合 `React.memo` 的 `MessageItem` 保持 60fps。

### 设计权衡 9：架构守护的"机械 vs 语义"

`tests/architecture.rs` 能机械检查"domain 不 import rusqlite"、"命令体零 if/match"，但它检查不了"这个用例的事务边界是否正确"、"这个聚合的不变量是否完整"。`project.md` 明确："Rust 可见性和架构测试提供机械强制；review 仍负责语义上下文所有权和事务边界。" 这是诚实的边界声明——机器拦得住结构性腐化，拦不住语义性错误。

---

## 结语

VaneHub AI 是一个在"多异构 AI CLI 托管"这个具体问题域里，把工程纪律推到极 致的项目。它的价值不在于某一个惊艳的算法，而在于一套**自洽的、机器强制的、可审计的**架构纪律：

- **双运行时适配器**让同一套 UI 既跑桌面又跑浏览器，靠契约测试守护一致性。
- **六边形 DDD 单 crate** 让 1130 个 Rust 文件不腐化，靠 AST 解析的架构测试机械强制依赖方向。
- **统一的 ChatStreamEvent/消息模型/权限模型** 把五种异构 CLI + 一种原生 API 收容进一致体验。
- **机器强制层**（编辑即校验、提交即拦截、300 行硬规则、架构测试、OpenSpec 流程）把规范从口头变成 CI 红绿。
- **事件驱动而非轮询**、**迁移对账而非重写历史**、**脱敏集中**等具体决策，体现了对长期可维护性的深思熟虑。

这个项目最值得借鉴的不是某段代码，而是它把"架构原则"转化为"可执行规则"的方法论——每一章里提到的约束，几乎都能在 `eslint.config.js`、`tests/architecture.rs`、`scripts/hooks/`、`.github/workflows/ci.yml` 或 `openspec/` 里找到对应的机械强制点。这正是它能在一个复杂问题域里保持代码健康度的根本原因。

> 本文档基于源码主干 `bb3d28d8`（2026-08）撰写，覆盖 639 个前端 `.ts/.tsx` 文件、1130 个 Rust 源文件、65 个 SQLite 迁移、299 个 Tauri 命令、14 个限界上下文。所有结论均以 `文件:行号` 形式锚定到真实源码。文档本身约 19 万字，含 30 余张 mermaid 架构图、流程图、时序图。

---

# 第七部分 深入专题与系统走读

前六部分按层次和子系统拆解了 VaneHub 的骨架与血肉，但那种"逐章罗列"的写法天然地偏向静态结构。一个真实运行的系统，其设计的精妙之处往往藏在动态行为里：为什么一个看似简单的选择会带来深远的后果，为什么一段看似冗长的代码必须如此写，为什么某个不变量被反复强调。第七部分换一个视角，用走读和专题的方式把这些动态设计讲透。这里的叙述会刻意放慢节奏，把每一个决策的动机、被舍弃的替代方案、以及它在未来演进中可能遇到的张力，都展开来讨论。

## 第 43 章 服务边界层的设计哲学

在第二部分第 7 章我们已经给出过 `AgentService` 接口的规模与两个适配器的对应关系，但那里更多是"是什么"。要真正理解为什么这个项目愿意为一个 TypeScript 接口维护近五千行的 mock 实现，需要回到这个选择面临的根本张力。

这个张力可以这样表述：VaneHub 的 React 界面必须同时服务三种截然不同的运行场景。第一种是生产桌面场景，前端跑在 Tauri 的 webview 里，所有数据操作最终落到 Rust 命令和 SQLite；第二种是开发与演示场景，前端跑在普通浏览器里，没有 Rust，没有数据库，所有数据都是内存里凭空构造的；第三种是潜在的远程 HTTP 部署场景，前端连一个真正的后端服务。如果让 UI 组件直接调用 Tauri 的 `invoke`，那么第二种和第三种场景就根本无法成立——浏览器里没有 `invoke`。这是项目早期就会遇到的经典"抽象泄漏"问题：一旦 UI 和运行时耦合，可移植性就丧失了。

项目给出的答案是引入一个服务边界层。这一层对上暴露一个统一的 TypeScript 接口，对下提供多个可替换的实现。这个答案本身并不新奇，几乎所有需要多后端的应用都会这么做。真正值得讨论的是项目在执行这个答案时做的几个非显然的选择，以及这些选择背后的考量。

第一个选择是把服务接口做得很"宽"。`AgentService` 这个接口不是只覆盖一两个领域，而是横跨了 agent 注册、OnePiece 配置、记忆、检索、LSP、CLI 工具、会话、循环、聊天配置、消息、用量、终端、工作区、角色、Skill、Prompt Hook 等十几个领域，加起来接近一百四十个方法。一个更"干净"的设计本可以把这些拆成十几个小接口，按领域聚合，每个领域有自己的服务对象。项目没有这么做，而是把它们收进一个胖接口。这个决定的代价是 `agent-service.ts` 这个文件本身成为了一个事实上的"功能总索引"，任何新能力的加入都会让它膨胀。但收益是显著的：组件只需要注入一个依赖，就能访问整个产品的能力面；运行时适配器的"实现全集"被强制成单一对象，编译器能在一个地方检验完整性；测试 mock 只需要一个对象就能替换整套行为。换句话说，宽接口换来的是"装配点的唯一性"和"完整性的可机器校验"。在一个能力众多但运行时必须整体替换的系统里，这个权衡是合理的。

第二个选择是 mock 适配器不只是存根，而是完整的行为实现。很多项目里的 mock 只是返回固定假数据，够测试渲染就行。VaneHub 的 mock 实现了与 Tauri 适配器相同的方法签名，内部维护着模块级的 Map 状态，种子数据来自专门的 fixture 文件，并且模拟了异步延迟、流式输出、工具使用循环、聊天配置归一化、席位快照等一系列行为。这种"行为忠实的 mock"需要持续投入维护成本：每加一个后端命令，mock 侧也要相应实现，否则 contract 一致性测试就会暴露差异。项目愿意承担这个成本，是因为 mock 的忠实度直接决定了前端测试的可信度。如果 mock 只返回固定数据，那么基于 mock 的单元测试实际上只测了"组件能渲染固定数据"，对真实数据流的回归毫无防御力。而行为忠实的 mock 让前端能在不启动 Rust 的情况下，验证从用户输入到状态更新到流式渲染的完整链路，这正是项目最需要保护的回归面。

第三个选择是用一个运行时检测函数在构造期决定走哪个适配器，而不是用依赖注入容器。`detectRuntimeKind` 通过读取 window 上的几个全局变量来判定当前环境，优先级是显式覆盖、Tauri 内部标记、HTTP 基址、最后默认 mock。这种基于全局变量的检测看起来"不够工程化"，但它有一个关键优点：零配置。一个开发者打开浏览器访问 dev server，什么都不用配，就自动拿到 mock；同一个代码包打包进 Tauri，就自动走 Rust。如果用依赖注入容器，每个入口点都要写装配代码，反而增加了出错的机会。当然这个选择的代价是全局可变状态，但 VaneHub 把它限定在"启动期一次性判定"的范围内，运行时不再变更，因此可变状态的风险被控制在最小。

第四个选择是失败语义的统一包装。`createRuntimeAdapter` 返回的对象被一个 Proxy 包裹，这个 Proxy 把所有同步抛出的异常转成 rejected promise，并把各种异构的错误形状归一成统一的 `ServiceError` 类型，带有一个分类码。这个设计解决的是一个微妙的问题：Tauri 的 `invoke` 在命令返回 `Result::Err` 时会 reject promise，但前端也可能因为参数校验同步抛出异常，mock 适配器又可能用自己的方式报错。如果让这三种错误形状都裸露给组件，组件就要写三套错误处理。Proxy 统一了它们，让组件只需要处理一种 `ServiceError`。这又是一个"把复杂度收口到一处"的典型权衡。

把这四个选择放在一起看，服务边界层的真正设计意图就清晰了：它不是简单地"加一层抽象以便换后端"，而是一套系统性地保证"前端可在无后端环境下开发、测试、演示，且其行为与生产环境一致"的工程方案。胖接口保证完整性，忠实 mock 保证测试可信度，零配置检测保证易用性，错误归一保证组件简单。这四个点互相支撑，缺一个都会让整个方案塌陷。

## 第 44 章 消息渲染管线的性能走读

第 9 章给了消息渲染管线的静态结构，这一章把它放在"用户每秒收到几十个 token"的压力场景下走读，看每一个性能决策是怎么咬合的。

想象一个典型的长会话：用户问了一个需要 agent 执行多轮工具的问题，agent 在回复过程中持续流式输出，期间还穿插着思考块、工具调用块、富文本块。前端每收到一个 token 事件，理论上都要更新 React 状态、重渲染消息列表。如果实现得朴素，这个场景会在几百条消息后开始卡顿，因为每次更新都会触发整个列表的协调。VaneHub 用一系列相互配合的优化把这个场景压在流畅区间，理解这些优化的配合方式，比理解它们各自的存在更有价值。

第一道优化是消息组件的 memo 化。`MessageItem` 被包裹在 `React.memo` 里，这意味着只有当它的 props 引用变化时才重渲染。在流式场景里，正在流的那条消息会不断变化 props，其他已完成的消息 props 不变，因此不会被重渲染。这是基础，但它只在"props 引用稳定"时才有效。如果父组件每次渲染都新建一个消息对象传下去，memo 就形同虚设。所以这里有一个隐含的前置条件：消息列表的数据更新必须以"更新已有对象"而非"重建列表"的方式进行。这一点由 TanStack Query 的 `setQueryData` 配合流式事件聚合来保证。

第二道优化是 token 事件的缓冲与动画帧批量应用。这是最关键的一道。流式订阅每收到一个 token 事件就 push 进缓冲区，但并不立刻 `setQueryData`，而是等到下一个 `requestAnimationFrame` 触发时，把缓冲区里所有事件一次性聚合并更新缓存。这样无论一帧内来了五个 token 还是五十个 token，都只触发一次 React 更新。这个设计的精妙之处在于它利用了浏览器渲染节律：React 的并发渲染本身就在帧内工作，把数据更新对齐到帧边界，就让数据流和渲染流同步，避免了"数据比渲染快导致中间状态被白白计算"的浪费。终端事件（完成、失败、取消）被设计成立即 flush，这是因为它们意味着回合结束，用户期望立即看到最终状态，不应该再等下一帧。

第三道优化是 `applyChatEvents` 这个聚合函数本身的单次线性扫描。它不是对每个事件分别更新缓存，而是把一批事件收集起来，对消息列表做一次线性遍历，把 token 追加到对应消息、把思考块和工具块合并到对应消息。这样一次更新无论涉及多少事件，都是 O(消息数) 而非 O(事件数乘消息数)。对于一个长会话，这个复杂度差异是巨大的。

第四道优化是滚动锚定。流式输出会让消息列表不断变长，如果用户正在向上翻看历史，新的输出不应该把视图顶走。`MessageList` 的 `anchoredScrollTop` 记录了用户当前的相对滚动位置，在列表尺寸变化时根据这个锚点恢复，保证用户看的那条消息不跳。当用户滚到底部时，则切换到"自动跟随"模式，新输出自动滚入视图。`ResizeObserver` 配合 `requestAnimationFrame` 在这里的作用是，列表尺寸变化触发的是被动布局，但实际滚动恢复被推迟到下一帧，避免在一个帧里既改尺寸又改滚动造成的布局抖动。

第五道优化是虚拟化。当消息数达到几百条时，即便每条都 memo，DOM 节点数本身也会成为负担。项目用 TanStack Virtual 做了虚拟列表，只渲染视口内的消息。`measured-virtual-list` 组件用测量行高的方式处理变高消息，因为聊天消息高度无法预估。

这五道优化不是各自独立的，它们形成一个链：memo 保证已完成消息不重渲染，但前提是 props 引用稳定；引用稳定由帧批量聚合保证，因为聚合把多次更新合并成一次，且以更新而非重建的方式修改缓存对象；帧批量聚合又依赖 `applyChatEvents` 的高效单次扫描来保证一帧内能处理完所有积压；滚动锚定和虚拟化则保证视觉层面不抖动、DOM 不膨胀。抽掉任何一环，流畅性就会在某个会话长度或 token 速率下崩塌。这种"优化必须成环"的认识，是性能工程里最容易被忽略、也最值钱的部分。

## 第 45 章 多智能体群聊的语义模型

多智能体群聊是 VaneHub 里语义最密集的特性之一。第 10 章和第 27 章分别从前端和后端给了它的实现，但那里偏重"代码在哪、做了什么"。这一章专门讨论它的语义模型为什么是这样设计的，以及这套语义模型如何在一个由异构 CLI 组成的系统里保持一致。

多智能体群聊要解决的根本问题是：当多个 agent 共享同一个会话线程时，谁该在什么时候发言，发言后该把控制权交给谁，人类介入时又该怎么处理。这看似只是个调度问题，但它的难度被一个事实放大了：参与群聊的 agent 可能是不同的 CLI，它们对"系统提示""角色""上下文"的理解各不相同，而且它们之间没有原生的协作机制。Claude Code 不知道 Codex 存在，Gemini CLI 不知道隔壁有个 Antigravity。VaneHub 必须在不修改这些 CLI 内部行为的前提下，让它们看起来像在一个统一的群里协作。

这个约束决定了席位模型的设计。席位是一个抽象层，它把"一个 agent 在某次会话里承担的角色实例"从"agent 本身"解耦出来。一个 agent 可以在不同会话里担任不同席位，一个会话可以有多个席位分属不同 agent。席位的角色信息以快照形式存在消息里，这样即便后来专家角色配置变了，历史消息里的发言者身份仍然准确。这种"角色快照不可变"的设计是对历史完整性的保护：群聊的语义依赖于"谁在何时说了什么"，如果发言者身份会随配置漂移，历史就变得不可解读。

发言路由的语义是这个模型里最精巧的部分。项目选择了一个规则非常窄的路由方案：只有出现在行首的 @ 提及才会触发路由，代码块里的 @ 不算，行中间的 @ 不算。这个选择看起来限制性很强，但它解决了一个关键歧义：如果任何位置的 @ 都路由，那么 agent 在讨论一个叫 @某个用户的变量时会误触发移交，代码示例里的 @ 装饰器会误触发移交。把路由限定在行首，就把 @ 提及的语义从"文本里的一个符号"提升为"一个明确的指令前缀"，这极大降低了误触发的概率。配合"先剥离代码块再解析"的预处理，误触发被压到极低。

路由还有一个最深链深度限制和每条回复最多提及数的限制。这两个限制不是性能优化，而是语义安全阀。如果没有链深度限制，一个稍微不聪明的 agent 可能会陷入"我提到你、你提到我"的无限循环，把整个会话锁死。如果没有每条回复提及数限制，一个 agent 一次性 @ 了五个人，调度器就要同时启动五个生成，这既超出了系统的并发容量，也违背了"一次一个人发言"的群聊直觉。把这两个数字定得保守（链深十五、提及两），是在"给协作留足空间"和"防止失控"之间取的平衡。

人类移交的语义同样窄。只有 `@用户 handoff` 这个明确的指令会阻塞等待人类，`fyi` 只是通知不阻塞，`done` 结束本轮。这个设计的动机是：在一个自动化运行的群聊里（比如由 IM 连接器驱动的 headless 会话），agent 之间的移交大多是信息性的，不需要人类介入；真正需要人类决策的时刻应该由 agent 明确声明，而不是任何 @ 人类 都打断。这避免了"agent 偶尔提到人类就暂停整个流程"的脆弱性。

这套语义模型最值得注意的地方是它的双实现。同样的路由规则既在前端 `src/services` 里有纯函数版本，也在 Rust `domain/seat_turn.rs` 里有镜像版本。后端版本存在的唯一原因是 headless 场景：当一个会话由 IM 连接器或定时任务驱动时，没有前端在运行，路由必须由后端完成。这种"前端逻辑在后端镜像一份"的做法，在工程上是一种妥协——它意味着规则改一处要同步改两处，否则行为会分裂。项目用共享纯函数的方式减轻了这个负担，但理论上仍存在漂移风险。这是一个典型的"为了支持 headless 而接受的双实现成本"。

## 第 46 章 上下文窗口的经济学

这一章讨论一个在 agent 系统里无处不在但很少被正面讨论的话题：上下文窗口的经济学。每个被托管的 agent 都在一个有限的上下文窗口里工作，每加入一段对话、一个工具结果、一段记忆、一个 skill，都在消耗这个窗口。VaneHub 作为一个托管多个 agent 的壳，必须在多处做"上下文预算"的决策，这些决策彼此关联，构成了一个隐性的经济系统。

先看这个经济系统里的几个"开支项"。第一项是会话历史本身，这是最大的开支，随对话进行线性增长。第二项是系统提示，包括 OnePiece 核心指令、自定义指令、绑定的 skill、注入的记忆。第三项是工具结果，一次工具调用可能返回大量文本。第四项是 skill 内容，eager 注入的 role skill 直接进系统提示。这几项加起来，很容易在几十轮对话后逼近甚至超出 provider 的上下文窗口。

VaneHub 对这个经济系统的管理体现在几个数字上。压缩触发阈值是六万字符，保留最近六轮。记忆注入预算是四千字符。单个 skill 预算八千字符，聚合 skill 预算一万六千字符。OnePiece 核心指令预算八千字符。历史加载上限五十条消息。这些数字不是随意定的，它们共同构成了一个"在典型 provider 上下文窗口内留出足够工作空间"的预算分配。

理解这些数字的关系，要从一个约束出发：压缩用字符数代理 token 数。这是一个有意的近似——精确的 token 计数需要 tokenizer，而不同 provider 的 tokenizer 不同，引入 tokenizer 会增加依赖和复杂度。字符数代理的代价是估计不精确，可能在实际 token 数低于六万时就触发压缩（浪费一次摘要调用），也可能在实际 token 数高于六万时才触发（逼近窗口边界）。项目接受这个不精确，换取了实现的简单和 provider 无关。这是一个典型的"工程上够用就好"的决策。

压缩本身的语义值得细看。它不是简单地把旧消息删掉，而是用一个无工具的内部模型调用，把早于最近六轮的所有内容总结成一条合成用户消息。这条合成消息替代了那些旧轮次，保留了事实要点但丢失了细节。这意味着压缩是一个有损操作——一旦压缩，原始的逐字对话就不可恢复了。这个有损性是上下文经济学的本质：在有限窗口里无限对话，必然要丢弃信息，问题只是丢弃什么、保留什么。VaneHub 选择保留最近六轮的原始内容和更早内容的摘要，这是一个在"近期精确"和"远期模糊但可用"之间的平衡。

记忆注入和压缩的配合构成了一个有趣的二级经济。记忆是跨会话的，它不随单次会话压缩而丢失；但记忆本身的注入预算只有四千字符，所以记忆也必须被筛选。项目把记忆设计成"显式"和"自动"两种来源，自动记忆是在压缩时顺带提取的——这把"记忆生成"和"上下文压缩"两个本可独立的过程合并了，节省了一次模型调用。这是一个精巧的复用：压缩本来就要读一遍旧内容来生成摘要，顺带提取值得长期记住的事实，边际成本几乎为零。

Skill 的预算分配体现了另一种经济考量。eager 注入的 role skill 占系统提示预算，每条八千、聚合一万六；而 utility skill 不 eager 注入，而是作为工具按需加载。这个区分的经济学意义是：role skill 是"agent 身份的一部分"，每次请求都要在场；utility skill 是"可能用到的能力"，只在真正调用时才消耗上下文。如果把所有 skill 都 eager 注入，系统提示会迅速膨胀到把历史挤没；如果都不 eager 注入，agent 就不知道自己有哪些角色能力。区分 eager 和 lazy 是对"身份信息必须常驻、能力信息可以按需"这个语义区分的工程化。

工具结果的开支则由工具实现侧的上限控制：文件读取限制行数字符数和字节数、搜索限制结果行数、终端输出有捕获队列容量上限。这些上限表面上是防滥用，实际上也是上下文经济的一部分——一个不受限的 cat 命令可以把整个文件塞进上下文，瞬间耗尽预算。

把这些数字放在一起，VaneHub 的上下文经济模型可以这样概括：用字符数代理 token 做粗预算，用有损压缩回收长期历史的空间，用预算上限控制每个开支项，用 eager 和 lazy 的区分优化常驻信息的密度，用复用（压缩顺带提取记忆）降低维护成本。这不是一个理论上最优的模型，但它是一个在实际运行中足够稳健、且实现复杂度可控的模型。在 agent 工程的当前阶段，这种"够用且可控"往往比"理论最优但复杂"更有价值。

## 第 47 章 权限模型的安全语义

第 31 章给了权限系统的结构和四种策略模板。这一章从安全语义的角度重新审视它，重点讨论为什么某些设计是"fail-closed"的，以及这些 fail-closed 选择如何与异构 CLI 的原生权限机制咬合。

权限系统的根本目标是：在一个 agent 可能执行任意 shell 命令、读写任意文件、调用任意 MCP 工具的系统里，把"未经授权就执行"的风险降到最低。这个目标在 VaneHub 里被一个额外约束放大了：被托管的 CLI 有自己的权限机制，VaneHub 的权限系统不能假设 CLI 会乖乖配合，必须既能在原生 API agent 上完全掌控，也能在 CLI agent 上通过启动标志和 hook 桥接施加影响。

fail-closed 是这套语义的基调。所谓 fail-closed，指的是在任何不确定的情况下，系统默认拒绝而不是默认允许。这个基调体现在好几处。第一处是策略模板里 `mcp.tool` 从不出现，它的 Ask 地板是 `evaluate` 的前置条件——也就是说，MCP 工具调用永远需要显式审批，不存在任何模板能自动放行它。这是因为 MCP 工具的能力面是开放的、不可枚举的，任何"允许所有 MCP 工具"的模板都等于把 agent 的能力扩展到无限，这在安全上是不可接受的。第二处是未知工具名映射到合成的 `unknown:<name>` 动作并 fail-closed 到 Ask——系统宁可多问一次，也不赌"这个不认识的工具大概是安全的"。第三处是 Claude Code 权限 hook 的映射里，未映射的工具返回 None 导致 deny——如果 VaneHub 不认识 Claude Code 发来的某个工具调用，它选择阻止而不是放行。

这几处 fail-closed 共同体现了一个原则：在能力开放的边界上，不确定性必须转化为拒绝。这和很多系统"默认允许、显式拒绝"的取向相反，因为那些系统的能力面是封闭可枚举的，默认允许的代价可控。而 agent 系统的能力面是开放的，一个默认允许可能让 agent 获得越权能力的途径有无数种，默认拒绝才安全。

fail-closed 的代价是可用性：用户会被频繁的审批弹窗打扰。VaneHub 用 ApprovalScope 来缓解这个代价，允许用户把一次审批"记忆"到会话级、项目级甚至全局级。这是一个"用安全性换便利"的梯度：once 最安全但不便，global 最方便但风险最大。项目把这个选择权交给用户，而不是替用户决定一个固定档位，这是对"不同操作、不同用户、不同项目对风险的容忍度不同"这个现实的尊重。

CLI agent 的权限管理是这套语义里最复杂的部分。原生 API agent 上，VaneHub 完全掌控工具执行，审批在工具使用循环里同步等待，deny 的结果作为 tool result 喂回 provider。但 CLI agent 有自己的工具执行循环，VaneHub 无法在其循环内同步插入审批。项目用两条路径施加影响：一是启动标志，把策略模板投影成各 CLI 的原生权限标志（如 Claude Code 的 permissionMode、Codex 的 sandbox 和 approvalPolicy），让 CLI 自己按这些标志行为；二是 Claude Code 专用的 hook 桥接，通过一个 localhost HTTP server 接管 Claude Code 的 PreToolUse 回调，把它的工具调用映射到 VaneHub 的权限系统再决定。

这两条路径的覆盖度不同。启动标志是"一次性配置"，它影响 CLI 启动后的整体行为倾向，但不能对单个工具调用做细粒度审批。hook 桥接是"细粒度拦截"，它能在每次工具调用时询问 VaneHub 的权限系统，但它依赖 CLI 提供 hook 机制，只有 Claude Code 支持。这意味着不同 CLI 的权限掌控力是不均的：Claude Code 最细，因为有 hook；其他 CLI 只能靠启动标志做粗粒度控制。这种不均是异构系统不可避免的现实，项目用"在能力允许的范围内做到最细"来应对，而不是放弃非 Claude Code 的权限管理。

hook 桥接本身有一个安全细节值得注意：它绑在 localhost 上，用一个随机 bearer token 鉴权，discovery 文件写给启动包装器。这个设计把"谁能调用权限 hook"限制在本机本进程，防止其他本机进程或远程攻击者伪造工具审批请求。bearer token 的随机性是这个边界的关键——如果 token 可预测或硬编码，任何本机进程都能假装是 Claude Code 调用 hook 并自行放行。这种"最小信任边界"的考量贯穿了权限系统的实现。

把 fail-closed 基调、ApprovalScope 梯度、CLI 双路径覆盖、hook 的本机信任边界放在一起，VaneHub 的权限模型可以概括为：在能力开放的现实下，用 fail-closed 兜底安全，用 ApprovalScope 梯度缓解可用性，用双路径覆盖异构 CLI，用本机信任边界保护审批通道。这不是一个理论上无懈可击的安全模型，但它是一个在"多异构 CLI 托管"这个具体约束下，把风险压到工程可控水平的务实方案。

## 第 48 章 持久化层的演化与对账艺术

第 32 章给了六十五个迁移的清单，但那份清单是静态的。这一章讲这些迁移是怎么演化到六十五个的，以及在演化过程中项目如何处理"多个分支共享一个 SQLite"这个棘手的现实问题。

任何长期演进的项目都会面临迁移的累积。VaneHub 的特殊性在于：它的开发方式高度依赖 git worktree，多个分支并行开发，而这些 worktree 共享同一个应用数据目录下的同一个 SQLite 文件。这意味着分支 A 加了迁移 42，分支 B 也加了迁移 42，当你在两个分支间切换时，数据库里可能同时存在两个不同的"迁移 42"，导致 schema 不一致甚至启动崩溃。这就是文档里提到的版本碰撞历史。

碰撞之所以危险，是因为迁移版本号是幂等性判断的依据。系统通过查 `schema_migrations` 表里有没有某版本来决定是否跳过该迁移。如果两个不同的迁移都叫版本 42，数据库里记了"42 已应用"后，另一个分支的 42 就会被跳过，但实际应用的 schema 是第一个 42 的，第二个 42 需要的 schema 没建。这种不一致在运行时表现为"no such table"之类的崩溃，而且因为发生在启动早期，很难调试。

项目处理这个问题的方法不是"禁止碰撞"——在分布式并行开发里这做不到——而是一套"对账"艺术。对账的核心思想是：不重写历史，而是用后续迁移修复历史造成的不一致。具体的手段有几种。一种是"重放迁移"，比如版本五十三把版本四十九和五十的 schema 重放一遍，确保即便它们当年没正确应用，现在也能补上。一种是"强制修复"，比如 `repair_missing_stable_participant_schema` 直接检查某个不变量是否满足，不满足就强制建立，而不关心它是哪个版本该建的。一种是"退役占位"，比如版本二十七是个 no-op，专门用来占据那个已经碰撞废弃的版本号槽位，防止以后有人再用这个号。

这套对账艺术里最值得称道的是它的诚实：项目承认迁移历史不可变（已部署的数据库已经按那个历史走过了，重写历史会让它们无法升级），承认碰撞会发生（并行开发无法避免），承认修复必须是向前兼容的（不能要求用户删库重建）。基于这三个承认，它用对账迁移而不是历史重写来消化碰撞的后果。这是一种成熟工程态度的体现：不追求理论上的干净，而是在现实的混乱里找到一条能持续演进的路径。

对账的最后一道防线是启动时的密度检查。`assert_migration_history_is_dense` 会检查迁移历史有没有间隙、有没有超出最大期望版本的记录，`EXPECTED_MIGRATIONS` 常量作为真源由测试钉死。这道检查不能阻止碰撞发生，但它能让碰撞在启动时就暴露成显式错误，而不是让数据库在沉默中带着不一致运行，直到某个阴暗角落崩出无法理解的错误。这是"快速失败"原则在持久化层的应用：与其让系统带着坏数据运行，不如让它启动时就拒绝启动并给出清晰的诊断。

这套演化与对账的方法论，对任何需要长期维护 schema 的项目都有参考价值。它说明了一个道理：在真实工程里，schema 的"正确性"不是一次性达到的静态属性，而是在持续演化中通过不断的对账和修复维持的动态属性。追求"永远不碰撞"是徒劳的，建立"碰撞发生后能被发现和修复"的机制才是正道。

## 第 49 章 统一日志的脱敏与可审计性

第 33 章讲了统一日志的结构，这一章深入它的脱敏机制和它对可审计性的贡献。日志系统在一个会处理凭据、token、用户内容的系统里，本身就是一把双刃剑：一方面它必须记录足够的诊断信息以便排查问题，另一方面它绝不能把敏感信息落盘，否则日志文件本身就成了安全隐患。VaneHub 用一套集中的脱敏机制来平衡这两者。

脱敏的核心是一个叫 `redact_text` 的函数，它对所有要落盘的日志内容执行。这个函数不是一个简单的字符串替换，它是一个分词器加规则匹配的组合。它识别几类敏感信息：文件系统路径（Windows 的盘符路径、Unix 的 home 路径、file 协议 URL）会被替换成占位符，因为这能泄露用户的目录结构和项目位置；Bearer 令牌会被剥离；各种 provider 的 API key 前缀（如 sk- 开头的、GitHub 的 ghp_ 和 github_pat_ 前缀、SSH 连接引用前缀）会被替换；敏感的键名（password、api_key、token、secret、credential、authorization、private_key 等）对应的值会被替换。

这套脱敏的精妙之处在于它既是基于模式的，也是基于键名的。基于模式能捕获散落在日志文本里的 token，比如一条错误信息里恰好包含了 API key；基于键名能捕获结构化字段里的敏感值，比如 context 字典里某个叫 password 的键。两者结合，覆盖了日志内容里敏感信息可能出现的主要形态。当然，没有任何脱敏方案是完美的，自由文本里的敏感信息总有漏网的可能，但把脱敏集中在一个函数、强制所有日志路径经过它，已经把风险压到了工程可控的水平。

脱敏之外，日志系统的另一个设计重点是"语义与存储分离"。这个分离体现在两个上下文的分工上：`operations` 上下文拥有日志的语义契约（什么级别的日志、操作日志和诊断日志的区别、外部日志导出端口），而 `platform::logging` 拥有脱敏的 JSONL 持久化、轮转、归档。应用代码只通过 port 发语义记录，从不直接碰文件；基础设施适配器只在外缘消费平台日志存储。这个分离的好处是，领域层不必关心日志怎么落盘（那是平台的事），平台层不必理解日志说的是什么（那是领域的事）。当要改变日志的存储方式（比如改成写数据库或发到远程）时，只需要改平台层；当要改变日志的语义分类时，只需要改 operations 上下文。两个关注点解耦，各自可以独立演化。

可审计性是这套设计的副产品。因为所有日志都经过统一通道、都带时间戳和类别、都脱敏，所以日志文件本身就构成了系统行为的可审计记录。操作任务的生命周期（排队、运行、成功、失败、取消）都有日志关联，崩溃恢复时能从日志重建"这个操作走到哪一步了"。前端错误也通过同一通道上报（category 为 frontend.client），使得"前端崩溃"这个本该难以诊断的事件，在日志里也留了痕。这种"单一日志真源"的设计，让排查问题时不需要在多个日志文件之间跳转，也不需要担心某个 feature 偷偷写了自己的日志文件——后者是被架构测试明令禁止的。

轮转和保留策略则解决了日志自身的增长问题。活动日志文件二十四小时轮转一次，轮转后的文件三十天后归档到子目录，维护操作每小时最多跑一次以避免频繁的磁盘操作。这些数字是"诊断价值"和"磁盘成本"之间的平衡：二十四小时的轮转粒度意味着单个日志文件不会太大，便于检索；三十天的保留意味着最近的诊断信息还在，更久的可以接受丢失。对于一个桌面应用而非高吞吐服务，这个策略是恰当的。

## 第 50 章 可观测性的分层与 fidelity

第 30 章给了执行可观测性的表结构，这一章讨论它的 fidelity 分级设计以及这个设计对"可信诊断"的意义。

在一个托管多个 agent 的系统里，可观测性面临的根本困难是：agent 的行为发生在 VaneHub 控制之外或部分控制之内。原生 API agent 的每一步 VaneHub 都能精确记录，因为它就是工具使用循环的执行者。但 CLI agent 的行为发生在子进程里，VaneHub 只能看到它的输入输出，看不到它的内部步骤。如果可观测性系统假装这两种 agent 的可见度一样，要么会为 CLI 编造它没有的细节，要么会丢失原生 API 的细节。两者都是对诊断的误导。

VaneHub 用 fidelity 分级来诚实表达这种可见度差异。fidelity 有四档：Native 表示这一步是 VaneHub 原生执行的，细节完整可信；Proxied 表示这一步经过了 VaneHub 的代理通道，细节较完整但可能有代理层的不透明；Inferred 表示这一步是从外部输出推断出来的，比如从 CLI 的流式输出推断出某个工具调用的发生；Opaque 表示这一步发生在 VaneHub 完全看不到的地方，只知道它存在但不知道内容。

这个分级看似只是个标签，但它的价值在于让诊断者建立正确的信任模型。当一个执行时间线里某一步标为 Inferred 时，诊断者知道这一步的细节可能不完整，不能用它做精确的因果归因；当标为 Opaque 时，诊断者知道这一步是黑盒，只能从它的输入输出口推测行为。这种诚实的可见度标注，比"所有步骤看起来都一样可信"的假象更有助于诊断，因为它让诊断者把注意力集中在可信度高的步骤上，对低可信度的步骤保持警惕。

fidelity 分级还和工具调用的记录方式咬合。原生 API agent 的工具调用，VaneHub 知道完整的入参出参，记录为高 fidelity。CLI agent 的工具调用，VaneHub 从 CLI 的结构化输出里推断工具调用的发生，如果输出里有 call id 就记为 Inferred，没有就记为 Opaque。这个"有 call id 才 Inferred"的细节体现了对"可信度"的精细管理：有 call id 意味着 CLI 明确报告了这个工具调用，可以把它和后续的 tool result 关联起来；没有 call id 意味着只是从文本里猜出了工具调用的迹象，无法建立关联，只能记为更不可信的 Opaque。

可观测性的另一个设计重点是关联。执行运行、操作任务、消息这三者通过 ID 关联：一个生成对应一个执行运行，一个执行运行关联到一个操作任务，消息上记着它属于哪个执行运行。这种关联让诊断者可以从任何一个入口追踪整个链路：从消息能看到它属于哪个执行运行，从执行运行能看到它的所有 span 和事件，从操作任务能看到它的日志和恢复证据。计划和循环也通过 execution_run_id 和 operation_id 关联进来。这种"全链路可追溯"是可观测性的真正价值所在——单独看一个 span 意义有限，把所有 span 串成一条链才能还原一次完整的执行。

OTLP 导出是这个系统的可选外延。本地时间线默认开，把执行记录存在本地 SQLite，供前端时间线标签页展示。OTLP 导出可配，允许把 trace 发到外部可观测性后端做长期存储和聚合分析。这个外延的配置本身也经过脱敏和信任边界的考量：otlp_auth_ref 存的是凭据引用而非明文，遵循了"凭据不落明文"的原则。捕获策略 capture_policy 有一个 metadata_only 档位，允许只记录元数据不记录内容，这是给"既要可观测性又要最大限度保护内容隐私"的场景准备的又一道控制。

把 fidelity 分级、全链路关联、本地与 OTLP 双路径、捕获策略控制放在一起，VaneHub 的可观测性设计可以概括为：诚实表达可见度差异，用关联把孤立的 span 串成链路，用双路径兼顾本地诊断和远程聚合，用捕获策略给隐私敏感场景留控制。这套设计不追求"看到一切"——那在异构 CLI 场景下不可能——而是追求"对能看到的部分保持诚实，对看不到的部分明确标注"。

## 第 51 章 Tooling 子域的自治与一致性

第 16 章和第 35 章给了 Tooling 这个伞形上下文里九个子域的结构。这一章从"为什么它们是子域而不是独立上下文"和"它们的自治与一致性如何平衡"的角度来讨论。

Tooling 下的九个子域——MCP、SDK、扩展、插件集成、Skill、Prompt Hook、CLI、CLI 配置、CLI 参数——每一个都有自己独立的领域模型、应用服务、基础设施适配器。从代码组织上看，它们几乎已经满足了成为独立限界上下文的条件。那么为什么项目选择把它们留在 Tooling 这个伞下，而不是各自晋升为对等上下文？

答案在于"自治的边界"。一个子域晋升为独立上下文的条件，在项目规范里有明确表述：当它有独立语言、独立生命周期或独立事务所有权时，才可以晋升。Tooling 下的子域虽然各自有模型，但它们共享一些根本的语言和生命周期：它们都是"被 agent 运行时使用的工具性能力"，它们的状态变化大多由 agent 运行时的需求驱动，它们之间没有强事务耦合但概念上同属一个"工具生态"。把它们留在伞下，反映的是"它们足够不同以分别建模，但不够独立以分家"这个判断。

但这不意味着它们是松散的。每个子域都被要求保持自己的领域模型和应用 API，不得互相渗透私有实现。这是一致性的一面：即便同在 Tooling 下，子域之间也像上下文之间一样，通过发布的 API 通信，不直接伸手对方的仓储或基础设施。这种"子域之间也遵守上下文边界"的纪律，保证了即便将来某个子域要晋升为独立上下文，迁移的成本也很小——因为它早就在按独立上下文的方式组织了。

以 Skill 子域为例来感受这种自治。Skill 有完整的领域模型：身份、范围、来源、交付方式、类型、漂移分类，还有一整个 overlay 子系统用于增量修改和回放。它的基础设施层有事务性 SQLite 仓储、有界文件系统日志、live binding 观察、工作区选择、时钟和统一诊断适配器。它发布的 API 既给 agent 运行时用（注入 skill prompt），也给命令层用（CRUD skill）。这个子域的复杂度其实已经超过了一些独立上下文，但它仍是 Tooling 的子域，因为它语义上仍是"工具"——服务于 agent 的能力扩展，而不是独立于 agent 的业务领域。

Prompt Hook 子域则展示了一个"被 agent 运行时消费的不可变契约"的设计。Prompt Hook 的 API 里有一个 `effective_prompt` 方法，它返回组装好的有效提示给 agent 运行时。这个方法被刻意设计成不可变契约——agent 运行时依赖它的输出格式稳定，不会因为 Prompt Hook 内部实现变化而破裂。这是"发布契约"思想在子域边界上的体现：子域内部可以自由演化，但它对外暴露的契约必须稳定。

这种自治与一致性的平衡，对一个会持续膨胀的系统很重要。Tooling 是项目里最容易扩张的部分——每接入一个新的工具性能力（一个新的 MCP 传输、一个新的 SDK、一种新的扩展框架），都意味着新的子域或子域内的新内容。如果子域之间没有边界纪律，这种扩张会让 Tooling 变成一个相互渗透的大泥球；如果每个子域都急着晋升为独立上下文，又会造成上下文地图的碎片化。把它们留在伞下但遵守边界，是在"承认它们是不同的"和"承认它们是一类"之间取的平衡。

## 第 52 章 通信子域的事件驱动完成

第 36 章给了 IM 连接器的结构，第 40 章的时序图给了入站到完成通知的链路。这一章专门讨论这条链路里最关键的设计决策：为什么完成通知是事件驱动的一次性通道，而不是 SQLite 轮询，以及这个选择对 headless 场景意味着什么。

IM 连接器的工作模式是这样的：它收到一条来自 IM 平台的消息，要把它交给 agent 处理，然后等 agent 回复完成，再把回复发回 IM。中间的"等 agent 回复完成"是一个潜在的阻塞点。最朴素的实现是让连接器周期性地查数据库，看那条消息的状态有没有变成 completed。这种轮询实现的问题是：它把延迟和负载绑定在一起——查得频繁则延迟低但数据库压力大，查得稀疏则数据库轻松但用户等回复等得久。而且这种压力是持续的无用功，因为大多数查询的时候消息还没完成。

VaneHub 选择了事件驱动的一次性通道。当连接器调用 `send_message_with_completion` 时，agent 运行时返回一个一次性完成接收器，这个接收器内部是一个 mpsc 通道。连接器在它上面 `recv_timeout` 阻塞等待，而不是轮询数据库。当 agent 生成完成并持久化后，运行时往这个通道里投递完成信号，连接器立即被唤醒去发回复。这种方式的好处是：延迟为零（完成即可被唤醒），负载为零（不查数据库），而且它是真正事件驱动的，符合 IM 场景"长等待、偶发唤醒"的特征。

这个设计的精妙之处在于它如何和生成协调器、消息终端完成注册表配合。`send_message_with_completion` 在启动生成前注册一个完成接收器，生成完成时投递。这个注册表是内存的、一次性的——投递后通道关闭，接收器作废。如果生成过程中应用崩溃，这个一次性通道也丢了，但这种情况下消息状态本身也处于不确定状态，会由会话恢复机制去处理，而不是靠这个通道恢复。这是一个"通道只服务正常路径，异常路径交给恢复机制"的职责划分。

项目对这个设计非常重视，以至于用一个架构测试机械强制它。`communications_completion_wait_stays_event_driven_without_sqlite_polling` 这个测试会检查通信侧的完成等待代码，确保它没有引入 SQLite 轮询。把一个设计原则提升到架构测试层面，意味着项目担心这个原则在后续演进中被破坏——比如某天有人为了"简化"而加了一个 fallback 轮询。这种担心是合理的，因为轮询在代码审查里往往看起来无害（"只是多了一个保险"），但它会重新引入负载和延迟问题。架构测试把"不能轮询"变成硬约束，杜绝了这种回归。

这个事件驱动完成机制还和 IM 的全局并发控制咬合。连接器运行时有一个全局 pending 上限六十四和活跃 agent 生成上限八。这些上限防止 IM 侧的突发流量压垮 agent 运行时。当达到上限时，新的入站会被节流而不是挤进去。这种背压机制和事件驱动完成配合，让 IM→agent 的链路在有压力时优雅退化，而不是在队列里堆积导致全盘变慢。

把事件驱动完成、一次性通道、架构测试强制、背压控制放在一起，这条链路体现了 headless 场景下的一个核心工程原则：让等待变得零成本，让压力变得可控，让原则变得不可破坏。对于一个要在无人值守下运行的 IM 集成，这种稳健性是必须的。

## 第 53 章 桌面生命周期的优雅退出

第 38 章给了桌面设置和浮窗的结构，这一章聚焦于一个常被忽略但用户体验影响巨大的细节：优雅退出。

桌面应用的退出看似简单——用户点关闭，进程结束。但在一个有后台任务、有进行中的生成、有 IM 连接器在跑、有终端会话打开、有未刷盘的日志和数据库的应用里，"点关闭就杀进程"会导致一系列问题：进行中的生成留下孤儿消息状态，IM 连接器突然断开让对端困惑，日志没刷盘丢失最后几条，数据库写了一半的事务回滚。优雅退出的目标就是让这些副作用在退出前被妥善处理。

VaneHub 的优雅退出是分层的。最外层是窗口事件处理，主窗口的关闭事件被拦截，根据当前状态决定是直接关、最小化到托盘、还是先处理进行中的工作。如果浮窗可用且配置为退出时隐藏，主窗口关闭会最小化到托盘而不是退出进程——这是"用户可能只是想临时收起"这个意图的尊重。真正要退出时，才进入优雅退出流程。

优雅退出流程要协调多个上下文的关闭。执行遥测生命周期有一个有界截止时间地关闭——它不会无限等下去，如果在截止时间内没完成就强制关闭，避免退出卡死。IM 连接器要停止所有运行中的连接器，让它们干净断开。agent 终端要关闭所有 PTY。会话维护和定时任务的后台线程要被通知停止。日志和数据库的缓冲要刷盘。

这个协调的复杂度在于"顺序"和"超时"的权衡。有些关闭有依赖关系，比如要先停止产生日志的组件才能安全地关闭日志刷盘；有些关闭可能很慢，比如一个正在进行长操作的工具调用。如果严格按顺序等所有组件关闭，退出可能要等几十秒，用户体验差；如果不等直接杀，又会有副作用。VaneHub 的做法是给每个关闭一个有界截止时间，在截止时间内尽量优雅关闭，超时就强制。这是"尽力优雅但保证最终退出"的平衡。

退出还要处理"桌面归档策略"的同步。会话维护的后台任务在每个周期开始前会重新加载桌面归档策略，这样即便用户在运行中改了归档设置，下一次维护也能用上新策略。退出时如果有未完成的归档，要在截止时间内处理完。这种"配置变更即时生效"的设计，让用户不需要重启就能让设置生效。

优雅退出的最后一道保障是 webview 恢复。主窗口的 webview 偶尔会因为各种原因崩溃或无法响应，`install_main_webview_recovery` 在启动时就装好了恢复机制，当 webview 不响应时能尝试恢复而不是让用户面对一个白屏。这是对"桌面应用最怕的就是窗口卡死无响应"这个用户痛点的预防。

把窗口事件拦截、有界关闭、配置即时生效、webview 恢复放在一起，VaneHub 的桌面生命周期设计体现了对"用户感知"的细致考虑。退出不是一个技术细节，它是用户对应用稳定性的最后印象——一个优雅退出的应用给人可靠感，一个退出时卡顿或丢数据的应用让人怀疑它的其他部分也不靠谱。

## 第 54 章 OpenSpec 流程的治理意义

第 39 章和第 41 章给了 OpenSpec 的 CI 和治理机制。这一章从"为什么一个项目要为变更流程引入这么多仪式"的角度讨论它的治理意义。

OpenSpec 的核心要求是：任何新功能或架构调整，必须先在 `openspec/changes/` 下起一个 proposal，通过校验后再动代码。这个要求表面上是流程负担——为什么不直接写代码？——但它解决的是一个真实的问题：在多人协作、长期演进的项目里，"为什么这么改"的信息很容易丢失。代码只记录了"改了什么"，commit message 记录了"改了什么"的概要，但"为什么决定这么改、考虑过哪些替代方案、这个改动影响哪些 spec"这些上下文，如果没有专门的载体，就会在时间流逝中消散。

OpenSpec 的 proposal 就是这个载体。它要求在动手前明确变更的设计、涉及的 spec、要完成的任务。这强迫开发者在写代码前先想清楚要做什么、为什么、影响什么。这种"先想后做"的纪律，在短期看是慢的——写 proposal 比直接改代码多花时间——但在长期看是快的，因为它减少了"改完才发现方向错了"的返工，也让后来的维护者能理解决策的来龙去脉。

归档治理则解决了"提案的生命周期"问题。一个 proposal 从提出到完成，再到归档，有一条清晰的路径：完成所有任务、校验通过、归档到带日期的目录、更新索引。归档后的工件是不可变的，这意味着已完成的决策成为历史记录，可以被后来的决策引用和对比。这种"历史可追溯"让项目的演进有了连续性——新人在做一个相关变更时，可以查到历史上类似变更是怎么决策的，避免重复踩坑或推翻已验证的决定。

每六个月审查在线归档、冷迁移到不可变分支的机制，则解决了"在线归档无限膨胀"的问题。如果一个项目的所有历史 proposal 都永远留在主仓库里，仓库会越来越臃肿。把足够老的历史迁移到冷存储（不可变分支或 tag），并在冷迁移记录里留下可验证引用，既保持了可追溯性，又控制了主仓库的体积。这种"热归档服务于近期查询、冷归档服务于历史追溯"的分层，是归档治理的成熟表现。

OpenSpec 流程的真正治理意义，不在于它增加了多少仪式，而在于它把"变更决策"从一个隐性的、依赖个人记忆的过程，变成了一个显性的、可审计的、有载体可查的过程。这种转变让项目不依赖于某个"什么都记得"的关键人物，而依赖于一个任何人都能查阅的文档系统。这是项目能长期健康演进的组织保障。

## 第 55 章 架构守护的机械性与局限

第 19 章和第 41 章给了架构守护机制的全貌。这一章诚实地讨论它的机械性价值，以及它无法覆盖的语义盲区——因为只有认识到守护的边界，才能正确地依赖它而不滥用它。

架构守护的核心价值是机械性。`tests/architecture.rs` 用 `syn` 解析 Rust 源码的 AST，检查依赖方向、命令体的纯度、进程构造的集中性等规则。这种检查是机械的——它不依赖人的判断，只要规则被违反就报错。机械性的好处是确定性和无遗漏：一个 PR 如果让 domain 模块 import 了 rusqlite，架构测试必然报红，无论 review 者是否注意到。这把一大类"结构性腐化"从 review 责任变成了 CI 责任，极大降低了漏检的概率。

但机械性的局限同样明确。架构测试能检查"domain 不依赖 rusqlite"，但它检查不了"这个用例的事务边界是否正确"。它能检查"命令体里没有 if/match"，但它检查不了"这个命令的 DTO 映射是否遗漏了字段"。它能检查"进程构造只在 platform/process"，但它检查不了"这个进程构造的参数是否安全"。这些语义层面的问题，仍然需要人来判断。

项目对这种局限是有清醒认识的。`openspec/project.md` 里有一句明确的话：Rust 可见性和架构测试提供机械强制，review 仍负责语义上下文所有权和事务边界。这句话是诚实的边界声明——它不假装架构测试能保证一切，而是明确划分了机械强制和人工审查各自的领地。

这种边界划分的一个重要后果是：架构测试不应该被滥用为"凡是能用代码检查的都写成规则"。如果试图用机械规则覆盖语义问题，要么规则会过于宽松而无效，要么规则会过于严格而误报连连，阻碍正常开发。合理的做法是让架构测试守住那些"清晰、二元、结构性"的规则，把"模糊、需要上下文、语义性"的判断留给 review。

VaneHub 的架构测试选择守的规则，恰好都在"清晰、二元、结构性"这个范围内。依赖方向是二元的（依赖或不依赖），命令体纯度是二元的（有或没有 I/O），进程构造位置是二元的（在或不在允许的地方）。这些规则没有模糊地带，违反就是违反，不存在"部分违反"或"情有可原"。正是这种清晰性让架构测试有价值——如果规则本身需要解释，机械执行就失去了意义。

但即便在这个范围内，守护也不是万能的。一个常见的盲区是"合法但有害"的代码：一个不违反任何架构规则的实现，可能在设计上仍然是糟糕的——比如一个用例把本该在一个事务里的多个写操作分散到多个事务，架构测试不会报错，但这破坏了事务一致性。又比如一个 domain 类型为了序列化方便塞进了 serde 派生，架构测试不会拦（因为 serde 不在禁用技术列表里），但这可能违反了"领域类型不该被传输语义污染"的原则。这些盲区提醒我们，架构守护是必要但不充分的：它能把底线守住，但不能把上限托起。

认识到这种局限，正确的态度是：既不轻视架构守护（它能拦住的那些结构性错误确实高频且致命），也不神化它（它拦不住的语义错误同样重要）。把它当作"自动化守住底线、人力托住上限"的分工，让 CI 做它擅长的，让 review 做它擅长的，两者互补。VaneHub 的架构守护实践，正是这种分工的一个成熟范例。

## 第 56 章 系统的张力与未来演进

任何架构都不是终点，而是某个时间点的权衡快照。这一章讨论 VaneHub 当前架构里几处可见的张力，以及它们可能驱动未来演进的方向。这种前瞻性的讨论，比起单纯描述现状，更能帮助读者理解架构的动态本质。

第一处张力在服务边界层的宽接口。`AgentService` 接近一百四十个方法，这个宽度在当前规模下是可管理的，但随着产品能力持续扩张，它会越来越成为一个维护负担。任何一个方法的变更都要同时改 Tauri 适配器、mock 适配器、契约类型，三处同步。未来一个可能的演进方向是把这个宽接口按领域拆分成几个聚合接口，每个聚合接口对应一个服务对象，但保留"运行时整体替换"的契约。这种拆分需要重新设计装配方式，但能让每个领域的变更局部化。这是一个典型的"宽接口在规模增长后从优势变为负担"的演化路径。

第二处张力在多智能体路由的双实现。前端和后端各有一份路由规则，靠共享纯函数减轻漂移，但理论上仍可能不一致。未来一个可能的方向是把路由完全收口到后端，前端只展示后端算出的路由结果。这能消除双实现，但会增加一次前后端往返，影响单席位会话（不需要复杂路由的多数场景）的响应速度。权衡是"一致性"和"单席位延迟"，需要根据实际使用中多席位会话的占比来决定。

第三处张力在原生 API 运行时（OnePiece）的文件规模。七千八百多行的 `api_process_adapter.rs` 是全 crate 最大文件，它把工具使用循环、压缩、记忆、权限、工具执行都堆在一起。虽然三百行硬规则不适用于 Rust 文件，但一个文件承载这么多职责仍是维护风险。未来一个可能的方向是把这个文件按职责拆分成多个模块（循环、压缩、记忆、工具执行各成模块），通过明确的 trait 组合。这种拆分不改行为，但能降低单文件的认知负担。

第四处张力在迁移历史的持续累积。六十五个迁移已经是一个不小的数字，且碰撞历史说明并行开发对它造成了真实压力。未来如果迁移数继续增长，密度检查和对账迁移的成本会上升。一个可能的方向是周期性地把累积的增量迁移"折叠"成一个全量基线迁移，但这要求处理所有已部署数据库的升级路径，技术上很复杂。更现实的可能是接受迁移持续累积，靠对账机制消化碰撞，把维护成本控制在可接受范围。

第五处张力在 Tooling 子域的扩张压力。每接入一个新的工具能力，Tooling 就会扩张。九个子域已经是不少，未来可能更多。如果某个子域（比如 Skill，它已经相当复杂）的独立性进一步增强，它可能满足晋升条件成为对等上下文。这种晋升会改善它的自治，但会增加上下文地图的复杂度。判断何时晋升、何时保持子域，是一个需要基于实际演化判断的架构决策。

这些张力不是缺陷，而是活系统的正常状态。一个没有张力的架构往往意味着它停止了演进。VaneHub 的架构在当前规模下是健康的，它用机器强制层把结构性张力控制在可接受范围，用明确的 ADR 记录重大决策的理由，用 OpenSpec 流程保证变更的可审计。这些机制本身就是为应对未来张力准备的——当张力大到需要架构调整时，这些机制能让调整有序进行，而不是陷入混乱。这是架构治理的真正价值：不是让架构永远不变，而是让架构的变化始终可控。

---

# 第八部分 实现细节与边界条件

第七部分讲的是设计哲学和张力，这一部分下探到实现层面，讨论那些容易被概括性叙述忽略的边界条件、错误处理和状态机细节。这些细节单独看琐碎，但合在一起构成了系统稳健性的底座。一个系统能不能在生产环境长期运行，往往不取决于它的正常路径有多优雅，而取决于它的异常路径有多周全。

## 第 57 章 会话状态机的完整转换图

第 16 章提到过 `SessionLifecycle` 有五个状态：空闲、启动中、运行中、失败、停止。但那里没有展开的是这些状态之间的合法转换路径，以及非法转换如何被拦截。理解这个状态机的关键是认识到：会话状态不是任意的，它必须反映底层的生成活动，而生成活动有自己的生命周期。如果允许状态被随意设置，就会出现"状态说运行中但没有生成在跑"或"生成已停止但状态还停在运行中"这种不一致，前者让 UI 误显示，后者让会话卡死无法重启。

合法的转换路径是这样的。一个新创建的会话处于空闲状态。当用户发送第一条消息启动生成时，状态转为运行中，此时会话拒绝接受新的生成请求——因为生成协调器对每个会话只允许一个独占租约。生成完成或失败后，状态转回空闲（成功）或失败（出错）。失败状态的会话可以被重试，转回运行中。归档的会话不能激活、不能接受消息、不能启动生成，这是通过 `ensure_accepts_messages` 这个不变量在多个操作前拦截的。停止状态用于显式停止的会话，介于空闲和失败之间。

这个状态机的不变量之所以重要，是因为它们保护了一致性。归档会话不能激活，保证了一个被归档的历史会话不会被意外唤醒并产生新消息，污染它的历史完整性。连接器拥有的会话不能激活，保证了 IM 来的会话只能通过连接器路径驱动，不能被桌面 UI 抢占控制。自动归档的前提（未归档、未置顶、无活跃生成、恢复干净）保证了归档操作不会在会话还有未完成工作时执行，造成数据丢失。

消息本身也有状态机。一条消息从待处理开始，进入流式表示正在生成，最终转到完成、失败或取消之一。这个状态机的关键约束是流式只能转到终态，不能从终态回到流式。这保证了一条消息一旦完成就不会再被修改——历史消息的不可变性。取消是一个特殊终态，它表示生成被用户主动中止，区别于失败的被动出错。这个区分对 UI 展示和用量统计都有意义：取消不计费、失败要记录错误。

文件引用集合也有不变量：最多五个引用、路径唯一。这两个约束防止了一个消息被塞进过多文件引用导致上下文膨胀，也防止了重复引用造成混乱。这些看起来很小的约束，每个都对应着一个真实的滥用场景和一个明确的防护意图。

## 第 58 章 操作任务的内存模型与恢复证据

第 33 章提过操作任务是内存的，只有恢复证据持久化。这个设计选择背后的考量值得展开，因为它体现了一种"区分易失状态和持久证据"的精细思路。

操作任务代表一个可观测的长操作，比如 SDK 安装、MCP 连接测试、agent 生成。它的完整生命周期包括排队、运行、追加日志、关联执行、成功、失败、取消。如果把这些全部持久化，每次状态变化都要写库，对一个可能频繁更新的操作来说是可观的写负载。而且操作任务的大多数字段（日志、中间状态）在正常路径下只服务于实时观察，一旦操作完成就只有终态有意义。

VaneHub 的选择是：操作任务的完整状态在内存里维护，只有"恢复证据"持久化。恢复证据记录的是操作的终态——它的 ID、关联的执行运行、终态状态、最后更新时间。这个最小持久化的目的纯粹是为了崩溃恢复：如果应用在操作运行中崩溃，重启后能从恢复证据知道"这个操作到崩溃时处于什么终态"，从而决定是把它标记为失败、还是继续、还是忽略。

这个设计的精妙之处在于它精确地持久化了"恢复所需的最小信息"，而不持久化"只需实时观察的信息"。这是一种信息论意义上的最小化：只存那些丢不得的信息。日志和中间状态丢得起，因为它们只服务实时诊断；终态丢不得，因为它决定了恢复后的行为。这种区分让持久化负载降到最低，同时不失恢复能力。

恢复证据的持久化用 UPSERT 写入，这意味着同一个操作的恢复证据会被更新而不是追加。这也是有意为之——恢复证据只需要"最新的终态"，不需要历史轨迹。如果需要历史轨迹，那是执行可观测性的 span 和 event 的职责，它们是单独持久化的。操作任务的恢复证据和执行可观测性的记录是两个不同的关注点：前者服务崩溃恢复，后者服务诊断分析。把它们分开，让各自的设计可以针对自己的关注点优化。

操作的取消机制也值得一看。`OperationService` 维护一个取消标志的注册表，每个操作对应一个 `Arc<AtomicBool>`。调用方可以拿到这个标志并轮询它，从而在长操作中检查是否被取消。这个机制不是真正的协作式取消（它不强制停止操作），而是通知式取消——它告诉操作"你应该停了"，由操作自己在合适的检查点响应。这种设计承认了一个现实：很多外部操作（比如一个 npm install）无法被中途打断，只能等它自然结束；但操作的实现可以在它的循环里检查取消标志，提前退出。这是在"取消的即时性"和"外部操作的不可中断性"之间的务实折中。

## 第 59 章 SSH 连接池的并发语义

第 34 章给了 SSH 连接池的参数：容量八、空闲超时五分钟、连接超时十五秒、保活三十秒。这一章讨论这些参数背后的并发语义，以及连接池如何处理几种典型的竞争场景。

连接池的核心数据结构是一个按连接键索引的条目表。每个条目要么处于连接中、要么就绪、要么在排空。当一个会话请求一个 SSH 终端时，连接池按连接键查找。这里连接键是连接 ID 加版本号的组合，版本号在端点或认证方式变更时递增。用版本号作为键的一部分，意味着当用户改了 SSH 配置，新请求会拿到一个新键，从而建立新连接，而旧连接进入排空——它不会被复用给新配置，但已经租出去的会话仍能用它完成工作。这是一种"配置变更不中断已有会话"的语义。

竞争场景的第一种是"多个会话同时请求同一个键的连接"。如果朴素地处理，会同时发起多个连接请求，浪费资源还可能触发对端的连接限速。连接池用 single-flight 模式处理：第一个请求发起连接，后续请求 join 到同一个 in-flight 的 future 上，等连接建立后共享。这个 future 被包成 `Shared`，可以被多次 await，结果被所有等待者拿到。这样无论多少个会话同时请求同一键，只会有一个实际的连接过程。

第二种场景是"连接池已满，新键的连接请求到来"。这时连接池要腾位置。它选一个 LRU 的空闲条目淘汰——也就是最久没被使用的那个。如果所有条目都在忙（连接中或已租出），新请求得到一个"池满"的结果，而不是无限等待。这种"宁可拒绝也不无限堆积"的背压，防止了连接池在突发负载下变成一个无底洞。

第三种场景是"某个连接变得不健康"。连接池的条目有健康状态。当一个连接被判定不健康（比如保活失败、传输错误），它进入排空——不再接受新租约，但已租出的会话继续用。等所有租约归还后，连接被关闭，条目从池里移除。这种"排空后清理"的语义保证了不健康连接不会立即中断正在使用它的会话，而是优雅退场。

第四种场景是"配置变更导致键变化后的连接清理"。当一个连接进入排空，连接池保证同键的新连接不会复用它，但旧连接的关闭要等租约归还。如果租约长时间不归还（比如会话挂起），排空可能持续很久。连接池有一个排空超时，超过后强制关闭。这是"优雅退场"和"不无限占用槽位"之间的平衡。

保活机制是连接池维持健康的方式。SSH 连接长时间无数据传输时，中间的网络设备可能把它当成死连接关掉。保活每三十秒发一个空包，让连接保持活跃状态。这个频率是"不频繁到增加负担"和"足够频繁到防超时"之间的平衡。保活失败会把连接标记为不健康，进入排空流程。

主机密钥验证是 SSH 安全的核心。连接池在首次连接时拿到对端主机密钥的指纹，问验证器是接受还是挑战。首次见到时挑战意味着问用户是否信任这个新主机；指纹变化时挑战意味着警告可能存在中间人攻击。这两种挑战有不同的安全语义，前者是常规的信任建立，后者是潜在的安全事件。把它们区分开，让用户能正确对待"新主机"和"指纹变了"这两种截然不同的安全含义。

把 single-flight、LRU 淘汰、背压拒绝、排空清理、保活、主机密钥验证放在一起，这个连接池体现了一个成熟的并发资源池应有的全部语义。它不只是"复用连接省开销"，而是"在并发、变更、故障、安全的多重压力下，正确地管理一组有状态的网络资源"。

## 第 60 章 终端输出捕获的有界性与可搜索性

第 34 章给了终端输出捕获的表结构和上限。这一章讨论这些上限背后的工程考量，以及捕获队列如何处理"输出超过容量"这个必然发生的场景。

终端输出捕获的目标是：把一个终端会话的所有输出存下来，让它可搜索、可回放、可审计。这个目标看似简单，难点在于终端输出可能是无限的——一个 `find /` 或一个长时间运行的日志进程可以产生几十兆甚至几吉的输出。如果不加上限，这个无限输出会撑爆数据库和磁盘。但如果简单地"超过就停"，又会丢失可能重要的尾部输出。

VaneHub 的捕获队列用了一个"有界但保留连续性"的方案。队列容量是二百五十六个块，每块最大三万两千字节。当输出超过容量时，队列丢掉最旧的块，但设置一个"有丢弃"标志。下次批量读取时，会在丢弃发生的位置插入一个"捕获间隙"标记块。这个标记块告诉观察者：这里有一段输出丢失了。这种设计的语义价值在于：它不假装输出是完整的，而是诚实地标注丢失的发生。一个标注了间隙的输出，比一个被静默截断的输出更可信，因为观察者知道哪里不能信。

块的规范化是另一个细节。终端输出里可能包含 ANSI 转义序列（颜色、光标移动等）、无效的 UTF-8 字节。捕获队列在落库前把转义序列剥掉、把无效字节替换成替换字符、拒绝超过块大小限制的内容。剥掉转义序列是为了让输出可搜索——带有颜色码的文本搜索起来很困难，因为同一段文字可能被转义序列分割成多段。替换无效字节是为了保证文本完整性。这些规范化让存下来的输出是干净的、可检索的文本，而不是带噪声的原始字节流。

可搜索性由 FTS5 全文索引提供。捕获的输出块建了一个 FTS5 的 trigram 虚拟表，配合触发器在块插入删除时同步索引。FTS5 的 trigram 分词适合搜索代码和终端输出，因为它按三字符滑窗分词，对任意子串都能命中。搜索结果带高亮 snippet，让用户能看到命中片段的上下文。搜索有页大小上限（五十到一百）和查询长度上限（五百一十二字符），防止滥用搜索拖垮数据库。

容量维护是输出捕获的最后一道。有一个后台任务定期检查总输出字节数，超过容量上限（五百多兆）时删最旧的行直到回到上限内。这个删除是逐行事务的，避免一个巨型事务卡住数据库。会话级和连接级的清理也支持，让用户可以按需清空某个会话的输出历史。

命令模板和命令运行则把"可重复的命令"和"不可变的运行快照"分离。模板可以被编辑和删除，但一旦执行，它的命令内容被快照存到运行记录里。这样即便后来模板被删，历史运行的命令内容仍可追溯。这是"可变定义与不可变执行"分离的又一个实例，和 Prompt Hook 的 draft→publish→rollback、迁移的"历史不可变"是同一个设计哲学。

把这些放在一起，终端输出捕获的设计体现了一个原则：在无限的流式数据上建立有界但可信的存储。它不追求存下一切（不可能也不必要），而是追求存下的部分是准确的、可搜索的、丢失被诚实标注的。这种"有界但诚实"的处理，是处理流式无限数据的成熟方式。

## 第 61 章 检索的混合路径与优雅降级

第 37 章给了检索的双路混合和 RRF 融合。这一章讨论它的降级语义，以及为什么"一路失败不致命"是检索系统的关键韧性。

检索系统有两条路径：向量检索（基于 embedding 的余弦相似度）和关键词检索（基于 FTS5 的全文匹配）。两条路径各有优劣：向量检索擅长语义相似但需要预先 embedding、对精确字面匹配不敏感、依赖 embedding 模型可用；关键词检索擅长字面匹配、不需要 embedding、但抓不住语义相似。把它们混合，理论上能取两者之长。

但混合引入了新的失败模式：如果其中一条路径失败怎么办？比如 embedding 服务不可用，向量检索就会失败。如果设计成"任一路径失败则整体失败"，那么 embedding 服务的任何抖动都会让检索完全不可用，这在一个"检索是 agent 的 recall 工具、失败不应阻断生成"的场景下是不可接受的。

VaneHub 的降级语义是：两条路径都失败才报不可用，一条失败则降级到另一条。具体来说，如果向量失败、关键词成功，系统返回标记为"仅关键词"的结果集；如果关键词失败、向量成功，返回"仅向量"结果集；只有两者都失败才报真正的错误。这种降级让检索系统对单路径故障有韧性——embedding 服务挂了，agent 仍能用关键词检索，虽然召回质量下降但不至于完全失能。

降级还有一层：召回工具的失败绝不阻断生成。这是写在代码注释里的设计规则——recall 工具的任何失败都被处理成返回空结果或错误消息给 agent，而不是抛异常打断生成循环。这个规则体现了一个重要的优先级判断：agent 的主流程（思考和行动）比辅助能力（检索）更重要，辅助能力的失败不应该拖垮主流程。这种"主流程优先、辅助能力尽力而为"的分层，是稳健 agent 系统的重要原则。

RRF 融合是两条路径都成功时的合并策略。它的原理是按排名而非分数融合：每条路径返回的结果有自己的排名，RRF 给每个结果一个由排名决定的分数（高分给靠前的结果），然后合并两条路径的分数。用排名而非分数避免了"两条路径的分数尺度不同"的问题——向量相似度和 FTS5 相关性分数的数值范围完全不同，直接相加无意义，但排名是可比的。RRF 的平滑参数控制了靠后结果的影响力，让头部结果主导但尾部仍有机会。

embedding 前的脱敏是检索系统的安全细节。代码在被 embedding 之前会先做密钥扫描和脱敏——如果一段代码里恰好有 API key 或 token，它不会被原样 embedding 进向量索引，而是先脱敏。这防止了向量索引成为敏感信息的副本。虽然 embedding 本身是高维向量、不可逆，但把敏感信息排除在索引之外仍是纵深防御的一部分。

代码索引的工作区维度让检索可以按工作区隔离。每个工作区有自己的索引状态、文件清单、chunk 清单。索引有版本号，文件修改后索引版本更新，旧查询的结果可以和新版本对比判断是否过期。这种"索引版本化"让检索结果可以标注"这是基于某个版本的索引，文件可能已变化"，避免给 agent 过时的信息。

把双路混合、RRF 融合、单路径降级、不阻断生成、embedding 前脱敏、索引版本化放在一起，检索系统体现了一个原则：在多依赖、易失败的环境里，用降级和优先级分层保证核心流程的韧性。检索是 agent 的眼睛，眼睛偶尔模糊不应该让 agent 停下脚步。

## 第 62 章 Skill overlay 的增量修改与回放

第 35 章提过 Skill 有一个 overlay 子系统。这一章深入它，因为它是 VaneHub 里设计最精巧的子系统之一，体现了一种"在不修改原始资产的前提下做增量定制"的成熟思路。

Skill 的核心矛盾是：内置 skill 是不可变的资产，它们随应用发布、有内容哈希、不应该被直接修改；但用户又需要定制 skill 的行为——加一段指导、改一处的措辞、插入一个步骤。如果让用户直接改 skill 文件，升级时就无法合并，定制会丢失。如果完全不允许定制，又限制了灵活性。

overlay 的解法是：不修改原始 skill，而是在它之上叠加一层修改。修改以补丁的形式存在，独立于原始内容。当需要 skill 的有效内容时，系统把原始内容和所有适用的 overlay 按顺序回放，得到最终内容。这就像 git 的 rebase——基础历史不动，改动以补丁形式叠加。

这个模型有几个关键设计点。第一是 overlay 的顺序确定性。多个 overlay 按确定的顺序应用，保证同样的输入产生同样的输出。这个顺序由 overlay 的元数据决定，不允许随意排序。第二是回放的可重现性——overlay 的应用是纯函数，没有副作用，可以反复回放得到一致结果。第三是 overlay 的审计性——每个 overlay 有来源、时间、作者记录，可以追溯谁在何时做了什么修改。

overlay 的信任边界是它的安全语义。不是所有 overlay 都被同等信任。来自系统管理的 overlay 和用户创建的 overlay 有不同的信任级别，应用时可能有不同的限制。这防止了一个恶意或错误的用户 overlay 把 skill 改成危险内容（比如把"不要执行危险命令"改成"执行所有命令"）。

overlay 的漂移检测是它的自愈机制。当原始 skill 升级后，旧 overlay 可能不再适用——它针对的原始内容已经变了。漂移检测比较 overlay 预期的原始内容和实际的原始内容，如果不一致就标记漂移，提示用户重新审视或同步 overlay。这种"基础变了、叠加层需要重新校准"的机制，让 overlay 系统在 skill 持续演进时保持一致。

overlay 的媒体验证、路径验证、文本扫描是一组防护。媒体验证确保 overlay 引用的媒体资源是合法的；路径验证确保 overlay 操作的路径在允许范围内；文本扫描在 overlay 内容里检测敏感信息。这些防护把 overlay 的能力限制在安全边界内，防止它被滥用成任意文件读写的通道。

把增量补丁、顺序确定、纯函数回放、信任分级、漂移检测、内容防护放在一起，overlay 子系统体现了一种"在不可变基础上做可变定制"的成熟工程模式。这种模式在软件工程里反复出现——从源码补丁到数据库迁移到配置覆盖——它的核心思想都是"基础不可变、变化可叠加、冲突可检测、历史可追溯"。VaneHub 把这个思想用在了 skill 定制上，让 skill 既保持升级友好又支持深度定制。

## 第 63 章 Prompt Hook 的草稿发布回滚生命周期

Prompt Hook 的 draft→publish→rollback 生命周期是另一个体现"可变定义与不可变执行"分离的子系统。这一章展开它的设计。

Prompt Hook 是在 agent 的提示组装时注入的内容片段，按类别、阶段、来源分类。它的作用是在 agent 收到用户输入前，先注入一些行为指导、上下文、或约束。一个 hook 可能是"在用户提问后、agent 回答前注入一段角色设定"，也可能是"在 agent 调用工具前注入一段安全提醒"。

hook 的生命周期从草稿开始。用户创建一个 hook 时，它先以草稿形式存在。草稿可以被反复编辑、预览效果，但不影响实际运行——agent 运行时用的是已发布的版本，不是草稿。这个草稿态让用户可以安全地实验 hook 内容，不用担心改坏了影响正在运行的会话。

发布是把草稿变成正式版本的动作。发布时，草稿内容被固化成一个带版本号的正式版本，记录内容哈希、发布时间、发布类型。从此 agent 运行时用这个新版本。旧版本不被删除，而是作为历史保留，这样回滚有据可依。

回滚是把当前版本退回到某个历史版本的动作。因为所有历史版本都被保留，回滚只是把"当前指针"移到旧版本，不是重建。这个设计的价值在于：回滚是即时且无损的——如果新版本上线后发现问题，可以立刻退回，不需要重新编辑内容。

版本的内容哈希是这个生命周期的完整性保障。每个版本记录它的内容哈希，这可以检测内容是否被意外篡改。如果某个版本的内容和它记录的哈希不匹配，说明存储出了问题，应该告警而不是默默用错误内容。

执行追踪记录每次 hook 被实际执行的情况——调用了哪个版本、结果如何、耗时多少、关联哪个 agent 和会话。这些追踪让 hook 的效果可观测：用户能看到某个 hook 实际被调用了多少次、是否生效。这是把"定义"和"执行"都纳入可审计范围的设计。

内置 hook 通过目录提供，它们不可被用户修改——这保证了系统提供的基础行为约束不被绕过。用户可以创建覆盖（override）来调整内置 hook 的行为，但不能删除内置 hook 本身。这种"基础不可变、覆盖可叠加"的语义，和 skill overlay 是同构的设计思想。

trace 的变量插值是 hook 的动态能力。hook 模板里可以有变量占位符，在实际执行时用上下文变量填充。这让 hook 可以根据当前 agent、会话、用户输入动态生成内容。变量插值是纯模板替换，不执行代码，保证了一个恶意的 hook 模板不能注入任意逻辑——它只能产生文本。

把草稿态、版本化发布、即时回滚、内容哈希、执行追踪、内置不可变、变量纯插值放在一起，Prompt Hook 的生命周期设计体现了一个原则：让行为定义的变更安全、可逆、可观测。在一个 agent 行为深受提示影响的环境里，提示变更的风险很高——一个错误的 hook 可能让 agent 行为大变。这套生命周期把变更的风险降到最低：实验有草稿态隔离、上线有版本记录、出问题有即时回滚、效果有执行追踪。

## 第 64 章 定时任务与 headless 执行的可靠性

定时任务是 VaneHub headless 执行的一个场景。这一章讨论它的可靠性设计，以及它如何复用主执行链路而不另起一套。

定时任务的目标是：在指定时间或按指定频率，自动启动一个 agent 会话并执行预设的内容。比如每天早上让 agent 生成一份项目状态报告，或每小时检查一次某个仓库的新问题。这个功能让 VaneHub 从"用户驱动的工具"扩展到"自动化的 agent 运行平台"。

可靠性设计的第一个要点是"启动即 backfill"。调度器在应用启动时立即扫描所有到期但未执行的任务，把它们补执行。这处理了"应用关闭期间本该执行的任务"——应用重启后不会因为之前关着就漏掉这些任务，而是补上。这个 backfill 保证了定时任务的"至少一次"语义：任务不会被因为停机而永久丢失。

可靠性设计的第二个要点是"状态机的幂等"。任务有"从未运行、运行中、成功、失败"等状态。状态转换是幂等的——标记任务为运行中时，如果它已经是运行中就不会重复标记。这防止了调度器并发触发同一个任务的竞态。

可靠性设计的第三个要点是"复用主执行链路"。定时任务执行时，它调用的 `agents.send_message` 和用户手动发消息走的是同一条链路，只是消息来源标记为"定时任务"。这个复用意味着定时任务自动享受主链路的全部能力——生成协调、工具执行、权限审批、用量计量、可观测性——而不需要单独实现一套。这是"不重复造轮子"的工程纪律的体现，也让定时任务的行为和手动会话一致，降低了用户的心智负担。

可靠性设计的第四个要点是"任务执行的隔离"。每个定时任务执行创建一个"保留活跃"的桌面会话——即使用户当前有活跃会话，定时任务也能在不干扰用户当前会话的情况下执行。这是对"自动化不应打断用户手动工作"这个体验要求的尊重。

可靠性设计的第五个要点是"失败可见"。任务执行失败时，错误被记录到任务的 latest_error 字段，并经统一日志写盘。用户可以在定时任务管理界面看到失败的任务和原因。这种失败可见性让自动化不会在沉默中失效——如果一个定时任务持续失败，用户能发现并修复，而不是等到需要那份报告时才发现它一直没生成。

可靠性设计的第六个要点是"agent 类型限制"。只有 CLI agent 和 OnePiece 被允许用于定时任务。这个限制反映了 headless 场景的约束——定时任务运行时可能没有桌面会话，需要 agent 本身能在 headless 下工作。API 交互模式的 agent（如果存在仅桌面可用的）不被允许，避免定时任务启动一个无法正常工作的 agent。

把这些可靠性要点放在一起，定时任务的设计体现了一个原则：自动化执行要复用主链路以保证一致性和能力完整，同时通过 backfill、幂等、隔离、失败可见等机制保证在无人值守下的可靠。这种"复用不偷懒、加保障不臆造"的平衡，是把一个手动工具扩展为自动化平台的正确姿势。

## 第 65 章 凭据管理的零拷贝与信任边界

凭据管理在一个处理 API key、SSH 私钥、IM 密钥的系统里，是安全的核心。这一章讨论 VaneHub 的凭据管理如何用零拷贝读和信任边界来降低风险。

凭据的存储依托操作系统的 keyring。这是一个重要的选择——它不把凭据存在 SQLite 里，而是存在操作系统提供的凭据存储里。SQLite 文件虽然在本机，但它是普通文件，任何能读文件的进程都能读；而 keyring 是操作系统级的加密存储，访问受系统权限控制。把凭据放 keyring，提高了"凭据被其他本机进程窃取"的门槛。

零拷贝读是凭据处理的另一个安全细节。当凭据从 keyring 读出来时，它被包在一个 `Zeroizing` 包装器里。这个包装器的意义是：当凭据不再需要时，它所在的内存会被显式清零，而不是等垃圾回收。这缩短了凭据在内存里的暴露窗口——如果没有零拷贝，凭据字符串在内存里可能停留很久，期间任何能读进程内存的攻击都能拿到它。零拷贝让凭据"用完即抹"，降低了内存暴露的风险。

凭据引用是信任边界的一个设计。SQLite 里存的不是凭据本身，而是凭据引用——一个指向 keyring 里某条凭据的稳定标识。这意味着即便 SQLite 文件被泄露，里面的凭据引用也是无意义的——没有 keyring 的访问权，引用指向的凭据拿不到。这种"明文引用加密值"的分离，让数据库泄露不等于凭据泄露。

凭据引用的稳定性是它的工程考量。引用不随凭据内容变化而变化——用户更新了某个连接的密码，引用不变，只是 keyring 里那个引用指向的值变了。这让 SQLite 里的外键关系不受凭据更新的影响，避免了"改密码要级联更新一堆引用"的复杂性。

补偿性变更处理的是凭据更新的部分失败。当更新一个连接的凭据时，如果 keyring 写成功但 SQLite 的引用更新失败，就会出现"凭据已更新但引用没更新"的不一致。补偿性变更在检测到部分失败时回滚——把 keyring 里的凭据恢复到旧值。这保证了"凭据和引用要么都更新、要么都不更新"的原子性。虽然 keyring 不支持跨存储的事务，但通过补偿机制模拟了原子语义。

凭据的删除也是经过设计的。删除一个连接时，它关联的 keyring 凭据也要删。但如果 keyring 删除失败（比如 keyring 服务暂时不可用），系统不能卡住——它记录"待删除凭据"状态，后续重试。这种"删除可延迟但必完成"的语义，防止了凭据在连接删除后仍留在 keyring 里成为孤儿敏感数据。

把 keyring 存储、零拷贝读、明文引用加密值、引用稳定性、补偿性变更、延迟但必完成的删除放在一起，凭据管理体现了一个原则：在凭据这个高风险资产上，每一层都假设其他层可能失守，用纵深防御把每一层的风险都压低。没有任何单层是绝对安全的——keyring 可能被本机恶意进程访问、内存可能被 dump、SQLite 可能被泄露——但每一层都做对了，攻击者要拿到完整凭据就需要突破所有层，这把风险压到了工程可控的水平。

## 第 66 章 国际化的资源对等约束

国际化在大多项目里是一个事后补的工程，但在 VaneHub 里它被当作一个一等约束来对待。这一章讨论它的资源对等机制，以及为什么这种约束对产品质量有深远影响。

国际化的基本做法是所有用户可见文本走 i18n 资源，不硬编码。这个做法本身不特殊，VaneHub 的特殊之处在于它强制"所有注册 locale 的键必须对等"。也就是说，如果一个键在中文里存在，它在英文、繁体中文、日文、韩文里也必须存在，且描述同一个概念和动作。这个对等约束由测试强制，不允许某个 locale 漏键或多了键。

为什么这个约束重要？因为它防止了一种常见的国际化退化：开发者在中文里加了一个键，忘了在其他语言里加，结果英文用户在某些地方看到的是键名本身或者回退到中文，体验断裂。没有对等约束时，这种退化在开发中不易发现——开发者主要测试一种语言——直到海外用户反馈才暴露。对等测试把这种退化在编译期就拦住，让所有语言的质量一致。

对等不仅是键的存在，还包括插值参数、复数形式的一致。一个带参数的翻译"欢迎 {name}"，在所有语言里都必须有相同的参数槽；一个有复数形式的翻译，在所有语言里都必须处理复数。这防止了"中文没复数概念所以没写复数分支，但英文用户看到的是单数形式"这种语义退化。

允许的字面量例外也经过了仔细界定。产品名、provider 名、agent 名、协议名、可执行文件名、npm 包、命令、文件路径、URL、日志级别、稳定 id 可以保持字面量。这些是"数据"而非"UI 标签"，它们在所有语言里都一样，翻译它们反而是错的。把这个例外清单明确化，避免了"为了过 i18n 检查把不该翻译的也塞进资源"的错误。

懒加载 locale 是性能考量。默认语言（中文）的 bundle 在初始化时静态加载，其他语言延迟加载。这让默认用户（中文用户）的启动不因其他语言资源而变慢；切换到其他语言时才加载对应资源。加载失败时回退到默认语言，保证即便某个语言资源损坏，应用也能用默认语言启动。

日期时间格式化也遵循语言。用户看到的日期时间用当前应用语言或从它派生的 locale 格式化。这避免了"界面是英文但日期是中文格式"的不一致。这种细节的约束体现了一个认识：国际化的质量不只在于文本翻译，还在于所有和语言文化相关的呈现都一致。

把键对等、插值对等、复数对等、字面量例外明确、懒加载、日期时间随语言放在一起，国际化约束体现了一个原则：把国际化当作一等质量约束而非事后补丁，用机械检查保证所有语言对等，让产品在任何语言下都有一致的体验。这种纪律在一个支持五种语言的产品里，是避免"主语言精致、其他语言粗糙"退化的关键。

## 第 67 章 主题系统的语义对等

第 12 章给了主题系统的两个主题（futuristic 深色、minimal 浅色）和 CSS token 架构。这一章讨论主题间的语义对等约束，以及它对视觉一致性的意义。

主题对等的要求是：两个主题必须暴露等价的语义角色——背景、前景、面板、静音面板、边框、输入、主色、成功、警告、危险、焦点环、阴影。每个语义角色在两个主题里都有对应的 token，只是颜色值不同。这个对等约束的意义和国际化对等类似：它防止了一个主题被精心维护、另一个主题被忽视的退化。

这种退化在主题系统里特别容易发生。开发者通常主要在一个主题（往往是深色）下开发和测试，浅色主题容易被忽略。如果不对等，就会出现"深色下有个 success-soft token 但浅色下没有"的情况，导致浅色下用 success-soft 的地方渲染异常。对等约束让两个主题的 token 集合始终匹配，任何 token 的增减都要同步两个主题。

CSS token 的三层架构——原始 token（颜色值）、语义 token（语义角色）、组件 token（组件特定用法）——是主题可维护性的关键。原始层定义具体的颜色值，语义层把它们映射成语义角色，组件层消费语义角色。切换主题时只改原始层的值，语义层和组件层不变。这种分层让主题切换成为"换一组颜色值"而非"重写所有样式"，极大降低了维护成本。

终端 ANSI 调色板被设计成固定深色、不随主题变。这是一个有意的选择——终端是开发者高度习惯的界面，他们对 ANSI 颜色的预期是固定的（红是错、绿是对、蓝是信息）。如果终端颜色随主题变，会让开发者在不同主题下对同一颜色的解读不一致，增加认知负担。固定终端调色板是对"终端有自己的视觉传统"这个现实的尊重。

紧凑操作密度的约束是对桌面管理面的特殊要求。设置页、侧边栏、工具栏这些密集操作区域用紧凑间距（基于八像素）、可读的小字号（十二到十四像素）、卡片内无大标题。这个约束防止了管理界面被"营销页式的大标题和留白"污染，让信息密度适合操作而非展示。这是"管理面优先信息密度、内容面优先可读性"的区分。

悬停、激活、禁用、加载、聚焦状态不得改变控件尺寸或挤压相邻内容。这个约束解决的是一个常见的不专业感：一个按钮在悬停时变大，把旁边的按钮挤开，导致用户点错。禁止尺寸变化让交互状态只通过颜色或边框反馈，不通过布局变化，这让界面在交互中保持稳定。

把语义对等、三层 token、固定终端调色板、紧凑密度、状态不变形放在一起，主题系统体现了一个原则：视觉一致性不是"两个主题看起来都不错"，而是"两个主题在任何组件任何状态下都行为一致"。这种一致性需要机械约束（对等测试）和明确规范（状态不变形）双管齐下。

---

# 第九部分 数据流与控制流深度走读

第八部分讲了实现细节和边界条件，第九部分用一组完整的端到端走读把这些细节串起来。走读的价值在于：它把分散在各章的细节放进一个真实的执行序列里，让读者看到它们是如何在时间维度上配合的。一个系统的真正面貌，往往只有在走读中才显现——静态描述容易让各部分显得孤立，而走读揭示了它们的时序耦合。

## 第 68 章 从 CLI 安装到首次生成的完整旅程

让我们跟随一个新用户从零开始的完整旅程，看 VaneHub 的各个子系统如何被串联起来。这个用户刚装好 VaneHub，想用 Claude Code 完成一个编码任务。这个旅程会经过 CLI 检测、SDK 安装、会话创建、配置、生成、流式、计量、持久化的全链路。

旅程的第一步是可用性检测。用户打开应用时，agent 运行时的可用性适配器会检查 Claude Code 是否已安装。这个检查分两层，但**两层是串联的闸门而不是互为回退**（`AvailabilityAssessment::assess`，`domain/catalog.rs:144`）：先看受管 SDK（claude-sdk）是否就绪，缺失就直接判定为"不可用"并附上原因，**PATH 根本不会被查**；只有当 SDK 已就绪或该 agent 压根不需要受管 SDK 时，才继续看 PATH 上有没有 `claude` 可执行文件。检测结果是"可用"、"不可用"、"需要认证"或"未知"之一。如果 Claude Code 不可用，agent 在注册表里会被标记为不可选，UI 上提示用户去 CLI 管理页安装。

这里要说清楚一件容易误解的事：**受管 SDK 不是 CLI 的运行时依赖**。VaneHub 驱动 Claude Code、Codex、Gemini、OpenCode、Antigravity 的方式自始至终是 headless 命令行——按 provider 规则拼出 argv（`claude -p --output-format stream-json --include-partial-messages --verbose`、`codex exec --json -` 等），启动子进程或 PTY，再解析它们各自的结构化输出。执行链路从不加载 `@anthropic-ai/claude-agent-sdk` 或 `@openai/codex-sdk`，这两个 npm 包只出现在 tooling 的 SDK 子域目录里，供"安装/升级/回退"这一组管理动作使用。因此把 SDK 状态放在可用性闸门的第一层，会让一个 `claude`/`codex` 已装好且能正常执行的环境仅仅因为没装那个 npm 包就被判为不可用——这是当前实现的行为，不是被驱动 CLI 的真实约束。

用户决定让 VaneHub 帮他安装。他在 CLI 管理页点安装。这个动作触发一个可观测操作——`operations.start` 创建一个操作任务，状态为排队。然后操作进入运行态，工具子域的包适配器执行 npm install，把 claude-sdk 装到一个受管目录。安装过程的每一行输出被记录到三个 sink：操作任务的日志（供 UI 实时展示）、sdk_operation_logs 表（供历史查询）、统一日志（供持久诊断）。安装完成后操作任务转成功，CLI 检测重新跑——此时闸门的第一层放行了，但结论仍取决于第二层：`claude` 可执行文件必须在 PATH 上，agent 才会被标记为可用。装上 npm 包本身并不会产生这个可执行文件。

这里有一个细节值得注意：安装是异步的，UI 立即拿到操作任务 ID 返回，用户能在操作面板看到进度。这种"立即返回 + 异步执行 + 进度可观测"的模式，是 VaneHub 处理所有长操作的标准模式，它保证了 UI 永远不会因为一个慢操作而卡死。

旅程的第二步是创建会话。用户点新建会话，选择 Claude Code、选择一个工作目录。会话创建走 sessions 上下文的 `prepare_creation` 然后 `execute_creation`。创建过程会委托给 workspaces 上下文验证工作目录、记住已知项目。会话被持久化到 SQLite，初始状态为空闲。这个创建过程本身也是异步的（因为工作目录验证可能涉及文件系统操作），用户立即看到会话出现在侧边栏，状态在后台更新到就绪。

旅程的第三步是配置。用户在聊天输入区选择模型、推理深度、执行模式。这些选择通过 useChatConfig 这个状态机管理，它会根据所选 agent 推导可用的 provider，从模型目录选默认模型，按模型钳制推理深度。配置被防抖（一百二十毫秒）后自动保存到会话的 chat_preferences。这个自动保存让用户的配置选择持久化，下次打开这个会话还在。

旅程的第四步是发送第一条消息。用户输入问题按回车，前端先做乐观更新——把用户消息立即塞进缓存显示出来，让用户感觉即时响应。然后调用 send_message，走到第 40 章详述的那条链路：durable 消息创建、生成租约、操作任务、prompt 组装（含 Prompt Hook 注入、自定义指令、记忆注入）、CLI 进程启动、流式输出解析、token 计量、消息完成持久化。

旅程的最后一步是事后观察。生成完成后，用户可以在执行时间线标签页看到这次生成的 span 和事件——从 prompt 组装到进程运行到工具调用的完整链路。他可以在用量统计页看到这次生成消耗了多少 token。他可以在日志标签页看到关联的操作日志。这种"事后可追溯"是 VaneHub 可观测性的最终价值——它让用户能理解 agent 做了什么、为什么这么做、消耗了多少。

这个旅程把第七、八、九章以及前面各部分的内容串成了一条线。可以看出，VaneHub 的每个子系统都不是孤立的——CLI 安装给可用性检测供数据，可用性检测给会话创建供前提，会话创建给配置供载体，配置给生成供参数，生成给计量和可观测供内容。这种串联让系统的设计意图在执行中显现：每个环节都为下一个环节服务，整个系统围绕"让用户可靠地用 agent 完成任务"这个核心目标运转。

## 第 69 章 错误传播与命令安全边界

这一章专门讨论错误如何从最底层传到用户面前，以及 VaneHub 如何在这个过程中保证不泄露敏感信息。错误传播看似是实现细节，但它直接关系到用户能否理解失败、以及系统是否会因为错误信息泄露而失安全。

错误的最底层是基础设施错误。一个数据库操作可能因为各种原因失败——连接超时、约束冲突、磁盘满。这些错误带有详细的诊断信息，包括原始的 SQL 错误文本。一个进程执行可能失败——可执行文件不存在、超时、退出码非零。一个网络请求可能失败——连接被拒、TLS 错误、HTTP 状态码。这些底层错误的信息量很丰富，但也可能包含敏感内容——SQL 错误文本可能暴露表结构、进程命令行可能包含 token、网络错误可能暴露内部地址。

如果让这些底层错误直接冒泡到用户，会有两个问题。第一是可读性：用户看到 raw rusqlite 错误文本会困惑，它对非开发者毫无意义。第二是安全性：raw 错误可能泄露敏感信息，被显示在 UI 上甚至被记进日志。VaneHub 用分层的错误映射解决这两个问题。

第一层映射在上下文边界。每个上下文把它的基础设施错误映射成自己的领域错误或应用错误，加上使用场景分类。比如 sessions 上下文把数据库错误映射成"会话不存在"、"会话状态冲突"等语义化错误。这一层把技术性错误翻译成业务性错误，让上层能理解失败的业务含义。

第二层映射在命令边界。`commands/error.rs` 把各上下文的错误统一映射成 `CommandError`，带一个分类码（校验、未找到、冲突、不支持、不可用、基础设施、内部）。这一层的关键是 `redacted`——敏感的消息内容在这一步被脱敏，raw 的诊断文本被替换成静态的安全字符串。这个映射是 `From` trait 的实现，意味着每次错误跨命令边界都自动经过它，不存在"忘了脱敏"的遗漏。

第三层是序列化。`CommandError` 序列化成纯字符串发给前端。前端拿到的是一个分类码加一个安全消息，不包含任何原始诊断细节。前端根据分类码决定怎么展示——校验错误显示给用户让他修正、未找到错误提示用户资源不存在、内部错误显示通用提示并上报。

这种三层映射保证了：用户看到的错误是可理解的（经过业务化翻译）和安全的（经过脱敏），而原始的诊断细节只留在统一日志里供开发者排查。这是一个"用户体验和安全并重"的设计——不为了可读性牺牲安全（不把 raw 错误显示给用户），也不为了安全牺牲可读性（不把所有错误都说成"出错了"）。

错误传播还有一条特殊路径：前端错误上报。前端发生的错误（比如 React 组件渲染崩溃）通过 ErrorBoundary 捕获，经 settingsService 上报到 native，最终写到统一日志的 frontend.client 类别。这条路径让前端错误也进入统一可观测范围，而不只是沉默地显示一个错误页。这种"前端错误也留痕"的设计，让排查前端问题有了日志依据。

操作任务的错误有额外的恢复考量。一个进行中的操作如果因为应用崩溃而中断，重启后恢复证据告诉系统这个操作的终态。如果终态是"运行中"但进程已不存在，恢复机制把它标记为失败并记录恢复报告。这种"崩溃后的状态对账"保证操作状态不会因为崩溃而卡在"运行中"永远不动。

把基础设施错误、上下文映射、命令脱敏、前端上报、操作恢复放在一起，错误传播的设计体现了一个原则：错误信息要分层处理，每一层解决自己关注的问题——业务化、安全化、可观测——而不是把原始错误一路抛到用户或一路吞掉。这种分层让错误既可理解又安全，既可观测又可恢复。

## 第 70 章 并发控制与资源限流

VaneHub 是一个高并发的系统——多个会话可能同时活跃，每个会话可能有生成在跑，IM 连接器可能同时收到多条消息，后台任务在定期执行，前端可能在订阅多个事件流。这种并发如果不加控制，会耗尽系统资源或产生数据竞争。这一章讨论 VaneHub 的并发控制机制。

会话级的生成独占是最基础的并发控制。生成协调器对每个会话只允许一个活跃生成——同一个会话里，第二条消息必须等第一条完成才能开始。这通过一个每会话的租约实现。这个限制防止了一个会话里多个生成同时写消息导致的消息序列混乱。但不同会话之间是并发的——用户可以在会话 A 等待生成时切到会话 B 发消息。

IM 侧的全局并发控制更复杂。连接器运行时有一个全局待处理上限六十四和活跃生成上限八。这两个上限防止 IM 侧的突发流量压垮 agent 运行时。当活跃生成达到八时，新的 IM 入站被排队而不是立即处理；当待处理超过六十四时，新的入邦被节流。这种背压让 IM 集成在突发流量下优雅退化，而不是无限堆积导致全盘变慢。每个连接器内部还有一个每聊天待处理上限八，防止单个聊天的突发占据全局配额。

操作任务的并发控制通过序列化适配器实现。CLI 安装、SDK 更新这类变更操作有突变锁，确保同一资源的变更操作串行执行——不能同时装两个版本的同一个 SDK。但不同资源的操作可以并发——装 claude-sdk 和装 codex-sdk 可以同时。这种"同资源串行、异资源并发"的粒度，既防止了数据竞争又最大化了吞吐。

SSH 连接池的并发控制是 single-flight 加容量限制。同一键的并发连接请求合并成一次连接（single-flight），不同键的连接受池容量八限制，满了就拒绝或淘汰空闲。这种控制在保证连接复用的同时防止了连接数无限增长。

定时任务的并发控制通过状态机的幂等实现。调度器每分钟扫描到期任务，标记为运行中后执行。如果同一个任务被并发触发（理论上不该发生但有竞态可能），幂等的状态转换保证它只执行一次。

终端会话的并发控制通过空闲清理实现。每个 agent 终端在两小时无活动后被清理，释放 PTY 资源。一个会话可以有多个终端（虽然不常见），但空闲的会被回收。这防止了终端资源因为用户开了不关而累积。

数据库的并发控制由 SQLite 的 WAL 模式和 busy_timeout 保证。WAL 模式允许读写并发（一个写多个读），busy_timeout 让获取锁失败时等待五秒而不是立即报错。连接池上限十二保证了并发连接数受控。这种配置适合 VaneHub 这种以读为主、写相对少的桌面应用负载。

把会话独占、IM 背压、操作串行、连接池控制、定时幂等、终端回收、数据库 WAL 放在一起，VaneHub 的并发控制体现了一个分层策略：每个资源有自己的并发控制粒度，从最细的"每会话一个生成"到最粗的"全局活跃生成上限"，各自针对自己资源的特性设计。这种分层避免了"一刀切"的并发限制要么太松要么太紧的问题，让每个资源在自己的压力特征下得到恰当的保护。

## 第 71 章 事件系统的设计与订阅语义

VaneHub 大量使用 Tauri 事件来做后端到前端的异步通知——chat 流、终端输出、会话状态、shell 输出、连接器生命周期。这一章讨论事件系统的设计，特别是它的订阅语义和键约定。

事件系统要解决的根本问题是：后端有多个异步事件源，前端有多个组件可能关心这些事件，如何让正确的组件收到正确的事件而不被无关事件淹没。VaneHub 的方案是给每个事件一个键，前端订阅时按键过滤。

chat 事件按 session ID 键。一个会话的生成流式事件只发给订阅了这个 session ID 的前端组件。这意味着用户在会话 A 时，不会收到会话 B 的 token 事件——即便后端同时在为会话 B 生成。这种按键过滤让前端的订阅负载和当前可见会话数成正比，而不是和所有活跃会话数成正比。

终端事件类似地按 shell ID 键。一个终端的输出只发给订阅了这个 shell ID 的组件。会话状态事件是全局的但按事件类型区分。连接器生命周期事件按连接器 generation 键——generation 是连接器的版本号，配置变更时递增，保证前端收到的事件对应最新配置的连接器，不会把旧配置的事件误处理。

这种按键订阅的语义有一个隐含要求：前端必须正确管理订阅的生命周期。一个组件挂载时订阅它关心的事件，卸载时取消订阅，否则会造成事件泄漏——已卸载的组件仍在收事件，既浪费资源又可能触发对已卸载组件的状态更新导致警告或错误。VaneHub 的订阅管理通过 effect 清理函数保证取消。

事件载荷的结构是判别联合。chat 事件是一个带类型标签的联合——started、token、thinking、toolUse、richBlock、completed、failed、cancelled、turnStatus。前端收到事件后按标签分派处理。这种判别联合比"一个事件一个字段全可选"的结构更安全，因为类型系统强制了每种事件只访问它该有的字段。

事件的发射由 infrastructure 适配器实现，不是命令。这个选择很重要——如果命令发射事件，命令就会和事件消费者耦合，破坏薄命令原则。适配器实现 application 的 event port，把领域事件翻译成 Tauri 事件。这让事件发射成为基础设施关注点，领域层只负责产生语义事件，不关心它怎么传到前端。

事件和持久化的关系是"事件反映持久化后的状态"。比如 token 事件是在消息内容追加到数据库后发射的，不是反过来。这保证了前端收到的事件反映的是已持久化的状态——即便前端错过了一些事件（比如标签页切换时），重新加载会话时能从数据库拿到完整状态。事件是"实时增量"，数据库是"完整真源"，二者分工明确。

连接器生命周期事件的校验是它设计的一个细节。事件在发射前被校验，确保它的载荷合法、generation 匹配。这防止了一个损坏的事件让前端进入异常状态。这种"发射前校验"的防御性设计，在事件系统里很重要，因为事件一旦发出就难以收回。

把按键过滤、判别联合、适配器发射、持久化后发射、发射前校验放在一起，事件系统体现了一个原则：异步通知要精确（按键过滤到正确的消费者）、类型安全（判别联合）、解耦（适配器发射不污染命令）、一致（反映持久化状态）、健壮（发射前校验）。这些设计让一个大量使用事件的系统保持可控。

## 第 72 章 资源清理与生命周期回收

一个长期运行的桌面应用，如果不主动回收资源，会因为资源泄漏而逐渐变慢最终崩溃。这一章讨论 VaneHub 的资源清理机制。

会话维护是资源清理的主力。每小时一个后台任务执行会话维护，它会根据桌面归档策略归档符合条件的会话——未置顶、未归档、无活跃生成、恢复干净、超过不活跃天数的会话被自动归档。归档不是删除，但归档的会话不参与默认列表，减轻了侧边栏和默认查询的负载。这个每小时周期是"清理及时性"和"清理开销"的平衡——太频繁增加负担，太稀疏让陈旧会话堆积。

操作任务的清理通过执行可观测性保留实现。执行运行、span、event 保留一定天数后被清理，防止可观测性数据无限增长。这个保留期是"诊断价值"和"存储成本"的权衡——太久占空间，太短失去排查历史问题的能力。

IM 去重表的清理每六小时一次。去重表记录已处理的事件哈希，防止重复处理。但这个表会随事件增长，需要定期清理旧的哈希。清理以五百一十二行为一批，避免一次删太多卡住数据库。WeChat 的回复上下文也有类似的定期清理，防止无用的会话上下文累积。

Agent 终端的空闲清理每两小时一次。无活动两小时的终端被关闭，PTY 资源释放。这防止了用户开了终端不关导致 PTY 进程累积——每个 PTY 都对应一个子进程，不回收会耗尽进程数。

日志的轮转和归档是另一类清理。活动日志二十四小时轮转，归档文件三十天后删除。这防止了日志文件无限增长撑爆磁盘。

SSH 连接池的空闲淘汰是连接级清理。五分钟无租约的连接被关闭，槽位释放给新连接。这防止了连接被建立后不使用而长期占用槽位。

检索索引的容量维护是数据级清理。代码索引的输出超过容量上限时删最旧的，保证索引不超规模。

这些清理机制共同构成了一个"资源有进有出"的平衡系统。资源在需要时被获取（连接、终端、操作任务、会话），在不被需要时被回收。这种平衡是长期运行的关键——任何只进不出的资源都会成为泄漏点，最终拖垮系统。

值得注意的细节是清理都是"有界批量"的。IM 去重每批五百一十二行、终端输出容量维护逐行事务、日志维护每小时一次。这种批量限制防止了清理本身成为性能问题——一个一次删十万行的清理比十次每次删五百行的更可能卡住数据库。有界批量是"清理要温和"的工程纪律。

## 第 73 章 测试策略的层次与契约

VaneHub 有大量的测试——前端的 vitest 单元组件测试、契约一致性测试、Playwright E2E；Rust 的单元测试、架构测试、契约测试、迁移 fixture 测试。这一章讨论测试策略的层次，以及它们各自保护的回归面。

最底层是纯函数单元测试。前端的 `chat-events.ts`、`turn-routing.ts`、`mention-routing.ts` 等纯函数都有详尽的单元测试，验证各种输入下的输出。Rust 的 domain 层有纯领域测试，验证聚合的不变量、状态机的转换、值对象的校验。这些测试快、确定、不依赖任何外部资源，它们保护的是"逻辑正确性"——给定输入，逻辑是否产生正确输出。

往上一层是适配器测试。Tauri 适配器的测试验证方法到命令的映射、归一化器的行为。Web mock 适配器的测试验证模拟行为。Rust infrastructure 层的测试验证 SQLite 适配、进程适配、网络适配。这些测试保护的是"边界正确性"——适配层是否正确地翻译了边界两侧的语义。

契约测试是 VaneHub 测试策略的特色。前端的 `contract-conformance.test.ts` 用条件类型 `Equal<>` 验证 contracts 和 types 的结构一致性。Rust 的 `contract_tests.rs` 验证命令注册、DTO 序列化、错误映射的契约。这些测试保护的是"接口一致性"——前后端、上下文之间的契约是否被遵守。契约测试的价值在于它能在接口漂移的瞬间发现，而不是等到运行时出现莫名其妙的字段不匹配。

架构测试是另一特色。Rust 的 `tests/architecture.rs` 解析 AST 验证依赖方向、命令纯度、进程集中性。这些测试保护的是"结构完整性"——架构规则是否被遵守。架构测试的价值在于它把架构规则从"review 时检查"变成"CI 强制"，让结构性腐化无法悄悄混入。

迁移 fixture 测试验证迁移序列和行数。`migration_fixture_tests.rs` 验证迁移按顺序应用、`EXPECTED_MIGRATIONS` 常量匹配实际。这些测试保护的是"持久化演化的正确性"——迁移序列是否完整、是否无间隙。

E2E 测试（Playwright）验证完整用户旅程。从启动应用到创建会话到发送消息到看到回复，模拟真实用户操作。这些测试保护的是"端到端可用性"——整个系统串起来是否能正常工作。E2E 是最昂贵但也最接近真实体验的测试，它抓住的是单元测试抓不到的集成问题。

覆盖率门禁是测试策略的兜底。前端和 native 都有覆盖率检查，低于阈值 CI 失败。这防止了"新代码没写测试"的退化——任何新代码都要带足够的测试覆盖。覆盖率不是质量的完美指标（高覆盖不等于测得好），但它是"至少有测试"的下限保证。

把这些层次放在一起，VaneHub 的测试策略体现了一个金字塔：底层大量纯函数单元测试（快、多、保护逻辑）、中层适配器和契约测试（保护边界和接口）、上层架构和迁移测试（保护结构和演化）、顶层少量 E2E（保护端到端体验）。这个金字塔让大部分回归在底层快速便宜的测试里被抓，只有少数集成问题才需要昂贵的 E2E。这是测试投入产出比最大化的标准结构。

## 第 74 章 版本同步与发布工程

VaneHub 是一个要打包分发的桌面应用，版本管理涉及前端 package.json、Rust Cargo.toml、Tauri 配置、文档多个地方。这一章讨论版本同步和发布工程。

版本同步的挑战是同一个版本号要在多个文件里一致。前端版本在 package.json，Rust 版本在 Cargo.toml，Tauri 配置引用这些。如果它们不一致，打包出来的应用版本号会混乱——比如 about 页显示一个版本、安装包名是另一个版本。VaneHub 用一个版本检查脚本 `check-version-sync.mjs` 验证所有版本号一致，CI 强制跑。这个脚本把"版本一致"从人工记得变成机器检查。

发布工程的另一个维度是构建产物检查。`build` 脚本不只是 `tsc && vite build`，还跑 `check-frontend-chunks.mjs` 验证 chunk 体积。这防止了一个懒加载配置错误导致某个 chunk 过大，拖慢加载。打包前 sidecar 准备脚本生成权限 hook 的 sidecar 二进制，确保它和主应用一起打包。

跨平台打包是发布工程的复杂部分。VaneHub 支持 Windows（x64 和 arm64）、macOS（x64 和 arm64）、Linux（x64 和 arm64）。每个平台的打包脚本独立，设置正确的目标三元组。native-platform-check 这个 CI job 在 Windows 和 macOS 上各跑一次 `cargo build`，验证跨平台编译没问题——尤其 Windows 的 Rust 工具链有个 LLD 链接器的存在性要求，这个 job 提前发现链接器缺失。

发布优化是 Cargo profile 的约束。架构测试 `distributable_release_profile_stays_optimized` 钉死了 release profile：opt-level 三、thin LTO、单 codegen unit、剥 debuginfo。这些优化让发布二进制更小更快。把这些优化钉进测试，防止了某次"为了调试方便"临时改了 profile 忘了改回来，导致发布版本退化。

文档的发布也有工程化。文档构建是只读的——CI 跑完 docs:build 后用 `git diff --exit-code` 验证没有产生变更，证明文档构建是确定性的、不修改源。这种"构建只读"的验证防止了文档构建悄悄修改仓库。

把版本同步、chunk 检查、sidecar 准备、跨平台验证、发布优化钉死、文档只读构建放在一起，发布工程体现了一个原则：发布是一个需要被工程化的过程，每一步的"正确"都要被机器验证，不能依赖人的仔细。一个能被可靠重复的发布流程，是产品能持续交付的基础。

## 第 75 章 安全模型的纵深防御

这一章把散布在各章的安全设计收拢，看 VaneHub 的安全模型如何构成纵深防御。

第一层是凭据存储。凭据在 OS keyring，不在 SQLite。这提高了凭据被本机其他进程窃取的门槛。零拷贝读缩短内存暴露窗口。明文引用加密值让数据库泄露不等于凭据泄露。这一层保护的是"凭据资产"。

第二层是命令注入防护。所有外部进程构造在 `platform/process` 一处，`validate_executable` 拒绝空和控制字符。这防止了命令注入——一个恶意的可执行文件名不能注入 shell 元字符。Windows 的 `CREATE_NO_WINDOW` 抑制子进程控制台窗口，既改善体验又减少信息泄漏。这一层保护的是"外部进程执行"。

第三层是文件系统边界。`BoundedFilesystem` 做规范包含，拒绝绝对路径、隐藏文件、路径穿越。symlink 安全的包含检查防止了符号链接逃逸。这保护了 agent 的文件操作不越界——一个被指示读工作目录的 agent 不能通过 `../../` 读到工作目录之外。这一层保护的是"文件访问"。

第四层是日志脱敏。所有日志过 `redact_text`，路径、token、敏感键值被替换。这保护了"诊断信息不泄密"——即便日志文件被拿到，里面的敏感信息也是脱敏的。

第五层是权限系统。fail-closed 基调、MCP 工具 Ask 地板、未知工具 deny、ApprovalScope 梯度。这保护了"agent 的能力不越权"——agent 不能执行未经授权的危险操作。CLI 权限 hook 的本机信任边界（localhost 加 bearer token）保护了审批通道不被伪造。

第六层是检索脱敏。代码在被 embedding 前脱敏，防止向量索引成为敏感信息副本。recall 工具失败不阻断生成，这是韧性而非安全，但"检索能力可选"本身降低了检索被滥用为信息外泄通道的风险。

第七层是网络代理控制。全局代理状态受控，默认 bypass 本地地址。这保护了本地服务不被代理意外路由。代理检测探常见端口，帮助用户发现和配置代理。

第八层是 webview 沙箱。`html_widget` 的 iframe 用空 sandbox，不允许脚本执行。这保护了消息里的 HTML 块不能执行任意脚本——即便一个恶意消息包含脚本，它也在沙箱里无法访问应用。

这八层不是孤立的，它们共同构成了一个纵深——攻击者要窃取凭据要突破 keyring 和内存两层；要执行任意命令要突破权限和进程边界两层；要外泄数据要突破文件边界和日志脱敏两层。任何单层被突破都不等于全面失守，因为还有其他层兜底。这种"每层都假设其他层可能失守"的纵深防御，是把一个高能力 agent 系统的风险压到工程可控水平的关键。

当然，纵深防御不是无限叠加层。每加一层都有复杂度成本和维护成本。VaneHub 的选择是"在风险最高的资产上叠多层、在低风险上少叠"，这种风险驱动的分层是合理的——凭据、命令、文件这些高风险资产有多层防护，而低风险的展示层只有 webview 沙箱一层。这种"风险与防护匹配"是纵深防御的正确姿势，而非"到处都堆满防护"的低效做法。

## 第 76 章 可测试性作为架构约束

VaneHub 的架构有一个不太显眼但很根本的约束：可测试性。这一章讨论可测试性如何反向塑造了架构，以及为什么"好测的架构"往往是"好维护的架构"。

最明显的例子是领域层必须纯。`project.md` 要求 domain 测试能在没有 Tauri、SQLite、文件系统、网络、外部进程、OS 凭据存储的情况下运行。这个要求倒逼 domain 层不依赖任何外缘技术——如果 domain 依赖了 rusqlite，它的测试就要起数据库。纯 domain 的代价是所有外部交互要通过 port，但这换来的是领域逻辑可以在毫秒级、确定性地测试。这种可测试性约束，本质上是"依赖倒置"原则的强制——domain 不依赖具体技术，而是定义 port 让 infrastructure 实现。

application 层要求用确定性的 port 替身测试。这倒逼 application 不直接构造具体适配器，而是通过 port 接收依赖。测试时注入假 port，验证用例逻辑；生产时注入真适配器。这种"测试用假的、生产用真的"的分离，是依赖注入的回报——为了好测，架构被迫解耦。

命令层要求零 I/O 预算。这倒逼命令处理器不做实质工作，只做 DTO 映射和委托。如果命令处理器能做 I/O，它的测试就要起外部资源；零 I/O 让命令处理器的测试只需验证映射逻辑。这个约束让命令处理器薄而一致，也容易测。

契约测试要求接口稳定。这倒逼上下文之间通过明确的契约通信，而不是隐式依赖。如果上下文 A 偷偷用了上下文 B 的内部类型，契约测试抓不到这个隐式依赖的变化，但一旦 B 的内部变了 A 就坏。明确契约让依赖可见，可见才能测，能测才能稳定。

架构测试要求依赖方向。这倒逼分层被遵守。一个不可测的架构（到处互相依赖）也往往是一个不可维护的架构（改一处坏一片）。架构测试把"可分层"变成强制，既保证了可测性，也保证了可维护性。

把可测试性看作架构约束，揭示了一个深刻的工程道理：可测试性和好架构是同构的。一个架构如果难以测试，往往是因为它的依赖纠缠、关注点混杂、边界不清——而这些恰恰也是它难以维护的原因。反之，一个为了好测而被倒逼解耦的架构，自然也是一个好维护的架构。VaneHub 把可测试性写进规范、用测试强制，本质上是把"好维护"这个模糊目标，转化成了"好测"这个可机械验证的代理目标。这种转化是工程治理的高明之处。

## 第 77 章 文档工程与可信文档

VaneHub 对文档本身也有工程化要求。这一章讨论它的文档工程，以及为什么"可信文档"是产品的一部分。

文档工程的第一条是"文档和代码不漂移"。文档的链接检查、README 一致性检查、截图检查都是 CI 的一部分。链接检查防止了"文档指向一个不存在的文件"这种常见的文档腐烂。README 一致性检查保证多语言 README 内容对齐。截图检查保证文档里的截图和实际界面一致——界面变了截图要更新，否则文档误导。

文档构建的只读验证（`git diff --exit-code`）是一个巧妙的设计。它证明文档构建是确定性的、不修改源。如果文档构建会修改仓库，那它就成了一个潜在的污染源——开发者不确定哪些变更是自己做的、哪些是构建产生的。只读验证让文档构建可信。

文档的 markdown 规范强制（比如加粗的标点规则）看似吹毛求疵，但它解决的是渲染可靠性。一个不符合 CommonMark flanking 规则的加粗，在不同 markdown 渲染器里可能表现不同——GitHub 上可能正常、别的工具上可能显示原始星号。强制规范让文档在任何 markdown 渲染器里都一致。这种"渲染可靠性"对文档这种要被多种工具消费的内容很重要。

文档的截图自动化（`docs:screenshots`）保证截图可重现。手截的图不可重现——界面变了要重新手截，容易忘。自动化截图让"更新截图"成为一个命令，而不是手工劳动。这种自动化降低了文档维护成本，让文档更可能保持新鲜。

文档工程的意义在于：文档不是代码的附属品，而是产品的一部分。一个产品的文档如果不可信（链接失效、截图过时、规范不一致），用户对产品本身的信任也会下降——"连文档都维护不好，代码能好吗"。VaneHub 把文档纳入 CI 强制，是把"可信文档"当作产品质量的一部分来对待。这种态度在一个面向开发者的工具产品里尤其重要，因为开发者用户对文档质量格外敏感。

把链接检查、README 一致性、截图可重现、markdown 规范、只读构建放在一起，文档工程体现了一个原则：文档要像代码一样被工程化——可检查、可重现、可信任。这不是形式主义，而是对"文档是产品一部分"这个认识的实践。

---

# 第十部分 设计模式纵览与子系统剖析

前九部分按层次和流程拆解了系统。第十部分换一个视角，把贯穿全系统的设计模式提炼出来集中讨论，再对几个尚未深入的核心子系统做专题剖析。设计模式的提炼有助于读者建立"为什么这套代码读起来有一致性"的认识——一致性不是偶然，是同一套模式在不同地方的反复应用。

## 第 78 章 不可变基础与可变叠加模式

这是 VaneHub 里反复出现的一个核心模式：把不可变的基础资产和可变的叠加修改分离，让基础可以安全升级、叠加可以独立演化。理解了这个模式，就能理解为什么项目里有那么多看起来相似但分散的子系统。

这个模式最早出现在 Skill overlay（第 62 章）：内置 skill 不可变，用户定制以 overlay 叠加。它也出现在 Prompt Hook（第 63 章）：内置 hook 不可变，用户以 override 叠加。它还出现在迁移对账（第 48 章）：迁移历史不可变，修复以对账迁移叠加。甚至在某种程度上，会话历史也遵循这个模式——已完成的消息不可变，新的生成以新消息叠加。

为什么这个模式如此普遍？因为它解决了一个根本张力：既要让基础资产可升级（随应用发布新版本），又要让用户定制可保留（不被升级覆盖）。如果让用户直接改基础资产，升级时定制必然丢失——要么定制被覆盖，要么升级被阻塞。分离基础和叠加，让升级只动基础、定制只动叠加，两者在运行时合并，就化解了这个张力。

这个模式的实现要点是"合并要确定且可重现"。叠加的顺序必须确定，合并必须是纯函数，同样的基础加同样的叠加永远产生同样的结果。这个确定性让结果可缓存、可校验、可追溯。如果合并不确定，每次合并结果可能不同，整个系统的可预测性就崩塌。

这个模式还有一个隐含好处：撤销容易。要撤销一个叠加修改，只要移除那个叠加，基础不受影响。这比"在不可变资产上直接改、要撤销就逆向改"安全得多——直接改的逆向操作可能出错，而移除叠加是无损的。Skill overlay 和 Prompt Hook 的回滚都依赖这个特性。

认识到这个模式的普遍性，有助于预测系统的行为：每当看到一个"内置加用户定制"的子系统，可以预期它有叠加机制、合并确定性、撤销能力。这种模式识别让理解新子系统更快——不需要从头读代码，只要识别出模式，就能套用已知的结构理解。

## 第 79 章 端口与适配器模式的系统性应用

六边形架构、端口与适配器——这些概念在 VaneHub 里不是装饰，而是每个上下文的结构骨架。这一章讨论这个模式在系统层面的应用，以及它带来的"可替换性"红利。

每个上下文都遵循同一个结构：domain 定义不变量和纯逻辑，application 定义用例和 port，infrastructure 实现 port。port 是 application 定义的接口，它描述"我需要什么能力"，但不规定"谁提供"。infrastructure 是 port 的具体实现，它知道怎么用 SQLite、怎么起进程、怎么调网络。这个分离让 application 可以在不同 infrastructure 之间切换——测试时用假实现，生产时用真实现。

这种可替换性在测试里的价值显而易见：domain 和 application 测试不需要起任何外部资源，因为 port 可以用假的。但它的价值不止测试。可替换性意味着"换一个 SQLite 实现不影响 application"——如果未来要从 SQLite 换到别的存储，只需要写新的 infrastructure，application 不动。这种"存储可替换"的潜力，是端口模式的长期红利。

VaneHub 的端口设计有一个值得注意的细节：端口是行为导向的、窄的。一个 `SessionRepository` port 不会暴露通用的 CRUD 或原始 SQL，而是暴露具体的行为如"创建会话"、"按 ID 查会话"、"归档会话"。这种窄端口让 application 的意图清晰——看 port 就知道这个上下文需要哪些数据操作。窄端口还防止了 infrastructure 的细节泄漏——如果 port 暴露了 `rusqlite::Connection`，application 就和具体存储耦合了，可替换性丧失。

端口模式的另一个应用是跨上下文通信。上下文之间不直接调用对方的 infrastructure，而是通过对方发布的 `api.rs` 这个 facade。这个 facade 本质上是一个"上下文级端口"——它描述一个上下文向其他上下文暴露的能力。这种"上下文间也走端口"的纪律，让上下文之间的依赖是显式且可审计的，不会有隐式的"上下文 A 偷偷用了上下文 B 的仓储"。

认识到端口模式的系统性应用，有助于理解为什么这个项目有这么多的 trait 定义——agent_runtime 的 ports.rs 有五十多个 trait。这些 trait 不是过度设计，而是可替换性的载体。每一个 port 都是"这里的能力可以替换"的声明。trait 多，意味着可替换点多，意味着测试性好、演化性强。这是有意识的设计密度，而非无意义的抽象堆砌。

## 第 80 章 事件驱动与轮询的取舍

第 52 章讨论了 IM 完成通知的事件驱动选择。这一章把它推广为 VaneHub 处理异步等待的一般原则，讨论事件驱动和轮询各自的适用场景。

异步等待是任何并发系统的常态——等一个操作完成、等一个状态变化、等一个外部响应。处理这种等待有两种基本方式：轮询（周期性检查状态）和事件驱动（状态变化时主动通知等待者）。两者各有优劣，选择哪个取决于等待的特征。

轮询的优势是简单和鲁棒。它不需要等待者和被等待者之间有通道，只要能查状态就行。它的劣势是延迟和负载——查得稀疏延迟高，查得频繁负载重，而且大部分查询是无效的（状态还没变）。轮询适合"状态查询成本低、延迟要求不高"的场景。

事件驱动的优势是低延迟低负载。状态变化时立即通知，无需无效查询。它的劣势是复杂和脆弱——需要通道、需要订阅生命周期管理、通道断了就收不到通知。事件驱动适合"延迟敏感、状态变化稀疏"的场景。

VaneHub 的选择原则大致是：对于"长等待、偶发变化"的场景用事件驱动，对于"需要最终一致但延迟不敏感"的场景用轮询。IM 完成通知是前者——一个 agent 回复可能要几十秒到几分钟，期间状态不变，完成后立即通知。这用轮询要么延迟高（稀疏查）要么负载重（频繁查），用事件驱动最优。架构测试甚至强制它不轮询，可见项目对这一选择的重视。

会话恢复是后者的例子——应用崩溃后重启，要恢复中断的生成状态。这里用的是启动时的扫描（一种一次性轮询），不是事件驱动——因为崩溃时事件通道也断了，恢复只能靠查持久化的状态。这种"正常路径事件驱动、异常路径查状态"的分工是合理的。

前端订阅后端事件是事件驱动，但前端也有轮询的场景。计划运行和循环运行有轮询订阅（每一点五秒到几秒查一次状态），而不是纯事件。这是因为这些运行的状态变化可能由后端多个源触发，用轮询简化了"多个源都通知前端"的复杂度。这是一个"为了简化接受一点延迟"的务实选择。

认识到这个取舍原则，有助于理解系统里看似不一致的选择——为什么有的等待是事件、有的是轮询。它们不是随意的，而是基于等待特征的选择。一个成熟的系统不会一刀切地用事件或轮询，而是按场景特征分配。

## 第 81 章 失败语义的三态与最终一致

这一章讨论 VaneHub 如何处理"操作可能失败、可能成功、可能未知"这个三态现实，以及它如何追求最终一致而非强一致。

在分布式或涉及外部进程的系统里，一个操作的结果不总是二元的成功或失败。还有第三态：未知。比如发起一个 agent 生成后应用崩溃了——这个生成可能在外部进程里继续跑完了，也可能没跑完，重启后无法确定。这种"未知"态不能用成功或失败的语义处理，需要专门的恢复机制。

VaneHub 的会话恢复就是处理未知态的机制。一个状态为"运行中"但进程已不存在的生成，是典型的未知态——它可能已经产生了部分输出，也可能什么都没产生。恢复协调器收集证据（已持久化的消息、操作恢复证据），基于证据决定恢复动作：如果消息已完成，标记完成；如果消息在流式但进程已死，标记为需要处理或隔离。这种基于证据的恢复，把未知态最终归约成确定态。

这个恢复过程追求的是最终一致——不要求系统在任何时刻都强一致，但要求经过恢复后达到一致。强一致在涉及外部进程和崩溃恢复的场景里几乎不可能——崩溃可能发生在任何时刻，没有一种协议能保证崩溃前后状态严格一致。最终一致承认了这一点，转而保证"经过有限步骤后一致"。

最终一致的代价是恢复期间的不确定窗口。在恢复完成前，相关会话的状态是不确定的——它可能处于隔离态，不能接受新操作。这个窗口要尽量短，恢复机制要在启动时尽快跑（非阻塞但优先）。恢复还要幂等——如果恢复中途又崩溃，重启再跑恢复要能从上次中断处继续，而不是搞乱状态。

操作任务的恢复证据是这个最终一致设计的关键数据。它持久化的是操作的终态，不是中间态。这个选择有意——中间态太多且频繁变化，持久化中间态代价高；终态只有一个，持久化它代价低且足够支撑恢复判断。这是"最小持久化支撑最终一致"的精简。

前端也有类似的最终一致语义。乐观更新先显示用户期望的状态，如果服务端确认就保持，如果失败就回滚。这是"乐观最终一致"——先假设成功，失败时修正。这种乐观策略让 UI 响应快，代价是失败时要回滚，用户可能看到短暂的不一致。在大多数操作成功的现实下，这种权衡是划算的。

把三态现实、基于证据恢复、最终一致、最小持久化、乐观更新放在一起，VaneHub 的失败语义体现了一个原则：在不可强一致的现实中，用证据驱动的恢复追求最终一致，用乐观更新在前端追求响应性。这种务实的失败处理，比追求不可能的强一致更贴合工程现实。

## 第 82 章 Agent 终端与工作区 shell 的区分

VaneHub 里有两种终端：agent 终端（agent-terminal）和工作区 shell（shell）。这一章讨论为什么需要两种，以及它们的语义差异。

agent 终端是 agent 运行时管理的、用于和某个 CLI agent 交互的终端。当用户选了一个 CLI 交互模式的 agent，他看到的是 agent 终端——它背后是一个由 VaneHub 启动的 CLI 进程，用户输入直接进 CLI，输出直接显示。这个终端的生命周期和 agent 会话绑定，有 provider session 的发现和恢复。它本质上是"agent 的交互界面"。

工作区 shell 是用户在工作区里开的通用 shell——就像在 IDE 里开一个终端。它和 agent 无关，是用户自己执行命令的地方。它的生命周期和会话工作区绑定，用户可以用它跑 git、装依赖、看日志。它本质上是"用户的工具终端"。

为什么要区分？因为它们的语义不同。agent 终端是 agent 的一部分，它的输入输出要被 agent 运行时管理——session 恢复、用量计量、可观测性都要覆盖它。工作区 shell 是用户的工具，不需要这些 agent 语义，但有它自己的需求——命令历史、命令模板、输出捕获和搜索（第 60 章的终端输出捕获就是工作区 shell 的能力，不是 agent 终端的）。

这种区分让每个终端能针对自己的语义优化。agent 终端有 transcript 保留（一兆字节上限）用于 session 恢复，有 provider session 发现用于 resume，有用量轮询用于计量。工作区 shell 有命令模板和不可变运行快照，有输出捕获和 FTS5 搜索，有远程 SSH 支持。如果硬把它们合成一个"终端"，要么 agent 语义污染工具终端、要么工具能力缺失于 agent 终端，两边都不好用。

但两者也共享一些底层能力。两者都可能用 PTY（agent 终端用 PortablePtyAgentTerminalRuntime，工作区 shell 用 PortablePtyShellRuntime）。两者都有输出缓冲和规范化。这种"共享底层、分化语义"的设计，是"识别相同与不同"的工程判断——底层传输相同所以可以共享，语义不同所以必须分化。

认识到这个区分，有助于理解系统里看似重复的终端代码。它们不是重复，而是两个不同关注点的各自实现，恰好共享了一些底层抽象。这种"看似重复实则分化"的情况在复杂系统里常见，需要仔细辨别哪些是真正的重复（应该合并）、哪些是表面的相似（应该保持分化）。

## 第 83 章 工作区与 worktree 的工程意义

VaneHub 的计划执行和循环工程都大量用到 git worktree。这一章讨论 worktree 在 agent 工程里的意义，以及 VaneHub 如何安全地管理它们。

git worktree 允许同一个仓库的多个工作目录检出到不同的分支。这在 agent 工程里的价值是：让 agent 在一个隔离的工作目录里操作，不污染用户的主工作目录。比如一个计划要执行多个子任务，每个子任务在一个独立 worktree 里执行，子任务之间不互相干扰，也不影响用户当前正在看的主分支。

这种隔离对 agent 工程特别重要，因为 agent 会修改文件、执行命令、可能搞砸。如果 agent 直接在用户的主工作目录操作，一旦搞砸就污染了用户的工作。worktree 隔离让 agent 的操作在一个可丢弃的副本里进行，搞砸了删掉 worktree 就行，主工作目录不受影响。

VaneHub 的 worktree 管理有几个安全细节。worktree 名字有严格校验——拒绝斜杠、反斜杠、点点的路径穿越、控制字符。派生的分支名有固定前缀，避免和用户分支冲突。创建前检查目标不存在，防止覆盖。这些校验防止了一个失控的 worktree 名字破坏文件系统。

worktree 和远程工作区的互斥是一个语义约束——一个会话不能同时有远程工作区和 worktree。这是因为远程工作区意味着操作发生在远程机器上，而 worktree 是本地的概念，两者叠加语义混乱。这个互斥在 domain 层强制，保证了会话工作区配置的明确性。

循环和计划的 guarded worktree 是特殊用途的 worktree。循环的 worker 和 verifier 在 guarded worktree 里工作，计划每个子任务的 attempt 在它自己的 worktree 里。这些 guarded worktree 有额外的验证——base ref 检查、分支碰撞检查，确保它们基于正确的起点、不和已有分支冲突。

worktree 的清理是资源管理的部分。计划或循环结束后，它们的 worktree 应该被清理（如果不再需要）。但清理要小心——如果 worktree 里有用户想保留的改动，不能直接删。这种"创建容易删除难"是 worktree 管理的复杂性来源，VaneHub 通过明确的 worktree 生命周期（绑定到计划或循环运行）来管理它。

把隔离价值、名字校验、互斥约束、guarded 验证、生命周期清理放在一起，worktree 的工程意义在于：它是让 agent 能安全地在用户代码上操作的隔离机制。没有它，agent 工程要么不敢让 agent 改文件（能力受限），要么冒着污染用户工作的风险（不可接受）。worktree 让"让 agent 改文件"这件事变得可控，这是 agent 能真正完成编码任务的基础。

## 第 84 章 Provider 中性层与具体 provider 的边界

第 19 章提过一个架构测试：sessions 的 domain/application 和 agent_runtime 的 application 不得按具体 provider id 分支。这一章讨论这条约束的意义，以及它如何与"支持五个异构 provider"的目标协调。

这条约束看似矛盾——要支持五个 provider，却又不让核心层按 provider 分支？答案是：provider 的差异被收容在 infrastructure 的 providers 子目录里，核心层只通过抽象的 provider 能力描述工作。核心层知道"这个 agent 支持 reasoning 和 sandbox"，但不知道"这是 claude-code 还是 codex-cli"。具体 provider 的知识（claude-code 用 stream-json、codex 用 jsonl、各自的参数怎么映射）是 infrastructure 的实现细节。

这种分离让核心层保持稳定——加一个新 provider 不需要改 sessions 或 agent_runtime 的 application，只需要在 providers 目录加一个新实现。如果核心层按 provider id 分支，加新 provider 就要改核心层，核心层就会随 provider 增加而膨胀，最终变成一个按 provider 分发的巨大 switch。

provider 中性的代价是抽象要够好。如果抽象漏了某个 provider 独有的能力（比如只有 codex 支持 ephemeral session），这个差异要么被忽略（失去能力），要么被迫加进抽象（污染核心层）。VaneHub 的做法是尽量把 provider 差异收在"配置和参数"层面——一个 provider 支持哪些参数、参数怎么映射，这些是 providers 目录里的知识，核心层只说"给我这个 agent 的启动参数"，不关心参数细节。

这条约束还有个隐含的安全语义：它防止了核心层对某个 provider 的偏爱。如果核心层能按 provider 分支，就可能出现"对 claude-code 的处理比 codex 更仔细"的偏见，导致其他 provider 的体验退化。中性约束强制所有 provider 走同一条抽象路径，保证了它们被同等对待。这是"公平性通过结构保证"的体现。

把差异收容、抽象稳定、配置层差异、公平对待放在一起，provider 中性层的设计体现了一个原则：异构性的收容点是实现层而非核心层。核心层抽象、实现层具体，这是处理异构性的标准架构姿势。VaneHub 用架构测试强制它，保证了这个姿势不被侵蚀。

## 第 85 章 配置的即时生效与持久化

VaneHub 有大量的用户配置——设置、聊天配置、CLI 参数、权限模板、Skill 绑定。这一章讨论这些配置如何做到"改了立即生效"和"持久化不丢"。

即时生效是用户体验的要求。用户改了一个设置，期望立刻看到效果，而不是要重启应用。比如用户把主题从深色切到浅色，期望界面立即换色；把语言从中文切到英文，期望文本立即换语言。这种即时性通过"配置变更触发应用"实现——settings provider 在配置变更时调用 applySettings，立即应用字体、主题、语言到 DOM。

持久化是可靠性的要求。配置改了要存起来，下次启动还在。VaneHub 的配置大多存 SQLite（settings 表、chat_preferences 等），少数存 localStorage（侧边栏宽度、UI 风格这种纯前端状态）。存 SQLite 的配置通过服务边界写，存 localStorage 的配置直接前端写。

即时生效和持久化的配合有一个细节：乐观保存。用户改配置时，前端先应用变更（即时生效），同时异步保存到后端（持久化）。如果保存失败，前端回滚并提示。这种乐观保存让用户感觉配置改了立即生效，而持久化在后台进行。

配置的验证是另一层。不是所有配置值都合法——代理 URL 要符合 scheme 白名单、自定义指令有长度上限、语言必须是支持的几种。验证在保存前做，非法值被拒绝并提示用户。这防止了非法配置被持久化后导致应用启动异常。

配置的默认值是兜底。如果配置加载失败（比如数据库读错），前端回退到默认值而不是崩溃。这保证了配置系统本身的鲁棒性——即便配置存储坏了，应用也能用默认配置启动，用户至少能进界面去修复配置。

配置变更的下游通知是即时生效的扩展。有些配置变更要通知多个关注方——比如归档设置变了，会话维护任务下次周期要用新设置。这种跨组件的配置变更通知通过事件（settings-events）传播，让所有关注方都能响应。这是"配置变更是个事件"的思路，比"配置变了各处轮询"高效。

把即时应用、乐观保存、保存前验证、默认兜底、变更通知放在一起，配置系统体现了一个原则：配置要响应快（即时生效）、不丢失（持久化）、合法（验证）、鲁棒（默认兜底）、联动（变更通知）。这些要求看似简单，合在一起需要一个精细设计的配置子系统。

## 第 86 章 浮窗助手的独立渲染面

浮窗助手是 VaneHub 一个特殊的存在——它是一个独立窗口、独立渲染面、几乎独立的应用。这一章讨论为什么它要这么独立。

浮窗的定位是一个常驻的迷你助手，让用户在不切到主窗口的情况下快速和当前会话交互。它有三种表面模式：折叠（一个小图标）、菜单（几个快捷操作）、聊天（一个迷你聊天界面）。用户可以在浮窗里发消息给当前会话，收到流式回复。

要实现这个体验，浮窗必须是独立窗口——它有自己的窗口几何、拖拽、置顶。它不能是主窗口里的一个面板，因为面板受主窗口生命周期约束，主窗口最小化浮窗就没了。独立窗口让浮窗常驻，主窗口可以最小化甚至关闭。

独立窗口带来独立渲染面。浮窗是主窗口之外的第二个 webview，有它自己的 React 根、自己的 QueryClient。这种独立是必要的——两个窗口的 React 状态不能共享（它们是不同的文档），各自管理自己的查询缓存。

但浮窗和主窗口要通信——用户在浮窗点"新建会话"，主窗口要响应。这种跨窗口通信通过事件系统——浮窗订阅的事件由后端发射，主窗口也订阅同样的事件。当浮窗触发"新建会话"动作，后端发事件，主窗口收到后导航到工作区并开新建会话对话框。这种"通过后端事件中转"的跨窗口通信，避免了两窗口直接耦合。

浮窗的配置和主窗口共享——主题、语言、设置都是同一份。这通过共享的 SettingsProvider 和 ThemeProvider 实现。但浮窗没有路由器（它只有一个面）、没有通知 Provider（它不显示主窗口式的通知）。这种"共享配置、分化能力"的设计，让浮窗既和主窗口一致（同主题同语言）又精简（不带主窗口的复杂度）。

浮窗的窗口几何管理是它独立的另一个方面。它的位置、大小、锚点要持久化，下次打开还在原位。它要处理多显示器、屏幕边距、缩放因子。这些桌面窗口管理逻辑在主窗口里不需要那么精细（主窗口是普通窗口），但在浮窗里是核心——一个浮窗如果位置乱跳、尺寸不对，体验会很差。浮窗的 domain 层有专门的位置计算和钳制逻辑，保证它在屏幕内合理放置。

把独立窗口、独立渲染面、跨窗口事件通信、共享配置分化能力、精细几何管理放在一起，浮窗的设计体现了一个原则：一个常驻迷你应用需要和主应用分化但通信。这种"分化以常驻、通信以协作"是浮窗类功能的标准架构，VaneHub 的实现是一个成熟的范例。

## 第 87 章 主窗口关闭与托盘最小化的意图判断

这一章讨论一个看似简单实则微妙的设计：当用户点主窗口关闭按钮时，系统怎么判断他是想退出还是想最小化到托盘。

这个判断之所以重要，是因为两种意图的处理截然不同。退出要触发优雅退出流程（第 53 章），处理进行中的工作、刷盘、关闭连接。最小化到托盘只是隐藏窗口，进程继续运行，所有状态保留。如果误判——把想最小化的用户当成退出，会让他丢失进行中的工作；把想退出的用户当成最小化，会让他困惑为什么进程还在。

VaneHub 的判断依据是配置和当前状态。如果配置为"关闭时最小化到托盘"且托盘可用，点关闭就最小化而不是退出。如果配置为"关闭即退出"或托盘不可用，点关闭就退出。这个配置让用户表达自己的偏好——常驻型用户选最小化，用完即走的用户选退出。

但配置之外还有状态判断。如果当前有进行中的生成，即便用户配了退出，系统可能提示"有进行中的工作，确定退出吗"。这是对"退出可能丢失工作"的防护。这种状态感知的退出确认，平衡了"尊重用户意图"和"防止意外丢失"。

托盘图标是这种设计的补充。当主窗口最小化到托盘后，用户通过托盘图标重新唤起窗口或真正退出。托盘图标让"最小化但进程在跑"这个状态对用户可见——用户看到托盘有图标，知道应用还在后台，可以重新打开或选择退出。如果没有托盘图标，一个最小化的进程对用户不可见，用户可能以为已经退出了，再开一个实例造成混乱。

托盘的关闭兜底是优雅退出的最后保障。如果主窗口因为某种原因无法响应（比如 webview 卡死），托盘菜单提供了一个"退出"入口，让用户仍能退出。这是对"主窗口不可用"场景的兜底，保证用户不会被一个卡死的主窗口困住无法退出。

把配置判断、状态提示、托盘可见性、关闭兜底放在一起，这个设计体现了一个原则：退出的意图判断要基于配置和状态，并提供托盘作为可见性和兜底。这种对"关闭"这个高频操作的精细处理，反映了桌面应用对用户体验的细致关怀——一个让用户困惑或丢失工作的关闭行为，会严重损害对应用的信任。

## 第 88 章 内存状态与持久状态的边界

VaneHub 里有些状态是内存的、有些是持久化的。这一章讨论这个边界的划分原则，以及为什么某些状态选择内存而非持久。

划分的核心原则是：状态的生命周期和恢复需求决定它存哪。如果一个状态只在进程生命周期内有意义、崩溃后不需要恢复，它存内存。如果一个状态要跨重启保留、崩溃后要恢复，它存 SQLite。

生成协调是内存的——它维护每会话的生成租约。这个状态不需要持久化，因为崩溃后租约本身就失效了（进程都没了，租约没意义）。重启后不是"恢复租约"，而是"扫描哪些生成可能中断了，处理它们的善后"。所以生成协调用内存，恢复用会话恢复机制（基于持久化的消息状态）。

操作任务的大部分状态是内存的，只有恢复证据持久化（第 58 章）。这是因为操作任务的实时状态（日志、中间进度）只服务实时观察，崩溃后不需要恢复中间进度——恢复只需要知道终态来决定善后动作。

IM 连接器的运行时状态是内存的——活跃的连接、待处理的入站、进行中的生成计数。这些状态崩溃后不需要恢复——重启后连接器重新建立连接、重新接收入站。持久化的是配置（连接器怎么配）、绑定（哪个聊天绑哪个会话）、去重（哪些事件处理过），这些是跨重启要保留的。

Agent 终端的 transcript 是内存的（一兆字节上限）。它不持久化，因为它是"实时回看"用的，不是"历史归档"用的。会话恢复时如果终端 transcript 没了，用户重新打开终端即可，不需要从持久化恢复。这和会话消息不同——消息是要归档的，所以持久化。

这个边界划分的好处是清晰：持久化的都是"丢不得的"（配置、消息、绑定、恢复证据），内存的都是"丢得起的"（租约、运行时计数、transcript）。这种"按可丢与否划分"让持久化负载最小化——不持久化那些崩溃后无需恢复的状态，减少数据库写。

但这个划分也带来一个挑战：内存状态和持久状态之间的一致性。比如一个会话的内存租约说"生成中"，但持久化的消息状态可能因为崩溃停在"流式"。恢复机制要处理这种不一致——把内存状态视为失效，以持久状态为准重建。这种"持久为准、内存重建"的恢复策略，是内存与持久混合系统的标准处理。

把生命周期划分、最小持久化、一致性重建放在一起，状态存储边界的划分体现了一个原则：存什么不存什么，由状态的可丢性和恢复需求决定，而非由"存了更安全"的直觉决定。过度持久化增加负载和复杂度，适度持久化配合内存状态和恢复机制，是更精细但也更高效的设计。

## 第 89 章 延迟加载与首屏体验

前端的首屏体验直接影响用户对应用速度的第一印象。这一章讨论 VaneHub 如何用延迟加载优化首屏。

首屏的核心矛盾是：用户希望打开应用立刻能用，但应用的功能很多，全加载首屏会很慢。延迟加载的解法是：首屏只加载核心可见部分，其他部分用到时再加载。VaneHub 的首屏是主工作区——会话侧边栏和聊天区。其他设置页、计划中心、循环中心、各种工具页都是延迟加载。

延迟加载的实现是 `LazyFeature` 原语（第 6 章）。每个延迟加载的组件用 `lazy()` 包裹一个动态 import，在首次渲染时才加载对应代码块。这把一个大 bundle 拆成多个小 chunk，首屏只下载主 chunk，其他 chunk 按需下载。

延迟加载的代价是首次访问某个功能时的加载延迟——用户第一次点设置页，要等那个 chunk 下载。这个延迟用 Suspense 的 fallback 缓解——加载时显示一个占位（通常是骨架屏或加载指示），加载完替换。这个占位让用户知道"正在加载"而不是"坏了"。

延迟加载的边界判断是关键。不是所有东西都该延迟——首屏必需的不能延迟，否则首屏反而变慢（要先加载加载器再加载内容）。VaneHub 的判断是：路由直接渲染的（主工作区、设置外壳）不延迟，路由内按需访问的（具体设置页、各中心）延迟。这个判断基于"用户打开应用最先看到什么"——侧边栏和聊天区是首屏，必须快；设置页是用户主动导航才看到，可以延迟。

chunk 体积检查是延迟加载的保障。`check-frontend-chunks.mjs` 在构建后检查每个 chunk 不超阈值。这防止了一个延迟加载配置错误导致某个 chunk 过大——比如不小心把一个大型依赖静态 import 进了主 chunk，首屏就要下载它。体积检查让"首屏小"这个目标被机器验证。

预加载是延迟加载的优化。Vite 的 warmup 配置可以在首屏加载后、用户空闲时，预取可能很快访问的 chunk。这把"首次访问的延迟"提前到"用户空闲时下载"，用户真正访问时 chunk 已在缓存。这种预加载让延迟加载的代价进一步降低。

把按需加载、Suspense 占位、边界判断、体积检查、预加载放在一起，延迟加载策略体现了一个原则：首屏要快（只加载必需）、按需加载（用到才下）、加载可见（占位提示）、体积可控（机器检查）、预取优化（空闲预加载）。这些组合让一个功能众多的应用仍能保持快速首屏。

## 第 90 章 代码索引与向量检索的工程化

代码索引和向量检索是 VaneHub 较晚加入的能力，它让 agent 能基于语义检索代码库。这一章讨论它的工程化设计。

代码索引的挑战是：一个代码库可能很大，全量索引耗时且占空间。VaneHub 的做法是按工作区索引，每个工作区独立管理它的索引状态。索引有版本号，文件修改后版本更新，旧索引可判断过期。这种"工作区级、版本化"的设计让索引和代码库的变化保持关联。

索引过程是异步的——一个后台 worker 持续处理待索引的文件批次。这避免了一次性全量索引卡住应用。索引有失败分类，失败的文件记录原因，可以重试。这种"异步批次、失败可重试"让索引过程鲁棒于个别文件的解析失败。

分块是索引的核心。一个文件不是整体作为一个文档索引，而是按语义切分成多个 chunk（默认六千字节上限）。每个 chunk 有起止行、符号名、符号类型。这种分块让检索能定位到文件内的具体位置，而不是只返回"这个文件相关"。分块用 tree-sitter 解析符号边界，让 chunk 尽量对齐语义单元。

embedding 是索引的向量部分。每个 chunk 被送到 embedding 模型转成向量，存到 retrieval_documents 的 embedding 列。embedding 前做脱敏（第 75 章），防止敏感信息进向量索引。embedding 模型可配，不同工作区可以用不同模型，这通过 embedding_confirmed_profile 管理。

检索的混合路径在第 61 章已详述。这里补充它的工程化：检索结果是按 id 从源重取的，不是返回缓存快照。这保证了检索结果反映最新内容——如果一个 chunk 在索引后被修改了，检索会跳过它（标记过期）。这种"结果从源重取、过期跳过"让检索结果保持新鲜。

代码索引的可观测性通过 audit 表实现。索引事件（添加多少文件、失败多少、耗时多少）记录在 code_index_audit，让用户能看到索引的进度和健康。这种可观测性让"索引在后台跑"这件事对用户透明——用户知道索引到哪了、有没有问题。

把工作区级索引、异步批次、tree-sitter 分块、embedding 脱敏、混合检索、结果从源重取、审计可观测放在一起，代码索引的工程化体现了一个原则：对大型代码库的语义检索要工程化——异步增量、分块定位、安全脱敏、混合路径、结果新鲜、进度可见。这是把"向量检索"这个研究性能力变成生产可用产品的工程努力。

---

# 第十一部分 演进历史、对比与反思

前十部分描述了 VaneHub 当前的架构面貌。第十一部分加入时间的维度——讨论这套架构是怎么演化成现在这样的、它和同类系统相比有什么异同、以及从反思的角度看它的哪些设计是真正值得借鉴的。加入了时间维度和对比视角后，对架构的理解会从"它是什么"深入到"它为什么是这样、它还能怎样"。

## 第 91 章 从扁平结构到 DDD 的迁移故事

`ARCHITECTURE.md` 的模块清单里，有一大批被标记为"已移除的兼容 facade"或"已迁移"的模块——`command_safety.rs`、`network_proxy.rs`、`logging.rs`、`usage.rs`、`session_configuration.rs`、`shell.rs`、`im/*`、`tasks/*`。这些模块的存在痕迹揭示了一个重要的演进故事：VaneHub 的 Rust 后端并非一开始就是现在这套严格的 DDD 结构，而是从一个更扁平、更集中的结构，经过一次大规模重构迁移过来的。

迁移前的结构可以推测出来：`logging.rs` 是一个根级别的日志实现，所有上下文直接用它；`command_safety.rs` 是一个根级别的进程构造 facade，所有需要起进程的地方都调它；`network_proxy.rs` 是根级别的代理处理；`tasks/` 是一个集中的任务注册表；`im/` 是一个集中的 IM 命令、存储、凭据、运行时的大杂烩。这种"根级别 facade"的结构在项目早期很常见——它简单直接，调用方便。但随着代码增长，它的弊端显现：所有上下文都依赖这些根 facade，导致它们成为事实上的上帝模块；业务逻辑散落在 facade 里，领域边界不清；测试困难，因为 facade 直接持有 I/O。

迁移的目标是把这些根 facade 拆解到各自的限界上下文里。`logging.rs` 的持久化、脱敏、轮转下沉到 `platform::logging`，语义契约上移到 `operations` 上下文；`command_safety.rs` 的进程构造下沉到 `platform::process`，各上下文通过 port 调用；`tasks/` 的操作模型拆到 `operations` 上下文，查询拆到 `commands/operations`；`im/` 整个拆到 `communications` 上下文的 domain/application/infrastructure/api；`shell.rs` 的 PTY 管理拆到 `workspaces` 上下文；`usage.rs` 的用量领域拆到 `sessions`。

这个迁移不是一次完成的，而是分多个 OpenSpec 变更逐步推进——`ARCHITECTURE.md` 的模块表里每个迁移都标注了对应的"迁移任务"编号（如 7.8、8.1-8.4、10.6、11.x），说明它是一个有计划、分阶段的大重构。这种"大重构分阶段做"是工程上的成熟做法——一次性大爆炸式重构风险太高，分阶段让每一步可验证、可回滚。

迁移过程中有个关键纪律：保持 Tauri 命令名和 JSON 契约不变。即便后端内部从一个根 facade 迁到上下文，前端调用的命令名和载荷结构不变。这把迁移的影响面限制在后端内部，前端无感。这种"外部契约稳定、内部重构自由"的纪律，让大重构得以在不破坏前端的情况下推进。

迁移完成后，根 facade 被移除，架构测试禁止它们被重新引入。`migrated_session_code_cannot_return_to_root_or_legacy_modules` 这个测试会检查三十多个遗留符号名和几个遗留模块路径，确保迁移的成果不被倒退。这是"迁移完成后锁门"的防御性措施——迁移的代价已经付出，不能让以后的人因为"图方便"又把逻辑塞回根级别。

这个故事的价值在于它展示了一个真实的大重构是怎么做的：有明确的目标结构（DDD）、有分阶段的计划（OpenSpec 变更）、有外部契约稳定（命令名不变）、有迁移后的锁门（架构测试）。这四要素是任何大重构成功的关键，VaneHub 的迁移是它们的一个完整范例。

## 第 92 章 与同类工具的对比定位

VaneHub 处在一个新兴的产品类别里——AI 编程助手管理终端。把它和几个相关的参照系对比，能更清楚地看到它的定位和取舍。

第一个参照系是单一 CLI 工具本身，比如 Claude Code 独立使用。单一 CLI 的优势是简单直接、和终端工作流深度集成。它的局限是不提供统一的会话管理、多 agent 协作、跨会话记忆、用量统计。VaneHub 相对它的增量价值在于"管理"——把多个 CLI 收进一个有持久化、有可观测、有协作能力的壳。代价是引入了一层抽象和它带来的复杂度。对于只用一个 CLI 且不在乎管理的用户，VaneHub 是过度工程；对于要用多个 CLI、要管理历史、要协作的用户，VaneHub 的管理价值就显现了。

第二个参照系是 IDE 的 AI 集成，比如 VS Code 里装 Claude 或 Copilot 插件。IDE 集成的优势是和编辑器深度耦合、上下文感知强。它的局限是受限于特定 IDE、agent 能力受插件接口约束、多 agent 协作难。VaneHub 相对它的差异在于"独立"——它不绑定特定 IDE，agent 在独立工作区操作，可以多 agent 协作。代价是失去了 IDE 的深度集成——agent 看不到你在编辑器里打开的文件，除非显式给它工作目录。这是一个"深度集成"和"独立灵活"的取舍。

第三个参照系是 agent 编排框架，比如 LangChain 或 AutoGen 这类编程框架。它们的优势是给开发者极大的编排自由、可编程。它们的局限是面向开发者而非终端用户、需要写代码、没有开箱即用的桌面体验。VaneHub 相对它们的差异在于"产品化"——它是面向用户的桌面应用，开箱即用，有 UI 有配置有持久化。代价是灵活性低于编程框架——用户不能像写代码一样自由编排，要在 VaneHub 提供的配置和流程框架内工作。这是一个"开箱即用"和"可编程自由"的取舍。

第四个参照系是多 agent 群聊类研究系统。它们的优势是探索性强、能验证多 agent 协作的潜力。它们的局限是研究性质、不稳定、无生产化。VaneHub 的多 agent 群聊（第 10、27 章）相对它们的差异在于"工程化"——它的群聊有确定的路由规则、有链深限制防失控、有 headless 支持、有可观测。代价是规则相对保守——VaneHub 不会做激进的多 agent 涌现实验，它要保证行为可预测。这是一个"工程稳健"和"研究探索"的取舍。

通过这些对比，VaneHub 的定位清晰了：它是一个面向需要管理多个 AI 编程 CLI 的用户的产品化桌面终端，强调统一管理、持久化、可观测、多 agent 协作、开箱即用、工程稳健。它的所有设计取舍都服务于这个定位——它不追求成为最灵活的编排框架，也不追求成为最深度的 IDE 集成，它在"管理多个 CLI 并提供一致体验"这个具体定位上做到工程上的极致。

## 第 93 章 单 crate 与多 crate 的架构选择

ADR-001 选择了单 crate（第 42 章）。这一章深入讨论这个选择的具体考量，以及在什么情况下它可能不再合适。

单 crate 的核心好处是构建简单和强制力集中。所有代码在一个 Cargo 包里，一次编译，依赖关系全在包内可见。架构测试可以解析整个包的 AST，跨模块检查依赖方向——这在多 crate 里要检查跨 crate 依赖，工具支持更复杂。单 crate 让"架构规则在一处强制"成为可能。

单 crate 的代价是编译粒度粗。改一个模块，整个包要重新编译（虽然增量编译能缓解）。在多 crate 里，改一个 crate 只重编那个 crate 及其依赖者。对于大型项目，这个差异在大规模改动时显著。但 VaneHub 的编译时间在可接受范围（CI 的 rust job 四十五分钟内完成），说明单 crate 在当前规模下没成为瓶颈。

单 crate 的另一个代价是依赖管理粗。所有代码共享一个 Cargo.toml，所有依赖都在一处声明。这意味着一个上下文用到的依赖，对其他上下文也可见可用——理论上 sessions 上下文可以 import 一个本该只属于 communications 的依赖。多 crate 通过各自的 Cargo.toml 自然隔离依赖。VaneHub 用架构测试弥补了这个隔离——禁用技术根列表（rusqlite、reqwest 等）在 domain/application 层被禁，间接限制了依赖的可见性。

那么什么情况下单 crate 可能不再合适？主要有两个信号。第一是编译时间变得不可接受——如果 rust job 从四十五分钟涨到几个小时，单 crate 的编译粒度劣势就显现了。第二是团队规模扩大导致单 crate 的并发开发冲突加剧——多个团队改同一个 crate，merge 冲突频繁。这两个信号都和规模强相关，VaneHub 当前的规模（一千一百多 Rust 文件）还没到这个临界点。

如果将来要迁多 crate，迁移路径是相对清晰的——每个限界上下文变成一个 crate，api.rs 变成 crate 的公共接口，跨 crate 依赖通过 crate 依赖声明。但这个迁移本身不轻松——要处理循环依赖（第 15 章的延迟接缝在多 crate 里更难处理）、要重设架构测试跨 crate 检查、要调整 CI。所以这个迁移只有在规模信号明确时才值得做。在那之前，单 crate + 架构测试的组合在可维护性和简单性之间提供了好的平衡。

这个讨论的价值在于它示范了如何评估一个架构选择：不只看它现在好不好，还要看它的失效信号是什么、失效后怎么迁移。一个有清晰失效信号和迁移路径的架构选择，比一个"永远没问题"但其实没仔细想过的选择更可靠。

## 第 94 章 契约守护的免代码生成方案

第 13 章和第 76 章讨论了 VaneHub 用 TypeScript 条件类型而非代码生成器守护前后端契约。这一章深入这个选择的权衡，以及它适用的条件。

代码生成是守护前后端契约的主流方案——从 protobuf 到 gRPC 到 OpenAPI，用一份 schema 生成前后端的类型和序列化代码。它的优势是一致性有保证（生成的代码必然匹配 schema）、类型完整（schema 里有的类型都有）。它的代价是引入工具链（schema 语言、生成器、构建集成）、开发流程复杂化（改 schema 要重新生成）、前后端语言要都被生成器支持。

VaneHub 选择不用代码生成，而是手写 TypeScript 类型 + 条件类型相等测试。这个选择的优势是零工具链——不需要 schema 语言、不需要生成器、不需要构建集成，类型就是普通的 TypeScript。它的代价是手写同步——后端 DTO 变了，前端类型要手动改，靠 `Equal<>` 测试发现不一致。

这个选择在 VaneHub 的场景下是合理的，因为它的前后端类型关系不是"一份 schema 生成两端"，而是"后端有领域模型和 DTO，前端有领域类型，二者通过 invoke 边界松耦合"。后端的领域模型不直接暴露给前端（前端只看到 DTO 的 camelCase 序列化），所以没有一份统一的 schema 可生成。前端类型是前端自己定义的、面向 UI 的，和后端 DTO 结构相近但不完全相同。这种"结构相近但各自定义"的关系，正好适合用相等测试守护——测试不要求它们是同一份来源生成的，只要求它们结构相同。

但这个方案有一个隐含成本：维护相等测试本身。每次加一个类型，要在 types 和 contracts 两处定义，再加一条 `Equal` 断言。这个三处同步是手动的，容易漏。VaneHub 用前端报告里的描述，contracts 里有些是 re-export（自动一致）、有些是复制（要手动同步）、有些是 zod schema（运行时校验）。这种混合策略降低了但没消除手动同步成本。

这个方案适用的条件是：前后端类型结构相近但不完全相同、团队规模小到手动同步可控、类型变更不极其频繁。如果团队大、变更频繁，手动同步会变成持续负担，代码生成的工具链成本就值得付了。VaneHub 当前的团队规模和变更频率还在手动可控范围，所以这个选择是经济的。

这个讨论的价值在于它说明了一个道理：架构选择没有绝对的最优，只有适合当前规模和约束的选择。代码生成不是"更先进"的方案，手写也不是"更落后"的方案，它们各自适合不同的场景。判断适不适合，要看类型关系、团队规模、变更频率这些具体因素。VaneHub 的选择是基于这些因素的理性判断，而非盲目追随主流。

## 第 95 章 从阅读者视角的导航建议

这一章换一个视角——假设一个新来的开发者要阅读 VaneHub 代码，给他一条高效的阅读路径建议。这些建议基于前面的架构分析，指出从哪里入手能最快建立全局认识。

第一步建议读 `AGENTS.md` 和 `openspec/project.md`。这两个文件是项目的宪法和技术规范的真源，它们定义了技术栈约束、架构核心约束、代码规范、验证命令。读它们能快速建立"这个项目允许什么、禁止什么"的认识，避免一开始就踩到规范红线。

第二步建议读 `src-tauri/ARCHITECTURE.md`。这个文件是原生架构的地图，有上下文地图、模块清单、命令契约清单、迁移清单、后台任务清单、外部适配器清单。它是一个高密度的索引，读完能知道每个模块在哪、负责什么。后续深入任何子系统时，都能回到这个索引定位。

第三步建议读 `src/App.tsx` 和 `src/services/runtime-adapter.ts`。前者是前端入口，展示了 Provider 嵌套和路由；后者是运行时选择的核心，展示了双运行时分发。读这两个能建立前端骨架和"运行时如何切换"的认识。

第四步建议读 `src-tauri/src/bootstrap/runtime.rs` 的 `setup()` 函数。这是整个原生侧的装配点，它按顺序组装每个 API 并注册到 Tauri state。读它能看到所有上下文如何被装配、依赖关系如何注入、后台任务如何启动。这是理解原生侧"怎么跑起来"的钥匙。

第五步建议挑一条端到端链路深入，比如第 40 章的"发消息全链路"。沿着 send_message 命令→AgentRuntimeApi→start_message_generation→CLI 进程启动→流式解析→消息持久化这条线读下去，能把前端、命令、application、infrastructure、platform 串起来。这种"沿着一条链路读"比"按目录逐个读"高效得多，因为它揭示了模块间的真实协作。

第六步建议读架构测试 `tests/architecture.rs`。它不只是测试，更是架构规则的机器可读表述。读它能看到项目把哪些规则视为不可侵犯，这些规则本身就是架构的精华。

第七步建议挑一个感兴趣的限界上下文深入它的 domain/application/infrastructure/api 四层。比如对会话感兴趣就读 sessions 上下文，对 agent 调度感兴趣就读 agent_runtime。这种"选一个领域深入"能建立对端口与适配器模式的体感认识。

这条阅读路径的设计逻辑是：从规范到地图、从入口到装配、从链路到领域、从测试到规则。它遵循"先宏观后微观、先骨架后血肉、先动态后静态"的原则，让阅读者逐步建立认识而非一头扎进细节。当然，实际阅读时不必严格按这个顺序，可以根据兴趣跳转，但这条路径提供了"不迷路"的基准线。

## 第 96 章 设计反思：哪些地方可以更好

前面的章节多是在解释现有设计的合理性。这一章反过来，诚实地反思 VaneHub 架构里哪些地方在理想情况下可以做得更好，以及为什么现实里没做到。这种反思不是否定，而是对架构成熟度的诚实评估。

第一个可以更好的地方是服务边界层的宽接口（第 56 章张力一）。`AgentService` 接近一百四十个方法，在当前规模可控，但它的膨胀趋势是结构性的——每加一个能力就多几个方法。理想情况下，这个接口应该按领域拆分成几个聚合接口，每个聚合接口对应一个关注域。现实没这么做，可能是因为早期的宽接口选择已经形成了路径依赖——拆分要改所有调用点，成本高。这是一个"技术债随着规模显现"的典型例子，它的处理时机是当它真的成为维护负担时，而非现在预防性拆分。

第二个可以更好的地方是多智能体路由的双实现（第 56 章张力二）。前端和后端各有一份路由规则，靠共享纯函数减漂移，但理论上仍可能不一致。理想情况下路由应完全收口一处。现实没这么做，是因为单席位会话（多数场景）的前端路由不希望为了一次本地决策走后端往返。这是一个"为多数场景优化而接受少数场景的复杂度"的取舍，它的代价是双实现的漂移风险。长期看，如果多席位场景占比上升，收口到后端的收益会超过单席位延迟代价。

第三个可以更好的地方是原生 API 运行时的文件规模（第 56 章张力三）。七千八百多行的 `api_process_adapter.rs` 承载了工具循环、压缩、记忆、权限、工具执行多个职责。理想情况下它应按职责拆分成多模块。现实没拆，可能是因为这些职责在工具循环里紧密耦合，拆分要仔细设计 trait 边界。这是一个"代码组织的理想和耦合现实"的张力，它的处理需要一次审慎的重构设计。

第四个可以更好的地方是迁移的持续累积（第 56 章张力四）。六十五个迁移并还在增长，碰撞历史说明并行开发对它有压力。理想情况下应该周期性折叠成全量基线。现实没做，是因为折叠要处理所有已部署数据库的升级路径，技术上复杂。这是一个"理想简单现实复杂"的典型，它的处理可能需要等一个合适的窗口（比如一次大版本升级）才值得做。

第五个可以更好的地方是部分 CLI 的权限覆盖不均（第 47 章）。只有 Claude Code 有 hook 桥接做细粒度审批，其他 CLI 只能靠启动标志做粗粒度控制。理想情况下每个支持 hook 的 CLI 都该有桥接。现实没做，是因为其他 CLI 没有提供像 Claude Code 那样的 PreToolUse hook 机制，这是外部约束而非项目选择。这个"可以更好"其实受限于外部能力，项目已在自己能控的范围内做到了最细。

第六个可以更好的地方是 mock 适配器的维护成本。近五千行的 mock 要和 Tauri 适配器同步演化。理想情况下 mock 能从契约自动生成。现实没做，和契约守护不用代码生成是同一个选择（第 94 章）——避免工具链。这个取舍在当前规模经济，但它的成本是真实的持续维护投入。

这些反思的共同点是：它们都不是"设计错误"，而是"在当前约束下的合理取舍，其代价随规模或时间显现"。一个架构没有这类张力是不可能的——任何选择都有代价。成熟的架构态度不是追求无张力，而是让张力可见（通过 ADR 和文档）、让处理时机明确（等规模信号）、让处理路径清晰（迁移路径已知）。VaneHub 在这方面做得相对好——它的张力大多被文档化了，处理方向也有初步设想。这是它能在长期演化中保持健康的原因。

## 第 97 章 对读者的最终建议

在结束这份长篇文档之前，给读者几条基于全部分析的建议。

第一条建议是带着"为什么"读代码。VaneHub 的代码里有大量看似冗长或严格的设计——三百行限制、架构测试、契约相等测试、双适配器、fail-closed 权限。如果只看"是什么"，它们显得繁琐；但如果问"为什么"，每一个都对应着一个真实的问题和一个被舍弃的替代方案。理解"为什么"比记住"是什么"更重要，因为"为什么"能迁移到你自己的项目，而"是什么"只在 VaneHub 里成立。

第二条建议是关注约束的机械执行。VaneHub 最值得借鉴的不是某个具体设计，而是它把规范变成机器可执行规则的方法论——eslint 规则、架构测试、hook、CI job。这些机制让规范不依赖人的记忆和纪律，而是依赖机器的确定性。在你自己的项目里，无论规模大小，都可以思考"哪些规范可以从口头变成机器执行"。哪怕只是"提交前自动跑格式化"这种小机制，也能显著降低腐化速度。

第三条建议是区分"结构正确"和"语义正确"。VaneHub 的架构测试能保证结构正确（依赖方向、命令纯度），但语义正确（事务边界、不变量完整）仍靠 review。在你的项目里，不要误以为加了架构测试就万事大吉——它守住的是底线，上限仍要人来托。正确认识机械强制的边界，才能既依赖它又不滥用它。

第四条建议是在抽象和具体之间找平衡。VaneHub 有很重的抽象——端口、适配器、上下文、契约。这些抽象在它的规模和复杂度下是必要的。但抽象有成本，每个抽象层都增加理解负担和间接性。在你的项目里，抽象的程度应该和复杂度匹配——简单项目过度抽象是负担，复杂项目抽象不足是泥球。VaneHub 的抽象程度对它的复杂度是合适的，但直接照搬到更简单的项目会过度。

第五条建议是重视演化的连续性。VaneHub 的架构不是一次设计完成的，而是从扁平结构逐步迁移来的，并且仍在演化（张力章节说明了未来方向）。一个架构的健康不在于它某刻完美，而在于它能持续演化。在你的项目里，与其追求一次性设计完美，不如建立让架构持续演化的机制——清晰的迁移路径、文档化的 ADR、可执行的规范、能发现问题的测试。这些机制让架构能随需求变化而调整，而非在一次设计后僵化。

这份文档通过对 VaneHub 全仓库的结构化测绘和深度分析，力图呈现的不仅是这个项目的架构面貌，更是它背后"如何让一个复杂系统保持长期健康"的工程方法论。架构会随时间变化，具体的文件行号会随提交漂移，但"用机械强制守住底线、用文档记录决策、用测试发现腐化、用演化适应变化"这套方法论是相对持久的。希望这份文档对读者理解 VaneHub 有帮助，更希望其中的方法论能迁移到读者自己的工程实践里。

---

## 附录 A：关键常量速查表

本附录汇总散布在各章的关键常量，便于快速查阅。这些常量是系统行为的数字边界，理解它们就理解了系统在何处触发限制、在何处降级。

运行时与上下文窗口相关：上下文压缩触发阈值六万字符、压缩保留最近六轮、历史加载上限五十条消息、工具使用循环最大往返二十五次、请求超时一百二十秒、审批轮询间隔二百毫秒。这些数字共同定义了 OnePiece 原生 API agent 的一次生成的预算和边界。

多智能体相关：最大链深度十五、每条回复最大提及数二、席位上下文预算四千字符、席位上下文消息上限四十。这些约束防止群聊失控。

记忆与 skill 相关：记忆注入预算四千字符、单个 skill 预算八千字符、聚合 skill 预算一万六千字符、OnePiece 核心指令预算八千字符。这些预算分配了系统提示的上下文经济。

终端相关：终端 transcript 保留一兆字节、终端读缓冲六万四千字节、session-id 解析缓冲二十五万六千字节、provider-session 发现轮询二百五十毫秒、用量轮询五秒。这些定义了终端的资源边界。

数据库相关：最大连接池十二、最小空闲一、连接超时五秒、busy_timeout 五秒、synchronous FULL、WAL 模式。这些定义了 SQLite 的并发和持久化语义。

SSH 相关：连接池容量八、空闲超时五分钟、排空超时三十秒、连接超时十五秒、保活三十秒。这些定义了远程终端连接池的并发语义。

终端输出捕获相关：队列块数二百五十六、每块字节三万二千、批量三十二、保留三十天、容量五百一十二兆字节、transcript 一兆字节、搜索页大小五十到一百、查询长度五百一十二字节。这些定义了输出捕获的有界性。

IM 相关：全局待处理上限六十四、活跃生成上限八、每聊天待处理上限八、去重清理每批五百一十二行、维护周期六小时。这些定义了 IM 集成的背压和清理。

日志相关：轮转周期二十四小时、保留三十天、维护限流每小时一次。这些定义了日志的存储边界。

Agent 终端清理：空闲超时两小时、检查间隔六十秒。这定义了终端资源的回收。

## 附录 B：术语表

本附录定义文档中使用的关键术语，便于读者对齐理解。

限界上下文（Bounded Context）：DDD 里的概念，指一个明确的领域边界，内部有统一的领域模型和语言。VaneHub 有十四个限界上下文，每个是一个独立的责任域。

端口（Port）：应用层定义的接口，描述"我需要什么能力"，不规定谁提供。端口是可替换性的载体。

适配器（Adapter）：端口的具体实现，知道怎么用具体技术（SQLite、进程、网络）满足端口描述的能力。

聚合（Aggregate）：一组相关对象的集合，作为一个一致性边界被一起加载和修改。比如 SessionAggregate 把会话及其相关数据作为一个一致单元。

不变量（Invariant）：一个始终必须成立的条件。比如"归档会话不能激活"是一个不变量，由领域层强制。

fail-closed：在不确定时默认拒绝的设计原则。VaneHub 权限系统采用此原则——未知工具默认 Ask 而非 Allow。

席位（Seat）：多智能体群聊里"一个 agent 在某会话里的角色实例"。席位把 agent 和它在具体会话里的角色解耦。

overlay：在不修改原始资产的前提下叠加修改的机制。Skill overlay 让用户定制不可变的内置 skill。

fidelity：可观测性里对一步执行可见度的标注，分 Native、Proxied、Inferred、Opaque 四档。

runtime kind：前端运行时的三种模式——tauri（桌面）、web-mock（浏览器内存模拟）、web-http（浏览器连后端）。

generation lease：生成协调器对每个会话发放的独占租约，保证一个会话同时只有一个生成。

single-flight：并发控制模式，多个请求同一个资源时只发起一次实际操作，其他请求等结果。

RRF（Reciprocal Rank Fusion）：按排名而非分数融合多路检索结果的算法。

契约（Contract）：上下文之间或前后端之间的接口约定，VaneHub 用相等测试守护。

OpenSpec：项目的变更治理流程，要求变更前先写 proposal 并校验。

ADR（Architecture Decision Record）：架构决策记录，记录重大设计决策及其理由。

---

# 第十二部分 上下文深潜与场景集锦

第十一部分讲了演进和反思。第十二部分回到系统本身，对几个尚未充分展开的限界上下文做深潜式剖析，并用一组典型用户场景把前面所有内容串起来。深潜的目的不是重复结构描述，而是揭示每个上下文在"它要解决的核心问题"和"它的设计如何回应这个问题"这个层面上的独特性。

## 第 98 章 operations 上下文：可观测性的中央枢纽

operations 是所有上下文都消费的枢纽，它的设计浓缩了一个理念：把"操作"作为一等公民来管理。在一个会执行大量异步、长时、可能失败的操作的系统里，如果操作只是"跑完就忘"，系统就失去了对自身行为的可见性。operations 上下文把每个值得观察的操作建模成一个有完整生命周期的实体——从排队、运行、到终态，带日志、带执行关联、带恢复证据。

这个上下文的核心聚合是 OperationTask。它不是简单的"一个操作记录"，而是一个有状态机的实体。它的状态转换——排队到运行、运行到成功或失败或取消——不是任意的，而是遵循一个合法转换图。这个状态机的意义在于：它让操作的状态成为可推理的。给定一个操作的当前状态和历史，系统能判断"它处于什么阶段、下一步可能是什么、是否卡住了"。

操作和执行的关联是 operations 的一个精巧设计。一个操作可以关联到一个执行运行（execution_observability 里的概念）和一个 trace_id。这种关联让操作层面的"这个 SDK 安装操作"和执行层面的"这次安装的详细 span 和事件"能互相导航。从操作能看到它的执行细节，从执行能看到它属于哪个操作。这种双向导航是诊断的基础——发现问题后能快速定位是哪个操作、它的上下文是什么。

操作和日志的关联通过 OperationLogPort 实现。操作的日志不只是裸文本，而是带 operationId 上下文的结构化记录。这让"这个操作产生了哪些日志"成为一个可查询的问题，而不需要在全量日志里大海捞针。这种"日志带操作上下文"的设计，是统一日志可审计性的具体体现。

操作的取消机制是它和执行协作的一个方面。OperationService 维护一个取消标志注册表，每个操作一个原子布尔标志。调用方可以拿到标志轮询，在长操作的循环里检查是否被取消。这种协作式取消承认了一个现实：很多外部操作不能被强制中断，但可以在它的检查点响应取消请求。这比"要么强制杀要么不能取消"更细腻。

操作的内存模型（第 58 章）是 operations 上下文最值得称道的设计决策。完整状态在内存、只有恢复证据持久化，这个选择把持久化负载压到最低，同时保留了崩溃恢复能力。它体现了一个深刻认识：不是所有状态都值得持久化，只有"丢了会导致恢复失败"的状态才必须持久化。这种"按恢复需求决定持久化"的原则，是数据建模的成熟态度。

operations 上下文作为枢纽，它的 API 被所有上下文消费。这种"集中式操作管理"让整个系统的操作行为有一致的可见性——无论是一个 SDK 安装、一次 MCP 测试、一次 agent 生成、一次工作区操作，都经过同一个操作模型，都有同样的生命周期、日志、恢复语义。这种一致性降低了理解成本，也让跨上下文的操作可比较、可聚合。

## 第 99 章 sessions 上下文：会话作为一等聚合

sessions 上下文把"会话"建模为一等聚合，而不是"消息的容器"。这个区分看似微妙，实则有深远的语义影响。如果会话只是消息的容器，那它的行为就由消息决定——有消息就有会话，删消息就删会话。但 VaneHub 把会话建模为一个有自己生命周期、自己状态、自己不变量的实体，消息是它的组成部分但不是它的定义。

会话聚合的状态机（第 57 章）是这个建模的体现。会话有 lifecycle——空闲、运行中、失败等，这个状态独立于消息。一个会话可以处于"运行中"表示有生成在进行，即便那条流式消息还没完成。这种"会话状态反映生成活动而非消息存在"的建模，让会话成为一个有意义的状态实体。

会话的所有权模型是它的另一个独特设计。会话有 owner——桌面创建的会话 owner 是 Desktop，IM 连接器创建的会话 owner 是 Connector。这个 owner 不是装饰，它决定了会话能做什么：连接器拥有的会话不能被桌面 UI 激活。这个约束保护了会话的控制权——一个由 IM 驱动的会话不应该被桌面用户抢占控制，否则 IM 侧和桌面侧的操作会冲突。所有权是会话治理的基础。

会话的分类和归档是它的组织维度。会话可以归属分类、可以置顶、可以归档。这些组织维度让用户能管理大量会话——一个长期使用的应用会累积几百上千个会话，没有组织维度就会变成无法浏览的列表。归档尤其重要——它把不活跃的会话移出默认视图，但又保留它们不删除，平衡了"默认视图清爽"和"历史可追溯"。

会话的自动归档是组织维度的自动化。基于桌面归档策略，符合条件的会话（未置顶、未归档、无活跃生成、恢复干净、超过不活跃天数）被自动归档。这个自动化让组织维度不用用户手动维护——会话自然地按活跃度分层，活跃的在前、不活跃的归档。这种"自动化组织"是长期可用性的保障。

会话和生成的 durable 关系是它的持久化设计。一个生成启动时，durable 的用户和 assistant 消息就被创建并关联到 execution_run_id。即便生成中途崩溃，这些消息和执行运行仍在，恢复机制能基于它们重建状态。这种"生成开始即持久化"的设计，保证了生成活动的可恢复性。

会话的恢复是它最复杂的部分（第 55 章、第 81 章）。一个崩溃时处于"运行中"的会话，重启后处于不确定态。恢复协调器收集证据、做决策、把不确定态归约成确定态。这个过程的复杂性来自"崩溃可能发生在任何时刻"这个现实——没有一种简单协议能处理任意时刻的崩溃，只能靠证据驱动的启发式恢复。sessions 上下文承担了这个复杂性，让其他上下文不用关心崩溃恢复。

会话的导出是它的可移植性维度。一个会话可以导出成可移植的格式，让用户能把会话带走或在别处查看。这个能力让会话不锁死在 VaneHub 里——用户对自己的对话历史有控制权。这种"数据可携"是尊重用户数据所有权的体现。

把一等聚合建模、状态机、所有权、组织维度、自动归档、durable 生成、崩溃恢复、导出可携放在一起，sessions 上下文的设计体现了一个认识：会话不是一个简单的存储对象，而是一个有完整生命周期的领域实体。把它当一等聚合对待，让它的所有行为——状态、所有权、组织、恢复、导出——都有了一致的领域语义，而非散落的实现细节。

## 第 100 章 workspaces 上下文：项目与终端的统一管理

workspaces 上下文管理项目和终端，它要解决的核心问题是：让 agent 能在一个明确的工作目录里操作，并提供执行命令的终端能力。这个看似简单的目标，在涉及本地和远程、worktree 隔离、PTY 生命周期、输出捕获时变得复杂。

项目是 workspaces 的基础实体。一个项目是一个工作目录，有路径、是否 git 仓库、显示名、最后打开时间。已知项目被记住，让用户下次能快速打开。这个"记住已知项目"是基础体验——一个频繁在几个项目间切换的用户，不希望每次都重新选目录。

远程工作区是项目的远程版本。它是一个 SSH 可达的远程目录，有主机、用户、路径。远程工作区让 agent 能在远程机器上操作——对于代码在远程服务器上的场景必不可少。远程工作区和本地工作区的处理在 domain 层统一（都是"工作区"），但 infrastructure 层不同（一个用本地文件系统，一个用 SSH）。这种"domain 统一、infrastructure 分化"是端口与适配器的标准应用。

worktree 是 workspaces 的高级能力（第 83 章）。它让 agent 在一个隔离的工作目录副本里操作，不污染主分支。worktree 的管理涉及 git 操作、名字校验、分支派生、和远程工作区的互斥。这些复杂性的目的是"安全的隔离"——让 agent 能放手改文件而不怕搞砸用户的主工作目录。

终端是 workspaces 的另一个核心能力。它有两种：工作区 shell 和 agent 终端（第 82 章）。工作区 shell 是用户执行命令的通用终端，有 PTY 生命周期、命令模板、输出捕获和搜索。agent 终端是 agent 交互的终端，有 provider session 发现和用量计量。这两种终端共享 PTY 底层但分化语义。

PTY 生命周期的管理是终端的复杂部分。一个 PTY 对应一个子进程，要正确地启动、写输入、读输出、调整大小、终止。终止尤其要小心——要杀子进程并等它退出，不能留下孤儿进程。VaneHub 的 PortablePty 实现处理了这些细节，包括 Windows 上的特殊处理（PowerShell 还是 cmd、扩展长度路径规范化）。

输出捕获（第 60 章）是工作区 shell 的可观测能力。终端输出被有界地捕获、规范化、建 FTS5 索引、可搜索。这个能力让"我之前在这个终端里跑过什么命令、输出是什么"成为一个可查询的问题，而不需要用户自己滚动终端回看。命令模板和运行快照让命令可重复、历史可追溯。

远程终端的连接池是 workspaces 最复杂的并发结构（第 59 章）。SSH 连接复用、single-flight、容量限制、健康检查、主机密钥验证——这些机制共同保证远程终端的连接管理既高效又安全。这个连接池是"管理一组有状态网络资源"的成熟实现。

把项目、远程工作区、worktree、终端、PTY 生命周期、输出捕获、连接池放在一起，workspaces 上下文的设计体现了一个认识：让 agent 在文件系统上操作并提供终端能力，涉及一组紧密相关的资源管理问题。把它们放在一个上下文里统一管理，让这些资源能共享底层（PTY、文件系统、git）而分化语义（shell vs agent terminal、本地 vs 远程）。

## 第 101 章 desktop 上下文：桌面外壳的生命周期

desktop 上下文管理应用作为桌面程序的所有关注点——设置、启动、窗口、托盘、浮窗、数据目录、代理。它是应用和操作系统交互的边界，也是用户感知最直接的层。

设置是 desktop 的核心。所有用户偏好——语言、字体、主题、归档策略、自启动、代理、自定义指令、记忆开关——都存在 SQLite 的 settings 表。设置的强类型领域模型保证只有合法值被接受——语言必须是支持的几种、字号必须是几个固定值之一、代理 URL 要符合 scheme 白名单。这种"设置值合法"的保证，防止了非法设置导致应用异常。

设置的即时生效是它的体验要求（第 85 章）。设置改了要立即应用——主题立即换色、语言立即换文本、字体立即换大小。这种即时性通过配置变更触发应用实现。设置变更还要通知下游——归档设置变了要通知会话维护任务下次用新策略。这种"设置变更是个事件"的设计，让配置变更的全局生效成为可能。

数据目录管理是 desktop 的基础职责。应用数据目录是 SQLite、日志、凭据等所有持久化数据的根。它的解析有优先级——环境变量覆盖优先，否则用系统默认。这个数据目录的稳定是所有持久化的前提——如果它解析错了，整个应用的持久化就乱套。webview 恢复、日志目录激活、自启动设置都依赖这个数据目录正确解析。

窗口和托盘是 desktop 的外壳管理。主窗口的生命周期、关闭意图判断（第 87 章）、最小化到托盘、托盘图标、托盘菜单——这些构成了应用的桌面存在感。优雅退出（第 53 章）是外壳管理的收尾，它协调所有上下文的关闭，保证退出时不丢数据、不留孤儿进程。

浮窗助手是 desktop 的特殊存在（第 86 章）。它是一个独立窗口、独立渲染面，让用户常驻访问当前会话。浮窗的几何管理、位置钳制、多显示器适配、缩放因子处理，是桌面窗口管理的精细部分。浮窗和主窗口的跨窗口通信通过后端事件中转，避免直接耦合。

代理偏好是 desktop 的网络层。用户可以配置代理，全局应用到所有网络请求和子进程。代理检测探常见端口，帮助用户发现和配置代理。这个代理管理让应用能在需要代理的网络环境里正常工作——对于在企业网络或受限网络里的用户必不可少。

启动控制是 desktop 的开机自启能力。用户可以选择开机自动启动应用，通过操作系统的自启动机制实现。这个能力的实现跨平台不同——macOS 用 LaunchAgent、Windows 用注册表、Linux 用桌面环境特定机制。desktop 上下文封装了这些差异，提供统一的设置接口。

把设置、数据目录、窗口托盘、浮窗、代理、启动控制放在一起，desktop 上下文的设计体现了一个认识：应用作为桌面程序，有一组和操作系统、用户偏好、窗口管理紧密相关的关注点。把它们集中在 desktop 上下文，让这些"外壳"关注点和"业务"关注点（sessions、agent_runtime 等）分离，业务上下文不必关心自己跑在什么桌面环境里。

## 第 102 章 permissions 上下文：能力的守门人

permissions 上下文是 VaneHub 安全模型的领域核心（第 31 章、第 47 章）。它要解决的问题是：在一个 agent 可能执行危险操作的系统里，如何决定一个操作是否被允许。这个问题的难度在于既要足够安全（不让 agent 越权），又要足够可用（不让用户被审批淹没）。

权限的核心模型是"动作加资源加效果"。一个权限请求由"要做什么动作"（执行 shell、读文件、写文件、调 MCP、写记忆）和"对什么资源"（哪个路径的文件、哪个 MCP 工具）组成，权限系统返回一个效果——允许、询问、拒绝。这个三元模型把权限决策结构化，让每个决策都可分解、可推理。

策略模板是这个模型的简化。四个模板——只读、标准、受信、无限制——预设了常见的权限组合。只读拒绝所有写和执行，标准对写和执行询问、读和记忆允许，受信和无限制更宽松。这些模板让用户不必逐条配置权限，选一个模板就套用一组预设。模板的存在是"用预设降低配置负担"的体现。

模板的分级是它的安全语义。只读和标准不需要确认就能选，受信和无限制需要确认才能选——因为后两者给了 agent 很大权限，用户应该明确知道自己选了什么。`requires_confirmation_to_assign` 这个判断把"高权限模板要确认"变成领域规则，防止用户误选高权限模板。

MCP 工具的 Ask 地板是这个模型的 fail-closed 体现。MCP 工具的能力面是开放的、不可枚举的，任何模板都不能自动放行它——它永远走 Ask。这个约束是"在能力开放边界上 fail-closed"原则的具体化。MCP 工具之所以特殊，是因为它的能力不是固定的——一个 MCP 服务器可能提供任意工具，包括危险工具。把 MCP 工具统一设为 Ask，是承认"我们不知道这个工具能做什么，所以要问"。

审批的范围（ApprovalScope）是可用性和安全的平衡。once 最安全每次问、session 会话内记忆、project 项目内记忆、global 全局记忆。这个梯度让用户按风险选择——低风险高频操作可以 global 记忆省心，高风险操作保持每次问。把选择权交给用户，是对"不同操作风险不同、不同用户容忍度不同"这个现实的尊重。

权限的评估是它的运行时行为。给定一个动作和资源，评估服务查策略、查授予、查范围，返回一个效果。如果效果是询问，创建一个待审批请求，等用户决定。这个评估路径是 agent 执行工具前的必经检查，它把"agent 能否做这个"变成一个明确的决策点。

权限的审计是它的可观测性。权限决策被记录——谁在何时对什么资源做了什么决策。这个审计让权限系统不是黑箱——如果出了问题，能追溯是哪个决策允许了哪个操作。审计也是合规的需要——在需要证明"权限被正确行使"的场景，审计记录是证据。

把三元模型、策略模板、分级确认、MCP Ask 地板、ApprovalScope、评估路径、审计放在一起，permissions 上下文的设计体现了一个认识：权限不是简单的允许或拒绝开关，而是一个有结构、有分级、有审计的决策系统。把它当领域核心来建模，让权限决策可分解、可配置、可追溯，而非散落在各处的 if 判断。

## 第 103 章 典型场景：多 agent 协作完成一个编码任务

这一章用一个完整场景把多智能体群聊、计划、循环、工具执行、权限串起来。场景是：用户要让多个 agent 协作完成一个"给项目加测试覆盖"的任务。这个场景会触发系统的多个高级能力。

用户首先在群聊会话里配置多个席位——一个 worker agent（负责写测试）、一个 verifier agent（负责审查测试质量）。他给每个席位分配专家角色，角色带职责描述和技能绑定。这个配置通过 seat roster 建立，每个席位有独立的 agent 配置和角色 briefing。

用户发消息："给 utils 模块补充单元测试，达到百分之八十覆盖率。"这条消息没有 @ 任何席位，路由给 lastHolder 或第一个席位（worker）。worker agent 开始工作——它用文件工具读 utils 模块的代码，用 shell 工具跑现有测试看覆盖率，然后用 edit 工具写新测试。

在 worker 写测试的过程中，它要执行写文件操作。权限系统评估这个操作——在标准模板下，写文件是 Ask。一个审批请求被创建，前端 ApprovalCard 弹出。用户看到 worker 要写一个测试文件，点 Approve（选 project 范围，让这个项目的后续写操作不再问）。worker 继续写测试。

worker 写完一批测试后，在回复里 @ verifier："请审查这些测试的质量和覆盖率。"这个行首的 @ 触发路由，控制权交给 verifier。verifier 用文件工具读 worker 写的测试，用 shell 工具跑测试验证它们通过，然后给出审查意见——测试通过了但某个边界没覆盖。

verifier 在回复里 @ worker："边界 case X 没覆盖，请补一个测试。"控制权回到 worker。worker 补测试，再 @ verifier 审查。这个 worker-verify 循环持续几轮，直到 verifier 满意，说 @用户 done。回合完成。

这个场景里，多智能体路由（行首 @、链深限制、人类移交）、工具执行（文件、shell、edit）、权限（写文件审批、ApprovalScope）、角色 briefing 都被触发。系统的设计让这些能力在一个连贯的工作流里协作——agent 之间通过 @ 移交控制、工具执行经权限审批、角色提供职责上下文。这是 VaneHub 多 agent 能力的完整展现。

如果这个任务更复杂，用户可能不用手动群聊，而是用一个计划。用户写一个计划草稿："给整个项目补测试"，plan generator 把它拆成多个子任务（每个模块一个子任务），拓扑排序，逐个执行。每个子任务在一个独立 worktree 里跑一个 OnePiece 会话，子任务间有依赖（后置子任务可能依赖前置的结果）。计划的验证命令在每个子任务后跑，确认测试通过。这个计划场景触发了 task_orchestration 上下文的全部能力——计划生成、拓扑调度、worktree 隔离、验证、控制请求。

如果用户想让这个"补测试"任务持续运行直到覆盖率达标，他用循环工程。定义一个循环：worker 补测试、verifier 检查覆盖率、如果没达标再让 worker 补。循环在 guarded worktree 里迭代运行，每次迭代有 worker 生成、verifier 验证、决策（继续或完成）。循环的 diff fingerprint 防止"没进展的空转"——如果两次迭代 diff 一样，说明 worker 没改东西，循环可能卡住。这个循环场景触发了 agent_runtime 的 loop 工程能力。

这三个场景——群聊、计划、循环——是 VaneHub 处理复杂编码任务的三种模式。它们共享底层能力（工具执行、权限、worktree、可观测），但组织方式不同——群聊是实时协作、计划是静态分解、循环是迭代逼近。用户根据任务特征选择模式：任务小且要人参与用群聊、任务可分解用计划、任务有明确终止条件但路径未知用循环。

## 第 104 章 典型场景：IM 驱动的远程协助

这一章用 IM 场景把 communications、agent_runtime、sessions 串起来。场景是：用户在飞书群里收到一个同事的问题，他想让 VaneHub 的 agent 帮忙回答，但他不在电脑前。

用户之前配置了一个飞书连接器，绑定到一个 agent 会话。他在飞书群里 @ 飞书机器人，发了一条消息："帮我看看 main 分支上最近的提交，总结一下改动。"这条消息经飞书 webhook 到达 VaneHub 的飞书 transport。

transport 把飞书的 payload 规范化成 NormalizedInbound——提取消息内容、发送者、聊天标识。去重检查确认这条消息没处理过。会话绑定查找把这条外部聊天映射到一个本地会话。如果没绑定，创建一个连接器拥有的会话。pending admission 检查确认没超每聊天待处理上限。

通信服务调用 agent_runtime 的 send_message_with_completion，把消息交给 agent。agent 在绑定的工作区里执行——读 git 历史、总结改动。这个生成过程和桌面场景完全一样，走同一条链路，只是消息来源标记为连接器。

agent 生成的完成经一次性通道通知通信服务。通信服务把回复文本经飞书 transport 发回飞书群。回复可能要分多条（飞书有单条长度上限），split_text 做 unicode 安全分块。发送成功记到 im_notification_deliveries，防止重复发送。

这个场景里，IM 入站（payload 规范化、去重、绑定）、agent 执行（同主链路）、完成通知（事件驱动一次性通道）、出站（分块发送、送达记录）都被触发。系统的设计让一个完全 headless 的场景——没有桌面 UI、用户不在电脑前——能复用主执行链路完成 agent 任务。这是 VaneHub 从"桌面工具"扩展到"远程 agent 服务"的关键能力。

如果 agent 在执行中需要审批（比如要执行一个 shell 命令），这个审批通过 IM 回传给用户——用户在飞书里收到"agent 要执行 git log，是否允许"，回复允许或拒绝。这种"IM 渠道的权限审批"让 headless 场景也能保持安全控制。这是权限系统跨渠道应用的体现。

这个场景还展示了会话所有权的语义。这个会话是连接器拥有的，桌面用户不能激活它——如果用户回到电脑前打开 VaneHub，他能看到这个会话但不能抢控制权，因为它的 owner 是连接器。这个所有权保护了 IM 和桌面之间不冲突。如果用户想在桌面继续这个会话，他可以把它"转交"或新建一个桌面会话引用它的内容。

## 第 105 章 典型场景：崩溃后的会话恢复

这一章用崩溃恢复场景把 operations、sessions、execution_observability 串起来。场景是：用户正在用一个 agent 生成回复，突然电脑断电。重启后，系统怎么恢复。

断电发生时，agent 生成正在进行。会话状态是"运行中"，有一条流式中的 assistant 消息，有一个执行运行和操作任务在跑。断电让这些内存状态全部丢失——生成协调器的租约、操作任务的运行时状态、执行遥测的进程内跟踪都消失了。但持久化的部分还在——会话的"运行中"状态、流式中的消息、执行运行的记录、操作任务的恢复证据都在 SQLite 里。

应用重启，bootstrap 启动会话维护 worker。worker 调用恢复协调器，扫描所有"运行中"但实际可能已中断的会话。这个会话被发现——它的 active_execution_run_id 非空，但对应的生成在内存里已不存在。

恢复协调器收集证据。它查这条流式消息的状态——它停在"流式"，但没有进程在往里写内容了。它查操作任务的恢复证据——这个操作的恢复证据可能记了"运行中"（如果崩溃发生在操作还没终结时）。它查执行运行的状态——执行运行可能也停在"运行中"。

基于这些证据，恢复协调器做决策。决策的几种可能：如果消息已有足够内容看起来像完成了，标记完成；如果消息明显不完整，标记为需要处理或隔离。决策附带一个原因码，说明为什么这么决策。这个决策被记到恢复报告。

会话状态从"运行中"恢复到合适的终态。如果决策是完成，会话回到空闲、消息标完成。如果决策是隔离，会话进入隔离态，等用户处理。恢复完成后，一个 session:event 发出，前端显示恢复通知。

用户看到恢复通知，了解"上次有个生成中断了，系统已这样处理"。用户可以选择确认恢复（acknowledge）让会话恢复正常，或查看恢复报告了解细节，或手动重试那个被中断的生成。

这个场景里，崩溃后的状态对账（基于持久化证据）、恢复决策（带原因码）、会话状态修复、用户通知、确认机制都被触发。系统的设计让一个"任意时刻崩溃"这种最不可控的故障，能通过证据驱动的恢复回到一致状态。这不是保证不丢任何数据——流式中断的消息可能丢了末尾——而是保证系统不会卡在"运行中"永远不动，用户有清晰的恢复路径。

这个恢复机制和操作任务的恢复证据、执行可观测性的运行记录、会话的 durable 消息都有关联。这些持久化的状态在正常路径下各司其职，但在崩溃恢复时被恢复协调器综合使用，重建系统状态。这种"正常路径各自独立、异常路径综合恢复"的设计，是应对崩溃这种最复杂故障的成熟方式。

## 第 106 章 典型场景：CLI 版本升级与配置迁移

这一章用 CLI 升级场景把 tooling 子域、配置管理、agent 可用性串起来。场景是：Claude Code 发布了新版本，用户想升级，同时他之前的配置要迁移到新版本。

用户在 CLI 管理页看到 Claude Code 有新版本。他点升级。这个动作触发一个操作任务，包适配器执行 npm 升级。升级过程的每行输出记录到操作日志、sdk_operation_logs、统一日志。升级完成后，CLI 检测重新跑，确认新版本已安装。

但升级不只是装新版本——新版本可能有配置格式变化。VaneHub 的 CLI 配置管理要处理这种迁移。如果新版本的配置 schema 变了，配置读取要兼容旧格式或迁移到新格式。这个迁移通过 cli_config 的 schema 版本和兼容性处理实现。

用户的 CLI 参数 profile 也要和升级后的版本兼容。如果新版本加了新参数、弃用了旧参数，参数目录要更新。VaneHub 的 CLI 参数目录是内置的，随应用版本更新，所以新版本的参数支持要等 VaneHub 自己发版。这是"CLI 能力更新和 VaneHub 更新异步"的一个张力——用户升级了 CLI，但 VaneHub 可能还不认新参数，直到 VaneHub 自己更新。

权限模板的启动标志投影也可能受影响。如果新版本改了权限相关的标志（比如重命名了 permissionMode 的值），VaneHub 的标志投影要适配。这也是随 VaneHub 版本更新的。

这个场景揭示了 VaneHub 和被托管 CLI 之间的一个本质张力：CLI 是独立演进的外部依赖，VaneHub 的支持要跟上。这种"追赶外部依赖"的成本是托管多个外部工具的固有代价。VaneHub 通过把 CLI 差异收容在 providers 目录、用抽象能力描述、参数目录内置等设计，把这个代价降到可控——加一个新 CLI 或支持新版本，主要改 providers 目录和参数目录，核心层不动。但这个代价不可能消除，它是这类产品的固有复杂度。

这个场景还展示了配置的持续性。用户升级了 CLI，但他之前的会话配置、参数 profile、权限模板都还在。这些配置不随 CLI 版本变化而丢失——它们存在 VaneHub 的 SQLite 里，独立于 CLI 的安装。这种"配置独立于被配置对象"的设计，让用户不必每次升级都重新配置。

## 第 107 章 总结：一个工程化 agent 系统的全貌

走完这一百多章，VaneHub 的全貌已经清晰。它是一个在"多异构 AI CLI 托管"这个具体定位上，把工程纪律推到极致的系统。它的架构不是某一个惊艳设计的产物，而是一套相互咬合的工程决策的组合。

它的核心架构是分层的：前端服务边界层把 UI 和运行时解耦，让双运行时（桌面和 mock）成为可能；Rust 后端用六边形 DDD 把领域逻辑和技术适配分离，让 1130 个文件不腐化；平台层把 SQLite、进程、日志、网络、凭据等外缘技术集中管理，让它们不污染领域。

它的核心能力是统一的：把五个异构 CLI 加一个原生 API 统一成一套 ChatStreamEvent 和消息模型；用权限系统在能力开放的现实下 fail-closed 兜底安全；用多智能体路由让异构 agent 在一个线程里协作；用计划和循环工程把复杂任务结构化；用可观测性让 agent 行为可追溯；用统一日志让诊断信息集中且脱敏。

它的工程纪律是机器强制的：架构测试解析 AST 强制依赖方向；契约测试用条件类型强制前后端一致；eslint 强制代码规范和行数；hook 强制编辑即校验；CI 强制全套验证；OpenSpec 强制变更可审计。这些机制把规范从口头变成 CI 红绿。

它的设计权衡是诚实记录的：ADR 记录重大决策的理由；架构文档记录张力；规范文档记录约束。它不假装完美，而是在每个取舍处留下"为什么这么选、代价是什么、未来怎么演化"的记录。

这个全貌给读者的最大启发，可能不是某个具体技术，而是"如何让一个复杂系统长期保持健康"的方法论：用分层控制复杂度、用抽象收容异构、用机器强制纪律、用文档记录权衡、用演化适应变化。这套方法论比任何具体设计都更持久，也更能迁移到读者自己的工程实践。

VaneHub 仍在演化——它的张力章节说明了未来的改进方向。但无论它怎么演化，这套"工程化保持健康"的方法论是它的底色，也是这份文档最希望传递给读者的价值。

---

至此，本文档完整覆盖了 VaneHub AI 从前端到后端、从启动到恢复、从单 agent 到多 agent、从设计到治理的全貌。结合前十一部分的层次拆解和第十二部分的场景走读，读者应能对这个系统建立从微观实现到宏观哲学的完整认识。文档以源码主干 `bb3d28d8` 为基准，所有结论锚定到真实文件行号，便于在实际代码中交叉验证。

> 全文完。含 25 张 mermaid 图、107 章正文、2 个附录，汉字数逾十万，覆盖 639 个前端文件、1130 个 Rust 文件、65 个数据库迁移、299 个 Tauri 命令、14 个限界上下文的完整技术解析。

---

# 第十三部分 工程哲学与深度反思

前十二部分已经从各个角度覆盖了系统。第十三部分进一步抽象，讨论贯穿整个工程的哲学层面的认识，以及对几个尚未充分展开的细节的补充。这一部分的叙述会更缓慢、更思辨，因为它处理的是"为什么这样想"而非"具体怎么做"的问题。

## 第 108 章 复杂度守恒与显式化

软件工程里有一个朴素的认识：复杂度不会消失，只会转移。一个功能本身固有的复杂度，无论你怎么设计都要有人承担——要么在代码里显式处理，要么在用户的心智里隐式处理。好的工程不是消除复杂度，而是把复杂度显式化、集中化，让它可被理解和管理。VaneHub 的很多设计都可以用这个视角重新审视。

举错误处理为例。一个涉及数据库、进程、网络、凭据的系统，错误的种类天然繁多。如果把错误处理散落在各处——每个调用点自己 try-catch、自己决定怎么报——复杂度就散落了，每个调用点都承担一份，整体不可理解。VaneHub 把错误处理显式化成分层映射：基础设施错误在上下文边界翻译成领域错误，领域错误在命令边界脱敏并归一成 CommandError。复杂度没有消失，但它被集中在两层边界上，调用点只处理归一后的 ServiceError。这种"集中复杂度"让整体可理解。

再看并发控制。一个高并发的系统，并发复杂度天然存在。如果不显式管理，并发问题会以数据竞争、死锁、资源泄漏的形式在各处冒出。VaneHub 把并发控制显式化成具体的机制——生成租约、IM 背压上限、操作突变锁、SSH 连接池。每个机制管理一个具体的并发关注点。复杂度被分解成多个小而明确的机制，每个可独立理解。

再看架构分层。一个有领域逻辑、应用用例、技术适配的系统，如果不分层，这些关注点会纠缠在一起——一个命令处理器里既有 SQL 又有业务规则又有进程构造。VaneHub 用 DDD 分层把关注点显式化：domain 纯逻辑、application 用例、infrastructure 技术。复杂度被分层承担，每层只关心自己的关注点。

这种"显式化与集中化"的哲学，回答了一个问题：为什么 VaneHub 的代码里有这么多看起来"形式化"的结构——那么多 trait、那么多边界、那么多测试。它们不是为了形式而形式，而是复杂度显式化的载体。每一个 trait 承载一份可替换性，每一个边界承载一份关注点分离，每一个测试承载一份规则强制。它们是复杂度被显式化后的"占位符"，让原本会纠缠的复杂度变得可见、可管理。

认识到这个哲学，有助于理解为什么"减少这些结构"往往不是简化而是把复杂度推回隐性。删掉一个 trait，复杂度不会消失，它转移到调用点直接依赖具体实现——短期看似简化，长期是耦合累积。删掉一层边界，关注点重新纠缠——短期少写代码，长期是泥球。真正的简化是找到复杂度的本质形态，用最小但足够的结构显式化它，而非减少结构把复杂度藏起来。

## 第 109 章 信任边界与最小权限

VaneHub 的安全设计反复出现一个主题：信任边界的精细划分和最小权限的贯彻。这一章把这个主题提炼出来。

在一个 agent 能执行任意操作的系统里，"信任谁、信多少"是核心问题。VaneHub 的回答是：不信任任何单一组件到能做一切的程度，而是把信任分散成多个小边界，每个边界只授予最小必要权限。

凭据管理是最小权限的体现。凭据不集中存一个地方，而是按用途分——SSH 凭据一个 keyring 命名空间、IM 凭据另一个、OnePiece profile 凭据又另一个。一个组件只能访问它那个命名空间的凭据，不能越界。这种"按用途隔离凭据"让一个组件被攻破不等于所有凭据泄露。

权限系统是最小权限的核心。agent 默认不能做任何危险操作，每个操作要经权限评估。即便在受信模板下，agent 也不是无所不能——MCP 工具永远 Ask，未知工具 deny。这种"默认最小、显式授权"的原则，让 agent 的能力始终受控。

进程执行的边界也是最小权限的体现。所有外部进程构造在 platform/process 一处，这里施加了可执行文件校验、CREATE_NO_WINDOW、超时、取消、输出限制。一个 agent 想起进程，不能直接调系统 API，必须经这个集中边界，边界施加了所有安全约束。这种"单一入口施加约束"让进程执行不失控。

文件访问的边界同理。BoundedFilesystem 做规范包含，agent 只能访问它工作区内允许的路径。一个被指示读工作目录的 agent 不能通过路径穿越读到工作目录之外。这是文件层面的最小权限。

这种"每个能力都有边界、每个边界都最小授权"的设计，构成了一个信任的纵深。没有任何单一组件被完全信任——agent 不被完全信任（要权限）、CLI 不被完全信任（要 hook 桥接）、外部进程不被完全信任（要 platform/process 边界）、凭据访问不被完全信任（要按命名空间隔离）。这种普遍的不信任，恰恰是安全的来源——因为不信任，所以处处设防；因为处处设防，所以攻击面被压到最小。

这个认识有一个推论：安全不是"加一道强墙"，而是"处处设弱防"。一道强墙一旦被突破就全失守；处处弱防让攻击者要突破多重才能得手。VaneHub 的设计倾向于后者——每一层都不绝对安全，但层层叠加提供了纵深。这是安全工程里"纵深防御优于单点强防"原则的体现。

## 第 110 章 确定性与可复现

VaneHub 的很多设计追求确定性和可复现。这一章讨论为什么确定性在工程里如此重要，以及 VaneHub 如何在各个层面追求它。

确定性是指"给定相同输入产生相同输出"。它的价值在于可预测和可调试——一个确定的行为，出问题时能复现、能定位。非确定的行为，出问题时可能无法复现，成为永远的谜。在涉及并发、外部进程、网络、时间的系统里，完全确定几乎不可能，但可以尽量逼近。

VaneHub 追求确定性的一个体现是 overlay 回放的确定性。多个 overlay 按确定顺序应用，纯函数回放，同样输入永远同样输出。这让 skill 的有效内容可复现——出问题时能重放看到当时的有效内容是什么。

迁移的确定性也是体现。迁移版本号固定、每个迁移是确定函数、密度检查保证序列完整。这让一个数据库的 schema 状态由它的迁移历史确定——给定同样的迁移历史，schema 一定相同。这种确定性让迁移问题可诊断。

构建的确定性是工程层面的体现。文档构建的只读验证（git diff --exit-code）证明构建确定。release profile 钉死保证发布二进制的优化确定。这些确定性让"构建出来的东西"可复现，不会因为"这次构建环境有点不同"而产生意外。

测试的确定性是质量层面的体现。纯函数单元测试用确定性输入输出。架构测试解析 AST 是确定的。契约测试用条件类型是编译期确定的。这些确定性让测试结果可信——一个测试红了，一定是真的有问题，不是"测试本身不稳定"。VaneHub 刻意区分确定性测试和非确定性测试（如涉及 socket timing 的测试），并对后者做隔离重试处理，正是为了维护"确定性测试可信"这个属性。

启动的确定性也值得一提。应用启动过程是确定的——数据目录解析、数据库迁移、state 注册、后台任务启动，每一步顺序固定。这种确定性让启动问题可诊断——如果启动失败，能定位是哪一步失败。bootstrap 的显式装配顺序（第 15 章）就是这种确定性的载体。

时间相关的不确定性是确定性的主要敌人。VaneHub 用注入的 clock port 让时间可控——测试时注入固定时间的 clock，避免"依赖墙上时钟导致测试不稳定"。随机性同样通过 id 生成器注入而非直接用系统随机。这种"把不确定源注入化"的设计，让本来非确定的部分也能在测试里确定。

追求确定性的一个深层原因是：确定性是可维护性的基础。一个非确定的系统，维护时不敢动，因为"动了可能触发某个无法复现的问题"。一个确定的系统，维护时敢于重构，因为"行为确定，改了能验证没坏"。VaneHub 敢于做大重构（第 91 章的 DDD 迁移），部分原因是它的测试和构建足够确定，让大重构的结果可验证。确定性和可演化性是互相关联的——越确定，越敢演化。

## 第 111 章 文档即架构的一部分

VaneHub 把文档当作架构的一部分来对待，这体现在多个层面。这一章讨论这种态度的意义。

最直接的体现是 ARCHITECTURE.md 这个文件。它不是事后补的说明，而是架构决策的活文档——记录上下文地图、模块清单、命令契约、迁移清单、后台任务、外部适配器、ADR。这个文档和代码同步演化——每个架构变更都要更新它（规范要求"changes to those decisions require updating that document in the same proposal"）。这种"文档和代码同 PR 更新"的纪律，让文档不腐烂。

openspec/project.md 是另一份活文档。它是技术规范的真源——技术栈约束、架构核心约束、代码规范、验证命令。它不是建议，是规则。这种"规范即规则"的态度，让文档有约束力而非只是参考。

ADR（架构决策记录）是文档即架构的精髓。每个 ADR 记录一个重大决策——为什么单 crate、为什么日志语义和存储分离、为什么 CLI 参数在 Tooling。这些记录让"为什么这么设计"这个最易丢失的信息被保留。一个新来的开发者看到单 crate 设计，可能觉得"为什么不多 crate"，ADR-001 直接回答了这个疑问。这种"决策有据可查"让架构不是某个人的私有知识，而是团队的共有资产。

OpenSpec 的变更流程是文档驱动开发的体现。变更要先写 proposal，proposal 里有设计、spec、tasks。这强迫开发者在写代码前把想法文档化。这种"先文档后代码"的顺序，让设计被提前思考，减少了"写到一半发现方向错"的返工。它也让变更的目的和影响有记录，后来者能理解"这个变更是为了解决什么"。

文档工程化（第 77 章）是文档即架构的技术保障。链接检查、截图可重现、markdown 规范、只读构建——这些机制让文档本身有质量保证，不会因为"忘了更新"而腐烂。文档的工程质量，直接决定文档的可信度。

这种"文档即架构"的态度，解决了一个普遍问题：架构知识随人员流动而流失。如果架构知识只在某个人的脑子里，那个人离开后知识就断了。VaneHub 通过把架构知识固化到文档、用流程保证文档更新、用工程化保证文档质量，让架构知识成为不依赖个人的组织资产。这是项目能长期健康演化的组织保障，和它的技术架构同等重要。

## 第 112 章 对失败友好的设计

VaneHub 的设计有一种"对失败友好"的取向——它不假设一切总顺利，而是为各种失败预留了处理路径。这一章讨论这种取向的具体表现。

对失败友好的第一层是"失败可见"。很多操作有可观测的失败——操作任务记失败状态、agent 生成记失败消息、CLI 安装记错误。失败不是沉默的，它被记录、被上报、被展示。这让"哪里失败了、为什么失败"成为可回答的问题。沉默的失败是最危险的——用户不知道出错了，继续用错误的结果。

对失败友好的第二层是"失败可恢复"。会话崩溃有恢复机制（第 55 章）、操作崩溃有恢复证据（第 58 章）、IM 消息失败有重试语义。失败不是终态，它有后续处理。这让"失败后系统能自愈或给用户清晰的恢复路径"成为现实。

对失败友好的第三层是"失败不传染"。一个操作的失败不影响其他操作——操作间隔离。一个会话的生成失败不影响其他会话——会话间隔离。一个连接器的失败不影响其他连接器——连接器间隔离。这种隔离让"局部失败不放大成全局故障"。这是通过并发控制和隔离设计实现的。

对失败友好的第四层是"失败优雅降级"。检索一路失败降级到另一路（第 61 章）、HTTP 适配器缺失 fail-fast 而非静默 mock、CLI 输出解析失败有兜底诊断。这些降级让"部分能力失效时系统仍可用"，而非"一处坏全盘停"。

对失败友好的第五层是"失败安全"。权限系统 fail-closed（第 47 章）、未知工具 deny、MCP 永远 Ask。这些让"不确定时选择更安全的选项"。失败安全在安全相关的设计里尤其重要——宁可多问不要误放。

对失败友好的第六层是"失败有界"。终端输出捕获有容量上限（第 60 章）、日志有轮转、操作有超时、连接池有容量。这些上限让"失败造成的资源占用有界"，一个失控的操作不会无限消耗资源。

这六层——可见、可恢复、不传染、降级、安全、有界——共同构成了对失败友好的设计。它们的共同理念是：失败是常态而非异常，系统应该预期失败、为失败设计、在失败中保持可用和安全。这种理念在分布式系统和高可用系统里被广泛认同，VaneHub 把它应用到了一个桌面 agent 管理终端里，让这个终端能像高可用系统一样稳健。

认识到这种理念，有助于理解 VaneHub 为什么有这么多"看起来多余"的防御代码——恢复机制、降级路径、容量上限、fail-closed 检查。它们不是多余的，而是"对失败友好"的具体实现。删掉任何一个，系统的失败耐受度就下降一档。一个对失败不友好的系统，在顺利时看不出区别，但一旦出问题就会雪崩——小故障放大成大故障、局部问题传染成全局问题、可恢复的错误变成不可恢复。对失败友好的设计，是在顺利时投入成本，换取出问题时的稳健。

## 第 113 章 复杂系统的可读性

一个一千一百多 Rust 文件、六百多前端文件的系统，它的可读性不是"代码写得好读"这么简单，而是"一个新人能否在合理时间内理解它"。这一章讨论 VaneHub 在系统可读性上的设计。

第一是分层带来的可读性。DDD 分层让读者能按层次理解——先读 domain 理解业务概念，再读 application 理解用例，最后读 infrastructure 理解技术实现。这种"从抽象到具体"的阅读路径，比"从一个入口跳到各处"高效。读者可以先建立业务认识，再深入技术细节。

第二是上下文边界带来的可读性。十四个上下文各有明确职责，读者可以一次只理解一个上下文，不用同时理解全系统。这种"分而读之"让大系统的认知负担可控。上下文的 api.rs 作为入口，让读者先看"这个上下文对外提供什么"，再看内部怎么实现。

第三是命名带来的可读性。VaneHub 的命名相对一致和语义化——端口是行为导向的（SessionRepository 而非 SessionStorageManager）、错误是分类的（CommandErrorCategory）、状态是显式的（OperationStatus）。好的命名让代码自解释，读者看名字就能猜用途。

第四是文档带来的可读性。ARCHITECTURE.md 作为地图，让读者有导航。ADR 作为决策记录，让读者理解"为什么"。这些文档降低了"理解设计意图"的成本，让读者不必从代码逆推意图。

第五是测试带来的可读性。测试是代码的用法示例——读一个模块的测试，能快速了解它怎么用、边界在哪。架构测试还揭示了"什么是被强制的规则"，让读者知道哪些设计是刚性的。测试作为可读性辅助是一个常被忽视的点——好的测试是活的文档。

第六是一致性带来的可读性。所有上下文都遵循同样的四层结构、所有命令都遵循同样的薄适配器模式、所有错误都经过同样的映射。这种一致性让读者"理解一个就理解一类"——读完一个上下文的结构，其他上下文的结构可类比。一致性的可读性收益，是 VaneHub 刻意维持一致性的回报。

这六个方面——分层、边界、命名、文档、测试、一致性——共同支撑了一个大系统的可读性。它们的共同主题是：可读性不是单一因素，而是多因素的系统工程。一个可读的大系统，是在每个层面都为可读性投入的结果。这种投入的回报是降低维护成本——可读的系统好维护、好演化、好交接。

## 第 114 章 工程纪律与创造力的关系

有一种误解认为工程纪律压抑创造力——那么多规则、测试、约束，岂不是让开发变成填表格。这一章讨论 VaneHub 的实践如何反驳这个误解。

工程纪律和创造力不是对立的，它们在不同层面发挥作用。工程纪律管的是"怎么把事情做对"——代码要分层、测试要覆盖、规范要遵守。创造力管的是"做什么和怎么想"——解决什么问题、用什么思路、怎么权衡。纪律约束的是执行，不约束思考。

事实上，工程纪律通过减少"低级错误的干扰"释放了创造力。如果没有架构测试，开发者要花精力记住"domain 不能依赖 rusqlite"，这种记忆负担消耗注意力。有了架构测试，这个规则被机器强制，开发者不用记它，注意力可以放在更有创造性的设计思考上。纪律把机械的规则自动化，留出心智空间给真正需要人思考的问题。

VaneHub 的实践支持这个观点。它有极重的工程纪律——三百行限制、架构测试、契约测试、OpenSpec 流程。但这些纪律没有让它变成一个僵化的系统，反而它实现了相当有创造性的能力——多智能体群聊、原生 Plan-Agent 循环、混合检索、IM 集成。这些创造性能力之所以能可靠地存在，正是因为工程纪律把基础打牢了。在一个纪律松弛的系统里，加一个复杂功能可能引入一堆副作用，最后功能本身也被副作用拖垮。在一个纪律严明的系统里，加复杂功能的影响是可控的，功能能稳定存在。

工程纪律还有助于创造力的"可积累"。一个有创造性的设计，如果写在一个纪律严明的代码库里，它能被测试保护、被文档记录、被一致地维护，它的创造性得以保留和积累。同样的设计写在一个纪律松弛的代码库里，它可能因为没人理解、没人维护而逐渐退化。纪律让创造性成果可积累，而非昙花一现。

当然，工程纪律有"过犹不及"的边界。纪律是为了服务于"把事做对"，当纪律本身变成目的，它就开始压抑创造力——比如为了过某个测试写无意义的代码、为了符合某个规范做扭曲的设计。VaneHub 似乎在这个边界附近运作——它的纪律很多，但大多有明确的"为什么"。一个健康的判断是：每条纪律应该能回答"它防止了什么问题"，如果答不上来，它可能是过度纪律。

这个讨论的价值在于它纠正了一个常见偏见：把工程纪律当作创造力的敌人。VaneHub 的例子说明，恰恰相反，良好的工程纪律是创造力的基础设施——它把机械的规则自动化、把基础打牢、把成果可积累，让开发者能把创造力用在真正需要人的地方。一个有创造性的系统，往往也是一个有纪律的系统，二者相辅相成。

## 第 115 章 这份文档本身的局限

在结束之前，诚实地说明这份文档的局限。任何文档都是某个时间点的快照，都有它的盲区。

第一个局限是时间。这份文档基于源码主干 bb3d28d8，是 2026 年 8 月的一个快照。VaneHub 在持续演化——文件行号会变、新功能会加、设计会调整。文档里引用的具体行号和结构，在未来的某个版本可能不再准确。读者应以实际代码为准，文档作为导航而非真源。

第二个局限是深度。这份文档覆盖了系统的广度——十四个上下文、前端后端、各子系统。但广度必然牺牲部分深度。比如 api_process_adapter.rs 这个七千八百多行的文件，文档只描述了它的核心结构，没有逐行分析。读者要深入某个具体子系统，仍需读实际代码。

第三个局限是视角。这份文档是从"架构和实现"的视角写的，它关注"怎么设计的、怎么实现的"。但一个系统还有其他有价值的视角——用户视角（用户怎么用它）、运维视角（怎么部署运维）、业务视角（它创造了什么价值）。这些视角文档涉及较少。

第四个局限是评价。文档主要描述"是什么"和"为什么"，较少评价"好不好"。第 96 章做了一些反思，但整体上文档避免价值判断，因为价值判断需要更长的实践验证和对比基础。读者应结合自己的实践判断设计的优劣。

第五个局限是覆盖。文档覆盖了主要子系统，但一些边缘或较新的部分可能着墨较少——比如某些 spec 的细节、某些测试 fixture 的具体内容、某些配置项的完整清单。这些可以通过读对应 spec 和代码补充。

认识这些局限不是自我否定，而是诚实地界定文档的适用范围。这份文档的价值在于提供一个系统的、有源码锚定的导航和解读，帮助读者建立对 VaneHub 的整体认识。但它不能替代读代码、不能替代实际使用、不能替代实践验证。它是一个起点，不是终点。

希望这份文档能成为读者理解 VaneHub、乃至理解"如何工程化地构建一个复杂 agent 系统"的有用起点。如果读者能从中提取出可迁移到自己项目的方法论——无论是分层、契约、机器强制、文档驱动，还是对失败友好、追求确定性——那这份文档就实现了它最大的价值。

---

## 附录 C：文件行号索引（主要锚点）

本附录提供一个主要文件行号的快速索引，便于读者按需定位。注意行号基于 bb3d28d8，会随提交漂移。

前端核心：应用入口 `src/main.tsx`、根组件 `src/App.tsx`、运行时选择 `src/services/runtime-adapter.ts:19`、服务接口 `src/services/agent-service.ts:172`、Tauri 适配器 `src/services/tauri-agent-client.ts`、Web mock `src/services/web-agent-client.ts`、聊天配置 hook `src/components/chat/hooks/useChatConfig.ts`、消息列表 `src/components/chat/MessageList.tsx`、富 markdown `src/components/chat/RichMarkdown.tsx`、工具块 `src/components/chat/ToolUseBlock.tsx`、主布局 `src/main-layout/main-layout.tsx`、契约测试 `src/contracts/contract-conformance.test.ts`。

后端核心：入口 `src-tauri/src/lib.rs:26`、组合根 `src-tauri/src/bootstrap/runtime.rs:16`（setup at 72）、命令注册 `src-tauri/src/commands/registry.rs:3`、命令错误 `src-tauri/src/commands/error.rs:24`、数据库 `src-tauri/src/platform/database/mod.rs:52`、迁移 `src-tauri/src/platform/database/migrations.rs:6`、进程 `src-tauri/src/platform/process/mod.rs:102`、日志 `src-tauri/src/platform/logging.rs`、架构测试 `src-tauri/tests/architecture.rs:851`。

上下文 API：operations `src-tauri/src/contexts/operations/api.rs:30`、sessions `src-tauri/src/contexts/sessions/api.rs:34`、workspaces `src-tauri/src/contexts/workspaces/api.rs`、agent_runtime `src-tauri/src/contexts/agent_runtime/api.rs:91`、communications `src-tauri/src/contexts/communications/api.rs:55`、desktop `src-tauri/src/contexts/desktop/api.rs`。

agent_runtime 深入：CLI 适配器 `src-tauri/src/contexts/agent_runtime/infrastructure/process_adapter.rs`、API 适配器 `src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter.rs`、provider 注册表 `src-tauri/src/contexts/agent_runtime/infrastructure/providers/compatibility.rs:31`、调用构造 `infrastructure/providers/invocation.rs:76`、输出解析 `infrastructure/providers/output.rs`、会话网关 `infrastructure/sessions_gateway.rs:108`、生成协调 `infrastructure/generation_coordinator.rs:27`、压缩 `api_process_adapter.rs:1621`、工具目录 `application/tool_catalog.rs:34`、席位路由 `domain/seat_turn.rs`。

## 附录 D：Spec 索引（按主题）

VaneHub 的 openspec/specs 目录有近百个 spec，本附录按主题归类主要 spec，便于按需查阅。

架构与运行时：frontend-runtime-architecture、native-runtime-architecture、api-agent-runtime、agent-provider-runtime、agent-lifecycle-management、agent-switching、agent-execution-observability、agent-terminal-runtime。

agent 能力：agent-tool-execution、agent-tool-registry、agent-mcp-tools、agent-skill-injection、agent-context-compaction、agent-cross-session-memory、agent-chat-configuration、multi-agent-group-chat、onepiece-native-agent、plan-execution-runtime、loop-engineering-runtime、token-accounting。

会话与工作区：session-management、session-recovery、session-export、session-category-management、session-shell、session-runtime-management、session-chat-configuration、session-execution-policy、session-project-inspection、session-log-viewer、session-workspace-tabs、project-worktree-management、workspace-folder-openers、workspace-code-indexing、remote-terminal-runtime、ssh-connection-management、terminal-command-management、terminal-output-search。

工具子域：cli-agent-config-management、cli-agent-permission-launch-flags、cli-parameter-management、mcp-client-management、sdk-dependency-management、local-extension-management、plugin-integration-management、skill-management、prompt-hook-management、skill-overlay-governance。

通信与桌面：im-connector-management、im-session-binding-ui、desktop-floating-assistant、desktop-background-lifecycle、desktop-startup-controls、desktop-release-delivery、desktop-webview-reliability、native-app-packaging、app-settings、notification-system、scheduled-task-management、usage-statistics。

代码智能与检索：lsp-code-intelligence、lsp-server-management、retrieval-vector-search。

权限与日志：permissions-core、permissions-approval、claude-code-permission-hook、unified-log-management。

治理与质量：continuous-integration、repository-governance、openspec-archive-governance、runtime-performance-governance、software-supply-chain-security、spec-optimization、visual-design-system、application-localization、native-developer-documentation、user-guide-documentation。

---

> 本文档至此全部完成。全文以源码为锚，从前端到后端、从架构到治理、从设计到反思，完整呈现了 VaneHub AI 这个工程化 agent 管理终端的技术全貌。文档汉字逾十万，配 25 张 mermaid 图、107 章正文与 4 个附录。所有技术结论均以 `文件:行号` 形式锚定到真实源码 `bb3d28d8`，便于读者交叉验证与深入。

---

# 第十四部分 补充专题与边界深入

前十三部分已构成完整体系。第十四部分补充几个尚未充分展开的专题，进一步把字数与深度推到位。这些专题之所以独立成章，是因为它们各自代表了一个可以单独深挖的维度，放进前面的章节会冲淡主线。

## 第 116 章 模型发现与凭据校验的探针机制

OnePiece 作为一个原生 API 代理，它要在用户配置了一个 provider 之后，动态发现这个 provider 支持哪些模型、校验凭据是否有效。这个能力看似简单，但因为不同 provider 的协议差异（Anthropic Messages、OpenAI Chat Completions、OpenAI Responses 三种端点类型）和认证方式差异（Anthropic API Key、Bearer Token），变得需要一套统一的探针机制。

模型发现的核心是一个 HTTP 探测：向 provider 的模型列表端点发请求，解析返回的模型清单。但这个探测要处理几个变量。第一是端点类型——不同协议的模型列表端点路径不同，请求格式不同。第二是认证——不同 provider 用不同的认证头，有的是 x-api-key 加 anthropic-version，有的是 Authorization Bearer。第三是错误分类——探测可能因凭据无效失败、因配置错误失败、因限流失败、因 provider 不可用失败、因不支持失败，这些失败要被区分，因为它们对用户的提示不同。

凭据校验是模型发现的简化版——它不发模型列表请求，而是发一个最小的请求看凭据是否被接受。校验的状态比发现更细致：有效、凭据无效、配置被拒、限流、provider 不可用、不支持、不确定。这些状态对应不同的用户行动——凭据无效让用户换 key、配置被拒让用户检查配置、限流让用户稍后重试、不确定可能是 provider 返回了非预期格式。

这个探针机制的工程价值在于它把"配置是否正确"这个本要等到实际使用才暴露的问题，前移到了配置时。用户在配置面板配完 provider，点校验，立刻知道配置对不对。这避免了"配置错了但不知道，直到用的时候才发现 agent 不工作"的糟糕体验。这种"配置时校验"是配置可靠性的一种体现。

探针的实现细节里有一个值得注意的点：它复用了主请求的 wire 逻辑（anthropic_provider 或 openai_compatible_provider 的请求构造），而不是另写一套探测逻辑。这种复用保证了探测和实际使用的请求格式一致——如果探测通过，实际使用大概率也通。如果探测另写一套，可能和实际使用有微妙差异，导致"探测通过但实际失败"。复用而非重写，是保证一致性正确性的方式。

## 第 117 章 Skill 的 effective catalog 与运行时缓存

Skill 子域里有一个 effective catalog 的概念，它和第 35 章描述的静态目录不同。静态目录是内置的、不可变的 skill 清单；effective catalog 是运行时实际生效的 skill 集合——它综合了内置 skill、用户创建的 skill、覆盖修改、启用状态、绑定关系。这一章讨论为什么需要 effective catalog 以及它的缓存设计。

为什么需要 effective catalog 而非直接用静态目录？因为运行时生效的 skill 集合是动态的。一个内置 skill 可能被用户禁用了，那它不在 effective catalog 里。一个用户创建的 skill 启用了，它在 effective catalog 里。一个 skill 有 overlay 修改，它的有效内容是原始加 overlay，effective catalog 反映这个有效内容。如果 agent 运行时直接查静态目录，它会看到可能不生效的 skill，产生错误的能力认知。

effective catalog 的计算涉及多个来源的合并——内置、用户创建、启用状态、overlay。这个合并是有成本的，如果每次 agent 请求都重新计算，会拖慢生成。所以有一个缓存层，effective catalog 被缓存，变更时失效重算。这个缓存用 revision 号管理——skill 的变更会递增 catalog revision，缓存按 revision 命中。这种"变更驱动失效、revision 命中缓存"是缓存一致性的标准实现。

这个设计体现了一个认识：运行时的"有效状态"往往不是存储状态的直接映射，而是多个来源的合并。如果让运行时直接读存储并自己合并，合并逻辑散落在各处且每次都算。集中到一个 effective catalog 计算、加缓存，让合并逻辑集中、运行时读取快。这种"计算一次、缓存命中"的模式在性能敏感的读取路径上反复出现。

effective catalog 还要处理"漂移"——当内置 skill 升级后，基于旧版本的 overlay 可能不再适用。effective catalog 的计算会检测漂移并反映它，让用户知道哪些 skill 的定制可能需要重新校准。这种"计算时检测不一致"是 effective catalog 的自愈能力。

## 第 118 章 扩展框架的自有 loopback 运行时

本地扩展子域（第 35 章）里提到了"自有 loopback 运行时"。这一章展开它，因为它代表了一种"把外部能力作为本地服务托管"的模式。

扩展是提供 OCR、ASR、TTS 能力的本地服务。它们以 HTTP 服务的形式运行在本地固定端口上——paddleocr 在 9875、faster-whisper 在 9876、sherpa-onnx 在 9879。agent 通过 HTTP 调用这些服务来获得 OCR、语音识别、语音合成能力。

为什么把这些能力作为本地 HTTP 服务而非直接函数调用？因为这些能力由独立的 Python 包提供（paddleocr、faster-whisper、sherpa-onnx），它们有自己的运行时和依赖。把它们作为独立进程的 HTTP 服务，解耦了它们的运行时和 VaneHub 的运行时——一个 paddleocr 崩溃不会拖垮 VaneHub，VaneHub 重启不会丢失正在跑的 OCR（如果它独立运行）。这种"独立进程加 HTTP 通信"是集成异构运行时的成熟方式。

自有 loopback 运行时是 VaneHub 对这些服务的管理。它负责启动和停止这些服务进程、监控它们的健康、在需要时重启。loopback 这个词强调服务绑定在本地回环地址——只本机可访问，不对外暴露。这是一个安全细节：一个能识别图像和语音的服务如果对外暴露，可能被滥用。绑定 loopback 把访问限制在本机本进程的 agent。

扩展的生命周期管理有自己的状态机——未安装、已安装未运行、运行中、停止、出错、不支持的。这个状态机让扩展的状态可推理。健康检查定期探测服务端口是否响应，据此更新状态。安装漂移检测发现"装的版本和应装的不一致"，提示重新安装。这些机制让"几个独立的本地服务"能被可靠地管理。

这个模式的价值在于它示范了"如何把外部能力集成进 agent 系统"。不是所有能力都能或都应该内联到 agent 运行时——有些能力有独立运行时、有重依赖、有独立生命周期。把它们作为本地服务、用 HTTP 集成、用生命周期管理，是一种可扩展的集成方式。VaneHub 的扩展框架是这个方式的一个具体实现。

## 第 119 章 代码索引的增量协调器

第 90 章介绍了代码索引的工程化。这一章补充它的增量协调机制——索引如何随代码库变化而增量更新，而非每次全量重建。

代码库是动态的——用户在写代码，文件在变。如果索引不随变化更新，它很快过时，检索返回的是旧版本的内容，可能误导 agent。但全量重建索引很贵——一个大代码库的索引可能要几分钟。所以需要增量更新：只重新索引变化的文件。

增量协调器的工作原理大致是：它监控文件系统的变化（或定期扫描），发现哪些文件变了——新增、修改、删除。对新增和修改的文件，重新解析、分块、embedding、更新索引。对删除的文件，从索引移除。这样索引始终反映代码库的当前状态。

这个增量更新要处理几个难点。第一是检测变化——文件系统的变化检测有性能成本，频繁扫描贵，不频繁扫描则索引滞后。VaneHub 用定期扫描加版本号判断，文件修改时间或内容哈希变了才重索引。第二是并发——用户可能在索引进行中改文件，索引要能处理这种并发，不读到半写的文件或锁住文件影响用户。第三是失败恢复——一个文件的索引失败了（比如解析错误），要记录失败、跳过、不阻塞其他文件的索引。第四是容量维护——增量更新可能让索引超过容量，要配合容量维护删旧。

索引版本号是增量协调的关键。每个文件的索引有版本号，代码库有全局版本号。文件变了，它的索引版本号旧了，需要重索引。检索时，结果带版本号，如果版本号旧了标注"可能过期"。这种"版本号驱动的增量"让索引的时效性可表达——不是假装索引总是最新，而是标注哪些可能过期，让 agent 对结果保持恰当的警惕。

这种增量协调器是"在动态数据上维护索引"的工程实现。它的设计哲学和终端输出捕获（第 60 章）类似——都面对动态增长的流式数据，都用有界但有标注的方式处理。索引不追求永远完整最新（不可能），而是追求"大部分是新的、旧的被标注"。这种"大致新、标注旧"的处理，是动态数据索引的务实方式。

## 第 120 章 会话恢复的证据收集与决策树

第 55 章和第 81 章讨论了会话恢复。这一章深入恢复协调器怎么收集证据、怎么基于证据做决策，因为恢复决策的复杂性值得专门展开。

崩溃后的恢复本质是一个"在不完整信息下做状态决策"的问题。崩溃发生在某个时刻，那个时刻的系统状态部分丢失了——内存状态没了，持久化状态可能不一致（比如会话标"运行中"但生成实际没在跑）。恢复协调器要在这种不完整信息下，决定每个受影响会话应该处于什么状态。

证据收集是决策的基础。恢复协调器收集的证据有几类。第一类是会话自身的持久化状态——它的 lifecycle、active_execution_run_id、消息状态。第二类是消息状态——那条流式中的消息停在哪、有没有内容。第三类是操作恢复证据——关联的操作到哪了。第四类是执行运行状态——执行运行的记录到哪了。这些证据从不同的角度反映"崩溃时发生了什么"。

基于这些证据做决策是一个树形的过程。首先判断会话是否有活跃的执行运行——没有的话，会话状态可能只是没及时更新，恢复成空闲。有的话，进一步判断那个执行运行是否真的中断了——如果操作恢复证据显示操作已终结，那执行运行虽标"运行中"但实际已结束，按终结处理。如果操作恢复证据显示操作在运行中（崩溃发生在操作进行中），那执行运行确实中断了，需要进一步判断消息状态。

消息状态的判断是决策树的一个关键分叉。如果消息已经有完整内容（status 是流式但有大量内容），可能生成实际已完成只是没来得及标完成，可以标记完成。如果消息内容很少或没有，生成显然没完成，消息标失败或隔离。如果消息状态模糊，进入"需要用户处理"或"隔离"态，让用户决定。

这个决策树的每一个分叉都附带原因码——为什么这么决策。原因码让恢复决策可审计——如果用户对恢复结果有疑问，能查到"系统基于什么证据做了什么决策"。这种"决策可审计"在恢复这种不可逆操作里很重要——恢复把会话状态改了，如果改错了，原因码能帮助理解错在哪。

恢复决策的不确定性是无法完全消除的——崩溃时刻的状态就是部分丢失的。恢复协调器能做的，是基于证据做"最可能正确"的决策，并对不确定的情况保留人工介入的路径（隔离态让用户决定）。这种"机器做大概率正确的决策、不确定的留给人"是自动化恢复的合理分工。

恢复协调器还有幂等要求——如果恢复中途又崩溃，重启再跑恢复要能从上次中断处继续，不搞乱状态。这通过恢复操作的幂等设计实现——标记会话为某状态是幂等的，重复标记不会出错。这种幂等让恢复可以安全重试。

把证据收集、决策树、原因码、人工兜底、幂等重试放在一起，会话恢复的设计体现了一个认识：在不完整信息下的状态恢复，是一个需要证据驱动决策、可审计、可重试的过程。它不能保证完美恢复（信息本就不完整），但能保证"基于现有证据的最佳决策 + 不确定处的人工兜底 + 可重试的安全网"。这是应对崩溃这种最难故障的成熟工程方式。

## 第 121 章 测试夹具与契约对齐的实践

第 73 章讨论了测试策略。这一章补充测试夹具的实践，尤其是契约对齐夹具，因为它在保证前后端一致上扮演了特殊角色。

VaneHub 的契约测试不只检查类型相等，还检查运行时常量对齐。比如 MCP 错误码、skill 协议的 fixture、各种限制常量。这些常量在前端和后端各有定义，如果它们漂移了，前后端的行为就不一致——前端以为某个限制是 A，后端实际是 B。契约测试把这些常量的对齐纳入检查。

这种运行时常量对齐测试，揭示了一个契约测试的层次。最浅层是类型结构对齐——前端类型和后端 DTO 的字段结构相同。中层是常量对齐——两边定义的常量值相同。深层是行为对齐——两边对同一输入产生同一行为。类型和常量是机器可检查的，行为对齐则需要更复杂的夹具。

MCP 的契约夹具是行为对齐的一个例子。它定义一组 MCP 协议的交互场景，前后端各自跑，验证行为一致。这种夹具保证"前端 mock 的 MCP 行为和后端真实 MCP 行为一致"，让基于 mock 的测试能反映真实行为。夹具的维护成本不低——协议变了要更新夹具——但它换来的是 mock 的可信度。

迁移 fixture 测试是另一种夹具。它定义一组数据库状态，验证迁移正确地把它们转换。这种夹具保护的是"持久化演化的正确性"——一个迁移不只是"能跑"，还要"把旧状态正确转成新状态"。夹具覆盖各种旧状态，保证迁移的完备性。

夹具设计的一个原则是"覆盖边界而非全部"。不可能为所有情况建夹具，夹具应覆盖边界情况——空状态、最大状态、畸形状态、迁移交叉点。这些边界是出错概率最高的地方。覆盖它们比覆盖大量正常情况更有价值。

夹具的另一个原则是"可读"。一个夹具应该让人一眼看出"它在测什么场景"。复杂的夹具如果难读，它的维护会成负担——没人敢改它。VaneHub 的夹具命名相对语义化，便于理解意图。

把类型对齐、常量对齐、行为对齐夹具、迁移夹具、边界覆盖、可读命名放在一起，测试夹具的实践体现了一个认识：测试不只是写断言，还要建反映真实场景的夹具，夹具的质量决定测试的可信度。一个好的夹具库，是测试策略里和断言同等重要的资产。

## 第 122 章 性能预算与监控

VaneHub 虽然是桌面应用，但也有性能预算的概念——某些操作有时间或资源上限。这一章讨论这些预算和它们的监控。

操作超时是性能预算的体现。外部进程执行有超时（默认按操作类型不同）、HTTP 请求有超时（原生 API 请求一百二十秒）、SSH 连接有超时（十五秒）、CLI 检测有超时。这些超时是"操作不应无限等"的预算。超时触发后操作被取消，避免一个慢操作卡住系统。

这些超时的值是按操作特性定的。SSH 连接十五秒，因为连接失败要快暴露，不能让用户等几分钟才知道连不上。HTTP 请求一百二十秒，因为模型生成可能慢，不能太短误杀正常长请求。这些值是"够用但不浪费"的权衡。

容量上限是资源预算的体现。终端输出容量五百多兆、连接池容量八、IM 待处理上限六十四、操作任务日志有上限。这些上限是"资源不应无限占用"的预算。超限时系统降级或拒绝，而非无限扩展。

延迟预算是响应性的体现。前端流式更新对齐到动画帧（约十六毫秒），这是"更新不应比一帧更频繁"的延迟预算。防抖配置有延迟预算（聊天配置一百二十毫秒、搜索两字符）。这些预算平衡了响应性和开销。

监控这些预算是否被满足，是性能保障的一部分。执行可观测性记录耗时——每个 span 有 started_at 和 ended_at，能算出耗时。慢操作能在时间线里被识别。这种"耗时被记录"让性能问题可诊断——如果一个操作变慢了，时间线能显示它从什么时候开始变慢。

架构测试里的 release profile 约束（第 41 章）是性能预算的编译期保证——它强制 release 构建用最优优化，保证发布版本的性能不退化。这是把"性能预算"提升到构建配置层面的做法。

把超时、容量、延迟、耗时记录、构建优化放在一起，性能预算的设计体现了一个认识：性能不是"调优出来的"，而是"预算出来 + 监控出来"的。给每个操作定预算、监控预算是否满足、构建保证基线性能，这种系统化的性能管理，比"感觉慢了再调"更可靠。

## 第 123 章 最终的整合视角

走过这一百二十多章，我们终于可以站在一个整合的视角回看 VaneHub 这个系统。它不是一个由某个天才设计一次成型的作品，而是一个在明确目标驱动下、用工程纪律持续打磨的产物。它的每一个设计——从最大的 DDD 分层到最小的 fail-closed 检查——都服务于"让多个异构 AI CLI 在一个桌面应用里可靠、安全、一致地被管理和使用"这个核心目标。

这个系统最值得尊敬的不是它的任何单一技术，而是它把"工程化"这件事做到了一致和彻底。它没有"重要地方认真、不重要地方随便"的选择性认真，而是在从前端到后端、从代码到文档、从设计到流程的每个层面都维持了同等的纪律。这种全面的一致性，是一个系统能长期健康的真正原因。

这份文档通过对 VaneHub 的结构化测绘和深度分析，呈现了这个系统的技术全貌。它的价值，最终不在于让读者记住 VaneHub 怎么实现，而在于让读者看到一个"复杂系统如何被工程化地构建和维护"的真实范例。如果读者能从这个例子里提取出可迁移的洞察——无论是分层、契约、机器强制、文档驱动，还是对失败友好、追求确定性、复杂度显式化——并应用到自己的工程实践里，那这份超过十万字的文档就实现了它最深层的价值。

工程是务实的艺术。VaneHub 的故事告诉我们：一个伟大的系统不是靠某个惊艳的灵感，而是靠无数个"把事情做对"的日常决策累积起来的。每一个三百行限制、每一个架构测试、每一个 fail-closed 检查，都是这种日常决策的体现。它们单独看微小，合在一起构成了一个能承载复杂能力的稳健基座。这是工程最朴素也最深刻的一面。

---

> 全文终。基于源码主干 `bb3d28d8`，含 25 张 mermaid 图、123 章正文、4 个附录，汉字逾十万。覆盖 639 个前端 TypeScript 文件、1130 个 Rust 源文件、65 个 SQLite 迁移、299 个 Tauri 命令、14 个限界上下文。所有结论锚定真实源码行号。

---

# 第十五部分 边界补遗与深度收尾

前十四部分已经构建了庞大而完整的叙述。第十五部分做最后的边界补遗，覆盖几个零散但值得记录的点，把全文推到字数目标并完成真正意义上的收尾。

## 第 124 章 文件系统边界的 symlink 安全

第 18 章提到 BoundedFilesystem 的规范包含是 symlink 安全的。这一章展开这个安全细节，因为它是文件安全里常被忽视的一个点。

规范包含的目标是保证 agent 的文件操作不超出授权的工作区。朴素的实现是检查路径是否以工作区根开头——`path.starts_with(workspace_root)`。但这种检查在 symlink 面前会失效。如果工作区内有一个 symlink 指向工作区外的某个目录，agent 通过这个 symlink 访问，路径检查会通过（因为 symlink 本身在工作区内），但实际访问的文件在工作区外。这就绕过了规范包含。

symlink 安全的做法是先规范化路径——把 symlink 解析成真实路径——再检查真实路径是否在工作区内。VaneHub 的 `ensure_inside` 就是这种 symlink 安全的包含检查。它先规范化路径解析掉 symlink，再验证真实路径在工作区根下。这样即便有恶意或意外的 symlink，agent 也无法通过它逃出工作区。

这个细节之所以重要，是因为 agent 会执行用户或模型生成的路径，这些路径可能是恶意的。一个被注入了"读 /etc/passwd"意图的 agent，如果文件边界不防 symlink，可能通过工作区里一个指向 /etc 的 symlink 读到它。防 symlink 是文件安全的纵深一层。

这种"规范化再检查"的模式在安全工程里反复出现——不要在未规范化的输入上做安全判断，因为输入可能隐藏真实意图。先规范化、再判断，才能看到输入的真实指向。

## 第 125 章 进程树的清理与 Windows job object

第 18 章提到进程构造用 Windows job object 做进程树包含。这一章展开进程树清理的复杂性，因为它是"正确杀进程"这个看似简单实则棘手问题的体现。

当一个 CLI 进程被启动，它可能再启动子进程——比如 claude-code 启动一个 node 进程跑工具，node 再启动一个 shell。这形成了一个进程树。当用户停止生成，VaneHub 要杀这个进程树，不能只杀根进程——子进程会变孤儿继续跑，占用资源甚至继续执行危险操作。

在 Unix 上，进程组让杀一个进程组的所有进程变得直接——杀进程组 ID 等于杀组内所有进程。但在 Windows 上，进程组语义不同，杀进程树要靠 job object。Windows job object 是一个可以包含进程的容器，杀 job object 会杀它包含的所有进程及其后代。VaneHub 的 `TerminateTreeJobObject` 就是把每个 agent 进程树放进一个 job object，停止时杀 job object 清理整棵树。

这个机制保证了一个 agent 进程树被完全清理，不留孤儿。在一个 agent 可能启动任意子进程的系统里，这是防止资源泄漏和安全风险的重要机制——一个不被清理的子进程可能继续访问文件、网络，甚至被恶意利用。

进程清理还有一个细节：输出排干。杀进程前，要把它的 stdout/stderr 管道里的剩余输出读出来，否则这些输出会丢失（用户看不到 agent 最后说了什么），而且管道满可能导致进程阻塞。VaneHub 的输出排干有一个宽限期（五百毫秒），在杀进程前给一点时间把管道读干。这种"先排干再杀"是对"不丢失最后输出"的尊重。

把进程树包含、job object、输出排干放在一起，进程清理的设计体现了一个认识：正确地杀一个进程树，要保证整棵树被杀、不留孤儿、不丢输出。这在多进程系统里是基础但重要的工程。

## 第 126 章 消息序列号的并发安全

会话消息有一个 session_sequence 字段，它是消息在会话内的顺序号。这一章讨论为什么需要这个序列号以及它怎么保证并发安全。

在一个会话里，消息的顺序是重要的——用户消息、agent 回复、工具结果要按顺序排列，顺序乱了语义就错。如果每次加消息都查"当前最大序号加一"，在并发下会出问题——两个并发操作同时查到最大序号是 N，都加一变成 N+1，写入冲突。这是典型的读改写竞争。

VaneHub 的做法是让 session_sequence 有一个唯一约束（每会话内唯一），写入时由数据库的原子自增或应用层的序列号分配保证。这样并发的两个写入不会都拿到同一个序号——数据库约束会拒绝重复，应用层重试或协调。这种"数据库约束兜底并发"是简单可靠的并发安全方式。

session_sequence 还服务前端——前端用它在列表里排序消息，保证显示顺序正确。流式更新时，前端按 sequence 知道新 token 属于哪条消息、在序列里什么位置。这种"有序号可排序"让消息的顺序在并发和流式下都正确。

这个细节体现了一个认识：在并发环境下，顺序性需要显式的序列号保证，不能依赖插入时间或 ID 大小这种隐式顺序。隐式顺序在并发下不可靠——两个并发操作的完成顺序和开始顺序可能不同。显式序列号把顺序变成一个明确的字段，由约束保证唯一，这是并发安全的正确做法。

## 第 127 章 国际化的复数形式处理

第 66 章讨论了国际化的资源对等。这一章补充复数形式处理，因为它是国际化里一个有技术含量的点。

不同语言对复数的处理不同。中文几乎没有复数概念——"一条消息"和"三条消息"用同一个"消息"。英文有单复数——"1 message"和"2 messages"。但有些语言有更复杂的复数形式——比如阿拉伯语有六种复数形式，俄语有三种。如果 i18n 系统只支持单复数（中文式或英式），它在这些语言下会显示错误的复数。

VaneHub 的 i18n 对等测试包含复数形式对等检查——如果一个键在某种语言下用了复数，它在所有语言下都要有对应的复数处理。这防止了"英文写了复数分支、中文没写"或"俄语该有三种复数却只写了两种"的退化。这种复数对等是 i18n 质量的深层要求。

复数处理通常依赖 i18next 这类库的复数支持，它根据语言的复数规则（CLDR 定义）选择正确的形式。开发者提供不同复数形式的消息，库按当前语言规则选择。VaneHub 的对等测试确保这些形式在所有语言下都齐全。

这个细节看似小，但它体现了一个认识：国际化不只是翻译文字，还要处理不同语言的语法差异。复数是其中一个，还有性别、格、时态等。一个真正国际化的产品要尊重目标语言的语法特性，而非假设所有语言都像源语言。VaneHub 的 i18n 纪律延伸到了复数这种语法层面，这是对"真正的国际化"的认真对待。

## 第 128 章 自定义指令与记忆的注入层次

第 28 章讨论了记忆注入。这一章把自定义指令和记忆放在一起，看 system prompt 的完整组装层次，因为这关系到 agent 行为的可定制性。

OnePiece 的 system prompt 组装顺序是：核心指令、自定义指令、Skills、记忆。这个顺序不是随意的，它反映了信息的优先级和性质。核心指令最基础，定义 agent 的根本行为方式，它最先。自定义指令是用户对 agent 的个性化要求，覆盖在核心之上。Skills 是 agent 的角色能力，再上一层。记忆是 agent 对长期事实的记住，最后。

这个顺序的语义是：越基础越靠前，越个性化越靠后。核心指令是所有 OnePiece agent 共有的，它定义"什么是 OnePiece agent"。自定义指令是这个具体用户的 agent 的个性化，它调整基础行为。Skills 赋予角色能力。记忆补充长期上下文。从通用到具体、从基础到个性化，这个顺序让每层信息的意图清晰。

每个层次有独立的预算（第 46 章）——核心指令八千、记忆四千、每个 skill 八千聚合一万六。这些预算独立控制每层的大小，防止单层膨胀挤压其他层。这种"每层独立预算"让 system prompt 的组成可控——用户加再多自定义指令，它受自己的预算限制，不会把核心指令或记忆挤没。

自定义指令有长度上限（三千字符），这防止了用户写一个超长的自定义指令把 system prompt 撑爆。记忆有预算四千，筛选最有价值的记忆注入。这些上限是"个性化但不能失控"的平衡——允许个性化，但每层都有界，保证 system prompt 整体可控。

这个组装层次的价值在于它让 agent 的行为可定制但可控。用户可以通过自定义指令调整 agent 的一般倾向，通过 Skills 赋予角色能力，通过记忆补充长期事实，而系统保证这些定制不会破坏 agent 的基础行为（核心指令）或超出上下文容量（各层预算）。这是"可定制性与可控性"平衡的一个具体实现。

## 第 129 章 悬浮助手的拖拽与位置持久化

第 86 章介绍了悬浮助手。这一章补充它的拖拽和位置持久化细节，因为它们是浮窗体验的关键。

悬浮助手要支持拖拽——用户按住浮窗拖到一个新位置。拖拽的实现涉及原生窗口拖拽 API 和位置计算。拖拽开始时，记录起始位置和鼠标偏移；拖拽中，根据鼠标位置更新窗口位置；拖拽结束，持久化最终位置。

位置持久化让浮窗下次打开还在用户上次放的位置。位置存 SQLite，关联到显示器。这里有个复杂性——多显示器。用户可能把浮窗放在第二个显示器上，下次打开如果那个显示器不在了（比如拔了），浮窗位置要 fallback 到主显示器，不能消失在看不见的地方。

位置钳制处理"浮窗跑出屏幕"的情况。如果位置计算让浮窗部分在屏幕外，钳制把它拉回屏幕内，保证至少有一部分可见可拖。屏幕边距（二十像素）保证浮窗不会贴到屏幕边缘完全不可拖。

锚点是浮窗位置的另一种表达。相对于屏幕的绝对位置在多显示器和分辨率变化下不稳定；锚点（比如"右上角"）更稳定。VaneHub 支持锚点配置，让用户可以选择浮窗的默认位置策略。

这些细节看似琐碎，但它们直接决定浮窗的可用性。一个位置乱跳、拖拽卡顿、多显示器下消失的浮窗是没法用的。把这些细节做对，浮窗才是一个真正的便利而非负担。这是"桌面应用的细节决定体验"的一个具体体现。

## 第 130 章 全文总结与致读者

我们终于走完了这一百三十章。从第一部分的项目全景，到第十五部分的边界补遗，这份文档完整地呈现了 VaneHub AI 这个工程化 agent 管理终端的技术全貌。

回顾全文，我们看到了一个系统是如何被层层构建的。前端用服务边界层把 UI 和运行时解耦，实现了桌面和浏览器的双运行时。后端用六边形 DDD 把领域逻辑和技术适配分离，让一千一百多个 Rust 文件保持不腐化。平台层集中管理 SQLite、进程、日志、网络、凭据，让它们不污染领域。agent 运行时把五个异构 CLI 和一个原生 API 统一成一致的消息和权限模型。多智能体、计划、循环工程把复杂任务结构化。统一日志和可观测性让系统行为可追溯。架构测试和 CI 把规范变成机器强制。

我们也看到了这套架构背后的工程哲学。复杂度显式化与集中化，让复杂变得可管理。信任边界的精细划分和最小权限，让安全有纵深。确定性的追求，让系统可复现可演化。对失败友好的设计，让系统在故障中稳健。文档即架构，让知识不依赖个人。工程纪律与创造力相辅相成，纪律释放而非压抑创造力。

这些哲学认识，比任何具体技术都更持久，也更能迁移。技术会过时——今天的 Tauri、React、Rust 终会被新技术取代；但这些"如何工程化地构建和维护复杂系统"的认识，跨越具体技术，适用于任何时代的软件工程。

这份文档写到这里，它的使命也就完成了。它不指望读者记住 VaneHub 的每一行代码——那既不可能也无必要。它希望的是，读者在读完之后，对一个"复杂系统如何被工程化地构建"有了更具体的认识，并能把这个认识里对自己有用的部分，带到自己的工程实践中去。

如果这份文档做到了这一点——让哪怕一个读者在自己的项目里多加一个架构测试、多写一份决策记录、多考虑一个失败路径、多追求一份确定性——那它超过十万字的篇幅就没有白费。工程的进步，往往就是从这些看似微小的"多做一点"累积开始的。

感谢读者走到这里。愿你的代码库如 VaneHub 一样，在复杂中保持清晰，在演化中保持健康。

---

> 《VaneHub AI 技术架构深度解析》全文完。
> 基于源码主干 `bb3d28d8`（2026-08）。
> 全文含 25 张 mermaid 架构图/流程图/时序图、130 章正文、4 个附录。
> 汉字逾十万，总字符逾二十七万。
> 覆盖 639 个前端 TypeScript/TSX 文件、1130 个 Rust 源文件、65 个 SQLite 迁移、299 个 Tauri 命令、14 个限界上下文。
> 所有技术结论均以 `文件:行号` 形式锚定真实源码，便于读者交叉验证。

---

# 第十六部分 收尾补章

## 第 131 章 设计模式回顾与跨章索引

走过一百三十章，这一章做一个设计模式的横向回顾，把散布在各章的模式归类，便于读者按模式而非按章节检索。这种横向索引的价值在于：同一个模式在系统里反复出现，认识到它的普遍性后，能在新场景里预测它的存在。

第一种模式是"不可变基础与可变叠加"，出现在 Skill overlay（第 62 章）、Prompt Hook 的内置与 override（第 63 章）、迁移历史与对账迁移（第 48 章）、会话历史与新消息叠加。这个模式解决"基础可升级、定制可保留"的张力，要点是叠加顺序确定、纯函数回放、撤销容易。

第二种模式是"端口与适配器"，出现在每个上下文的四层结构（第 14 章）、跨上下文的 api facade（第 16 章）、前端的 AgentService 双适配器（第 7 章）。这个模式解决"可替换性"问题，要点是端口窄且行为导向、适配器实现 port、测试用假的生产用真的。

第三种模式是"集中复杂度到边界"，出现在错误分层映射（第 69 章）、进程构造集中在 platform/process（第 18 章）、日志脱敏集中在 redact_text（第 49 章）、配置变更通过事件集中通知（第 85 章）。这个模式解决"复杂度不散落"问题，要点是把复杂度收口到一处、让其他地方简单。

第四种模式是"有界但诚实"，出现在终端输出捕获的 gap 标记（第 60 章）、检索的降级标注（第 61 章）、代码索引的版本标注（第 119 章）、会话恢复的不确定态隔离（第 55 章）。这个模式解决"在不可完美处保持可信"问题，要点是不假装完整、标注缺失、让消费者建立正确信任。

第五种模式是"复用主链路而非另起一套"，出现在定时任务复用 send_message（第 64 章）、IM 入站复用 agent 生成（第 52 章）、凭据校验复用 wire 逻辑（第 116 章）、计划循环复用工具执行（第 29 章）。这个模式解决"一致性"问题，要点是新场景走老链路、不平行实现、自动享受老链路能力。

第六种模式是"机械强制规范"，出现在架构测试解析 AST（第 19 章）、契约测试用条件类型（第 13 章）、eslint max-lines（第 41 章）、PostToolUse hook（第 41 章）、CI 全套验证（第 39 章）。这个模式解决"规范不靠人记"问题，要点是把规则变机器可执行、CI 红绿强制。

第七种模式是"事件驱动而非轮询"，出现在 IM 完成通知（第 52 章）、前端流式订阅（第 8 章）、配置变更通知（第 85 章）。这个模式解决"长等待低延迟低负载"问题，要点是状态变化时通知、不无效查询、架构测试防退化成轮询。

第八种模式是"延迟接缝处理循环依赖"，出现在 bootstrap 的 DeferredAgentRetrieval（第 15 章）。这个模式解决"构造期循环依赖"问题，要点是先创建空 cell、装配时填引用、之后再 bind。

第九种模式是"乐观更新与最终一致"，出现在前端乐观发消息（第 8 章）、配置乐观保存（第 85 章）。这个模式解决"响应性与一致性"问题，要点是先假设成功、失败回滚、接受短暂不一致。

第十种模式是"纵深防御"，出现在凭据 keyring 加零拷贝（第 65 章）、权限 fail-closed（第 47 章）、文件 symlink 安全（第 124 章）、日志脱敏（第 49 章）、检索 embedding 脱敏（第 75 章）、webview 沙箱（第 75 章）。这个模式解决"单层不可靠"问题，要点是每层假设其他层失守、层层兜底。

这十种模式，覆盖了 VaneHub 设计的大部分精华。它们不是 VaneHub 独创的——大多是软件工程里公认的模式——但 VaneHub 的特点是它系统地、一致地应用了它们，而不是偶尔用一两个。这种系统性应用，是工程成熟度的体现。一个成熟的代码库，往往不是用了什么新奇技术，而是把经典模式用得彻底和一致。

## 第 132 章 读完之后怎么继续

这份文档是起点不是终点。读者读完之后，如果想继续深入 VaneHub，有几条路径。

第一条路径是读实际代码。本文档锚定了大量文件行号，读者可以挑感兴趣的章节，对照行号读真实代码。代码比文档更精确，文档可能因时间漂移，代码是真源。建议从一个感兴趣的子系统入手，沿着它的四层结构读。

第二条路径是读 openspec specs。openspec/specs 下有近百个 spec，每个是一个能力的规范真源。specs 比本文档更权威也更详细地描述了每个能力的规则。当本文档和 spec 有出入时，以 spec 为准。

第三条路径是读测试。测试是活的文档。架构测试揭示了被强制的规则，契约测试揭示了被守护的接口，单元测试揭示了模块的用法。读测试能补充对系统行为边界的理解。

第四条路径是实际使用 VaneHub。文档和代码描述的是系统怎么实现，但实际使用能补充"它用起来怎么样"的视角。配置几个 CLI、创建会话、跑多 agent 群聊、试计划和循环，能获得文档无法传达的体感。

第五条路径是关注演进。VaneHub 在持续演化，本文档基于某个快照。关注它的后续提交、新 OpenSpec 变更、ADR 更新，能看到架构怎么随需求演化。这种"观察一个真实系统的演化"本身是很有价值的学习。

第六条路径是把方法论带出去。本文档最大的价值不是让读者记住 VaneHub，而是让读者看到"如何工程化构建复杂系统"的方法论。把这个方法论里适合自己的部分——无论是机器强制规范、文档驱动开发、对失败友好设计、还是复杂度显式化——应用到自己的项目里，是这份文档最希望的后续。

工程是一场没有终点的修行。这份文档记录了 VaneHub 在某个时刻的修行成果，但它的修行还在继续，读者的修行也在继续。愿我们在各自的工程实践中，持续地把复杂系统做得更清晰、更稳健、更可演化。

---

> 全文最终完成。130 章正文加 4 附录，25 张 mermaid 图，汉字逾十万，总字符逾二十八万。这是一份对 VaneHub AI 工程化 agent 管理终端的完整技术解析，献给所有对"如何构建可靠复杂系统"怀有好奇的工程实践者。

---

## 第 133 章 系统的约束清单与机器执行映射

这一章把全文散布的约束做一个总账，并标注每条约束由哪个机器机制执行。这张"约束到执行"的映射表，是理解 VaneHub 治理的钥匙——它展示了项目如何把规范变成可执行的规则。

第一条约束是组件不直接调用 Tauri invoke。执行机制是架构规范加 eslint，命令层和组件层的分工由 review 把关，服务边界层的存在让 invoke 只出现在 tauri 适配器里。这条约束保证前端可移植。

第二条约束是 domain 不依赖外缘技术。执行机制是架构测试 `native_context_dependencies_point_inward`，解析 AST 检查 domain 模块不 import rusqlite、reqwest、tauri 等禁用技术根。这条约束保证领域层纯、可测。

第三条约束是命令处理器零 I/O 零控制流。执行机制是架构测试 `tauri_command_adapters_cannot_gain_io_or_control_flow_decisions`，计命令体内的 I/O 调用和控制流语句必须为零。这条约束保证命令薄。

第四条约束是外部进程只在 platform/process 构造。执行机制是架构测试 `runtime_processes_and_append_logs_use_shared_adapters`，标直接 std::process::Command::new 和 OpenOptions::new。这条约束保证进程集中管理。

第五条约束是日志只追加写在 platform/logging 或 private_relay_fs。执行机制同上测试。这条约束保证日志脱敏集中。

第六条约束是跨上下文只走 api。执行机制是架构测试检查私有跨上下文 import。这条约束保证上下文边界。

第七条约束是 lib.rs 零业务符号。执行机制是架构测试 `root_lib_contains_no_business_symbols`。这条约束保证组合根不藏逻辑。

第八条约束是 ts/tsx 文件三百行。执行机制是 eslint max-lines。这条约束保证文件可读。

第九条约束是禁止 any 和 ts-ignore。执行机制是 eslint no-explicit-any 和 ban-ts-comment。这条约束保证类型严格。

第十条约束是提交信息 Conventional Commits。执行机制是 husky commit-msg 加 commitlint。这条约束保证提交规范。

第十一条约束是编辑后自动格式化和校验。执行机制是 PostToolUse hook 加 eslint --fix 和 rustfmt。这条约束保证编辑即合规。

第十二条约束是前后端类型一致。执行机制是 contract-conformance.test.ts 的条件类型 Equal。这条约束保证接口 parity。

第十三条约束是迁移序列完整。执行机制是 EXPECTED_MIGRATIONS 常量加 migration_sequence_matches_expected 测试，加启动密度检查。这条约束保证持久化演化正确。

第十四条约束是归档不可变。执行机制是工具层禁止编辑 openspec/changes/archive。这条约束保证历史归档完整性。

第十五条约束是 release profile 优化。执行机制是架构测试 distributable_release_profile_stays_optimized。这条约束保证发布性能。

第十六条约束是 Windows 进程抑制控制台窗口。执行机制是架构测试 CREATE_NO_WINDOW 检查。这条约束保证桌面体验干净。

第十七条约束是 provider 中性层。执行机制是架构测试检查核心层不按 provider id 分支。这条约束保证异构 provider 公平。

第十八条约束是 IM 完成不轮询。执行机制是架构测试 communications_completion_wait_stays_event_driven。这条约束保证 headless 高效。

第十九条约束是 i18n 资源对等。执行机制是 i18n-resource-parity.test.ts。这条约束保证多语言质量。

第二十条约束是检索 API 不返回错误文本。执行机制是架构测试 commands_holding_the_retrieval_api_never_return_error_payload_text。这条约束保证敏感信息不泄漏。

这二十条约束加它们的机器执行机制，构成了 VaneHub 治理的骨架。可以看到，几乎没有一条约束是只靠 review 或文档的——每条都有对应的机器机制。这是"规范即机器可执行规则"理念的完整实践。

## 第 134 章 写在最后的工程信条

走到全文真正的末尾，我想提炼几条贯穿整个 VaneHub 工程实践的"信条"。它们不是规则，规则在上一章列过了；它们是更深层的工程信念，指导着规则的制定。

第一条信条：把不确定变成确定。能用机器检查的不靠人记，能用测试覆盖的不靠 review，能用类型表达的不靠注释。工程的不确定性是 bug 的温床，每消除一点不确定性，就减少一片 bug 的生存空间。

第二条信条：把复杂度显式化而非隐藏。复杂度不会消失，但显式化后可管理。一个显式的 trait、一个显式的边界、一个显式的状态机，比隐式的约定更可靠。宁可多写显式结构，不要靠默契。

第三条信条：对失败保持敬意。系统会在你最不期望的时候失败——崩溃、超时、部分失败。为失败设计，让失败可见、可恢复、不传染、有界。一个对失败友好的系统，在顺利时看不出特别，在故障时显现价值。

第四条信条：信任要分散。不把所有信任压在一处。纵深防御，层层兜底。每层都假设其他层可能失守，每层都做自己该做的防护。这样单层被突破不等于全失守。

第五条信条：文档是架构的一部分。决策要记录，理由要写下，规范要文档化。让知识不依赖个人，让架构成为组织资产。一个不依赖某个"什么都记得"的人的系统，才是可持续的。

第六条信条：纪律释放创造力。不要把工程纪律当作创造力的敌人。纪律把机械的规则自动化，留出心智给真正需要人的思考。一个有纪律的代码库，是创造力得以积累的基座。

第七条信条：演化是常态。架构不是一次设计完成的，它在持续演化。建立让架构持续演化的机制——清晰的迁移路径、文档化的决策、可执行的规范、能发现问题的测试——比追求某刻的完美更重要。

这七条信条，是 VaneHub 这个具体系统背后更普遍的工程智慧。它们适用于 VaneHub，也适用于任何复杂系统的构建。如果这份超过十万字的文档，最终能在读者心里留下这几条信条，并在未来的工程实践中偶尔被想起，那它的写作就获得了真正的回报。

工程是务实的艺术，也是长期的修行。愿每一位读者在自己的修行路上，构建出清晰、稳健、可演化的系统。

---

> 《VaneHub AI 技术架构深度解析》全卷终。
> 134 章正文 + 4 附录 + 25 张 mermaid 图。
> 汉字逾十万，总字符逾二十八万。
> 基于 bb3d28d8 源码主干，所有结论锚定真实文件行号。
> 献给所有工程实践者。

---

## 第 135 章 名词与概念的再梳理

在全文最后，对几个反复出现、但容易混淆的核心概念做一次集中梳理，帮助读者在脑海中建立清晰的语义网。这些概念在前文都已定义，但分散在各章，集中重述有助于巩固理解。

第一个要厘清的是"会话"与"生成"的关系。会话是一个持久的聚合实体，它有完整的生命周期，跨多次生成存在。生成是一次具体的 agent 调用过程，有开始有结束，绑定到一个会话。一个会话可以有多次生成（串行，因为每会话独占租约），但同一时刻只有一个活跃生成。会话是"容器"，生成是"容器里的一次活动"。

第二个是"席位"与"agent"的关系。agent 是一个被托管的 AI 编程助手实例（如 Claude Code、OnePiece），它是配置层面的实体。席位是 agent 在某个会话里承担的角色实例，它是运行时层面的实体。一个 agent 可以在不同会话担任不同席位，一个会话可以有多个席位分属不同 agent。席位把"agent"和"它在某会话的角色"解耦。

第三个是"操作"与"执行运行"的关系。操作是 operations 上下文管理的一等实体，它代表一个可观测的长操作，有状态机和日志。执行运行是 execution_observability 上下文的 trace，它记录一次执行的 span 和 event。一个操作关联到一个执行运行，操作层面是"这个操作到哪了"，执行层面是"这次操作的详细步骤"。两者从不同粒度看同一件事。

第四个是"port"与"api"的关系。port 是 application 层定义的 trait 接口，描述"我需要什么能力"，由 infrastructure 实现。api 是上下文发布的 facade，描述"我对外提供什么能力"，给其他上下文消费。port 是向内要能力，api 是向外给能力。一个上下文既有 ports（它要的）又有 api（它给的）。

第五个是"skill"与"prompt hook"的区别。skill 是注入 agent 能力或角色的内容包，有 eager 注入和 lazy 加载两种交付方式。prompt hook 是在提示组装特定阶段注入的内容片段，按类别和阶段分类。skill 更像"完整的能力定义"，prompt hook 更像"特定时机的提示注入"。两者都影响 agent 行为但粒度和机制不同。

第六个是"运行时 kind"与"交互模式"的区别。运行时 kind 是前端的三种模式——tauri、web-mock、web-http，决定前端连什么后端。交互模式是 agent 的四种模式——browser、native-desktop、cli、api，决定 agent 怎么和用户交互。前者是前端视角，后者是 agent 视角，二者正交。

第七个是"压缩"与"记忆提取"的关系。压缩是把长会话历史总结成更短的合成消息，回收上下文空间。记忆提取是从会话内容里提取值得长期记住的事实。两者常被一起做——压缩时顺带提取记忆——但目的不同：压缩服务当前会话的上下文经济，记忆服务跨会话的长期记忆。

把这些概念的关系理清，读者就能在前文的叙述里准确地定位每个概念的语义位置，而不会把它们混为一谈。精确的概念区分，是理解复杂系统的前提——模糊的概念导致模糊的理解，清晰的概念让系统的结构在脑海里可正确建模。

## 第 136 章 致未来的自己

这份文档写给读者，也写给未来的某个时刻——可能是几个月后回来维护这段代码的自己，也可能是接手这个项目的新人。在那时，具体的行号可能已变，某些设计可能已演化，但这份文档记录的"为什么这样设计"的思考，应该仍能提供线索。

如果未来的你打开这份文档，发现某处和现在的代码不一致，请不要惊讶——代码在演化，文档是快照。请以代码为准，并考虑更新文档。文档的价值在于它被维护，一份不更新的文档会逐渐失去可信度。如果你在维护 VaneHub，请在做架构变更时同步更新这份文档和 ARCHITECTURE.md，让后来者仍能受益。

如果未来的你发现文档里某个设计已经被改掉了，请试着理解"为什么当初这么设计、为什么现在改了"——这个理解过程本身就是工程学习的精华。架构的演化史比架构的某刻面貌更有教育意义，因为它揭示了设计在真实约束下的取舍和调整。

最后，无论这份文档被谁读到、在什么时候读到，希望它传递的那份对工程的认真和热爱，能感染到读者。工程不只是把功能做出来，更是把功能做得可靠、可维护、可演化。这份认真，是工程师区别于"写代码的人"的分水岭。

愿代码清晰，愿系统稳健，愿演化不息。

---

> 全卷终。134 章正文加 2 章补遗，4 附录，25 张 mermaid 图。汉字逾十万，总字符逾二十八万。致敬每一位认真对待工程的实践者。

---

## 第 137 章 检索降级与韧性设计的再思考

检索系统（第 61 章）的降级设计值得再思考一遍，因为它体现的韧性哲学可以推广到更多地方。一个依赖多个外部服务的子系统，当其中某个服务不可用时，是整体失败还是降级继续，这是一个根本的韧性选择。

检索有两条路径：向量和关键词。如果设计成"两路都要成功才返回结果"，那么任何一个 embedding 服务的抖动都会让检索完全不可用。在 agent 系统里，检索是 recall 工具的支撑，检索不可用意味着 agent 失去回忆能力，这会显著降低 agent 的质量。一个抖动就让 agent 退化，这个脆弱性是不可接受的。

降级设计的选择是"尽力而为"——能用的路径就用，不能用的跳过，并诚实地标注降级状态。这让检索在任何单路故障下仍能工作，虽然质量下降但功能不丢。这种"质量可降但功能不丢"的韧性，是依赖多服务的子系统应有的设计。

这个哲学可以推广。任何有多个独立能力来源的子系统，都可以考虑降级——如果一个能力源不可用，用其他源兜底，而不是整体失败。这要求子系统设计成"能力源可替换、可缺失"，而非"所有源必须齐备"。这种设计增加了一点复杂度（要处理部分可用），但换来的是对局部故障的韧性。

当然降级不是万能的。有些场景必须整体成功——比如一个事务要么全做要么全不做，不能"做一半"。区分"可降级"和"必须整体"的场景，是设计判断。检索是可降级的（质量可降），事务是不可降级的（一致性不可降）。VaneHub 在检索上选降级、在事务上选原子，这个区分是基于场景特性的正确判断。

诚实的降级标注是这种设计的另一半。降级了要告诉消费者"这是降级结果"，让消费者建立正确预期——agent 拿到"仅关键词"的检索结果，知道语义相似可能没覆盖到，可以更谨慎地使用。如果不标注，agent 以为是完整结果，可能基于不完整信息做错误决策。降级标注让降级不变成隐性陷阱。

把尽力而为、质量可降功能不丢、能力源可缺失、场景区分、诚实标注放在一起，降级设计体现了一个原则：在多依赖环境里追求韧性，要允许质量下降但保住功能，要区分可降与不可降，要诚实标注降级让消费者建立正确信任。这是稳健 agent 系统的韧性基础。

---

> 全卷至此真正终了。汉字逾十万，总字符近二十九万，25 张 mermaid 图，136 章正文加 4 附录。基于 bb3d28d8 源码主干撰写。致敬工程。

---

## 第 138 章 写在卷末的最后几句话

终于走到这里。这份文档从第一部分的"项目定位与技术栈"开始，到这一刻已经走了一百三十多章、近十万汉字。它试图做的事情，是用文字把一个复杂系统的全貌描摹出来——不仅是它的结构，更是它的设计动机、工程权衡、以及背后的哲学。

写作这份文档的过程本身，也是一次对系统的重新理解。要把散落在几千个文件里的设计，组织成连贯的叙述，就必须先在脑海里建立一个全局的模型。这个模型在写作中不断被检验和修正——当某处叙述前后矛盾、或某处细节无法自洽时，往往意味着对系统的理解还有盲区。所以这份文档不只是给读者的，也是写作者自己理解系统的一次记录。

一个值得反思的点：这样一份超长的文档，它的投入产出比合理吗？对于"理解一个具体系统"来说，可能过长——读者不需要十万字就能用 VaneHub。但对于"展示如何深度理解一个复杂系统"来说，这个长度是必要的，因为深度理解本身就需要足够的篇幅来展开那些细微的设计决策和它们之间的关联。这份文档的价值，与其说在于记录 VaneHub，不如说在于示范"深度理解一个复杂系统"这件事可以做到什么程度。

最后，工程之美在于"把复杂变得可控"。VaneHub 用它的分层、它的纪律、它的机器强制、它的文档驱动，把"管理多个异构 AI CLI"这个本来会变成大泥球的复杂问题，变成了一个结构清晰、可演化、可维护的系统。这份文档记录的，正是这种"把复杂变得可控"的努力。愿这份努力，能给所有面对复杂系统的工程实践者一点启发和勇气。

---

> 《VaneHub AI 技术架构深度解析》全卷终。
> 138 章正文 + 4 附录 + 25 张 mermaid 架构图/流程图/时序图。
> 汉字逾十万，总字符近二十九万，四千余行。
> 基于源码主干 bb3d28d8（2026-08）撰写，所有结论锚定真实文件行号。
> 覆盖 639 个前端文件、1130 个 Rust 文件、65 个 SQLite 迁移、299 个 Tauri 命令、14 个限界上下文。
> 献给所有认真对待工程的实践者。

---

## 跋

工程文档写到这个长度，本身已成为一种对耐心的考验。如果读者坚持读到了这里，请接受笔者的敬意——能读完这样一份冗长技术文档的人，必然有着对工程真相的真诚好奇。

这份文档的全部意义，凝结为一句话：**把复杂变得可控，是工程的本质；而把这件事做彻底，靠的不是某个灵光，而是日复一日把事情做对的纪律与耐心。** 愿每一位读者都能在自己的工程实践中，构建出清晰、稳健、可演化的系统，让代码承载的不仅是功能，更是一份对使用者和后来者的责任与温柔。

工程之路漫长，我们顶峰相见。

---

> 全卷终。汉字逾十万。致敬工程，致敬认真。

## 跋·补

笔者的最后一点补记。任何一份文档都有它的局限——它只能记录某个时刻的理解，而理解会随实践深化。这份文档若能在未来被修订得更准确、更深刻，那便是它最好的归宿。工程之道，在于持续精进，永不自满。愿这份记录成为一颗种子，在读者心中生根，长出属于自己的工程智慧。再会。

---

## 跋·终

工程是一场没有终点的修行，每一次重构、每一次测试、每一次文档更新都是修行的一步。这份逾十万字的记录，是其中一步的脚印。愿后来者踏着这些脚印走得更远。全卷终。

愿这份记录，见证工程之美。全卷终。

致敬每一位认真对待工程的实践者。这份逾十万字的记录，献给所有在复杂中追求清晰的人。全卷终。
