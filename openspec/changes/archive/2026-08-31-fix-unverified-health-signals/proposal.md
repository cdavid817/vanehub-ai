## Why

Three signals in this repository report a result they never established. Each was found by looking
at what a green run actually proves, not by a failure.

**Dependency updates for Rust never arrive.** `.github/dependabot.yml` points the cargo ecosystem at
`directory: /src-tauri`. The root cargo workspace landed in #180 and moved `Cargo.lock` to the
repository root; `src-tauri/Cargo.lock` no longer exists, and all 78 dependencies in
`src-tauri/Cargo.toml` are `workspace = true` with no version literal to bump. `dependabot.yml` was
last touched in #75, before that move. The observable consequence: Dependabot has opened npm pull
requests and **zero** cargo pull requests, while twenty cargo advisories have been raised against
this repository over its life. Nothing anywhere says the cargo updater is idle.

**A browser test asserts nothing on CI.** `real_playwright_worker_bounds_page_operations_handoff_and_artifact_bytes`
guards on `node_modules/playwright/package.json` existing. The `Rust` job runs `npm ci`, so that
guard passes — but browsers are installed only in the `Documentation` and `Playwright E2E` jobs, so
`context.create` fails and the test returns early through a bare `return`. It is green on every run
and has never exercised the bounds it is named for. On a developer machine that happens to hold a
browser cache it takes the full path instead, becoming a local flake source. The signal is inverted:
green where it proves nothing, red where it does.

**A cleanup that ran is reported as a cleanup that failed.** `IsolatedServerTester::run` sets its
deadline before creating the minimal project and spawning the server, then hands the *remainder* to
cleanup. When spawn is slow the remainder is zero, `ManagedChild::shutdown` issues `start_kill()`
and returns `ShutdownTimedOut` without waiting, and the phase is reported failed for a child that
was in fact killed. The spec already requires cleanup "within a fixed deadline"; today that deadline
is whatever the caller had left over, which can be nothing.

**A ceiling nobody is testing decides the result.** The Skill Tool process tests spawn a real
`rustc` and assert that an argument stays one literal token, that a denied environment variable is
refused, and that the child ceiling holds. They run under the *product's* ten-second bound on a whole
skill invocation. On `main`, `Native Check (windows-latest)` went red when one `rustc` startup
exceeded it on a loaded runner — an assertion about argument literalness failing for a reason it
does not mention. The same suite's child-ceiling assertion matches `ResourceLimit(_)` with a
wildcard, and a wall-time timeout produces that same variant, so it can pass for the opposite of the
reason it claims.

**An assertion that cannot hold on Windows.** `writer_stays_inside_the_user_selected_directory`
compares the writer's answer against the path the test handed in. The writer canonicalizes before
writing, and on Windows `canonicalize` returns the extended-length form, so the comparison
fails on a question — containment — that it is not actually asking. No CI job runs the full
`cargo test` on Windows, so it has never been observed there.

## What Changes

- Point the cargo ecosystem at the workspace root so the Rust updater has a lockfile to read, and
  keep the existing `webview2-com` version pin.
- Give the Playwright sidecar test a loud, reasoned skip that names the missing prerequisite, so a
  run that asserted nothing cannot be read as coverage.
- Give cleanup in the isolated server test a minimum budget floor, so a forced termination is
  observed and reported truthfully even when the caller's own deadline is already spent.
- Give the Skill Tool process tests a wall-time they are not asserting on, and make the one
  assertion that *is* about a ceiling name the ceiling it means.
- Compare canonical paths on both sides of the dossier export containment assertion.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `continuous-integration`: dependency update automation covers every ecosystem this repository
  builds, and a test that cannot run says so instead of passing.
- `lsp-server-management`: the isolated server test's cleanup has a floor of its own rather than the
  remainder of the caller's budget.

## Impact

- `.github/dependabot.yml` — one directory, for the cargo ecosystem only.
- `src-tauri/src/contexts/browser_automation/infrastructure/playwright_sidecar_tests.rs` — the skip
  path.
- `src-tauri/src/contexts/code_intelligence/infrastructure/server_test.rs` — the cleanup deadline.
- `src-tauri/src/contexts/tooling/skill_tools/infrastructure/host_process.rs` — test limits only.
- `src-tauri/src/contexts/skill_evolution_generation/infrastructure/export_adapter_tests.rs` — the
  containment assertion only.

No product surface changes. The third item changes one observable contract: `run(timeout)` may now
exceed `timeout` by at most the cleanup floor, which is the cost of reporting cleanup honestly.
Nothing documented promised otherwise.
