## Design Decisions

### 1. files-tab: Tree-section error display

**Decision**: 使用紧凑的 `<p role="alert">` 横幅，位于文件列表上方，样式遵循 `PartialNotice`（`rounded border border-border bg-muted px-2 py-1 text-xs text-muted-foreground`），而不是完整的 `WorkspaceState`（`h-full min-h-40` 会挤占行空间）。

**Alternatives considered**:
- 每个目录的错误指示器 — 需要每个目录的错误状态跟踪，对于已与文件读取错误共享 `error` 状态的组件来说增加了复杂性
- 仅预览面板错误 — 已存在但位置错误；用户需要在树形部分看到反馈

### 2. files-tab: No expand on failure

**Decision**: 在 `toggleDirectory` 中，仅当 `loadDirectory` 成功后才将路径添加到 `expanded`。失败时，提前返回，不调用 `setExpanded`。

### 3. changes-tab: Selection preservation

**Decision**: 使用 `useRef(false)` 守卫，初始自动选择仅在首次数据到达时触发。会话切换时通过现有 effect 重置守卫（`}, [sessionId]`）。

### 4. changes-tab: Diff truncation

**Decision**: 在差分面板主体上方放置 `PartialNotice`，镜像文件列表中已有的 `status.truncated` 通知。

### 5. changes-tab: Redundant state removal

**Decision**: 移除 `status` 状态变量；所有使用场景直接引用 `statusQuery.data`。这消除了一类状态同步 bug。

### 6. changes-tab: Component extraction

提取了三个小组件以提高可读性：
- `FileRow` — 文件列表按钮，包含状态码和种类标签
- `DiffBody` — 差分内容，包含加载/错误/空/二进制/超大文件状态
- `Toggle` — 用于工作区/已暂存和统一/拆分模式切换的切换按钮

### 7. i18n dedup

**Decision**: 基于原始 JSON 源的精确字符串解析进行删除，而非通过 `i18next` 键解析（后者会去重）。这防止了源文件中的未来重复。

### 8. Dead code removal

**Decision**: 完全删除 `remote-terminal-panels.tsx` 和 `.test.tsx`。这些类型在 `types/remote-terminal.ts` 中保留，因为它们仍被实时 `remote-terminal-client.ts` 服务层传递使用。

### 9. Guardrail expansion

**Decision**: 添加了 7 个 session-workspace UI 文件（`agent-terminal-tab.tsx`、`chat-tab.tsx`、`diff-view.tsx`、`execution-timeline-tab.tsx`、`folder-opener-control.tsx`、`log-entry-article.tsx`、`session-tabs.tsx`）到 `checkedFiles` 列表中。在添加前已扫描硬编码文本，未发现违规。
