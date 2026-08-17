## Context

The repository already has canonical Agent Runs in `operations`, provider execution in `agent_runtime`, worktree/filesystem boundaries in `workspaces`, metadata timelines in `execution_observability`, context evidence manifests, usage read models, bounded artifacts, and unified redacted logging. The missing layer is an evaluation application that freezes a task and Agent configuration, coordinates isolated attempts, verifies outcomes, and projects comparable local results without duplicating those systems.

The MVP must work in desktop and deterministic Web/mock modes, support OnePiece plus a managed CLI Agent, remain CI-runnable without paid models, and expose a dense responsive UI in both visual themes.

## Goals / Non-Goals

**Goals:**

- Parse and validate a versioned, bounded task-manifest format for 3–5 stable local fixtures.
- Execute one or more Agent attempts in clean fixture copies, correlate every attempt to a canonical Run, and distinguish harness failures from task failures.
- Apply deterministic checks before optional structured judging and collect provenance-rich outcome, efficiency, context, and reliability metrics.
- Persist bounded evaluation metadata and expose catalog, run, comparison, detail, timeline, and JSON export contracts through both frontend adapters.
- Provide deterministic fake-Agent coverage, negative security tests, repeatable performance evidence, UI visual coverage, and one minimal native desktop benchmark.

**Non-Goals:**

- Public/cloud leaderboards, remote runners, background Agent infrastructure, marketplace integration, or roadmap 06+ work.
- A general arbitrary-command runner, container sandbox, or new permission architecture.
- Fabricated token/cost values, raw prompt/log/diff storage in evaluation tables, or LLM judgment overriding deterministic failure.
- Parallel native execution in the MVP; arena selections run sequentially to make resource and isolation behavior predictable.

## Decisions

### 1. Extend existing bounded contexts through published APIs

Evaluation orchestration and read models live under `execution_observability`, because their purpose is measuring and comparing execution evidence. It calls published `operations`, `agent_runtime`, and `workspaces` APIs assembled in bootstrap; it never imports their infrastructure. This avoids a new peer context and keeps the complete bounded-context map unchanged. An alternative evaluation context would create a second owner for runs, execution, and metrics before the domain warrants it.

### 2. Use a constrained JSON-compatible YAML manifest subset

Manifests are versioned YAML files, but parsing is limited to declarative fields and validated values: stable ids, relative fixture paths, bounded prompt/timeout, allowlisted acceptance command ids, expected files, forbidden patterns, and metric flags. Commands resolve through repository-owned verifier profiles; manifest text is never passed to a shell. JSON remains valid YAML and is used for shipped fixtures to avoid adding a parser dependency. A general YAML dependency or free-form shell commands would increase supply-chain and command-injection risk.

### 3. Model arena and attempts separately

An evaluation arena freezes task version, ranking algorithm version, and requested Agent snapshots. Each attempt owns a canonical Run with owner type `evaluation_attempt`, a fresh isolated fixture directory, terminal classification, metrics, verification, and artifact references. Arena success is an aggregate presentation state; it cannot rewrite attempt outcomes.

### 4. Reuse workspace isolation with a bounded evaluation adapter

Native evaluation asks the `workspaces` API to copy an allowlisted fixture tree beneath an evaluation root, reject traversal/symlinks/oversize inputs, record the source revision, and remove incomplete workspaces after cancellation/timeout. Completed workspaces are retained only through the existing retention policy for inspection. Web/mock creates an in-memory isolation snapshot with the same contract and no native side effects.

### 5. Deterministic verification is authoritative

Verifier order is acceptance command profiles, static assertions, and diff rules. Any deterministic failure fixes the task outcome as failed. An optional structured judge records model, rubric/prompt version, seed/temperature support, evidence references, confidence, and notes, but can only annotate a deterministically passing result. Harness setup/parse/persistence failures use separate `benchmark_error` classifications.

### 6. Measurements preserve availability and provenance

Metric values carry `reported`, `estimated`, or `unavailable` quality. Missing tokens remain null; context metrics link existing evidence-manifest ids; costs are emitted only with a frozen pricing snapshot id and currency. Comparisons show independent columns and a transparent lexicographic ranking version: deterministic success, regression count, intervention count, then only mutually available efficiency metrics. Missing values are never converted to zero.

### 7. Keep persistence bounded and content-safe

SQLite stores manifests' safe metadata, arena/attempt snapshots, scalar metrics, verification summaries, classifications, and artifact ids. Prompt bodies, secrets, raw command output, full diffs, and absolute paths are excluded. Large output/diff artifacts use the existing artifact/log facilities and retention. Writes for a terminal attempt and its verification/metrics are atomic.

### 8. Preserve asynchronous service boundaries

`startEvaluation` returns an operation/canonical arena identifier before variable-duration work completes. React polls/subscribes through `AgentService`; only `tauri-agent-client.ts` invokes declared commands. The Web adapter simulates queued/running/terminal transitions deterministically. Existing results remain visible while refreshes run.

### 9. UI is a compact local operations surface

The Eval route uses shared semantic tokens and primitives: catalog/configuration, live attempts, sortable result comparison, selected-attempt verification/diff/context/tool timeline, and export. Desktop uses a stable multi-column layout; narrow widths use drawers/stacking without hidden actions. Both futuristic/minimal variants share semantics and screenshot fixtures.

## Risks / Trade-offs

- [Real CLI behavior differs by provider] → Snapshot provider/model/config and use the existing Agent runtime boundary; CI relies on the fake Agent while native smoke selects only an installed supported Agent.
- [Fixture commands could execute arbitrary host code] → Manifests reference allowlisted verifier ids, paths remain inside isolated roots, environment is sanitized, and negative tests cover shell metacharacters, traversal, symlinks, and oversized fixtures.
- [Long runs consume disk and CPU] → Sequential MVP execution, bounded timeouts/output/artifacts, cancellation cleanup, and retention maintenance.
- [Cross-context orchestration can erode boundaries] → Bootstrap composes published APIs; architecture tests forbid private/infrastructure imports.
- [Ranking can mislead when metrics are absent] → Versioned transparent ordering and per-column provenance; no opaque aggregate score.
- [SQLite migration rollback cannot drop tables safely] → Additive migration only; older binaries ignore new tables, while feature disablement leaves bounded records intact.

## Migration Plan

1. Add additive SQLite evaluation tables/indexes and migration compatibility tests.
2. Ship fixture manifests, native APIs/commands, and adapters behind the new route without transforming existing data.
3. Seed no user result rows; catalog is read from repository-owned built-ins.
4. On rollback, remove the route/commands while leaving additive tables untouched for forward compatibility.

## Open Questions

- Public leaderboard and remote/background runners remain roadmap dependencies and are intentionally deferred.
- Provider-specific reliable pricing snapshots can be expanded later; the MVP supports explicit snapshots and otherwise reports cost unavailable.
