## Context

The repository has a packaging workflow but no pull-request or `main` branch validation workflow. Existing npm scripts already expose lint, build, unit-test, and contract checks, while Cargo and Playwright provide the native and browser checks. GitHub-hosted Ubuntu runners require Tauri's Linux development packages before Rust compilation.

## Goals / Non-Goals

**Goals:**

- Run frontend, Rust, and E2E validation for every pull request and push to `main`.
- Use lockfile-based npm installation and the stable Rust toolchain with formatting and Clippy components.
- Preserve a Playwright HTML report when E2E execution fails.
- Keep independent validation areas visible as separate GitHub checks.

**Non-Goals:**

- Package or publish desktop applications.
- Add new test frameworks, production dependencies, or application behavior.
- Test an operating-system matrix; packaging remains responsible for cross-platform builds.

## Decisions

- Use three Ubuntu jobs: frontend, Rust, and Playwright. Independent jobs expose focused failures and allow E2E to run even when another validation area fails. A single sequential job was rejected because an early failure would suppress later evidence.
- Use Node.js 22 with `npm ci`, matching the existing packaging workflow and committed npm lockfile. Switching package managers is outside project constraints.
- Run `npm run contracts:check` as the requested Vitest contracts gate because it targets the repository's contract-conformance test directly; the general `npm run test` suite is outside the explicitly requested CI list.
- Install the established Tauri Linux prerequisites before Cargo commands. Rust uses stable with `rustfmt` and `clippy`, and Cargo commands target `src-tauri/Cargo.toml`.
- Install only Playwright Chromium and its system dependencies, matching the sole configured Playwright project. Upload `playwright-report/` with `if: failure()` and `if-no-files-found: ignore` so report upload cannot mask the original test result.

## Risks / Trade-offs

- [GitHub runner image changes can affect native packages] → Install required packages explicitly in the Rust job.
- [Three jobs repeat checkout and some setup] → Accept minor setup overhead for parallel execution and clearer required checks.
- [A failed browser launch may produce no HTML report] → Ignore a missing report during upload while retaining the E2E failure.
