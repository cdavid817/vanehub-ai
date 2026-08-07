## 1. Pin current search behavior

- [ ] 1.1 Add a repository test that seeds several sessions, each with multiple matching messages at different timestamps, and asserts a two-character query returns the expected sessions in `updated_at DESC` order
- [ ] 1.2 Extend that test to assert each returned session's match context is its **newest** matching message, including a case where two messages in one session share a `created_at` and the `rowid` tiebreak decides
- [ ] 1.3 Add the same assertions for a three-character query so the FTS branch is pinned before the short branch is rewritten
- [ ] 1.4 Confirm 1.1-1.3 pass against the current implementation, establishing the equivalence baseline

## 2. Separate the two branches

- [ ] 2.1 Split the single templated statement into two complete statements, keeping the column list, session-metadata predicates, ordering, and limit in shared constants
- [ ] 2.2 Confirm the FTS branch statement is byte-equivalent in behavior to today's and that 1.3 still passes

## 3. Bounded short-query statement

- [ ] 3.1 Replace the short-query branch's ranking CTE with an `EXISTS` membership predicate correlated on `sessions.id`
- [ ] 3.2 Resolve match context via a correlated scalar subquery for the newest matching message id, joined back to `messages` on its primary key for content
- [ ] 3.3 Preserve the exact ordering key (`created_at DESC, rowid DESC`) so newest-match selection matches the previous `ROW_NUMBER()` behavior
- [ ] 3.4 Confirm tests 1.1 and 1.2 still pass unchanged

## 4. Confirm the plan actually improved

- [ ] 4.1 Capture `EXPLAIN QUERY PLAN` for the short-query statement and confirm it reports `SEARCH ... USING INDEX idx_messages_session_created (session_id=?)` with no `MATERIALIZE` of a ranked match set
- [ ] 4.2 Confirm the FTS branch's plan is unchanged from before the split
- [ ] 4.3 Sanity-check that a query matching nothing still returns an empty result rather than erroring

## 5. Verification

- [ ] 5.1 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [ ] 5.2 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [ ] 5.3 `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] 5.4 `npm run lint:ci`, `npm run test`, and `npm run build` to confirm no frontend impact
- [ ] 5.5 `openspec validate bound-short-query-message-search --strict`
- [ ] 5.6 `openspec validate --specs --strict`
