# Tasks

已按 2026-09-07 的实际完成状态勾选；验证项见 verification/results.md。

## 1. 规范与域

- [x] 1.1 `openspec validate add-qwen-iflow-config-profiles --strict` 通过后再改代码。
- [x] 1.2 `cli_config/domain`：`SUPPORTED_AGENT_IDS` 扩到七家；新增 `QwenCode`（含 `QwenAuthStrategy`）与 `IflowCli` payload 变体及 `IFLOW_MANAGED_KEYS`；`agent_id` / `supports_credential` / `requires_credential` / `managed_keys` / `validate` 覆盖两家（Qwen 高级环境不得含受管键或密钥形键；iFlow 高级设置不得含受管键、密钥形键，值仅标量）；补域测试。

## 2. 投影、导入与探测

- [x] 2.1 `live_config.rs`：`primary_path`（`.qwen/.env`、`.iflow/settings.json`）、`paths`（Qwen 始终报告 `.env` 与 `settings.json` 两个目标；仅 API key 策略下写第二个）、`qwen_fragment` / `iflow_fragment`、`project_qwen`（`.env` + `security.auth.selectedType`）/ `project_iflow`（根级 upsert、密钥物化）、`import_qwen` / `import_iflow`、`discover_current` 与 `discover_exclusive` 分派。
- [x] 2.2 投影测试：两家均保留非受管内容、移除上一 profile 的受管键、导入回读、发现可见且不改文件；Qwen 的 `settings.json` 其他键保留；恶意 / 损坏文件按 Malformed 报告不修改。
- [x] 2.3 `api.rs` 密钥探测映射（chat-completions + Bearer；Qwen 保留官方登录时拒绝探测）；`native_config_reader.rs` 两家的当前模型发现及测试。

## 3. 前端与 Web/mock

- [x] 3.1 `types/cli-agent-config.ts`：id 列表与两个 payload 接口；`config/cli-agent-provider-presets.ts`：两家的预设与自定义 payload；`services/web-cli-config-state.ts`：凭据规则与校验。
- [x] 3.2 `settings/pages/agents`：选择器名称键、payload 表单（Qwen 端点 / 模型 / 认证策略，iFlow 端点 / 模型）、对话框与列表的凭据判断；五种语言文案。
- [x] 3.3 前端测试：预设目录测试（数量与端点类型）、payload 表单测试、Web adapter 客户端测试（保存后 DTO 不含密钥）。

## 4. 端到端、桌面与文档

- [x] 4.1 Playwright `agent-global-config.spec.ts` 增加 Qwen Code 与 iFlow 的创建流程；桌面 `domain-cli-tooling.e2e.mjs` 的 id / 路径 / kind 映射扩到七家（写路径按该层规则仍不执行）。
- [x] 4.2 用户指南（zh-CN / en）的 Agent 配置与 ACP CLI 页面、开发者指南 `cli-lifecycle.md` 补两家的配置文件与边界说明。

## 5. 验证

- [x] 5.1 AGENTS.md「校验命令」全部命令，以及 `npm run architecture:check`、`npm run test:coverage`、`npm run contracts:check`、`npx playwright test`。
- [x] 5.2 `npm run test:desktop:build` 后运行 `desktop-smoke`；按平台记录 PASSED / FAILED / BLOCKED / NOT RUN。（Linux PASSED；Windows / macOS NOT RUN）
- [x] 5.3 `openspec validate add-qwen-iflow-config-profiles --strict`；在 `verification/results.md` 记录命令、退出码与证据目录。
