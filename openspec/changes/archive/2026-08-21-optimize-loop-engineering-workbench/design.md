## Context

See `proposal.md` for motivation. The Loop Center already has a three-panel responsive shell, a four-step definition dialog, query-backed monitoring, run controls, and inspection links. The frontend Loop contract supports definition CRUD and the complete run lifecycle, while native execution already performs authoritative validation during definition save and run start.

The change crosses React presentation, query state, frontend service contracts, Tauri/Web adapters, and potentially the `agent_runtime` and `workspaces` native boundaries. It must preserve loaded history during slow operations, keep every native call outside React components, and retain equivalent Web/mock behavior without claiming local execution.

## Goals / Non-Goals

**Goals:**

- Make a saved definition directly understandable, manageable, and runnable without reopening the editor.
- Make preflight informative and non-mutating while retaining authoritative validation at start.
- Keep the user's current decision and next action visible across desktop and narrow layouts.
- Derive summaries and iteration comparisons from durable Loop data without introducing a second run-state authority.
- Reuse existing visual tokens, shared controls, inspection surfaces, and query infrastructure.

**Non-Goals:**

- Change the preparing, acting, verifying, deciding, or finalizing lifecycle.
- Add scheduling, automatic merging, worktree deletion, analytics, templates, or verification-command auto-detection.
- Persist derived presentation summaries or acceptance-criterion pass states as new domain truth.
- Add a UI library, state store, animation framework, or new top-level native bounded context.

## Decisions

### 1. Give the center surface explicit definition and run modes

The center surface will render a definition overview whenever a definition is selected and no run is selected. Selecting a run switches the same surface to a run workspace with a persistent action header, phase rail, and content sections. The left panel remains navigation and the right panel remains contextual inspection.

This keeps the existing three-panel mental model while eliminating the dead-end empty-run state. A separate route per run was considered, but would duplicate selection state and make narrow-layout return behavior more complex.

### 2. Keep critical controls in the run surface and contextual detail in the inspector

State-derived controls will be reused through a shared control model and rendered in the persistent center action region. The inspector retains limits, identifiers, workspace metadata, and deep inspection links. There will be one mutation owner per action so rendering controls in a new location does not duplicate requests.

Keeping controls only in the inspector was rejected because the inspector is hidden behind a drawer below the desktop breakpoint and human acceptance is a primary workflow state, not secondary metadata.

### 3. Add one non-mutating readiness projection behind the Loop service

The frontend contract will gain a readiness operation that returns a typed report with overall readiness and ordered checks. Each check has a stable code, category, blocking flag, status, localized-message inputs, and optional remediation target. The report checks project, branch, role eligibility, verification commands, path policy, enabled state, and active-run conflict without creating a run, worktree, process, or session.

The Tauri adapter invokes a Loop-specific command backed by the existing `agent_runtime` application service. Native code composes published `workspaces` APIs and Agent registry/readiness contracts rather than reaching into their infrastructure. The Web adapter returns deterministic simulated checks and marks the report simulated.

Preflight is advisory against races: `startLoop` remains the authoritative modifying command and repeats all safety-critical validation. Folding preflight into start was rejected because failures would arrive too late to support guided remediation and would blur non-mutating readiness with launch.

### 4. Reuse project discovery and add bounded branch discovery through service contracts

Known projects will come from the existing project/workspace service projection. A bounded branch-list operation will return stable branch references for a selected canonical local Git project. It performs read-only Git inspection in the native `workspaces` context and returns simulated choices in Web/mock mode.

The editor stores the same canonical path and branch strings already used by `LoopDefinition`; no schema migration is needed. If a saved value disappears from discovery, the frontend adds it as an unavailable retained option rather than silently selecting a replacement.

Allowing arbitrary path text as the normal flow was rejected because it shifts discovery and correction to the final submit error. A deliberately retained unavailable value protects edit compatibility with moved or temporarily offline repositories.

### 5. Implement duplicate as composition over existing definition creation

Duplication will build a new create request from the selected definition, force `enabled: false`, clear identity/version timestamps, and require a distinct name. The native create path remains responsible for validation and persistence, so no duplicate-specific command or database migration is needed.

A native row-copy command was rejected because it would duplicate validation rules and create a second persistence pathway for the same aggregate.

### 6. Derive budgets and comparisons in a pure presentation layer

Budget consumption, latest activity, verification deltas, change-count deltas, and no-progress explanations will be computed from `LoopRun`, its immutable definition snapshot, iterations, and evidence. Pure selectors will be independently unit tested. Unknown or absent evidence will render as unknown/not evaluated, never as a pass.

Persisting these values was rejected because all inputs are already durable and a second stored projection could drift after recovery or adapter reconciliation.

### 7. Use progressive disclosure for evidence and a focused acceptance composition

Iteration cards default to a concise outcome summary and disclose the chronological raw evidence list on request. The acceptance panel composes acceptance criteria, required checks, Verifier findings, changes, risks, and controls in the center surface; session, logs, files, and changes continue to open existing inspection surfaces.

The frontend cannot claim criterion-level automated pass unless evidence explicitly supports it. Initial criterion rows therefore use evidence-backed or not-evaluated states. Adding AI-generated criterion judgments is outside this change.

### 8. Preserve asynchronous data while management operations run

Definition mutations, discovery, branch loading, preflight, and start will use query/mutation state with stable operation feedback. Loaded definitions and run history stay visible during refresh. Successful mutations invalidate the narrowest relevant query keys; failures keep selection and entered feedback intact.

Components will call only the frontend service boundary. Tauri `invoke()` remains confined to the Tauri adapter, Web/mock maintains the same Promise-based contracts, and variable-duration native work follows the existing observable-operation conventions where it cannot complete promptly.

## Risks / Trade-offs

- [Preflight can become stale before start] → Treat it as advisory, repeat authoritative checks during start, and refresh the report after a rejected start.
- [The center header may become crowded] → Show status, current activity, budget, and state-primary actions only; move identifiers and secondary metadata to the inspector or overflow menu.
- [Existing definitions may reference undiscoverable projects or branches] → Retain unavailable values visibly and block start only through explicit readiness results.
- [Derived comparisons depend on optional evidence details] → Render unknown values explicitly and compare only fields with durable evidence on both iterations.
- [Duplicating controls can cause duplicate mutations] → Use one shared action model and mutation controller with pending-state idempotence.
- [Web/mock could be mistaken for native readiness] → Carry and display a simulated marker on project choices and readiness reports.

## Migration Plan

1. Add additive frontend types and service methods, then implement deterministic Web/mock behavior and adapter parity tests.
2. Add native read-only branch discovery and Loop readiness aggregation without changing existing start semantics or SQLite schema.
3. Introduce pure presentation selectors and the definition overview before moving controls, keeping existing inspector controls until parity tests pass.
4. Switch to the persistent run header, decision-oriented iterations, and acceptance panel; remove duplicated default evidence and inspector control rendering.
5. Update localized resources, component tests, responsive tests, Playwright coverage, and both-style visual QA.

Rollback removes the additive UI and service operations; existing definitions, runs, evidence, sessions, and worktrees require no data rollback.
