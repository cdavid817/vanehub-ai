## Context

VaneHub runs the same React application in Web/mock and Tauri modes, with SQLite and terminal processes owned by Rust. Profiling found four independent sources of avoidable work: a debug-oriented Cargo release profile, eager settings module mounting, wildcard/N+1 historical search, and full-string copies for every terminal output append. The changes cross the React shell, the service-backed session flow, SQLite migrations, and the native terminal runtime, so performance constraints must be enforced at those existing boundaries.

## Goals / Non-Goals

**Goals:**

- Reduce work performed before a user visits a settings page.
- Make message-content search index-backed for ordinary queries and eliminate per-result session loads.
- Make terminal transcript append cost proportional to new/evicted chunks rather than the retained transcript size.
- Restore optimized production compilation and add repeatable regression gates.
- Preserve Web/mock behavior, Tauri service boundaries, visited settings state, search result contracts, and terminal replay semantics.

**Non-Goals:**

- Adding Redis, a network cache, or a general-purpose Rust cache.
- Replacing SQLite or changing public frontend service interfaces.
- Redesigning the settings, search, or terminal user experience.
- Claiming an end-to-end latency percentage without comparable production workloads.

## Decisions

### Use first-visit module loading for every settings page

The settings registry will expose dynamic loaders for all page modules. The shell will start with only the default page visited, add a page on navigation, and keep every visited page mounted and hidden when inactive. This prevents unvisited pages from importing or starting service queries while preserving the existing stateful-mount contract. A settings-specific error boundary and loading fallback remain scoped to the content region.

Alternatives considered: unmounting inactive pages reduces memory further but violates state preservation; lazily loading only the two largest pages leaves most service effects eager.

### Debounce in React and index persisted message text in SQLite

React will submit a trimmed query only after a 250 ms quiet period and only for at least two characters. The Rust repository will continue bounded results, use metadata `LIKE` matching, use an FTS5 trigram index for message substrings of at least three characters, and use a short-query fallback for compatibility. The result query will select session and match context in one statement instead of loading each result separately.

A forward-only migration will create an external-content FTS5 table, backfill existing messages, and maintain it with insert/update/delete triggers. This follows the FTS5/trigram pattern already used for terminal-output search and adds no service-interface change.

Alternatives considered: Redis adds deployment and invalidation costs without addressing local SQL shape; a Rust result cache risks stale session/message data; a B-tree cannot accelerate leading-wildcard message searches.

### Store retained terminal output as bounded chunks

Rust will use a small `VecDeque<String>` abstraction capped at 1 MiB. Appends add a chunk and evict or trim only front chunks; a contiguous snapshot is created only when attach/replay requires one. The frontend replay cache will mirror the chunked representation and join chunks only when painting a remounted terminal.

Alternatives considered: continuing to truncate one `String` performs repeated memmoves; persisting transcripts to SQLite would change the explicit transcript persistence boundary.

### Enforce deterministic structural and artifact budgets

Tests will verify release-profile values, first-visit settings mounting, indexed search behavior, and transcript bounds. The frontend chunk checker will calculate the static entry closure from the Vite manifest and reject a gzip total above 350 KiB or any JavaScript chunk above 700 KiB raw. These budgets are above the optimized measured result and low enough to catch reintroducing the profiled eager paths.

Timing assertions are intentionally excluded from normal unit tests because shared CI hosts make small wall-clock thresholds flaky.

### Keep runtime boundaries unchanged

React continues to call the session service abstraction. Tauri adapter signatures and Web/mock adapter signatures do not change. SQLite indexing and query planning remain in Rust; browser mode continues to search its in-memory mock data with the same bounded service result shape.

## Risks / Trade-offs

- [FTS5 index increases database size and write work] → Limit the index to message content and maintain it transactionally with triggers.
- [Trigram MATCH does not support queries shorter than three characters] → Use a bounded SQL fallback for two-character queries and suppress one-character requests in React.
- [Joining frontend transcript chunks still allocates during remount] → Snapshot only on remount/attach, not on every output event, and retain the existing 1 MiB cap.
- [Dynamic settings imports can expose loading failures] → Keep localized loading/error boundaries scoped to the active content page and support retry.
- [Bundle budgets may require deliberate updates after justified features] → Report the exact measured artifact and keep the threshold in the versioned checker.

## Migration Plan

1. Add and strictly validate this proposal, design, delta specs, and tasks.
2. Restore the Cargo release profile and add its contract test.
3. Convert settings registration/mounting and add first-visit tests.
4. Add the SQLite FTS migration, repository query, debounce, and search tests.
5. Introduce native and frontend bounded transcript buffers with boundary tests.
6. Extend frontend artifact checks and run all project validation commands.

Rollback can revert application code and leave the additive FTS table/triggers in existing databases; older binaries ignore them. A later migration may remove the index if storage recovery becomes necessary.

## Open Questions

None for this phase. Broader runtime caching will be reconsidered only after production traces identify stable, repeatedly computed hot data with an explicit invalidation contract.
