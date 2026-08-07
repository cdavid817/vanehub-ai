## Context

`SqliteSessionRepository::search` builds one statement from a shared skeleton, swapping only a `message_source` fragment between two branches:

- **≥3 characters**: `FROM session_message_fts JOIN messages ON messages.rowid = session_message_fts.rowid WHERE session_message_fts MATCH ?1`
- **<3 characters**: `FROM messages WHERE messages.content LIKE ?1 ESCAPE '\'`

Both feed the same CTE, which ranks matches with `ROW_NUMBER() OVER (PARTITION BY session_id ORDER BY created_at DESC, rowid DESC)` and keeps `match_rank = 1`, then `LEFT JOIN`s that onto `sessions` and applies `ORDER BY sessions.updated_at DESC LIMIT ?3`.

The branch threshold is forced by the schema. `session_message_fts` is declared `tokenize='trigram'`, and the trigram tokenizer emits no tokens for inputs shorter than three characters, so a two-character `MATCH` cannot match anything. This was verified rather than assumed: two-character `MATCH`, quoted-prefix `"xx"*`, and bare `xx*` each return zero rows, for CJK and Latin input alike. The `LIKE` fallback is therefore required for correctness, not a shortcut.

What is *not* forced is the cost. The window function must see every match before ranking, so the outer `LIMIT` cannot prune it. On the `LIKE` branch those matches come from an unindexed full scan, so a short query pays a full table scan plus a full sort of everything it matched.

## Goals / Non-Goals

**Goals:**

- Make short-query search cost proportional to the number of sessions returned rather than to the number of matching messages in the database.
- Keep results byte-identical: same sessions, same order, same match context.
- Keep search a single result query, with no per-result round trip from Rust.

**Non-Goals:**

- Changing the three-character threshold. It is a property of the trigram tokenizer, and lowering it would silently return nothing.
- Replacing the tokenizer or adding a CJK-aware one. SQLite ships no bigram tokenizer, and a custom one is a C extension — out of scope and against the no-new-dependency constraint.
- Making two-character search index-backed. With stock FTS5 that is not achievable; this change makes the unavoidable scan cheap to consume, it does not eliminate it.
- Touching the FTS branch's query shape.

## Decisions

### Restructure only the short-query branch

The `LIKE` branch becomes a correlated form:

- membership via `EXISTS (SELECT 1 FROM messages m WHERE m.session_id = sessions.id AND m.content LIKE ?1 ESCAPE '\')`
- match context via a correlated scalar subquery selecting the newest matching message id for that session, joined back to `messages` on its primary key to retrieve content

Both correlated lookups drive `idx_messages_session_created`, which already exists. The measured plan confirms `SEARCH m USING INDEX idx_messages_session_created (session_id=?)` in place of `MATERIALIZE ranked` + `SCAN messages`.

Alternative considered: keep the window function but add an inner `LIMIT`. Rejected — it would silently drop older matches for sessions beyond the cut, changing which sessions appear.

Alternative considered: pre-filter the CTE to sessions matching on metadata. Rejected — message-only matches are the common case, so the filter would exclude the results users are searching for.

### Leave the FTS branch alone

Correlating an FTS `MATCH` per candidate session would execute the full-text query once per session. The FTS index already prunes to matching rows before ranking, so materializing that set is the cheaper shape. The two branches therefore stop sharing one skeleton and become two complete statements.

This trades a little duplication for each branch being optimal and independently readable. The shared `SESSION_SEARCH_SELECT` column list, session-metadata predicates, ordering, and limit stay common.

### Equivalence is a test obligation, not an assumption

The rewrite changes how the newest-match-per-session is chosen. `ROW_NUMBER()` and `ORDER BY ... LIMIT 1` agree only if the ordering key is identical, including the `rowid` tiebreak. Tests must assert both forms select the same message when several messages in one session match, and when timestamps tie.

## Risks / Trade-offs

- **`EXISTS` and the scalar subquery evaluate the same predicate twice per session** → Each is an early-terminating index seek, and the measurement already includes both. If it ever matters, the pair collapses into one `LEFT JOIN` on the correlated id.
- **A query matching nothing still scans every session's messages** → Unchanged from today, and inherent to a leading-wildcard `LIKE`. The improvement targets queries that match, which is what users actually run.
- **Two statements instead of one may drift** → Mitigated by keeping the column list, metadata predicates, ordering, and limit in shared constants so only the match strategy differs.
- **Behavior parity is invisible to the type system** → Covered by tests that run both branches over the same fixture and compare results, including the sub-three-character and at-three-character boundary.

## Migration Plan

None required. No schema change, no new index, no persisted state, no data backfill. The change is confined to statement construction inside one repository method, and rollback is reverting the commit.

## Open Questions

- Should the two-character path also apply to three-character queries whose characters cannot produce a usable trigram in practice? Deferred: the current threshold is correct for the tokenizer, and widening it without evidence would push more queries onto the scan path.
