## Why

VaneHub AI has substantial Vitest, Rust, and Playwright suites, but it does not measure code coverage or require critical native paths to meet a coverage floor, and its browser E2E tests do not exercise the Tauri-backed Agent lifecycle. This leaves regressions in Agent startup, MCP routing, SQLite transaction behavior, and interactive React flows difficult to detect consistently before merge.

## What Changes

- Add deterministic frontend and Rust coverage reporting that includes otherwise unimported production source and retains reviewable CI artifacts.
- Require the critical Rust paths for Agent startup and terminal control, MCP routing, and SQLite transaction behavior to maintain at least 80% line coverage, while establishing a measured non-regression baseline for the wider native crate.
- Add React component interaction tests with Vitest, a DOM environment, and Testing Library for session-category drag-and-drop and Prompt Hook editing.
- Add a native integration test for the supported lifecycle: create a Session, open its Agent Terminal, stop the terminal, and delete the Session, using real SQLite persistence and deterministic process doubles.
- Tighten GitHub Actions so coverage, tests, lint, builds, strict Clippy, and OpenSpec validation are merge-blocking quality gates with useful failure diagnostics.
- Preserve the existing Web/mock Playwright suite and frontend service boundary; this change does not add Agent CRUD or invoke real provider CLIs in deterministic tests.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `continuous-integration`: Add frontend and Rust coverage reporting, critical-path coverage thresholds, retained reports, and stricter code-quality gates to pull-request and `main` validation.
- `frontend-runtime-architecture`: Require user-observable component interaction coverage for critical drag-and-drop and Prompt Hook editing behavior through runtime-neutral service doubles.
- `native-runtime-architecture`: Require measurable critical-path Rust coverage and a cross-context Session/Agent Terminal lifecycle integration test with real SQLite transaction behavior.

## Impact

- Frontend test infrastructure and dependencies in `package.json`, Vite/Vitest configuration, shared test utilities, and targeted components under `src/main-layout/` and `src/settings/pages/prompt-hooks/`.
- Native test infrastructure under `src-tauri/`, including coverage collection for selected `agent_runtime`, MCP, database, and repository transaction paths.
- GitHub Actions validation and coverage artifacts in `.github/workflows/ci.yml`.
- Both desktop and Web runtimes are covered: native integration targets the desktop application boundary, while component tests and existing Playwright tests preserve Web/mock behavior.
- React components continue to use service interfaces, and tests must not introduce direct Tauri `invoke()` calls or bypass runtime adapters.
