# Design

## Context

`cli_config` 的形态已经稳定：每家一个 tagged payload 变体、一个主文件路径（Codex 多一个 `auth.json`）、一个「受管片段」函数（漂移指纹的依据）、一个投影函数（把 profile 写进 CLI 自有文件且保留非受管内容）、一个导入函数（从当前文件反推 profile）。Gemini 是 dotenv 形态的模板，Antigravity 是 JSON 根级投影的模板，Codex 是双文件写入的模板。本变更不引入新形态，只把两家映射到这三种既有模板上。

两家 CLI 的真实读取方式（2026-09-07 从已安装版本核实）：

| CLI | 认证选择 | 密钥 / 端点 / 模型 |
| --- | --- | --- |
| Qwen Code 0.23.0 | `~/.qwen/settings.json` 的 `security.auth.selectedType`；缺省时按环境推断：`OPENAI_API_KEY` + `OPENAI_BASE_URL` + `OPENAI_MODEL` 三者齐全即 `openai` | `~/.qwen/.env`（用户级），亦读 `~/.env`、项目级 `.qwen/.env` |
| iFlow 0.5.19 | `~/.iflow/settings.json` 根级 `selectedAuthType`（必须显式为 `openai-compatible`，环境变量单独存在会落到已废弃旧认证） | 同文件根级 `apiKey` / `baseUrl` / `modelName`；`IFLOW_*` 环境变量优先 |

## Decisions

### Qwen Code：dotenv 主文件 + settings.json 单键

- payload `QwenCode { base_url, model, auth_strategy: PreserveOfficial | ApiKey, advanced_env }`，与 Gemini 同构。受管 `.env` 键：`OPENAI_API_KEY`、`OPENAI_BASE_URL`、`OPENAI_MODEL`。
- `paths()` 对 Qwen 始终返回 `[.qwen/.env, .qwen/settings.json]`（状态页据此展示两个目标）；`ApiKey` 策略下投影额外写 `settings.json` 的 `security.auth.selectedType = "openai"`，其余设置保留；`PreserveOfficial` 下不碰 `settings.json`，`.env` 中移除受管密钥键，只写端点与模型。理由：Qwen 一旦在 settings 里选过 `qwen-oauth`，`.env` 的推断就不再生效，只写 `.env` 会让「应用成功」但 CLI 仍走 OAuth。
- 漂移片段只取 `.env` 受管键（与 Gemini 一致）；`selectedType` 被用户改回 OAuth 不计入漂移，属已知边界，写在文档里。
- 导入 / 发现从 `.env` 读取；有 `OPENAI_API_KEY` 视为 ApiKey 策略。

### iFlow：JSON 根级投影，密钥物化

- payload `IflowCli { base_url, model, advanced_settings }`，无认证策略选项：iFlow 官方服务已关闭，唯一可用的就是 OpenAI 兼容自定义 API，所以 `requires_credential()` 恒真。
- 受管键 `selectedAuthType`、`apiKey`、`baseUrl`、`modelName` 加高级设置；投影按 Antigravity 的方式 upsert 根级键并保留其余（iFlow 自己写入的 `cna` 等标识必须保留）。
- 密钥物化进 `settings.json` 属规范允许的「CLI 自身要求」情形（与 Codex `auth.json` 同类）；VaneHub 侧仍只存密钥引用，DTO 不回传。导入时从 `apiKey` 取密钥、`selectedAuthType != openai-compatible` 的文件仍可导入端点与模型但标记需要密钥。

### 预设

复用 OnePiece 供应商目录：每个 `openai-chat-completions` 端点为两家各生成一条（Qwen 的 API key 策略、iFlow 恒需密钥），另加 `qwen-code-qwen-official`（`PreserveOfficial`，模型 `coder-model`）。不生成 `openai-responses` 条目：两家都走 chat-completions。

### 密钥探测

两家都映射为 `OpenAiChatCompletions` + `Bearer`；Qwen `PreserveOfficial` 与 Gemini 一样拒绝探测（没有密钥可验）。

### 前端

- `cliConfigAgentIds` 追加两项；选择器分组自动多两项；`agentNameKeys` 与五种语言文案补齐。
- 表单：Qwen 复用 Gemini 形态（端点 / 模型 / 认证策略），iFlow 只有端点 / 模型；两家均无高级区。
- Web/mock：校验规则（Qwen 模型必填、iFlow 模型必填且需密钥）与 `cliConfigNeedsCredential` 同步。

## Alternatives considered

- **Qwen 只写 `.env` 不碰 `settings.json`**：在用户曾选 OAuth 的机器上静默失效；拒绝。
- **iFlow 走 `IFLOW_*` 环境变量注入而不写文件**：只对 VaneHub 自己启动的进程有效，用户在终端里直接跑 `iflow` 拿不到，不符合「全局配置」语义；拒绝。
- **为 Kimi / Qoder / CodeBuddy / Copilot / Cursor 也加 profile**：CLI 本身没有第三方端点入口，profile 无处投影；拒绝。

## Risks

- Qwen 在 `settings.json` 里还可能有 `model.name` / `model.baseUrl` / `modelProviders` 等更高优先级的形态；本变更不管理它们，若用户配置了这些，`.env` 可能被覆盖。发现阶段不做猜测，文档说明。
- iFlow 的 `settings.json` 字段名来自安装版本的实际读取逻辑而非公开文档，版本升级可能变化；投影测试固定当前字段名，升级时由测试暴露。
