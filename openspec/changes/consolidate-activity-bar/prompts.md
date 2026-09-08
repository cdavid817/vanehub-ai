# consolidate-activity-bar — 分组实施 Prompt

用法：把 `consolidate-activity-bar/` 整个目录放到 `openspec/changes/` 下，先跑
`openspec validate consolidate-activity-bar --strict`；通过后，按顺序把下面每个 Prompt
单独喂给 Claude Code（一组一个会话或一个 turn，不要一次性丢全部）。

每组的公共前置（Claude Code 已通过 AGENTS.md 知道，无需重复粘贴，此处仅备忘）：
React 19 + TS strict、Tailwind、仅用 React 内置状态、组件不得直接 `invoke()`、
单文件不超过 300 行、所有用户可见文案走 i18n 且五个 locale 同步、
禁止 `any` / `@ts-ignore`。

---

## Prompt 0 — 立项校验（可选，先确认 change 包本身没问题）

```
读取 openspec/changes/consolidate-activity-bar/ 下的 proposal.md、design.md、tasks.md
和 specs/ 目录，然后：

1. 跑 `openspec validate consolidate-activity-bar --strict`，把失败项逐条列出来。
2. 检查 spec delta 是否覆盖了所有会被这次改动影响的主规范。重点核对
   openspec/specs/settings-center-ui/spec.md 和
   openspec/specs/frontend-runtime-architecture/spec.md 里提到 activity bar 的条款，
   判断它们是否需要 MODIFIED delta。
3. 只报告结论和需要补的 delta 草稿，先不要改任何 src/ 下的代码。
```

---

## Prompt 1 — 路由与目标模型（tasks 1.1–1.2）

```
实施 openspec/changes/consolidate-activity-bar 的任务组 1。只做这一组，不要提前动
activity bar 组件或 inbox/automations 页面。

范围：
- src/main-layout/workspace-route.ts
- src/main-layout/use-main-layout-model.ts
- 对应的 workspace-route.test.ts / use-main-layout-model 测试

要求：
1. WorkspaceDestination 收敛为 "sessions" | "inbox" | "automations"。
2. WorkspaceLocation 增加可选 view 字段；路径形如 /workspace/<destination>[/<view>]。
   sessions 保持既有的 /workspace/sessions/<id|new> 形状不变，"new" 段的保留语义不变。
3. parseWorkspaceLocation 按 design.md 的重定向表处理 legacy 值
   （mission-control / system-activity / work-board → inbox，loops / goals → automations，
   evaluations → 打开设置的 evaluation 页并保留 sessions 作为背景目标）。
   未知 destination 与未知 view 分别回落到 sessions 和该目标的默认 view。
4. 重定向必须同时作用于 URL 解析和 localStorage 里 vanehub.workspace.location.v1 的旧值，
   不做数据迁移。
5. formatWorkspaceLocation 与解析保持往返一致。

验收：为重定向表的每一行、未知值回落、往返一致性补单测。
跑 `npm run lint:ci`、`npm run test`、`npx tsc --noEmit`。
本组不改 i18n，不改组件渲染。
```

---

## Prompt 2 — 活动栏（tasks 2.1–2.3）

```
实施 openspec/changes/consolidate-activity-bar 的任务组 2，依赖任务组 1 已合入。

范围：
- src/main-layout/workspace-activity-bar.tsx（重构）
- src/main-layout/main-layout.tsx（接线）
- src/i18n/locales/*.json（五个 locale）

要求：
1. 把 11 个逐项回调 props 换成声明式配置：
   items: ActivityItem[] 与 utilityItems: ActivityItem[]，
   ActivityItem = { id, icon, label, shortcut, badge?, onSelect, ariaControls? }。
   组件只负责渲染与可访问性，不再持有任何具体目标的知识。
2. 渲染四个入口：Sessions / Inbox / Automations 在上组，Settings 在下组。
   删除 Loops、Scheduled Tasks、Task Board、Goals、Evaluations、System Activity、
   Mission Control、Help 八个入口及其回调与 xxxVisited 标志。
3. Sessions 入口保留现有的侧栏展开/折叠语义和 aria-expanded。
4. badge 只在 badge > 0 时渲染；本组只给 Inbox 接 badge，数值先接现有
   systemActivityUnread，任务组 3 再改成合并计数。
5. 在 workspace shell 层注册 Mod+1..4；文本输入或 composer 获得焦点时忽略。
   tooltip 里带上快捷键。
6. i18n：新增 layout.activityBar.inbox / .automations，删除已退休入口的 key，
   五个 locale（en / ja / ko / zh-CN / zh-TW）同步。

验收：更新 activity bar 组件测试（四个入口、单一 badge、快捷键、焦点守卫、
icon-only 可访问名）。跑 `npm run lint:ci`、`npm run test`、`npm run build`、
`npx vitest run src/i18n/i18n-resource-parity.test.ts`。
```

---

## Prompt 3 — 收件箱（tasks 3.1–3.3）

```
实施 openspec/changes/consolidate-activity-bar 的任务组 3。

新建 src/inbox/，把现有的 mission-control 与 system-activity 组合成一个关注面，
不要改动这两个目录里的数据获取、事件契约或 fixture。

要求：
1. inbox.tsx 渲染三段：Needs attention / Running / Recently finished。
   - Needs attention = Mission Control 的 attention 排序运行 + 未读且 severity 为
     warning/critical 的 System Activity 条目
   - Running = Mission Control 非终态运行
   - Recently finished = Mission Control 的 recent 段
   复用 mission-control/event-coalescer.ts 与
   system-activity/activity-presentation-registry.ts，System Activity 条目以行的形式
   渲染在对应段内。
2. 每行保留其原有归属面的跳转与控制动作（沿用 Mission Control 既有的 onNavigate 契约）。
3. badge 归属改到 Inbox：合并 Mission Control attention 计数与 System Activity 未读计数，
   按 run id 去重，同一 run 只计一次。
4. 提供 "Activity log" 折叠区，内嵌现有 system-activity-view 的时间线（保留其搜索与
   severity 过滤）。导出、重建、健康面板这三块控件不放这里——任务组 5 会搬到设置。
5. Inbox 头部加 List / Board 视图切换，Board 模式渲染现有 work-board 组件；
   所选视图与其他布局偏好一起持久化。
6. 单文件超过 300 行就拆分。

验收：为三段归类、badge 去重、视图切换持久化、Activity log 折叠补组件测试。
跑 `npm run lint:ci`、`npm run test`、`npm run build`。
```

---

## Prompt 4 — 自动化（tasks 4.1–4.3）

```
实施 openspec/changes/consolidate-activity-bar 的任务组 4。

新建 src/automations/，作为 Loops / Scheduled / Goals 三个页签的薄壳。

要求：
1. automations.tsx 用与现有目标相同的方式按页签懒加载 loop-center、定时任务、goal-center，
   首次访问后保持挂载。
2. 定时任务从对话框改为内联页面内容：沿用现有的表单、校验、列表、刷新、
   变更与错误保留行为，只换容器。原 spec 里"焦点移入对话框"改为"焦点移到该面第一个
   可交互控件"；不再渲染 modal。切走页签再回来，已加载列表与未提交草稿要保留。
3. 删除 main-layout 里的 scheduledTasksOpen 对话框状态与触发路径。
4. Mission Control 与 Goal Center 通过 onNavigate 发出的跳转目标，改为映射到新的
   destination + view。
5. 支持 /workspace/automations/<tab> 深链直达对应页签。

验收：更新定时任务相关组件测试为内联形态；补页签懒加载与深链测试。
E2E 里 desktop-scheduled-tasks 这一层会依赖对话框选择器，同步改掉
（`npm run test:desktop:build` 后 `npm run test:desktop:scheduled-tasks`）。
跑 `npm run lint:ci`、`npm run test`、`npm run build`。
```

---

## Prompt 5 — 设置迁移（tasks 5.1–5.3）

```
实施 openspec/changes/consolidate-activity-bar 的任务组 5。

要求：
1. settings-page-types.ts 的 SettingsPageId 增加 "evaluation"；在 Agents 分组下新增设置页，
   直接包裹现有 evaluation-center 组件，不改其内部行为。
2. observability 设置页增加 "System activity" 区块，承接从 system-activity-controls.tsx
   与 system-activity-health-panel.tsx 搬过来的导出、重建、健康面板控件。
   这些控件的服务调用与状态机不变，只换宿主页面。
3. Help 移到设置侧栏底部分组；main-layout 里 onHelp 的活动栏入口删除
   （它本来就是 onOpenSettings("help")）。
4. 五个 locale 补齐新页面与新区块的文案。

验收：更新 settings-pages.test.ts 的页面 id 顺序断言；补 observability 新区块的测试。
跑 `npm run lint:ci`、`npm run test`、
`npx vitest run src/i18n/i18n-resource-parity.test.ts`。
```

---

## Prompt 6 — 验证与收尾（tasks 6.1–6.4）

```
完成 openspec/changes/consolidate-activity-bar 的任务组 6。

1. 通读任务组 1–5 的改动，补齐遗漏的组件测试：活动栏配置渲染、路由重定向、
   Inbox 三段与 badge 去重、Automations 页签、设置页迁移。
2. 重写所有经由已退休入口导航的 E2E 流程，改走新入口或顶栏搜索；
   新增 legacy 路径重定向与 Mod+1..4 的覆盖。
3. 顶栏搜索需要能搜到并跳转 Task Board、Evaluations 以及 Automations 的每个页签——
   确认这条已实现，没有就补上。
4. 在 futuristic 与 minimal 两种风格下，按桌面宽度与窄宽度目视检查 Inbox 与
   Automations：重叠、截断、对比度不足、空白面板。
5. 按 AGENTS.md「校验命令」一节逐字跑全部命令，另加：
   `openspec validate consolidate-activity-bar --strict`
   `npm run architecture:check`
   `npx playwright test`
   `npm run test:desktop:build` 然后 `npm run test:desktop:smoke`
      与 `npm run test:desktop:scheduled-tasks`
6. 把 tasks.md 里已完成项勾上，逐条记录验证结果。桌面层结果只报当前平台，
   不外推到其他操作系统。
```

---

## 一次性替代方案（不推荐，但如果你想让 Codex 当架构师先过一遍）

```
读取 openspec/changes/consolidate-activity-bar/ 全部内容与 src/main-layout/、
src/mission-control/、src/system-activity/、src/loop-center/、src/goal-center/、
src/work-board/ 的现状。

不要写代码。回答三件事：
1. design.md 的重定向表和 Inbox 三段归类，在现有数据契约下是否可实现？哪一段缺数据？
2. tasks.md 的六组顺序有没有隐藏依赖倒置？哪两组其实必须合并或对调？
3. 哪些现有测试与 E2E 层会因为这次改动失败？列清单，按"必须改"和"会自动通过"分类。

输出一份修订建议，我会据此调整 change 包后再交给实现方。
```
