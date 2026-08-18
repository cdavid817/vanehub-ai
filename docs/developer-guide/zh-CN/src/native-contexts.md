# Native bounded context

native 代码按所有权组织,而非按 UI 页面组织。

| Context | 拥有 |
| --- | --- |
| `agent_runtime` | Agent 目录、provider 执行、终端会话、Loop、Multi-Agent 协调 |
| `sessions` | 会话、消息、分类、聊天配置、导出、用量 |
| `workspaces` | 项目、worktree、有边界的文件/Git 查询、PTY shell |
| `tooling` | CLI、MCP、SDK、扩展、插件、Skills、Prompt Hooks |
| `communications` | 连接器配置、凭据、路由、入站投递 |
| `desktop` | 设置、路径、启动、窗口、托盘与浮动生命周期 |
| `operations` | 可观测的操作以及统一诊断/操作日志契约 |
| `retrieval` | Agent 记忆与工作区代码索引、Tree-sitter 解析、FTS/向量搜索、embedding 确认与代码索引审计元数据 |

每个 context 对进程内消费方发布一个 `api.rs` facade。其他 context 不得直接深入其 repository 或基础设施模块。Bootstrap 模块在应用边界处组装具体的依赖。

Tauri command 是传输适配器,而非业务服务。跨 command 的错误值会被映射为安全的字符串或显式的传输错误 DTO。

## 检索与工作区代码

`retrieval` 拥有持久化的代码索引工作区标识、配置、文件清单、chunk、symbol、向量与有边界的本地审计记录。它在组装边界处消费工作区根目录,但不会导入 `workspaces` 的 repository。`agent_runtime` 只消费带类型的代码检索端口,并提供当前会话文件夹;模型无法向 `search_code` 提供工作区 id 或文件夹。

native worker 执行元数据优先的核对,只读取或解析新增或变更的文件。Tree-sitter grammar、chunk 拆分查询与脱敏策略共享一个版本标记。工作区代码 embedding 受一个显式确认的网关控制,该确认与工作区 id、generation、provider profile 和模型绑定。FTS 保持以工作区为作用域,并在确认之前就可用;来自另一个工作区或模型的向量永远不会成为候选。

native 诊断使用统一日志端口,且只包含安全的 id、阶段、计数、时长、模型 id 与原因类别。归一化的相对路径只保留在有边界的 SQLite 审计表中。原始代码、搜索查询、凭据、检测到的密钥值、绝对路径与 provider body 都被排除在代码索引诊断与遥测之外。

如需了解完整实现的 context 与 command 清单,请把 [`src-tauri/ARCHITECTURE.md`](../../reference/native-architecture.md) 与生成的 [native API 参考](native-api-reference.md) 一起阅读。
