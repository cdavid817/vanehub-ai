# 校验与真实联调 Runbook

## 1. 执行环境与证据格式

先记录 HEAD、分支、平台/架构、Node/npm/Rust/OpenSpec 版本、工作树初始状态。下列命令依据审阅基线 AGENTS.md 与 package.json；执行时重新读取当前版本，以更近的仓库要求为准，不削减或绕过质量闸门。

结果使用 PASSED / FAILED / BLOCKED / NOT RUN。必须有真实命令、退出码和证据路径。BLOCKED 需要具体缺失条件；NOT RUN 不是通过。未要求的可选能力不做成功宣传，但其“不支持”负向测试仍需执行。

## 2. OpenSpec（先于生产代码）

```bash
openspec --version
openspec validate extend-cli-providers-with-acp --strict
openspec validate --specs --strict
```

第一条 change 校验不可由全主规范校验替代。若当前规范发生变化，MODIFIED requirement 按当前完整段落整合并保留原有 scenario 标题。不编辑 archive。工具不可用时检查仓库既有本地安装方式，不擅自修改锁文件或宣称校验已过。包内结构检查仅辅助，不能代替 OpenSpec 官方 CLI。

## 3. 基础必跑命令

在仓库根目录执行：

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
openspec validate extend-cli-providers-with-acp --strict
```

Cargo check/clippy/test 使用 workspace；不要仅检查主 Tauri crate。没有 Rust/OS 构建依赖则记录 BLOCKED 与原始错误，不更改代码关闭构建特性来假通过。

## 4. 本变更适用的附加检查

```bash
npm run test:coverage
npm run coverage:policy:test
npm run version:unit:test
npm run contracts:check
npm run architecture:check
npm run docs:check
npx playwright test
npm run desktop:unit:test
npm run test:desktop
```

如改动参数生成源，使用当前 `npm run contracts:generate` 等生成器，再运行 check；不要手改单一生成文件。构建/覆盖率/文档失败需要修复，不调低门槛、不新增 lint max-lines 豁免、不绕过 hooks。

定向桌面调试可先 `npm run test:desktop:build`，然后运行当前 package.json 已有的 `test:desktop:cli-terminal`、`test:desktop:cli-management`、`test:desktop:scheduled-tasks` 等层。单层测试不能替代全套必要验收。遵守测试语言固定、原生设置持久化与 side-effect guard，不以 localStorage 证明原生持久化。

## 5. 新功能定向自动测试

实现 [test-matrix.md](test-matrix.md) 中用例，再根据实际新增测试文件运行 Vitest / Cargo / Node / Playwright。不要声称本包中存在 fake ACP 可执行代码；本包提供的是需要实现的规格。

protocol/permission fixture 不能使用真实用户账号；spawn 所有权、取消和拒绝要有临时文件/进程观察证据。逐项对应 [requirements-index.json](requirements-index.json) 与 [acceptance.md](../acceptance.md)。

## 6. 真实 CLI Smoke

前提：用户已明确授权此 Provider 在隔离测试工作区执行真实任务，安装与认证来自受信配置。默认不自动安装全局 CLI、不改登录、不调用付费模型、不读取真实业务仓库数据。授权不存在则记录 BLOCKED，不制造“免费验证”。

每个活跃 Provider 的同一精确版本在对应平台记录：

| 步骤 | 动作 | 证据 |
| --- | --- | --- |
| 身份 | 只读版本/帮助与真实路径 | 发行形态、版本、平台/架构、指纹摘要 |
| 握手 | 由实现后的 VaneHub 建立连接 | 协议版本、声明能力、安全摘要 |
| 文本 | 两轮简单无敏感文本 | 两次正常 turn 结束，同一会话 |
| 工具 | 在临时目录请求受控文件操作 | 拒绝不执行；批准后只改授权文件 |
| 取消 | 取消活动 turn | 协议结局或升级清理证据 |
| 交互 | 能触发时测试问题/计划/权限 | scoped request 与响应关联 |
| 恢复 | 若明确支持则 load 已知 session | 绑定匹配、无重复消息/用量 |
| 故障 | 关闭所拥有的测试连接 | interrupted 与无盲重放 |
| 回收 | 结束测试与检查进程 | 无孤儿进程、不触碰其他会话 |

iFlow 只做其本地 legacy 终端范围，不以缺失官方服务要求申请新账号或启用已关闭 API。真实功能不可触发或环境不具备时照实记录，不把合成触发的结果填入 live 列。

## 7. 分平台报告

Windows、macOS、Linux 分别记录，架构分别记录。WSL 是 Linux 环境，不能证明 Windows 原生通过。明确支持范围与未支持的上游架构；例如上游未支持架构应验证拒绝路径，不宣称全平台运行。

## 8. 实施结束

更新 tasks 中真实完成的项与 results；保留失败和阻塞。默认不自动归档、commit、push 或开 PR。用户另行要求归档时，再执行项目要求的官方归档流程与索引更新，不能用 zip 代替 Git 中归档工件。
