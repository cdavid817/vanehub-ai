# LSP 代码智能

语言服务器协议（LSP）集成让原生 API Agent 可以向本地语言服务器查询定义、引用、悬停信息和当前诊断。该功能默认停用；必须先启用对应语言，并为每个本地工作区显式授予信任。

## LSP 是什么

LSP 出现之前,每个编辑器要支持"智能感知"(补全、跳转定义、错误提示)都得为每种语言各写一套插件——这是经典的 **M×N 问题**:M 个编辑器 × N 种语言 = M×N 套独立实现。微软在 2016 年提出 LSP,把它拆成 **M+N**:

- 每种语言只实现**一个** Language Server(懂这门语言的语法与类型系统)
- 每个编辑器只实现**一个** LSP Client(懂如何与任意 Language Server 通信)

两边通过统一协议对话,互不关心对方内部实现。VaneHub AI 扮演的就是 Client 一侧——它替 Agent 跟本地已装的 Language Server 说话。

**通信方式**:传输层通常是 stdio(子进程管道);消息格式是 JSON-RPC 2.0,每条消息带 `Content-Length` 头 + JSON 体;消息分两类——Request/Response(一问一答,如"这个符号定义在哪")与 Notification(单向通知,不需回复,如"文件内容变了")。

**生命周期**:Client 启动 Server 子进程 → 发 `initialize` 声明自己支持哪些能力 → Server 回自己支持哪些能力,握手完成 → 进入正常工作 → `shutdown` → `exit` 优雅关闭。这个**能力协商**很关键:双方都可以只实现协议的子集,通过 capabilities 字段知道对方能干什么。

### LSP 的核心功能

协议本身覆盖的能力远不止 VaneHub AI 暴露给 Agent 的这四个:

| 功能 | 方法名 | 作用 |
| --- | --- | --- |
| 跳转定义 | `textDocument/definition` | Go to Definition |
| 查找引用 | `textDocument/references` | Find All References |
| 悬浮提示 | `textDocument/hover` | 显示类型/文档 |
| 诊断报错 | `textDocument/publishDiagnostics` | 实时语法/类型错误(**Server 主动推送**) |
| 自动补全 | `textDocument/completion` | 光标处候选列表 |
| 重命名 | `textDocument/rename` | 跨文件安全重命名 |
| 代码操作 | `textDocument/codeAction` | 快速修复、重构建议 |
| 格式化 | `textDocument/formatting` | 代码格式化 |
| 语义高亮 | `textDocument/semanticTokens` | 比正则高亮更准确的着色 |
| 大纲符号 | `textDocument/documentSymbol` | 文件结构树 |

文档同步方面,Client 用 `didOpen`/`didChange`/`didClose` 把文件状态实时告诉 Server,Server 内部维护文档快照做增量分析。**VaneHub AI 只向 Agent 暴露只读能力**——重命名、代码操作、格式化这些会改文件的能力不开放,见下文[限制与结果状态](#限制与结果状态)。

## 为什么 Agent 需要 LSP

- **精准上下文提取** —— 相比"整个文件塞给模型"或纯 grep 抓取,LSP 给的是**语义级**信息:某函数的所有调用点、某类型的完整定义、跨文件符号解析。这比单纯的 AST/正则更准,因为 Language Server 做了真正的类型检查与跨模块解析。
- **降低"幻觉编辑"风险** —— Agent 改代码前可以先用 definition/references 确认影响面,而不是靠猜。
- **诊断回路** —— 改完代码后直接拿到编译器/类型检查器的报错,形成"编辑 → 验证 → 修正"闭环,不用等你手动跑 build。
- **成本权衡** —— 启动 Language Server(尤其 rust-analyzer 对大 workspace 的首次索引)有可观的时间与内存开销。这正是 VaneHub AI 把 LSP 做成**默认停用、按语言启用、按工作区授信**的原因,而不是每个会话都拉起一个实例。

## 支持的服务器与工具

| 语言 | 服务器 | 默认启动参数 | 项目根标记 |
| --- | --- | --- | --- |
| Rust | `rust-analyzer` | 无 | 最近的 `Cargo.toml` |
| TypeScript 与 JavaScript | `typescript-language-server` | `--stdio` | 最近的 `tsconfig.json`、`jsconfig.json` 或 `package.json` |
| Go | `gopls` | 无 | 最近的 `go.mod` |
| Python | `basedpyright-langserver`，否则 `pyright-langserver` | `--stdio` | 最近的 `pyproject.toml`、`setup.py`、`setup.cfg` 或 `requirements.txt` |
| C 与 C++ | `clangd` | 无 | 最近的 `compile_commands.json`，或 `build/compile_commands.json` |

**C/C++ 是唯一不会回退的语言**。其它语言在找不到标记时会把工作区根目录当作项目根；而 `clangd` 没有编译数据库就会假定默认编译参数，然后给出**看起来很确定但其实是错的**定义和诊断，所以 VaneHub AI 宁可把请求报为不可用。

两个 Python 服务器都装了时优先用 `basedpyright-langserver`：装这个分支是一个刻意的选择，装上游的那个不是。发现面板会写明实际选中了哪一个。

支持哪些语言由桌面构建决定，不由设置页决定。设置页按当前构建注册的语言逐个渲染卡片：构建不认识的语言无法配置；构建认识但在当前操作系统上跑不起来的语言，会显示为**不支持**，而不是显示成"没找到可执行文件"。

Agent 可以使用九个只读工具：

| 工具 | 返回内容 | 是否每个服务端都提供 |
| --- | --- | --- |
| `find_definition` | 工作区相对的定义位置和有界代码预览 | 实践中是 |
| `find_references` | 确定性排序的引用，最多返回 50 条 | 实践中是 |
| `get_hover` | 有界的类型签名和文档 | 实践中是 |
| `get_diagnostics` | 当前或明确标记为过期的版本化诊断 | 始终 |
| `find_type_definition` | 符号**类型**的声明位置，最多 20 条 | 否 |
| `find_implementations` | 接口、trait 或抽象成员的实现，最多 20 条 | 否 |
| `find_workspace_symbols` | 单个项目内按名称匹配的符号，最多 50 条 | 否 |
| `get_document_symbols` | 一个文件里的声明，扁平化，每条都标注它的外层符号 | 否 |
| `find_call_hierarchy` | 某个函数的调用方，或它调用了什么，最多 50 条 | 否 |

后五个值得单独说明：服务端可能根本不提供它们。工具仍然存在、仍然会回答——回答的是 `unavailable` 状态而不是沉默——因为"服务端是否支持某个方法"是在它启动时协商出来的，不是在会话开始时就能知道的。`gopls` 和 `rust-analyzer` 九个全都提供；更老或更小的服务端往往只到前四个。运行状态卡片列出的是当前服务端**实际**协商出来的能力。

`find_workspace_symbols` 除了查询词还要一个文件路径。这个路径不是过滤条件，它决定搜索哪个项目的索引：一个仓库里可以有多个同语言项目，而一个语言服务器一次只索引其中一个。

当前本地工作区满足条件时，这些工具会同时出现在普通会话和 Plan Mode 中。本阶段不支持 Java 语言服务器。

## 安装语言服务器

### Rust

使用当前稳定版 Rust 工具链安装 `rust-analyzer`：

```bash
rustup component add rust-analyzer
rustup component add rust-src
rust-analyzer --version
```

其他平台安装方式见上游 [rust-analyzer 安装指南](https://rust-analyzer.github.io/book/rust_analyzer_binary.html)。

### TypeScript 与 JavaScript

使用 npm 安装语言服务器和 TypeScript runtime：

```bash
npm install -g typescript-language-server typescript
typescript-language-server --version
```

`--stdio` 是这个服务器的默认启动参数，由 VaneHub AI 提供；如果你的安装需要别的参数，可以在**启动参数**中替换。当前前置要求见上游 [TypeScript Language Server 项目](https://github.com/typescript-language-server/typescript-language-server#installing)。

### Go

```bash
go install golang.org/x/tools/gopls@latest
gopls version
```

`gopls` 装在 `$(go env GOPATH)/bin`，这个目录不一定在桌面应用继承到的 `PATH` 里。前置要求见上游 [gopls 安装指南](https://pkg.go.dev/golang.org/x/tools/gopls#section-readme)。

### Python

```bash
npm install -g basedpyright   # 或：npm install -g pyright
basedpyright-langserver --help
```

两个服务器都可用，`--stdio` 由 VaneHub AI 提供。前置要求见 [basedpyright](https://docs.basedpyright.com/) 或 [pyright](https://microsoft.github.io/pyright/#/installation)。

### C 与 C++

`clangd` 随 LLVM 分发，用系统包管理器安装即可；然后需要为每个项目生成编译数据库：

```bash
clangd --version
# 在要被服务的项目里，用 CMake：
cmake -S . -B build -DCMAKE_EXPORT_COMPILE_COMMANDS=ON
```

生成的 `build/compile_commands.json` 才是让项目可被识别的东西。其它构建系统(含 Make 项目用的 `bear`)见上游 [clangd 安装指南](https://clangd.llvm.org/installation)。

## 为工作区启用 LSP

以下步骤需要桌面客户端：

1. 打开**设置 → Agent 配置**，找到**语言服务器智能**。
2. 打开**启用 LSP 集成**，再启用 Rust 和/或 TypeScript/JavaScript。
3. 选择**刷新发现结果**。如果桌面进程看不到可执行文件，在**可执行文件覆盖路径**中填写绝对路径。
4. **启动参数**留空即使用上表中的默认值。要自定义就每行填一个参数——你填的列表会**替换**默认值而不是追加，所以需要 `--stdio` 的服务器必须自己列上它。填了内容但解析后为空列表，含义是"启动时不带任何参数"，这与留空不是一回事。
5. 除非确实需要服务器专用配置，否则把初始化选项保留为 `{}`。该值必须是有大小限制的 JSON 对象。
6. 保存配置。
7. 在**测试语言服务器**中执行隔离测试，检查发现、启动进程、初始化和清理四个阶段。
8. 阅读信任说明，在**受信任的工作区**中填写本地工作区绝对路径，然后选择**信任工作区**。
9. 为该本地工作区打开原生 API Agent 会话。无需启用代码索引，四个 LSP 工具也可以生效。

改动启动参数就是改动服务器运行时的命令行，因此该配置下已在运行的服务器会先排空并重启，再处理下一个请求。服务器测试使用隔离的最小项目，不会授予工作区信任。启用 Tree-sitter 代码索引也不会授予 LSP 信任。

## 理解工作区信任

语言服务器是使用当前操作系统账户权限运行的本地可执行文件。工作区信任只控制 VaneHub AI 的自动启动，并把 Agent 可见结果限制在当前工作区；它不是操作系统沙箱。如果账户有权限，服务器仍可能访问工作区之外的资源。只信任你了解的仓库和可执行文件路径。

撤销信任会拒绝新的请求，并停止该工作区拥有的语言服务器进程。远程和 SSH 工作区不能启动本地 LSP。

## 读取生命周期状态

| 状态 | 含义 |
| --- | --- |
| **未运行** | 当前没有进程；满足条件的工具调用可以按需启动。 |
| **正在启动** | 正在启动可执行文件。 |
| **正在初始化** | 正在协商 LSP 能力和文档行为。 |
| **就绪** | 已允许协议请求；后台项目索引可能仍在继续。 |
| **等待重启** | 意外退出触发了有界指数退避。 |
| **失败** | 重启预算已耗尽，或发生终止性的安全错误。 |
| **正在停止** | VaneHub AI 正在执行协议关闭和进程清理。 |

处于就绪状态且十分钟内没有活动请求或文档租约的服务器会自动关闭。配置或信任变化会先排空受影响的进程，再启动替代实例。桌面应用退出时会依次发送 `shutdown` 和 `exit`；只有有界清理无法完成时才强制终止进程。

## 选择 `search_code` 还是 LSP

| 需求 | Tree-sitter `search_code` | 实时 LSP 工具 |
| --- | --- | --- |
| 按名称、文本或含义发现代码 | 更适合 | 需要准确文件位置或诊断目标 |
| 编译器级定义和引用 | 不支持 | 支持 |
| 当前类型、悬停文档和诊断 | 不支持 | 支持 |
| 不启动外部服务器进程 | 支持 | 不支持 |
| 跨会话保留索引 | 支持 | 不支持 |
| 首阶段语言覆盖 | 八类语言 | Rust 与 TypeScript/JavaScript |

两项能力互为补充。Agent 成功修改文件后，系统会使已打开的 LSP 文档失效；如果代码索引已启用，还会同时排队执行定向索引协调。

## 限制与结果状态

- 磁盘内容是权威版本；VaneHub AI 不维护未保存的编辑器缓冲区。
- 本阶段没有文件系统 watcher。Agent 写入会立即使准确路径失效；shell、Git 或外部编辑器的改动会在下一次语义查询前检测。
- 定义最多返回 20 条，引用最多返回 50 条；结果会保留 `total` 和 `truncated` 元数据。
- 悬停文本、预览、诊断、协议帧、队列和完整工具输出都有硬上限。
- `ready` 且结果为空表示服务器没有找到内容；`warming`、`timeout`、`unavailable`、`failed` 和过期诊断是不同的降级状态。
- 本阶段不包含补全、重命名、格式化、Code Action、工作区编辑、调用/类型层级或持久化 LSP 增强索引。
- LSP 服务器没有标准化可移植的进程内存和已索引文件数，因此状态面板不报告这些指标。

## 故障排查

### 没有发现可执行文件

先在普通终端运行版本命令。桌面应用看到的 `PATH` 可能与交互式 shell 不同，尤其是从图标启动时。修改 `PATH` 后重启 VaneHub AI，或填写可执行文件的绝对覆盖路径。

### 隔离服务器测试失败

根据失败阶段缩小范围：

- **发现**：可执行文件不存在，或手动覆盖路径无效。
- **启动进程**：依赖、权限或可执行文件自身阻止了启动。
- **初始化**：服务器拒绝最小项目、返回了错误能力，或初始化超时。
- **清理**：优雅关闭失败，强制清理也未完成。

### C/C++ 请求报告缺少项目标记

`clangd` 装好了、发现面板也显示可用，但工作区里没有编译数据库，于是没有任何东西可以用来服务这次请求。生成一个即可——`cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON`、`bear -- make`，或你的构建系统的等价命令——放在项目根目录或其 `build` 子目录下。这与"没装上"是刻意区分开的两件事：服务器本身没问题，读不了的是项目。

### 某种语言显示为"当前操作系统不支持"

这与"没找到可执行文件"是两回事。当前构建注册了这门语言，但声明它的服务器在这个平台上不运行——所以没有可安装的东西，启用开关和隔离测试都不可用。"没找到可执行文件"则显示为**不可用**并附带原因，安装服务器或填写覆盖路径即可解决。

### Agent 没有获得 LSP 工具

确认正在使用桌面端；总开关和对应语言开关已启用；服务器发现结果可用；当前规范化本地工作区已信任；会话文件属于支持的语言。远程会话不能启动原生 LSP 工具。

### 结果显示 warming、过期或已截断

`warming` 通常表示进程仍在启动或项目仍在索引。过期诊断属于旧的磁盘文档版本，可在分析追上后重新请求。截断结果仍然有效，但受到硬上限约束；可以缩小目标符号，并查看返回的 `total`。

### 服务器反复进入等待重启或失败

查看**运行状态**中的安全原因，修复可执行文件、项目或初始化选项，再重新执行隔离测试。撤销并重新授予信任会停止旧进程，但不会修复无效的服务器安装。

## 相关

- 持久化索引与实时 LSP 的分工 → [代码索引](code-indexing.md)
- 语言服务器开关所在的设置页 → [工具与扩展](tooling.md#agent-配置)
- LSP 协议本身：分层与生命周期、能力协商、文本同步模型 → [LSP 技术架构](../../../agent-infrastructure/lsp-architecture.md)
