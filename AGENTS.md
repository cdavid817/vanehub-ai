# AGENTS.md

> 本文件是 VaneHub AI 项目所有 AI 编程助手(Claude Code / Gemini CLI / Antigravity CLI / OpenCode / Codex 等)的统一入口。
> CLAUDE.md、GEMINI.md 均指向本文件,请勿分别维护三份不同内容。
> 详细技术选型说明与完整代码规范见 `openspec/project.md`;具体场景的实现示例见 `.claude/skills/`。

## 项目概览

VaneHub AI 是一个桌面端多 AI 编程助手管理终端,用于统一管理和切换 Claude Code、OpenCode、Codex CLI、Gemini CLI、Antigravity CLI 等多个 AI 编程代理。同一套 React UI 既可运行在 Tauri 桌面客户端内,也可通过 Web/mock adapter 以浏览器页面形式运行。

## 技术栈(严格约束,不允许引入替代方案)

- 前端:React 19 + TypeScript(strict mode)+ Vite
- 桌面运行时:Tauri 2.x(Rust)
- 状态管理:仅用 React 内置 state/context,不引入 Redux/Zustand/MobX
- 样式:Tailwind CSS,不写内联 style,不引入 styled-components/CSS Modules/其他 UI 组件库
- 数据库:SQLite,通过 Rust 侧访问,前端不直接连库
- 测试:Vitest(单元/组件测试,CI 强制覆盖率门槛)+ Playwright(E2E)
- 包管理:npm(项目已有 package-lock.json,不要切到 pnpm/yarn)

## 架构核心约束

- React 组件必须依赖 `src/services/agent-service.ts` 定义的服务接口,**禁止**组件内直接调用 Tauri `invoke()`
- `src/services/tauri-agent-client.ts`(桌面实现)与 `src/services/web-agent-client.ts`(Web/mock 实现)必须保持接口一致,新增能力要同时改两处
- `src-tauri/` 负责 Rust 侧的 CLI 检测、启动路由、SQLite 注册表与会话状态,不要把这类逻辑下沉到前端

## 日志规范

- 新增诊断日志、操作日志、前端持久化日志事件时,必须遵循 `openspec/specs/unified-log-management/spec.md`
- Rust/native 侧日志必须通过统一日志服务写入,禁止新增 feature-local 日志文件或绕过脱敏直接落盘
- SDK/CLI/任务类操作输出必须保留页面内展示能力,同时写入统一日志目录
- React 组件不得直接写本地日志文件;需要持久化的前端错误必须通过 service boundary 上报到 native logging service
- 日志必须支持 `error`、`warn`、`info`、`debug` 级别语义,敏感信息必须在落盘前脱敏

## 代码规范(可执行规则,详细版见 openspec/project.md)

- 提交前必须跑通本文件末尾「校验命令」一节的**全部**命令。不要在这里另维护一份子集——两份清单迟早漂移,而漂移的那一半正是 CI 会拦下来的东西
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
├─ src/platform/database/  # SQLite 访问层(schema 与迁移)
openspec/
├─ changes/          # 未归档的变更提案
│  └─ archive/       # 已完成变更的历史记录
├─ specs/            # 已确认规范(唯一真源)
└─ project.md         # 项目上下文和详细规范
```

## 变更流程

任何新功能或架构调整,必须先在 `openspec/changes/` 下起一个 proposal,通过 `openspec validate --specs --strict` 校验后再动代码。不要跳过 spec 直接改代码。

## OpenSpec 归档治理

- 已完成变更的唯一在线归档位置是 `openspec/changes/archive/YYYY-MM-DD-<change-name>/`;完整 Markdown 工件必须保留在 Git 中,不可用 zip/tar 替代。
- 归档前必须完成 tasks,执行 `openspec validate <change-name> --strict`,并在涉及代码时记录实现验证结果。正常流程禁止使用 `--no-validate`;仅无主规范影响的变更可使用 `--skip-specs`。
- 使用 `openspec archive <change-name>` 后,必须执行 `powershell -ExecutionPolicy Bypass -File scripts/Update-OpenSpecArchiveIndex.ps1`,并将主 specs、归档目录和索引一起提交。
- 查询归档时优先读取 `openspec/changes/archive/archive-index.json`,按 `changeName` 或 `capabilities` 过滤;仅在定位到具体变更后才读取其 Markdown 工件。
- 每 6 个月审查一次在线归档。迁往冷归档前必须验证目标 Git 仓库、不可变分支或 tag,在 `openspec/archive-cold-migrations.md` 记录可验证引用后,才能移除在线副本。

## 提交与 PR 语言规范

- 创建 PR 时,标题与描述一律使用英文。
- 提交信息(commit message)同样一律使用英文。

## 机器强制层(hooks 与提交拦截)

本仓库的规范不只写在文档里,以下机制会在你操作时自动执行。收到拦截反馈时,正确做法永远是修代码,而不是想办法关掉闸门。

- **编辑即校验**:`.claude/settings.json` 注册了 PostToolUse hook(`scripts/hooks/post-edit-quality.mjs`):每次编辑/写入 `.ts`/`.tsx` 后自动运行 `eslint --fix` 并把剩余错误回报给你;编辑 `.rs` 后自动运行 `rustfmt`,格式化失败通常意味着你写出了语法错误。
- **提交即拦截**:`git commit` 触发 husky——lint-staged 对暂存的 TS/JS 跑 `eslint --fix`、对 `.rs` 跑 `rustfmt`;commitlint 要求提交信息符合 Conventional Commits,允许的 type:build/chore/ci/deps/docs/feat/fix/perf/refactor/revert/style/test。
- **300 行是 ESLint 硬规则**:`max-lines`(按物理行计)对全部 ts/tsx 生产代码生效;测试文件豁免;存量超限文件在 `eslint.config.js` 中列有技术债豁免清单——禁止向清单新增文件,新代码一律 ≤300 行。
- **中文文档的加粗写法**:`npm run docs:check` 会拒绝 `**结论。**下一句` 这种写法——句末标点留在 `**` 之内时,按 CommonMark 的 flanking 规则它不构成合法的闭合定界符,GitHub 上要么原样显示星号,要么把加粗配对到错误的文字上。写成 `**结论**。下一句`。
- **禁止绕过**:不得使用 `git commit --no-verify`、`git push --force`;不得为了让校验通过而修改或删除 `.husky/`、`.claude/settings.json`、eslint 豁免清单、lint-staged/commitlint 配置。即使本地绕过,CI 也会以同样标准全量复查。
- `openspec/changes/archive/` 是不可变历史归档,工具层已禁止直接编辑;归档只能走 `openspec archive` 流程。
- 个人化的权限放宽或本地实验配置写在 `.claude/settings.local.json`(已 gitignore),不要改动仓库级 `.claude/settings.json`。

## 校验命令(改完必须全部跑通)

**逐字照抄参数。** `npm run lint` 而非 `lint:ci`、`cargo clippy` 不带 `--all-targets -- -D warnings`、漏掉 `cargo fmt`——这几种写法本地都会通过,而 `.github/workflows/ci.yml` 会拦下来。

`check`/`clippy`/`test` 用 `--workspace` 而不是 `--manifest-path src-tauri/Cargo.toml`:仓库已是 cargo workspace,`--manifest-path` 只覆盖 `vanehub-ai` 一个 crate,`vanehub-permission-hook` 等成员会被静默跳过。`fmt` 是例外,CI 就用 `--manifest-path`。

```bash
npm run lint:ci
npm run test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run native:panic:check
cargo test --workspace
openspec validate --specs --strict
```

上面这些之外,CI 还有几条本地不必每次跑、但相应改动落到你手上时必须跑的:

- `npm run test:coverage`(CI 用它取代 `npm run test`,带覆盖率门槛)、`npm run coverage:policy:test`、`npm run version:unit:test`、`npm run contracts:check`
- `npm run architecture:check`:CI 每次都跑,内部串联 `lint:ci`、`tsc --noEmit` 与架构 fitness 测试;改动跨上下文依赖或 Tauri 边界时本地补跑一次
- UI 行为变更时:`npx playwright test`(CI 的 e2e job 恒跑,本地在改动 UI 行为时必须跑)
- 桌面运行时、Tauri 启动链路或 IPC 行为变更时:`npm run desktop:unit:test` 与 `npm run test:desktop`;后者会真实构建并启动当前操作系统的测试专用桌面客户端,本地结果不得外推到其他平台
- `npm run test:desktop` 含六层,各自独立报结果与证据目录:`desktop-smoke`(启动、IPC、导航)、`desktop-cli-terminal`(CLI Agent 终端往返)、`desktop-cli-management`(临时 PATH 上的 CLI 发现、计划、执行、验证与跨重启持久化)、`desktop-session-workspace`(9 个工作区标签的真实内容)、`desktop-dialogs`(主路径对话框的焦点与提交)、`desktop-settings-persistence`(设置改动跨真实重启)。可用 `npm run test:desktop:<层名>` 单独跑
- 需要 CLI Agent 的层用 `tests/desktop/fixtures/cli/` 下的固定桩程序顶替真实 CLI,不发生模型调用,也不读凭据;`desktop-cli-management` 另建一整套临时 PATH 假 CLI 与假包管理器(`tests/desktop/cli-management-fixture.mjs`),运行结束由 `scripts/desktop/cli-side-effect-guard.mjs` 反查是否碰过真实 npm/WinGet/Vendor URL/凭据/用户数据库——查不到证据即判失败,不判跳过
- 原生 UI 层的选择器写死 zh-CN,并在启动时通过设置服务固定语言:客户端默认语言跟随宿主 locale,不固定就会在非中文 runner 上因文案不符而失败
- 持久化只认原生设置服务,不接受 `localStorage` 作为证据——那是 Web/mock adapter 的持久化,不是桌面客户端的
- CI 的 `Desktop Smoke` 会在 Windows、macOS、Linux 原生 runner 上独立运行;报告结果时逐个平台使用 `PASSED`、`FAILED`、`BLOCKED` 或 `NOT RUN`,失败证据从对应平台的 Actions artifact 获取
- 起了 proposal 时:`openspec validate <change-name> --strict`——CI 对 `openspec/changes/*` 下每个变更逐个校验,`--specs --strict` 不覆盖这一层
