# LSP 代码智能

原生 LSP 基础能力由独立的桌面端限界上下文负责。它为原生 API Agent 提供实时语义代码智能，但不把持久化 Tree-sitter 代码索引作为进程或配置前置条件。

## 运行基线

第一阶段只支持以下服务器：

| 语言族 | 可执行文件 | 固定启动行为 | 项目根标记 |
| --- | --- | --- | --- |
| Rust | `rust-analyzer` | stdio LSP | 最近的 `Cargo.toml` |
| TypeScript/JavaScript | `typescript-language-server` | VaneHub 自动附加 `--stdio` | 最近的 `tsconfig.json`、`jsconfig.json` 或 `package.json` |

使用 rustup 管理的标准 Rust 工具链时，执行：

```bash
rustup component add rust-analyzer
rustup component add rust-src
```

安装 TypeScript 服务器及其 TypeScript runtime：

```bash
npm install -g typescript-language-server typescript
```

平台打包方式或前置要求不同时，以 [rust-analyzer 二进制安装指南](https://rust-analyzer.github.io/book/rust_analyzer_binary.html)和 [TypeScript Language Server 仓库](https://github.com/typescript-language-server/typescript-language-server#installing)为准。

在**设置 > Agent 配置**中打开总开关和语言开关，确认自动发现结果或填写绝对可执行文件覆盖路径，保存有界的初始化选项对象，执行隔离服务器测试，再显式信任规范化的本地工作区。所有开关和信任记录默认都是停用状态。启用代码索引不等于授予 LSP 信任。

## 所有权与边界

原生侧主要所有权位于 `src-tauri/src/contexts/code_intelligence/`：

| 分层 | 职责 |
| --- | --- |
| `domain/` | 语言/服务器身份、信任、配置、进程状态、能力、版本、规范化位置、诊断和 fail-soft 结果 |
| `application/` | 仓储与原生环境端口 |
| `infrastructure/` | 发现、项目根、进程注册表、JSON-RPC、分帧、初始化协商、文档租约、诊断、结果规范化、服务器测试、关闭和统一诊断 |
| `api.rs` | 唯一的跨上下文代码智能门面 |

`agent_runtime` 拥有消费方端口 `AgentCodeIntelligencePort` 和 `AgentWorkspaceMutationPort`。Bootstrap 把这些端口适配到 `CodeIntelligenceApi`；Agent 代码不得导入代码智能基础设施。Retrieval 通过独立的公开 `CodeIndexApi` 接收定向变更协调，两个上下文不共享内部实现。

前端继续遵守统一服务边界：

```text
React 设置组件
  -> AgentService
    -> tauri-agent-client.ts -> 已注册 Tauri command -> CodeIntelligenceApi
    -> web-agent-client.ts   -> 确定性内存 Web/mock 适配器
```

React 组件不得直接调用 `invoke()`。Web/mock 代码不得导入原生文件系统或进程适配器，也不得声称真实服务器已经启动。

## 进程与协议生命周期

进程以规范化会话根、检测到的项目根、服务器类型和配置指纹作为联合键。同一工作区中的嵌套项目因此可以使用同类服务器的独立实例。

```text
absent -> starting -> initializing -> ready -> stopping -> absent
                    \-> backoff -> starting
                    \-> failed
```

- 工具请求按需启动满足条件的服务器；有界语言清单可以发出预热提示。
- `ready` 表示 `initialize`/`initialized` 握手完成，不表示服务器后台索引已经结束。
- 意外退出会使待处理请求失败，清除文档和诊断状态，并进入有界指数退避。
- 重启预算耗尽后进入 `failed`，直到冷却路径允许使用新的预算。
- 就绪进程连续十分钟没有活动请求或文档租约后自动关闭。
- 配置替换和信任撤销复用同一个排空停止路径。
- 应用退出时并发停止服务器，先尝试 `shutdown` 和 `exit`，再在全局截止时间内强制终止剩余进程树。

传输层是基于子进程 stdin/stdout 的有界 JSON-RPC 2.0。`Content-Length` header、协议帧、stderr 捕获、队列、待处理请求、并发请求、服务器通知和规范化输出都有硬上限。只读基础阶段会拒绝服务器发起的工作区编辑。

## 文档、位置与诊断

磁盘内容是权威版本。语义请求发出前，文档准入会规范化相对路径，并拒绝绝对路径、目录穿越、隐藏路径、非文件、二进制、无效 UTF-8、超限文件和符号链接逃逸。VaneHub 不维护未保存的编辑器缓冲区。

第一次请求发送 `didOpen`。磁盘快照变化后递增版本，并根据协商结果发送全文或单段连续增量 `didChange`；空闲或服务器停止时发送 `didClose`。Agent 的准确文件写入立即使匹配租约失效。Shell、Git 和外部编辑器的变化不依赖 watcher，而是在下次请求磁盘内容时检测。

Agent 坐标和规范化结果范围使用 1-based。协议坐标使用 0-based 和协商的位置编码，未选择编码时回退到 UTF-16。诊断通知会替换每个文档的版本化快照；当前空结果、过期、预热、超时、不可用和失败状态不能混为一谈。

## Agent 工具与硬上限

Provider-neutral catalog 会在普通会话和 Plan Mode 中按条件暴露四个只读工具：

| 工具 | 协议方法或来源 | 上限 |
| --- | --- | --- |
| `find_definition` | `textDocument/definition` | 20 个有效位置 |
| `find_references` | `textDocument/references` | 50 个有效位置，确定性排序 |
| `get_hover` | `textDocument/hover` | 有界签名、文档和序列化输出 |
| `get_diagnostics` | `textDocument/publishDiagnostics` 缓存 | 有界条目数和消息内容 |

工作区范围始终来自当前会话。模型不能选择工作区、根目录、服务器路径或 URI scheme。规范化后只保留当前工作区内通过准入的 `file:` 位置。

每个结果使用 `ready`、`warming`、`timeout`、`unavailable` 或 `failed`，可选代码智能失败不会终止 Agent generation。只有 `ready` 加空结果表示成功但没有找到内容。带计数的结果保留有效总数、返回数、过滤、过期和截断元数据。

## Tree-sitter 检索与 LSP

两项能力解决不同问题，并保持独立所有权：

| 维度 | Tree-sitter `search_code` | 实时 LSP |
| --- | --- | --- |
| 所有者 | `retrieval` | `code_intelligence` |
| 状态 | 持久化清单、代码块、符号、FTS 和可选向量 | 临时进程、文档、能力和诊断 |
| 查询形态 | 对工作区索引执行文本或语义检索 | 准确文档位置或诊断文档 |
| 语义深度 | 语法结构和可选 embedding 相似度 | 编译器/语言服务器级定义、引用、类型和诊断 |
| 激活条件 | 每工作区索引配置 | 总开关、语言开关、可执行文件、本地会话和显式信任 |
| 独立可用性 | 不需要语言服务器 | 不需要持久化代码索引 |

Agent 文件写入成功后发布一条 best-effort 变更信号。Bootstrap 使 LSP 租约失效，并把规范化路径交给有界、合并重复项的代码索引队列。任何一路后续失败都不会改变已经成功的文件工具结果。

## 持久化与日志

SQLite 保存默认停用的主机级配置和规范化工作区信任记录。可执行文件、固定参数、初始化选项和信任修订共同形成配置指纹，旧配置进程不能继续服务新请求。

生命周期和协议诊断必须写入统一日志。安全元数据包括服务器/语言身份、生命周期转换、方法类别、耗时、计数、重启次数、超时/取消类别、退出码和安全工作区身份。不得持久化原始协议 payload、源代码或悬停内容、诊断消息、stderr、环境值、可执行参数、凭据或私有绝对路径。

## 扩展限制

基础阶段明确不包含 Python、Go、Java、C/C++、远程工作区、动态下载服务器、格式化、补全、重命名、Code Action、工作区编辑、调用/类型层级、文件系统 watcher、未保存缓冲区和持久化 LSP 增强索引。不能只在 catalog 中新增名称就暴露可变更方法；它需要独立 OpenSpec 变更、权限分析、Plan Mode 处理、协议上限和工作区隔离测试。

LSP 没有标准化可移植的服务器内存或已索引文件数，因此状态契约必须保持这些指标不受支持，不能虚构数值。

## 排障与验证

- **发现失败**：比较桌面进程和交互式 shell 能看到的可执行文件，再测试绝对覆盖路径。配置的覆盖路径无效时不得静默回退。
- **启动失败**：检查 runtime 依赖和可执行权限，但日志中不得记录环境或参数。
- **初始化失败或超时**：查看隔离测试阶段和安全原因；错误能力必须 fail closed，清理仍必须运行。
- **等待重启或重启耗尽**：检查进程注册表快照和限流后的安全诊断。修复原因后再重置信任或配置。
- **诊断过期**：核对本地文档版本，只在有界查询截止时间内等待替代通知。
- **工具缺失**：核对桌面 runtime、本地工作区、总开关/语言开关、可执行文件发现、显式信任、文件语言和 catalog 资格。
- **返回位置消失**：先检查 URI scheme、规范化包含关系、文档准入、位置转换和结果上限，再判断是否为传输丢失。

常用的原生定向校验：

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib contexts::code_intelligence
cargo test --manifest-path src-tauri/Cargo.toml --test architecture
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

前端适配器、组件和 Web/mock 行为由 Vitest 覆盖；文档中的设置流程由 LSP Playwright 场景覆盖。提交前还要运行 `AGENTS.md` 中的全部仓库校验命令。
