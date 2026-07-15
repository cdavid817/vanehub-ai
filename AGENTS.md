# AGENTS.md

> 本文件是 VaneHub AI 项目所有 AI 编程助手(Claude Code / Gemini CLI / OpenCode / Codex 等)的统一入口。
> CLAUDE.md、GEMINI.md 均指向本文件,请勿分别维护三份不同内容。
> 详细技术选型说明与完整代码规范见 `openspec/project.md`;具体场景的实现示例见 `.claude/skills/`。

## 项目概览

VaneHub AI 是一个桌面端多 AI 编程助手管理终端,用于统一管理和切换 Claude Code、OpenCode、Codex CLI、Gemini CLI 等多个 AI 编程代理。同一套 React UI 既可运行在 Tauri 桌面客户端内,也可通过 Web/mock adapter 以浏览器页面形式运行。

## 技术栈(严格约束,不允许引入替代方案)

- 前端:React 18 + TypeScript(strict mode)+ Vite
- 桌面运行时:Tauri 2.x(Rust)
- 状态管理:仅用 React 内置 state/context,不引入 Redux/Zustand/MobX
- 样式:Tailwind CSS,不写内联 style,不引入 styled-components/CSS Modules/其他 UI 组件库
- 数据库:SQLite,通过 Rust 侧访问,前端不直接连库
- 测试:Playwright(E2E)
- 包管理:npm(项目已有 package-lock.json,不要切到 pnpm/yarn)

## 架构核心约束

- React 组件必须依赖 `src/services/agent-service.ts` 定义的服务接口,**禁止**组件内直接调用 Tauri `invoke()`
- `src/services/tauri-agent-client.ts`(桌面实现)与 `src/services/web-agent-client.ts`(Web/mock 实现)必须保持接口一致,新增能力要同时改两处
- `src-tauri/` 负责 Rust 侧的 CLI 检测、启动路由、SQLite 注册表与会话状态,不要把这类逻辑下沉到前端

## 代码规范(可执行规则,详细版见 openspec/project.md)

- 提交前必须通过:`npm run lint`、`npm run test`、`cargo clippy --manifest-path src-tauri/Cargo.toml`
- TypeScript:禁止 `any`,禁止 `// @ts-ignore`(需要绕过时用 `// @ts-expect-error` 并写明原因)
- React:函数组件 + Hooks,禁止 class component;单文件不超过 300 行
- Rust:跨 Tauri command 边界的错误必须转换为 `Result<T, String>` 或自定义 error enum,`unwrap()`/`expect()` 仅限测试代码
- 注释只写"为什么这样做",不写代码翻译式注释

## 项目文件规范(概要,完整版见 openspec/project.md)

```
src/
├─ components/       # 纯展示型 React 组件,不直接依赖 Tauri API
├─ services/         # 前端服务边界层(唯一允许被组件依赖的一层)
├─ hooks/            # 自定义 hook
src-tauri/
├─ src/commands/     # 每个 Tauri command 一个文件,按功能域分组
├─ src/db/           # SQLite schema 与迁移脚本
openspec/
├─ changes/          # 未归档的变更提案
├─ specs/            # 已确认规范(唯一真源)
└─ archive/          # 已完成变更的历史记录
```

## 变更流程

任何新功能或架构调整,必须先在 `openspec/changes/` 下起一个 proposal,通过 `openspec validate --specs --strict` 校验后再动代码。不要跳过 spec 直接改代码。

## 校验命令(改完必须全部跑通)

```bash
npm run test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
openspec validate --specs --strict
```
