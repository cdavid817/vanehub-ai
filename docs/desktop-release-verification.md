# 桌面客户端发布验证清单

每次版本发布前，必须在真实构建的桌面客户端上跑通本清单。桌面验证分三个层级，**只有第一层是 PR/CI 的硬门槛**，后两层按下表的规则决定是否阻塞发布。层的清单以 `scripts/test-desktop.mjs` 的 `gatedLayers` / `fullSuiteLayers` 与 `tests/desktop/spec-manifest.mjs` 为准——本文不写死层数。

## 0. 三个层级

| 层级 | 入口 | 需要真实 CLI / 凭据 | 阻塞发布 | 产出证据 | 当前适用平台 |
| --- | --- | --- | --- | --- | --- |
| **1. PR/CI 必跑的 fixture 门槛** | CI 的 `desktop-smoke` job 执行 `npm run test:desktop`；在 `CI` 环境下脚本只跑 `gatedLayers`（核心冒烟与会话删除）。本地等价命令：`CI=1 npm run test:desktop` | 否。CLI Agent 由 `tests/desktop/fixtures/cli/` 的桩程序顶替，不发生模型调用、不读凭据；`scripts/desktop/cli-side-effect-guard.mjs` 反查是否碰过真实 npm/WinGet/凭据 | **是**——任一平台 `FAILED` 即不得打 tag | `test-results/desktop/<runId>/`（`summary.json`、统一日志、截图），CI 按平台上传 artifact | Windows、macOS、Linux 原生 runner 各跑一次，逐平台报告 |
| **2. 可选的完整 fixture 套件** | `npm run test:desktop`（非 CI 环境默认）或 `VANEHUB_DESKTOP_FULL_SUITE=1`；CI 里由 `desktop-full` job 在带 `desktop-full-suite` 标签或手动触发时运行，跑 `fullSuiteLayers` 全部层 | 否，同上 | 发布前**必须至少在一个平台跑通**；其它平台可豁免，但豁免必须写进发布记录并附原因 | 同上，逐层独立汇总 | 与第 1 层相同 |
| **3. 外部提供商实网资格验证** | `npm run test:desktop:external-provider`（`tests/desktop/wdio.external-provider.conf.mjs`，spec 由 `spec-manifest.mjs` 的 `EXTERNAL_PROVIDER` 门标记）；以及 `test:desktop:agent-evaluation:live-opencode` / `live-onepiece`；CI 只在 `desktop-external-provider` job 手动或定时触发时运行 | **是**。需要已登录的真实 CLI（`VANEHUB_DESKTOP_LIVE_AGENTS`）、OnePiece provider 凭据、可选 SSH（`VANEHUB_DESKTOP_MUTATE_HOST` + `VANEHUB_SSH_*`） | **否**。它不是发布门槛，而是"live-qualified"状态的证据来源：没有跑过的能力只能标为 fixture-qualified | 同上，另加逐 Agent 的 `PASSED` / `PASSED WITH BLOCKED` / `BLOCKED` / `FAILED` / `NOT RUN` 记录（平台、CLI 版本、日期） | 只对实际运行的平台成立，不得外推 |

`AGENTS.md` 里列出的 `agent-mcp`、`local-media`、`skills`、`feishu-im` 等"可单独运行的额外 suite"是第 2 层之外的独立层：它们各有 `test:desktop:<层名>` 入口，`agent-mcp` 已进入 `fullSuiteLayers`，其余不属于 `all`，需要时单独运行并单独报告。

第 1、2 层的执行方式是：构建带 `desktop-e2e` feature 的调试客户端，再用 WDIO + tauri-driver 驱动它。`npm run test:verify` 已经编排了 `npm run test:desktop`，所以走完整验证流程时无需单独执行。

## 1. 结果状态语义（先读这一节）

单层运行的汇总行有三种可能：

| 汇总 | 含义 |
| --- | --- |
| `PASSED` | 执行的用例全部通过，且本轮没有任何用例因前置缺失被跳过 |
| `PASSED WITH BLOCKED (n skipped — see BLOCKED above)` | 执行的用例通过，但有 n 个用例因前置缺失被跳过——**绿色不等于覆盖完整** |
| `FAILED` / `BLOCKED` | 有用例失败，或该层整体无法启动（缺构建产物、npm 入口等） |

发布记录必须逐平台使用五档：`PASSED`、`PASSED WITH BLOCKED`、`BLOCKED`、`FAILED`、`NOT RUN`。**必需场景（下节标注）被跳过时，该平台不得记为 `PASSED`**；记 `PASSED WITH BLOCKED` 并由发布负责人逐条签署豁免或补齐前置后重跑。脚本退出码只反映"已执行部分是否通过"，不代表发布要求全部满足。

发布必需（release-blocking）场景是第 1 层的全部用例，以及第 2 层里运行时基线（`smoke`）、原生注册表与 Agent 终端生命周期（`feature-sweep` 的非实网部分）、全屏渲染（`screen-sweep`）。**真实 CLI 与 OnePiece 的实网会话（`sessions` 的实网部分）、SSH、IM、扩展安装以及任何外部 CLI 的实网会话都属于第 3 层，不是发布门槛**——它们决定某个能力能否标为 live-qualified，未跑过时该能力保持 fixture-qualified，并在发布记录里如实写明。

## 2. 前置条件（不满足会以 BLOCKED 跳过）

| 能力 | 需要的前置 | 缺失后果 |
| --- | --- | --- |
| CLI Agent 会话与终端（第 3 层） | 被验证的 CLI 在 PATH 上且已登录（读各自的 `~/.claude`、`~/.codex` 等，不受测试隔离目录影响），并通过 `VANEHUB_DESKTOP_LIVE_AGENTS` 指名；第 1、2 层不需要真实 CLI，用 `tests/desktop/fixtures/cli/` 的桩程序 | 相关用例跳过 |
| 需要出网代理的 CLI | `HTTPS_PROXY` / `HTTP_PROXY` | claude-code 报 `403 Request not allowed` |
| OnePiece 实网会话 | `VANEHUB_ONEPIECE_API_KEY`，或 `VANEHUB_ONEPIECE_PROFILE_ID`（指向本机已安装应用中的 profile，密钥由 WDIO 进程直接从操作系统凭据管理器读取） | 实网用例跳过 |
| SSH 连接 | `VANEHUB_SSH_HOST`、`VANEHUB_SSH_USER`、`VANEHUB_SSH_PASSWORD` | SSH 用例跳过 |
| 扩展安装 | 可用的 `python` + `pip`，且 pip 索引可达 | 安装用例跳过 |

OnePiece 需要一个模型提供商（provider）才能跑实网会话。本仓库验证时使用的形状（**密钥不写进仓库**，只放环境变量或操作系统凭据管理器）：

| 字段 | 值 |
| --- | --- |
| provider id | `deepseek` |
| endpoint type | `openai-chat-completions` |
| base URL | `https://api.deepseek.com/v1` |
| 可用模型 | `deepseek-v4-pro`、`deepseek-v4-flash` |

远程 provider 必须走预设目录（`save_onepiece_provider_profile`），不能用 `save_custom_onepiece_provider_profile`——后者刻意只接受 local/private 运行时。

**代理必须写 `http://` 形式。** `claude` 底层是 Node undici，它不支持 SOCKS——给 `socks5://` 会让请求无限重试，最终表现为认证失败而不是代理配置错误，极易误判。若本机代理同时提供 SOCKS5 与 HTTP CONNECT（常见于 clash/v2ray），用 HTTP 端口。

回环必须绕过代理，否则本地模型端点探测会被代理接管，把真实检查变成假结果：

```bash
env HTTPS_PROXY=http://127.0.0.1:<port> \
    HTTP_PROXY=http://127.0.0.1:<port> \
    NO_PROXY=127.0.0.1,localhost \
    VANEHUB_ONEPIECE_PROFILE_ID=<profile-id> \
    VANEHUB_SSH_HOST=<host> VANEHUB_SSH_USER=<user> VANEHUB_SSH_PASSWORD=<password> \
    npm run test:desktop
```

凭据只从环境变量读取，不得写入仓库内任何文件。

## 3. 测试点矩阵

规格文件在 `tests/desktop/specs/`，每个 spec 独立启动一次客户端。

### `smoke.e2e.mjs` — 运行时基线（发布必需）

真实运行时启动、React bootstrap 就绪、跨 IPC 调用、本地模型端点发现与验证、代码评审打开与逐块回退、Agent Run 生命周期与 Mission Control 投影、评测导出、原生 WebView 导航、干净退出。

### `feature-sweep.e2e.mjs` — Web 侧无法覆盖的原生路径（非实网部分发布必需）

- 全部内置 CLI Agent 均出现在原生注册表且返回合法可用性状态
- Agent 终端全生命周期：open → 真实 PTY 输出 → input → resize → stop → 二次 stop 为空操作
- OnePiece 经实网 provider 收发消息
- 会话、消息、用量统计在原生数据库中留存

### `screen-sweep.e2e.mjs` — 全屏渲染（发布必需）

逐屏截图工作区目的地、设置页与会话工作区标签页，存 `test-results/desktop/<runId>/screens/`。清单**由 `WorkspaceDestination`、`SettingsPageId`、`SessionTabId` 三个联合类型派生**——屏幕数量以这三个类型为准，新增屏幕时同步更新 spec 顶部常量，不要在文档里手工维护数字。

渲染断言会拒绝 `Command X not found`，不只是致命错误边界——**一个被妥善处理的错误渲染出的页面看起来完全健康**，「目标」功能整体不可用就是这样被漏过又被抓到的。

### `sessions.e2e.mjs` — 单 Agent 与多 Agent 会话（实网部分属第 3 层，不阻塞发布）

对被指名的 Agent 发真实一轮对话并断言回复。**两种 prompt 投递形态都必须覆盖**：claude-code、codex-cli 走 stdin，gemini-cli、opencode、antigravity-cli 走 argv，是两条不同代码路径。只测其中一条会把「其中一条路径上的 Agent 发不出对话」报成健康。六个 ACP Agent 走第三条路径（ACP stdio 受管会话），其实网验证记录见 `openspec/changes/extend-cli-providers-with-acp/tasks.md`。

多 Agent 部分覆盖入座、重载存活、离座。注意离座是 `leftAt` 墓碑而非删除，「房间里有谁」要按活跃座位过滤。

### `native-flows.e2e.mjs` — Tauri 与 Web 实现最易分叉处（可豁免项逐条签署）

- MCP：add → 对仓库自带 stdio fixture 的真实握手 → status → toggle → update → export → remove
- 权限策略模板下发与回读
- CLI 检测与 `install_cli_version` 真实 npm 安装
- 扩展框架经 pip 安装与卸载
- SSH 连接创建、主机密钥确认、真实连通测试、删除，并断言存储记录不回显密码

安装类用例刻意重装当前版本、装完即卸：驱动完整管线，但让宿主机回到初始状态。**CLI 安装范围限定 opencode**，claude-code 与 codex-cli 不得被卸载或写入配置。

## 4. 已知缺陷（BLOCKED 不等于环境问题）

每条缺陷必须带状态与核对信息；已修复的条目保留一个发布周期后删除。状态含义：**已修复**＝当前 HEAD 源码已消除根因；**待真实环境复核**＝源码层面已重做或无法静态判断，需在真实桌面运行中确认；**未修复**＝根因仍在。

| 现象 | 根因 | 状态 | 核对 |
| --- | --- | --- | --- |
| opencode、gemini-cli、antigravity-cli 会话 `runner_invalid_launch` | `RunnerLaunchSpec::validate` 曾拒绝一切控制字符，多行 prompt 走 argv 的 Agent 无法启动 | **已修复**——校验已放行 Tab、CR、LF，其余控制字符仍拒绝（`application/runner.rs` 的 `validate_value` 及其注释） | 2026-09-04 静态核对 `dev@6f3da9fa`；多 Agent 桌面套件已在真实 CLI（claude/codex/opencode）上通过 |
| codex-cli 报不可用（二进制装好也无法建会话） | 可用性闸门曾先查受管 SDK、缺失即判不可用且不再查 PATH | **待真实环境复核**——可用性判定已重做为按启动路径解析可执行文件并回退 PATH（`infrastructure/availability.rs` 注释记录了该缺陷与修复动机） | 2026-09-04 静态核对 `dev@6f3da9fa`；需真实桌面运行确认 |
| antigravity-cli 会话无响应且 `lifecycle=running` | 进程启动后挂住，疑似非交互环境下 keyring 超时 | **待真实环境复核**——环境相关，无法静态判断 | 最后复现记录早于 2026-09-04，未在当前 HEAD 重跑 |
| SSH 连接测试导致客户端进程消失 | 未定位。原生日志无任何 SSH 记录，符合硬崩而非错误返回 | **未修复（未定位）**——SSH 属可豁免场景，但豁免必须写入发布记录；README 与用户指南的远程工作区章节不得宣称该路径已通过发布验证 | 最后复现记录早于 2026-09-04，未在当前 HEAD 重跑 |

## 5. 结果判读

- 逐条结果看 WDIO reporter 的 `✓` / `✖` / `-`，`-` 是跳过。
- 每轮结束会打印 `BLOCKED on this host:` 清单，逐条说明为何跳过；汇总行在存在跳过时显示 `PASSED WITH BLOCKED`（见第 1 节）。
- 证据在 `test-results/desktop/<runId>/`：`summary.json`、`logs/native/vanehub.log`（统一日志）、`screens/`（全部屏幕截图）、失败截图。
- 规格文件之间共享同一个隔离数据目录，因此存在顺序耦合。新增 spec 必须自行清理：恢复工作区路由（应用重启会恢复上次目的地）、删除自己创建的 provider profile（活跃 profile 会决定下一个 spec 看到哪个）。
- 本机结果不可外推到其他平台。CI 的 `Desktop Smoke` 在 Windows、macOS、Linux 原生 runner 上各跑一次，报告时逐平台使用第 1 节的五档状态。
