## 1. Driver lifecycle reliability

- [x] 1.1 Update the desktop WDIO driver startup path so every worker verifies or restores the test-owned embedded driver before opening its session.
- [x] 1.2 Add a regression check covering a worker that starts immediately after the previous worker's native shutdown.
- [x] 1.3 Recover a test-owned desktop process that remains alive after session teardown, and fail with driver-lifecycle evidence if its port cannot be released.
- [x] 1.4 Avoid WebKitGTK's false clickable hit-test for tabs inside an overflow strip while preserving real WebDriver click and rendered-panel assertions.
- [x] 1.5 Keep UI evaluation downloads and transient SQLite cleanup retries inside the isolated desktop run lifecycle.

## 2. Failure diagnostics

- [x] 2.1 Preserve bounded, redacted browser error and unhandled-rejection details in desktop fatal-marker evidence.
- [x] 2.2 Update screen-sweep assertions and WDIO result handling to distinguish assertion failures from expected blocked scenarios.

## 3. Verification

- [x] 3.1 Run focused desktop harness tests and `VANEHUB_DESKTOP_FULL_SUITE=1 npm run test:desktop` on Linux.
- [x] 3.2 Run `npm run lint:ci`, `npm run test`, `npm run build`, `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm run native:panic:check`, `cargo test --workspace`, and `openspec validate --specs --strict`.
- [x] 3.3 Run `openspec validate stabilize-desktop-wdio-lifecycle --strict` and record the desktop evidence status by platform.
- [x] 3.4 Reconcile stale desktop observability expectations with the connector-scoped session access contract exposed by both native and Web adapters.
