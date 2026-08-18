# 测试、打包与发布

运行与改动相匹配的仓库校验命令:

```powershell
npm run lint:ci
npm run test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
openspec validate --specs --strict
```

文档改动还需额外运行:

```powershell
npm run docs:check
npm run docs:test
npm run docs:screenshots:check
npm run docs:build
```

前端测试覆盖纯契约与可见的组件行为。Playwright 覆盖浏览器 Web/mock 运行时;通过它并不代表 Tauri 桌面运行时也通过了。native 测试覆盖领域不变量、应用端口编排、持久化/迁移、命令映射、进程安全与生命周期行为。

影响运行时的桌面改动还需额外使用:

```powershell
npm run desktop:unit:test
npm run test:desktop
```

`test:desktop` 会为当前操作系统构建并启动一个带埋点的 native Tauri 产物,等待真实的 React WebView,调用真实的 Rust 后端 `get_settings` 命令,执行一次稳定的导航交互,并请求一次干净的应用关闭。它会设置一个隔离的临时 `VANEHUB_APP_DATA_DIR`;切勿将该变量指向正常的用户数据。

带埋点的产物通过 `desktop-e2e` Cargo feature 和 `src-tauri/tauri.desktop-e2e.conf.json` 启用仅测试可用的 WebDriver 插件与权限。正常的打包命令不包含该 feature。失败证据从截图、驱动输出、进程状态以及既有的已脱敏统一 native 日志写入到 `test-results/desktop/<run-id>/` 之下。

本地结果仅适用于当前平台。CI 在原生 Windows、macOS 与 Linux runner 上独立运行 `Desktop Smoke`,且禁用了矩阵的 fail-fast。逐个平台审查并报告为 `PASSED`、`FAILED`、`BLOCKED` 或 `NOT RUN`;切勿从一个平台推断另一个平台。失败或被阻塞的任务上传带平台标签的证据产物,而成功的任务不保留临时应用数据。

打包通过 Tauri 面向 Windows、macOS 与 Linux。签名凭据属于受保护的发布环境,绝不放入仓库配置或截图。参见已签入的 [发布签名指南](../../reference/release-signing.md)。

## 测试层级

测试金字塔自下而上分为五层,每一层覆盖不同的风险面,且彼此不能互相替代——尤其"Playwright 通过 ≠ desktop 通过"。

```mermaid
flowchart TD
    Unit[Vitest 单元/组件测试<br/>纯契约与可见组件行为]
    E2E[Playwright e2e<br/>仅 Web/mock 运行时]
    Rust[Rust 原生测试<br/>领域不变量/迁移/进程安全/生命周期]
    Smoke[Desktop Smoke<br/>插桩桌面产物 desktop-e2e feature]
    Matrix[CI Matrix<br/>Windows / macOS / Linux<br/>fail-fast: false]
    Unit --> E2E
    Unit --> Rust
    E2E --> Smoke
    Rust --> Smoke
    Smoke --> Matrix
    Matrix --> Gates{各平台独立判定}
    Gates -->|通过| PASSED[PASSED]
    Gates -->|失败| FAILED[FAILED]
    Gates -->|被阻塞| BLOCKED[BLOCKED]
    Gates -->|未运行| NOTRUN[NOT RUN]
```

层级说明：

- **Vitest**：单元/组件测试，覆盖纯契约与可见的组件行为。覆盖率门槛由 `coverage-policy.json` 强制——前端 `minimumLines = 45.2%`。
- **Playwright e2e**：仅覆盖浏览器 Web/mock 运行时；通过它并不代表 Tauri 桌面运行时也通过了。
- **Rust 原生测试**：覆盖领域不变量、应用端口编排、持久化/迁移、命令映射、进程安全与生命周期行为。整体覆盖率门槛 `minimumLines = 67%`，另有三个 `criticalGroups` 各要求 `minimumLines = 80%`：
  - `sqlite-transactions`：`sessions/infrastructure/transactions.rs`、`platform/database/mod.rs`、`platform/database/migrations.rs`；
  - `agent-startup-and-terminal-control`：`agent_runtime/application/terminal_service.rs`；
  - `mcp-routing`：`tooling/mcp/infrastructure/relay.rs`。
- **Desktop Smoke**：插桩桌面产物通过 `desktop-e2e` Cargo feature 与 `src-tauri/tauri.desktop-e2e.conf.json` 启用仅测试可用的 WebDriver 插件与权限；正常打包命令不包含该 feature。它会构建并启动当前操作系统的带埋点 Tauri 产物,等待真实 React WebView,调用真实 Rust 后端 `get_settings` 命令,执行一次稳定导航,并请求干净关闭。本地结果仅适用于当前平台,不得外推。
- **CI Matrix**：`Desktop Smoke` 在原生 Windows、macOS 与 Linux runner 上独立运行,且禁用了矩阵的 `fail-fast`。逐个平台审查并报告为 `PASSED`/`FAILED`/`BLOCKED`/`NOT RUN`；切勿从一个平台推断另一个平台。

补充脚本在改动落到相应区域时必须跑：`npm run test:coverage`(CI 用它取代 `npm run test`)、`npm run coverage:policy:test`、`npm run version:unit:test`、`npm run contracts:check`。UI 行为变更时 `npx playwright test`;运行时/启动链路/IPC 变更时 `npm run desktop:unit:test` 与 `npm run test:desktop`。

## 发布流程

发布是一次跨三平台的同步打包与签发。版本号在 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 三处必须一致,由 `version:check` 守护。

```mermaid
sequenceDiagram
    participant Dev as 发布者
    participant Sync as 版本同步
    participant Check as version:check + 全量验证
    participant Tag as git tag
    participant PKG as 三平台打包 job
    participant Win as Windows runner
    participant Mac as macOS runner
    participant Lin as Linux runner
    participant Pub as publish job
    Dev->>Sync: 同步版本号<br/>package.json / Cargo.toml / tauri.conf.json
    Sync->>Check: version:check + lint:ci + test + build<br/>+ cargo fmt/clippy/test/check<br/>+ openspec validate --specs --strict
    Check-->>Dev: 全绿才继续
    Dev->>Tag: 打 tag
    Tag->>PKG: 触发三平台 package workflow
    par Windows
        PKG->>Win: NSIS .exe<br/>签名
    and macOS
        PKG->>Mac: .dmg<br/>notarize + staple
    and Linux
        PKG->>Lin: .deb + AppImage
    end
    Win-->>Pub: 上传产物
    Mac-->>Pub: 上传产物
    Lin-->>Pub: 上传产物
    Pub->>Pub: 生成 SHA256SUMS<br/>生成 SPDX SBOM<br/>生成证言 attestation<br/>汇编 Release Notes
    Pub-->>Dev: 发布完成
```

发布要点：

- **版本同步**：`package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 三处版本号必须一致；`scripts/check-version-sync.mjs` 做交叉校验,`version:unit:test` 是其单元测试。
- **全量验证先行**：打 tag 前必须跑通 `AGENTS.md` 末尾的全部校验命令,外加 `version:check`。
- **三平台产物**：Windows 产出 NSIS `.exe`；macOS 产出 `.dmg` 并走 notarize + staple；Linux 产出 `.deb` 与 `AppImage`。
- **publish 产物清单**：`SHA256SUMS`（逐文件 sha256，并校验无重复哈希）、SPDX SBOM、证言（attestation）、Release Notes。
- **更新器签名**：自动更新器使用 `TAURI_SIGNING_PRIVATE_KEY`（与密码）签名,签名密钥属于受保护发布环境,绝不放入仓库配置或截图。空密钥走 rehearsal-only 路径,不产生可分发的更新签名。
- **签名凭据隔离**：签名凭据只在 CI 受保护环境注入,正常本地打包命令不含 `desktop-e2e` feature,也不接触签名密钥。

打包与签名细节见 `src-tauri/ARCHITECTURE.md` 与 `../../reference/release-signing.md`;CI 编排见 `.github/workflows/ci.yml` 与 `.github/workflows/package.yml`。

## 关键脚本与命令

测试与发布链路的核心脚本与门槛:

- **Vitest**:`package.json` `test: "vitest run"`、`test:coverage: "vitest run --coverage --maxWorkers=4 --testTimeout=15000"`;配置在 `vite.config.ts`(排除 `tests/docs/**` 与 `tests/e2e/**`,coverage 输出 `./coverage/frontend`)。
- **Playwright**:`playwright.config.ts` `testDir: "./tests/e2e"`,chromium 单 project,`webServer` 起 `npm run dev --port 5174`;`tests/e2e/` 有 45 个 spec。
- **桌面原生测试**:`npm run test:desktop` = `node scripts/test-desktop.mjs all`,分 `test:desktop:build` 与 `test:desktop:smoke`;`test-desktop.mjs` 用 `--features desktop-e2e --config src-tauri/tauri.desktop-e2e.conf.json` 构建插桩 debug 工件,起真实 React WebView → invoke `get_settings` → 导航 → 干净退出;隔离临时 `VANEHUB_APP_DATA_DIR`;失败证据写 `test-results/desktop/<run-id>/`。
- **`desktop:unit:test`** —— 跑 `scripts/desktop-*.node-test.mjs`(自动化边界)。
- **覆盖率门槛** `coverage-policy.json`:frontend `minimumLines: 45.2%`、native `minimumLines: 67%`,三个 `criticalGroups` 各 80 行(`sqlite-transactions`、`agent-startup-and-terminal-control`、`mcp-routing`);检查脚本 `scripts/check-coverage-policy.mjs`。
- **CI Desktop Smoke** `.github/workflows/ci.yml`:`desktop-smoke` job matrix windows/macos/ubuntu,`fail-fast: false`,Linux 用 `xvfb-run -a npm run test:desktop`,失败按平台标注证据。
- **打包目标** `package.json`:6 个目标 `package:windows:{x64,arm64}`、`package:macos:{x64,arm64}`、`package:linux:{x64,arm64}`,每个先 `sidecar:prepare -- --release --target=...`。
- **版本同步** `scripts/check-version-sync.mjs`:三处(package.json/Cargo.toml/tauri.conf.json)版本必须一致,`version:unit:test` 是其单元测试。
- **签名凭据**:受保护 `release` environment 存凭据(APPLE_CERTIFICATE/APPLE_SIGNING_IDENTITY/TAURI_SIGNING_PRIVATE_KEY/WINDOWS_CERTIFICATE 等);`environment` 由 `github.ref_type=='tag'?'release':'build-preview'` 决定;updater 用 `TAURI_SIGNING_PRIVATE_KEY` 生成 `createUpdaterArtifacts`,公钥内嵌 tauri.conf.json。
