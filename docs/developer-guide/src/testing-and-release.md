# Testing, packaging, and release

Run the repository verification commands appropriate to the change:

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

Documentation changes additionally run:

```powershell
npm run docs:check
npm run docs:test
npm run docs:screenshots:check
npm run docs:build
```

Frontend tests cover pure contracts and visible component behavior. Playwright covers the browser Web/mock runtime; passing it does not claim that the Tauri desktop runtime passed. Native tests cover domain invariants, application port orchestration, persistence/migrations, command mapping, process safety, and lifecycle behavior.

Runtime-affecting desktop changes additionally use:

```powershell
npm run desktop:unit:test
npm run test:desktop
```

`test:desktop` builds and launches an instrumented native Tauri artifact for the current operating system, waits for the real React WebView, invokes the real Rust-backed `get_settings` command, performs a stable navigation interaction, and requests a clean application shutdown. It sets an isolated temporary `VANEHUB_APP_DATA_DIR`; never point that variable at normal user data.

The instrumented artifact enables test-only WebDriver plugins and permissions through the `desktop-e2e` Cargo feature and `src-tauri/tauri.desktop-e2e.conf.json`. Normal packaging commands do not include that feature. Failure evidence is written beneath `test-results/desktop/<run-id>/` from screenshots, driver output, process state, and the existing redacted unified native logs. Local results apply only to the current platform; CI validates Windows, macOS, and Linux independently.

Packaging targets Windows, macOS, and Linux through Tauri. Signing credentials belong in protected release environments, never in repository configuration or screenshots. See the checked-in [release signing guide](../reference/release-signing.md).
