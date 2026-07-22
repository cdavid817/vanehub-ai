## Why

Pull requests and updates to `main` currently lack one repeatable GitHub gate that exercises the repository's required frontend, contract, Rust, and browser checks. Adding CI makes regressions visible before changes are merged and preserves browser diagnostics when E2E tests fail.

## What Changes

- Add a GitHub Actions CI workflow for pull requests and pushes to `main`.
- Run ESLint, the TypeScript/Vite build, Vitest contract conformance, Rust formatting, `cargo check`, Clippy, Rust tests, and Playwright E2E checks.
- Upload the Playwright HTML report as a workflow artifact when E2E execution fails.

## Capabilities

### New Capabilities

- `continuous-integration`: Defines the required GitHub-hosted validation pipeline and failure diagnostics for repository changes.

### Modified Capabilities

None.

## Impact

- Adds `.github/workflows/ci.yml` and associated repository automation behavior.
- Uses existing npm, Cargo, and Playwright commands without changing production dependencies or application APIs.
- Affects validation for both desktop and Web runtime code while preserving the existing frontend/backend and adapter boundaries.
