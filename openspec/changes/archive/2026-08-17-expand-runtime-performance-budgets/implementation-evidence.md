# Implementation Evidence

## Deterministic Performance Gates

`npm run performance:check` passed on Windows x64 using build profile `test` and source commit `f3441915d4658432380a983f8b089fe975d92f4e`. The worktree contained this uncommitted change when the evidence was captured.

| Dataset | Version | Metric | Baseline | Measured | Budget | Delta | Status |
| --- | ---: | --- | ---: | ---: | ---: | ---: | --- |
| repo-small | 1 | context.candidate-operations | 18 | 18 | 32 | 0 | PASSED |
| repo-large | 1 | context.occupancy-overflows | 0 | 0 | 0 | 0 | PASSED |
| runs-1000 | 1 | mission-control.query-count | 4 | 4 | 4 | 0 | PASSED |
| runs-1000 | 1 | mission-control.loaded-rows | 50 | 50 | 50 | 0 | PASSED |
| runs-1000 | 1 | run.retained-events | 8000 | 8000 | 8000 | 0 | PASSED |
| token-stream-high-rate | 1 | stream.update-batches | 20 | 20 | 20 | 0 | PASSED |
| terminal-long | 1 | terminal.max-chunk-bytes | 4096 | 4096 | 32768 | 0 | PASSED |
| repo-large | 1 | lsp.max-response-items | 50 | 50 | 50 | 0 | PASSED |
| repo-large | 1 | context.selection-p95 (dedicated) | 1.159 ms | 1.159 ms | 1.449 ms | 0 ms | PASSED |
| repo-small | 1 | app.cold-start (informational, unsampled) | 0 ms | 0 ms | 0 ms | 0 ms | PASSED |

The known N+1 regression measured 1,001 queries against a baseline and budget of 4 and was rejected without mutating the accepted manifest.

## Windows Dedicated Benchmark

`npm run performance:benchmark` passed on Windows x64 with seven samples per dataset. Results are evidence-only and are not shared-runner gates.

| Dataset | Version | P50 | P95 | Status |
| --- | ---: | ---: | ---: | --- |
| repo-small | 1 | 0.096 ms | 0.375 ms | PASSED |
| repo-medium | 1 | 2.809 ms | 3.344 ms | PASSED |
| repo-large | 1 | 26.142 ms | 38.615 ms | PASSED |
| runs-100 | 1 | 0.079 ms | 0.211 ms | PASSED |
| runs-1000 | 1 | 0.127 ms | 0.173 ms | PASSED |
| terminal-long | 1 | 0.067 ms | 0.342 ms | PASSED |
| token-stream-high-rate | 1 | 0.001 ms | 0.053 ms | PASSED |

Focused native measurements also passed for Context Engine selection, 1,000 canonical Run histories, constant-query Mission Control persistence, Tree-sitter parsing, code indexing/search, LSP definition/references, and 16 MiB terminal search. Their console evidence is retained in the verification run output.

## Compatibility and Scope

- No SQLite schema or migration changed.
- No public Tauri command or IPC payload changed.
- No `AgentService` contract changed; both Web and Tauri adapters retain the same interface.
- The Web-only 100/1,000 Run seed/reset exports follow existing `*ForTest` fixture conventions and are not adapter methods.
- No bounded-context ownership, unified-log path, production UI component, theme token, or user-visible behavior changed.
- Roadmap item 11 and later requirements were not implemented.

## Native Platform Status

| Platform | Status |
| --- | --- |
| Windows x64 | PASSED |
| macOS | NOT RUN |
| Linux | NOT RUN |

## Final Quality Gates

| Gate | Result |
| --- | --- |
| `npm run lint:ci` | PASSED |
| `npm run test` | PASSED — 281 files, 1,287 tests |
| `npm run build` | PASSED — 16 lazy chunks and 130.6 KiB gzip main closure verified |
| `npm run test:coverage` | PASSED — 70.4% statements, 66.62% branches, 66.18% functions, 74.44% lines |
| `npm run contracts:check` | PASSED — 3 tests |
| `npx playwright test` | PASSED — 151 tests |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASSED |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | PASSED |
| `cargo test --manifest-path src-tauri/Cargo.toml` | PASSED — 3,481 main-library tests plus all integration, architecture, binary, and doc-test targets |
| `cargo check --manifest-path src-tauri/Cargo.toml` | PASSED |
| `npm run desktop:unit:test` | PASSED — 11 tests |
| `npm run test:desktop` | PASSED — Windows x64 native client built and exercised |
| `openspec validate --specs --strict` | PASSED — 136 specs |
| `openspec validate expand-runtime-performance-budgets --strict` | PASSED |

The full browser run emitted pre-existing React `flushSync` and `ResizeObserver` console diagnostics. All 151 Playwright cases passed; no runtime-performance budget was breached. The full test run also exposed and fixed a time-initialization defect in the Context Quality Web fixture by rebuilding its ledger after the test clock is installed.
