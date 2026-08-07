# 技术栈与选型理由

> **所有版本号取自 `package.json` 与 `src-tauri/Cargo.toml`**，不做推测。`AGENTS.md` 将其中多数列为"严格约束，不允许引入替代方案"。

## 硬性约束

**以下选型在 `AGENTS.md` 中被明确禁止替换**：

| 领域 | 选定方案 | 明确排除 |
|---|---|---|
| 状态管理 | React 内置 state / context | Redux、Zustand、MobX |
| 样式 | Tailwind CSS | 内联 style、styled-components、CSS Modules、其他 UI 组件库 |
| 包管理 | npm | pnpm、yarn |
| 数据库访问 | Rust 侧访问 SQLite | 前端直连数据库 |
| 桌面运行时 | Tauri 2.x | Electron 等 |

**约束的意义在于减少决策面**：一个桌面应用同时跑在三种运行时里，多一套状态方案就多一处两边行为不一致的可能。

## 版本固定策略

**本仓库对依赖版本有三档处理**：

| 档位 | 写法 | 适用 |
|---|---|---|
| 精确固定 | `=0.32.0` | 生态耦合紧、行为敏感的依赖 |
| 兼容范围 | `^1.2.3` / `"2"` | 多数依赖 |
| 主版本 | `"1"` | 稳定的基础库 |

**用 `=` 精确固定的有**：全部 OpenTelemetry / tracing 系（7 个）、`russh`、`webview2-com`。这三类的共同点是**次版本升级也可能改变可观察行为**。

## 前端

| 依赖 | 版本 | 选型理由 |
|---|---|---|
| `react` / `react-dom` | `19.2.8` | 项目基线；配合 Tauri 的 webview 渲染 |
| `typescript` | `6.x`（别名 `npm:@typescript/typescript6`） | strict 模式；禁止 `any` 与 `@ts-ignore` |
| `@typescript/native` | `npm:typescript@^7.0.2` | 原生 TS 工具链 |
| `vite` | `8.2.0` | 开发期冷启动快，产物按需分包 |
| `@vitejs/plugin-react` | `6.0.5` | React 支持 |
| `tailwindcss` | `4.3.3`（`@tailwindcss/vite`） | 工具类优先，避免样式文件与组件分离 |
| `@tanstack/react-query` | `5.101.4` | 服务端状态的缓存与失效；**管的是异步数据而非应用状态**，不违反约束 |
| `@tanstack/react-virtual` | `3.14.9` | 长列表虚拟化（会话列表、日志、终端输出） |
| `react-router` | `8.3.0` | 路由 |
| `react-hook-form` | `7.84.0` | 表单状态与校验 |
| `zod` | `4.4.3` | 运行时 schema 校验，与 TS 类型对齐 |
| `@xterm/xterm` | `6.0.0` | 终端渲染；配 `@xterm/addon-fit` `0.11.0` |
| `react-markdown` | `10.1.0` | 对话内容渲染 |
| `remark-gfm` / `remark-math` | `4.0.1` / `6.0.0` | GFM 表格与数学语法 |
| `rehype-highlight` / `rehype-katex` | `7.0.2` / `7.0.1` | 代码高亮与公式渲染 |
| `katex` | `0.18.1` | 数学公式排版 |
| `mermaid` | `11.16.1` | 对话中的图表渲染 |
| `i18next` / `react-i18next` | `26.3.6` / `17.0.11` | 界面多语言（5 种） |
| `lucide-react` | `1.28.0` | 图标 |
| `@radix-ui/react-slot` | `1.3.3` | 组合式组件；**只用 Slot，不引整套 UI 库** |
| `class-variance-authority` / `clsx` / `tailwind-merge` | `0.7.1` / `2.1.1` / `3.6.0` | 变体与类名合并 |
| `tailwindcss-animate` | `1.0.7` | 动画工具类 |
| `react-error-boundary` | `6.1.2` | 错误边界，配合前端日志上报 |

**注意 TypeScript 用的是别名包**：`typescript` 指向 `npm:@typescript/typescript6`，`@typescript/native` 指向 `typescript@^7.0.2`。排查依赖问题时不能按标准包名去找。

## 桌面运行时

| 依赖 | 版本 | 选型理由 |
|---|---|---|
| `tauri` | `2`（feature `tray-icon`） | 相比 Electron 体积小、内存占用低，原生侧用 Rust |
| `tauri-build` | `2` | 构建期支持 |
| `tauri-plugin-autostart` | `2.5.1` | 开机自启 |
| `tauri-plugin-dialog` | `2` | 原生文件/目录选择 |
| `tauri-plugin-opener` | `2` | 外部程序打开 |
| `@tauri-apps/api` | `2.0.0` | 前端侧 API |

### Windows 专属

| 依赖 | 版本 | 用途 |
|---|---|---|
| `webview2-com` | `=0.38.2` | WebView2 交互，**精确固定** |
| `windows` | `0.62.2` | Job Object、ToolHelp、安全授权等 |
| `windows-sys` | `0.61` | `Win32_Storage_FileSystem` |

**`windows` crate 启用的 feature 透露了实现细节**：`Win32_System_JobObjects`（进程树终止）、`Win32_System_Diagnostics_ToolHelp`（线程快照）、`Win32_Security_Authorization`（私有中继目录 ACL）。

### Unix 专属

`libc` `0.2`。

## 数据与持久化

| 依赖 | 版本 | 选型理由 |
|---|---|---|
| `rusqlite` | `0.40`（`bundled`、`trace`） | `bundled` 免除用户安装 SQLite；`trace` 支持语句追踪 |
| `r2d2` / `r2d2_sqlite` | `0.8` / `0.35` | 连接池，支持并发读写 |
| `keyring` | `4.1.5` | 凭据交给操作系统密钥链，不自己存密码 |
| `zeroize` | `1` | 敏感数据用后清零 |
| `dirs` | `6` | 跨平台标准目录 |

详见 [数据层](data-layer.md)。

## 进程与终端

| 依赖 | 版本 | 选型理由 |
|---|---|---|
| `portable-pty` | `0.9.0` | 跨平台 PTY；CLI Agent 需要真实终端语义（颜色、光标、交互提示、TUI 框线） |
| `process-wrap` | `9.1.0`（`std`、`tokio1`） | 进程组管理，确保子进程树可被整体终止 |
| `tokio` | `1`（`full`） | 异步运行时 |

详见 [进程与 PTY](process-and-pty.md)。

## 网络与协议

| 依赖 | 版本 | 选型理由 |
|---|---|---|
| `reqwest` | `0.13`（`blocking`、`json`、`socks`） | HTTP 客户端；`socks` 支持代理环境 |
| `axum` | `0.8` | 本地 HTTP 服务（权限钩子桥接、MCP 中继） |
| `rmcp` | `3.0.1` | MCP 官方 Rust 实现；启用 client、子进程传输、SSE、Streamable HTTP |
| `russh` | `=0.62.5`（`ring`） | 纯 Rust SSH；**精确固定**，SSH 行为变化风险高于一般依赖 |
| `tokio-tungstenite` | `0.30`（`rustls-tls-webpki-roots`） | WebSocket |
| `tokio-rustls` / `webpki-roots` | `0.26` / `1` | TLS |
| `prost` | `0.14` | Protobuf（IM 连接器协议） |
| `qrcode` | `0.14.1`（`svg`） | 微信授权二维码 |
| `http` / `url` | `1` / `2` | 基础类型 |
| `base64` / `sha2` / `rand` | `0.23` / `0.11` / `0.10` | 编码与摘要 |

## 可观测性

| 依赖 | 版本 |
|---|---|
| `opentelemetry` | `=0.32.0`（`trace`、`metrics`、`logs`） |
| `opentelemetry-otlp` | `=0.32.0`（`http-proto`、`reqwest-blocking-client`） |
| `opentelemetry_sdk` | `=0.32.1` |
| `opentelemetry-semantic-conventions` | `=0.32.1`（`semconv_experimental`） |
| `opentelemetry-appender-tracing` | `=0.32.0` |
| `tracing` | `=0.1.44`（`attributes`、`std`） |
| `tracing-opentelemetry` | `=0.33.0` |
| `tracing-subscriber` | `=0.3.23`（`registry`、`std`） |

**这一组全部用 `=` 精确固定，是本仓库固定版本最集中的地方**——OpenTelemetry Rust 生态各 crate 之间版本耦合紧密，浮动版本极易导致编译期类型不匹配。**升级必须整组同步。**

**注意各 crate 都关闭了 `default-features`**，只按需启用——这套生态的默认特性会拉进不需要的依赖。

详见 [可观测性架构](observability-architecture.md)。

## 序列化与配置格式

| 依赖 | 版本 | 用途 |
|---|---|---|
| `serde` / `serde_json` | `1` / `1` | 序列化基础 |
| `toml` / `toml_edit` | `1.1` / `0.25.12` | TOML 读写（`toml_edit` 保留格式） |
| `json5` | `1.3.1` | JSON5 解析 |

**同时支持 TOML、JSON、JSON5 三种格式**，因为各家 CLI 的配置文件格式不同——Codex 用 TOML，Claude Code 用 JSON，某些工具用 JSON5。

**`toml_edit` 与 `toml` 并存**：前者能在修改配置时保留注释与格式，改用户的配置文件时不会把他们的注释洗掉。

## 文件与文本

| 依赖 | 版本 | 用途 |
|---|---|---|
| `globset` | `0.4` | glob 匹配 |
| `ignore` | `0.4` | 遵守 `.gitignore` 的文件遍历 |
| `regex` | `1` | 正则 |
| `chrono` | `0.4`（`serde`） | 时间 |
| `uuid` | `1`（`v4`） | id 生成 |
| `thiserror` | `2` | 错误类型派生 |
| `async-trait` | `0.1` | 异步 trait |
| `futures-util` | `0.3` | 异步组合子 |

## 测试与质量

| 依赖 | 版本 | 用途 |
|---|---|---|
| `vitest` | `4.1.10` | 单元与组件测试，CI 强制覆盖率门槛 |
| `@vitest/coverage-v8` | `4.1.10` | 覆盖率 |
| `@playwright/test` | `1.62.1` | E2E 与文档截图 |
| `eslint` | `10.8.0` | 含 `max-lines` 300 行硬规则 |
| `@eslint/js` | `10.0.1` | 基础规则 |
| `typescript-eslint` | `8.65.0` | TS 规则 |
| `eslint-plugin-react-hooks` | `7.1.1` | Hooks 规则 |
| `lint-staged` | `17.3.0` | 暂存区校验 |
| `syn` | `3`（`full`、`visit`） | **Rust 侧架构测试：解析自身源码做静态断言** |
| `proc-macro2` | `1`（`span-locations`） | 配合 `syn` 定位到具体行 |
| `tempfile` | `3` | 测试临时目录 |

**`syn` 出现在 `dev-dependencies` 是本项目一个特色**——用它在测试中解析自身源码，对架构约束做编译期之外的静态检查，例如"每个 Tauri command 恰好注册一次"。`span-locations` feature 让断言能指出具体位置。详见 [架构总览](README.md#架构约束的机器强制)。

## 构建产物

**release profile 做了体积与性能取舍**（`src-tauri/Cargo.toml`）：

| 配置 | 值 | 效果 |
|---|---|---|
| `opt-level` | `3` | 最大优化 |
| `lto` | `"thin"` | 跨 crate 优化，比 `fat` 编译快 |
| `codegen-units` | `1` | 更好的优化，牺牲编译并行度 |
| `strip` | `"debuginfo"` | 去调试符号减小体积 |

**crate 类型是三合一**（`[lib]`）：`staticlib`、`cdylib`、`rlib`，因为 Tauri 的构建流程需要多种链接形态。

**有两个 binary target**，因此必须声明 `default-run = "vanehub-ai"`（`Cargo.toml:7-10`）——否则 Tauri 的 `tauri dev` / `tauri build`（内部调用不带 `--bin` 的 `cargo run`）会直接失败。第二个 binary 是权限钩子，见 [权限架构](permissions-architecture.md#claude-code-钩子桥接)。

**前端产物有分包检查**：`npm run build` 在 `tsc && vite build` 之后执行 `node scripts/check-frontend-chunks.mjs`，对分包结果做断言。实测 16 个懒加载 chunk，主静态闭包 108.2 KiB gzip。

## 已知取舍

- **OTel 全固定版本升级成本高** —— 升一个就要一起升，且要验证类型兼容。
- **`bundled` SQLite 增大二进制** —— 换来零安装依赖。
- **`codegen-units = 1`** —— release 编译明显变慢。
- **TypeScript 用别名包** —— 工具链版本不是标准 npm 名称，排查依赖问题时需注意。
- **`react-query` 与"不引入状态管理库"的边界靠约定维持** —— 没有机器强制。
- **三种配置格式解析器并存** —— 是被各 CLI 的现状逼出来的，不是主动设计。
- **`syn` 版本 3 用于测试** —— 它随 Rust 语法演进，升级 Rust 时可能需要同步升级。

## 相关文档

- [架构总览](README.md)
- [数据层](data-layer.md)
- [进程与 PTY](process-and-pty.md)
- [前端架构](frontend.md)
- [可观测性架构](observability-architecture.md)
