## Context

Canonical Run state and events are owned by `operations`; `agent_runtime`, task orchestration, and other execution contexts consume that published Run API while retaining their owner-specific execution policies. Evaluation metrics, execution timelines, Plans, Sessions, approvals, reviews, context evidence, usage, and logs already have owning contexts and frontend services. The current UI exposes these through separate destinations and `MainLayout` keeps visited destinations mounted. Mission Control must aggregate only safe identifiers and compact projections without importing private repositories across bounded contexts or issuing per-row frontend requests.

The canonical Run tables already provide additive persisted snapshots/events and bounded list/detail queries. Existing service composition uses `AgentService` plus Tauri and Web adapters; direct feature services also exist for observability and review. Architecture fitness rules require React-to-service isolation and cross-context access through published APIs.

## Goals / Non-Goals

**Goals:**

- Make canonical Run snapshots the sole lifecycle source and build one bounded operational projection.
- Keep summary queries constant-count and indexed for 100+ historical Runs.
- Reuse owning surfaces and lazy evidence services for detail and actions.
- Provide deterministic Web behavior and real desktop parity with safe additive migration.
- Coalesce noisy events while flushing attention and terminal changes immediately.

**Non-Goals:**

- Replacing Session, Plan, Loop, Goal, Eval, Approval, Review, or log UIs.
- Introducing a new native bounded context or a generalized background-runner abstraction from roadmap item 12.
- Inventing remote-runner data, token usage, cost, verification, artifacts, or context evidence.
- Implementing roadmap item 08 security architecture or later roadmap items.

## Decisions

### 1. Operations owns the Mission Control query application service

The existing `operations` context owns canonical Runs, their guarded lifecycle, and persistent snapshots/events, so it will own the Mission Control query use case and DTO-neutral projection models. It may consume deliberately published immutable APIs from Sessions, agent runtime, task orchestration, execution observability, permissions, evaluations, Goals, and review-supporting workspace contracts through application-owned ports assembled by bootstrap. Owner-specific retry and verification remain delegated to the corresponding published APIs; no context reaches into another context's infrastructure.

Alternative considered: a new `mission_control` context. Rejected because the dashboard has no independent lifecycle or invariants and would duplicate canonical Run ownership.

### 2. One bounded overview query, lazy detail facets

The overview command returns counts, bounded attention/active/recent pages, filter facets, a cursor, and safe `MissionControlRunSummary` records. SQL selects from canonical Run snapshots with additive indexes and a small fixed set of joined or batched projections; it never fetches log/diff/artifact bodies. Detail returns a facet availability manifest. Selecting a facet calls its existing bounded owning service and correlates by stable links.

Alternative considered: frontend fan-out across six services. Rejected because it creates N+1 latency, inconsistent snapshots, and cross-runtime drift.

### 3. Additive projection fields and indexes, no destructive backfill

Where canonical snapshots lack safe display metadata, additive nullable columns or a projection table will be introduced in the centralized database migration order. Existing Run records remain queryable with unavailable fields. Projection updates share the authoritative state transaction or consume idempotent safe events; migration does not rewrite legacy business tables.

Rollback uses an older binary that ignores the new table/indexes. Failed migrations roll back transactionally.

### 4. Shared frontend contract and runtime-neutral actions

Mission Control types live in `src/types` and mirrored contracts. `AgentService` exposes overview, detail manifest, retry/verification controls, and a subscription or reconciliation signal. The Tauri adapter alone invokes new commands. The Web adapter owns deterministic seeded fixtures and simulates only contract-visible changes. Components receive service and navigation callbacks and contain no runtime checks.

Open and approval/review actions resolve typed navigation targets. Approval decisions themselves remain in the existing approval UI; Review Changes routes to the existing Code Review Center.

### 5. State-driven action policy remains native-authoritative

Summary `availableActions` is a hint for presentation. Every mutation is revalidated against canonical state/version, owning runtime capabilities, and permission policy. Cancel and resume reuse existing controls; retry and verification delegate to existing owner services. Race losers reconcile rather than forcing state.

### 6. Event reducer separates urgent and noisy updates

State, attention, and terminal events invalidate or patch immediately. Usage/progress events are keyed by Run id and coalesced into one bounded update window. Mount, reconnect, and `visibilitychange`/focus trigger a bounded refetch. The reducer rejects stale sequence/version updates and terminal transitions flush pending progress first.

### 7. Compact keep-alive workspace UI

`MainLayout` adds a lazy visited Mission Control destination and activity icon. The page uses a summary strip, attention queue, bounded active/recent lists, and a detail drawer/pane. Desktop uses compact columns; narrow layout uses stacked cards and a scrollable/select detail navigator. All visible text is present in every registered locale and semantic tokens cover both themes.

### 8. Verification is structural and scenario-driven

Rust tests cover aggregation, authorization, redaction, migrations, query plans/query counts, restart reconciliation, invalid cursors, stale versions, and event coalescing inputs. Vitest covers contract parity, deterministic mocks, filters/sorts/actions, reducer behavior, accessibility, and layouts. Playwright covers multi-Run attention and failure flows plus four visual combinations. Desktop E2E covers a real operation projection. Performance evidence uses explain plans, fixed query counters, page/item bounds, and event batch counts rather than shared-runner timing.

## Risks / Trade-offs

- [Cross-context data can drift] → Store stable links and unavailable states, reconcile from authoritative APIs, and never infer terminal state from secondary evidence.
- [A broad join can regress startup] → Bound all sections, add selective indexes, assert query plans and constant query count, and lazy-load detail bodies.
- [Action hints become stale] → Treat hints as presentation only and revalidate every mutation with current Run version.
- [High-frequency events cause render churn] → Coalesce non-urgent events per Run and immediately flush only state/attention/terminal events.
- [Legacy Runs lack metadata] → Keep fields optional and visibly unavailable; do not destructive-backfill or fabricate values.
- [Navigation targets vary by owner] → Use a typed target union and explicit unavailable state, keeping destination-specific resolution out of React cards.

## Migration Plan

1. Add and transactionally test any required nullable projection storage/index migration.
2. Deploy native query/control APIs and frontend adapters before exposing navigation.
3. Enable the Mission Control destination after Web and Tauri contract parity tests pass.
4. On rollback, older binaries ignore additive schema and existing workflows remain authoritative.
