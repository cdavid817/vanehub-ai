# 设置中心

设置中心的分组结构，以及界面语言、主题、字号、默认权限模板、开机自启、网络代理、数据目录与日志目录这些基础配置项。

单个功能的设置项在各自的章节里说明；这一章只讲设置中心本身和跨功能的基础配置。

左侧活动栏的**设置**进入设置中心，左侧是设置项导航，右侧是配置页。共 20 个设置页：

| 设置页 | 内容 |
| --- | --- |
| **基础配置** | 见[下一节](#基础配置) |
| **CLI 管理** | 各 CLI 的安装检测、冲突诊断与升级，见 [安装并认证 CLI](getting-started.md) |
| **CLI 参数** | 按 CLI Agent 配置启动参数，见 [工具与扩展](agent-configuration.md#cli-参数) |
| **扩展能力** | 本地多模态能力的安装与启停，见 [工具与扩展](extensions.md#扩展能力) |
| **插件集成** | 内置产品集成与就绪检测，见 [插件集成](plugin-integration.md) |
| **MCP 服务器** | MCP server 配置与按 Agent 绑定，见 [MCP 服务器](mcp.md) |
| **Agent 配置** | 按 Agent 配置 provider、端点与模型（含 OnePiece），见 [工具与扩展](agent-configuration.md#agent-配置) |
| **Agent 权限策略** | 权限策略与审批模板，见 [权限审批](permissions.md) |
| **专家角色** | 角色与评审策略，见 [专家角色](expert-roles.md) |
| **AI 个性化** | Custom Instructions 与跨会话记忆，见 [个性化](personalization.md) |
| **Skill 管理** | Skill 安装与绑定，见 [Skill 管理](skill-management.md) |
| **Prompt Hook** | 钩子管理，见 [Prompt Hook](prompt-hooks.md) |
| **IM 能力** | IM 连接器配置，见 [远程与 IM](im-connectors.md) |
| **SSH 连接** | 保存的 SSH 连接，见 [远程与 IM](remote-workspaces.md) |
| **执行可观测性** | 执行追踪与日志采集策略，见 [可观测性](observability.md) |
| **使用统计** | Token 用量统计，见 [使用统计](usage-statistics.md) |
| **代码智能** | 语言服务器启用、发现与工作区信任，见 [LSP 代码智能](lsp-code-intelligence.md) |
| **本地媒体** | 本地 OCR、语音识别与语音合成引擎，见 [本地媒体](local-media.md) |
| **关于** | 版本、更新检查、changelog、仓库链接，见 [版本更新](app-updates.md) |
| **使用文档** | 以你的界面语言渲染随应用附带的产品文档 |

## 基础配置

**设置 → 基础配置**是设置中心的默认落地页，管的是应用本身的行为，与具体 Agent 无关。

![设置中的基础配置页面](assets/screenshots/settings-basic-zh-CN.png)

| 分组 | 项 | 说明 |
| --- | --- | --- |
| **外观** | 界面语言 | 客户端默认跟随宿主系统 locale |
| | 主题、字号 | 影响全局渲染 |
| **安全** | 默认权限模板 | 新建会话的默认模板，语义见[权限审批](permissions.md) |
| **启动** | 开机自启 | 与[系统托盘](user-interface.md#系统托盘)联动 |
| | 浮动助手开关 | 开启后才有[浮动助手](user-interface.md#浮动助手)窗口 |
| **网络** | 节点信息、网络代理 | 代理支持带认证 |
| **存储** | 数据目录、日志目录 | 改完需重启，按新目录重建；日志路径细节见[故障排查](troubleshooting.md#日志在哪) |
| | 文件夹打开器 | 决定「在文件管理器中打开」调用什么 |

> **改数据目录要当心**。多个 worktree 共用同一个数据库时，跨分支的迁移版本号可能撞车，见[故障排查](troubleshooting.md#启动报-no-such-table)。

## 相关

- 权限模板 → [权限审批](permissions.md)
- Agent 与 CLI 配置 → [Agent 与 CLI 配置](agent-configuration.md)
- 主窗口布局 → [用户界面](user-interface.md)
