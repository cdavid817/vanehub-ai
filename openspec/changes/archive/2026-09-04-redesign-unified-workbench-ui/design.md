# Design: Unified Workbench UI Redesign

## Context

VaneHub AI 的前端已经通过 React、Tauri/Web Adapter、语义 Token、i18n、Lazy Feature 和 OpenSpec 形成较好的工程边界，但界面架构仍以功能叠加为主。当前 `MainLayout` 直接编排活动栏、可调会话侧栏、会话九标签、信息面板以及多个隐藏挂载的业务中心；设置 Shell 同样保留访问过的页面挂载。结果是壳层承担过多领域细节，页面生命周期隐含在 JSX 中，响应式主要依靠隐藏与压缩。

本 Change 不重写领域模型，而是重构“用户如何进入、浏览、检查和操作这些领域对象”。设计必须同时满足：

- Web 与 Tauri 行为一致；
- React 仅通过 frontend service interface 访问能力；
- `minimal` 与 `futuristic` 两种主题功能等价；
- 全部可见文本走 i18n；
- 不新增 UI Library、全局状态管理框架或 inline style；
- 生产 TS/TSX 文件不超过仓库上限；
- 后台 Agent/Run 生命周期不依赖页面是否挂载；
- 已完成的 `improve-workspace-ui-ergonomics` 修复作为基线保留。

## Goals

1. 将一级导航从实现模块列表调整为任务域；
2. 让主工作面在常见桌面宽度保持可读；
3. 把低频配置和详情从默认界面移入渐进披露容器；
4. 建立统一页面骨架和状态语义；
5. 让 Mission Control、Evaluation、Schedule 等 UI 完整兑现已有领域规范；
6. 以局部 mutation、虚拟化、可见性暂停和有界查询控制性能；
7. 建立后续 UI 变更必须同步更新的测试与截图契约。

## Non-Goals

- 不合并 Rust 领域上下文；
- 不创建跨领域可写数据库表作为“统一对象模型”；
- 不在 Dashboard 复制权威编辑、Diff、Approval 或 Log 体验；
- 不改变 Scheduler 是否在应用关闭时运行；
- 不将 Feature Flag 保留为长期产品选项；
- 不强制一次提交完成所有文件迁移；
- 不用视觉重构掩盖缺少 Service Contract 的功能。

## Decision 1: 一级导航采用五个任务域

### Decision

定义稳定目的地：

```ts
export type WorkbenchDestination =
  | "sessions"
  | "projects"
  | "runs"
  | "plan"
  | "quality";
```

Settings 与 Help 属于全局工具，不占用业务目的地名额。二级路由负责保留领域独立性：

```ts
type RunsRoute =
  | { section: "attention" | "active" | "history"; runId?: string }
  | { section: "loops"; definitionId?: string; loopRunId?: string }
  | { section: "schedules"; scheduleId?: string };

type PlanRoute =
  | { section: "board"; viewId?: string; workItemId?: string }
  | { section: "goals"; goalId?: string };

type QualityRoute =
  | { section: "evaluations"; experimentId?: string; comparisonIds?: string[] };
```

`WorkspaceActivityBar` 只渲染五个业务入口；Loops、Schedules、Mission Control、Board、Goals、Evaluation 通过二级导航和 Command Center 进入。

### Rationale

用户通常从“当前会话、项目、运行、计划、质量”思考，而不是从内部表名或实现模块思考。归组只改变导航，不改变领域所有权。

### Alternatives rejected

- **保留所有图标，只优化 Tooltip**：不能解决入口数量、相似图标和高度不足；
- **使用可滚动活动栏**：隐藏了问题且降低发现性；
- **把所有能力放进 Command Center**：会损害首次可发现性；
- **把领域真正合并**：会破坏现有 Ports & Adapters 与 DDD 边界。

## Decision 2: AppShell 只编排框架，不直接编排领域细节

### Target structure

```text
App
└── AppShell
    ├── TopBar
    │   ├── RouteBreadcrumb
    │   ├── CommandCenterTrigger
    │   ├── GlobalAttentionSummary
    │   └── NotificationAndUtility
    ├── ActivityRail
    └── RouteOutlet
        └── DestinationLayout
            ├── ContextNavigation
            ├── WorkSurface
            ├── InspectorHost
            └── RuntimePanelHost
```

`AppShell` 接收 route registry 和命令 registry，不导入 Mission Control、Loop、Board 等领域 Service。每个目的地模块通过 Lazy Feature 注册：

```ts
export interface DestinationDefinition {
  id: WorkbenchDestination;
  path: string;
  icon: LucideIcon;
  labelKey: string;
  loader: LazyFeatureLoader<DestinationProps>;
  lifecycle: PageLifecyclePolicy;
  commandGroup: string;
}
```

### Consequences

- `main-layout.tsx` 从领域中心的总编排器拆成 shell + destination outlet；
- 领域模块仍可单独 lazy load；
- 深链首次进入不依赖“曾经点击过”的 visited flag；
- 页面的 mounted/suspended 状态由 registry 声明，而不是散落的 `useState<boolean>`。

## Decision 3: 采用“最多三块常驻 + 一个底部 Panel”

### Pane model

```ts
export interface WorkbenchPaneState {
  navigation: {
    open: boolean;
    width: number;
    presentation: "inline" | "sheet";
  };
  inspector: {
    open: boolean;
    width: number;
    presentation: "inline" | "sheet";
    mode: "overview" | "follow" | "pinned";
  };
  runtime: {
    open: boolean;
    height: number;
    activeTab: RuntimePanelTabId;
    maximized: boolean;
  };
}
```

宽度由 `ResizeObserver` 或容器查询决定：

- Wide：Navigation 与 Inspector 可同时 inline；
- Standard：Navigation inline，Inspector 默认 Sheet；
- Compact：Navigation 与 Inspector 均为互斥 Sheet；
- Narrow：单工作面，辅助区域全屏 Sheet。

主工作面设置最小可读宽度；Pane 算法优先折叠 Inspector，再折叠 Navigation，不允许把主区压到不可用。用户手动状态和自动响应状态分开保存，窗口重新变宽时恢复手动意图。

### Persistence

新增版本化、非敏感偏好：

```ts
interface WorkbenchLayoutPreferencesV2 {
  version: 2;
  destination: Partial<Record<WorkbenchDestination, {
    navigationWidth?: number;
    inspectorWidth?: number;
    runtimeHeight?: number;
    preferredInspectorOpen?: boolean;
    preferredRuntimeTab?: RuntimePanelTabId;
  }>>;
}
```

读取旧 `vanehub.session-sidebar.width.v1` 作为 Sessions 导航初值，成功迁移后写入 V2。坏值按边界 clamp，不阻塞 App 启动。

## Decision 4: Command Center 使用前端 Provider Registry，避免跨领域写耦合

### Interfaces

```ts
export interface WorkbenchSearchRequest {
  query: string;
  scopes: WorkbenchSearchScope[];
  limit: number;
  cursor?: string;
  routeContext?: WorkbenchRouteContext;
  signal: AbortSignal;
}

export interface WorkbenchSearchResult {
  key: string;
  kind: "session" | "project" | "run" | "goal" | "work-item" | "evaluation";
  title: string;
  subtitle?: string;
  status?: SemanticStatus;
  route: WorkbenchRoute;
  updatedAt?: string;
  keywords?: string[];
}

export interface WorkbenchSearchProvider {
  id: string;
  supports(scope: WorkbenchSearchScope): boolean;
  search(request: WorkbenchSearchRequest): Promise<WorkbenchSearchPage>;
}

export interface WorkbenchCommand {
  id: string;
  labelKey: string;
  keywords: string[];
  isAvailable(context: WorkbenchCommandContext): boolean;
  run(context: WorkbenchCommandContext): Promise<void> | void;
}
```

`CommandCenter` 并发调用注册 Provider，并对结果做有界合并。每个 Provider 仍调用所属领域 Service；不建立可写“全局搜索上下文”。请求可取消，旧查询不得覆盖新结果。初版可优先 Session/Project/Run，随后增加 Goal/Work Item/Evaluation。

### Privacy

只搜索本地安全摘要；查询不发送给模型 Provider。结果不得包含 Prompt、Response、Tool Input、Credential、未经裁剪的路径或原始错误。

## Decision 5: 路由是可分享的选择真相，瞬时浮层留在本地状态

### Route state

以下状态进入 URL：

- 目的地与二级 Section；
- 当前 Session/Project/Run/Goal/Work Item/Evaluation；
- Saved View id；
- 可复制的主要筛选与排序；
- Comparison experiment ids；
- 来自 EvidenceLink 的 scope id（有界且归属校验）。

以下状态不进入 URL：

- hover；
- Popover open；
- Sheet 拖动中；
- 临时 textarea；
- 未提交密码/Token；
- transient toast。

路由解析必须验证 stable id。当前对象不存在、已删除、无权限或不属于 route scope 时，显示明确状态并回退列表，不渲染空白页。

### Back navigation

从 Run 打开权威 Session/Review/Logs 时，携带 `returnTo` 内部 route token；返回恢复原筛选、滚动锚点和选中 Run。禁止用任意外部 URL 作为 return target。

## Decision 6: 页面生命周期显式声明

```ts
export type PageLifecyclePolicy = {
  keepAlive: "never" | "draft-only" | "always";
  suspendWhenHidden: boolean;
  refreshOnFocus: boolean;
  backgroundUpdates: "none" | "terminal-only" | "all";
};
```

### Rules

- 默认 `keepAlive: "never"`；
- 仅含未提交草稿且难以序列化的页面使用 `draft-only`；
- `always` 必须在代码注释和测试中说明原因；
- 隐藏页面不保留高频 polling、timer、ResizeObserver 或大 DOM；
- Query Cache、service-side run 与 UI mount 是不同概念；
- 正在运行的 Agent/Loop/Schedule 不因 UI 卸载被取消；
- 回到页面时执行 bounded reconciliation；
- terminal/attention 转换可通过全局低成本 event coordinator 更新摘要。

Settings 当前“访问过即永久隐藏挂载”改为 registry 策略；Mission Control/Evaluation 的 polling 根据 document visibility、route visibility 和 active entity 调整。

## Decision 7: 会话工作面从九标签改为四主面 + Runtime Panel

### Registry

```ts
export type SessionPrimarySurfaceId = "work" | "changes" | "files" | "report";
export type SessionRuntimeSurfaceId =
  | "terminal-history"
  | "shell"
  | "logs"
  | "traces";

export interface SessionSurfaceDefinition {
  id: SessionPrimarySurfaceId | SessionRuntimeSurfaceId;
  region: "primary" | "runtime";
  labelKey: string;
  scope: "session" | "seat-optional" | "seat-required";
  retention: "unmount" | "cache" | "keep-mounted-while-active-run";
  liveUpdates: "none" | "visible" | "background-terminal";
  badgeSource?: WorkspaceEvidenceBadgeSource;
}
```

### Mapping

| Current tab | Target |
|---|---|
| `chat` | `work`：API/多 Agent 渲染 Chat；单 Agent CLI 渲染现有 Agent Terminal |
| `changes` | `changes` |
| `documents` | `files` 的 Documents view/filter |
| `files` | `files` 的 Explorer view |
| `terminal` | Runtime Panel `terminal-history` |
| `shell` | Runtime Panel `shell` |
| `logs` | Runtime Panel `logs` |
| `traces` | Runtime Panel `traces` |
| `report` | `report` |

Slash 命令、EvidenceLink 和 Mission Control 导航继续接受旧 tab id，在 route adapter 中转换为新 Surface + Subview。实现完成后内部新代码只使用新 id；旧 id 仅保留在兼容解析层。

### Mount behavior

主面一次只挂载活动内容，需保留状态的视图使用 Query Cache 或显式 draft store。Runtime Panel 只挂载打开过且符合 retention 的 tab；每个 tab 收到 `isVisible`，但不能仅依赖 CSS `display:none` 假装暂停。

## Decision 8: Inspector 由选择模型驱动

### Selection contract

```ts
export type WorkbenchSelection =
  | { kind: "session"; sessionId: string }
  | { kind: "message"; sessionId: string; messageId: string }
  | { kind: "tool"; sessionId: string; messageId: string; toolCallId: string }
  | { kind: "file"; sessionId?: string; projectId?: string; pathId: string }
  | { kind: "change"; sessionId: string; changeId: string; pathId?: string }
  | { kind: "run"; runId: string }
  | { kind: "loop-iteration"; loopRunId: string; iterationId: string }
  | { kind: "evaluation-result"; experimentId: string; resultId: string };
```

路径使用安全 `pathId` 或经过所属 Service 验证的相对路径，不把任意文件系统路径作为跨组件命令。

### Inspector behavior

- 无具体选择：`overview`；
- 主区选择：`follow`，自动更新；
- 用户 Pin：`pinned`，后续选择不替换；
- 对象删除或 scope 变化：显示 unavailable 并允许返回 overview；
- 详情 Provider 按 kind lazy load；
- 每个 Provider 明确 available/unavailable/restricted/error；
- EvidenceLink 跳往权威页面，不在 Inspector 复制编辑器、Diff 或完整日志。

### Information panel migration

Basic、Members、Usage、Skills、IM、Code Index 等不再横排为同级小标签。Session Overview 使用 Section/Accordion；低频复杂管理跳往 Settings/Projects。保留当前查询和操作能力，但重新组织，不静默删除。

## Decision 9: Composer 使用“摘要 + Run Configuration”

### Default surface

常驻：

- textarea；
- attachment/media；
- context chips；
- 当前 `Agent · Model` 摘要；
- Send/Stop。

高级配置 Popover：

```text
Agent & runner
Provider & model
Reasoning / thinking
Permission policy
Profile / per-message override
Advanced execution
```

字段显示 effective value、来源（message override/profile/default）、是否仅本次，并提供 Reset。本次 override 不自动写回 Profile。高风险权限值在关闭 Popover 后仍以 warning summary 可见。

### Error model

- 字段验证错误紧贴字段；
- Send 失败保留草稿或已持久化用户消息；
- Stop/Recovery 错误贴近状态操作；
- Toast 只做补充；
- 发送期间不锁定阅读、复制、Inspector 和 Runtime Panel。

## Decision 10: 消息窗口化必须保留锚点语义

直接替换成固定高度虚拟列表不可接受，因为 Markdown、Tool、Rich Block 高度动态，流式消息持续变化。

实现分两步：

1. 抽象 `ConversationWindowModel`，先保留现有 DOM，实现稳定 key、锚点、测量和可测试滚动策略；
2. 接入 `@tanstack/react-virtual` 的动态测量，使用 bottom anchor：
   - near-bottom 时跟随最后消息；
   - 阅读历史时保持先前 bottom offset；
   - prepend 历史时保持第一个可见 message + intra-item offset；
   - streaming item 改高只在 near-bottom 时滚动；
   - 选中/聚焦虚拟项时保证可见和可恢复。

Tool-heavy message 内部也应窗口化或延迟渲染昂贵 Rich Block。虚拟化不能改变消息顺序、speaker seat、状态、审批或 compaction card 语义。

## Decision 11: 管理页面采用共享 Collection/MasterDetail 模型

### Shell variants

```ts
type ManagementPageLayout =
  | { kind: "collection"; list: ReactNode; inspector?: ReactNode }
  | { kind: "master-detail"; master: ReactNode; detail: ReactNode }
  | { kind: "board"; board: ReactNode; detail?: ReactNode };
```

统一 Page Header：

- title；
- bounded summary；
- one primary action；
- More。

统一 Toolbar：

- search；
- filter trigger；
- active filters；
- saved view；
- view/sort；
- batch mode。

Create/Edit 使用 Sheet 或 Dialog，不在 Header 内联展开。详情路由化；窄屏 detail 替换 list 并有明确返回。

### Async model

```ts
interface AsyncViewState<T> {
  data?: T;
  initialLoading: boolean;
  refreshing: boolean;
  error?: DisplayableError;
  stale: boolean;
}

interface MutationState {
  targetKey: string;
  operationId?: string;
  pending: boolean;
  error?: DisplayableError;
}
```

mutation 只禁用目标动作，保留其他内容。Optimistic update 仅用于可安全回滚操作；Cancel/Approval 等竞态敏感操作等待 canonical response，但仍不锁整页。

## Decision 12: Board 与 Goal 共用选择、筛选和对象 Picker 基础

### Board

- Board query 由 Saved View + filters + sort 组成；
- 卡片拖拽提交一次 stage mutation；
- 非拖拽路径使用 “Move to…” Menu/Listbox；
- prev/next 和 stage select 不再同时常驻；
- 失败回滚卡片并在卡片位置显示错误；
- Mobile 使用 grouped list；
- WIP limit 是可选呈现，不改变后端状态机；
- 多选操作必须先确认目标集合。

### Goal

新增 `ExecutionTargetPicker`，Provider 分别搜索 Session、Run、Loop、Work Item，返回安全摘要和 stable id。UI 不允许普通用户手输 raw id；诊断/开发模式可通过受控高级入口粘贴并验证。

Goal 完成仍需现有手工验收语义。关联图是投影视图，不写入第二套关系。

## Decision 13: Mission Control 是 Runs 首页，不是另一个执行器

### Information architecture

```text
Runs
├── Attention
├── Active
├── History
├── Loops
└── Schedules
```

Mission Control 保留 canonical Run 聚合与权威导航。详情按需加载九类 evidence，但导航改为可读的 Section Navigation；宽度不足时使用 Select，不横向压缩九个 tab。

`lazyDetail` 等通用占位必须删除。统一 Facet 状态：

```ts
type EvidenceAvailability =
  | { kind: "available"; value: EvidencePage }
  | { kind: "unavailable"; reasonCode: string }
  | { kind: "restricted"; permission: string }
  | { kind: "loading" }
  | { kind: "error"; error: DisplayableError };
```

只有 `available` 才渲染领域内容。Run action 根据 canonical state、owner capability、permission 和 version witness 计算，不凭 UI 文字猜测。

## Decision 14: Loop 主区围绕阶段与决策，不围绕三栏常驻

Definitions 与 Runs 通过二级 route 分开。Run 页面：

```text
RunHeader(status, phase, primary action, more)
PhaseStepper
DecisionPanel (only when action required)
IterationTimeline
SelectedIterationSummary
Inspector (on demand)
```

Iteration 行默认显示目标、结果、状态、时长、Token/预算摘要和关键验证。完整 Prompt/工具/产物/日志按证据链接加载，不把所有内容塞入 Accordion。

Accept/Continue/Reject 前显示后果。决定提交后仅锁 Decision Panel，保留时间线阅读；冲突时刷新 canonical state 并解释。

## Decision 15: Evaluation 围绕 Experiment 与 Comparison

### Page model

```text
EvaluationHome
├── ExperimentList
├── ExperimentDetail
│   ├── Summary
│   ├── ResultTable
│   └── ResultInspector
└── Comparison
    ├── Baseline selector
    ├── 2-4 experiment columns
    ├── metric deltas
    └── regression drill-down
```

结果不是裸 PASS/FAIL。显示：

- outcome icon + label；
- evaluator/check；
- threshold/expected；
- measured value；
- bounded reason；
- evidence/artifact links；
- baseline delta。

表格使用服务分页或虚拟化；列配置保存为本地非敏感偏好。活动实验事件驱动，轮询作为缺失事件恢复，窗口失焦/路由隐藏时退避。

## Decision 16: Scheduled Tasks 升级为正式页面

`ScheduledTasksDialog` 不再承担完整 CRUD。`/runs/schedules` 使用列表/详情/Editor Sheet。需要补齐的最小 service contract：

```ts
interface ScheduledTaskService {
  listScheduledTasks(query: ScheduledTaskQuery): Promise<ScheduledTaskPage>;
  getScheduledTask(id: string): Promise<ScheduledTaskDetail>;
  createScheduledTask(input: CreateScheduledTaskInput): Promise<ScheduledTask>;
  updateScheduledTask(id: string, version: string, input: UpdateScheduledTaskInput): Promise<ScheduledTask>;
  duplicateScheduledTask(id: string): Promise<ScheduledTaskDraft>;
  setScheduledTaskEnabled(id: string, version: string, enabled: boolean): Promise<ScheduledTask>;
  runScheduledTaskNow(id: string, version: string): Promise<OperationRef>;
  deleteScheduledTask(id: string, version: string): Promise<void>;
  listScheduledTaskRuns(id: string, query: PageQuery): Promise<ScheduledTaskRunPage>;
  previewScheduledTaskOccurrences(input: SchedulePreviewInput): Promise<OccurrencePreview>;
}
```

若现有服务已有同等操作，复用并适配命名，不重复实现。Preview 可在前端使用同一 recurrence library 或通过 Service 计算，但 Web/mock 与 Tauri 必须返回同形状且时区语义一致。

页面必须持续显示当前限制：调度器依赖应用运行，启动只处理允许的 catch-up。该提示不是 Error，而是产品能力说明。

## Decision 17: Settings 使用元数据 Registry 支撑导航、搜索和生命周期

```ts
interface SettingsPageDefinition {
  id: SettingsPageId;
  category: SettingsCategoryId;
  labelKey: string;
  descriptionKey: string;
  icon: LucideIcon;
  keywords: string[];
  fields: SettingsSearchField[];
  loader: LazyFeatureLoader<SettingsPageProps>;
  saveMode: "immediate" | "draft" | "mixed";
  lifecycle: PageLifecyclePolicy;
  risk: "normal" | "sensitive" | "dangerous";
}
```

搜索索引来自 Registry 和页面导出的静态字段元数据，不通过挂载所有页面抓 DOM。结果定位 `/settings/:page#field-id`，页面加载后聚焦/高亮，尊重 reduced motion。

保存模式：

- immediate：每行 mutation + 回滚；
- draft：`DraftActionBar`；
- mixed：页面显式划分；
- dangerous：独立 Danger Zone。

离开保护由 Settings Shell 统一协调，不允许每页各自注册浏览器对话框。Secret 不进入搜索摘要、URL 或诊断复制。

## Decision 18: Projects & Workspaces 是现有真相的聚合入口

Project 页面只读聚合现有 Project History、Git inspection、Worktree、SSH Workspace、Session 与 Run 安全摘要。写操作仍交给对应服务。

```ts
interface WorkspaceSummary {
  workspaceId: string;
  kind: "local" | "ssh";
  displayName: string;
  displayPath?: string;
  projectId?: string;
  git?: {
    repository: boolean;
    branch?: string;
    dirty?: boolean;
    worktree?: string;
  };
  trust: "trusted" | "untrusted" | "unknown" | "revoked";
  recentSession?: SafeSessionSummary;
  activeRuns: number;
  lastOpenedAt?: string;
  availability: "available" | "missing" | "disconnected";
}
```

远程信任必须持续可见；断开或路径失效不被当作“空项目”。Create Session 从 Project 进入时预填 workspace id，但用户仍在 Review 步骤确认。

## Decision 19: 双主题共享层级，只在表达上不同

`futuristic` 可以有更深背景和有限光感，`minimal` 更平坦；两者必须共享：

- 组件结构；
- spacing/density；
- focus ring；
- status semantics；
- disabled/loading/error；
- responsive arrangement；
- target size。

禁止为主题建立两套 JSX 或改变字段顺序。设计 Token 分为 semantic layer 与 theme value layer。所有新组件只消费 semantic token。

## Decision 20: 测试策略是组件契约，而不是实现结束后的截图补丁

### Test layers

1. **Pure/unit**：route parser、registry、filter model、pane algorithm、lifecycle policy、status mapping；
2. **Component**：keyboard model、focus return、AsyncBoundary、row mutation、Inspector Provider；
3. **Service contract**：Tauri/Web mock shape、Unavailable/Restricted、version witness；
4. **Playwright Web**：完整主流程、断点、键盘、主题、i18n；
5. **WebdriverIO/Tauri**：xterm、窗口 resize、native dialog、filesystem/SSH、安全边界；
6. **Structural performance**：DOM/query/subscription/update count；
7. **Visual regression**：核心矩阵，而不是每个随机状态。

### Required screenshot matrix

```text
themes: minimal, futuristic
locales: zh-CN, en, ja
widths: 1600, 1280, 1024, 768, 640
surfaces:
  sessions/default
  sessions/runtime-panel
  sessions/inspector
  runs/attention
  runs/detail
  loops/action-required
  plan/board
  quality/comparison
  schedules/editor
  settings/search-result
```

Tauri 至少在 Windows、macOS、Linux 真实执行 smoke；未执行的平台不得标记通过。

## Data and migration

### No-authority migration

UI route、pane、saved view 和列偏好不是领域真相。任何新持久化必须版本化、可删除、可重建。建议 storage keys：

```text
vanehub.workbench.layout.v2
vanehub.workbench.saved-views.v1
vanehub.workbench.table-columns.v1
vanehub.settings.drafts.v1      // only non-secret and explicitly allowed
```

跨设备同步不在本 Change 内。含 secret、prompt、tool input、raw log、external identity 的值不得进入这些记录。

### Route compatibility

旧 route adapter 至少保留一个稳定版本周期。内部导航立即使用新 route。兼容层记录匿名计数或开发日志，以便确认旧 route 是否仍被测试/使用；不得包含用户内容。

## Rollout

### Phase 0: Baseline

- rebase 已完成 ergonomic change；
- 记录现有 route/service/test/locale/visual baseline；
- 建立 Feature Flag 和 V1/V2 可比较入口；
- 不在没有真实截图时删除控件。

### Phase 1: Foundations

- tokens/primitives；
- destination registry；
- pane algorithm；
- command center skeleton；
- route compatibility；
- lifecycle coordinator。

### Phase 2: Sessions

- list model and virtualization；
- surface registry；
- runtime panel；
- inspector；
- composer；
- message selection/windowing；
- create wizard；
- old tab adapter。

### Phase 3: Runs and planning

- Mission Control facets；
- Loop redesign；
- Schedule page/contracts；
- Board/Goal shared shell and mutations。

### Phase 4: Quality, settings, projects

- Evaluation；
- Settings registry/search/save；
- Projects & Workspaces。

### Phase 5: Stabilization

- performance/a11y/visual/native；
- migration telemetry/debug evidence；
- remove V1；
- strict OpenSpec validation and archive.

Feature Flag 只在迁移期存在。V1 删除前必须确认旧 route 兼容、所有 P0 和 required scenario 通过。

## Risks and mitigations

### Risk: Message virtualization breaks streaming scroll

Mitigation: first extract and test anchor model, then virtualize; keep a rollback switch for the virtualization implementation only, not the entire V2 shell.

### Risk: Unmounting hidden pages loses drafts

Mitigation: page registry declares `draft-only`; Settings Shell serializes permitted non-secret draft or blocks navigation. Tests distinguish UI state from backend run state.

### Risk: New IA hides familiar entry points

Mitigation: old routes redirect; Command Center indexes old keywords; first-run “Moved to” hints appear once; Activity Rail tooltips show sub-capabilities.

### Risk: Inspector duplicates authority

Mitigation: every detail provider marks read-only summary vs editable authority and uses `EvidenceLink` to navigate; no duplicated diff/editor/approval implementation.

### Risk: Change too large to review

Mitigation: milestone commits, shared primitives first, one domain per PR stack, no “big bang” file move, and an acceptance matrix mapping every requirement to tests.

## Open Questions resolved by this design

- **Does “Workspace” disappear?** No. It becomes the stable primary `work` surface: CLI sessions render Agent Terminal, API/multi-Agent sessions render conversation.
- **Are Logs/Traces removed?** No. They move to Runtime Panel and remain reachable through old slash/deep-link adapters.
- **Does Runs merge domains?** No. It groups navigation; Mission Control, Loop and Schedule retain their services and truth.
- **Does UI unmount cancel Agents?** No. Business execution remains service-owned.
- **Is a new UI library required?** No. Build on Tailwind, semantic tokens, Lucide, CVA/Radix primitives already present.
- **Are current settings pages deleted?** No. Registry navigation and search are reorganized; page removal requires a separate domain decision.
