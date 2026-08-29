# Change: Redesign unified workbench UI

## Why

VaneHub AI 已经形成会话、项目、Agent CLI、Loop、Run、任务、目标、评测、定时任务和设置等完整能力，但当前前端仍以“逐项增加入口和控件”的方式增长。主窗口同时存在活动栏、会话列表、会话 Header、九个工作区标签、Seat 切换、Turn Status、右侧多标签信息面板和页面内部工具条；管理中心也普遍把创建、筛选、指标、详情和危险动作放在同一视觉层级。

静态审计共记录 137 个问题，其中 24 个为 P0。最重要的不是配色，而是：

- 一级导航按实现模块平铺，而不是按用户工作流分区；
- 九个会话工作区标签和右侧多标签在常见宽度下溢出或被隐藏；
- Composer、Board、Mission Control、Settings 等默认暴露过多低频控件；
- Mission Control 多个详情 Facet 仍是占位内容；
- Scheduled Tasks 作为大 Dialog 承载完整管理流程，缺少编辑、复制、立即运行和完整历史入口；
- Board/Goal 等单项 mutation 使用全局 busy/重载，打断无关工作；
- 长会话列表与消息历史缺少统一虚拟化策略；
- Settings 仅有页面级/当前页搜索，保存和 keep-alive 语义不一致；
- 复杂控件的键盘模型、响应式重排和状态表达不统一；
- 每个中心重复实现 Header、筛选、详情、空状态和异步反馈。

如果继续按模块局部修补，导航层级、组件重复和隐藏页面资源占用会继续累积。本 Change 以一次可分批合入的大重构建立统一信息架构、工作台壳层、设计系统、上下文 Inspector、Runtime Panel 和管理页面范式。

## What Changes

### 1. 统一一级信息架构

将业务目的地收敛为五个任务域：

- Sessions；
- Projects & Workspaces；
- Runs；
- Plan；
- Quality。

Settings 与 Help 固定在工具区。现有 Loops、Mission Control、Scheduled Tasks、Board、Goals、Evaluation 保留领域能力和独立深链，但分别归入 Runs、Plan、Quality。旧 URL 必须重定向并保留当前对象、筛选和返回上下文。

### 2. 重构工作台壳层

建立 `AppShell`、`DestinationLayout`、`SplitPane`、`Inspector`、`RuntimePanel` 和 `CommandMenu`。宽屏最多三个永久并列区域：上下文导航、主工作面、Inspector。窄屏将上下文导航和 Inspector 转为可访问 Sheet，而不是继续压缩主内容。

顶部搜索升级为 `Ctrl/Cmd+K` Command Center，可搜索和跳转 Session、Project、Run、Goal、Work Item、Evaluation，并执行受权限和状态约束的常用命令。

### 3. 重构会话体验

- 会话列表改为注意力优先的分组、虚拟化列表、统一筛选和批量模式；
- 九个平级标签收敛为四个主工作面，并将 Shell/Logs/Traces 等运行工具放入 Runtime Panel；
- `Documents` 与 `Files` 合并为统一文件工作面；
- Composer 默认只保留输入、附件、当前运行配置摘要和 Send/Stop，高级设置进入 `Run Configuration`；
- Message/Tool/Rich Block/File/Change 可被选中并由 Inspector 展示专属证据；
- Seat 导航使用与 Tab 一致的键盘模型；
- 创建会话改为四步 Wizard + Review；
- 保留现有流式消息、滚动锚点、恢复、安全、Service Boundary 和多 Agent 语义。

### 4. 将固定信息面板升级为上下文 Inspector

Inspector 支持 Session Overview、Follow Selection 和 Pinned 三种模式。未选对象时按 Section 显示参与者、Runtime、Usage、Skills、Workspace、IM；选中对象时显示该 Message、Tool、File、Change、Span 或 Run 的详情和权威证据跳转。Inspector 在 Standard/Compact 宽度下以 Sheet 呈现并保留等价入口。

### 5. 统一管理页面范式

建立共享 `PageHeader`、`Toolbar`、`FilterBar`、`DataTable`、`EntityList`、`StatusBadge`、`AsyncBoundary`、`MutationStatus`、`FormSection`、`DraftActionBar`、`ActionMenu` 和 `EvidenceLink`。

- Header 只保留身份、摘要、一个主动作和 More；
- 搜索/筛选使用统一查询模型并支持 Active Filter Chips/Saved Views；
- 创建/编辑使用 Sheet/Dialog，不在 Header 展开；
- 单项 mutation 使用局部 pending、optimistic update 和失败回滚；
- 初次加载、刷新、空数据、筛选无结果、Unavailable、Restricted、Error 明确区分；
- 状态不能只靠颜色。

### 6. 模块级重构

- **Settings**：字段级全局搜索、分类导航、统一即时保存/草稿保存/Danger Zone、声明式页面生命周期；
- **Projects & Workspaces**：增加一级管理页，统一本地/远程、Git、Worktree、信任、最近会话和运行；
- **Board**：Saved Views、Sheet 编辑、局部 mutation、统一阶段移动、键盘/触控等价路径、移动端 Stage List；
- **Goals**：对象 Picker 代替原始 target id、统一 MasterDetail、局部 mutation、关联执行图；
- **Mission Control**：Attention-first、真实按需详情、移除占位 Facet、状态相关动作、可见性驱动刷新；
- **Loops**：Definitions/Runs 分离、两栏主布局、上下文 Inspector、Decision Panel 和紧凑 Iteration Timeline；
- **Evaluation**：实验向导、DataTable、Baseline、回归差值、2～4 实验对比、可打开 Artifact；
- **Scheduled Tasks**：从 Dialog 迁移到 `/runs/schedules` 页面，补齐 Edit/Duplicate/Run now/History/Timezone/Future preview；
- **Top Bar**：目的地路径、Command Center、待处理摘要、通知/设置，移除同权次要动作。

### 7. 性能、可访问性和视觉回归门禁

- 为 Session/Message/Run/Work Item/Evaluation 大数据 Fixture 建立结构预算；
- 隐藏页面停止不必要的轮询、计时器和 Observer；
- Tab、Toolbar、Listbox、Menu、Dialog、Sheet、Drag Alternative 使用各自正确键盘模型；
- `minimal` 与 `futuristic` 双主题在 zh-CN/en/ja 和 Web/Tauri 关键尺寸下具备截图基线；
- 任何用户可见字符串进入同步 locale 资源；
- 不引入新的 UI 框架、全局状态管理器或内联样式逃生口。

## Capabilities

### New Capabilities

- `workbench-design-system-ui`: 定义统一工作台信息架构、壳层、共享 Primitive、响应式组合、页面生命周期、异步反馈、可访问性和视觉回归要求。

### Modified Capabilities

- `main-layout-ui`: 重构一级导航、三面板组合、会话列表、主工作面、Runtime Panel、上下文 Inspector、创建会话和路由兼容。
- `chat-experience`: 增加 Composer 渐进披露、可选择证据、消息历史窗口化、统一状态层级和 Seat 键盘语义。
- `settings-center-ui`: 增加字段级全局搜索、分组导航、保存语义、离开保护和声明式 keep-alive。
- `project-worktree-management`: 增加项目与工作区一级管理页、持续信任状态和项目上下文操作。
- `unified-todo-board`: 增加 Saved Views、局部 mutation、Sheet 编辑、多选和完整非拖拽阶段移动。
- `goal-management`: 增加目标 Picker、MasterDetail、局部 mutation 和关联执行体可视化。
- `agent-mission-control`: 补齐真实 Run 详情，归入运行中心，调整导航、刷新和注意力工作流。
- `loop-management-ui`: 调整 Definitions/Runs 信息架构、Run 监控布局、Iteration Inspector 和验收决策面板。
- `agent-evaluation`: 增加实验配置、Baseline、差值、对比、结果解释和大表格交互。
- `scheduled-task-management`: 将管理入口路由化并补齐编辑、复制、立即执行、时区预览和运行历史。

## Impact

### Frontend

主要影响：

- `src/main-layout/`
- `src/session-workspace/`
- `src/components/chat/`
- `src/settings/`
- `src/work-board/`
- `src/goal-center/`
- `src/mission-control/`
- `src/loop-center/`
- `src/evaluation-center/`
- `src/styles.css`
- `src/services/`
- locale resources
- Web fixtures、Playwright、WebdriverIO 和组件测试

新增共享前端目录建议为 `src/ui/`，领域组合迁移到 `src/features/` 可分批完成，不要求一次移动全部文件。

### Service contracts and native runtime

本 Change 以复用现有 Service Boundary 为原则。只有 UI 要求当前 Service 无法支持时才添加最小契约，例如：

- 跨对象 Command Center 的有界搜索；
- Scheduled Task update/duplicate/run-now/history/future-occurrence preview；
- Settings 搜索元数据和页面生命周期不需要 native API；
- Evaluation compare/baseline 若当前 DTO 不足，使用添加字段和新 query，不重写权威评测模型；
- Project/Workspace 概览从现有项目历史、SSH、Session、Run 聚合安全摘要，不建立第二真相源。

所有 Tauri 与 Web/mock Adapter 必须同步；React 不得直接 `invoke()`。

### Data migration

优先无数据迁移。新增 Saved Views、Pane Preferences 或 Schedule metadata 时：

- 使用 additive migration；
- 版本化 JSON/SQLite 记录；
- 提供默认值和回退；
- 不改写 Session、Run、Loop、Goal、Work Item、Evaluation 的权威记录；
- 旧 URL、本地 sidebar width 和已保存设置尽可能迁移或兼容读取。

### Existing change relationship

`improve-workspace-ui-ergonomics` 已覆盖侧栏分隔、创建会话分区、Help、Board/Goal 局部视觉、通知位置和 Session Recovery。本 Change SHALL 以该 Change 为前置基线：

- 不回退 `normalizeDisplayPath()`；
- 不恢复被修复的侧栏覆盖；
- 不删除 Help 文档页；
- 不合并 runtime recovery 与 evidence recovery；
- 不恢复全局 mutation busy；
- 对已有组件进行演进或迁移，而不是并行复制。

## Goals

- 在不删除领域能力的前提下显著减少永久导航和默认可见控件；
- 让核心工作流在 1024px 及以上桌面宽度可完成，并在 640～1024px 使用任务式单面板组合；
- 让 Run、Loop、Evaluation、Schedule 的详情从“有入口”升级为“可决策、可追踪、可恢复”；
- 统一跨模块视觉层级、异步状态、键盘交互和路由状态；
- 保持双主题、国际化、Web/Tauri 契约和后台执行语义；
- 让后续新增/删除/修改 UI 时有可复用骨架和强制测试清单。

## Non-Goals

- 不重写 Rust DDD 上下文或将多个领域合并为同一数据库模型；
- 不在 Mission Control 中复制 Chat、Diff、Approval、Editor 或 Logs 的权威实现；
- 不改变 Scheduled Task“应用打开时执行”的运行语义；后台守护调度需另立 Change；
- 不引入第三方 UI 组件库、Redux 类状态管理器或 CSS-in-JS；
- 不一次性改变所有品牌图标、插图和营销页面；
- 不用大规模动效掩盖状态切换；
- 不以 Feature Flag 永久维护 V1/V2 双实现；
- 不在静态审计阶段删除无法确认的能力。

## Risks

- 大范围路由和 Shell 变化可能破坏深链、返回上下文和已挂载状态；
- 消息虚拟化若处理不当会破坏流式追加和历史锚点；
- 隐藏页面暂停策略若与后台运行混淆，可能错误停止业务执行；
- Scheduled Task 与 Evaluation 需要少量新 Service Query，可能扩大 Change；
- 两套主题和多语言截图矩阵显著增加测试量；
- 旧组件迁移期间可能出现两套 Toolbar/Status 表达。

这些风险通过分批提交、兼容路由、明确页面生命周期、契约测试、结构性能 Fixture 和 Feature Flag 迁移窗口控制。

## Delivery Strategy

Change 分为六个可独立验证的里程碑：

1. Baseline and contracts；
2. Shared design system and shell；
3. Session experience；
4. Runs and planning；
5. Quality, settings, projects；
6. Stabilization and removal of legacy UI。

每个里程碑必须在进入下一阶段前通过对应组件测试、Playwright、Web/mock 契约和截图门禁；不允许把 137 项问题在最后一次性验收。
