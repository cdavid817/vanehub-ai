# Change: Unify governed memory read scope across runtime surfaces

## Why

注入已消费 personalization 的 scope/audience 策略，recall 仍使用无上下文的 `search(query, limit)`。实际回源又被旧 compatibility 视图限制为 active + global + all-agents：这既可能让 project-only / 禁用 global 的会话通过 recall 读到公共全局内容，也让合法 workspace / selected-agent 记忆无法召回。不能把这一现状描述为“所有私有工作区记忆都已泄露”。

同样的 compatibility 调用出现在 `pinned_bodies`，合法的工作区引用可进索引却加载不到正文。另有 Context Engine memory source 在 personalization snapshot 之前查询检索，是 temporary/read-disabled 可能绕过的独立入口。投影 audience 的 SQL LIKE、未知 scope 的默认分支及 Web preview 的读取策略计算也会造成范围分歧。

## What Changes

- 新建可信的内部 MemoryReadContext，复用现有 policy resolver，冻结 generation/seat 主体与读取策略，所有读取入口强制携带；模型输入仍只有 query/limit。
- 以同一领域谓词判定 lifecycle → effective read → scope → audience；精确匹配稳定 Agent ID，未知数据拒绝，不以作者、显示名或来源目录代替授权。
- 分开“完整资格域”和“注入候选页”：200 条引用、正文选择预算、recall top-k 各自独立，合法第 201 条之后的记忆仍可检索。
- 通过 owning-context API 提供完整授权 metadata 查询域，FTS/向量都在排名和 top-k 之前过滤，回源时再校验权威 id/revision/hash/当前范围。
- 接通合法 scoped memory 的索引与正文读取；owner 管理、后台索引、旧 compatibility 和 runtime read 使用不同类型边界，禁止把 compatibility 简单放宽为全池。
- 将 snapshot 解析前移至 Context Engine 之前；名称/description/索引行、selector、正文、recall、Context Engine 和宿主重注入均受相同判定。
- 修复正文选择的 immutable ID 引用及 surfaced 去重的 Agent/seat 上下文隔离；不改造抽取流程。
- 同步原生真实会话预览与 Web/mock 读取规则，区分有权限但空池、禁止读取、检索未配置和未支持通道。

## Capabilities

### New Capabilities

- `memory-read-scope`: 可信读取上下文、统一资格域、候选授权、权威交付与失败关闭规则。

### Modified Capabilities

- `agent-cross-session-memory`: 注入索引/正文、CLI 交付、ID 选择与 surfaced 规则。
- `retrieval-vector-search`: governed recall、索引源语义、两路过滤、故障降级与 Web 合约。
- `unified-personalization-governance`: 每代快照语义、真实会话绑定、精确资格预览和安全元数据。
- `agent-context-engine`: 记忆源必须在资格解析后运行，缓存/重注入保持来源与上下文绑定。

## Impact

桌面 Rust 改动涉及 agent_runtime、personalization、retrieval、sessions 的已发布边界及 bootstrap wiring。React 仅经 AgentService；Tauri/Web adapter 保持类型与行为一致。Markdown v2 文件继续权威，SQLite/FTS/embedding 是派生数据。按需 additive migration 派生表，不改变现有 memory id、scope/audience、用户 policy 或来源历史。

检索语义存在有意兼容修正：移除“所有 Agent/目录无条件可召回”和“结果不得是注入结果的子集”旧保证，替换为同一资格域、独立排序预算。保留无 embedding 配置不注册 recall、正常无命中为空成功、单路故障降级和检索故障不终止生成。CLI 继续按自身能力提供 index-only，本变更不为黑盒 CLI 新建 recall 协议或改写它的原生记忆。

已存在的 `strengthen-governed-cross-session-memory` 覆盖部分同名规范；本变更完整定义该读取子范围及接口，不以其抽取/episode 项完成作为前置条件。实施与后续归档必须协调重叠条款，不能用旧 delta 覆盖新主规范。

## Non-Goals

- 不新增 repository scope、角色 audience、记忆 schema v3 或跨 worktree 自动共享。
- 不改造自动抽取、候选审批幂等、记忆 episode、评估飞轮或通用权限系统。
- 不声称撤回已经交付给模型的历史内容，也不把 audience 等同于操作系统文件保密沙箱。
- 不因新增读取资格自动扩大记忆正文发送给外部 embedding provider 的授权。

## Delivery Gates

真实临时 Markdown v2 + SQLite + 两路检索 fixture 验证四条读取路径；合法 workspace/selected-agent 正文必须成功，不允许全部返回空作为修复。temporary/read-disabled 时 memory selector、memory source、query embedding 和正文加载均零调用，生成仍完成。覆盖 >200 条合法记忆、越权 top-k 挤占、权威文件变化、同名不同 ID、不同 seat/worktree/远程连接和 Web/native 判定一致性。只通过 OpenSpec 校验不代表实现完成。
