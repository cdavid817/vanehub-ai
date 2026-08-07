## Why

`session-management` already requires that a two-character query "SHALL use a bounded compatibility query". The implementation does not meet that requirement in any meaningful sense: the fallback bounds only the *result count*, while the work it performs is unbounded.

The message-match subquery ranks candidates with `ROW_NUMBER() OVER (PARTITION BY session_id ...)`. A window function cannot be short-circuited by the outer `LIMIT`, so SQLite must materialize **every** matching message and sort the whole set before the limit applies. On the short-query path that set is produced by an unindexed `content LIKE '%..%'` scan of the entire `messages` table.

Measured on a synthetic 60 000-message / 500-session database, the two shapes return identical rows:

| Query shape | Time | Plan |
| --- | --- | --- |
| Current window function | 69.7 ms | `MATERIALIZE ranked`, `SCAN messages`, `USE TEMP B-TREE FOR LAST 2 TERMS OF ORDER BY` |
| Bounded correlated form | 5.1 ms | `SEARCH m USING INDEX idx_messages_session_created (session_id=?)` |

This path is not an edge case for every user equally. The three-character threshold is not a tuning choice — the FTS5 `trigram` tokenizer produces no tokens below three characters, so a shorter query cannot use the index at all. Confirmed empirically: two-character `MATCH`, quoted-prefix `"xx"*`, and bare `xx*` all return zero rows, for CJK and Latin alike. Chinese search terms are commonly two characters ("报错", "部署", "配置"), so for Chinese users the *typical* query is precisely the one that takes the slowest path.

## What Changes

- The short-query message-match subquery is restructured from a window function over all matches into a correlated per-session lookup that resolves the newest matching message through the existing `idx_messages_session_created` index.
- The three-character FTS path keeps its current shape. Correlating an FTS `MATCH` per session would re-run the full-text query once per candidate, which is worse than materializing an index-pruned match set.
- Result semantics are unchanged: the same sessions, in the same order, with the same "newest matching message per session" match context.
- No change to the three-character threshold, the tokenizer, the FTS schema, the service result shape, or any Tauri command signature. No migration.
- Not breaking.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `session-management`: strengthens the `Indexed desktop message search` requirement so "bounded compatibility query" constrains the **work performed**, not merely the row count returned. Adds the guarantee that short-query search resolves its match context through per-session index lookups instead of ranking every matching message in the database.

## Impact

**Runtime scope: desktop only.** The Web/mock adapter simulates search over fixture data and is untouched, as is the frontend service contract — this changes SQL inside one repository method.

- Affected code: `src-tauri/src/contexts/sessions/infrastructure/sqlite_repository.rs` (the `search` method's SQL construction).
- The two branches stop sharing one templated statement skeleton and become two complete statements, since the bounded shape and the FTS shape now differ structurally rather than only in their `FROM` fragment.
- No frontend/backend isolation impact: React still reaches search through the existing service interface.
- No new dependencies, no schema change, no new index — the required index already exists.
- Existing behavior that must not regress: results still come from a single result query, with no per-result round trip from Rust.
