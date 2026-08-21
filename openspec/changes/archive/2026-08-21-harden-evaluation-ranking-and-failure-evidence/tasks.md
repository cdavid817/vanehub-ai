## 1. Ranking tiers

- [x] 1.1 Replace `success_rank` in `evaluation_engine.rs` with a three-tier outcome rank (deterministic success > deterministic task failure > non-completion) and apply it as the first key in `compare_aggregates`, leaving failed-checks, interventions and tool-calls as the later keys
- [x] 1.2 Add engine tests in `evaluation_engine_tests.rs` covering the tier boundary: a `task_failed` attempt with two failing checks outranks an `agent_failed` attempt with no checks; a `succeeded` attempt still outranks both; ordering within the non-completion tier still falls through to the metric comparisons
- [x] 1.3 Extend the `ranked` tests in `evaluation_api.rs` so the read-side ordering asserts the new tiering, not only the success-first case

## 2. Ranking version

- [x] 2.1 Move `EVALUATION_RANKING_VERSION` to `deterministic-v2` in `domain/evaluation.rs`
- [x] 2.2 Update `src/services/web-evaluation-client.ts` mock arenas to report `deterministic-v2`, and update `src/services/web-evaluation.test.ts` and any other assertion pinned to the v1 string
- [x] 2.3 Add a test that a stored arena keeps the ranking version it was recorded under rather than being rewritten on read

## 3. Dispatch diagnostics

- [x] 3.1 Stop discarding the dispatch `Err` in `EvaluationApi::execute`: route it into the aggregate as a failed `EvaluationCheck` with the stable `agent-dispatch` check id, redacted through an exact-match safe-reason rule in the domain (`safe_dispatch_diagnostic`), not the substring allowlist -- see design.md
- [x] 3.2 Confirm the diagnostic check does not change the attempt's classification — a dispatch failure stays `agent_failed`, not `task_failed` — and add the test that pins it
- [x] 3.3 Add a redaction test: a dispatch error carrying an absolute path or credential-shaped value must not reach the persisted check summary
- [x] 3.4 Mirror the shape in the Web/mock adapter so a mock `agent_failed` attempt also carries a bounded diagnostic check, keeping the two adapters at parity

## 4. Verification

- [x] 4.1 Extend `tests/desktop/specs/ui-evaluation.e2e.mjs` so a failed attempt's detail pane is asserted to name a reason instead of rendering an empty verification block
- [x] 4.2 Extend `tests/desktop/specs/domain-evaluation.e2e.mjs` to assert the arena reports `deterministic-v2` and that a dispatch-failed attempt carries at least one check
- [x] 4.3 No change needed: the mock's dispatch-failure branch only appears from the third Agent onwards and the page offers two, so the spec's expectations are unmoved; its screenshots are written to the test output path rather than compared against a stored baseline, so there is nothing to refresh (verified by running it: 5 passed)
- [x] 4.4 Run the full gate: `npm run lint:ci`, `npm run test`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo check`, `npm run contracts:check`, `openspec validate --specs --strict`, `openspec validate harden-evaluation-ranking-and-failure-evidence --strict`
- [x] 4.5 Run `npx playwright test` and the desktop evaluation specs (`VANEHUB_DESKTOP_FULL_SUITE=1`), and report each platform's result as PASSED / FAILED / BLOCKED / NOT RUN rather than extrapolating from one host
