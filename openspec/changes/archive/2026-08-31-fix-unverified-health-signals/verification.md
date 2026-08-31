# Verification

Run on Windows 11, on the branch head. Nothing below is inferred from another platform.

| Command | Result |
| --- | --- |
| `npm run lint:ci` | PASSED |
| `npm run test` | PASSED — 2 823 |
| `npm run build` | PASSED |
| `npm run deps:config:test` | PASSED — 7 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | PASSED |
| `cargo check --workspace` | PASSED |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASSED |
| `npm run native:panic:check` | PASSED |
| `cargo test --workspace` | PASSED — 6 251, 15 ignored, 0 failed |
| `openspec validate --specs --strict` | PASSED — 147 |
| `npm run architecture:check` | PASSED |

The two Windows steps that `Native Check` runs were also run as CI runs them:
`cargo test --manifest-path src-tauri/Cargo.toml skill_tool --lib` (137) and the same with
`--features skill-tool-module-runtime` (142).

## Each fix, and the evidence it is a fix

**Cargo updates reach a lockfile.** `npm run deps:config:check` reports all three ecosystems
verified. The check's own suite includes the exact failing shape — a directory holding
`Cargo.toml` but no `Cargo.lock` — so it refuses the configuration this change replaces rather than
merely accepting the new one.

**The browser test says when it did not run.** Verified in both directions: with a browser present
the full path runs and passes, and under `PLAYWRIGHT_BROWSERS_PATH=0` it prints
`SKIPPED … the `playwright` package is installed but no Chromium browser is`. The `Rust` job now
installs Chromium, so on CI it takes the first path rather than the second.

**Cleanup is budgeted.** `a_spent_caller_deadline_still_leaves_cleanup_enough_to_observe_the_kill`
fails without the floor with `left: Failed, right: Succeeded` — the reported-failure defect
reproduced — and passes with it.
`a_cleanup_that_finishes_early_does_not_wait_out_the_floor` covers the other direction.

**The Skill Tool ceiling.** The failure on `main` was
`ResourceLimit("process.wall-time")` from a `rustc` startup that exceeded the product's ten-second
invocation bound. Both Windows steps now pass locally. The child-ceiling assertion names
`aggregate`, which excludes the timeout codes (`wall-time`, `process.wall-time`) that the previous
wildcard would have accepted.

**The dossier export assertion.** Reproduced on this host before the fix: the writer returned
`\\?\C:\Users\…\dossier.json` while the test compared against `C:\Users\…`. Both sides are now
canonical.

## Per-platform

| | Windows | macOS | Linux |
| --- | --- | --- | --- |
| Full `cargo test --workspace` | PASSED | NOT RUN | NOT RUN |
| `Native Check` skill-tool steps | PASSED | n/a — the steps are Windows-only | n/a |
| Playwright sidecar full path | PASSED | NOT RUN | NOT RUN |
| Playwright sidecar skip path | PASSED | NOT RUN | NOT RUN |

The `Rust` job on CI is the Linux gate for the cleanup floor and the sidecar test; the
`Native Check (windows-latest)` job is the gate for the skill-tool steps that went red on `main`.

## Not covered

- **Dependabot itself is not exercised.** The check verifies that each configured directory holds
  something the updater reads; whether GitHub's updater then opens a pull request is observable only
  after a scheduled run. The causal link between the old directory and the absence of cargo pull
  requests is strongly supported — npm pull requests exist, cargo ones never have, and the
  configuration predates the lockfile move — but it is an inference, not a reproduction.
- **Six ceilings still share one reason code.** `SkillToolInvocationBudget::reserve` answers
  `ResourceLimit("aggregate")` for host calls, output bytes, file bytes, network bytes, child
  processes and concurrency alike, so a caller cannot tell which ceiling refused it. The assertion
  tightened here is as specific as the production code permits. Widening the vocabulary is a
  user-visible reason-code change and belongs in its own proposal.
