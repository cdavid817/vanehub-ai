## Why

对 9 个会话工作区标签页（chat、changes、documents、files、terminal、shell、logs、traces、report）的审计发现了多处缺陷和死代码：

1. **i18n 重复键** — `zh-CN.json` 和 `en.json` 各包含 211 个重复的顶级键，来自一次合并错误。重复块中缺少 7 个键，可能导致某些命名空间下的翻译查找静默失败。
2. **files-tab 静默展开错误** — 当子目录展开在 `files-tab` 中失败时，错误仅出现在预览面板（右侧）中，而非用户点击所在的树形面板。目录在视觉上被标记为已展开（ChevronDown），但没有渲染子节点。
3. **changes-tab 选择重置** — 当 git 状态查询重新获取时，用户选择的文件会无条件重置为 `items[0]`。
4. **changes-tab 截断被忽略** — `GitDiffResult.truncated` 从未被检查，差分截断未显示任何指示。
5. **remote-terminal-panels 死代码** — 7 个组件中包含硬编码的英文字符串，除了它们自己的测试文件外，从未被导入。

## What Changes

- 从两个语言环境文件中删除重复的 i18n 键块（每个文件 211 行）
- 在树形部分添加错误通知，使用 `PartialNotice` 模式；重构 `toggleDirectory`，在加载失败时不展开
- 添加 `useRef` 守卫，自动选择仅在初始加载时触发；在差分面板中显示 `PartialNotice` 以指示截断
- 删除 `remote-terminal-panels.tsx` 和 `remote-terminal-panels.test.tsx`
- 重构 `changes-tab.tsx`：提取 `FileRow`、`DiffBody` 组件；移除冗余的 `status` 状态
- 扩展 i18n 可见文本防护栏，覆盖所有 session-workspace UI 文件（+7 个文件）
- 为两个修复添加带有失败测试的回归测试（TDD：RED → GREEN → REFACTOR）

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `session-workspace-files`: Tree section 现在显示展开错误；目录在加载失败时保持折叠状态。
- `session-workspace-changes`: 文件选择在状态重新获取时保持不变；差分截断显示部分通知。
- `i18n-resources`: 重复键回归防护；扩展了 session-workspace/ 的防护栏覆盖范围。

## Impact

- Frontend: 仅限 `src/session-workspace/`、`src/i18n/locales/`、`src/i18n/i18n-visible-text-guardrail.test.ts`、`src/i18n/i18n-resource-parity.test.ts`
- 后端: 无更改
- 数据库: 无更改
- 依赖: 无新依赖
- 测试: 新增 8 个测试（files-tab 2 个，changes-tab 6 个）；删除 1 个测试（remote-terminal-panels 1 个）
