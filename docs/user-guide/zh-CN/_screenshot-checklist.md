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

1. **截图跑在 Web/mock 运行时**（`npm run dev`），Playwright 连的是浏览器。**无法截取 Tauri 桌面窗口**。规范允许 `desktop-reviewed` 类别，但那需要人工拍摄并人工审核，不在自动流水线内。
2. **Web/mock 截图不得用作原生副作用的证据**。它渲染的是同一套界面，可以展示界面长什么样、控件在哪；但不能用来证明"真的启动了进程 / 写了文件"。

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
| `session-workspace-zh-CN.png` | `quick-start.md` | 会话工作区，9 个标签页 + 信息面板 | 新建 → 创建 → 关闭成功提示 | 逐个标注标签名 |
| `session-traces-zh-CN.png` | `observability.md` | 链路标签，执行时间线与链路拓扑 | 同上 → 点「链路」 | 箭头指向「不可见」徽标与「观测缺口」提示 |
| `session-logs-zh-CN.png` | `observability.md` | 日志标签，搜索与时间定位 | 同上 → 点「日志」 | 红框标注搜索框与定位控件 |
| `im-connected-zh-CN.png` | `remote-and-im.md` | IM 页，飞书处于**已连接**（Web/mock 模拟态） | 建会话 → 设置 → IM → 配默认路由 → 填飞书凭据 → 勾选启用 | 红框标注「已连接」徽标；**时间戳已遮罩** |

## 待补充

**只剩一项**。其余候选都已完成，见上表。

| 建议文件名 | 拟用于 | 需要展示的页面/状态 | 复现步骤（待验证） | 建议标注 |
| --- | --- | --- | --- | --- |
| `loop-definition-dialog-zh-CN.png` | `loop-engineering.md` | Loop 定义对话框，限额与验收命令 | 活动栏「循环工程」→ 左栏 **+** | 红框标注迭代上限与验收命令区 |

补的时候留意：对话框里若有实时计算的下次运行时间之类的动态内容，需要用 `mask` 遮掉，否则回归校验会失败。

## 尝试过但放弃的：工具审批

**结论：可以走到，但截不稳，已放弃。**

审批状态在 Web/mock 中是可达的——配好 OnePiece provider、用它建会话、发一条消息，工具调用块就会停在 `awaiting_approval`。过程中确认了几件事，值得记下来：

| 发现 | 说明 |
| --- | --- |
| 只有 API Agent 会触发 | Web/mock 的模拟审批限定 `launch.kind === "api"`，即只有 OnePiece |
| 配置不能刷新页面 | OnePiece provider 存在模块内存里，`page.goto` 会清空，必须全程走客户端路由 |
| 审批区在折叠块内 | `<details>` 默认收起，展开才可见 |

**放弃的原因是无法确定性截图**。消息在等待审批期间始终处于流式状态，气泡宽度跟随最宽的兄弟块变化。依次尝试过：等最后一个工具块到达、用 CSS 固定块宽度、按工具名精确定位、过滤可见元素——最后两次运行仍稳定地相差 14867 像素，说明渲染是双模态的。

**提交一张会间歇性挂 CI 的截图，比没有截图更糟**，因此该场景已从清单与脚本中移除。若将来要补，方向是让 mock 提供一个「流式已结束但仍待审批」的确定状态。

## 桌面专属：**本轮已放弃，不配图**

以下界面依赖原生运行时，无法自动截取，需要人工拍摄并标为 `desktop-reviewed`。**本轮决定不为它们配图**，相关章节以文字说明代替。

| 场景 | 为什么自动化不了 | 当前处理 |
| --- | --- | --- |
| SSH 连通后的远程终端 | 需要真实 SSH 连接 | 文字说明，见 `remote-and-im.md` |
| 微信扫码授权二维码 | 需要真实凭据，且二维码是动态内容 | 文字说明 |
| CLI 冲突的真实检测结果 | 依赖本机实际安装状态，无法构造 | 文字说明，已有 Web/mock 的 CLI 页截图 |
| 权限审批的真实拦截 | 需要真实进程执行 | 文字说明，见 `permissions.md` |

**IM 已连接状态不在此列**——它在 Web/mock 中可达，已截取并明确标注为模拟态，见上表 `im-connected-zh-CN.png`。

### 将来若要补拍

规范对 `desktop-reviewed` 截图有硬要求：

- **不得包含**凭据、令牌、个人文件系统路径、未脱敏日志
- 需提供**本地化的替代文本**
- 需在 `screenshots.json` 中标 `runtime: "desktop-reviewed"`，并经人工审核
- **不得**把 Web/mock 截图冒充成原生行为的证据

由于人工截图不进 Playwright 回归，补拍后要额外注意：界面改版时它们不会被自动发现失效。
