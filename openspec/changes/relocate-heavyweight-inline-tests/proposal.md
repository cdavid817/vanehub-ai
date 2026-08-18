## Why

The optimization ticket that requested this work asked for "heavyweight inline test modules" to be relocated out of the library source tree into `src-tauri/tests/`, justified by inline tests being "compiled into the lib on every build, slowing `cargo check`". Every load-bearing part of that framing is wrong, and this change corrects the record rather than inheriting it.

**The two files are not inline test modules.** Both are already `#[cfg(test)] mod tests;` sibling files — `sessions/infrastructure/mod.rs:27` and `agent_runtime/application/mod.rs:240` declare them. The extraction that `extract-api-adapter-inline-tests` performed for `api_process_adapter.rs` is already done here. There is no inline `mod tests` block to move.

**There is no compile-time win to claim.** `cargo check` cfg-strips test code after parsing, so any saving is parse-only. `extract-api-adapter-inline-tests` measured this directly on a 7,772-line extraction: four "after" runs at 56.7/54.7/67.5/65.6s against three "before" runs at 71.9/67.1/66.4s, with an A-B-A bracket putting a third "after" run at 67.5s — inside the "before" range. Within-condition spread exceeded the between-condition difference. This change promises no compile-time improvement.

**`src-tauri/tests/` is unreachable for these tests — not for most of them, for all 131.** A file in `src-tauri/tests/` is a separate crate that sees only the library's public API. That API is one item:

```rust
// src-tauri/src/lib.rs
mod bootstrap;   mod commands;   mod contexts;   mod platform;   // all private
pub fn run() { ... }
```

Three independent blocks, any one of which is sufficient:

1. `mod contexts;` is private at the crate root, so nothing beneath it is nameable from another crate. `contexts/mod.rs` declares `pub(crate) mod sessions;` and `pub(crate) mod agent_runtime;` — `pub(crate)` is invisible outside the crate regardless.
2. `mod test_support;` is `#[cfg(test)]`. An external test crate links the library built *without* `--cfg test`, so `TempDirectory`, `FixedClock`, and `SequenceIdGenerator` do not exist in the artifact it links against. `sessions/infrastructure/tests.rs` uses `crate::test_support::TempDirectory` in every fixture.
3. `sessions/infrastructure/tests.rs` imports `allocate_message_sequences`, `indexed_search_statement`, and `compatibility_search_statement` from `super::sqlite_repository`. All three are `pub(super)` (`sqlite_repository.rs:1036`, `:1189`, `:1225`) — visible only inside `infrastructure`. No location in the crate other than a child of `infrastructure` can see them, let alone another crate.

Reaching `src-tauri/tests/` would require making `contexts`, `platform`, and a large transitive closure beneath them `pub`. That inverts `openspec/project.md` — "Modules are private by default. Use the narrowest practical visibility (`pub(super)` or `pub(crate)`); public context access goes through `api`" — and would publish the entire native internals as crate API to relocate test files. Confirming this: no file in `src-tauri/tests/` references `vanehub_ai_lib` at all. `architecture.rs` parses source as text; the MCP tests drive subprocesses. The directory has never hosted a test of library internals, because the library exposes nothing to test.

The budget table's `owner: "relocate-heavyweight-inline-tests"` is not independent corroboration. `freeze-large-file-line-budgets` design.md:104 recorded it while assuming this relocation was possible ("That is exactly what the planned relocation of heavyweight inline tests to `src-tauri/tests/` does"). It is the same unverified premise written down twice.

### What is actually wrong with these files

With the ticket's rationale gone, the measured case that survives is cohesion and navigability:

| File | Lines | Tests | Subject modules | Largest module it tests |
|---|---:|---:|---:|---:|
| `sessions/infrastructure/tests.rs` | 5,110 | 64 | 13 | `sqlite_repository.rs` (1,278) |
| `agent_runtime/application/tests.rs` | 4,628 | 67 | 8+ | `service.rs` (4,337) |

`sessions/infrastructure/tests.rs` is 42% of its subtree (5,110 of 12,013) and four times the size of the largest production module it covers. Neither file is "one subject's tests grown large" — each is an undifferentiated bucket spanning many subjects, holding groups with no relationship to one another. In the sessions file, FTS ranking tests and usage-ledger projection tests share nothing but a filename.

The neighbouring directory already shows the intended shape. `agent_runtime/application/` contains **16** per-subject `*_tests.rs` files totalling 5,337 lines — an average of 334 lines each, each named after the module it exercises (`loop_control_tests.rs`, `runner_tests.rs`, `seat_turn_tests.rs`). `tests.rs` at 4,628 lines is the holdout that predates that convention, not an expression of it.

This distinction matters because the repository has a written position against splitting test files: `eslint.config.js:59-61` exempts test files from `max-lines` because "用例行数随覆盖线性增长,硬拆损害用例内聚" (test line counts grow linearly with coverage; hard-splitting harms case cohesion). That reasoning protects a `.test.ts` growing alongside its one subject. It does not sanction a 5,000-line bucket spanning thirteen subjects — splitting *that* by subject increases cohesion rather than destroying it. This change therefore splits along subject seams only, and never splits a group of tests that share a subject.

**Scope honesty:** this is a modest reviewability improvement, not the compile-time or architectural win the ticket described. It is proposed on that reduced basis.

## What Changes

- Split `src-tauri/src/contexts/sessions/infrastructure/tests.rs` into subject-named child modules under `sessions/infrastructure/tests/`, keeping `tests.rs` as the coordinator that declares them and holds the fixtures shared across groups.
- Split `src-tauri/src/contexts/agent_runtime/application/tests.rs` the same way, under `agent_runtime/application/tests/`. The ~1,600-line `FakeWorld` port double stays in the parent, where every child module continues to see it.
- Every test body, fixture, and helper moves verbatim. **No production item is touched, and no item's visibility is widened** — a child module already sees all of its ancestors' private items, which is the same property `extract-api-adapter-inline-tests` relied on.
- Lower the `ARCH-NATIVE-006` path budgets for both files to their post-split measurements.
- **Explicitly out of scope: `src-tauri/src/contexts/tooling/skills/application/tests.rs` (4,049 lines).** It collides with three in-flight changes — `add-skill-configuration-management` (31/53 tasks), `add-delegated-utility-skills` (6/89), and `expand-builtin-skill-catalog` (0/84) — all of which add Skill tests to this file. Splitting it now would create merge conflicts across three lanes for a reviewability benefit. Its budget entry stays as recorded, and its owner should be reassigned once those lanes land.
- **No relocation to `src-tauri/tests/`**, for the reasons measured above. That goal is withdrawn, not deferred.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This is a pure test-code reorganization with no externally observable behavior change: no Tauri command, SQLite schema, adapter contract, domain rule, or runtime behavior is affected in either the desktop or Web runtime. The set of tests that run is unchanged, proven by byte-identical before/after test-name inventories. The change sets `skip_specs: true`.

## Impact

- `src-tauri/src/contexts/sessions/infrastructure/tests.rs` — shrinks to a coordinator holding shared fixtures; gains `mod` declarations.
- `src-tauri/src/contexts/sessions/infrastructure/tests/` — new subject-named test modules.
- `src-tauri/src/contexts/agent_runtime/application/tests.rs` — same treatment.
- `src-tauri/src/contexts/agent_runtime/application/tests/` — new subject-named test modules.
- `src-tauri/tests/architecture.rs` — two path budgets lowered. Neither subtree has a registered subtree budget, so no subtree ceiling is affected.
- No production Rust file, frontend file, or Tauri command is touched. No frontend/backend isolation or runtime adapter boundary is affected.
