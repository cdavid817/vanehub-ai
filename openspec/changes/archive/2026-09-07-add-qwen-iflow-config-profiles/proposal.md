# Add Qwen Code and iFlow global configuration profiles

## Why

`extend-cli-providers-with-acp` 把七家 CLI 接进了发现、启动、ACP 会话与参数目录，但「CLI 全局配置」（`cli_config`：第三方端点 / 模型 / 密钥的 profile，保存、校验、全局应用、漂移检测、启动同步）仍只覆盖原来五家。2026-09-07 的实测证明七家里恰有两家原生支持自定义 OpenAI 兼容端点：Qwen Code 读 `~/.qwen/.env` 的 `OPENAI_API_KEY` / `OPENAI_BASE_URL` / `OPENAI_MODEL`，并以 `~/.qwen/settings.json` 的 `security.auth.selectedType` 选择认证方式；iFlow 读 `~/.iflow/settings.json` 根级 `selectedAuthType` / `apiKey` / `baseUrl` / `modelName`。用 DeepSeek 端点分别直连两家都返回了正确结果（Qwen 另经 VaneHub ACP 适配器完成了完整生命周期）。

今天用户要给 Qwen Code 或 iFlow 换一个模型供应商，只能手改这些文件。而 VaneHub 已经为 Codex / Gemini / OpenCode 提供了同类能力，且 Skill、参数目录、会话都把这两家当作一等 Agent。另外五家（Kimi、Qoder、CodeBuddy、Copilot 仅官方账号；Cursor 仅企业端点）的 CLI 不提供第三方端点配置，本变更不为它们假造 profile。

## What Changes

- `cli_config` 支持的稳定 Agent id 从五家扩到七家：新增 `qwen-code` 与 `iflow-cli`；其余五家（Kimi、Qoder、CodeBuddy、Copilot、Cursor）继续被拒绝，且拒绝发生在读写任何文件之前。
- 新增 Qwen Code profile kind（`qwen-code`）：端点、模型、认证策略（保留官方 Qwen OAuth 登录 / OpenAI 兼容 API key）、高级环境变量。应用时把受管键写入 `~/.qwen/.env`，在 API key 策略下同时把 `~/.qwen/settings.json` 的 `security.auth.selectedType` 设为 `openai`（否则用户此前选过的 OAuth 会压过 `.env`）；两文件走既有的快照 / 原子替换 / 回滚路径。非受管键、非受管设置原样保留。
- 新增 iFlow profile kind（`iflow-cli`）：端点、模型、高级设置；始终要求密钥，应用时把 `selectedAuthType: openai-compatible`、`apiKey`、`baseUrl`、`modelName` 投影到 `~/.iflow/settings.json` 根级（iFlow 只从该文件读认证类型，密钥必须物化到 CLI 自有文件，与 Codex `auth.json` 同一类规则），其余键原样保留。
- 两家都接入既有生命周期：列表 / 保存 / 复制 / 删除 / 密钥校验（OpenAI chat-completions + Bearer 探测）/ 全局应用 / 漂移检测 / 当前配置发现与导入 / 启动同步 / 切走回填；预设目录为两家生成 OpenAI 兼容端点的条目，Qwen 另有一条「官方 Qwen OAuth」预设。
- 前端：Agent 配置页选择器多两项；payload 表单、Web/mock adapter 校验与状态、预设、类型与契约同步；文案五种语言。
- 文档与测试：用户指南与开发者指南补两家；域测试、投影 / 导入 / 漂移测试、Web adapter 测试、Playwright、桌面 `domain-cli-tooling` 的常量与路径映射。

## Capabilities

### New Capabilities

- 无。

### Modified Capabilities

- `cli-agent-config-management`：「Supported CLI Agent configuration profiles」扩到七家并明确其余新 CLI 的拒绝语义；新增「Qwen Code global configuration profiles」与「iFlow custom API configuration profiles」两条 requirement。

## Impact

### Native

- `contexts/tooling/cli_config/domain`：`SUPPORTED_AGENT_IDS`、两个 payload 变体与认证策略枚举、受管键、校验与测试。
- `contexts/tooling/cli_config/infrastructure/live_config.rs`：两家的路径（Qwen 双文件）、受管片段、投影、导入、发现与测试。
- `contexts/tooling/cli_config/api.rs`：密钥探测请求映射。
- `contexts/tooling/cli/infrastructure/native_config_reader.rs`：两家的当前模型发现（供会话默认模型使用）。
- 无迁移：profile 表以 `agent_id` + JSON payload 存储，新增 kind 不改 schema。

### Frontend / Web

- `types/cli-agent-config.ts`、`config/cli-agent-provider-presets.ts`、`services/web-cli-config-state.ts`、`settings/pages/agents/*`（选择器、payload 表单、对话框与列表的凭据规则）、五种语言文案。Tauri 与 Web/mock 契约同步；Web 模式照旧不声称写本地文件。

### Compatibility and scope

- 不改动原五家的 payload、路径与行为；不改变 ACP / 终端启动对环境变量的处理（`child_environment` 已保留 `OPENAI_*` 给 qwen-code）。
- 不为不支持第三方端点的五家 CLI 提供 profile；不替用户登录；不在日志、DTO、SQLite 中出现密钥。
- 不覆盖 Qwen 的项目级 `.qwen/.env`、`QWEN_HOME` 重定向与 `settings.json` 内 `model.*` / `modelProviders` 高级形态：只管理用户级 `.env` 受管键与 `security.auth.selectedType` 一项。

## Delivery and Acceptance

验收以 AGENTS.md 全量校验命令、`openspec validate add-qwen-iflow-config-profiles --strict`、Playwright 与桌面 `desktop-smoke`（`domain-cli-tooling` 覆盖两家的目录、发现与校验路径，写路径按该层既有规则不执行）为准；本机为 Linux，其余平台按 NOT RUN 记录。任务见 [tasks.md](tasks.md)。
