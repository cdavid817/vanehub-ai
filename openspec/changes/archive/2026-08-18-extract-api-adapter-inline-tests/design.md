## Context

See proposal.md — Why for the measurements and motivation.

Four facts about the file constrain the move:

- The inline `mod tests` at line 6,156 opens with `use super::*;`. A child module keeps access to the parent's private items, so the glob import continues to resolve after the move without widening any visibility.
- There are **zero** `#[cfg(not(test))]` items in the file. Every one of the 19 `#[cfg(test)]` items is test-only scaffolding with no production counterpart, so none of them is half of a test/production alternate pair that would have to stay behind.
- Four of those items are `#[cfg(test)]` methods declared inside production `impl` blocks (`execute_tool_call`, `execute_tool_call_with_code_intelligence`, `execute_tool_call_with_workspace_mutations`, `execute_tool_call_with_skills`). Rust allows an inherent `impl` in any module of the defining crate, so they can move into the tests module as a separate `impl` block.
- The file is referenced by name from `src-tauri/tests/architecture.rs:1283`, which reads it to enforce a composition-root rule.

## Goals / Non-Goals

**Goals:**

- Remove test code from the production compilation surface of the largest file in the repository.
- Leave every external `use` path byte-identical.
- Keep the diff mechanically reviewable: a reviewer should be able to confirm "nothing but moves" without reading 7,772 lines of test bodies.

**Non-Goals:**

- Splitting the production half into responsibility modules. That is the follow-up change; conflating the two would destroy the "pure move" property that makes this diff reviewable.
- Rewriting, deduplicating, renaming, or re-grouping any test.
- Changing any production item, including the ones the moved tests exercise.

## Decisions

### Keep `api_process_adapter.rs` and add a child module directory

Rust 2018 allows a module declared in `foo.rs` to have submodule files under a sibling `foo/` directory. So `api_process_adapter.rs` stays exactly where it is, gains one `#[cfg(test)] mod tests;` line, and the body lands in `api_process_adapter/tests.rs`.

*Alternative rejected — convert to `api_process_adapter/mod.rs`*: this renames the file, which turns the whole production half into a rename+edit in the diff, breaks `git log --follow` for reviewers, and forces a same-commit update of the `architecture.rs:1283` reference and the recorded path budget key. The follow-up module split will pay that cost once, for a reason; this change should not pay it for none.

### Move the test-only scaffolding too, not just `mod tests`

A `mod tests` move alone leaves 19 `#[cfg(test)]` items in the production half. They are already invisible to production builds, so leaving them costs no compile time — but it does leave a reader of the production file stepping over port doubles and test constants, and it leaves the next person unsure where test scaffolding belongs. Moving them is the same mechanical operation and finishes the separation.

### Verify the move by test-name inventory, not by diff reading

A "pure move" claim is only as good as its evidence. The check is: capture the list of test names `cargo test --lib` reports for this module before and after, and require the two lists to be identical. That catches an accidentally dropped, renamed, or duplicated test in a way that reading a 7,772-line diff does not.

*Alternative rejected — trusting the test count*: a dropped test plus an accidentally duplicated one nets to zero. Compare names, not counts.

### The subtree budget will need a small explicit raise

Moving the tests keeps the `agent_runtime/infrastructure` aggregate essentially unchanged, but the split adds a `mod` declaration and re-imports in the new file, so the aggregate rises by a small amount. Per `freeze-large-file-line-budgets`, this change raises the subtree budget by the measured delta and states the reason. If the delta is larger than the module boilerplate can account for, that is the signal the move was not pure, and the cause must be found rather than the budget widened.

## Risks / Trade-offs

- **The move silently drops or duplicates a test** → The before/after test-name inventory is the specific control for this, and it is a task, not an afterthought.
- **A moved test needs a visibility widening to compile** → It should not: a child module sees the parent's private items. If a case does need `pub(super)`, that is a signal the item was reached through some other path, and it must be recorded and reviewed rather than waved through. No production item may become more visible than `pub(super)`.
- **The follow-up module split will re-touch this file anyway** → True, and that is the point of doing this first: the follow-up gets to reason about ~6,100 lines of production code instead of 13,927, and its diff will not be buried under moved test bodies.
- **Merge conflicts with concurrent work on this file** → This lane owns the file for its duration. The other two lanes in this batch touch `src-tauri/src/platform/database/` and `src/services/` and do not overlap.

## Migration Plan

No deployment or data migration. The change is compile-time reorganization; a revert is a plain `git revert`.
