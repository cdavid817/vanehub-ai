# Verification

Date: 2026-07-18
Platform: Windows x64, Tauri 2, WebView2
Isolation: the packaged-runtime smoke used an absolute `VANEHUB_APP_DATA_DIR` below `src-tauri/target`; the user's VaneHub SQLite database, logs, and credentials were not opened or modified. Host policy denied recursive removal afterward, so the generated smoke directory remains only under the Git-ignored target tree.

## Automated verification

| Check | Result |
|---|---|
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Pass |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Pass: 496 library tests, 8 architecture tests, 0 failures, 1 intentionally ignored child fixture |
| `cargo check --manifest-path src-tauri/Cargo.toml` | Pass with no code warnings |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets` | Pass with no warnings |
| `npm run lint` | Pass with 0 errors; 16 pre-existing React Hooks warnings remain |
| `npm run test` | Pass: 53 files and 171 tests |
| `npm run contracts:check` | Pass: 1 contract-conformance test |
| `npm run build` | Pass; the existing Vite large-chunk advisory remains |
| `npx playwright test` | Pass: 40 Web/mock browser tests |
| `npm run tauri:build` | Pass: release EXE plus MSI and NSIS bundles generated |
| `openspec validate "refactor-rust-ddd-architecture" --strict` | Pass |
| `openspec validate --specs --strict` | Pass: 48 main specifications |

The Windows linker emitted only its localized library-creation messages during `cargo test`; `cargo check` and Clippy confirmed there are no Rust code warnings.

## Packaged desktop smoke

The release EXE was launched hidden with an isolated app-data directory and a temporary WebView2 CDP port. The main WebView remained responsive at `http://tauri.localhost/workspace` while the following real Tauri commands were exercised:

| Scenario | Result |
|---|---|
| Native startup and data isolation | Pass. `get_data_management_info` reported the isolated `vanehub.sqlite` path before any mutating command ran. |
| Settings | Pass. `get_settings` returned the persisted settings contract. |
| MCP task | Pass. A local no-network stdio fixture produced the expected failed terminal operation with an associated log, proving background task and status retrieval behavior. |
| CLI refresh | Pass. `refresh_cli_detections` reached `succeeded` with ten operation log entries. |
| Session and chat | Pass. A CLI session was created through its background operation, its chat configuration round-tripped, and an empty message was rejected without persisting a message or launching an Agent. |
| Workspace shell | Pass. A native PTY was created, resized, written to, and killed through the published shell commands. |
| IM status | Pass. `list_im_connectors` returned all five connector status views without storing credentials or using live networks. |
| Operation registry | Pass. MCP, CLI, and session operations remained queryable by their stable ids. |
| Cleanup | Pass. The shell, session, and MCP fixture were removed; the exact smoke process was terminated and its CDP listener released. |

## Web/mock boundary

The 40-test Playwright run covered browser session/chat, CLI settings, IM settings, and workspace surfaces. A production-source scan of `web-*.ts` and `runtime-*.ts` service implementations found no `@tauri-apps` imports or `invoke()` calls; the only matches were negative assertions in boundary tests.

## Deferred follow-up

No DDD migration task or compatibility requirement is deferred. The following nonblocking repository/toolchain advisories remain outside this Rust architecture change:

- Resolve the 16 existing React Hooks lint warnings in their owning frontend changes.
- Revisit frontend chunk splitting for the existing Vite large-chunk advisory.
- Change the bundle identifier ending in `.app` only through a separate migration-aware proposal because it affects platform application identity and data paths.
- Add packaged-runtime smoke coverage for non-Windows targets in platform-specific CI; this run intentionally used no live MCP endpoint, Agent provider, IM credential, or external network.
