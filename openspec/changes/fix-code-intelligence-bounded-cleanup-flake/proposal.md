## Why

`code_intelligence::infrastructure::server_test_tests::initialize_timeout_forces_bounded_process_tree_cleanup_without_cancellation` fails under load and passes in isolation. That has now happened twice, which is what turns it from an anecdote into a defect worth its own change.

Recorded evidence, all on Windows:

| when | result | context |
| --- | --- | --- |
| full workspace run, `npx openspec` running concurrently | FAILED | first observation |
| 7 isolated re-runs immediately afterwards | 7 PASS | — |
| full workspace run, 490s against a usual ~343s | FAILED | second observation, alongside `local_runner_windows_spawn_cancel_benchmark` |
| 3 isolated re-runs immediately afterwards | 3 PASS | — |

Ten passes, two failures, and both failures are on the loaded runs. The test spawns a real `lsp-hang` fixture process and asserts that an initialize timeout of `Duration::from_secs(2)` produces `InitializeTimedOut` followed by a `ForcedTermination` cleanup. A two-second budget measured against real process startup is a budget that a busy machine can miss for reasons that have nothing to do with the code under test.

`local_runner_windows_spawn_cancel_benchmark_records_bounded_cleanup` failed once in the same loaded run and passed 3/3 isolated. It is listed here as a second data point rather than a second defect: one observation is not repeat evidence, and this change should not quietly widen to cover it without one.

The cost of leaving it is not the red build. It is that a suite with a known load-sensitive failure teaches everyone reading it to re-run rather than to look — and the next real regression arrives wearing the same clothes.

## What Changes

* **Establish what the test is actually asserting.** There are two candidates and they need separating before anything is edited: that cleanup is *bounded* (a property), or that cleanup completes *within two seconds of wall clock on this machine* (a measurement). Only the first is a contract.
* **Reproduce deliberately rather than by waiting.** Run the test under induced load until the failure is observable on demand. A defect that can only be seen by accident cannot be shown to be fixed.
* **Separate the timeout under test from the harness's patience.** The initialize timeout is the behaviour being verified; how long the test is willing to wait for the resulting cleanup to be observed is a different number, and conflating them is what makes a slow machine look like a broken one.
* **Keep the assertion about ordering and outcome**, which is the real contract: an initialize timeout must force bounded process-tree cleanup, report `InitializeTimedOut`, and report `ForcedTermination` — without cancellation.

Explicitly forbidden, and named because each would make the symptom disappear without touching the cause: fixed sleeps, test-level retry, `#[ignore]`, `--test-threads=1` as a fix rather than a diagnostic, and simply enlarging the two-second budget until the failures stop.

## Impact

* Affected specs: none expected — this is a test-harness defect, not a behaviour change. If investigation shows the *production* cleanup is genuinely unbounded under load, that becomes a spec-affecting change and this proposal is amended rather than quietly widened.
* Affected code: `src-tauri/src/contexts/code_intelligence/infrastructure/server_test_tests.rs`, and `server_test.rs` only if the evidence turns out to implicate it.
* Not blocked by, and does not block, `fix-portable-pty-bounded-termination` or `fix-private-relay-windows-acl-contract`. It is recorded separately precisely so none of the three borrows another's evidence.
