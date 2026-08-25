## 1. MCP Runtime Projection

- [x] 1.1 Extend prepared MCP projection data to carry bounded provider environment overrides alongside arguments and cleanup ownership.
- [x] 1.2 Prepare active workspace-visible MCP servers for every CLI generation while attaching relay observation metadata only when explicitly enabled.
- [x] 1.3 Generate and structurally merge invocation-scoped OpenCode MCP configuration without logging or overwriting unrelated inline configuration.
- [x] 1.4 Add Rust unit tests for observation-disabled availability, Claude/Codex argument projection, OpenCode environment projection and merge failures, scope filtering, and cleanup.

## 2. Desktop Agent Fixtures

- [x] 2.1 Add deterministic Claude Code, Codex, and OpenCode process fixtures that validate their production invocation shapes and exercise the projected MCP protocol path.
- [x] 2.2 Add a deterministic local OnePiece-compatible provider fixture that requests the cached MCP tool, waits for approval, and completes without a real credential.
- [x] 2.3 Add fixture evidence and cleanup helpers that distinguish provider projection, MCP initialize/list/call traffic, and Agent completion.

## 3. WebdriverIO Verification

- [x] 3.1 Add an isolated Agent MCP WebdriverIO configuration and npm entry point with run-scoped application data, CLI configuration, provider fixtures, MCP fixtures, and artifacts.
- [x] 3.2 Verify MCP creation, successful connection/cache, and actual use in separate Claude Code, Codex, OpenCode, and OnePiece single-Agent sessions.
- [x] 3.3 Verify actual MCP use by each routed seat in a heterogeneous Claude Code, Codex, and OpenCode multi-Agent session, including resumed or handed-off turns.
- [x] 3.4 Add the Agent MCP layer to composed desktop orchestration and update orchestration contract tests.

## 4. Verification

- [x] 4.1 Run focused Rust MCP/Agent runtime tests and fixture contract tests.
- [x] 4.2 Build and run the Agent MCP WebdriverIO desktop layer and retain its evidence summary.
- [x] 4.3 Run `npm run lint:ci`, `npm run test`, `npm run build`, `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm run native:panic:check`, `cargo test --workspace`, and `openspec validate --specs --strict`.
- [x] 4.4 Run `npm run architecture:check`, `npm run desktop:unit:test`, `npm run test:desktop`, and `openspec validate verify-agent-mcp-runtime --strict` for the affected native boundary and desktop behavior.
- [x] 4.5 Preserve WebDriver session teardown on macOS by allowing WDIO to delete the session before the test-only application exit runs.
