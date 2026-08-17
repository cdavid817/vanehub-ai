## Why

Canonical Runs, evaluation metrics, approvals, reviews, Goals, Plans, Loops, Sessions, and operation telemetry already exist, but their operational state is spread across separate surfaces. Operators need one bounded, attention-first view that answers what is running, blocked, awaiting input, failed, or recently completed without duplicating the owning workflows.

## What Changes

- Add a Mission Control workspace destination with bounded summary counts, an attention inbox, active Runs, and recently completed Runs.
- Add a native Mission Control read projection over canonical Run snapshots and existing owning-context contracts, with stable pagination, filtering, sorting, detail availability, navigation targets, usage/verification summaries, and no per-row aggregate or log loading.
- Add detail tabs for overview, plan/tasks, timeline, tools, files/artifacts, review, tests/verification, context, usage, and logs; unavailable evidence is represented explicitly and loaded only when requested.
- Add state- and permission-aware Open, Cancel, Resume, Retry, approval navigation, Review Changes, and Run Verification actions through the shared frontend service boundary and contract-compatible Tauri and deterministic Web/mock adapters.
- Reconcile canonical state on mount, app focus, and terminal events while coalescing high-frequency events so token traffic does not rerender the dashboard per token.
- Add localized, accessible, compact responsive UI for futuristic/minimal themes at desktop and narrow widths, plus functional, visual, desktop, safety, bounded-query, and performance verification.
- Preserve all existing business pages as the owners of editing, chat, approval, review, and verification workflows; Mission Control links to them rather than cloning them.

## Capabilities

### New Capabilities

- `agent-mission-control`: Defines the bounded cross-Run read model, attention workflow, detail availability, actions, reconciliation, runtime parity, security, performance, and responsive Mission Control UI.

### Modified Capabilities

- `main-layout-ui`: Adds Mission Control as a persistent workspace destination while preserving mounted workspace state and responsive navigation.
- `agent-run-state-management`: Extends the shared Run service with bounded Mission Control query/control projections and explicit retry semantics without changing canonical lifecycle ownership.

## Impact

- Both Tauri desktop and Web/mock runtimes are affected.
- Frontend contracts/types, `AgentService`, Tauri/Web adapters, workspace routing/activity bar, localized resources, Mission Control components, and Playwright coverage are extended.
- Native work remains in existing bounded contexts: canonical Run state/control and the dashboard query projection stay in `operations`; owner-specific execution behavior remains in `agent_runtime`, `task_orchestration`, and the other owning contexts behind their published contracts. No new bounded context or runtime-specific React branch is introduced.
- SQLite changes, if required by the audited query plan, are additive and retain compatibility with existing databases and older binaries.
- Existing Code Review Center, permission approval, evaluation, observability, unified logging, Goals/Plans/Loops/Sessions, and notifications remain authoritative and are linked rather than copied.
