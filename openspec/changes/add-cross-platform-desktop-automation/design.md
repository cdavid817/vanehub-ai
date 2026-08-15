## Context

See `proposal.md` for motivation. Today the repository has Vitest and Cargo coverage gates, a Playwright suite that runs against the Vite Web/mock runtime, platform packaging commands, and native build checks. It does not have an automated layer that launches a built Tauri application and proves the WebView-to-Rust boundary.

The native bootstrap already honors an absolute `VANEHUB_APP_DATA_DIR`, and unified logs derive their fallback location from it. The existing Playwright suite is valuable and remains the fast browser layer. The desktop layer must fit React/service/Tauri adapter/Rust boundaries, must not put direct `invoke()` calls into product components, and must not ship automation permissions in release artifacts.

## Goals / Non-Goals

**Goals:**

- Produce one repeatable desktop build-and-smoke path that behaves consistently on Windows, macOS, and Linux.
- Exercise the real native application while isolating test-only instrumentation and user data.
- Make launch, readiness, IPC, basic interaction, shutdown, and diagnostics deterministic enough for blocking CI.
- Reuse current package scripts, Tauri metadata, unified logging, accessible selectors, and CI conventions.

**Non-Goals:**

- Migrating or deleting the existing Playwright Web/mock suite.
- Covering every business workflow in desktop E2E during this change.
- Automating native file dialogs, tray menus, installers, upgrades, or uninstallers.
- Adding visual-regression baselines, AI visual review, performance benchmarks, soak tests, external LLM tests, or automatic code repair.
- Making the instrumented desktop test artifact byte-for-byte identical to a release artifact.

## Decisions

### Use two complementary E2E layers

Playwright continues to run the Vite frontend with Web/mock adapters for broad, fast UI behavior. A new WebdriverIO/Tauri layer launches a native artifact and owns desktop smoke. Reports and commands label these layers separately so browser success cannot satisfy desktop verification.

Alternative considered: move all E2E tests to the desktop driver. Rejected because it would make the existing broad UI suite slower and more platform-dependent without improving every test's signal.

### Use a dedicated test build feature and Tauri configuration

The Rust automation plugins are optional dependencies selected by a dedicated Cargo feature, and automation capabilities/global APIs are enabled only by a dedicated Tauri test configuration. The desktop build command explicitly selects both. Normal `tauri:build`, packaging, and release commands select neither, and automated checks verify that separation.

The embedded WebDriver provider is the default because it supplies one protocol across Windows, macOS, and Linux and avoids a separate platform driver lifecycle. The service still launches a real Tauri/WebView/Rust process. Test permissions are kept to the smallest set needed for element automation, the real IPC probe, and log forwarding.

Alternative considered: use the external `tauri-driver` path everywhere. Rejected because direct platform WebDriver support is not uniform on macOS and would force a different primary architecture there.

### Build an unbundled, instrumented native artifact for routine smoke

Routine desktop verification builds the native executable without producing installers. This preserves the real runtime boundary while keeping feedback time below full packaging. Packaging and installed-application verification remain separate release concerns.

The artifact resolver reads the Tauri product configuration, Cargo package/default binary metadata, selected target triple, profile, and target directory. It returns a structured manifest containing the absolute executable path, platform, architecture, profile, and test-feature state. It removes stale candidate ambiguity by accepting only the artifact produced for the current invocation.

Alternative considered: hard-code `src-tauri/target/release/vanehub-ai.exe`. Rejected because it fails across platforms, target triples, profiles, custom target directories, and renamed binaries.

### Keep orchestration in Node and native lifecycle in Rust/Tauri

A small TypeScript/Node orchestrator owns host detection, temporary directories, command sequencing, deadlines, artifact resolution, WebdriverIO invocation, result aggregation, and evidence indexing. Native application data, SQLite, logging, window lifecycle, and application commands remain in Rust/Tauri. React product components continue to use service interfaces and runtime adapters.

The orchestrator exposes focused npm commands for build and smoke plus a composed desktop command. A later unified repository verification command may compose the existing mandatory checks and desktop command without duplicating their implementations.

Alternative considered: implement the full orchestrator in Rust. Rejected because test-runner configuration and repository command composition already live in the Node toolchain, while no product-native responsibility requires moving there.

### Reuse native data-directory isolation

For each run, the orchestrator creates a unique temporary root and passes its absolute data subdirectory as `VANEHUB_APP_DATA_DIR`, plus a correlation-only `VANEHUB_TEST_RUN_ID`. Fixtures and result artifacts live in sibling directories. Startup is refused if the resolved data directory is not absolute, is outside the run root, or aliases the normal application-data location.

The test mode does not change business results or bypass adapters. It only enables automation transport, isolation, bounded timeouts, and diagnostics. Cleanup removes the temporary data only after evidence has been copied; failed runs retain the bounded evidence directory, not the live database by default.

Alternative considered: add separate database and log environment variables. Rejected because the current native bootstrap and unified logger already derive both safely from `VANEHUB_APP_DATA_DIR`.

### Define readiness through observable UI plus a real read-only command

Frontend readiness is a stable root marker set after React bootstrap completes. Backend readiness is proven by executing the existing read-only `get_settings` command against the running Tauri application without mocking and validating its response shape. A stable user interaction then navigates a critical surface using accessibility roles or a narrowly added `data-testid` where no semantic selector exists.

The direct test-harness call is not product component code and does not alter the service boundary. Product UI remains prohibited from importing Tauri APIs directly.

Alternative considered: add a test-only readiness command. Rejected because an existing production command provides stronger evidence that real bootstrap state, SQLite, command routing, and serialization are ready.

### Track process ownership from the spawned root

WebdriverIO and the orchestrator record the application root PID, driver PID when applicable, and descendants attributable to the run. Normal shutdown is requested first. After a bounded wait, forced cleanup targets only recorded owned processes. Name-wide termination commands and broad process matching are forbidden.

On Windows, cleanup should use the process/job ownership primitives already present in the native platform layer where applicable; Unix runners use the spawned process group or recorded descendant set. Cleanup never searches for and kills every process named after the application.

### Treat unified native logs as evidence, not a new sink

Frontend console and driver diagnostics are captured by the desktop test service. Native evidence is copied from the isolated unified log directory after its existing redaction pipeline. The result index references each evidence item and notes collection failures without replacing the original test failure. No desktop-test-specific native log file is introduced.

Results use a run-scoped `test-results/desktop/<run-id>/` directory with a machine-readable summary and bounded subdirectories for screenshots and logs. The existing repository ignore policy covers generated test results.

### Add a non-cancelling native CI matrix

CI adds Windows, macOS, and Linux desktop smoke executions with fail-fast disabled. Linux runs with the display/runtime prerequisites required by its WebView. Each job builds locally on its native runner, runs the same npm desktop command, and uploads platform-labelled evidence only on failure or blockage. Existing native build and Playwright jobs remain until later evidence supports consolidation.

## Risks / Trade-offs

- [The embedded WebDriver/plugin ecosystem changes independently from Tauri] → Pin compatible dependency versions, add a minimal runner self-test, and keep the provider behind a narrow test-only boundary.
- [Instrumented artifacts differ from release artifacts] → Explicitly report test-feature state and retain separate packaging/release validation; desktop smoke claims runtime-chain coverage, not byte identity.
- [Cross-platform selectors or timing become flaky] → Prefer accessibility roles and explicit readiness signals, use bounded polling instead of fixed sleeps, and start with one minimal smoke path.
- [Startup background work makes isolation slow] → Reuse a fresh application-data directory and measure startup before changing timeouts; do not bypass initialization.
- [Cleanup could affect a user process] → Require PID/run ownership evidence before termination and fail safely when ownership is uncertain.
- [Failure evidence could expose secrets] → Collect native logs only after unified redaction, bound frontend/driver output, and exclude the temporary database from default CI artifacts.
- [Three native jobs increase CI cost] → Keep the initial suite to smoke, use unbundled builds, and defer broad functional desktop coverage.

## Migration Plan

1. Add pinned test dependencies and test-only Tauri feature/configuration without changing normal build defaults.
2. Add unit-tested platform detection, artifact resolution, isolation, lifecycle, evidence, and result helpers.
3. Add the minimal desktop smoke and run it locally on Windows against the isolated native artifact.
4. Add macOS and Linux CI executions, resolve platform-specific startup issues, and retain diagnostics on failure.
5. Add composed npm commands and agent-facing verification guidance only after the three platform jobs are stable.

Rollback removes the desktop smoke jobs and test-only feature/configuration while leaving existing Vitest, Cargo, Playwright, packaging, and production runtime behavior unchanged.
