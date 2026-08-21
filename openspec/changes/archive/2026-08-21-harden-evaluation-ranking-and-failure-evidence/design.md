## Context

See `proposal.md` — Why for the two defects and their evidence.

Two facts about the current code shape the approach.

**Ranking is a pure function of stored data.** `compare_aggregates` reads only `outcome`, `checks` and `metrics`, all of which are persisted per attempt (`evaluation_attempts.safe_snapshot_json`, plus the `evaluation_metrics` and `evaluation_verifications` tables). Nothing about ranking depends on run-time state, so it can be derived on read without a stored rank column and without a schema migration.

**A related read-side fix has already landed.** `EvaluationApi::get`/`list` now apply `ranked()` (attempt id as final tiebreak) because the repository returns attempts `ORDER BY attempt_id` and the sort computed at the end of `execute` was discarded. That work makes the "Arena is read back after it finished" scenario already true; this change alters *what* `ranked` orders by, not *where* it is applied.

`EVALUATION_RANKING_VERSION` is a compile-time constant copied into every arena at creation and stored in `evaluation_arenas.ranking_version`, so historical rows already carry the version they were produced under.

## Goals / Non-Goals

**Goals:**

- One comparison function, used by both the native engine and the Web/mock adapter's fixture data, whose tiering is legible from the enum rather than from a boolean plus an incidental empty-list count.
- Dispatch failure reasons reach the user through the same bounded, redacted path every other evaluation field already uses.
- Arenas stored under `deterministic-v1` stay readable and keep reporting v1.

**Non-Goals:**

- Re-ranking or migrating historical arenas. A v1 arena keeps its v1 ordering semantics; this change does not backfill.
- Adding a rank column, a rank field on the attempt, or any schema migration.
- Distinguishing *between* non-completion outcomes (timeout vs. stuck vs. Agent failure). They share one tier; ordering within it stays on the existing metric comparisons.
- Surfacing raw provider or process errors. The diagnostic is a redacted summary, not a log excerpt.

## Decisions

**Graded outcome tier instead of a binary success rank.** Replace `success_rank(&EvaluationOutcome) -> u8` (currently `u8::from(matches!(_, Succeeded))`) with a tier function returning three ranks: success, deterministic task failure, non-completion. Ordering stays `tier`, then `failed_checks`, then `interventions`, then `tool_calls`.

*Alternative considered — keep the binary rank and make `failed_checks` return `usize::MAX` for an empty check list on a non-completion outcome.* Rejected: it encodes the tier inside a count, so a future attempt that legitimately has zero checks (a task with no verifier profiles) would be punished for it. The defect is about outcome class, and the fix should say so.

*Alternative considered — synthesize a failing check for every non-completion outcome so `failed_checks` separates them naturally.* Rejected for outcomes other than dispatch failure: it would put fabricated verification rows in `evaluation_verifications` for events that never ran a verifier, which the "Deterministic verification is authoritative" requirement reads as acceptance evidence.

**Version bump to `deterministic-v2`, no dual-algorithm support.** The constant moves; new arenas are stamped v2; stored v1 arenas keep their string. The read-side ranking applies the current algorithm to whatever it reads, so a v1 arena re-read after this change *is* ordered by v2 rules while reporting `deterministic-v1`.

That inconsistency is accepted deliberately, and it is bounded: the only rows it can affect are arenas that already finished, whose ordering is not load-bearing for any decision the user has yet to make, and whose export still names v1 so a consumer can tell which rules the *recorded* result was produced under. The alternative — keeping a v1 comparator alive and dispatching on the stored string — buys correctness for historical display at the cost of two ranking implementations that must both stay tested forever, for a product whose evaluation history is local, small, and disposable. Revisit if arenas ever become shareable between machines.

**Dispatch error becomes a check, not a new field.** `EvaluationApi::execute` currently drops the `Err(String)` from `NativeEvaluationAgentAdapter::dispatch`. Route it into the aggregate as a failed `EvaluationCheck` with a stable `check_id` (`agent-dispatch`), so it persists, exports, and renders in the detail pane through paths that already exist.

**Exact-match safe reasons, not the substring allowlist.** Corrected during implementation. The plan was to reuse `mapper::safe_error`, whose allowlist passes any message *containing* `not found`, `requires`, `unavailable`, `unsupported`, `invalid` or `unknown`. Both reasons an ordinary host actually hits — `evaluation Agent is not installed and available` and `evaluation supports OnePiece or an available managed CLI Agent` — contain none of those markers (`available` is not `unavailable`), so every useful diagnostic would have collapsed to the generic sentence and the change would have delivered a differently-worded empty panel.

Widening the substring list was rejected: an allowlist relaxed by marker is how a redaction boundary leaks, because the next error that happens to contain the marker carries a path or a token out with it. Instead the domain names the dispatch failures it writes about its own preconditions (`SAFE_DISPATCH_REASONS`), the dispatch gate returns those constants rather than free-form literals, and `safe_dispatch_diagnostic` passes a reason through only on **equality**. That is strictly tighter than the substring rule — a message that merely contains a safe reason is redacted — while being the thing that makes recording a diagnostic worth doing at all. `mapper::safe_error` keeps its existing behavior for command errors and now delegates to the domain so both paths share one generic sentence.

*Alternative considered — an `error_summary: Option<String>` on the attempt.* Rejected: a new field needs new persistence, a new DTO field, new redaction, and new UI, to carry something the check list already models.

**The diagnostic must not re-create the defect being fixed.** A dispatch-failed attempt now has one failed check where a timed-out attempt still has none — under `failed_checks` alone the timeout would rank ahead.

Corrected during implementation: the outcome tier alone does *not* prevent this, because both attempts sit in the same non-completion tier and the failed-checks key runs inside it. `failed_checks` therefore returns 0 for any non-completion outcome — an attempt that never reached verification has no acceptance evidence to count, and the only checks it can carry are diagnostics. This is the same "absence beats evidence" inversion as the tier fix, one level down, and it is why the two halves still ship together.

## Risks / Trade-offs

- **A v1 arena re-read after the bump is ordered by v2 rules while reporting v1** → Bounded and accepted; see the version decision above. The export still names the version the result was recorded under.
- **`safe_error` is an allowlist of substrings (`not found`, `requires`, `unavailable`, …) and falls back to a generic sentence** → Some dispatch failures will render as the generic message rather than a specific reason. That is strictly better than today's empty panel, and widening the allowlist is a separate, reviewable change.
- **The Web/mock adapter can drift from the native tiering again** → The `contracts:check` conformance suite and the desktop specs both assert on the same shapes; the tasks add a mock arena whose fixture exercises the tier boundary so a drift fails a test rather than a user's reading of a table.
- **Ranking is recomputed on every read** → Bounded by `MAX_ARENA_ATTEMPTS` (8) per arena and a 100-arena page; a sort over 8 items per arena is not measurable against the SQLite reads that precede it.
