## 1. 基线与契约

- [x] 1.1 核对当前 HEAD/AGENTS.md 和相关主规范，记录与分析基线的差异；协调三个活动变更的 overlap，不覆盖已有代码或归档。
- [x] 1.2 实现前运行 `npx --yes @fission-ai/openspec@1.8.0 validate unify-memory-read-scope --strict`，按本变更已确定的读取语义推进。
- [x] 1.3 定义 MemoryReadSubject/Context、pinned handle、完整 AuthorizedMemoryQuery、可用性/原因码与合约版本；保持 query/limit 工具 schema，不新增 readScope 设置。
- [x] 1.4 盘点四条入口、generic memory read、Skill 委托、Context Engine 重注入及 owner/索引 API 调用者，确保 Agent 没有可到达的无 context 搜索或 owner fallback。

## 2. 资格与真实上下文

- [x] 2.1 以现有 domain eligibility 统一 read/global/workspace/session/audience/lifecycle 语义，建立 Rust/SQL/Web 共用 fixture；精确 Agent ID、未知字段拒绝。
- [x] 2.2 修复投影和相关 audience 过滤的 LIKE/大小写语义、未知 scope/default 分支及非法 workspace 组合，不让名称/摘要先于正文泄漏。
- [x] 2.3 sessions owner 解析实际 Agent/seat、generation、mode、workspace；区分明确无 workspace 与解析失败，保留 worktree/remote identity 规则。
- [x] 2.4 把一次 personalization snapshot 前移至 Context Engine 之前，所有消费者复用冻结政策；保持现有政策下次 generation 生效及 last-known-good 规则。
- [x] 2.5 实现 read-context 的归属/epoch/health 校验，直接调用、未知身份、结束 generation、跨 seat 或 workspace 复用均拒绝。

## 3. 权威读取与索引

- [x] 3.1 在 personalization owning API 增加 eligible metadata 游标、摘要页和 pinned read；按 ID/revision/hash/metadata fingerprint 读取，避免全量正文加载。
- [x] 3.2 权威 metadata/body 一致读取并在交付前复核；覆盖外部编辑不升 revision、文件替换、archive/delete、scope/audience 改动；陈旧 handle 丢弃不偷换新版。
- [x] 3.3 分离 RuntimeMemoryRead、OwnerMemoryManagement 和 MemoryIndexMaintenance；保留 compatibility 公共语义，禁止仅放宽其过滤修问题。
- [x] 3.4 索引源覆盖全部合格 active v2 记录的本地 FTS，source_agent/folder 仅溯源；补齐 source id 映射和修订 metadata。
- [x] 3.5 核对已有 embedding 外发授权；此前未索引的 scoped/audience 记录无可验证授权时仅本地 FTS，不新增自动外发或新同意系统；保留向量同模型规则。
- [x] 3.6 必要时追加派生结构 migration 与有界可重入 reconcile/rebuild；不重写 v2 文件或用户 scope/audience/policy，不把不完整源快照当删除全集。
- [x] 3.7 基线无restricted memory外发授权，首版固定FTS-only；claim/retry/rebuild/模型切换/恢复统一排除，并在embed前回源重验queue版本和scope/audience，覆盖公共项排队后变restricted，禁止借代码索引确认授权。

## 4. 受治理检索

- [x] 4.1 扩展 AgentRetrievalPort、DeferredAgentRetrieval、RetrievalApi 与 query 合约为强制 context；隔离遗留无上下文入口和索引维护入口。
- [x] 4.2 从 owning API 的完整一致 metadata 游标分批构建 query-local 授权关系；不依赖 200 refs、不拼巨大 IN，取消/错误/连接复用都清理。
- [x] 4.3 FTS 在 rank/LIMIT 前、vector 在加载/打分前 JOIN/EXISTS 授权关系；复用 RRF、独立 top-k 与稳定排序，metadata 失败不回退全池。
- [x] 4.4 融合后只读有界合法候选正文，交付前重新复核；有限补取与 partial/unavailable 可辨别，不用旧索引文字替代权威记录。
- [x] 4.5 保持 keyword_only/vector_only/正常空结果/暂不可用区别；禁用 memory 时禁止 query embedding，生成继续。

## 5. 所有消费端

- [x] 5.1 注入索引/CLI index 的摘要也走当前资格与权威 metadata 检查，MEMORY.md 不直接用作 runtime ACL；保留每轮注入、budget/truncation 和 CLI index-only 边界。
- [x] 5.2 selector 返回 immutable ID/候选 handle，不按重名首个匹配；pinned_bodies 改走治理读取，合法 workspace/selected-agent 正文可实际注入。
- [x] 5.3 surfaced 去重按 session+Agent/seat+context 和 id/revision/hash；先判资格后去重，不把 mtime 或已展示当授权。
- [x] 5.4 recall catalog 与执行双门禁，模型伪造 scope 不生效；保留无 embedding 配置不注册 recall、注入仍可用。
- [x] 5.5 ContextRequest/Memory source 接通可信 read context，temporary/read-disabled 不运行该源，内部候选保留 memory provenance，不能被 protected 标志扩大资格。
- [x] 5.6 对宿主可识别的旧 memory result/candidate/重注入片段重验上下文；换 Agent/seat/工作区不复用旧授权，准确声明 CLI 历史与模型转述不可撤回。

## 6. UI、Web 与文档

- [x] 6.1 预览拆分 hypothetical/bound-session，真实会话由 native 解析身份；假设预览不产生 runtime authority，修复远程/legacy folder 表现不一致。
- [x] 6.2 扩展 read allowed、eligible 总数、index 条数/截断、recall availability 与安全原因，明确允许读但空池和索引尚未完成。
- [x] 6.3 Web 实现完整读取相关 policy 继承与 hard mode narrowing，修复 global/read 开关和空池误判；Tauri/Web/TS/DTO/mapper 同步。
- [x] 6.4 更新现有设置/预览文案，中英文和两主题/窄屏/可访问性；不新增重复开关，CLI 支持 index 不冒充支持 recall。
- [x] 6.5 更新开发者/用户手册跨会话记忆说明，澄清共享存储与受治理读取、worktree 身份、每代政策和独立预算。
- [x] 6.6 统一日志只记录有界安全指纹/授权计数/原因/延迟，模型不可见排除记录存在性；不新增 feature log 或原始内容持久化。

## 7. 验收与交付

- [x] 7.1 按 acceptance.md 运行真实 v2/SQLite/FTS 与固定 vector/provider fixture 的正反例；特别验证四入口、>200、scope/audience 正文、temporary 零 memory 调用与仍完成生成。
- [x] 7.2 跑并发上下文、撤销/修订、篡改 metadata、缓存/重注入、single-path degradation、reconcile 中断与 owner 管理隔离测试。
- [x] 7.3 对 201/1,000/10,000 条池测查询计划、候选/metadata/正文加载数、P50/P95 和峰值内存；断言结构预算，不以全部禁用或全量正文扫描通过。
- [x] 7.4 运行 `npm run lint:ci`、`npm run test`、`npm run build`。
- [x] 7.5 运行 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`npm run native:panic:check`、`cargo test --workspace`。
- [ ] 7.6 按当前 AGENTS.md 执行 `npm run contracts:check`、`npm run architecture:check`、`npm run desktop:unit:test`、`npm run test:desktop` 及 `npx playwright test`；按实际平台报告，不外推其他系统。
- [x] 7.7 运行本 change strict 及 `npx --yes @fission-ai/openspec@1.8.0 validate --specs --strict`，检查合并预览保留原场景和 overlap 协调；文档修改执行 `npm run docs:check`。
- [x] 7.8 仅勾选完成且有验证的任务；总结实际结果、NOT RUN/BLOCKED、外发/CLI边界与兼容变化。不自动归档、commit、push、merge 或发布。
