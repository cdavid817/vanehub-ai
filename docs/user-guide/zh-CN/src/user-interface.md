# 用户界面

主窗口的布局、导航，以及会话之外的那些入口：会话列表、Agent 选择、对话区、浮动助手、循环中心、通知与系统托盘。

会话内部的九个工作区标签页见[会话工作区](session-workspace.md)；设置项见[设置](settings.md)。

## 会话管理

### 新建会话

选**新建**打开创建会话对话框，选会话类型（单 Agent / 多 Agent）、Agent、工作区（本地/远端）、项目文件夹、会话名称。Git 项目会标 **Git** 标记并可创建 worktree。多 Agent 时分配席位，见 [多 Agent 群聊](multi-agent-workflow.md)。

![创建会话对话框](assets/screenshots/create-session-zh-CN.png)

### 会话列表

左侧会话列表支持三种展示模式：**列表 / 分类 / 按项目**。可按名称或内容搜索、按 Agent 筛选、批量选择、拖拽排序、收藏筛选。**右键会话**可重命名、删除、归档、导出、置顶、分配分类。

### 专注模式

顶栏的**专注模式**折叠左侧会话列表和右侧信息面板，让工作区占满；再点一次恢复。顶栏还有**全局搜索**，跨会话搜消息和内容。

### 活动栏导航

会话列表左侧的活动栏切换主要功能区：**会话 / 循环工程 / 任务看板 / 目标中心 / Agent 评测 / 任务控制台 / 定时任务 / 设置 / 帮助**。

## Agent 类型

VaneHub AI 接入 6 个 Agent，分两类：

### 外部 CLI Agent

前五个是**外部 CLI**——VaneHub AI 启动它们的进程并管理进程之外的部分（启动参数、权限拦截、输出采集），真正的代码生成由 CLI 自己完成。**官方订阅登录由各 CLI 自己管**，VaneHub AI 不保存由此产生的凭据；但要把它们换成第三方兼容端点，可以在[设置 → Agent 配置](agent-configuration.md#agent-配置)里配。

| Agent | 提供方 | 命令 | 说明 |
| --- | --- | --- | --- |
| Claude Code | Anthropic | `claude` | Anthropic 官方 CLI，需 Anthropic 订阅或 API 凭据 |
| Codex CLI | OpenAI | `codex` | OpenAI 官方 CLI，需 OpenAI 账号 |
| Gemini CLI | Google | `gemini` | Google 官方 CLI，用 Google 账号认证 |
| Antigravity CLI | Google | `agy` | Google 官方 CLI，走 Google 登录并存入系统钥匙串 |
| OpenCode | OpenCode | `opencode` | 开源 CLI，支持多家 provider |

安装、认证与可用性检测见[安装并认证 CLI](getting-started.md)。

### VaneHub 原生 Agent：OnePiece

**OnePiece** 不同：它直接通过 HTTP 调用模型 provider，完全在应用内运行，**不依赖任何外部 CLI**。它的 API Key 由 VaneHub AI 保存，支持 25 家 provider（Anthropic、OpenAI 等官方目录 + 常用兼容端点），也可配置自定义兼容端点。

- 不想装 CLI 就能用——见[原生 API Agent](native-agent.md)
- 即使你主力用外部 CLI，记忆提取也由 OnePiece 代做，所以通常也要配好 OnePiece

## 对话

### 发送消息

工作区底部的输入框写任务：**Enter 发送、Shift+Enter 换行**。输入框上方有一排选择器和开关：

| 控件 | 作用 |
| --- | --- |
| Provider / Model / Agent 下拉 | 按你配置的可用项切换 |
| 交互模式 / 推理深度 / 配置 | 调整本次对话参数 |
| 流式开关 | 是否流式输出回复 |
| 思维链开关 | 是否显示 thinking 内容 |
| 增强按钮 | 对提示词做增强处理 |
| 文件引用 / 附件 | 把文件作为上下文附加给 Agent |

### 查看回复

Agent 回复支持富内容渲染：代码块（语法高亮）、Mermaid 图（内嵌渲染流程图/时序图）、思维块（折叠展示 thinking 过程）、工具调用（显示工具名与参数/结果）、图片/音频/卡片/清单/diff（按类型渲染）。长对话里有**回到底部**按钮快速跳到最新消息；首次进入显示**欢迎屏**。

### 轮次状态

对话区顶部有**轮次状态栏**：当前发言权归属、等待人类时长、轮次完成、链深度提示。多 Agent 群聊交接时显示 `交接 1/15`。细节见 [多 Agent 群聊](multi-agent-workflow.md)。

## 浮动助手

设置里开启浮动助手后，桌面上有独立浮窗：在浮窗里启动会话/助手（不必打开主窗口）、状态徽章显示运行状态、主操作菜单快速发起任务。

## 循环中心

左侧活动栏的**循环**管理 Loop 工程：运行列表与检视、运行控件（暂停/继续/取消/接受/拒绝）、验证命令编辑器、时间线。Loop 的概念与创建见 [Loop Engineering 工程](loop-engineering.md)。

![循环中心](assets/screenshots/loop-center-zh-CN.png)

## 通知

顶栏的铃铛图标进入**通知中心**：未读数角标、全部已读、清除通知。通知的作用域（全局/会话）和四类通知见 [定时任务与通知](scheduled-tasks.md)。

## 系统托盘

桌面端在系统托盘有图标：显示/隐藏主窗口、开机自启开关在**设置 → 基础配置**里、托盘通知与系统通知联动。

## 相关

- 术语不熟 → [核心概念](core-concepts.md)
- 第一次用 → [创建第一个会话](first-session.md)
- 会话内的标签页 → [会话工作区](session-workspace.md)
- 配置项 → [设置](settings.md)
