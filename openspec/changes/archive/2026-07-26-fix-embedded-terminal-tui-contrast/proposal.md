## Why

内嵌终端(工作区 agent 终端与 Shell 终端)在运行 Codex CLI 等全屏 TUI 时出现黑底色块且其上文字低对比、无法阅读。根因是终端被强行做成"透明背景 + 只中和两类 ANSI 背景色"的浅色外观:xterm.js 以 `allowTransparency` 透明渲染,CSS 只把 `xterm-bg-0`(ANSI 黑)与 `xterm-bg-257`(默认背景)改成浅色,而 Codex 用 256 色 / 24 位 truecolor 背景绘制的输入框、选中行、状态栏未被覆盖(truecolor 走行内样式,CSS 类根本无法拦截),于是按原始深色渲染成黑块,其上为深色终端准备的文字失去对比度。

## What Changes

- 内嵌终端(agent 终端 + Shell 终端)改为渲染在一块**自洽、不透明的深色终端画布**上,ANSI 前景/背景色按标准终端方式原生渲染,不再依赖"透明 + 逐色 CSS 兜底"。
- 在 `src/styles.css` 新增一组**语义化终端调色板 token**(完整 16 色 ANSI 前景/亮色、不透明终端背景、光标、选区前景/背景),供两个注册视觉样式复用,避免在组件里硬编码调色板。
- `createTerminalTheme()` 由这些 token 生成**完整**的 xterm 主题(不透明背景 + 16 色全集),取代当前只映射 `black`/`brightBlack` 两色、背景透明的残缺主题。
- 移除只兜半截的 CSS 覆盖(`.xterm-bg-0`、`.xterm-bg-257`、`.xterm-fg-257` 改色与 `.xterm-viewport` 透明),终端容器背景由浅色面板改为终端画布背景 token,消除深色 TUI 块与浅色面板之间的割裂。
- 该改动同时惠及后续接入的其他全屏 TUI(Claude Code、Gemini CLI 等),不需要为每个 CLI 再逐色追加兜底。

## Capabilities

### New Capabilities

（无）

### Modified Capabilities

- `session-workspace-tabs`: 在"主题感知的会话标签"能力下补充要求——内嵌终端(工作区 agent 终端与 Shell 终端)必须在自洽的不透明终端画布上以完整 ANSI 调色板渲染 agent/TUI 输出,使全屏 TUI 绘制的背景色块及其文字在两个注册视觉样式下都保持可读,不得残留未中和的深色块或低对比文字。

## Impact

- 影响运行时:桌面端与 Web 端均受影响(改动集中在前端渲染层)。
- 受影响代码:
  - `src/session-workspace/terminal-theme.ts`(`createTerminalTheme` 生成完整不透明主题)
  - `src/styles.css`(新增终端调色板 token;移除/替换 `.xterm-bg-0`/`.xterm-bg-257`/`.xterm-fg-257`/`.xterm-viewport` 覆盖;终端容器背景)
  - `src/session-workspace/agent-terminal-tab.tsx` 与 `src/session-workspace/shell-tab.tsx`(终端宿主容器背景类、`allowTransparency` 取值)
- 不改动:前端 service 边界、Tauri 命令、适配器契约、SQLite;不涉及前后端隔离或 runtime adapter 边界变化。
- 无破坏性变更;不新增依赖。
