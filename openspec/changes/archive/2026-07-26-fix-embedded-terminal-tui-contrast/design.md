## Context

工作区 agent 终端(`src/session-workspace/agent-terminal-tab.tsx`)与 Shell 终端(`src/session-workspace/shell-tab.tsx`)都用 xterm.js 渲染 PTY 原始输出,共用 `src/session-workspace/terminal-theme.ts` 的 `createTerminalTheme()` 和 `src/styles.css` 中 `.ucd-agent-terminal` / `.ucd-shell-terminal` 一组样式。

现状:`createTerminalTheme()` 返回的主题 `background: rgba(0,0,0,0)` 透明,16 色 ANSI 只映射了 `black`/`brightBlack` 两色;`styles.css` 用 `.xterm-viewport{background:transparent}` + `.xterm-bg-0`/`.xterm-bg-257` → `panel-muted`、`.xterm-fg-257` → `foreground` 把终端"漂白"成浅色。Codex 等全屏 TUI 用 256 色 / truecolor 背景绘制输入框、状态栏,这些既非索引 0 也非默认背景,不被上述覆盖命中;truecolor 更是走 xterm DOM 渲染器的行内样式,CSS 类选择器无法拦截。结果这些区域按原始深色渲染成黑块,其上为深色终端配的浅色/中间色文字失去对比度 → 黑底看不清。

## Goals / Non-Goals

**Goals:**

- 内嵌终端在一块自洽、不透明的深色画布上渲染,提供**完整 16 色 ANSI 调色板**,让任意 ANSI/256/truecolor 前景与背景都能原生、可读地呈现。
- 终端配色来自 `styles.css` 的**语义化终端 token**,不在组件里硬编码调色板;两个注册视觉样式(`futuristic`/`minimal`)复用同一套终端画布。
- agent 终端与 Shell 终端共用同一渲染路径,一处修复两处受益,并覆盖后续接入的其他全屏 TUI。

**Non-Goals:**

- 不强制 agent CLI 切换到浅色主题,不改写其色彩输出;终端忠实渲染,只是给它一块深色底。
- 不修改 PTY 环境变量 / 色深(不设 `TERM`/`COLORTERM`)。
- 不改动前端 service 边界、Tauri 命令、适配器契约或 SQLite。
- 不改动终端下方 VaneHub 自带的命令输入框(`.ucd-agent-terminal-input`),它仍随应用主题。

## Decisions

1. **新增语义化终端 token(`src/styles.css`)**
   - 定义一组终端专用 token:`--terminal-background`、`--terminal-foreground`、`--terminal-cursor`、`--terminal-selection`、`--terminal-selection-foreground`,以及完整 16 色 `--terminal-ansi-{black,red,green,yellow,blue,magenta,cyan,white}` 与 `--terminal-ansi-bright-{...}`。
   - 终端是自成一体的深色面,采用一套固定的深色 ANSI 调色板,放在基础 `:root` 层,`futuristic`/`minimal` 默认共享(样式如需可覆盖同名 token,组件不感知)。
   - 这些 token 以**完整颜色值**(hex)定义,而非项目其它 token 的 HSL 三元组——16 色 ANSI 调色板用 hex 是终端领域通用写法,更可读可维护;此偏差仅限终端调色板,并在此记录理由。

2. **`createTerminalTheme()` 返回完整不透明主题(`terminal-theme.ts`)**
   - 改为读取上述终端 token 的**原始计算值**(新增 `terminalColor(name, fallback)`,直接返回 computed value,不再包 `hsl()`),各 token 带一个安全 fallback 深色默认值。
   - 返回完整 xterm `ITheme`:不透明 `background`、`foreground`、`cursor`、`cursorAccent`、`selectionBackground`、`selectionForeground`,以及 `black/red/green/yellow/blue/magenta/cyan/white` + 8 个 `bright*`。

3. **两个终端组件设为不透明画布**
   - `agent-terminal-tab.tsx` 与 `shell-tab.tsx` 的 `new XtermTerminal({...})` 将 `allowTransparency` 由 `true` 改为 `false`(不透明背景由主题填充)。
   - 终端宿主 `<div className="ucd-agent-terminal ...">` / `ucd-shell-terminal` 去掉内联 `bg-[hsl(var(--panel-muted))]`,背景改由 `styles.css` 中 `.ucd-agent-terminal, .ucd-shell-terminal { background: var(--terminal-background); }` 提供,使 padding 边框与终端底色一致。
   - 保留现有 `themeObserver`(监听 `data-theme` 变化后重建主题)。

4. **移除只兜半截的 CSS 覆盖(`styles.css`)**
   - 删除 `.xterm-viewport{background:transparent}`、`.xterm-bg-0`→`panel-muted`、`.xterm-bg-257`→`panel-muted`、`.xterm-fg-257`→`foreground` 这四组 agent/shell 终端覆盖(它们只对索引 0/默认色生效,是问题的来源)。终端背景/前景改由 xterm 主题统一提供。

## Risks / Trade-offs

- **浅色应用模式下终端是"深色孤岛"**:这是刻意取舍。agent CLI 普遍是深色 TUI,深色画布是让其忠实且可读的唯一稳妥解法(等价于绝大多数 IDE 的内嵌终端做法),优先保证可读性。
- **既有测试/截图**:需检查 `src/session-workspace/session-workspace-components.test.tsx` 与 `tests/` 下工作区/文档截图用例是否断言了终端浅色外观,必要时同步更新期望。
- **token 偏差**:终端调色板用 hex 而非 HSL 三元组,与项目其它 token 约定不同——已在 Decisions 记录理由,范围仅限终端。
- **视觉一致性**:改动后需在 `futuristic`/`minimal` × 浅/深 下目视确认终端可读、无残留浅色缝隙、无低对比文字(对照 project.md「Frontend Visual Design」的 Visual QA 要求)。
