# consolidate-activity-bar

VaneHub AI 活动栏由 10 个入口收敛为 4 个（Sessions / Inbox / Automations / Settings）的
OpenSpec 变更包。

## 目录内容

```
consolidate-activity-bar/
├─ proposal.md      Why / What Changes / Capabilities / Impact
├─ design.md        目标模型、重定向表、Inbox 与 Automations 组合方式、风险与取舍
├─ tasks.md         6 组 18 项实施清单
├─ prompts.md       分组实施 Prompt（交给 Claude Code 用）
└─ specs/
   ├─ main-layout-ui/spec.md              MODIFIED 活动栏 + ADDED 三个需求
   └─ scheduled-task-management/spec.md   RENAMED + MODIFIED（对话框 → 页面）
```

## 使用

```bash
# 1. 放进仓库
cp -r consolidate-activity-bar <repo>/openspec/changes/

# 2. 校验（README.md 与 prompts.md 不属于 OpenSpec 工件，校验器会忽略）
cd <repo>
openspec validate consolidate-activity-bar --strict

# 3. 按 prompts.md 逐组实施
```

## 范围

纯前端。不涉及 Tauri command、service 接口、runtime adapter、数据库或依赖变更。
持久化的 workspace location 无需迁移：未知 destination 已有回落到 sessions 的行为。

## 不在本次范围内

- Mission Control、System Activity、Loop Center、Goal Center、Task Board、
  Evaluation Center 各自的内部实现
- 会话侧栏、信息面板、设置中心布局（除新增 Help 入口与 evaluation 页外）
- 目标（Goals）是否应改为 Inbox / Automations 上的 group-by 维度——留待后续变更
