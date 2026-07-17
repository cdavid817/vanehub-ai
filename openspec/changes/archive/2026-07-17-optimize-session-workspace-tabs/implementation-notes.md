## 第一版实现说明

本文件记录当前代码已经采用的短期实现、Web/mock 数据边界，以及后续扩展时不应被误认为长期架构的部分。长期设计与取舍仍以 `design.md` 为准。

### 当前实现边界

| 能力 | 第一版实现 | 当前上限/行为 | 后续优化入口 |
|---|---|---|---|
| Tab 容器 | 首次点击挂载；之后用 `hidden`/`block` 保活 | 切换会话重置为仅 Chat，旧会话 Shell 随卸载清理 | 带 LRU 的每会话 Tab 状态缓存 |
| Files | 按目录逐层读取并在前端保留展开/选中状态 | 单层最多 500 项；跳过隐藏项；预览最多 1 MiB | 请求取消、虚拟树、文件监听、分块读取 |
| Documents | Rust 有界递归发现，复用 Files 文本读取 | 深度 6、最多 300 个 `.md/.markdown/.txt`；Markdown 禁用原始 HTML | 原生文档索引、更多格式、安全链接处理 |
| Changes | Rust 输出 status/file/hunk/line 结构，前端由一个模型生成 unified/split | diff 最多 2 MiB；只读 working/staged/untracked | 虚拟化、增量 diff、commit/base 对比、Git 写操作另立提案 |
| Terminal | 从当前已加载消息的 `toolUse` 聚合执行卡片 | 当前历史最多 1000 条消息，并显示部分结果提示 | 原生聚合、耗时/成本维度、报告导出 |
| Shell | 桌面使用 `portable-pty`；Web 使用显式模拟器 | 每个已挂载 Shell Tab 一个进程；无重连/持久 scrollback | 多 Shell、重连、后台策略、搜索与可选脱敏历史 |
| Logs | 读取统一 JSONL 日志，按精确 `sessionId`、级别和搜索过滤 | 每页最多 200 条；不写 SQLite；导出只含过滤后的脱敏记录 | 日志索引/sidecar、归档选择、流式导出 |
| Report | 纯函数聚合消息 token、字符估算、工具排行和时间线 | 仅聚合当前有界消息历史，不把字符估算伪装为 token | 后端聚合、缓存 token、成本/时长和导出 |

### 哪些数据采用 Web/mock

Mock 只存在于 `web-agent-client.ts` / `web-session-workspace-client.ts` 的浏览器适配器。Tauri 桌面适配器不会读取这些 fixture。

| 页面/能力 | Web/mock 数据 | 桌面真实来源 | 页面提示 |
|---|---|---|---|
| Chat | Web adapter 生成确定性的流式回复、tokenUsage 和一个 `read_file` tool-use 事件 | 当前 Agent/CLI 会话消息 | Web 回复正文明确为 preview |
| Files | 固定目录树：`docs`、`src`、`README.md`、`package.json` 及其文本内容 | 会话注册根目录下的受限文件读取 | 文件内容包含 Web preview/mock 说明 |
| Documents | 固定的 README、architecture.md、notes.txt | 会话根目录有界扫描 | 文档正文标识为 Web preview |
| Changes | 固定分支 `worktree/web-preview`、3 条 status 和 `src/main.ts` 结构化 diff | 会话根目录执行受限 Git 命令 | 分支名和内容带 `web-preview`/`web-mock` |
| Terminal | 来自 Web mock Chat 产生的 `read_file` tool-use；不是独立硬编码卡片数组 | 当前会话消息中的真实 `toolUse` | 卡片数据与 Chat 消息一致 |
| Shell | `web-shell-*` 会话、`mock>` 提示符和输入回显；不启动本地进程 | 平台默认 Shell 的真实 PTY | 顶部“模拟环境”徽标和终端 banner |
| Logs | 3 条固定、已脱敏的 info/debug/warn 日志 | 当前统一日志文件中精确匹配 sessionId 的记录 | 本地导出返回“Web 预览模式不支持” |
| Report | 不单独造数据；聚合 Web mock Chat 消息、tokenUsage 和 tool-use | 聚合所选桌面会话消息 | 仍遵循部分历史/估算标签 |
| 右侧 Files/Changes 概览 | 复用上述 Web workspace service 结果 | 复用同一桌面 service 合约 | 不维护第二套 UI 数据模型 |

### Mock 使用原则

- Web/mock 结果必须可重复，以便 Playwright 稳定断言。
- 模拟 Shell 只回显，不执行命令、不访问本地文件系统。
- Web 导出不得伪造成功路径，统一返回 typed `unavailable`。
- Terminal 与 Report 从同一消息模型派生，避免一份“看起来成功”但与聊天无关的孤立 mock。
- 新增能力必须同时实现 Tauri 和 Web adapter；无法在浏览器真实执行的能力要显式标注 simulated/unavailable。

### 已知短期实现与优化点

- Tab 是组件级 lazy mount，不是 bundle code-splitting；若体积继续增长，可对非 Chat Tab 增加动态 import。
- Files/Documents/Logs 当前以组件本地 state 管理请求；后续可统一接入可取消 query 和跨右侧概览共享缓存。
- Logs 当前扫描活动 JSONL 文件；文件显著增大后再评估索引，不引入第二份日志真源。
- Report 当前没有持久化快照；刷新后由消息重新聚合。
- Xterm 主题在 `data-theme` 变化时从语义 CSS 变量重算；后续新增主题只需满足相同 token 合约。
- 当前 Git parser 以受控参数和有界输出为前提；超大仓库应增加后台任务、超时与增量结果。

### 验证入口

- 前端纯函数/组件/适配器：`npm run test`
- TypeScript 与生产构建：`npm run lint`、`npm run build`
- Web/mock 八 Tab 流程：`tests/e2e/session-workspace-tabs.spec.ts`
- 桌面文件/Git/日志/Shell：`cargo test --manifest-path src-tauri/Cargo.toml`
- OpenSpec 一致性：`openspec validate "optimize-session-workspace-tabs" --strict` 与 `openspec validate --specs --strict`
