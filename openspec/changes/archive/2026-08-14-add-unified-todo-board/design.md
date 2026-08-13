## Context

See `proposal.md` for motivation. Sessions and Scheduled Tasks currently live behind the agent service, while Plans use a separate plan service. SQLite is shared by the native contexts, Plan runs are globally pageable, Plan drafts are not globally discoverable, and Scheduled Tasks retain only their latest run Session. The workspace already lazy-loads full-screen Plan and Loop destinations.

## Goals / Non-Goals

**Goals:**

- Add a durable board aggregate without making it the owner of Session, Plan, or Scheduled Task lifecycle.
- Reconcile sources idempotently and preserve one card when it gains multiple links.
- Make source lineage explicit enough to suppress execution-child Session duplication.
- Keep desktop and Web behavior contract-compatible and keep React isolated from Tauri APIs.
- Provide responsive, accessible organization with deterministic persisted ordering.

**Non-Goals:**

- Multiple user-defined boards, collaborative sync, custom columns, tags, WIP policies, nested manual subtasks, or automation-rule editing.
- Automatically moving board stages from runtime status changes.
- Deleting, archiving, pausing, or otherwise mutating a source when a work item changes lifecycle.
- Treating every PlanRun or execution Session as a top-level work item.

## Decisions

### Board metadata and runtime projections remain separate

Native storage uses `work_items` for user-owned organization and `work_item_links` for stable source identity. List responses join or query canonical source tables to return current projections. This avoids stale copied runtime status and prevents board movement from becoming a hidden runtime command. The rejected alternative was one polymorphic task table containing every lifecycle; incompatible state machines made that model ambiguous.

### Reconciliation is native, transactional, and idempotent

The board list operation first performs a bounded reconciliation transaction. Unique `(source_kind, source_id)` links guarantee repeatability. Eligible Sessions are those with user origin; Plan summaries aggregate by Plan id; every Scheduled Task is eligible. Existing links win even when archived. Web/mock performs the same algorithm over in-memory stores. Event-driven hooks can later reduce reconciliation latency, but correctness does not depend on every creation path remembering a hook.

### A link can be primary or supporting

A work item can hold multiple links, but a source belongs to at most one work item. Automatic reconciliation creates a primary link. User linking attaches execution, automation, or supporting sources. Child Sessions remain resolvable through lineage and activity history without a top-level link. This supports manual-to-Agent workflows while preventing duplicate ownership.

### Ordering uses sparse integer ranks per stage

Move requests carry target stage and neighboring identity rather than accepting arbitrary client rank values. The service allocates midpoint ranks and normalizes a stage when gaps are exhausted. This centralizes ordering conflict behavior and supports pointer and explicit keyboard controls through one operation.

### A dedicated WorkBoardService owns the frontend boundary

React depends on `WorkBoardService`; `tauri-work-board-client` contains invocations and `web-work-board-client` provides an in-memory implementation. This avoids further growth of the already broad AgentService and reflects the native bounded context. Shared contract verification covers both adapters.

### UI is a lazy full-screen workspace destination

Todo Board joins the activity destination union and lazy-loads its feature folder. Wide layouts show horizontally scrollable columns; compact layouts use a stage selector with one column. Native HTML drag events are optional enhancement only; menus and move controls are normative. Filters are applied client-side over the reconciled active or archive result for MVP-sized local datasets.

### Lineage and run history are additive migrations

Sessions gain nullable origin columns with legacy rows interpreted as user origin. Scheduled executions append to a new history table and continue updating current latest-run fields. Plan discovery adds a global aggregate query but does not change immutable PlanRun snapshots. Migrations are additive and idempotent.

## Risks / Trade-offs

- [Reconciliation during list can add latency on very large stores] → Use indexed uniqueness, one bounded transaction, summary projections, and only reconcile missing links.
- [Legacy child Sessions may be mistaken for direct work] → Backfill only relationships provable from Plan attempts or Scheduled Task latest/history data; otherwise preserve user origin.
- [Cross-context SQL couples reconciliation to existing schemas] → Keep source reads behind application ports where practical and cover schema/query behavior with migration and repository tests.
- [Sparse ranks can converge after frequent moves] → Normalize only the affected stage transactionally.
- [Deleting a source leaves a broken link] → Preserve the work item and return an unavailable projection as required.
- [Automatic import creates a busy first view] → Default to Inbox/Planned stages, provide source/project filters, and support independent archive immediately.

## Migration Plan

1. Apply additive work-item, link, Session-lineage, and scheduled-run-history schema changes.
2. Backfill provable Plan and Scheduled Task Session lineage without rewriting ambiguous legacy Sessions.
3. On first board query, reconcile eligible existing sources transactionally; unique link constraints make retries safe.
4. Deploy adapters and UI after the native commands and Web implementation conform to the same contract.
5. Rollback may hide/remove the UI and commands while leaving additive tables and columns intact; existing source behavior remains unchanged.
