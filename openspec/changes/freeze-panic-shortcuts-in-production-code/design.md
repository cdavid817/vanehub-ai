## Context

See proposal.md — Why for the measurement that rules out the ticket's approach.

The constraint that decides the whole design: CI already runs `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`. Any lint set above `allow` in `Cargo.toml` therefore applies to test targets too, and these two lints have 9,560 test-code hits.

## Goals / Non-Goals

**Goals:**

- Make AGENTS.md's rule about panic shortcuts mechanically true for production code.
- Require no exemption in test code, now or for any test written later.
- Leave the 35 existing violations visible and individually attributable rather than absorbed into a blanket allowance.

**Non-Goals:**

- Fixing the 35 violations. That is error-handling work across five bounded contexts and belongs in its own change, which the whitelist entries name.
- Enforcing anything in test code. The project's position is that panic shortcuts are correct there, and `eslint.config.js` takes the same line about test-file size.
- Adding `[lints]` to `Cargo.toml`. See below.

## Decisions

### Enforce with a target-scoped clippy invocation, not a `Cargo.toml` lint level

`[lints.clippy]` has no target selectivity. Setting `unwrap_used` to `warn` or `deny` makes it apply everywhere, and `--all-targets -- -D warnings` promotes it to an error in tests.

A separate invocation scoped to `--lib --bins` with `-D clippy::unwrap_used -D clippy::expect_used` expresses exactly the intended rule and nothing more. It costs one CI step.

*Alternative rejected — `Cargo.toml` lints plus per-test-module `#![allow(...)]`*: the ticket's plan. It requires exemptions in several hundred test modules, and, worse, in every test module written afterwards. A rule whose cost falls on people who are not violating it will be worked around.

*Alternative rejected — `cfg_attr(test, allow(...))` at crate root*: `#![cfg_attr(test, allow(clippy::unwrap_used))]` only exempts the `test` cfg for the crate's own unit-test build. It does not cover integration test targets under `src-tauri/tests/`, which `--all-targets` also builds, and it re-introduces the crate-wide blanket this change is trying to avoid.

### The whitelist is file-level, and each entry names its retirement

Each of the 11 files gets `#![allow(clippy::unwrap_used, clippy::expect_used)]` with a comment recording the count and the change expected to remove it. File-level rather than line-level because 12 of the 35 are in one file and line-level noise would obscure the code; file-level rather than crate-level because the point is that the list is short, enumerable, and shrinking.

The entries are debt markers with the same contract as the line budgets from `freeze-large-file-line-budgets`: removing one needs no ceremony, adding one requires justification.

### Two of the eleven files are `domain` layer, and that is worth saying out loud

`retrieval/domain/code_redaction.rs` and `task_orchestration/domain/graph.rs` hold nine of the 35. A panic shortcut in a domain layer is the least defensible placement in this codebase's own architecture, since domain code is meant to be pure and independently testable. The whitelist comments say so, so the follow-up change has an obvious starting order.

## Risks / Trade-offs

- **A contributor adds a panic shortcut inside an existing whitelisted file and the gate stays green** → True, and accepted for now. The alternative — line-level allows — makes 35 sites into 35 scattered annotations. The follow-up change removes the files from the list one at a time, and the list is short enough to finish.
- **The gate runs clippy a second time and lengthens CI** → It reuses the same build cache as the existing clippy step in the same job, so the marginal cost is analysis of two targets, not a rebuild.
- **Someone later adds `[lints.clippy]` to `Cargo.toml` and breaks the test build** → The proposal and the whitelist comments both record why it is deliberately absent.
- **`--lib --bins` misses a production target added later** — for example a new binary or a build script → Build scripts are not covered by either selector. This is recorded as a known gap rather than papered over; adding a target means revisiting the gate's selector.

## Migration Plan

No deployment step. The gate is additive: it fails only on new production panic shortcuts. Reverting is removing the CI step, the npm script, and eleven attribute lines.
