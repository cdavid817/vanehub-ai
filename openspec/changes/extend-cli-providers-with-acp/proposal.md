## Why

VaneHub AI 需要扩展国内及常用编码 CLI 的覆盖范围，但只向选择器添加名称不能完成接入。当前审阅基线为 `main@34a449e80a1badc16fcfb49dcbad26134aecf2d9`：受审计 CLI 目录和运行时兼容注册表都只包含 `claude-code`、`codex-cli`、`gemini-cli`、`opencode`、`antigravity-cli`；启动语法与权限映射也按这些 ID 分派。既有 Provider SDK、CLI 环境快照、操作计划、权限与统一日志基础应继续复用。[仓库证据](references/repository-audit.md)

本变更交付 Qwen Code、Kimi Code CLI、Qoder CLI、CodeBuddy Code、GitHub Copilot CLI、Cursor Agent CLI 的静态内置适配，以及 iFlow 的明确受限历史兼容入口。国内 CLI 优先，整个 change 的实现范围不因阶段排序而缩水。

六种活跃 CLI 已有官方 ACP 入口，但协议相同不代表权限、认证、会话恢复、扩展方法或安装渠道相同。iFlow 官方已公告在 2026-04-17 关闭官方服务和 API；已安装程序仍可使用自定义 API，不能作为仍有官方服务的新产品推广。[外部证据](references/official-sources.md)

## What Changes

- 增加七个稳定 Provider ID；现有五个 ID、默认选择和历史会话不改名、不替换。
- 为六种活跃 CLI 增加原生终端和受管 ACP 会话适配；实际可用性按发行形态、版本、平台、宿主实现与运行策略判定。
- iFlow 保留本地检测、用户显式启用和原生终端兼容，默认无自动安装、无受管对话、无无人值守调用；不猜测其 ACP 参数。
- 扩展现有 source-aware CLI 环境管理，不另建安装器或旁路 PATH 扫描器。npm 源、已审计 vendor 源与 detect-only 源保持各自语义。
- 新增本地 stdio ACP 会话通道，将进程、连接、会话和 prompt turn 生命周期分开；保留既有 headless 和 PTY 实现。
- 完成双向请求、取消、审批、用户问题、计划确认、流式事件、断线和有界资源治理，不能把 ACP 当成新的 stdout parser。
- 扩展内部 Provider SDK 的 transport-aware 契约与 data-only Manifest V2，继续接受 V1，继续禁止外部动态 Provider 与可执行 Manifest。
- 权限投影复用当前 permission context。只能对真实代理的操作提供宿主强制保障，不宣称 ACP 本身构成 OS 沙箱。
- 会话持久化绑定 Provider、CLI 发行形态、实际程序、协议通道、工作目录与外部会话 ID；禁止最近会话猜测、跨 CLI 假恢复和副作用自动重放。
- 增加能力驱动的 CLI 管理、创建会话、运行状态和审批交互；Tauri 与 Web/mock 契约同步，web-http 缺失实现仍明确报错。
- 新 Provider 经同一运行时进入多 Agent 与定时任务；无法满足无人值守权限或恢复要求时显式阻止，不降级为自动批准。
- 交付无真实模型调用的协议桩、Provider 合约测试、UI/E2E、桌面测试和独立真实联调证据模板。

## Capabilities

### New Capabilities

- `expanded-cli-provider-adapters`: 六种活跃 CLI 的独立适配及 iFlow 历史兼容边界。
- `acp-agent-runtime`: stdio ACP 连接、会话、轮次、消息、取消与有界资源管理。
- `acp-permission-bridge`: ACP 审批、扩展交互、受控文件/终端代理及权限保障等级。
- `cli-session-transport-binding`: 会话身份、发行形态、恢复、通道切换和迁移兼容。
- `cli-provider-capability-ui`: 安装与就绪状态、能力选择、审批和中文/英文交互。
- `cli-provider-automation`: 多 Agent 与定时任务的能力门控、隔离、等待和恢复语义。

### Modified Capabilities

- `provider-plugin-sdk`: 扩展内部契约、Manifest 版本、transport conformance；保留原 requirement 与 scenario 标题及既有安全约束。
- `cli-environment-management`: 追加扩展目录、身份校验、认证引导与 source-safe 生命周期要求；不重写已实现的安装计划模型。

## Impact

### Desktop / native

影响 Rust `agent_runtime`、`tooling/cli`、现有权限/操作/日志公共边界、组合根及必要的增量 SQLite 迁移。ACP 协议与子进程 I/O 均在 Rust infrastructure，不在 React 或前端 Web Worker 启动本地程序。

### Web

同一 React UI 使用 service；Web/mock 使用显式标记的确定性合成状态、假协议交互，不声称访问了宿主 PATH、凭据或文件。此次不提供远程 ACP server 或新的 HTTP 后端。

### Compatibility and scope

本 change 是功能增量，不包含动态插件市场、自动切换模型供应商、全局重构老 Provider、自动迁移用户 CLI 配置、跨产品会话转换、自动删除 Worktree，或未获授权的安装/登录/付费调用。原始终端与受管 ACP 属于同一 Provider 的不同执行通道，不创建 `qwen-acp` 等重复 Agent 身份。

产品目标是完整接入；若实际版本不支持某能力，必须返回真实的 unsupported/unknown。真实 CLI 或平台联调受环境阻塞时，保留未通过验收项与原因，不能把所有能力关掉后宣布整体完成。

## Delivery and Acceptance

实施顺序为：基础协议和门控 → Qwen/Kimi → Qoder/CodeBuddy → Copilot/Cursor → iFlow 与全场景回归。全部阶段属于当前 change，具体任务见 [tasks.md](tasks.md)，发布门槛见 [acceptance.md](acceptance.md)。本包只有规范和执行说明，不包含已经实施或已经通过真实联调的声明。
