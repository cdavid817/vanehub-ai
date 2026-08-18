# 运行时与服务边界

React 组件依赖带类型的 frontend service。它们禁止直接导入 Tauri `invoke()`、打开 SQLite、启动 CLI 或直接访问本地文件系统。

## 桌面路径

1. 组件调用一个 service 接口。
2. Tauri frontend adapter 将请求映射到一个已声明的 command。
3. 薄薄的 Rust command 校验并映射 transport DTO。
4. 拥有该能力的 native application service 通过注入的 port 执行用例。
5. Infrastructure adapter 执行 SQLite、进程、文件系统、网络或 OS 相关工作。

可能较慢的工作会在完成前返回一个操作标识，并通过 operations 边界暴露进度。

## Web/mock 路径

Web adapter 以确定性的内存态实现同一套 frontend 契约。它可以为 UI 开发模拟执行与时序，但禁止声称本地进程已运行、SQLite 已变更或某个操作系统动作已发生。

## 新增能力

- 先扩展与运行时无关的 service 接口。
- 当 UI 消费该能力时，同时实现 Tauri adapter 和 Web/mock adapter。
- 将 provider 特定的启动行为保留在 Agent Runtime infrastructure 之后。
- 对用户可见的错误保持本地化，native 诊断写入统一的脱敏日志管道。

TypeScript 模型契约生成的决策（`ts-rs`）记录为 `src-tauri/ARCHITECTURE.md` 中的 ADR-005。早期的单 CLI chat 运行时叙事已被多 Agent group chat 运行时（`openspec/specs/multi-agent-group-chat/`）取代。

## 运行时选择与适配器

前端不直接嗅探宿主环境。每个 service 都通过 `createRuntimeAdapter` 在启动期一次性选定一个实现，之后整条调用链都走这个被选中的 adapter。`detectRuntimeKind` 是这条分派的唯一决策点，顺序如下：先看 `__VANEHUB_RUNTIME__` 显式声明（测试与调试覆盖用），其次看 `__TAURI_INTERNALS__` 是否存在以判定桌面 Tauri 运行时，再看 `__VANEHUB_HTTP_BASE_URL__` 是否存在以判定 web-http 部署，最后回退到默认的 web-mock。web-http 分支要求调用方为该 service 注册一个 `webHttp` adapter；缺失时 `createRuntimeAdapter` 直接抛错，而不是静默落到 web-mock ——在已声明是真实部署的宿主上用假数据继续运行，会把静默的错误伪装成业务成功。

```mermaid
flowchart TD
    Start([应用启动 / service 取值]) --> Detect[detectRuntimeKind host=window]
    Detect --> Q1{__VANEHUB_RUNTIME__<br/>显式声明?}
    Q1 -- 是 --> UseExplicit[使用显式 RuntimeKind]
    Q1 -- 否 --> Q2{__TAURI_INTERNALS__<br/>存在?}
    Q2 -- 是 --> KindTauri[RuntimeKind = tauri]
    Q2 -- 否 --> Q3{__VANEHUB_HTTP_BASE_URL__<br/>存在?}
    Q3 -- 是 --> KindWebHttp[RuntimeKind = web-http]
    Q3 -- 否 --> KindWebMock[RuntimeKind = web-mock<br/>默认]
    UseExplicit --> Resolve
    KindTauri --> Resolve[createRuntimeAdapter<br/>按 RuntimeKind 选 adapter]
    KindWebHttp --> Resolve
    KindWebMock --> Resolve
    Resolve --> Q4{RuntimeKind = web-http<br/>且未提供 webHttp adapter?}
    Q4 -- 是 --> Throw[抛错:<br/>禁止静默用假数据]
    Q4 -- 否 --> Bind[返回单一 service 实现]
    Bind --> TauriImpl[tauri-agent-client<br/>每方法映射 snake_case command]
    Bind --> WebMockImpl[web-agent-client<br/>确定性内存态]
    Bind --> WebHttpImpl[webHttp adapter<br/>外部 HTTP 部署]
```

`AgentService` 是一个超大的聚合接口，覆盖 Agent 生命周期、会话、MCP、工具、IM、扩展、权限、工作板、SDK、SSH 连接等子域。每个子域都有一个对应的 `runtime-*-client.ts` 文件，内部各自调用 `createRuntimeAdapter`，传入成对的 Tauri 实现与 Web/mock 实现。两条实现必须保持接口一致——新增能力要同时改 `tauri-agent-client.ts`（每个方法映射到一个 snake_case 的 Tauri command）与 `web-agent-client.ts`（用确定性内存态模拟同一语义）。

约束要点：

- **单例 `agentService`** 在前端模块图解析期构造一次，组件不自行 `new`。
- **组件经 hooks 依赖**：React 组件只依赖 `src/hooks/` 暴露的 hook 与 `src/services/agent-service.ts` 的接口，不直接 import `tauri-agent-client` 或 `web-agent-client`，也不直接调用 `invoke()`。
- **desktop 就绪埋点无条件标记**:`main.tsx` 在所有运行时(Tauri、web-mock、web-http)渲染完成后都会无条件设置 `root.dataset.vanehubBootstrap="ready"`;唯一桌面相关的条件分支是 `if(import.meta.env.VITE_DESKTOP_E2E==="1")`,在该分支下才加载 `@wdio/tauri-plugin` 并注册 `vanehubFatalError` 监听。没有 `desktop_ready`/`report_desktop` 之类的 native 命令——"就绪"只是一个 dataset 标记,不向 native 上报。
- **web-http 无 adapter 必抛错**：这是防止生产部署静默退化到假数据的关键闸门。新增 service 在 web-http 部署下没有真实后端时，应让该 service 在 web-http 分支抛错，而不是回退到 web-mock。

## 关键文件与契约

### 运行时探测

`detectRuntimeKind()` 是运行时分派的唯一决策点，按固定顺序判定：先看 `__VANEHUB_RUNTIME__` 显式声明(测试与调试覆盖用)→ 看 `__TAURI_INTERNALS__` 是否存在以判定桌面 Tauri 运行时 → 看 `__VANEHUB_HTTP_BASE_URL__` 是否存在以判定 web-http 部署 → 以上都不命中则回退到默认的 web-mock。

### 适配器

`createRuntimeAdapter()` 按选定的 `RuntimeKind` 绑定三套适配器之一:`tauri`/`webHttp`/`webMock`。`web-http` 分支要求调用方为该 service 注册 `webHttp` adapter,缺失时直接抛错,而不是静默落到 `web-mock`。

### AgentService 聚合接口

`AgentService`(`src/services/agent-service.ts`,第 214 行起)是一个超大的聚合接口,覆盖 Agent 生命周期、会话、MCP、工具、IM、扩展、权限、工作板、SDK、SSH 连接等子域。每个子域对应一个 `runtime-*-client.ts` 文件,内部各自调用 `createRuntimeAdapter`,传入成对的实现。

### 成对实现

- `tauri-agent-client.ts` —— 每个方法映射到一个 snake_case 的 Tauri command;
- `web-agent-client.ts` —— 用确定性内存态模拟同一语义。

两份实现必须保持接口一致,新增能力要同时改这两处。

### 单例与依赖路径

单例 `agentService` 在 `runtime-agent-client.ts` 中于模块图解析期构造一次,组件不自行 `new`。React 组件只依赖 `src/hooks/` 暴露的 hook 与 `src/services/agent-service.ts` 的接口,不直接 import `tauri-agent-client` 或 `web-agent-client`,也不直接调用 `invoke()`。

### desktop 就绪埋点

desktop 就绪埋点在 `main.tsx` 中处理:渲染完成后无条件设置 `root.dataset.vanehubBootstrap="ready"`(Tauri、web-mock、web-http 三种运行时都会设置),没有向 native 上报 "desktop 就绪" 的命令。唯一桌面相关的条件分支是 `if(import.meta.env.VITE_DESKTOP_E2E==="1")`,在该分支下加载 `@wdio/tauri-plugin` 并注册 `vanehubFatalError` 监听;这条约束由 `desktop-instrumentation-boundary.test.ts` 验证。

服务边界与运行时选择的权威定义见 `openspec/specs/frontend-runtime-architecture/spec.md` 与 `src-tauri/ARCHITECTURE.md`。
