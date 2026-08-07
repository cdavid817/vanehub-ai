## 1. Pin current behavior before changing it

- [ ] 1.1 Add a failing test asserting a timed-out command's descendant is terminated: spawn a fixture that launches a long-lived grandchild, let the command exceed its timeout, and assert the grandchild is no longer running
- [ ] 1.2 Add a test asserting a command that exits successfully while a descendant survives is reported as completed, with its output intact and the descendant left running
- [ ] 1.3 Confirm 1.1 fails and 1.2 passes against the current implementation, so the pair captures exactly what changes and what must not

## 2. Windows containment variant

- [ ] 2.1 Extract the job creation and child assignment shared by the existing `KillOnCloseJobObject` so a second wrapper can reuse it without duplicating the containment setup
- [ ] 2.2 Add the terminate-on-kill wrapper: assign to a job, omit `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, delegate `try_wait`/`wait` to the inner child, and terminate the job in `start_kill`
- [ ] 2.3 Reuse the existing containment-failure path so a child that cannot be contained is killed and reaped rather than left running uncontained
- [ ] 2.4 Add a Windows test asserting the new wrapper reports completion while a descendant survives — the deliberate inverse of `managed_child_wait_does_not_finish_while_a_descendant_survives`
- [ ] 2.5 Add a Windows test asserting dropping the new wrapper's handle after a successful run does NOT kill surviving descendants

## 3. Unix containment path

- [ ] 3.1 Verify by test whether `ProcessGroupChild::try_wait`'s inner-child fallback reports the direct child's exit while a group member survives
- [ ] 3.2 If 3.1 holds, wire `ProcessGroup::leader()` into the bounded execution path; if not, set the group via `CommandExt::process_group(0)` and signal the negated pgid on the kill path, keeping std's `try_wait`
- [ ] 3.3 Add a Unix test asserting a timed-out command's process-group descendants are terminated

## 4. Wire containment into the bounded execution path

- [ ] 4.1 Spawn through the containment wrapper in `output_with_control`, taking the stdout/stderr pipes from the wrapped child
- [ ] 4.2 Keep the completion decision on the directly launched process — do not gate the poll loop on the containment primitive
- [ ] 4.3 Route the timeout and cancellation paths through tree termination, leaving the returned error variants and collected output unchanged
- [ ] 4.4 Confirm the tests from 1.1 and 1.2 now both pass

## 5. Regression scope

- [ ] 5.1 Confirm `ManagedChild` / `ManagedTokioChild` semantics are untouched and their existing tree-wait and kill-on-close tests still pass
- [ ] 5.2 Confirm `spawn_detached` is unchanged, so deliberately detached launches (folder openers) still outlive the call
- [ ] 5.3 Exercise a package-manager-style command that leaves a background process and confirm it is still reported as successful rather than timing out

## 6. Verification

- [ ] 6.1 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [ ] 6.2 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [ ] 6.3 `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] 6.4 `npm run lint:ci`, `npm run test`, and `npm run build` to confirm no frontend impact
- [ ] 6.5 `openspec validate contain-timed-out-process-trees --strict`
- [ ] 6.6 `openspec validate --specs --strict`
