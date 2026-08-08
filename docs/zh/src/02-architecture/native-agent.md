# 原生 API Agent（OnePiece）

> **不依赖任何外部 CLI 的内置 Agent**：OnePiece 直接通过 HTTP 调用模型 provider，自带版本化核心指令、两层记忆与对记忆池的混合检索，是 VaneHub AI 里唯一 `launch.kind = "api"` 的 Agent。

## 这一层解决什么问题

**它填补的是"没装 CLI 也要能用"以及"需要更细粒度控制"的场景**。外部 CLI Agent 是黑盒——VaneHub AI 只能管进程之外的部分；OnePiece 则完全在本进程内，工具调用、上下文构造、记忆写入都可被直接观测和控制。

**它还承担一项幕后职责**：为 CLI 包装的 Agent 代做记忆提取，见 [个性化](personalization.md#提取时机与执行者)。

## 能力与运行时边界

| 能力 | 说明 | 运行时 |
|---|---|---|
| Provider 目录 | 25 家预置 provider，含默认与备选模型 | **仅桌面** |
| 自定义端点 | 在预置之外配置兼容端点 | **仅桌面** |
| 模型发现 | 从 provider 拉取可用模型列表 | **仅桌面** |
| 凭据校验 | 保存前验证 API Key 可用性 | **仅桌面** |
| 核心指令 | 版本化的内置行为指令，带体积预算 | **仅桌面** |
| 工具调用 | 调用 MCP 工具与内置工具 | **仅桌面** |
| 上下文压缩 | 长对话自动压缩 | **仅桌面** |
| 跨会话记忆 | 压缩时提取并持久化 | **仅桌面** |
| 记忆检索（recall） | 对记忆池做向量 + 关键词混合检索 | **仅桌面** |
| 检索降级 | 单路不可用时自动退化并标记 | **仅桌面** |
| 索引重试 | 按失败类别决定是否重试 | **仅桌面** |

## Provider 目录

**目录是前后端共享的单一真源**：`src/config/onepiece-provider-catalog.json`，Rust 侧通过 `include_str!` 在编译期内联（`src-tauri/src/contexts/agent_runtime/application/onepiece_provider_catalog.rs:5`）。这样前端选择器与后端调用逻辑不会各自维护一份而漂移。

**当前 `catalogVersion = 3`，收录 25 家 provider**：

| 类别 | 数量 | 条目 |
|---|---|---|
| `official` | 2 | Anthropic（默认 `claude-sonnet-4-6`）、OpenAI（默认 `gpt-5.4`） |
| `common` | 23 | OpenRouter、DeepSeek、Zhipu GLM、Kimi / Moonshot、SiliconFlow、Alibaba Bailian、Volcengine Ark、Groq、xAI、Mistral、Together AI、Fireworks、NVIDIA NIM、Cerebras、MiniMax（国内 / 全球）、StepFun、Baichuan、PPIO、Qiniu、ModelScope、Xiaomi MiMo、Z.AI |

**每个条目携带 11 个字段**（`onepiece_provider_catalog.rs:16-27` 的 `CatalogEntry`）：`id`、`displayName`、`category`、`iconKey`、`provider`、`defaultModelId`、`fallbackModels`、`apiKeyUrl`、`docsUrl`、`defaultEndpointType`、`endpoints`。

**`apiKeyUrl` 与 `docsUrl` 是可用性设计**——界面能直接给出"去哪儿申请 Key"的链接。

**调用实现分两类**：`infrastructure/anthropic_provider.rs`（Anthropic 原生协议）与 `infrastructure/openai_compatible_provider.rs`（OpenAI 兼容协议，覆盖绝大多数条目）。

## 模型发现与凭据校验

**模型列表从 provider 实时拉取**（`infrastructure/onepiece_model_discovery.rs`）：

| 特性 | 说明 |
|---|---|
| 双格式兼容 | 同时解析 **OpenAI 信封格式与裸数组格式**（`:186` 的测试 `parses_openai_and_array_shapes`） |
| 按策略切认证头 | `:199` 的测试 `applies_strategy_specific_auth_headers` |
| 响应体上限 | **2 MB**（`:19` 的 `MAX_RESPONSE_BYTES`），避免异常响应撑爆内存 |
| 凭据校验 | `:75` 的 `validate_credential`，保存前验证 |

## 调用构造

**`build_invocation_with_role` 处理一个关键问题**（`infrastructure/providers/invocation.rs:52-58`）：席位的角色简报该放在哪里。

**答案是放进 CLI 自己的 system-prompt 通道，而不是普通提示词文本**，注释说明了原因：

> 简报不能作为普通提示词文本传递：**那个通道会被上下文压缩影响，长会话中角色会被丢掉，Agent 会悄悄不再扮演它。**

**没有 system-prompt 通道的 Agent 不在这里注入**——调用方退回逐轮注入，并**把该席位标记为"非压缩免疫"**，而不是让这里静默丢弃。

**这条设计把"角色会不会在长对话中失效"变成了一个显式的、可被上层感知的属性**，而不是一个只有跑长了才发现的隐性缺陷。

### 权限模板治理的例外

**`POLICY_TEMPLATE_GOVERNED_AGENT_IDS` 只包含三个 CLI Agent**（`invocation.rs:7-12`），注释写明：

> `claude-code` 被有意排除，因为它的策略模板已经通过 `claude-code-permission-hook` 的逐调用钩子动态强制执行，而不是靠启动参数。

详见 [CLI 集成](cli-integration.md#差异吸收点一启动参数)。

## 核心指令

（`infrastructure/core_instructions.rs`）

| 项 | 值 |
|---|---|
| 版本常量 | `ONEPIECE_CORE_VERSION = "1.0.0"` |
| 内容来源 | `include_str!("onepiece-core-v1.md")`，编译期内联 |
| 生效范围 | **只对 `onepiece`**（`:16-20`） |

**有测试守住版本与体积**（`:27` 的 `shipped_onepiece_core_is_versioned_and_within_budget`）：断言版本号正确、内容非空，并对体积设了预算——核心指令占用的是每次调用的上下文额度，不能无限增长。

## 两层记忆

| 层 | 机制 |
|---|---|
| 短期 | 对话超长时触发上下文压缩，保留要点 |
| 长期 | 压缩触发时提取记忆，写入 `agent_memories` 表 |

记忆是**主机级共享池**，不再按 Agent 隔离。完整说明见 [个性化的 Agent 记忆](personalization.md#agent-记忆)。记忆写入受权限系统 `memory.write` 动作管辖。

## 记忆检索（recall）

**这是 OnePiece 的能力，不是全局功能**——界面入口挂在 OnePiece 配置下（`src/settings/pages/agents/onepiece-retrieval-section.tsx`）。

### 检索的对象是记忆池，不是项目代码

**`SourceKind` 当前只有一个变体**（`retrieval/domain/document.rs:4-8`）：`AgentMemory`。

`IndexSourcePort` 的注释也写明（`application/indexing_service.rs:30`）：**第 1 期唯一的实现是 bootstrap 里的记忆表适配器**。

**换言之：recall 检索的是你积累下来的记忆，不是仓库文件。**

### 查询不带作用域

**这是一个有理由的省略**（`retrieval/domain/query.rs:1-3`）：

> 检索面向的是同一个主机级共享记忆池——`agent-memory-shared-pool`（迁移 42）之后，`agent_memories` 对每个 Agent 全量可见，最近记忆的注入也不再按 agent/folder 过滤。查询因此不带 scope：**带了反而会让 `recall` 只能搜到系统提示词里已经注入过的内容的一个真子集。**

`RetrievalQuery` 因此只有两个字段（`query.rs:5-8`）：`text` 与 `limit`。

### 混合检索与融合

**两路召回，RRF 融合**：

| 命中方式 | 值（`query.rs:11-15` 的 `MatchedVia`） |
|---|---|
| 仅向量 | `vector` |
| 仅关键词 | `keyword` |
| 两路都中 | `both` |

**融合算法是 RRF（Reciprocal Rank Fusion）**（`domain/fusion.rs`），两处取舍代码里已用中文写明：

- **平滑常数取 60**（`fusion.rs:5` 的 `RRF_SMOOTHING`），沿用原论文取值。作用是压低头部名次的边际优势，让"两路都中游"胜过"一路头名、另一路缺席"——这正是混合检索想要的行为。
- **同分时按 id 升序**（`fusion.rs:8`），保证同样输入永远给出同样顺序，否则测试与界面都会闪动。

**结果条目**（`query.rs:43-50` 的 `ScoredHit`）：`source_id`、`content`、`created_at`、`score`、`matched_via`。

### 降级

**单路不可用时不整体失败，而是退化并标记**（`query.rs:28-32` 的 `Degradation`）：

| 降级 | 含义 |
|---|---|
| `keyword_only` | 向量路不可用（例如嵌入服务故障），只用关键词 |
| `vector_only` | 关键词路不可用，只用向量 |

**降级被显式暴露**，因此调用方知道这次结果的质量前提。

#### 铁律：检索失败永不让生成失败

**这条写在 `search()` 的文档注释里**（`application/search_service.rs:44-45`，引设计文档 §8.1）：

> 检索失败**永不**让生成失败。除"未配置"外，任何一路的失败都只是降级，因为把一个可选增强能力的故障冒泡成生成失败是不可接受的。

**两路都失败时返回 `Err(RetrievalError::Unavailable)`**，但注释说明了为什么这不违反铁律（`:69-71`）：

> 复用已有的 `Err` 路径而不是新增一种降级值：`execute_recall` 已有分支会把 `Err` 转成**成功的**工具结果"检索暂时不可用"。

**模型拿到的是一次成功的工具调用**，内容是「暂时不可用」，而不是一个工具错误。这样既不必给模型新增一套词汇，生成也不会因此中断。

**「两路都失败」与「两路都可用但都没命中」必须区分**——后者是正常的空结果，前者是故障。用 `Err` 与空 `Vec` 区分开，两者在模型侧的表述也不同。

#### 超额召回 4 倍

```rust,ignore
let over_fetch = query.limit.saturating_mul(4).max(query.limit);
```

**每路各取 4 × limit 条候选再融合**。原因是融合会重排——只取 limit 条的话，某一路排在第 limit+1 位、但另一路排得很靠前的条目就永远进不了融合。

**`saturating_mul` 加 `max` 是溢出保护**：`limit` 极大时乘法饱和，`max` 保证结果不小于 `limit` 本身。

#### 模型写的 query 要先截断

```rust,ignore
let text = truncate_for_embedding(&query.text);
```

注释说明了不截断的双重后果（`:53-55`）：

> query 是模型自撰的，长度不受任何约束。用与索引侧相同的上限截断：超长 query 会让 embedding 调用直接失败（对用户表现为**无声的 `keyword_only` 降级**），同时把几千 token 的短语塞进 FTS。

**「无声的降级」是关键词**——不截断的话，用户看到的不是报错，而是检索质量莫名其妙变差。**用与索引侧相同的上限**则保证 query 与被索引内容处在同一尺度上。

#### 回查源表：已删记忆永不外泄

**融合出候选 id 后不直接用索引里的内容，而是回源表取权威内容**（`:88-91`）：

> 索引行可能陈旧，源已删则跳过——这保证已删记忆永不外泄，也是显式撤销失败时的第一道兜底（§5.3）。

**索引与源表可能不同步**：用户删了一条记忆，索引行可能还在。回查源表并跳过缺失项，让删除立即生效，不依赖索引清理是否成功。

#### 取全部候选而不是前 limit 条

**这是最容易写错的一处**（`:93-95`）：

> 取的是**全部**候选而不是前 `limit` 条——下面的 `take(limit)` 在跳过"源已删"的条目**之后**才截断，只回查前 `limit` 条会让一条已删记忆白占一个名额。

**举例**：`limit = 5`，融合后前 5 条里有 1 条已删。若只回查前 5 条，最终只能返回 4 条；回查全部候选，则第 6 条会补上来。

**候选数本身有界**（两路各至多 `over_fetch` = 4 × limit），所以「取全部」不会失控。

### FTS 查询转义

**`escape_fts_query` 把整条查询转义成单个 FTS5 字符串字面量**（`query.rs:57`），注释解释了为什么不能照抄仓库里既有的做法：

> 仓库里唯一的既有 FTS 消费方 `contexts/workspaces/infrastructure/output_search.rs:36-47` 是把原始串直接塞进 `MATCH ?1` 的，只挡空串与超长。**这里不能照抄**：`recall` 的 query 由模型自由生成，含 `"` `*` `:` `-` `OR` `NEAR` 时 FTS5 会按查询语法解析，**轻则语义跑偏，重则整条语句报错**。转义成短语后，trigram tokenizer 下的子串匹配正是我们想要的行为。

**这是一处典型的"看似能复用、实则不能"**：两个调用方的 query 来源不同（人输入 vs 模型生成），可信度不同，处理方式因此必须不同。

### 索引状态与重试

**三种索引状态**（`document.rs:26-30` 的 `IndexState`）：`pending`、`indexed`、`failed`。

**四种失败类别，只有两类可重试**（`document.rs:58-79` 的 `FailureCategory`）：

| 类别 | 可重试 | 理由 |
|---|---|---|
| `auth` | 否 | 确定性失败 |
| `invalid_request` | 否 | 确定性失败 |
| `rate_limit` | **是** | 瞬时 |
| `network` | **是** | 瞬时 |

注释直言（`document.rs:57`）：**`Auth` / `InvalidRequest` 是确定性失败，重试只会烧配额。**

**重试退避是五级的**（`indexing_service.rs:26` 的 `RETRY_BACKOFF_SECONDS`）：`1, 4, 15, 60, 300` 秒。

### 嵌入截断与全文索引的分工

**超长内容在 embedding 前截断**（`indexing_service.rs:28` 的 `EMBEDDING_CONTENT_LIMIT = 8000`），但注释指出一个巧妙的补偿：

> **FTS 仍索引全文，所以长记忆的尾部仍可被关键词命中。**

两路召回在这里形成互补：向量路只看前 8000 字符，关键词路看全部。

### 端口设计的一处性能考量

**`IndexSourcePort` 有两个取数方法**（`indexing_service.rs:31-38`），分工明确：

| 方法 | 用途 |
|---|---|
| `snapshot()` | **全量**快照；`reconcile` 需要全局视图才能判定孤儿行 |
| `fetch(source_ids)` | 按 id 取，命中不到的直接缺席（源已删） |

注释说明为什么检索路不能用 `snapshot()`：**它会在生成的工具调用里同步加载并克隆整个共享池，而源表是只增不减的。**

**`created_at` 刻意不复制进索引行**（`indexing_service.rs:46-48`）：它只存在于源表，避免又多一处会陈旧的副本。

### 向量存储

向量以字节形式存储（`domain/vector.rs`）：`encode_embedding`（`:3`）、`decode_embedding`（`:11`）、`cosine_similarity`（`:25`）。嵌入通过 `infrastructure/openai_embedding_adapter.rs` 获取。

## 配置流程

```mermaid
flowchart LR
  A["选择 provider<br/>25 家目录"] --> B["填入 API Key"]
  B --> C["validate_credential<br/>校验凭据"]
  C -->|通过| D["list_models<br/>拉取模型列表"]
  D --> E["选定模型<br/>可配 fallback"]
  E --> F["OnePiece 可用"]
  F -.同时启用.-> G["CLI Agent 的记忆提取"]
  C -->|失败| B
```

## 界面入口与前端服务

### 配置 provider

设置中心 → Agent 配置页 → OnePiece 配置面板（`src/settings/pages/agents/onepiece-configuration-panel.tsx`）：

1. 在 provider 目录（`onepiece-provider-catalog.tsx`）中选择厂商，或用 provider 对话框（`onepiece-provider-dialog.tsx`）配置自定义端点
2. 填入 API Key，保存前会做凭据校验
3. 选择模型；目录条目已提供默认模型与备选列表

档案操作对话框见 `onepiece-profile-action-dialog.tsx`，前端预设在 `src/config/onepiece-provider-presets.ts`。

### 配置检索

同一页面的检索区（`onepiece-retrieval-section.tsx`）配置嵌入模型与索引设置。

### 在会话中使用

创建会话时选择 Agent `onepiece`，交互模式为 `api`。未完成 provider 配置时该 Agent 显示为不可用——`src/services/mock-agent-data.ts:52-53` 给出的不可用理由即 "OnePiece requires provider configuration."

## 边界与限制

- **仅桌面可用** —— provider 调用、SQLite 索引与记忆均依赖原生运行时。
- **必须先配 provider** —— 未配置凭据时 Agent 处于 `unavailable`；**且 CLI Agent 的记忆提取也一并失效**。
- **检索对象只有记忆** —— `SourceKind` 当前只有 `AgentMemory`，不索引项目代码或文档。
- **检索需要嵌入服务** —— 嵌入不可用时退化为 `keyword_only`，而非完全不可用。
- **嵌入只覆盖前 8000 字符** —— 长记忆的尾部只能靠关键词路命中。
- **核心指令不可编辑** —— 编译期内联且有体积预算。
- **模型目录可能滞后** —— `catalogVersion` 是静态目录，provider 新增模型需目录更新或走模型发现动态拉取。
- **与外部 CLI 不共享会话** —— OnePiece 的会话独立，不能迁移到 CLI Agent。

## 相关文档

- [个性化](personalization.md) —— 核心指令、记忆、专家角色与 OnePiece 的代提取职责
- [工具生态](tooling.md) —— MCP 工具接入
- [可观测性](observability-architecture.md) —— 原生保真度的 Span
- [权限审批](permissions-architecture.md) —— `memory.write` 与 `mcp.tool` 管辖
- [数据层](data-layer.md) —— 记忆表与迁移 42
