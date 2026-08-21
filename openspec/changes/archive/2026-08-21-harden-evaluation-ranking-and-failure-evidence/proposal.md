## Why

Two defects found while driving Agent evaluation end to end through the desktop client (WebdriverIO, `tests/desktop/specs/ui-evaluation.e2e.mjs`) are not implementation slips — they change what the arena *means*, so they cannot be fixed without moving the spec first.

**Missing evidence is currently a ranking advantage.** `compare_aggregates` (`evaluation_engine.rs:127-141`) ranks on `success_rank`, then on `failed_checks`. `success_rank` is binary — `Succeeded` or not — and `failed_checks` counts failing checks in a list that is *empty* for every non-completion outcome, because `aggregate_error` (`evaluation_engine.rs:140`) returns `checks: vec![]` for `AgentFailed`, `TimedOut`, `Stuck`, `Cancelled` and `BenchmarkError`. So an Agent that crashed before writing a line ranks **ahead** of an Agent that produced a patch failing one test: zero failed checks beats one. That is the exact shape `agent-evaluation` already forbids — "SHALL NOT treat the unavailable value as zero or use it as a hidden ranking advantage" — but the requirement is written about metric coverage, and the ranking scenario beside it only pins the passing case. Neither sentence, read literally, rules out what the code does, so the fix needs a requirement that does.

**A failed attempt carries no reason at all.** `EvaluationApi::execute` calls `aggregate_evaluation(dispatch.map(|result| result.evidence), …)` (`evaluation_api.rs:229`), which discards the `Err(String)` from `NativeEvaluationAgentAdapter::dispatch` and returns `aggregate_error`. The user is left with `agent_failed`, `0/0` checks, no metrics, and a one-line timeline — verified on screen: an arena run against an Agent with no configured model renders exactly that, with nothing anywhere in the UI saying the Agent had no model. The spec requires benchmark-error classification and redacted safe reasons for *isolation* failures (`Isolation setup is unsafe`), but says nothing about dispatch failures, so today's silent drop is spec-conformant.

## What Changes

- **BREAKING (export format): ranking version moves `deterministic-v1` → `deterministic-v2`.** Every arena, export payload and stored snapshot carries `rankingVersion`; a change in ordering semantics under an unchanged version string would make two exports incomparable while claiming they are comparable. Consumers that pinned `deterministic-v1` (the Web/mock adapter and its tests, `tests/e2e/evaluation-center.spec.ts`) move with it. Arenas already stored under v1 keep their recorded version and are ranked under the algorithm the version names.
- **Ranking gains a graded outcome tier**, so a deterministic task failure with recorded evidence outranks an Agent failure, timeout, stuck, cancellation or benchmark error that recorded none. Within a tier the existing comparison (failed checks, then interventions, then tool calls) is unchanged.
- **A failed Agent dispatch records a bounded, redacted diagnostic check** on the attempt, so `agent_failed` arrives with a reason a user can act on instead of an empty panel. The check is evidence, not a verdict: it never converts a failure into a success, and its presence must not push the attempt below an attempt that failed with no evidence at all — the tiering above is what keeps that honest.
- Web/mock adapter follows the same rules, as the runtime-parity requirement already demands.

This affects **both runtimes**: the ranking and diagnostic evidence are produced natively, and the Web/mock adapter has to simulate the same shapes. No adapter boundary moves — React keeps reading everything through `agentService`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-evaluation`: `Arena comparison is transparent and versioned` gains a requirement that outcome tiering must not let absent evidence outrank recorded evidence, and pins the ranking version bump. `Deterministic verification is authoritative` gains a requirement that a failed Agent dispatch records a bounded redacted diagnostic as non-authoritative evidence.

## Impact

- `src-tauri/src/contexts/execution_observability/application/evaluation_engine.rs` — `success_rank`, `compare_aggregates`, `aggregate_evaluation`.
- `src-tauri/src/contexts/execution_observability/domain/evaluation.rs` — `EVALUATION_RANKING_VERSION`.
- `src-tauri/src/contexts/execution_observability/evaluation_api.rs` — dispatch error routed into the aggregate instead of dropped.
- `src/services/web-evaluation-client.ts` — mock arenas report the new ranking version and the same diagnostic shape.
- Tests: `evaluation_engine_tests.rs`, `evaluation_api.rs` unit tests, `tests/e2e/evaluation-center.spec.ts`, `tests/desktop/specs/domain-evaluation.e2e.mjs`, `tests/desktop/specs/ui-evaluation.e2e.mjs`.
- Not affected: SQLite schema (no migration — `rankingVersion` is already a stored string and checks already have a table), the Tauri command surface, and the frontend service interface.
