# Implementation notes

实施日期 2026-09-07，worktree `feat/cli-supplement`，叠加在 `extend-cli-providers-with-acp` 与 `harden-sqlite-write-transactions` 之上，未提交。

## 落点

### Native（`src-tauri/src/contexts/tooling/cli_config`）

- `domain/mod.rs`：`SUPPORTED_AGENT_IDS` 5 → 7；新增 `QwenAuthStrategy { PreserveOfficial, ApiKey }`、`QWEN_MANAGED_ENV_KEYS`（`OPENAI_API_KEY` / `OPENAI_BASE_URL` / `OPENAI_MODEL`）、`IFLOW_MANAGED_KEYS`（`selectedAuthType` / `apiKey` / `baseUrl` / `modelName`）；payload 变体 `QwenCode`（端点、模型、认证策略、高级环境）与 `IflowCli`（端点、模型、高级设置）；`agent_id` / `requires_credential`（Qwen 按策略，iFlow 恒真）/ `managed_keys` / `validate`（Qwen 拒绝受管键与密钥形键进高级环境；iFlow 高级设置 ≤ 16 项、仅标量、拒绝受管键与密钥形键）。域测试三条，含对 Kimi / Qoder / CodeBuddy / Copilot / Cursor 的拒绝断言。
- `infrastructure/live_config.rs`：路径 `.qwen/.env`（Qwen 主文件，`paths()` 附带 `.qwen/settings.json`）与 `.iflow/settings.json`；`dotenv_fragment` / `root_json_fragment` 两个通用片段函数（Qwen / iFlow 的漂移指纹）；`project_qwen`（与 Gemini 同构）+ `project_qwen_auth_selection`（只写 `security.auth.selectedType = "openai"`，保留 `security` / `auth` 其余键；仅 API key 策略下作为第二个写入目标，走既有快照 / 原子替换 / 回滚）；`project_iflow`（根级 upsert，密钥物化，缺密钥时 `CredentialRequired` 且不写文件）；`import_qwen` / `import_iflow`；`discover_current` 与 `discover_exclusive` 分派。投影测试两条（含 preserve-official 不碰 `settings.json`、iFlow 保留 `cna` 标识、导入回读），损坏文件用例加入 iFlow。
- `api.rs`：密钥探测映射两家为 OpenAI chat-completions + Bearer；Qwen preserve-official 与 Gemini 同样拒绝探测。
- `tooling/cli/infrastructure/native_config_reader.rs`：`discover_qwen_model`（`.env` 的 `OPENAI_MODEL`）与 `discover_iflow_model`（`settings.json` 的 `modelName`）供会话默认模型使用，附测试。
- 无迁移：profile 表以 `agent_id` + JSON payload 存储；`agents` 表已在 ACP 变更中含两家 id，外键成立。

### Frontend

- `types/cli-agent-config.ts`（id 列表、`QwenCodeConfigPayload` / `IflowCliConfigPayload`；文件贴近 300 行上限，两段注释压成单行）。
- `config/cli-agent-provider-presets.ts`：每个 `openai-chat-completions` 端点为两家各生成一条预设（Qwen API key 策略、iFlow 恒需密钥），另加 `qwen-code-qwen-oauth-official`（preserve-official，模型 `coder-model`）；`agentLabels` 表替代原三元表达式；自定义 payload 两条。
- `services/web-cli-config-state.ts`：凭据规则与模型必填校验；`settings/pages/agents`：`QwenFields`（端点 / 模型 / 认证策略）、`IflowFields`（端点 / 模型）、对话框与列表的凭据判断、选择器名称键；五种语言新增 `agentConfigurations.agent.qwen` / `.iflow`。
- 测试：预设目录（providerId 集合 27 → 28，两家仅 chat-completions）、Agent 配置页选择器按钮列表（8 项）、payload 表单两条、Web adapter 生命周期一条（含 Kimi 仍被拒绝）。

### E2E 与文档

- Playwright `agent-global-config.spec.ts`：Qwen Code 与 iFlow 用 DeepSeek 预设创建并全局应用（Web 模拟），断言密钥不出现在页面。
- 桌面 `domain-cli-tooling.e2e.mjs`：常量扩到七家，第二目标改为 `{codex-cli: auth.json, qwen-code: settings.json}` 映射；写路径仍按该层规则不执行。
- 文档：用户指南 `agent-configuration.md`（zh-CN / en）表格与标签列表、`acp-cli-agents.md` 新增「第三方端点」节；开发者指南 `cli-lifecycle.md`；根目录 `docs/cli-agent-global-configuration.md` 的文件表、端点可用性与预设段落。

## 顺带修正

- `tests/architecture.rs` 的 `agent_runtime/infrastructure` 聚合行数预算 72,652 → 73,112：`extend-cli-providers-with-acp` 的 live 测试（prompt / 审批 / 拒绝 / 取消 / 重启恢复）新增约 460 行测试代码，生产预算不变。归属写在预算旁。

## 已知边界（与 design.md 一致）

- Qwen 的漂移指纹只看 `.env` 受管键；用户把 `settings.json` 的认证方式改回 OAuth 不计入漂移。
- Qwen 的 `settings.json` 内 `model.*` / `modelProviders` 等更高优先级形态不纳管。
- iFlow 字段名来自安装版本（0.5.19）的实际读取逻辑；升级后若变化由投影测试暴露。
