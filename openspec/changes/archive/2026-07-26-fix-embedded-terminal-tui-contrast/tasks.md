## 1. 终端调色板 token

- [x] 1.1 在 `src/styles.css` 新增语义化终端 token:`--terminal-background`、`--terminal-foreground`、`--terminal-cursor`、`--terminal-selection`、`--terminal-selection-foreground`,以及完整 16 色 `--terminal-ansi-{black,red,green,yellow,blue,magenta,cyan,white}` 与 `--terminal-ansi-bright-{black,red,green,yellow,blue,magenta,cyan,white}`,以固定深色 hex 值定义在基础 `:root` 层,供 `futuristic`/`minimal` 共享
- [x] 1.2 确认 token 命名与放置不与既有语义 token 冲突,且两个视觉样式均能继承(基础 `:root` 定义,`futuristic`/`minimal` 未覆盖即继承)

## 2. 完整不透明 xterm 主题

- [x] 2.1 在 `src/session-workspace/terminal-theme.ts` 新增 `terminalColor(name, fallback)`,读取终端 token 的原始计算值(不包 `hsl()`),每个 token 带安全深色 fallback
- [x] 2.2 重写 `createTerminalTheme()` 返回完整 xterm `ITheme`:不透明 `background`、`foreground`、`cursor`、`cursorAccent`、`selectionBackground`、`selectionForeground`,以及 `black/red/green/yellow/blue/magenta/cyan/white` + 8 个 `bright*`,全部取自终端 token

## 3. 终端组件改为不透明画布

- [x] 3.1 `src/session-workspace/agent-terminal-tab.tsx`:`new XtermTerminal({...})` 的 `allowTransparency` 改为 `false`;终端宿主 `div.ucd-agent-terminal` 去掉内联 `bg-[hsl(var(--panel-muted))]`
- [x] 3.2 `src/session-workspace/shell-tab.tsx`:同样将 `allowTransparency` 改为 `false`;终端宿主 `div.ucd-shell-terminal` 去掉内联 `bg-[hsl(var(--panel-muted))]`
- [x] 3.3 保留两个组件现有的 `themeObserver`(`data-theme` 变化后重建主题)行为不变

## 4. 收敛 CSS 覆盖

- [x] 4.1 在 `src/styles.css` 为 `.ucd-agent-terminal, .ucd-shell-terminal` 设置 `background: var(--terminal-background)`,使 padding 边框与终端底色一致
- [x] 4.2 删除针对 agent/shell 终端的半截覆盖:`.xterm-viewport{background:transparent}`、`.xterm-bg-0`→`panel-muted`、`.xterm-bg-257`→`panel-muted`、`.xterm-fg-257`→`foreground`(改由 xterm 主题统一提供背景/前景)

## 5. 测试与验证

- [x] 5.1 更新 `src/session-workspace/session-workspace-components.test.tsx` 的两条"终端可读性"用例,由断言旧的透明+逐色覆盖契约改为断言不透明画布 + 完整调色板 token
- [x] 5.2 `npm run lint` 通过(0 error;既有 warning 均在 `.claude/worktrees/` 副本,非本次文件)
- [x] 5.3 真实测试套件通过:排除 `.claude/worktrees/` 后 `vitest run` 92 文件 / 301 测试全绿。注:裸 `npm run test` 因未跟踪的 `.claude/worktrees/main-dev/`(嵌套 worktree 的 node_modules 第三方测试 + e2e specs 被误扫)exit 1,属既有环境噪音,与本改动无关
- [x] 5.4 `npm run build` 通过
- [x] 5.5 目视 QA:operator 在桌面端实机运行 Codex 确认已修复(终端为深色画布、文字可读、无黑底盖字)
- [x] 5.6 `openspec validate "fix-embedded-terminal-tui-contrast" --strict` 通过
