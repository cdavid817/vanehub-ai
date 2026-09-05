# Tree-sitter 代码索引

工作区代码被 Tree-sitter 解析成有边界的代码块（chunk）与符号元数据，落入按工作区划分的持久化索引。这是 `retrieval` 限界上下文的本地那一半——**本地流水线无需任何外部服务即可运行**，可选的语义增强流水线才涉及外部向量嵌入。跨会话记忆池是另一个独立关注点，见[检索与向量搜索](retrieval.md)。

索引**默认禁用**：`CodeWorkspace` 初始为 `enabled: false`、phase `Disabled`，OnePiece 自动策略默认也是 `Disabled`（不自动注册工作区）。远程（SSH）工作区**当前不支持**代码索引：工作区发现对远程会话直接返回空，且注册依赖本地路径规范化。

## 两条流水线

### 本地流水线（无外部依赖）

```text
文件清单（ignore walker，尊重 .gitignore）
    → 工作区边界（canonicalize + 相对化，拒绝绝对路径与 ..）
    → 文件准入（见下）
    → 语言识别（按扩展名）
    → Tree-sitter 解析（容忍语法错误）
    → 符号提取（.scm 查询）+ 代码块生成（预算切分）
    → 脱敏
    → FTS 与元数据持久化（同一文件事务）
```

本地流水线的终点是 FTS5 全文索引与代码块/符号元数据，`search_code` 在此即可工作。

### 语义增强流水线（可选，须显式确认）

后端只有一个 phase 字段；前端把它派生成两条通道的状态。语义通道的派生状态：`not_applicable`（local 模式，未请求语义）、`disabled`、`pending`（扫描/解析中）、`unconfigured`（未配置向量嵌入）、`awaiting_confirmation`（等待外部嵌入确认）、`embedding`、`ready`、`degraded`（`src/services/code-index-contract.ts`）。

**local 模式不经过嵌入确认**：协调结束直接落 `Ready`/`Degraded`，worker 对 local 工作区跳过 `prepare_code_embedding`。只有 semantic 模式在解析完成后进入 `AwaitingEmbeddingConfirmation`。

## 文件准入规则与优先级

准入在 `domain/code_admission.rs` 的 `admit_metadata` 中按序判定（`.gitignore` 由 walker 在更早阶段过滤；符号链接与非普通文件、去重也在扫描层处理）：

1. **选定根**：`selected_roots` 支持每工作区多个索引根目录（空串表示工作区根），不在选定根下 → `OutsideSelectedRoots`；
2. **强制敏感路径**（`is_mandatory_sensitive_path`，用户配置不可覆盖，路径先小写归一）：目录组件 `.ssh`/`.aws`/`.azure`/`.gcp`/`.kube`/`credentials`/`secrets`；文件名 `.env` 与 `.env.*`、`credentials(.json)`、`application_default_credentials.json`、`id_rsa`/`id_dsa`/`id_ecdsa`/`id_ed25519`、`.netrc`；扩展名 `key`/`pem`/`p12`/`pfx`/`jks`/`keystore`/`crt`/`cer`/`der`；
3. **用户排除 glob**：上限 128 条、每条 ≤256 字符，不含 `/` 的模式自动按 `**/<pattern>` 匹配；
4. **语言**：扩展名不识别或该语言未启用 → `LanguageDisabled`。启用语言是**八个**枚举变体：JavaScript、TypeScript、Python、Rust、Go、Java、C、C++——TSX 不是独立语言，而是 TypeScript 按 `.tsx` 后缀选用 TSX grammar；
5. **大小**：默认 `DEFAULT_MAX_FILE_BYTES = 100 KB`（可配，上限 10 MB，不可为 0）；
6. **二进制嗅探**：读前 8 KB 含 `\0` 即判二进制跳过。

跳过不计入失败，只累计计数并聚合为 `Skipped` 审计。

## 解析、符号与代码块

**解析容忍语法错误，"任何语法错误导致整文件失败"不成立。** `load_and_parse` 从不检查语法树是否含 ERROR：存在可恢复语法错误时，仍从有效的 named subtree 提取符号与代码块。要区分四个层次：

- **局部节点跳过**——只有匹配 `.scm` 查询的 named 节点成为符号，ERROR 区域自然不产出符号块；
- **文件级解析失败**——仅五类：不可读、超大小限、非法 UTF-8、grammar 初始化失败、parser 失败（与语法正确性无关）；失败计数后继续下一个文件；
- **文件准入跳过**——不算失败（上一节）；
- **索引整体降级**——本轮 `failed > 0` → phase `Degraded`；semantic 模式下有失败还会跳过本轮嵌入。

**符号是可选的，不是每个代码块必有。** 代码块持久化类型里 `symbol_name: Option<String>`、`symbol_kind: Option<String>`；文件没有任何符号匹配时生成整文件 fallback 代码块（`chunk_key = "fallback:<part>"`，符号字段为空）。符号定义元数据（名称、种类、定义范围）与代码块在同一文件事务中持久化；符号的容器归属（`container_name`）由包含关系事后推导，`.scm` 查询本身不产出容器。

超过预算（`DEFAULT_MAX_CHUNK_BYTES = 6 KB`，编排层固定传入、不可配置）的符号在 named 子节点边界上切分，每块仍能回溯到源符号与文件范围。

> **规格差距**：`workspace-code-indexing` spec 要求"不得嵌入未解析的整文件 fallback"，但当前实现对无符号文件会生成整文件 fallback 代码块并写入索引、进入嵌入队列。这是一处待决策的 spec 与实现冲突（收敛实现或修订 spec）。

## 安全边界：脱敏

准确表述是：**解析器会读取原始代码，但未经脱敏的代码块不得写入检索索引、不得发送给外部嵌入、不得记录到统一日志、不得作为搜索结果返回**。实现上：

- 代码块内容的唯一构造点先脱敏（`code_chunker.rs`），持久化入口再脱敏一次，并以脱敏后文本计算 `content_hash` 写入 `retrieval_documents.content`；FTS 由触发器从该列取内容，索引的也是脱敏文本；
- 嵌入读取的正是这份脱敏后的行；搜索结果 `snippet` 直接取该列，不回读原始文件；
- 协调与批处理日志只记 workspaceId、phase、generation、计数、耗时、模型；审计只存规范化相对路径与原因类别。

脱敏是**六类正则模式的已知模式检测，不是完整 DLP**：私钥 PEM 块、带引号的敏感赋值（`api_key`/`token`/`password` 等关键字）、无引号的同类赋值、`bearer` 令牌、provider 令牌前缀（`sk-`、`ghp_`、`github_pat_`、`AKIA…`）、内网 URL（localhost/私有网段）。命中替换为 `[REDACTED]` 并累计 `redaction_count`。正则编译失败时**失败关闭**：整段内容替换为 `[REDACTED]`，绝不透出原文。此外强制敏感路径在准入层已把最高风险文件挡在解析之外。

## 索引版本、清单与对账

- `CODE_INDEX_VERSION`（当前 `"1"`）覆盖 grammar 兼容性、Tree-sitter 查询、切分与脱敏策略；版本不匹配的文件被标记陈旧并重建（读取工作区时检测到版本过期会同步清除文件行并递增 generation——读路径有这个隐式副作用）。
- 清单（`CodeFileManifest`）记录路径、`content_hash`、mtime、大小、索引版本；对账先比 size+mtime+version，再比内容哈希，未变文件直接跳过（元数据优先、只读取新增或变更文件）。
- **定向对账**：Agent 文件写入成功后经 `notify_targeted_change` 进入有界合并队列（单次上限 512 条路径）。`CodePathChange` 定义了 `Upsert`/`Delete`/`Rename` 三语义（Rename 展开为删旧 + 增新），但**当前生产链路只投递 `Upsert`**——没有文件系统 watcher，Delete/Rename 变体目前仅测试构造（代码中以 dead_code 注记留给后续 watcher）。路径不再满足准入或已不存在时按删除处理。
- **取消**：`reconcile_*_cancellable` 变体存在但生产未接线（调用的是非 cancellable 包装）；实际中断靠 generation 漂移检查。

### 禁用、重建、删除是三个不同操作

| 操作 | 行为 |
| --- | --- |
| **禁用** | 仅 `enabled=false`，phase → `Disabled`，generation+1，清空嵌入确认；**不删数据** |
| **重建** | 删除全部文件行（级联删代码块/符号/文档），generation+1，phase 回 `Scanning`，清空确认，记 `Rebuilt` 审计 |
| **删除索引** | 删除工作区行本身，一切消失 |

另有 **refresh**：同步跑一次对账并返回状态。

## 外部嵌入确认

semantic 模式解析完成后停在 `AwaitingEmbeddingConfirmation`，等待用户显式确认。确认绑定三元组 **profile_id + model + generation**（工作区由行主键隐含），三处必须全等才放行：进入 embedding 的决策点、批次守卫、向量检索前置。确认对话框展示 provider/profile、模型、总代码块数与预计嵌入请求数（`total_chunks.div_ceil(32)`；这决定网络与费用影响）。

**没有独立的"撤销确认"命令**。确认通过三条路径被隐式作废（三列置 NULL 且 generation+1）：保存配置、重建、索引版本失效。作废后 phase 打回等待确认，在飞批次被守卫丢弃，语义检索降级为仅关键词。

嵌入失败重试：单条最多 5 次，退避 `[1, 4, 15, 60, 300]` 秒；认证/无效请求类错误立即放弃，网络类到次数上限放弃。单条内容送嵌入前截到 8 000 字符。

## `search_code` 工具契约

- 输入**恰好** `query`（必填）+ `limit`（可选，默认 5，钳制 1–20），`additionalProperties: false`；有专门测试钉住这一形状。
- **工作区由可信运行时隐式确定**（会话的 workspace folder），模型不能指定工作区或路径根。工具只在该工作区索引已启用且 phase 非 `Unavailable` 时进入工具目录。
- 返回条目字段：`file_path`、`start_line`、`end_line`、`language`、`symbol_name`（可空）、`symbol_kind`（可空）、`snippet`（脱敏后文本）、`matched_via`，顶层可带 `degraded`。
- 检索为向量 + FTS 关键词的 RRF 融合（over-fetch 为 `limit×4`）；**local 模式无向量通道且不标记 degraded**（本地无嵌入是预期状态），semantic 缺向量标 `keyword_only`、缺关键词标 `vector_only`、两者皆缺返回"暂不可用"的软结果。
- **搜索片段不能替代精确读取**：片段是有边界、已脱敏的索引文本，需要精确内容时应使用文件读取工具。

## 设计所在

权威需求位于 spec；本章描述当前实现并标注差距。

- [openspec/specs/workspace-code-indexing](../../../../openspec/specs/workspace-code-indexing/spec.md)

已知的 spec 与实现差距（除上文 fallback 冲突外）：spec 要求的 created/modified/renamed/deleted 四类定向协调，生产链路目前只投递 Upsert（无 watcher）；spec 要求的协作式取消通道未接线（靠 generation 判定）；spec 提及按 Retry-After 兑现限流间隔，嵌入适配器中未见该头解析。spec 的 Purpose 段仍是归档占位文本，待补写。

拥有此部分的 `retrieval` 限界上下文见 [Native 限界上下文](native-contexts.md)；跨会话记忆那一半见[检索与向量搜索](retrieval.md)；与 LSP 的职责对比见 [LSP 代码智能](lsp-code-intelligence.md)。
