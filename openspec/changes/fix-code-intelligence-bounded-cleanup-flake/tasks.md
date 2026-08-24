# Tasks

## 1. Establish what is being asserted

- [ ] 1.1 Read `initialize_timeout_forces_bounded_process_tree_cleanup_without_cancellation` and separate its two candidate claims: that cleanup is *bounded* (a property, and the contract), versus that cleanup completes within two seconds of wall clock on this machine (a measurement, and not one). Nothing is edited before this is written down.
- [ ] 1.2 Identify every wall-clock number the test depends on, including the `Duration::from_secs(2)` initialize timeout and any implicit patience in the harness around it.
- [ ] 1.3 Decide which of those is the behaviour under test and which is the harness waiting. Conflating them is what makes a slow machine look like a broken one.

## 2. Reproduce on demand

- [ ] 2.1 Reproduce the failure under induced load rather than by waiting for a busy run. A defect that can only be observed by accident cannot be shown to have been fixed.
- [ ] 2.2 Record the reproduction recipe in the change, so a reviewer can see it fail before seeing it pass.
- [ ] 2.3 Confirm the same recipe leaves the rest of the suite unaffected — if induced load breaks twenty tests, the recipe is measuring the machine, not this test.

## 3. Repair

- [ ] 3.1 Keep the contract assertions exactly as they are: an initialize timeout forces bounded process-tree cleanup, reports `InitializeTimedOut`, then reports `ForcedTermination`, without cancellation.
- [ ] 3.2 Make the test's own patience independent of the timeout it is verifying.
- [ ] 3.3 If the evidence implicates production cleanup rather than the harness, stop and amend the proposal: that is a behaviour change with a spec impact, not a test fix, and it must not arrive disguised as one.

## 4. Verification

- [ ] 4.1 The reproduction recipe no longer fails, run repeatedly.
- [ ] 4.2 `cargo test --workspace` under the same induced load.
- [ ] 4.3 `cargo test --workspace`: Windows, Linux, macOS.
- [ ] 4.4 `clippy`, `fmt`, `openspec validate --strict`.

## Forbidden

- [ ] X.1 No fixed sleep.
- [ ] X.2 No test-level retry.
- [ ] X.3 No `#[ignore]` and no skip.
- [ ] X.4 No `--test-threads=1` as a fix. It is a legitimate diagnostic and an illegitimate repair: it hides the interaction rather than removing it.
- [ ] X.5 No enlarging the two-second budget until the failures stop. That converts an unproven bound into a longer unproven bound.
- [ ] X.6 No changes to `fix-portable-pty-bounded-termination` or `fix-private-relay-windows-acl-contract` from this change, and none of the three may borrow another's evidence.

## Out of scope

- [ ] Y.1 `local_runner_windows_spawn_cancel_benchmark_records_bounded_cleanup` failed once in the same loaded run and passed 3/3 isolated. One observation is not repeat evidence. It is recorded here as a second data point, and this change does not widen to cover it without one.

## Evidence

| when | result | context |
| --- | --- | --- |
| full workspace run, `npx openspec` concurrent | FAILED | first observation |
| 7 isolated re-runs | 7 PASS | — |
| full workspace run, 490s against a usual ~343s | FAILED | second observation |
| 3 isolated re-runs | 3 PASS | — |

Ten passes, two failures, both failures on loaded runs. All on Windows. The test spawns a real `lsp-hang` fixture process and asserts against a two-second initialize timeout.

## Status

- Implementation: **NOT STARTED**
- Windows / Linux / macOS: **NOT RUN**
- Archive: **BLOCKED**
