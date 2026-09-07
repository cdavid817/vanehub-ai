# 实施记录

状态：代码实现与自动化验证已完成（2026-09-06）；真实 CLI live smoke 与三平台桌面验收未执行，见 [verification/results.md](verification/results.md)。本文件记录实际落点、与既有实现的整合方式、架构决策与已知限制，不宣称未验证的内容。

## 当前工作树

- HEAD / 分支：`34a449e80a1badc16fcfb49dcbad26134aecf2d9`，分支 `feat/cli-supplement`（worktree `.claude/worktrees/cli-supplement`，自本地 `main` 创建，与审阅基线 `main@34a449e` 相同）。
- 已有未提交修改：开工时 `git status` 干净；本次所有改动均未提交，全部保留在工作树。
- 与审阅基线差异：无（HEAD 即基线）。`AGENTS.md`、`openspec/project.md`、`config.yaml` 按当前版本执行。
- 重叠 change：`openspec/changes/` 下无其他未归档 change 触及 provider registry、tooling::cli 目录、权限投影或会话绑定；未发现同名抽象。既有 `archive/` 未编辑。
- OpenSpec CLI：1.9.0。`openspec validate extend-cli-providers-with-acp --strict` 与 `openspec validate --specs --strict` 均通过；delta specs 中 MODIFIED 块保留原 requirement/scenario 标题。

## 架构和依赖决定

| 决定 | 结果 | 理由 |
| --- | --- | --- |
| Transport 建模 | `ProviderTransport { Terminal, Headless, AcpStdio }` 加在既有 `ProviderCapabilities` 上；tooling 目录镜像为 `CliManagedTransport`，一致性测试 `the_tooling_catalog_and_the_runtime_agree_on_transport_and_lifecycle` 断言两者一致 | 不新建平行 registry；管理页无需越层读运行时 |
| ACP 实现 | 自研 Rust 子集（`providers/acp/`），未引入 `agent-client-protocol` crate | 运行时端口是线程化的 `Send + Sync` trait，官方 crate 以 async/LocalSet 为中心；有界预算（1 MiB 帧、64 层深度、1,024 入站队列、128 待决请求、64 KiB stderr 尾部）需在结构上强制 |
| 路由 | `CompositeAgentProcessGateway` 三路分发：`api` → API 适配器，`managed_transport() == AcpStdio` → `AcpAgentProcessAdapter`，其余 → headless 适配器；`CompositeToolApprovalPort` 按 process id 前缀（`acp:`）转发审批 | 应用层不按 provider id 分支 |
| Manifest V2 | data-only，新增 `transports`、`lifecycle`，`usage` 三态（`true/false/"unavailable"`）；V1 文档原样解析 | 原五种保持 V1 精确文档，manifest 回归测试不变 |
| 持久化 | 迁移 112 `cli-execution-bindings`：新表 `cli_execution_bindings`（provider、distribution、installation fingerprint、transport、profile、workspace、exact external session id、connection epoch） | 旧会话无绑定行，继续旧路径；删除会话先 `release_managed_connections` 再删绑定，不碰 CLI 全局数据 |
| 权限 API | 复用 `permissions::api::{Action, Resource, Effect}` 与 `ToolApprovalPort`；ACP 待决交互进入 `PendingInteractionStore`，以 (session, turn, epoch, RPC id, tool call, policy revision) 定域、一次消费 | 不新增第二套审批状态机；关窗/超时（30 分钟）/断线/取消一律拒绝 |
| 参数目录 | `catalog.v2.json` 新增七家（仅 user-editable：model 全部；Qoder `--thinking` 枚举、Kimi/Qoder/CodeBuddy `--agent`、CodeBuddy/Copilot `--effort` 枚举、Qwen `--safe-mode`/`--bare`、iFlow `--thinking`），audit 记录引用本机安装版本的 `--help` 与官方页面；`MANAGED_CLI_AGENT_IDS` 扩为 12，legacy baseline 仍只覆盖原五家。策略旗标不进目录：`RUNTIME_POLICY_AGENT_IDS` 的七家由 `direct_policy_arguments(scope)` 在解析后前置（ACP 只传只读旗标） | 用户授权安装后有了真实 binary 版本证据；权限旗标按传输区分，目录值表达不了 |
| 安装来源 | Qwen/Kimi/Qoder/CodeBuddy 仅 npm；Copilot npm + WinGet `GitHub.Copilot`；Cursor reviewed vendor installer（`cursor.com/install`、Windows `install?win32=true`，下载到有界文件审查后执行，不 pipe-to-shell）；iFlow `local` detect-only | 未确认内部切源语义的厂商脚本不接成自动操作 |
| Cursor 身份 | `CliIdentityRule::Reviewed { canonical_path_markers: ["cursor-agent","cursor"], version_output_markers: ["cursor"] }`，不匹配 → `IdentityMismatch` 且不启动 | `agent` 是通用 basename |
| CodeBuddy 环境 | 三个 profile：international（无变量）、china（`CODEBUDDY_INTERNET_ENVIRONMENT=internal`）、ioa（`=ioa`）；按会话 provider_id（`codebuddy-china`/`codebuddy-ioa`）选择，随绑定持久化 | 环境变量不是凭据，与 UI 语言无关 |
| Copilot 进程 | 每个绑定独立进程；不同策略不复用连接 | 官方 ACP server 在启动时固定 tools/reasoning |
| 策略旗标（按真实 `--help` 核验） | `direct_policy_arguments(agent, template, scope)`：终端按 provider-matrix §3a 投影全部四个模板；ACP 只传只读旗标。Qoder 只读无对应模式 → 终端拒绝（`terminal-readonly-unsupported`）；其余全部 HostProjected（Copilot/Cursor 的标准模板为 ProviderDelegated 询问默认） | 首版依据文档写的 `auto_edit` 与「Kimi 无只读模式」在安装真实程序后被 `--help` 推翻并修正 |
| iFlow | `ProviderLifecycle::Legacy { service_shutdown: "2026-04-17" }`，transports 仅 `Terminal`；headless 与 ACP 网关均拒绝；前端 `legacy` capability tag 进入创建会话对话框的「历史兼容」分组，默认隐藏、点击显示、永不作为默认选择 | 不声称官方服务，不安装，不迁移配置 |
| 缺认证归类 | `AcpError::AuthRequired`（JSON-RPC `-32000`，ACP `auth_required`）→ reason code `acp-authentication-required`；适配器在 `session/new`/`session/load` 上把它报为「{agent} 未登录，请在终端用 CLI 登录后重试」，不启动登录 | 真实程序 live 探测显示六家中五家以该码拒绝未登录的 `session/new`；此前会被误报为协议违规 |
| 富文本块 | ACP 通知（stop reason、中断、无人值守拒绝、计划、可用命令、模式切换、厂商通知、子 agent 文本、非文本内容）统一以前端 `card` 形状（`id/kind/v/title/bodyMarkdown/tone/fields/meta`）落库，机器可读字段在 `meta` | 复用既有 `RichBlocks` 渲染器，不新增前端块类型 |

## 各阶段实际成果

### 公共运行时（tasks 1、3、4、10）

- `contexts/agent_runtime/domain/provider.rs`：`ProviderTransport`、`Unavailable` usage、transports 能力；`application/provider.rs`：`ProviderAcpInvocationRequest/Spec`、默认 `prepare_acp`；`application/ports.rs`：`ManagedConnectionControlPort`（release/shutdown）。
- `infrastructure/providers/definitions.rs`：12 条 provider 定义（原五 + 七），ACP 入口 grammar、阻塞/通知扩展、账号 profile、manifest 生成。
- `infrastructure/providers/acp/`：`framing`（UTF-8 NDJSON、有界）、`jsonrpc`（分类与 typed error）、`connection`（子进程、独立 reader、单 writer、请求关联、迟到响应丢弃、epoch）、`session`（initialize/协商、session/new、支持时 session/load 回放计数丢弃、prompt 驱动、stopReason、未知请求 method-not-found、未知通知忽略、cancel + 2 秒宽限 + 自有进程树回收）、`handlers`（request_permission → 策略 Allow/Deny/Ask、fs/terminal 代理、Cursor ask_question/create_plan）、`interactions`（定域一次消费、超时拒绝）、`proxy_fs`（canonical path、授权根、8 MiB 上限、写入走权限）、`proxy_terminal`（受治理执行器、每会话 8 个、256 KiB 输出、跨会话拒绝）、`environment`（凭据环境清洗）、`binding`（SQLite 绑定）、`adapter`（`AgentProcessGateway` 实现、连接池、unattended 策略）、`blocks`（前端 card 形状）。
- `infrastructure/composite_process_gateway.rs`：三路路由与审批端口；`bootstrap/agent_runtime.rs` 接线；`commands/sessions/delete_session.rs` 删除前释放连接。
- `platform/database/migrations/mod.rs`：迁移 112。

### 目录、安装身份与认证（task 2）

- `tooling/cli/domain/registry.rs`：七条目录项、`SOURCE_LOCAL`、`CURSOR_INSTALLER`、`QODER_COMPATIBILITY`（排除 Windows aarch64）、`CURSOR_IDENTITY`；`definition.rs`：`CliToolLifecycle`、`CliIdentityRule`、`CliManagedTransport`；`source.rs`：`Uv` 来源种类（Kimi 旧发行形态识别）；`status.rs`：`IdentityMismatch`；`environment_discovery.rs`：`classify_source_with_target`。
- 认证探针：七家均 `Undocumented` → 状态 `unknown`；设置页检测只跑 `--version`，不启动 ACP/登录/模型调用。
- 显式操作（2.5）：`check_cli_connection` 命令 → `AgentRuntimeApi::check_managed_connection` → `ManagedConnectionControlPort::check_connection`（ACP 适配器实现：解析可执行文件、经 runner 权限边界、启动、仅 `initialize`、终止进程、返回脱敏协商摘要；非 ACP/legacy 拒绝，协议版本不符报错）。目录新增 `login_docs_url`（七家为官方文档 HTTPS 链接，原五家 `None`），DTO/契约 `loginDocsUrl`；前端 `cli-connection-actions.tsx` 提供「检查连接」与「登录说明」，后者经 `openExternalUrl`（仅 HTTPS）在点击时打开。Web/mock 对 ACP 且已安装的工具返回固定摘要，其余拒绝。
- DTO/契约：`commands/tooling/cli_environment/{dto,mapper}.rs` 与 `src/contracts/cli-environment-snapshot.ts`、`src/types/cli-environment-snapshot.ts` 新增 `lifecycle`、`legacyServiceShutdown`、`managedTransport`。

### 逐家（tasks 5–9）

| Provider | ACP 入口 | 终端 | 特有实现 |
| --- | --- | --- | --- |
| qwen-code | `qwen --acp` | `qwen`（`--resume/--session-id`） | 独立 grammar，不继承 Gemini 结论 |
| kimi-cli | `kimi acp` | `kimi`（`--session`） | npm/uv 发行形态区分；`--plan`/`--yolo`/`--auto` 三档 |
| qoder-cli | `qoder --acp` | `qoder`（别名 `qodercli`；`--resume/--session-id`） | Windows arm64 排除；无只读模式 |
| codebuddy-code | `codebuddy --acp` | `codebuddy` | 三个账号 profile；`_meta` 内 `codebuddy.ai/memberEvent|teamUpdate` 子事件归属同一 seat（`provider_subagent_provenance`） |
| copilot-cli | `copilot --acp --stdio` | `copilot` | 每绑定独立进程；npm + WinGet |
| cursor-agent-cli | `agent acp` | `agent` | 身份校验；`cursor/ask_question`、`cursor/create_plan` 阻塞应答；`cursor/update_todos`、`cursor/task`、`cursor/generate_image` 通知投影 |
| iflow-cli | 不声明 | `iflow` | legacy、detect-only、自动化拒绝 |

### 前端（task 11）

- 服务层：`mock-agent-data.ts`（七条 mock 注册项）、`chat-configuration.ts`、`model-family.ts`、`web-cli-environment-snapshots.json`（七条快照）；Tauri 与 Web/mock 同步。
- 创建会话：`agent-display-order.ts`（顺序）、`create-session-agents.ts`（`legacy` 分组、默认不选 legacy）、`create-session-agent-section.tsx`（历史兼容 opt-in 按钮、legacy 提示）。
- CLI 管理页：`cli-status-badges.tsx`、`cli-environment-card.tsx`、`cli-overview-tab.tsx`（lifecycle/transport 徽章、关闭日期提示）。
- 策略页 `agent-policies-page.tsx`、`types/permissions.ts`：新 provider 纳入。
- i18n：en/zh-CN/zh-TW/ja/ko 全部补齐；`agent-visual-identity.ts` 使用中性图标（无可嵌入的厂商标志）。

### 多 Agent / 自动化（task 12）

- `domain/seat_roster.rs`：七家 model family 为 `Unknown`；`chat_profile.rs`：默认 provider/model。
- unattended：`AcpAgentProcessAdapter` 按 `request.interactive` 决定；非交互运行的权限请求以拒绝选项应答并落 `acp_unattended_rejection` 卡片，不自动批准。
- legacy provider 的 headless/ACP 生成在能力协商处失败（`UnsupportedCapability`），自动化与多 seat 无法使用 iFlow。
- 隔离与漂移测试（12.4）：`two_sessions_run_concurrently_on_isolated_processes`（两个会话两条进程、输出互不串、按会话单独释放）与 `installation_drift_between_turns_retires_the_binding_and_refuses_resume`（可执行文件变化后池中进程被退役，持久化绑定以 `binding-installation-changed` 拒绝恢复，不发 `session/load`、不重放 prompt，可显式开新会话）。

### 文档（task 14）

- README（en/zh-CN/ja）七行 + ACP 说明；用户指南新章 `acp-cli-agents.md`（en/zh-CN）及 core-concepts/index/getting-started 更新；开发指南新章 `acp-runtime.md`（en/zh-CN）、`runtime-boundaries.md` 修正「本项目未实现 ACP」的旧说法、`cli-lifecycle.md` 目录表；`docs/provider-sdk/*` 新增 Schema 2、transport、安全规则。

### 真实程序暴露的运行时缺陷（已修）

- `acp/proxy_terminal.rs`：等待子进程退出的线程在持有 child 互斥锁的情况下阻塞等待 50 ms 后立刻再取锁，`kill`/`release` 因此饥饿数十秒（本机观测 19–29 s），`sleep 30` 得以自然结束并报 exit 0。改为锁内只做非阻塞 `try_wait`、锁外休眠；kill 用例从 30 s 降到 0.5 s，并改为按耗时断言。

- 顺带观察（未改，超出本变更范围）：`infrastructure/local_runner.rs` 的退出等待线程采用同一「持锁阻塞 25 ms 后立刻再取锁」模式，取消路径可能遭遇同类饥饿。

### 全量 smoke 暴露的两处既有持久化缺陷（已修，与本变更无直接关系）

- `tooling/skill_tools/infrastructure/sqlite_repository.rs::save_trust`：信任写入用延迟事务「先读 revision 行、再写 trust 行」。当另一连接（每次 Skill 变更后触发的 registry refresh）恰好持有写锁时，SQLite 拒绝共享锁升级并立即返回 `SQLITE_BUSY`（不等待 `busy_timeout`），信任决定以 `storage` 错误丢失；同进程内的 spec 重试因 revision 已 `valid` 而在更早的断言处失败，掩盖了首因。原生日志 `skill-tool-trust outcome=storage` 与同一毫秒的 `skill-tool-registry-refresh` 印证。改为 `TransactionBehavior::Immediate`，与仓库其他读后写事务一致。

- `tooling/cli_parameters/infrastructure/sqlite_profile_repository.rs::{replace_if_revision, reset_if_revision}`：同一模式。修复上一条后复跑全量 smoke，`domain-cli-tooling` 的 opencode 参数保存/重置用例在两轮各三次重试中分别于 `save_cli_parameter_profile` 与 `reset_cli_parameter_profile` 失败，单独运行 6/6 通过；同一轮原生日志里 `execution_observability.retention` 每十几秒记一次「deferred after a storage error」，宿主内存与负载正常，说明是 WAL 下延迟事务读后升级写锁遇到并发提交即刻 `BUSY_SNAPSHOT`，而非资源不足。两处改为 `TransactionBehavior::Immediate`。

- 顺带观察（未改，超出本变更范围）：仓库内仍有约 110 处 `connection.transaction()` 延迟事务，其中读后写的形态在后台写入（registry refresh、retention、scheduled maintenance）并发时都可能以同样方式失败；已另起变更 `harden-sqlite-write-transactions`(含全仓事务点审计清单)统一改为 Immediate 并加架构守护。

### 参数目录扩到十二家后的一处布局修正

- `src/settings/cli-parameters/cli-parameter-rail.tsx`：Agent 导航栏此前在所有断点都 `sticky`；十二条目在窄视口（390 px）下叠在字段上方并随滚动盖住「启动范围」切换，Playwright 的窄视口用例点击被拦截。改为仅在并排布局（`lg:`）下 sticky，并限高滚动。

### 桌面层暴露的两处修正

- `src/evaluation-center/evaluation-center.tsx`：注册表超过 8 个 Agent 后，默认全选会被后端 `MAX_ARENA_ATTEMPTS = 8` 拒绝。现在默认预选不超过 8 个，超过时禁用「运行竞技场」并显示原因（`evaluation.tooManyAgents`）；`tests/desktop/specs/ui-evaluation.e2e.mjs` 改为恢复初始选择而不是勾选全部。
- `src/session-workspace/agent-terminal-tab.tsx`：对已退出/已释放终端的 resize 会以「Agent terminal is not connected」拒绝，此前作为未处理 rejection 触发致命错误边界（桌面 screen sweep 在 sessions 与设置页两处命中）。resize 的拒绝现在被吞掉，输入的拒绝仍会浮现。

- `tests/desktop/specs-session-workspace/session-workspace.e2e.mjs`：deep-tree 搜索用例改用共享的 `clickWorkspaceTab()` 点击「文件」标签（会话创建后的激活重渲染会让原始句柄永不可点；本文件其余用例本就如此）。`tests/desktop/specs/ui-evaluation.e2e.mjs`：恢复初始选择而不是勾选全部 Agent。

### 因目录扩大而更新期望的既有测试

`tests/e2e/cli-management-settings.spec.ts`（卡片 5 → 12、冲突/来源/需处理集合）、`tests/desktop/specs-cli-management/cli-lifecycle.e2e.mjs`（快照 5 → 12，七个新工具在 fixture PATH 上必须 `not-found`）、`tests/e2e/evaluation-center.spec.ts` 与 `src/evaluation-center/evaluation-center.test.tsx`（mock arena 8 个 agent 上限，改为取消勾选七个新 agent）、`tooling/cli/application/environment_{refresh,service}_tests.rs`、`agent_runtime/infrastructure/{schema,tests}.rs`、`platform/database/mod.rs`、`skills/infrastructure/sqlite_repository.rs`（种子/刷新计数 5/6 → 12/13）、`tests/architecture.rs` 与 `scripts/architecture/frontend-rules.mjs`（行数预算按实测上调并写明理由）。未删除任何测试，未放宽任何断言语义。

## 上游差异与功能限制

| 项 | 类型 | 说明 |
| --- | --- | --- |
| 参数目录仅覆盖已核验的少量参数 | 审查范围 | 七家各 1–3 个 user-editable 参数；未纳入的旗标（如 Qwen `--extensions`、Copilot `--allow-tool`）需要下一次审查 |
| 认证状态恒为 unknown | 上游无稳定只读探针 | 显式登录仍在终端；不猜测已登录 |
| usage/费用 | 上游能力 | ACP 不上报，UI 显示不可用 |
| session/load | 握手决定 | 对端未声明 `loadSession` 时显式新会话 |
| Kimi 旧 uv 发行形态 | 宿主策略 | 只检测报告，不迁移；旧形态未做兼容承诺 |
| Qwen/Kimi/Qoder/CodeBuddy 厂商脚本 | 未确认内部切源 | 仅 npm 受管，脚本作为说明 |
| Cursor 安装器完整性 | 上游无签名 | `ArtifactIntegrity::Unverified`，下载后审查披露 |
| CodeBuddy 内部 Team 扩展 | 部分 | 子事件归属实现；未支持的 Team 扩展方法按未知请求返回 method-not-found |
| 版本基线 | 已安装并握手 | 用户授权后本机安装七家真实程序，六家 ACP 握手全部成功（protocol 1，均声明 loadSession）；版本与 authMethods 见 provider-matrix §4。CodeBuddy 把账号环境作为 authMethods 暴露（`iOA/external/internal/selfhosted`），与本实现的 profile 环境变量并行，未据此改动 |

## 验证与阻塞

命令、退出码与证据见 [verification/results.md](verification/results.md)。阻塞项：七家 CLI 已按用户授权安装并完成真实握手；登录/付费调用仍未授权 → 文本/工具/审批/取消/恢复的 live 步骤 BLOCKED；`npm run test:desktop` 需要构建并启动桌面客户端，本机仅 Linux x86_64，其他平台 NOT RUN。

### 第三方模型端点的 live 验证（2026-09-07，用户授权）

- 用户提供 DeepSeek 的 OpenAI 兼容端点与密钥并授权验证。Qwen Code 原生支持 `--auth-type openai` + `OPENAI_API_KEY` / `OPENAI_BASE_URL` / `OPENAI_MODEL`，直连一轮 14 s 返回 `OK`；经 VaneHub ACP 适配器（新增 opt-in 测试 `live_prompt_turn_completes_through_the_acp_adapter`）一轮 9.35 s、23 个事件、`Completed`。用户随后授权继续：新增 opt-in 测试 `live_tool_approval_and_cancel_through_the_acp_adapter`（审批一次后写入落地、Deny 不问不写、取消报 `cancelled` 且同连接续用）与 `live_resume_across_a_host_restart_through_the_acp_adapter`（`shutdown_all` 后新适配器经 `session/load` 恢复上下文）均 PASSED。模拟重启必须用 `shutdown_all` + 新实例，`release_session` 语义是删会话、连持久化绑定一起删。凭据只经进程环境变量传入，`child_environment` 对 qwen-code 保留 `OPENAI_*`，未写任何 CLI 全局配置，未记录到日志或文档。
- 其余六家：Kimi、Qoder、CodeBuddy、Copilot 的 CLI 不提供第三方端点配置；Cursor 仅企业服务账号端点；iFlow 需要 `~/.iflow/settings.json` 指定 `selectedAuthType: openai-compatible`，密钥可走 `IFLOW_API_KEY` / `IFLOW_BASE_URL` / `IFLOW_MODEL_NAME`（环境变量单独使用会落到已废弃的旧认证方式）；按此直连一轮 28 s 返回 `OK`，settings 已恢复原样。iFlow 在应用内仍为 detect-only。
- VaneHub 的「CLI 全局配置」（`cli_config`，第三方端点/密钥 profile）当时只覆盖原五家；Qwen Code 与 iFlow 的 profile kind 已由后续变更 `add-qwen-iflow-config-profiles` 补齐。

### 启动开发客户端时暴露的一处既有桌面缺陷（2026-09-07 本分支先修，2026-09-08 撤回改用 main 的修法）

- `tooling/cli_parameters/domain/definition.rs` 的 `CliParameterDependencies` 用 `skip_serializing_if` 省略空数组，桌面端 DTO 对没有依赖的参数（几乎全部）发出 `dependencies: {}`；前端契约把 `requiresAll` 定为数组并在 `view-model.ts::unmetDependencies` 直接索引，CLI 参数设置页因此在桌面客户端整页崩到错误边界（Web/mock 走 zod 默认值不受影响，桌面层没有 spec 覆盖该页，所以 PR #210 引入后一直未暴露）。改为两个数组始终序列化，生成器 `scripts/generate-cli-parameter-catalog.mjs` 同步始终输出，`src/generated/cli-parameter-catalog.json` 与参数矩阵文档重生成；原生「生成契约与注册表一致」测试与 `contracts:check` 均通过；运行中的 dev 客户端重建后不再报错。
- 2026-09-08 二次合并 main 时发现 PR #287（`fix-cli-parameters-native-payload-normalization`）已在桌面适配器用 registry 的 Zod 归一化修了同一缺陷，并以回归测试钉住「目录省略空数组」这一线上格式。两处同修会让该测试失败，且线上格式的取舍以 main 为准，因此本分支的原生侧修正（`definition.rs` 始终序列化两个数组、生成器同步、目录与矩阵重生成）整体 revert，保留 main 的适配器归一化。

### 代码审查修正（2026-09-07，用户要求的全量复审）

- `acp/connection.rs::terminate`：原顺序先取 writer 锁再杀子进程。若代理停止读 stdin 而宿主一次写入正阻塞在满管道上，writer 锁被占住，终止就会等在它要终止的进程上。改为先终止进程树、再收回 writer：管道断裂让阻塞写立即失败并释放锁。
- `acp/session.rs::dispatch_request`：宿主自造的待审批 id（`acp-fs-write-<路径哈希>` / `acp-terminal-<命令哈希>`）对同一目标稳定不变，代理若在第一个写请求未决时再发同路径写请求，第二次注册会静默覆盖第一次，第一个请求永远得不到应答直到取消。改为 id 已在待决表中时追加本轮序号；新增回归测试 `two_pending_writes_for_the_same_path_get_distinct_ids_and_both_land`。
- `acp/proxy_terminal.rs::wait_for_exit` + `handlers.rs`：`terminal/wait_for_exit` 会让驱动线程最多阻塞 60 秒，其间取消标志无人检查，用户取消要等命令自己结束。`HandlerContext` 现携带本轮 `cancel` 标志，等待循环命中即返回超时形态，驱动随即走 `session/cancel` 路径。
- 复审中确认但未改的既有问题：`workspaces/infrastructure/capture_maintenance.rs::enforce_capacity` 每删一行就重算一次 `SUM`（O(n²)），且目前没有任何调用方；`proxy_terminal::release_epoch` 对每个终端顺序等待最多 5 秒（8 个终端最坏 40 秒）；`terminal_request` 允许代理覆盖已清洗环境中的同名变量（注释与行为不符，风险低）。
- 行数预算：`agent_runtime/infrastructure` 聚合 73,112 → 73,238、生产 40,300 → 40,334，理由写在预算旁。

### 七家 CLI 的品牌图标（2026-09-07，用户要求）

- `src/assets/agent-icons/`：Qwen Code（文档站 favicon）、Kimi（kimi.com PWA 图标）、Qoder（qoder.com 图标）、iFlow（iflow.cn 图标）四个 PNG 缩至 64×64；CodeBuddy 取自已安装 npm 包内的 `dist/web-ui/pwa-icon.svg`（官网拒绝脚本访问）；Cursor 取官方 `favicon.svg`；Copilot 内联 Primer Octicons `copilot-24`（MIT，`currentColor`）。来源、日期与商标说明见目录内 `PROVENANCE.md`。
- `AgentBrandIcon` 对六家走 `<img data-agent-icon>`，Copilot 走内联 SVG；未知 id 仍回退到通用 Bot 图形。`agent-visual-identity.ts` 的 lucide 图形保留给在场/席位小徽标，注释同步更正。测试覆盖七家渲染与未知 id 回退。

### 第二轮审查修正（2026-09-07）

- `acp/adapter.rs::bind`：对同一绑定发起第二轮时，原代码先把绑定从表里移除、再因 `active_turn` 存在返回冲突，活动连接由此脱离管理（轮次结束后不再可复用，下一轮被迫重新拉起并 `session/load`；`release_session` 也找不到它）。改为先判忙碌返回冲突、只在空闲时才淘汰替换；回归测试 `a_busy_binding_is_refused_but_not_evicted`。
- 复核通过、未改：兼容性注册表与 V2 manifest 的传输/用量一致性校验、组合网关按进程前缀与 provider 传输路由、审批端口按前缀分派、绑定仓储的 upsert/删除、注册表种子与迁移 112、CLI 身份规则与架构排除、`IdentityMismatch` 在状态徽标中的呈现、参数目录七家条目的渲染器与约束、`resolve_launch` 的策略令牌前置顺序、Web/mock 的连接检查拒绝分支、legacy 分组的显式展开。

