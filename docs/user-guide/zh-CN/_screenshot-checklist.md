# 中文用户指南 · 截图任务清单

> 本文件位于书根，mdBook 只读取 `src/`，不会被收录进文档站。

## 截图机制（重要前提）

本仓库的文档截图**不是手工截的**，而是由具名 Playwright 场景确定性生成：

| 环节 | 位置 |
| --- | --- |
| 场景定义 | `tests/docs/documentation-screenshots.spec.ts` 的 `scenarios` 映射 |
| 清单 | `docs/user-guide/screenshots.json` |
| 生成 | `npm run docs:screenshots:update` |
| 校验 | `npm run docs:screenshots:check`（CI 恒跑） |

**新增一张截图 = 加一个 scenario 函数 + 加一条 `screenshots.json` 条目 + 跑一次 update。**

### 两条硬约束

1. **截图跑在 Web/mock 运行时**（`npm run dev`），Playwright 连的是浏览器。**无法截取 Tauri 桌面窗口。**规范允许 `desktop-reviewed` 类别，但那需要人工拍摄并人工审核，不在自动流水线内。
2. **Web/mock 截图不得用作原生副作用的证据。**它渲染的是同一套界面，可以展示界面长什么样、控件在哪；但不能用来证明"真的启动了进程 / 写了文件"。

### 一条容易踩的坑

**不要为尚不存在的图片提前写图片链接。**`scripts/validate-docs.mjs` 会校验每个图片链接的目标文件存在，且它是**对原始文本做正则匹配、不解析 Markdown**——用反引号包裹或放进代码块都不能让它忽略。占位链接会直接让 CI 失败。

**正确顺序是：先出图，再插链接。**

### 不要随意重新生成既有截图

本机渲染与 CI 环境存在字体与抗锯齿差异。`update` 会覆盖所有截图，导致既有图片产生无意义的 diff。**仅在有意审核 UI 变更时才运行 update**，并在提交前还原不该变的文件。

## 已完成

| 图片文件名 | 所在文档 | 展示的页面/状态 | 复现步骤 | 建议标注 |
| --- | --- | --- | --- | --- |
| `create-session-zh-CN.png` | `first-session.md` | 创建会话对话框，单 Agent，已填项目与标题 | 新建 → 填 `D:\VaneHub-Demo` → 填标题 | 无 |
| `create-session-en.png` | （英文版 `first-session.md`） | 同上，英文界面 | 同上 | 无 |
| `create-session-multi-agent-zh-CN.png` | `multi-agent-workflow.md` | 创建会话对话框，**已选中多 Agent**，显示席位分配区 | 新建 → 填项目与标题 → 点击「多 Agent」 | 建议红框标注「会话类型」区与席位分配区 |
| `permissions-zh-CN.png` | `permissions.md` | Agent 权限策略页，5 个 Agent 与四档模板 | 访问 `/settings?section=agent-policies` | 红框标注模板按钮组 |
| `personalization-zh-CN.png` | `personalization.md` | 个性化页，自定义指令与记忆两区 | `/settings?section=personalization` | 红框标注「关于你」「回复风格」 |
| `expert-roles-zh-CN.png` | `personalization.md` | 专家角色页，三个内置角色 | `/settings?section=expert-roles` | 箭头指向复制按钮 |
| `mcp-zh-CN.png` | `tooling.md` | MCP 服务器页，用户配置与项目配置 | `/settings?section=mcp` | 红框标注传输方式标签 |
| `cli-zh-CN.png` | `tooling.md` | CLI 管理页，四个 CLI 卡片 | `/settings?section=providers` | 红框标注「本地 CLI 检测仅在桌面运行时可用」提示 |
| `usage-zh-CN.png` | `automation.md` | 使用统计页，Token 卡片与趋势 | `/settings?section=usage` | 箭头指向「真实数据覆盖率」 |
| `loop-center-zh-CN.png` | `loop-engineering.md` | 循环工程中心三栏布局（空态） | 活动栏「循环工程」 | 标注左栏「定义」与右栏「检查器」 |
| `skills-zh-CN.png` | `skill-management.md` | Skill 管理页，按 Agent 分组与 6 个内置 Skill | `/settings?section=skills` | 红框标注左侧「按 Agent 管理」 |
| `prompt-hooks-zh-CN.png` | `tooling.md` | Prompt Hook 页 | `/settings?section=prompt-hooks` | 无 |
| `im-zh-CN.png` | `remote-and-im.md` | IM 能力页，默认路由与五个连接器 | `/settings?section=im` | 红框标注「默认路由」，箭头指向「个人微信 · 实验性」 |
| `ssh-zh-CN.png` | `remote-and-im.md` | SSH 连接页 | `/settings?section=ssh-connections` | 无 |
| `extensions-zh-CN.png` | `tooling.md` | 扩展能力页，PaddleOCR 与 faster-whisper | `/settings?section=extensions` | 箭头指向「预计磁盘占用」 |
| `observability-zh-CN.png` | `observability.md` | 执行可观测性页，本地时间线与 OTLP | `/settings?section=observability` | 红框标注「保留天数」与「外部采样比例」 |
| `scheduled-tasks-zh-CN.png` | `automation.md` | 定时任务对话框，任务列表与新建表单 | 活动栏「定时任务」 | 箭头指向底部补跑说明 |

## 待补充

以下截图有价值但**尚未验证能否在 Web/mock 中稳定复现**，需要先确认目标界面在模拟数据下渲染完整，再补 scenario。

| 建议文件名 | 拟用于 | 需要展示的页面/状态 | 复现步骤（待验证） | 建议标注 |
| --- | --- | --- | --- | --- |
| `settings-agent-policies-zh-CN.png` | `permissions.md` | 设置 → Agent 权限策略，四档模板可见 | 活动栏「设置」→ 侧栏「Agent 权限策略」 | 红框标注模板选择区 |
| `approval-dialog-zh-CN.png` | `permissions.md` | 审批弹窗，含作用域选项 | **需要触发一次 `Ask` 判定**——Web/mock 是否可复现待确认 | 箭头指向记忆范围选项 |
| `settings-personalization-zh-CN.png` | `personalization.md` | 设置 → 个性化，两段指令与记忆开关 | 活动栏「设置」→ 侧栏「个性化」 | 红框标注「关于你」「风格规则」 |
| `settings-expert-roles-zh-CN.png` | `personalization.md` | 设置 → 专家角色，三个内置角色 | 活动栏「设置」→ 侧栏「专家角色」 | 红框标注「职责」字段 |
| `loop-center-zh-CN.png` | `loop-engineering.md` | 循环工程中心，定义列表 | 活动栏「循环工程」 | 无 |
| `loop-definition-dialog-zh-CN.png` | `loop-engineering.md` | Loop 定义对话框，限额与验收命令 | 循环工程 → 新建 | 红框标注迭代上限与验收命令区 |
| `session-tabs-zh-CN.png` | `quick-start.md` | 会话工作区，9 个标签页可见 | 创建会话后进入工作区 | 逐个标注标签名 |
| `traces-tab-zh-CN.png` | `observability.md` | 链路标签，Span 树 | 会话 → 链路标签 | 箭头指向「不透明」节点 |
| `logs-tab-zh-CN.png` | `observability.md` | 日志标签，搜索与定位 | 会话 → 日志标签 | 红框标注搜索框与定位控件 |
| `settings-mcp-zh-CN.png` | `tooling.md` | 设置 → MCP 服务器 | 活动栏「设置」→ 侧栏「MCP 服务器」 | 无 |
| `settings-cli-zh-CN.png` | `tooling.md` | 设置 → CLI 管理，四个 CLI 状态 | 活动栏「设置」→ 侧栏「CLI 管理」 | 红框标注冲突提示 |
| `scheduled-tasks-zh-CN.png` | `automation.md` | 定时任务对话框，频率选择 | 活动栏「定时任务」 | 红框标注频率选项 |
| `usage-statistics-zh-CN.png` | `automation.md` | 设置 → 使用统计，四维 token | 活动栏「设置」→ 侧栏「使用统计」 | 箭头指向口径说明 |

## 桌面专属、无法自动截图的项

以下界面依赖原生运行时，Web/mock 中要么不渲染、要么只是模拟态，**若要配图需人工截取并标为 `desktop-reviewed`**：

| 场景 | 说明 |
| --- | --- |
| SSH 连接成功后的远程终端 | 需要真实 SSH 连接 |
| IM 连接器已连接状态 | 需要真实开放平台凭据 |
| 微信扫码授权二维码 | 同上，且含动态内容 |
| CLI 冲突的真实检测结果 | 依赖本机实际安装状态 |
| 权限审批的真实拦截 | 需要真实进程执行 |

**人工截图前请注意规范要求**：不得包含凭据、令牌、个人文件系统路径、未脱敏日志；需提供本地化的替代文本。
