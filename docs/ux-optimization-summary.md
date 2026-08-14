# VaneHub AI 用户体验优化实施总结

- 实施日期：2026-08-14
- 分支：`worktree-ui-ux-optimization`，基线提交 `3164bdc8`
- 对应审计：`docs/ux-audit-report.md`
- 范围：审计中判定为 A 类（样式 / 文案 / 微交互）的条目直接实现；B 类改为 OpenSpec change package，尚未实现

---

## 1. 已实现的改动

### P0-1 统一会话生命周期文案

同一个 `lifecycleState` 过去在会话列表和信息面板走两套并行文案，`failed` 在列表里显示"需要输入"、在面板里显示"失败"。一个已经失败的会话被标成"需要输入"，会让用户去等一个永远不来的提示符。

- 新增 `src/lib/session-lifecycle.ts`，把状态到文案键、状态到色调的映射收敛到一处
- `src/main-layout/session-sidebar.tsx` 改用统一映射，删除本地重复的 label map
- 状态圆点不再恒为绿色，改为按状态取色（运行=成功色，启动中=警告色，失败=危险色，空闲/已停止=中性色）
- 从五个 locale 删除已失去引用的 `layout.idle`、`layout.running`、`layout.stopped`、`layout.needsInput`、`layout.pendingVerification`、`layout.ready`
- 新增 `src/lib/session-lifecycle.test.ts`，其中一条用例专门守住"失败不得与健康态共用色调"

改动后同一会话在会话列表、工具栏、信息面板、会话头部四处显示一致。

### P0-2 会话头部状态药丸读取真实生命周期

- `src/session-workspace/session-conversation-header.tsx` 原本只判断 `isStreaming`，会话 `stopped`/`failed` 时照样显示绿色"就绪"，未选中会话时也显示
- 改为：流式状态是生命周期之上的叠加态而非替代态；无会话时不渲染药丸
- 色调改用 `src/styles.css` 已有的 `.ucd-status-success/warning/danger` 共享工具类，而不是自己拼颜色

### P0-3 顶栏搜索框

顶栏的 `<input>` 没有 `value`、`onChange`、`onKeyDown`，打字和回车都没有任何反应。

**这一项的处置方式与审计初稿的建议不同，原因是它受既有规范约束。** `main-layout-ui` 规范写着 "search SHALL NOT be removed from the top bar without a replacement control"，所以直接删除入口并不合规。同一条规范允许"等效的图标触发控件"，因此改为：顶栏搜索按钮现在展开会话侧栏并把焦点交给那个**已经能用**的会话搜索框。

这样既消除了假控件，又没有新增任何能力，也不需要改规范。`layout.searchPlaceholder`、`layout.closeSearch` 随第二个输入框一起删除，`layout.openSearch` 保留原文案「打开搜索」。

实现过程中还牵出一个与本条相关的缺陷：侧栏的 `inert` 原本由 effect 设置，而 React 的子组件 effect 先于父组件运行，导致"展开侧栏 + 聚焦搜索框"在同一次点击里发生时，聚焦打在仍处于 inert 的子树上而被浏览器拒绝。改为 React 19 的声明式 `inert` prop（commit 阶段生效，早于任何 effect）后解决。

### P1-2 活动栏「帮助」按钮

原本没有 `onClick`。现在打开「关于」设置页——保留入口且不引入新目的地，符合规范里 "keep the Help entry available without introducing a new Help destination" 的表述。`workspace-activity-bar.test.tsx` 补了回调断言。

### P1-3 设置侧栏「导出当前配置」按钮

原本没有 `onClick`。实现配置导出属于新增业务功能，超出本轮约束，因此移除入口并把能力缺口记入建议清单。`app.settings.export` 从五个 locale 删除。

### P1-5 顶栏硬编码假会话号

`#SID-20260714` 与实际会话无关、切换会话也不变。已移除。

### P1-6 版本号硬编码与漂移

这一条比审计时判断的更严重：硬编码在**两处**，且都已经漂移。`src/services/about-service.ts` 和设置侧栏都写死 `0.1.0`，而 `package.json` 已经是 `0.1.0-preview.1`——所以「关于」页显示的版本本身就是错的。`scripts/check-version-sync.mjs` 只覆盖 package.json、Cargo.toml、tauri.conf.json，管不到前端。

改为构建期注入：`vite.config.ts` 从 package.json 读版本并通过 `define` 注入 `__APP_VERSION__`，`about-service.ts` 消费它。漂移在结构上不再可能发生。侧栏那行来源不明的 `Build 2026-07-14` 一并删除。

### P1-7 会话副标题渲染原始枚举

`session.interactionMode` 被直接渲染成裸的 `cli`。新增 `session.interactionMode.*` 四个键（browser / native-desktop / cli / api）覆盖五个 locale。

### P1-8 创建会话失败信息被截断

`truncate` 会把"路径不存在""SSH 连接失败"这类较长错误截掉，而那正是唯一可操作的部分。改为允许换行、加 `role="alert"`。

### P2-1 设置侧栏分组

17 项平铺改为 5 组：常规 / Agent / 能力与扩展 / 集成与连接 / 诊断与关于。

**顺序未动。** `tests/e2e/settings-navigation-order.spec.ts` 断言了这 17 项的相对顺序，测试标题写明该顺序是围绕常见配置流程刻意编排的。因此只做**保持原顺序的连续分组**——重排会推翻一个已固化的产品决策，那不该由本轮顺手决定。代价是"个性化"落在「能力与扩展」组里，语义上略勉强。

### P2-2 术语统一

同一页有三个名字：侧栏「基础配置」、面包屑「基础配置」、正文「通用设置」。五个 locale 的 `basic.title` 已对齐到 `settings.pages.basic`。

### P2-5 Toast 遮挡

Toast 原本在右下角，压住输入区的发送按钮。**中途改过一次方向**：先移到右上角，重新截图后发现它改为压住信息面板的页签栏——等于把遮挡从一个控件挪到另一个控件。最终定在左下角，两个控件都避开，进出动画方向同步改为从左侧滑入。

这一条说明视觉验证不是走过场：仅看代码无法发现"修好一个遮挡、制造另一个遮挡"。

### P2-6 空态区分「筛选无结果」与「无数据」

- `src/work-board/work-board.tsx`：新增 `filtersActive` 判断，空列显示 `todoBoard.emptyFiltered` 或 `todoBoard.empty`
- `src/main-layout/session-sidebar.tsx`：Agent 筛选命中零条但底层有会话时，显示 `layout.noSessionsForFilter`

### P2-7 信息面板空态

无会话时五个字段各自渲染一遍"未选择会话"。改为单个「图标 + 标题 + 说明」空态。

### P2-8 删除死代码

- `src/main-layout/conversation-sidebar.tsx`（100 行）、`flow-canvas.tsx`（145 行）、`info-panel.tsx`（99 行）：零引用
- `main-layout.tsx` 导出的 `ConversationCard` 及其测试文件：只被自己的测试渲染，主布局从未挂载

审计报告里"`info-panel.tsx` 仅被自身测试引用"的说法不准确——那条 grep 命中的是 `"./session-info-panel.tsx"` 和 `"feature/info-panel"` 里的子串，实际零引用。报告已更正。

合计删除 385 行死代码。

### P2-13 Agent 不可用原因

`unavailableReason` 是后端诊断文本（Web/mock 为 `OnePiece requires provider configuration.`，原生侧有 `authentication required` 等），按 `openspec/project.md` 属于**允许保持字面量**的例外，所以这不是规范违规。

改为用已有的类型化信号 `availabilityState` 出本地化主标签（可用 / 不可用 / 需要登录 / 状态未知），后端原文降为次要说明行。顺带把硬编码的 `text-amber-600` 换成语义 token `text-[hsl(var(--warning))]`。

---

## 2. B 类：提案已实现

OpenSpec change：`openspec/changes/harden-workspace-dialogs-and-empty-states`，通过 `--strict`，任务 1–5 全部完成。

### P1-1 四个手写模态迁移到共享原语

`ApplicationDialog` 新增可选 footer 槽位——创建会话弹窗是"头部固定 + 中间滚动 + 底部固定"的三段布局，套一层会产生双滚动条。未传 footer 的 25 个既有调用方渲染完全不变。

迁移了创建会话、定时任务、批量删除确认、CLI 冲突四处，它们现在都有 Escape 关闭、Tab 焦点循环、焦点返回和 ARIA 关联。新增 `application-dialog.test.tsx` 覆盖这些行为。

### P1-4 移除浏览器原生弹窗 —— **审计把数量报少了**

审计说有 2 处 `window.prompt/confirm`。**这个数字是错的**：当时那条 grep 用的 `src\**\*.tsx` 通配在 `Select-String -Path` 下不会深层递归。递归扫描后实际有 **13 处**。

如果只改那 2 处，我写进 `visual-design-system` 的"禁止浏览器原生弹窗"就是一条假规范。所以本轮全部处理：

- 分类创建：抽出 `src/main-layout/create-category-dialog.tsx`（`main-layout.tsx` 已 420 行且在技术债豁免清单里，不宜继续堆）
- 定时任务删除：改行内两步确认。**没有用嵌套模态**——那会让两个 Escape 监听同时生效，一次按键关掉两层
- 其余 11 处形态一致（`window.confirm` 拦截破坏性操作），新增 `src/components/ui/use-confirmation.tsx` 收敛：Promise 形态，调用方保持 `if (!(await confirm({...}))) return;` 的原有控制流
- 新增机器守卫：`application-dialog.test.tsx` 里一条用例递归扫描 `src/**`，断言零 `window.prompt/alert/confirm`

四个 stub `window.confirm` 的单测改为驱动真实的应用内弹窗——比 stub 原生 API 更贴近实际行为。

### P1-9 循环工程空态

改为「图标 + 标题 + 说明 + 主按钮」，与聊天欢迎屏和通知中心一致。检查器面板的空态也补了说明。

规范 delta 覆盖三个 capability：

- `visual-design-system`：新增「共享模态行为」与「应用内文本输入」两条要求，让后续模态按构造继承而不是各自重新推导
- `main-layout-ui`：创建会话弹窗要求改用共享模态行为；新增定时任务/批量删除的模态行为要求与应用内分类创建要求
- `loop-management-ui`：新增首次进入空态要求

设计文档记录了一个真实的取舍：创建会话弹窗是"头部固定 + 中间滚动 + 底部固定"的三段布局，而 `ApplicationDialog` 目前是单一滚动区。方案选择给原语加可选 footer 槽位（对现有 25 个调用方保持默认渲染不变），而不是套一层导致双滚动条。

同时记录了主要风险：创建会话弹窗被 `tests/docs/documentation-screenshots.spec.ts` 用 `.fixed.inset-0 .ucd-panel` 选择器和最小包围盒断言，迁移会同时改变类结构和渲染尺寸，截图基线必须在同一个变更里更新——这正是它属于 B 类而非直接改的原因。

---

## 3. 第二轮：剩余 P2

### P2-3 任务板命名统一

原有三个名字：`work-board`（前端模块 / 服务 / 类型，且对应 Rust context `work_board`）、`todo-board`（destination 值与 DOM id）、「任务看板」（界面文案）。

按层次判断，真正多余的是中间那个：destination 标识符和 DOM id 既不属于领域层也不属于展示层，却自成一名。已统一为 `work-board`，与渲染它的模块及其后端 context 对齐；`onTodoBoard` 回调同步改为 `onWorkBoard`。

**没有动的两处，是有意的**：i18n 键 `todoBoard.*` 和文案「任务看板 / Todo Board」保留，因为规范里这个能力就叫 Todo Board（`unified-todo-board`、`main-layout-ui` 的 "Todo Board workspace destination"）；Rust context `work_board` 与 `WorkItem` 领域类型也保留，AGENTS.md 要求原生重构必须保持 Tauri command 名不变。剩下的「领域叫 work board、界面叫 Todo Board」是正常的分层，不是缺陷。

### P2-4 窄屏页签栏

先核实了现状：`session-tab-bar.tsx:80` 本来就有 `overflow-x-auto`，页签**能滚**——问题是滚动条不可见，看起来像被硬裁掉，用户不知道右边还有内容。

因此不需要溢出菜单，加可见的滚动提示即可。在 `styles.css` 新增共享工具类 `.ucd-scroll-strip`（细滚动条 + 常驻轨道），应用到页签栏。该文件 123 行，远低于 300 行上限，无需拆分。

### P2-9 活动栏语义分组

「定时任务」开的是弹窗，其余四项切换的是工作区目的地，此前混在同一排图标里。已拆成三组：目的地（会话 / Plan 执行 / 循环工程 / 任务看板）、工具（定时任务，带 `aria-haspopup="dialog"` 和上分隔线）、通用（设置 / 帮助）。

连带影响：Tab 顺序中「任务看板」现在排在「定时任务」之前，`workspace-activity-bar.spec.ts` 的键盘导航断言已相应更新。

### P2-10 移除创建会话弹窗的冗余行

「当前选中：Claude Code」与已高亮并带对勾的卡片重复，白占一屏滚动。已删除，`createSession.selectedAgent` 从五个 locale 移除。

`onepiece-agent.spec.ts` 曾用这行确认选中状态，改为直接断言卡片的 `aria-pressed="true"`——那本来就是更贴近语义的信号。

### P2-11 浅色主题终端：**按审计字面去做会造成功能回退，因此改了做法**

审计建议是"是否给浅色主题配浅色终端"。核实后不能这么做：`styles.css:98-102` 与 `terminal-theme.ts` 都写明了理由——全屏 TUI（Codex、Claude Code 这类）自己用 256/truecolor 绘制背景块，其配色前提就是深色画布。把背景改亮会让这些 TUI 的自绘背景与前景色错乱，属于功能回退。

因此**画布底色不动**，只解决审计真正观察到的现象（浅色 chrome 上一块无框黑矩形，视觉割裂）：新增 `--terminal-frame-border` 主题变量，给终端加圆角、边框和内阴影收口，浅色主题用更亮的边框色。TUI 依赖的调色板一个值都没改。

### P2-12 工作区路由 —— 已实现

`openspec/changes/address-workspace-destinations-by-route`，通过 `--strict`，任务 1–5 全部完成。

调查中发现一个比审计描述更有力的理由：`frontend-runtime-architecture` 的「Routed frontend surfaces」**已经**要求路由层能寻址各表面 "without relying on a single component-local view flag"，而四个目的地正是靠 `MainLayout` 里的 `destination` useState 在切——**现状本身就不满足既有规范**。

四个目的地、活动会话、会话创建现在都可寻址：`/workspace/sessions`、`/workspace/sessions/new`、`/workspace/sessions/<id>`、`/workspace/plans`、`/workspace/loops`、`/workspace/work-board`。URL 段与 P2-3 统一后的标识符一致，没有再引入第三个名字。后退键、深链、重启恢复都生效，`?createSession=1` 这个临时参数也去掉了。

**关键设计：用单个 `/workspace/*` 路由，而不是四个兄弟路由**。所有访问过的目的地保持挂载、用 `hidden` 隐藏，`main-layout-ui` 和 e2e 都断言了"返回时保留状态"。React Router 默认卸载上一个路由元素——四个兄弟路由这种最自然的写法会**恰好摧毁**这项被断言的行为。单路由 + 内部解析在结构上排除了这个问题，而不是靠"记得别那么写"。

连带发现：`loopCenterVisited` 这类标志原本在点击处理里置位，意味着直接访问 `/workspace/loops` 会渲染一片空白。已改为从当前目的地派生。

**实现中踩到的真实死循环**。首轮 e2e 大面积失败，报错是「waiting for element to be visible, enabled and **stable**」重复上百次——元素在，但永不稳定。根因是 `use-main-layout-model.ts` 里的 `sessions: sessionsQuery.data ?? []`：数据未加载时**每次渲染都新建数组**，而我的协调 effect 依赖了它，于是每渲染必跑、跑就导航、导航又渲染。修法是把协调逻辑抽成 `use-workspace-session-route.ts`，依赖收敛为原始值和稳定回调，数组存在 ref 里读取。

同一原因下 `onNavigate` 也改成接收**完整 location** 而非局部补丁——补丁形态必然闭包当前 location、每渲染都变。

另一处需要保护的是失败重试：`useSessionSwitch` 在后端拒绝时会回滚活动会话，路由却仍指向那个会话，会无限重试。用一个 ref 把重试限制为每次路由变化一次。

---

## 4. 未处理项

### 明确不做

| 编号 | 内容 | 原因 |
| --- | --- | --- |
| P1-10 | 全局快捷键体系 | 新增能力，撞"只优化已实现功能"的约束。已记入建议清单 |

### 文档漂移（审计 §0.1）

`openspec/project.md` 记 7 个 bounded context，`src-tauri/src/contexts/` 实际有 15 个，多出的 8 个（`code_intelligence`、`execution_observability`、`permissions`、`retrieval`、`skill_evolution_evidence`、`ssh_connections`、`task_orchestration`、`work_board`）无归属说明。属于文档修复，与 UI 优化混在一起会让两边都难 review，建议独立处理。

---

## 4. 验证结果

以下命令全部在本 worktree 实跑通过：

| 命令 | 结果 |
| --- | --- |
| `npm run lint:ci` | 通过，0 警告 |
| `npx tsc --noEmit` | 通过 |
| `npm run test` | 219 个测试文件、974 条用例全部通过 |
| `npm run build` | 通过，16 个懒加载 chunk，主静态闭包 124.0 KiB gzip |
| `npm run docs:check` | 通过 |
| `npm run docs:screenshots:check` | 21 项通过（基线已重新生成，见下） |
| `npm run contracts:check` | 通过 |
| `openspec validate --specs --strict` | 107 项全部通过 |
| `openspec validate harden-workspace-dialogs-and-empty-states --strict` | 通过 |
| `openspec validate address-workspace-destinations-by-route --strict` | 通过 |
| `npx playwright test` | 104 条全部通过 |
| `npm run test`（最终） | 222 个文件、994 条用例全部通过 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | 通过（Rust 侧零改动） |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 通过 |

### 期间修复的连带失败

单元测试：

- `workspace-activity-bar.test.tsx`：新增 `onHelp` 必填 prop 导致类型错误，已补参数并加上回调断言
- `basic-settings-page.test.tsx`：断言的是对齐前的 ja 文案「一般設定」，已更新为「基本構成」
- `top-bar.test.tsx`：新增用例渲染展开态顶栏会挂载 `NotificationCenter`，需要 `NotificationProvider` 包裹

e2e（首轮 8 失败 → 全绿）：

- `application-locales.spec.ts`：五个 locale 断言的都是对齐前的正文标题，已更新
- `onepiece-agent.spec.ts`：断言会话副标题显示裸的 `api`，即被本轮修掉的行为，改为断言本地化后的「API」
- `plan-execution.spec.ts`：**这条是我引入的**。我一度把顶栏搜索按钮的 aria-label 改成「搜索会话」，而 Playwright 的 `getByRole` name 选项默认是**大小写不敏感的子串匹配**，于是它与该 spec 用来定位活动栏的 `name: "会话"` 撞成 strict mode 违规。恢复原文案「打开搜索」即解决——比改测试去迁就一个我新引入的标签更合理
- `workspace-activity-bar.spec.ts`：原用例断言点击后出现第二个搜索输入框，即被本轮删掉的假控件，改为断言侧栏展开且其搜索框获得焦点
- `session-workspace-tabs.spec.ts` 首轮失败但单独重跑通过，属于负载下的偶发抖动，非本轮回归

e2e（第二轮 P2 改动后 1 失败 → 全绿）：

- `workspace-activity-bar.spec.ts` 的键盘导航断言：P2-9 把「任务看板」并入目的地组后，它在 Tab 顺序中排到了「定时任务」之前。属于本次改动的预期结果，断言已更新

e2e（第三轮模态迁移后 6 失败 → 全绿）：

- `application-locales.spec.ts` ×2 与 `documentation-screenshots.spec.ts`：用 `.ucd-panel` 定位创建会话弹窗，而 `ApplicationDialog` 不带这个类。改用 `getByRole("dialog")`——按角色定位本来就更稳
- `cli-parameters-settings.spec.ts`、`onepiece-agent.spec.ts`、`session-category-management.spec.ts`：用 `page.once("dialog", …)` 驱动浏览器原生弹窗，弹窗改成应用内之后这个钩子不再触发。改为直接点击弹窗里的按钮
- `workspace-activity-bar.spec.ts`：点击定时任务弹窗的「关闭」按钮，而 `ApplicationDialog` 不渲染关闭按钮（25 个既有调用方都靠 Escape 与遮罩关闭）。改为按 Escape，同时删除孤儿键 `scheduledTasks.close`

e2e（路由改动后 → 全绿）：

- 首轮大面积失败源于上文那个渲染死循环，修复后剩两条：`workspace-activity-bar` 断言点开定时任务后 URL 是 `/workspace`（现在带目的地段，且定时任务是弹窗不应改路由），以及我自己写的保留性断言竞态到了 Vite 冷启动。两处均已修正，隔离重跑两轮 32/32 稳定
- **一次误判**：与 e2e 并发跑时单测有 6 个文件失败，隔离重跑 994 条全过。这与仓库已知的并发敏感性一致，不是回归

单测（模态迁移后 6 失败 → 全绿）：

- 4 个测试文件 stub 了 `window.confirm`，已改为驱动真实弹窗
- `session-sidebar.tsx` 因新增弹窗代码涨到 307 行、越过 300 行硬上限且不在豁免清单里。把状态色映射 `lifecycleDotClass` 移到它本该在的 `session-lifecycle.ts`，而不是往豁免清单加文件

### 文档截图基线

**这是我在第一轮汇报时的疏漏。** 我跑了 `npm run docs:check`（链接与 README 一致性），但没跑 CI 里独立的 `docs:screenshots:check`——后者拿仓库中已提交的 PNG 做像素比对。补跑后确认它当时**已经是红的**：`create-session-en` 有 5% 像素差，且该套件是 serial 模式，首个失败即中断，后面的基线根本没被比对到。

第一轮的顶栏、状态药丸、设置侧栏分组、版本号等改动本就会改变渲染，第二轮的活动栏分组与终端边框又叠加了一层。已执行 `npm run docs:screenshots:update` 重新生成，20 张基线更新，`docs:screenshots:check` 21 项通过。已人工抽查 `session-workspace-zh-CN.png` 与 `create-session-zh-CN.png`，确认是预期变化而非渲染损坏。

`cargo clippy --all-targets -- -D warnings` 与 `cargo test` 未跑——两轮都没有改动任何 Rust 代码，且这两条在全新 worktree 里需要完整重编译。若要求完全照搬 CI 清单，应补跑。

### 视觉验证

用 Web/mock 运行时在 `minimal` 与 `futuristic` 两种风格、1440×900 与 860×900 两种宽度下实拍了改动前后各 16 张截图（`.ux-audit/shots/`，已加入 `.gitignore`）。改动后确认：四处状态标签一致、副标题显示「命令行」、假会话号消失、版本显示 `v0.1.0-preview.1`、设置分组渲染正确、空态收敛为单条、Toast 不再压住任何控件。

截图同时确认 Escape 仍关不掉创建会话弹窗——符合预期，那属于尚未实现的 B 类。

---

## 5. 后续建议

1. **会话页签与信息面板页签仍是组件内状态**。它们是第二层可寻址性，有自己的保留语义（`session-workspace-tabs`），本轮刻意排除
2. **`loop-definition-dialog.tsx` 是最后一个手写模态**。它自己实现了 Escape 与 autofocus，不算缺陷，但没有继承共享行为。`session-context-panel.tsx` 的上下文菜单同样没有 Escape 处理——它是菜单不是模态，超出本轮范围
3. **考虑给前端版本显示加同步校验**，或至少在 `check-version-sync.mjs` 的注释里说明前端版本已由构建注入、无需纳管
4. **重新审视设置导航顺序**。本轮受限于既有 e2e 断言只做了连续分组，「个性化」落在「能力与扩展」组内并不自然。若要调整顺序，应连同 `settings-navigation-order.spec.ts` 一起改，并说明新的编排依据
5. **审计里那条 grep 的教训值得记一笔**：`Select-String -Path 'src\**\*.tsx'` 不做深层递归，导致 `window.confirm` 的数量从 13 报成 2。凡是"全仓扫描"的结论，应该用 `Get-ChildItem -Recurse` 取文件列表再喂给 `Select-String`
