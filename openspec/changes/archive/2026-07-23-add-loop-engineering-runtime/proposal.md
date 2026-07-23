## Why

VaneHub AI can currently run individual Agent sessions and scheduled one-shot tasks, but it cannot safely pursue an engineering goal across bounded iterations with independent verification, durable evidence, explicit stop rules, and human acceptance. A first-class Loop capability is needed to turn the existing session, worktree, operation, and logging foundations into a controlled engineering feedback cycle rather than an unbounded chat or ad hoc automation.

## What Changes

- Add durable Loop definitions, runs, iterations, evidence, and lifecycle state for manual, goal-driven engineering loops.
- Run each Loop in an isolated local Git worktree with one Worker session, an independent read-only Verifier session, deterministic verification commands, and native decision rules.
- Add hard iteration, time, runtime-error, and no-progress limits; support pause-after-step, immediate cancellation, restart recovery, and mandatory human acceptance before success.
- Add a dedicated Loop Center in the desktop and Web/mock workspace for defining Loops, monitoring phases and iterations, inspecting evidence, and accepting, continuing, or rejecting results.
- Extend frontend service contracts and both runtime adapters with equivalent Loop management behavior; React remains isolated from direct Tauri calls.
- Persist native Loop state in SQLite and associate all operation output and diagnostics with unified, redacted logging.
- Explicitly exclude scheduled triggers, autonomous task discovery, parallel Workers, automatic commits/PRs/merges/deployments, nested Loops, remote workspaces, and free-form flow-canvas authoring from the first phase.

## Capabilities

### New Capabilities
- `loop-engineering-runtime`: Defines Loop configuration, isolated execution, iterative Worker and Verifier orchestration, deterministic evidence, stop rules, recovery, persistence, and runtime adapter parity.
- `loop-management-ui`: Defines the Loop Center, Loop creation flow, run monitoring, iteration evidence, controls, responsive behavior, localization, and human acceptance interactions.

### Modified Capabilities
- `main-layout-ui`: Adds a persistent Loops activity entry and switches the workspace between session and Loop management destinations.
- `project-worktree-management`: Extends guarded worktree creation to Loop runs and preserves Loop worktrees for explicit human review.
- `session-runtime-management`: Adds non-activating Loop-owned Worker and Verifier sessions with isolated generation state and hidden-by-default session navigation behavior.

## Impact

- Frontend contracts and models in `src/types/`, `src/contracts/`, and `src/services/agent-service.ts` gain Loop definition, run, iteration, evidence, command, and control APIs.
- `src/services/tauri-agent-client.ts` and `src/services/web-agent-client.ts` gain contract-equivalent Loop implementations; React components continue to depend on the service boundary.
- The workspace shell gains a Loop destination and dedicated management components using existing semantic tokens, localization, notifications, and session inspection surfaces.
- The Rust `agent_runtime` context owns Loop invariants and orchestration, while using published `sessions`, `workspaces`, and `operations` contracts assembled in bootstrap.
- SQLite migrations add Loop-owned persistence without changing existing session or scheduled-task records.
- Native execution adds guarded verification-process calls and unified log associations. No new frontend state library, UI library, package manager, or feature-local log files are introduced.
