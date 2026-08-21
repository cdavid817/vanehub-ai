## Why

The Loop runtime already supports durable definitions, isolated runs, independent verification, recovery, evidence, and human acceptance, but the Loop Center does not expose those capabilities as a complete operational workflow. Users cannot directly run or manage an existing definition, must type project and branch identifiers manually, and must move between an empty center surface and a secondary inspector to understand or control a run.

## What Changes

- Add a definition overview state that presents the selected Loop's goal, scope, Agents, verification policy, limits, recent outcomes, and a primary start action when no run is selected.
- Add direct definition management for start, enable or disable, duplicate, and guarded delete, with active-run consequences made explicit.
- Replace free-form project and base-branch entry with known local Git project selection and branch discovery while preserving service-boundary and Tauri/Web adapter parity.
- Add a preflight check before start that reports project, branch, Agent eligibility, verification command, path-scope, and active-run readiness with actionable remediation.
- Move state-appropriate run controls and the current activity summary into a persistent run header so critical actions remain visible at narrow widths.
- Rework iteration presentation around decision-ready summaries, budget consumption, changes since the prior iteration, verification outcomes, Verifier findings, and recovery guidance without duplicating the full evidence list by default.
- Present awaiting-acceptance runs as a focused decision panel that relates acceptance criteria, required checks, Verifier advice, changes, risks, and accept/continue/reject actions.
- Preserve existing native runtime semantics, durable history, worktrees, unified logging, localization, accessibility, and both registered visual styles.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `loop-management-ui`: Expand the Loop Center contract from basic definition editing and run monitoring into a complete definition-to-run-to-human-decision workbench for both desktop and Web/mock runtimes.

## Impact

- Frontend Loop Center components, query hooks, localized resources, responsive behavior, and component/E2E coverage will change.
- The frontend Loop service contract and both Tauri and Web/mock adapters will gain parity operations for definition duplication, readiness/preflight inspection, and project/branch selection where an equivalent service does not already exist.
- Native changes, if required for preflight aggregation or duplication, remain inside the existing `agent_runtime` context and commands; React will not call Tauri directly.
- Existing Loop run lifecycle, SQLite records, verification execution, session ownership, worktree preservation, and unified-log behavior remain backward compatible.
- No new UI library, state-management library, database, or runtime dependency is introduced.
