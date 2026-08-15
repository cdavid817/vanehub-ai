## Why

The existing Vitest, Cargo, and Playwright suites verify frontend, native, and Web/mock behavior, but they do not prove that a built Tauri application can start, render its React surface, cross a real IPC boundary, and shut down safely. Runtime-affecting changes therefore need a repeatable current-platform desktop smoke gate with isolated data and reviewable failure evidence.

## What Changes

- Add a cross-platform desktop verification capability that builds and launches a real Tauri test artifact on the current operating system.
- Add a minimal desktop smoke contract covering application launch, frontend readiness, real Tauri IPC readiness, basic interaction, fatal-error detection, clean shutdown, and owned-process cleanup.
- Add a test-only WebdriverIO/Tauri automation boundary that is excluded from normal production and release builds.
- Reuse `VANEHUB_APP_DATA_DIR` for per-run temporary data isolation and add run correlation without bypassing product services, Rust commands, SQLite, or the Tauri IPC boundary.
- Resolve the desktop artifact from project and build metadata instead of hard-coding a platform-specific executable path.
- Collect screenshots, test assertions, driver output, process state, and redacted unified application logs when desktop verification fails.
- Add stable npm entry points for desktop build and smoke verification while retaining the existing Playwright Web/mock suite as a separate fast browser layer.
- Extend CI so Windows, macOS, and Linux runners each build, launch, and smoke-test their own native desktop artifact without claiming unexecuted platforms passed.
- Define explicit `PASSED`, `FAILED`, `BLOCKED`, `NOT RUN`, and reason-bearing `NOT REQUIRED` result states.

This change affects the desktop runtime and repository verification workflow. It does not change Web/mock product behavior or replace browser Playwright tests.

## Capabilities

### New Capabilities

- `desktop-runtime-verification`: Defines real native desktop artifact construction, test-only automation isolation, smoke acceptance, data and process safety, evidence collection, and verification result reporting.

### Modified Capabilities

- `continuous-integration`: Requires native Windows, macOS, and Linux jobs to execute desktop smoke verification and retain diagnostic artifacts on failure.

## Impact

- Adds Node/WebdriverIO development dependencies and test-only Tauri plugin dependencies behind an explicit Cargo feature and dedicated Tauri test configuration.
- Adds desktop test configuration, helpers, smoke specifications, artifact resolution, orchestration scripts, and npm commands.
- Updates the Tauri bootstrap and capabilities only for the test build; production and release builds must not expose automation plugins or their permissions.
- Reuses the existing native database override and unified redacted logging system rather than introducing feature-local persistence or logs.
- Updates GitHub Actions platform jobs and artifact retention behavior.
- May add stable accessibility attributes or `data-testid` selectors to critical React surfaces without allowing tests to bypass the frontend service boundary.
- Does not change the React service contract, Tauri/Web adapter parity, or application database schema unless implementation discovery identifies a separately specified need.
