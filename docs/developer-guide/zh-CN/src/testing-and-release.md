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
