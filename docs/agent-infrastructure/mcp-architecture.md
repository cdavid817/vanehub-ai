# MCP（Model Context Protocol）技术架构

> VaneHub AI 技术文档 · Agent 基础设施系列
>
> 本文介绍 MCP 的完整技术体系：协议模型与三角色架构、传输层、核心原语、协议生命周期（含 2026-07-28 无状态化重大变更）、扩展框架、授权与安全模型。适用于 MCP 客户端集成（rmcp）、多 CLI 工具的 MCP 配置管理与权限治理的实现参考。
>
> 本文以当前最新规范 **2026-07-28** 为基准，同时覆盖旧版（2025-11-25 及更早）的行为差异——生态中大量存量 Server/CLI 仍运行在旧版协议上，客户端实现必须双栈兼容。

---

## 1. 概述

### 1.1 定义

MCP（Model Context Protocol，模型上下文协议）是 Anthropic 于 2024 年 11 月发布的开放标准，为 AI 应用与外部工具、数据源之间定义统一的连接接口。其目标是把「M 个 AI 应用 × N 个数据源」的 M×N 集成问题降为 M+N：任一实现协议的 Server 可被任一实现协议的 Client 使用。

常见类比：**MCP 之于 AI 应用，如同 USB-C 之于外设**——统一的物理接口，两侧各自演化互不耦合。

### 1.2 与 Function Calling 的关系

二者不是竞争关系，而是不同层次：

| 层次 | Function Calling | MCP |
|------|-----------------|-----|
| 定位 | 模型能力：LLM 决定"调什么、传什么参数" | 应用层协议：工具如何被发现、描述、调用、治理 |
| 作用域 | 模型 ↔ 应用之间的结构化输出约定 | 应用 ↔ 外部世界之间的标准化连接 |
| 生态 | 各家模型厂商 API 各自定义 | 跨厂商开放标准 |

典型链路：LLM 通过 function calling 产出工具调用意图 → Host 将其映射为 MCP `tools/call` 请求 → Server 执行并返回结果 → Host 拼回模型上下文。

### 1.3 解决的问题

- **集成碎片化**：每个 AI 应用为每个数据源写一次适配器 → 一次实现，处处可用
- **能力发现**：工具、资源、提示词以标准 schema 自描述，客户端可动态发现
- **治理切面**：调用统一经过协议层，为权限控制、审计、可观测性提供天然的拦截点

---

## 2. 架构模型：Host / Client / Server

MCP 定义三个角色：

```mermaid
flowchart LR
    subgraph HostApp["Host（宿主应用）"]
        direction TB
        LLM[LLM 会话与编排]
        POL[权限策略 / 用户同意 / 审计]
        C1[MCP Client #1]
        C2[MCP Client #2]
        C3[MCP Client #3]
    end

    C1 <-->|"stdio (JSON-RPC)"| S1["Server: 文件系统<br/>本地子进程"]
    C2 <-->|"Streamable HTTP"| S2["Server: GitHub<br/>远程服务"]
    C3 <-->|"Streamable HTTP + OAuth"| S3["Server: 企业内部系统<br/>远程服务"]

    S1 --- D1[(本地磁盘)]
    S2 --- D2[(GitHub API)]
    S3 --- D3[(内部数据库)]
```

- **Host（宿主）**：面向用户的 AI 应用（IDE、桌面 Agent 应用、聊天客户端）。负责创建与管理多个 Client、执行安全策略与用户授权、聚合各 Server 能力并编排 LLM 会话。
- **Client（客户端）**：Host 内部的协议端点，**与 Server 一对一连接**。负责协议协商、消息路由、能力隔离。一个 Host 挂 N 个 Server 就持有 N 个 Client 实例。
- **Server（服务端）**：暴露具体能力（工具/资源/提示词）的程序，可以是本地子进程，也可以是远程 HTTP 服务。

> **设计要点**：一对一的 Client-Server 关系是有意为之的安全边界——每个 Server 只能看到自己连接内的信息，Host 掌握全局视图并充当"防火墙"，Server 之间互不可见。这也是 Host 侧实现 PDP/PEP 权限模型的天然挂载点：策略决策在 Host（PDP），执行拦截在每个 Client 连接（PEP）。

---

## 3. 协议基础

### 3.1 消息层：JSON-RPC 2.0

所有 MCP 消息均为 JSON-RPC 2.0，三种消息类型：

| 类型 | 结构 | 语义 |
|------|------|------|
| Request | `{jsonrpc, id, method, params}` | 期待响应 |
| Response | `{jsonrpc, id, result \| error}` | 对某 Request 的应答 |
| Notification | `{jsonrpc, method, params}`（无 id） | 单向通知，不期待响应 |

错误码分区（2026-07-28 起正式化）：JSON-RPC 保留区（`-32700`~`-32600` 系列）、实现自定义区 `-32000`~`-32019`、MCP 规范保留区 `-32020`~`-32099`（如 `HeaderMismatch -32020`、`UnsupportedProtocolVersion -32022`）。

### 3.2 版本机制与演进时间线

MCP 采用日期式版本号（`YYYY-MM-DD`），各版本关键变化：

```mermaid
timeline
    title MCP 规范版本演进
    2024-11-05 : 首个正式版
               : stdio + HTTP+SSE 双传输
               : Tools / Resources / Prompts / Sampling / Roots
    2025-03-26 : OAuth 2.1 授权框架
               : Streamable HTTP 取代 HTTP+SSE
               : Tool Annotations
    2025-06-18 : 移除 JSON-RPC batch
               : Elicitation（服务端向用户请求输入）
               : 结构化工具输出 / Resource Links
               : MCP 服务器归类为 OAuth Resource Server
    2025-11-25 : URL 模式 Elicitation
               : Tasks（实验性）
               : 授权增强（含 CIMD 方向）
    2026-07-28 : 无状态核心（移除 initialize 握手与会话）
               : MRTR 取代服务端发起请求
               : Extensions 框架（Tasks / MCP Apps）
               : Roots / Sampling / Logging 弃用
               : subscriptions/listen 统一订阅流
```

**双栈兼容是客户端的现实约束**：规范有正式的特性生命周期政策（Active → Deprecated → Removed，弃用窗口至少 12 个月），存量 Server 长期停留在旧版本。客户端应以 2026-07-28 为主实现，同时保留对旧版握手流程的探测与回退。

---

## 4. 传输层

### 4.1 stdio 传输

Server 作为 Host 的**子进程**运行，通过标准输入/输出交换换行分隔的 JSON-RPC 消息：

- 消息流：Client 写 Server 的 `stdin`，Server 写 `stdout`；`stderr` 保留给日志（2026-07-28 弃用协议级 Logging 后，stderr 日志是官方推荐迁移路径之一）
- 约束：`stdout` **只能**输出协议消息——Server 若向 stdout 打印调试信息会直接破坏协议流（集成 CLI 工具时的高频事故源）
- 适用：本地工具（文件系统、git、本地数据库），零网络配置、进程生命周期即连接生命周期

> **工程注意（PTY 场景）**：MCP stdio 传输要求干净的管道语义。若宿主以 PTY 方式管理 CLI 子进程（为了终端交互），MCP Server 子进程应走独立的普通 pipe 而非 PTY——PTY 的回显、行编辑与控制序列会污染 JSON-RPC 流。

### 4.2 Streamable HTTP 传输

远程 Server 的标准传输。核心机制：

- Client 通过 **HTTP POST** 发送 JSON-RPC 请求到单一 MCP endpoint
- Server 响应可以是普通 JSON，也可以升级为 **SSE 流**（`Content-Type: text/event-stream`），用于在单个请求的响应流上推送进度通知、流式结果
- 2026-07-28 要求 POST 携带标准头 `Mcp-Method` / `Mcp-Name`（使中间层无需解析 body 即可路由与审计），并支持经 `x-mcp-header` 从工具参数注入自定义头

**旧版（≤2025-11-25）**：存在协议级会话（`Mcp-Session-Id` 头）、独立的 HTTP GET 长连接用于服务端主动通知、SSE 断线续传（`Last-Event-ID`）。**这些在 2026-07-28 全部移除**（见 §6）。

**HTTP+SSE 传输**（2024-11-05 引入、2025-03-26 起被 Streamable HTTP 取代）已正式进入弃用状态，新实现不应支持。

### 4.3 传输对比

| 维度 | stdio | Streamable HTTP |
|------|-------|-----------------|
| 部署形态 | 本地子进程 | 远程服务 |
| 认证 | 进程信任边界（继承宿主权限） | OAuth 2.1 |
| 扩展性 | 单机 | 2026-07-28 起可无状态水平扩展 |
| 典型场景 | 文件系统、本地开发工具 | SaaS 集成、企业服务 |

---

## 5. 核心原语（Primitives）

### 5.1 服务端原语

#### Tools（工具）——模型可调用

Server 暴露给 **LLM** 决策调用的可执行能力。

```mermaid
sequenceDiagram
    participant U as 用户
    participant H as Host (含 LLM 编排)
    participant C as MCP Client
    participant S as MCP Server

    U->>H: "帮我查一下仓库的 open issues"
    H->>C: tools/list
    C->>S: tools/list
    S-->>C: 工具清单 (name, description, inputSchema, ttlMs)
    C-->>H: 工具清单
    H->>H: 将工具 schema 注入 LLM 上下文
    H->>H: LLM 产出 tool_use: list_issues(state="open")
    H->>U: 请求用户授权（策略允许时可自动放行）
    U-->>H: 同意
    H->>C: tools/call {name, arguments}
    C->>S: tools/call
    S->>S: 执行（调 GitHub API）
    S-->>C: result {content[], structuredContent, isError}
    C-->>H: 结果
    H->>H: 结果拼回上下文，LLM 生成回答
    H-->>U: 回答
```

工具定义关键字段：

- `name` / `title` / `description`：LLM 依据这些文本选择工具——描述质量直接决定调用准确率
- `inputSchema` / `outputSchema`：JSON Schema（2026-07-28 起放开为完整 JSON Schema 2020-12 关键字集，含 `$ref` 解析规则）
- `annotations`：行为提示，如 `readOnlyHint`、`destructiveHint`、`idempotentHint`、`openWorldHint`——**仅是提示，不是安全保证**，Host 不得据此跳过授权
- 返回：`content[]`（text / image / audio / resource_link / 嵌入资源）+ 可选 `structuredContent`；业务失败用 `isError: true` 表达（让 LLM 可见并自我修正），协议失败才用 JSON-RPC error

#### Resources（资源）——应用可读取

以 URI 标识的只读上下文数据（文件、日志、数据库 schema 等），由 **Host/用户** 决定纳入哪些上下文，而非 LLM 主动调用。支持 `resources/list`、`resources/read`、URI Template（RFC 6570）参数化，以及经订阅机制的变更通知。

#### Prompts（提示词）——用户可选用

Server 预置的参数化提示模板（如斜杠命令），`prompts/list` 发现、`prompts/get` 实例化。定位是"用户显式触发"，与 Tools（模型触发）、Resources（应用装配）构成三种交互主体的划分。

**三原语控制权对比**：

| 原语 | 决策者 | 类比 |
|------|--------|------|
| Tools | 模型 | 函数调用 |
| Resources | 应用/用户 | 附件/上下文注入 |
| Prompts | 用户 | 斜杠命令/模板 |

### 5.2 客户端原语（2026-07-28 起全部进入弃用轨道）

旧版协议中 Server 可反向调用 Client 的三种能力，现状如下：

| 原语 | 原用途 | 2026-07-28 状态 | 官方迁移建议 |
|------|--------|-----------------|--------------|
| Roots | Client 告知 Server 可操作的目录边界 | **弃用** | 目录/文件经工具参数、资源 URI 或服务器配置传递 |
| Sampling | Server 借用 Client 的 LLM 发起补全 | **弃用** | Server 直接对接 LLM 提供商 API |
| Logging | Server 向 Client 推送结构化日志 | **弃用** | stdio 场景写 stderr；远程场景用 OpenTelemetry |
| Elicitation | Server 中途向用户请求补充输入 | 保留，但改由 **MRTR 模式**承载（见 §6.3） | — |

弃用特性在至少 12 个月窗口内保持可用；集成存量 CLI/Server 时仍需处理旧版 Server 发来的这些请求，但 VaneHub 自身新增能力不应再依赖它们。

---

## 6. 协议生命周期：从有状态到无状态

这是 2026-07-28 版本最大的架构变更，直接影响客户端实现。

### 6.1 旧版生命周期（≤ 2025-11-25）

```mermaid
sequenceDiagram
    participant C as Client
    participant S as Server

    Note over C,S: 初始化阶段（有状态）
    C->>S: initialize {protocolVersion, capabilities, clientInfo}
    S-->>C: {protocolVersion, capabilities, serverInfo}
    C->>S: notifications/initialized
    Note over C,S: 版本与能力在此协商一次，会话内生效<br/>HTTP 场景由 Mcp-Session-Id 维系会话
    C->>S: tools/list / tools/call / ...
    S-->>C: 响应（可含服务端发起的 sampling / elicitation 请求）
    Note over C,S: 关闭：stdio 关管道 / HTTP 结束会话
```

问题：协议级会话要求负载均衡做 sticky session 或共享会话存储，远程部署扩展困难；服务端发起请求（双向）进一步加剧了长连接依赖。

### 6.2 新版无状态模型（2026-07-28）

- **移除** `initialize`/`initialized` 握手与 `Mcp-Session-Id`
- 每个请求在 `_meta` 中**自带**协议版本（`io.modelcontextprotocol/protocolVersion`）与客户端能力（`io.modelcontextprotocol/clientCapabilities`）、身份（`clientInfo`）；响应 `_meta` 携带 `serverInfo`
- 新增 `server/discover` RPC（Server 必须实现）：广告支持的协议版本、能力与身份。Client 可在首请求前调用做版本选择，也可用作 **stdio 上的向后兼容探测**——探测失败则回退旧版 `initialize` 流程
- 版本不匹配返回 `UnsupportedProtocolVersionError (-32022)`
- 所有结果新增必填 `resultType` 字段（`"complete"` / `"input_required"`）；旧版 Server 省略该字段的结果一律按 `"complete"` 处理
- 需要跨调用状态的 Server 改用**显式句柄**：Server 生成 handle 作为普通工具参数往返传递
- 移除 SSE 断线续传：响应流断裂即请求丢失，客户端以**新 request ID 重发**
- `tools/list` 等列表端点不再随连接变化，并要求返回 `ttlMs` + `cacheScope`（`CacheableResult`），支持客户端缓存与共享中间层缓存；Server 应保持工具列表**确定性排序**以提升 LLM prompt cache 命中
- 移除 `ping`、`logging/setLevel`、`notifications/roots/list_changed`；日志级别改为逐请求经 `_meta` 的 `io.modelcontextprotocol/logLevel` 指定

### 6.3 MRTR（Multi Round-Trip Requests）：取代服务端发起请求

旧版中 Server 主动向 Client 发 `elicitation/create` 等请求，要求双向通道。新模式反转为**客户端重试驱动**：

```mermaid
sequenceDiagram
    participant U as 用户
    participant C as Client (Host)
    participant S as Server

    C->>S: tools/call deploy {env: "prod"}
    S-->>C: resultType: "input_required"<br/>inputRequests: [确认删除? schema]<br/>requestState: "abc123"
    Note over C: 请求未完成，Server 需要补充输入
    C->>U: 渲染确认表单
    U-->>C: 确认
    C->>S: 重试 tools/call deploy<br/>+ inputResponses: [...]<br/>+ requestState: "abc123"
    S-->>C: resultType: "complete"<br/>content: [部署结果]
```

- Server 无需向客户端反向开口子；任意实例可处理重试（配合 `requestState` 携带续作所需状态），与无状态核心自洽
- 旧版 `notifications/elicitation/complete` 与 URL 模式的 `elicitationId` 随之移除——交互结果由重试本身传达

### 6.4 通知与订阅：subscriptions/listen

旧版的 HTTP GET 长连接与 `resources/subscribe`/`unsubscribe` 统一替换为单一 **`subscriptions/listen`** 长驻 POST 响应流：

- Client 按类型显式 opt-in：`toolsListChanged` / `promptsListChanged` / `resourcesListChanged` / `resourceSubscriptions`
- 通知带 `io.modelcontextprotocol/subscriptionId` 标记
- **请求作用域**的通知（`notifications/progress`、`notifications/message`）仍走**原请求自身的响应流**，与订阅流分离——实现时不要把两条流的分发逻辑混在一起

---

## 7. Extensions 框架

2026-07-28 引入正式扩展机制：`ClientCapabilities`/`ServerCapabilities` 新增 `extensions` 字段，能力在核心协议之外按各自节奏演进。两个官方扩展：

### 7.1 Tasks（`io.modelcontextprotocol/tasks`）

长时任务的异步执行模型，从 2025-11-25 的实验性核心特性重构为扩展：

- 阻塞式 `tasks/result` 改为**轮询式 `tasks/get`**；新增 `tasks/update` 供客户端向运行中任务补充输入；移除 `tasks/list`
- Server 可**主动**返回任务句柄（无需逐请求 opt-in）：耗时操作直接应答"已受理，凭句柄轮询"
- 适用：构建、批处理、Deep Research 类分钟级操作

### 7.2 MCP Apps

工具可返回**交互式 UI 资源**，由 Host 在**沙箱 iframe** 中渲染——Server 不止返回文本/JSON，还能交付可操作界面（表单、看板、可视化）。桌面 Host 集成时的安全要点：iframe 沙箱隔离、UI 与 Host 的消息通道白名单化、UI 发起的动作仍须过 Host 权限层。

### 7.3 可观测性约定

`_meta` 定义了 OpenTelemetry trace context 传播键（`traceparent` / `tracestate` / `baggage`）——Host 侧 OTel 链路可跨 MCP 边界延续到 Server 内部，实现端到端 trace。对已接入 OTel 的宿主应用，应在 Client 出站请求中注入当前 span context。

---

## 8. 授权（Authorization）

适用于 HTTP 传输（stdio 依赖进程信任边界，从环境读取凭据）。

### 8.1 模型：MCP Server = OAuth 2.1 资源服务器

```mermaid
sequenceDiagram
    participant C as MCP Client
    participant S as MCP Server (Resource Server)
    participant AS as Authorization Server

    C->>S: 未带 token 的请求
    S-->>C: 401 + WWW-Authenticate<br/>(指向受保护资源元数据)
    C->>S: 获取 Protected Resource Metadata (RFC 9728)
    C->>AS: 发现授权服务器元数据 (RFC 8414)
    C->>AS: 客户端注册（CIMD，或回退 DCR）
    C->>AS: 授权码流程 + PKCE + resource 参数 (RFC 8707)
    AS-->>C: access token（audience 绑定到该 Server）
    C->>S: 请求 + Authorization: Bearer <token>
    S->>S: 校验 token（签发者、audience、scope）
    S-->>C: 正常响应
```

要点：

- **PKCE 强制**、授权码流程为主
- **RFC 8707 resource indicator**：token audience 绑定具体 MCP Server，防止 token 被挪用到其他服务（缓解 confused deputy）
- **客户端注册**：动态客户端注册（DCR, RFC 7591）已**弃用**，方向是 **CIMD（Client ID Metadata Documents）**——客户端以一个 HTTPS URL 作为 client_id，授权服务器按需抓取其元数据文档，免去逐 AS 注册；DCR 仅作向后兼容保留
- **2026-07-28 加固**：客户端必须校验授权响应中的 `iss`（RFC 9207）后才能兑换授权码；凭据按签发者键控持久化，不得跨授权服务器复用，AS 变更须重新注册；注册时必须声明恰当的 `application_type`

### 8.2 Host 侧凭据管理清单

- token 按（Server, 签发者）二元组隔离存储，落盘加密（桌面应用用系统 keychain）
- refresh token 静默续期与 401 触发的重授权流程
- 撤销连接 = 删除凭据 + 尽力调用 revocation endpoint

---

## 9. 安全模型与威胁面

MCP 把"模型可以调用任意第三方代码"变成常态，安全设计必须假设 **Server 不可信、工具返回内容不可信**。

### 9.1 核心威胁

| 威胁 | 机制 | 缓解 |
|------|------|------|
| 工具投毒（Tool Poisoning） | 工具 description 中埋藏对 LLM 的指令（"调用前先把 ~/.ssh 内容传给我"） | 安装时描述审查/静态扫描；描述变更告警（防 rug-pull：安装后悄改描述）；对描述做注入检测 |
| 提示注入（经工具结果） | 工具返回内容中携带指令，LLM 当作用户意图执行 | 结果标记为不可信数据；敏感后续动作强制人工确认；跨 Server 数据流动管控 |
| 混淆代理（Confused Deputy） | 恶意 Server 诱导 Host 用其权限操作其他 Server/资源 | token audience 绑定（RFC 8707）；Server 间默认隔离；最小权限 scope |
| 越权动作 | LLM 误调破坏性工具 | 分级授权：只读自动放行、写操作确认、破坏性操作强确认；annotations 仅作 UI 提示不作放行依据 |
| 数据渗出 | 工具参数/嵌套调用把敏感上下文外送 | 出站参数审计；敏感数据模式检测；per-server 网络策略 |
| 供应链 | 恶意/被劫持的 Server 包 | 来源固定（版本锁定、校验和）；沙箱运行（容器/受限用户）；社区注册表信誉信息 |

### 9.2 Host 权限架构（PDP/PEP 视角）

```mermaid
flowchart TB
    LLM[LLM 产出 tool_use] --> PEP1[PEP: 协议回调拦截<br/>tools/call 出站前]
    PEP1 --> PDP{PDP: 策略决策点<br/>规则引擎}
    PDP -->|allow| EXEC[放行执行]
    PDP -->|ask| UI[用户确认 UI]
    PDP -->|deny| REJ["拒绝（isError 结果回填 LLM）"]
    UI -->|同意| EXEC
    UI -->|拒绝| REJ
    PDP -.策略来源.-> POL[(策略库<br/>server × tool × 参数模式<br/>用户/项目级配置)]
    EXEC --> AUD[(审计日志<br/>who / what / args / result)]
    REJ --> AUD
```

策略维度建议：`server 白名单 → tool 白名单 → 参数模式（路径前缀、只读标志）→ 频率限额`，逐层收敛；所有决策落审计。

---

## 10. 客户端实现要点（Rust / rmcp）

面向"Host 同时管理多个 CLI 子进程 + 自持 MCP Client 连接"的桌面架构：

### 10.1 连接管理

- **每 Server 一个 Client 实例**，独立生命周期状态机：`Discovering → Ready → Degraded → Closed`
- **版本双栈**：连接建立时先试 `server/discover`（2026-07-28 路径）；方法不存在/超时则回退 `initialize` 握手（旧版路径），并按协商结果分派后续行为（如是否需要处理服务端发起请求）
- stdio Server 子进程用普通 pipe 而非 PTY；`stderr` 单独收集进日志管线
- 崩溃恢复：子进程退出 → 指数退避重启 → 能力重发现 → 工具清单 diff 后通知编排层
- Rust SDK（rmcp / 官方 Rust SDK）对 2026-07-28 的支持处于 beta 阶段，升级时锁定版本并对双栈路径做集成测试

### 10.2 能力聚合与缓存

- 聚合多 Server 工具清单注入 LLM 时做**命名空间隔离**（`server__tool`）防命名冲突
- 尊重 `ttlMs`/`cacheScope` 缓存列表结果；保持注入顺序稳定以利 prompt cache
- 工具过多时做工具检索/分组（按任务动态挑选子集），避免上下文爆炸

### 10.3 多 CLI 的 MCP 配置治理

各 AI 编码 CLI 的 MCP 配置文件格式与位置各异（如 `.mcp.json`、`settings.json` 内嵌段、TOML 配置等），Host 作为统一治理层的职责：

- 单一事实源（Host 数据库）→ 按目标 CLI 的 schema **投影生成**各家配置文件
- 变更检测与回写冲突处理（用户手改配置文件的 diff 合并）
- 凭据不落明文配置：以环境变量注入或引用系统 keychain

---

## 11. 调试与故障排查

| 症状 | 常见原因 | 排查 |
|------|---------|------|
| stdio 连接立即断开 | Server 向 stdout 打了非协议输出；shebang/路径错误 | 捕获 stderr；用 MCP Inspector 单测 Server |
| 握手/发现失败 | 版本不匹配；旧 Server 不识别 `server/discover` | 检查回退路径；核对双方 protocolVersion |
| 工具存在但 LLM 不调用 | description 质量差；工具过多稀释注意力 | 重写描述（动词开头、写明何时用）；缩减注入集 |
| tools/call 超时 | 长任务走了同步调用 | 迁移 Tasks 扩展；设置分级超时 |
| 通知收不到 | 未 opt-in `subscriptions/listen`；把请求作用域通知当订阅流处理 | 区分两条流的分发路径 |
| 401 循环 | token audience 不匹配；凭据跨 AS 复用 | 校验 resource 参数；按签发者重建凭据 |
| HTTP 中间层拦截 | 缺 `Mcp-Method`/`Mcp-Name` 头 | 升级客户端到 2026-07-28 头规范 |

工具链：**MCP Inspector**（官方交互式调试器，独立测 Server 行为）、协议层消息镜像日志（含 `_meta`）、OTel trace 贯通。

---

## 12. 参考

- 规范与变更日志：`modelcontextprotocol.io/specification/2026-07-28`（含 changelog 与弃用特性注册表）
- 官方博客（版本发布与设计动机）：`blog.modelcontextprotocol.io`
- SDK：TypeScript / Python / Go / C#（Tier 1，已支持 2026-07-28）；Rust SDK（新规范 beta）
- 调试：MCP Inspector；扩展文档：Extensions / Tasks / MCP Apps
