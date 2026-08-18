# OnePiece native Agent

OnePiece 是 VaneHub 内置的第一方 Agent。与基于 CLI 的 Agent 不同,它完全通过 native API 运行时运行:`launch_kind = api`、`agent_origin = builtin`,预留稳定 id 为 `onepiece`。它在首次启动时被植入注册表,即便尚未存在任何 provider 配置或凭证时也保持可见。

## 身份与生命周期

OnePiece 身份由注册表拥有,而非由 provider 配置拥有。它与多个命名、由 catalog 支撑的上游 provider **Profile** 相分离,每个 Profile 独立保管自己的凭证。同一时刻至多有一个 Profile 被显式激活用于运行时生成。创建 Profile 时必须选择一个由所选 provider 拥有且经过评审的 endpoint 类型——不接受用户随意提供 provider 身份、接口格式或 Base URL。

## Provider 目录与 Profile 生命周期

OnePiece 的 provider 目录是单一真源——前端 JSON `src/config/onepiece-provider-catalog.json` 由 Rust 侧 `include_str!` 直接嵌入二进制(`onepiece_provider_catalog.rs`),解析失败即 panic。

- **目录结构** —— `catalogVersion: 3`,24 家 provider。`category` 仅 `anthropic` 与 `openai` 为 `official`,其余 22 家(含 openrouter、deepseek、zhipu-glm、kimi、siliconflow 等)为 `common`。每条 provider 含 `id`/`displayName`/`defaultModelId`/`fallbackModels`/`apiKeyUrl`/`docsUrl`/`defaultEndpointType`/`endpoints`。
- **endpoint 字段** —— `baseUrl`/`interfaceFormat`(`anthropic` | `openai-compatible`)/`authStrategy`(`x-api-key` | `bearer`)/`source`/`modelDiscovery`。
- **模型发现策略** `modelDiscovery.strategy` 四值:`anthropic`、`openai`(绝大多数)、`openai-array`(仅 Together AI)、`catalog`(运行时保留)。发现时先注入 catalog 静态模型(`fallbackModels` + profile model),再按策略拉取实时模型,过滤非聊天模型(`is_chat_model`,排除 embedding/embed-/rerank/tts/audio/image 等关键词),上限 1000 个;实时发现失败则回落 catalog 并带 `warning: "live-unavailable"`。

### Profile 数据结构

`OnePieceProviderProfile` 字段:`id`/`name`/`sourceProviderId`/`sourceEndpointType`/`sourcePresetVersion`/`provider`/`modelId`/`interfaceFormat`/`baseUrl`/`active`/`credentialPresent`。Profile 的 scoped 凭据键为 `onepiece-profile:{profile_id}`。`onepiece_provider_profiles` 表硬性绑定 `agent_id = 'onepiece'`(CHECK 约束),并用**部分唯一索引** `UNIQUE(agent_id) WHERE active=1` 从数据库层保证同一时刻最多一个 active profile。

### 生命周期与凭据回滚

Profile 的创建/激活/删除都带凭据双向回滚:

- **保存 catalog profile** —— 新 id 形如 `onepiece-profile-{uuid}`;已存在 profile 不可改 source provider/endpoint;首个 profile 自动激活(`previous.active || existing.is_empty()`);凭据有效值优先级为瞬态 key > scoped 旧凭据 > active 时 runtime 凭据;DB 写失败时回滚 scoped 凭据。
- **激活** —— 目标 profile 必须存在;`authentication_mode != "required"` 直接激活,required 且无 key 拒绝;先把当前 active profile 的 runtime 凭据落回其 scoped key(防丢失),再把目标 scoped 凭据写入 `onepiece`,失败回滚 runtime 凭据。
- **删除** —— 删 scoped 凭据;若为 active 还删 `onepiece` 凭据;DB 删除失败时恢复两处凭据。
- **重置** —— 清空 `agents.onepiece` 行,删除 `onepiece` 凭据**及所有 profile 的 scoped 凭据**。

### 凭据校验(保存前实际调用一次)

`validate_onepiece_provider_credential` 在保存前发起一次最小成本探测:`max_tokens=1` / `max_output_tokens=1`、body 仅 "Reply OK."、超时 15s、禁重定向、响应上限 2MB。HTTP 状态分类:2xx→Valid;401/403→InvalidCredential;400/404/405/409/415/422→ConfigurationRejected;429→RateLimited;5xx→ProviderUnavailable;其余→Inconclusive。`discover` 与 `validate` 命令用 `spawn_blocking` 包裹(底层是阻塞式 HTTP 客户端)。

### 自定义 Profile 校验

`EndpointProfileSnapshot::new()` 校验:base_url 归一化(去尾斜杠,禁 `@`/空白/控制字符);**只允许 `openai-compatible`**;timeout 范围 `100..=120_000`ms;Local 端点必须 loopback(`localhost|127.0.0.1|[::1]`);runtime kind 与 privacy 必须匹配;Required 必须有凭据、None 不得有凭据;context 容量 `1_024..=10_000_000`。错误枚举 `ProviderProfileError`。

## 设计所在

本章用于为贡献者定向。权威需求——稳定身份、注册表植入、预留 id 冲突处理、Profile 生命周期以及 provider-directory 契约——位于 spec 中。

- [openspec/specs/onepiece-native-agent](../../../../openspec/specs/onepiece-native-agent/spec.md)

与 CLI Agent 配置共享的 provider 目录以及 native API 运行时,在 [Runtime and service boundaries](runtime-boundaries.md) 与 [Native bounded contexts](native-contexts.md) 中介绍。
