# Performance evidence

## Measurement scope

- Baseline source: `main` commit `15e5bf0` before this change.
- Optimized source: `feature/performance-optimization` in this change worktree.
- Host: Windows x86_64, Node/Vite production build, Rust stable MSVC toolchain.
- Timing values are diagnostic only because cold-cache and concurrent-host state were not controlled; artifact sizes and structural/query-plan checks are the regression evidence.

## Frontend startup closure

| Measure | Baseline | Optimized | Difference |
| --- | ---: | ---: | ---: |
| Main static JS closure, raw | 1,354.4 KiB | 428.1 KiB entry plus its static imports | Dynamic settings modules removed from the startup closure |
| Main static JS closure, gzip | 372.7 KiB | 123.2 KiB | -249.5 KiB (-66.9%) |
| Settings modules loaded before visiting settings pages | 12 page modules mounted | 1 default page module mounted | 11 eager page mounts removed |

The optimized value is emitted by `scripts/check-frontend-chunks.mjs` from the Vite manifest. The same checker enforces a 350 KiB gzip static-closure budget and a 700 KiB raw per-chunk budget. The optimized production build emitted all 14 settings pages as dynamic entries and passed both budgets.

## Historical-session search

Baseline profiling against a synthetic SQLite workload of 5,000 sessions and 50,000 messages observed an approximately 85.4 ms median for a 50-result message query and used a leading-wildcard message scan followed by per-result session/message loads. These timing figures are not used as a CI assertion.

The optimized repository uses:

- an FTS5 trigram virtual table for message-content substrings of at least three characters;
- migration-time backfill plus insert/update/delete synchronization triggers;
- a single bounded result statement that returns session rows and latest message context;
- a bounded compatibility scan only for two-character queries; and
- a 250 ms frontend debounce with one-character queries suppressed.

The native regression test verifies that SQLite reports a `VIRTUAL TABLE INDEX` plan and that insert, update, and delete mutations remain synchronized.

## Terminal retention

Before this change, every native and frontend output append rebuilt or shifted a retained string of up to 1,000,000 units. Both runtimes now retain 1 MiB in chunks, evict only the oldest necessary chunks, and join a snapshot only at the attach/remount boundary. Deterministic tests cover independent appends, UTF-8-safe trimming, and the hard byte bound.

## Native release profile

The baseline manifest explicitly used `opt-level = 0`, full debug information, and debug assertions in `[profile.release]`. The optimized profile uses `opt-level = 3`, ThinLTO, one codegen unit, and debuginfo stripping with debug assertions disabled by default. An architecture contract test parses the Cargo manifest to prevent regression.
