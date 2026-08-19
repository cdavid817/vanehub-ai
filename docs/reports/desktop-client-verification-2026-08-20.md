# VaneHub AI 桌面客户端运行验证报告（2026-08-20）

承接 [`e2e-test-report-2026-08-19.md`](e2e-test-report-2026-08-19.md)。上一轮是静态审查加单测；这一轮全部在**真实构建、真实启动的桌面客户端**上跑，起点 `43621d88`。

Web mock adapter 从内存作答，这一轮发现的缺陷没有一条在它上面可观测——这是必须上桌面端的理由，不是偏好。

## 1. 结论摘要

- 命令覆盖率从 30/407（7%）提升到 182/407（44%）；有副作用的命令从 10/191（5%）到 103/191（53%）。
- 确认并修复 **12 个产品缺陷**，每个都附带先红后绿的回归测试。
- 三条曾被当作缺陷的现象**证伪**：worktree 残留、antigravity 失败、gemini 拒绝——分别是断言写错、账号区域限制、账号档位限制。
- 桌面套件从 10 个 spec 扩到 13 个，新增的三个**只通过 UI 控件**驱动，不走 `core.invoke`。

## 2. 缺陷清单

级别口径同上一轮：P0 = 数据损坏或主链路不可用；P1 = 核心功能在可达场景下失效且无自愈；P2 = 局部降级或误报。

| 编号 | 级别 | 现象 | 根因 | 修复 commit |
| --- | --- | --- | --- | --- |
| D-01 | P0 | 整个 Goals 功能域在桌面端不可用，命令调用落到核心 handler 后报未知命令 | `supplemental_registry.rs` 维护了两份清单：`generate_handler!` 注册了 11 个 goals 命令，`is_command()` 却一个都没列，于是全部被路由回核心 handler | `13b0738f` |
| D-02 | P1 | 5 个 CLI Agent 中有 3 个完全无法对话，失败于进程创建之前 | 启动值校验用同一条规则管参数、可执行文件、cwd 与环境变量，拒绝全部控制字符。而组合出来的 prompt 天然跨行，于是作为 argv 传递 prompt 的 Agent 一律被拒 | `d281984f` |
| D-03 | P1 | codex-cli 无法启动 | 可执行文件按**标识符**上限（128 字符）校验，而它解析出来的是 vendored npm 绝对路径 | `86e8b86c` |
| D-04 | P1 | gemini-cli 在 Windows 上必定 spawn 失败 | 它解析到 `.cmd` shim，而自 BatBadBut 加固起 `std::process::Command` 拒绝给批处理传含 CR/LF 的参数；`cmd.exe` 8191 字符命令行上限还会静默截断 | `90c7ffad` |
| D-05 | P1 | claude-code 的回复被协议噪声取代 | 解析器对未建模的结构化事件回落到 `parse_generic_line`，把信封本身当成 Agent 的话发出去。argv 里有 `--include-partial-messages`，一轮就有 8 个 `stream_event` 包裹着唯一那条 `assistant` | `0941515e` |
| D-06 | P1 | 已安装并可用的 CLI 被报为不可用 | 可用性判定让托管 SDK 先说话，SDK 未安装即否决；而执行路径根本不加载那个包 | `d948b41d` |
| D-07 | P1 | SSH 连接池在无 runtime 的线程上 `tokio::spawn`，直接 abort 整个应用 | 应用层直接调运行时 API | `dd852fd3` |
| D-08 | P1 | 任何 Skill 工具都无法启用：`set_skill_tool_trust` 返回成功，读回的 revision 仍是 `trusted=false` | `apply_trust` **没有任何生产调用点**——写好了、导出了、单测覆盖了，就是没接进 `revision_state`/`revision_states`。既有测试自己调用该 helper，因此测到了投影却测不到接线 | `535afac2` |
| D-09 | P2 | 个性化设置保存后重新加载即消失（Agent 侧仍收得到） | 五个个性化键都能解析为 mutation 并落盘，但 `AppSettings` 没有对应字段。响应缺字段，前端归一化分不清「缺失」与「未设置」，一律替换成默认值 | `39bcf278` |
| D-10 | P2 | Windows 上批处理 wrapper 存在潜在注入面 | `terminal_wrapper` 把 token 写进脚本文件供解释器读回，却只拒绝 NUL；批处理对裸换行没有任何转义手段 | `ae133547` |
| D-11 | P2 | `cargo clippy -- -D warnings` 在 main 上失败 | D-03 把 `detail` 接进生命周期日志时，参数数越过 clippy 阈值。当时的验证跑在最后一个参数加入之前，报告里记下了一次已经不成立的绿 | `775c257b` |
| D-12 | P2 | 桌面验证 harness 在句柄滞后时清理失败 | 清理无重试 | `7fd897ee` |

### 证伪的三条

| 现象 | 实际原因 |
| --- | --- |
| subagent worktree 留下管理记录残留 | **我自己的断言写错了**：`!listed.contains("subagent-")` 匹配到了仓库自身的临时目录名。改用 `--porcelain` 后不存在该缺陷 |
| antigravity-cli 会话失败 | 账号区域限制，非产品缺陷（按指示不再深入） |
| gemini-cli 报 `IneligibleTierError` | 账号档位限制。修掉 D-04 后它能到达 provider，被拒是账号层面的事 |

## 3. 新增测试

### 3.1 桌面套件（`tests/desktop/specs/`）

| Spec | 覆盖 |
| --- | --- |
| `domain-skills` / `domain-prompt-hooks` / `domain-work-board` / `domain-cli-tooling` / `domain-observability` | 五个此前从未被客户端跑过的功能域 |
| `native-flows` | Web mock 无法表达的原生链路：pip 扩展安装、真实 SSH 连接 |
| `sessions` | 单 Agent 与多 Agent 会话行为，逐 Agent 断言真实回复 |
| `ui-chat` / `ui-settings` / `ui-workspace` | **只通过 UI 控件**：一次真实发送、slash 补全、`/help`、编写器状态、文件引用；字体大小、MCP 服务器、Prompt Hook、Escape 与取消关闭；会话创建对话框、活动栏、会话页签、看板阶段流转、会话筛选 |

三个 `ui-*` spec 存在的理由是 IPC 覆盖证明不了的那一层：命令可以完美无缺，而没有按钮接到它、对话框静默吞掉提交、校验消息落在错误的字段上、「取消」照样写库——这些在命令级测试里全都通过。

### 3.2 单元 / 集成

| 用例 | 防止的回归 |
| --- | --- |
| `a_persisted_trust_decision_is_visible_in_the_revision_state_it_authorizes` | D-08。只经由 repository 读回，即命令真正走的那条路 |
| `a_saved_personalization_setting_is_readable_in_the_response_that_reports_it` | D-09。同时补全了那个自称「complete legacy contract」却漏掉五个字段的断言 |
| `draining_survives_without_an_ambient_runtime` | D-07。普通 `#[test]`，无 runtime，这正是缺陷成立的条件 |
| `validate_token` 控制字符用例 | D-10。从只测 NUL 扩到 CR、LF、ESC |

## 4. Harness 自身的缺陷（六个，全部会把产品诬告成坏的）

这一节单独列，因为它们的失败模式最危险：**报告说产品坏了，而失败截图上产品好好的**。

| 现象 | 根因 |
| --- | --- |
| 字体大小与看板阶段选择「无反应」 | `selectByAttribute` 找到 `<option>` 后点击它，而 WebView2 中闭合 select 的选项由 OS 绘制、不在页面里，点击报成功却什么也没发生。驱动日志显示 `elementClick` 返回 null，取值在整个 30s 轮询里纹丝不动 |
| 输入框「不接受输入」 | 每字符一次 Backspace 的清空方式，只要有一次按键抢在点击后光标落定之前就会少删一个，残留字符随即混进新值——12 字符的字段清出了 `Ivanehub-ui-settings-e2e` |
| 「对话框没有打开」（截图里开着） | Settings 外壳用 `hidden` 停放非当前页而非卸载，前一个失败用例留下的对话框仍在文档里，使后续计数读到 2 |
| 「对话框没有提供任何已安装 Agent」（截图里五个都在） | id 的各个 span 之间没有行盒，`getText()` 把卡片折叠成 `Claude CodeClaude Codeclaude-code`，按行精确匹配一无所获 |
| `selectOption` 抛 `object is not iterable` | `$$` 解析出的是 chainable array-like，其 `map` 不是数组方法 |
| 「对话框始终不认这是 Git 项目」 | 路径在 blur 时才被检查，而 `Tab` 没能移走焦点——失败截图上字段仍带聚焦环，下方既无 "Git 项目" 也无报错，说明 `inspectPath` 压根没执行 |

## 5. 未完成与后续

| 项 | 状态 | 说明 |
| --- | --- | --- |
| `openspec archive correct-cli-launch-and-availability-contracts` | **阻塞** | 本机 `D:\cdavid\Documents\` 下的目录改名被 OS 拦截，`openspec archive` 依赖目录改名，因此必定失败。已排除沙箱、MAX_PATH 与句柄占用 |
| claude-code 增量流式 | 决定不做 | 不是「是否有双计风险」的权衡，而是记录在案的事实：CLI 把 delta 包在 `stream_event` 里，同一轮还会发终态 `assistant` 携带完整文本。拆包会先发 `PO` 再发 `PONG`。要流式就必须同时抑制终态事件的文本，那会改变「回复」对账本与用量的定义，应当单独立项 |
| provider 拒绝是否进 registry | 决定不做 | 可用性是文件系统探测，答的是「二进制在不在」，且每次列举都会遍历全部 Agent。让它回答「provider 会不会接受这个账号」等于在该路径塞进每 Agent 一次网络往返，而答案在真正发起会话时已经过期。更合适的形态是记住上次观测到的拒绝并在 Agent 卡片上做提示，那属于会话状态而非注册表 |
| 全局作用域 Skill 逃出隔离数据目录 | 待处理 | `skills/infrastructure/filesystem/paths.rs:31` 把全局作用域根解析为 `%USERPROFILE%`/`$HOME`，不受 `VANEHUB_APP_DATA_DIR` 重定向。现有 spec 只断言「未创建」并留了 `delete_skill` 兜底，但任何未来会**创建**全局 Skill 的桌面 spec 都会污染宿主 |
| 面板折叠状态未持久化 | 待决策 | `sessionSidebarCollapsed`、`infoPanelCollapsed`、`workspaceTabsCollapsed` 都是纯 `useState`（`main-layout.tsx:86-90`），只有侧栏宽度、呈现模式与展开分组进了 localStorage。这像产品缺口而非测试缺口 |
| Enhance 按钮未接线 | 待决策 | `ButtonArea` 接受可选 `onEnhance`（`ButtonArea.tsx:124`），而 `ChatInputBox` 与 `ApiSessionComposer` 都没传。按钮照常渲染、草稿可发送时照常可点，调用的是 `undefined` |
| 清空与补全绕过 `updateSuggestions` | 待决策 | `onClear` 与 `onSelectSlashCommand` 直接 `setDraft`（`api-session-composer.tsx:116`、`:128`），补全查询只在 `onChange` 里推进。于是用 X 清掉 `/` 开头的草稿会让补全浮层停在空编写器上方 |

## 6. 环境注意事项

- **代理**：claude-code 与 codex-cli 需要出网代理。harness 只在 `HTTPS_PROXY`/`https_proxy` 以 `http` 开头时转发——Node 的 undici 不支持 `socks5://`。本机 `all_proxy=socks5://127.0.0.1:9999`，同一端口也接受 HTTP 代理请求，因此需以 `HTTPS_PROXY=http://127.0.0.1:9999` 形式运行。缺代理时 claude-code 报 `403 Request not allowed`、gemini-cli 弹认证提示，二者都是环境问题而非回归。
- **凭据**：OnePiece 的 live 用例需要 `VANEHUB_ONEPIECE_API_KEY` 或 `VANEHUB_ONEPIECE_PROFILE_ID`。两者皆无时报 BLOCKED 而非失败。本轮所用的 provider key 与 SSH 口令均只经环境变量与 OS 凭据库传入，未进入仓库；因已出现在会话记录中，建议轮换。
- **spec 间共享状态**：同一轮内所有 spec 共享一个 `VANEHUB_APP_DATA_DIR`，存在顺序耦合。`exit_application` 会让 harness 重启客户端，不调用它的 spec 则继承上一份数据目录。
