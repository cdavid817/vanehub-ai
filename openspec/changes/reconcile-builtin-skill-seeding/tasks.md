## 1. Reproduce before changing anything

- [x] 1.1 Write a failing test that puts a built-in's source directory on disk with no registry
      record, runs `ensure_builtins`, and asserts the Skill ends up registered — it should fail
      today with `Conflict`
- [x] 1.2 Extend it to assert the other five built-ins still register when one is unusable, which
      is the property that turned this installation's registry into zero rows rather than five
- [x] 1.3 Capture the current real-world state as a fixture: six source directories present, zero
      registry rows, no deletion tombstones

## 2. Let the filesystem layer answer "is it already there?"

- [x] 2.1 Give `create_document` a way to distinguish "a directory is already present" from a real
      conflict, so the application layer can decide, instead of only ever receiving `Conflict`
- [x] 2.2 Add a read path that loads an existing `SKILL.md` and reports whether it is readable and
      parseable, without modifying it
- [x] 2.3 Keep the existing conflict behavior for user-create and import: those callers asked to
      create something new, and adopting a stranger's directory would be wrong

## 3. Reconcile in `ensure_builtins`

- [x] 3.1 For a built-in with no record, branch on disk state: create when absent, adopt when a
      readable source is present, report a per-Skill failure when the source is unusable
- [x] 3.2 Register the adopted content as it exists on disk; do not rewrite the file
- [x] 3.3 Confirm the resulting record's content hash reflects disk, so `MetadataChanged` drift
      reports a divergence from the shipped definition rather than the record claiming to be pristine
- [x] 3.4 Keep intentional deletions winning: a tombstoned built-in stays unregistered even with a
      source present
- [x] 3.5 Reconcile and report each built-in independently so one failure cannot discard the others

## 4. Make `UnregisteredSource` repairable

- [x] 4.1 Replace the no-op arm in the synchronization path with the same adoption logic
- [x] 4.2 Clear the drift issue once the source is registered
- [x] 4.3 Report a failed adoption with its reason rather than leaving an issue no action can clear

## 5. Fix the diagnostics

- [x] 5.1 Stop emitting `error` level for an already-present built-in
- [x] 5.2 Attribute the remaining diagnostic to the operation that produced it — today a
      filesystem-layer `Conflict` is logged under `skill.seed-builtins`, which points an
      investigation at the wrong file
- [x] 5.3 Log an adoption at info level, naming which Skills were adopted

## 6. Verification

- [x] 6.1 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 6.2 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 6.3 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 6.4 `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 6.5 `npm run lint:ci`
- [x] 6.6 `npm run test`
- [x] 6.7 `npm run build`
- [x] 6.8 `openspec validate --specs --strict` and
      `openspec validate reconcile-builtin-skill-seeding --strict`
- [x] 6.9 `npx playwright test` with `PLAYWRIGHT_PORT` pinned to a free port — the config defaults
      to 5174 with `reuseExistingServer: true`, and another worktree's dev server there would
      silently test that checkout instead
- [ ] 6.10 Launch the desktop app on the affected installation and confirm the Skill management page
      lists the six built-ins, that the four per-start `error` lines are gone, and that a Skill can
      be bound to an Agent
- [ ] 6.11 Confirm recovery needed no user action — the registry populated on the first start after
      the fix

> 6.10 and 6.11 remain open. This branch cannot boot the affected installation's database for an
> unrelated reason: version 49 is recorded there as `workspace-code-index-foundation` from another
> branch, so this branch's `plan-execution-foundation` is skipped and startup panics on a missing
> `plan_runs` table. A clean-boot launch from this branch migrates and starts normally, and the
> recovery itself is covered on the production stack by
> `skills::infrastructure::recovery_tests` — a migrated SQLite database plus the real
> `ManagedSkillFilesystem`, diverged the same way the installation is and recovering on the next
> listing. What is still unverified by hand is the Skill management page rendering and binding a
> Skill to an Agent through the UI.
