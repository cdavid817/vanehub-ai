## 1. Persistence and domain policy

- [x] 1.1 Add migration tests for criterion evidence bindings, PlanRun policy snapshots, driver intent, finalization runs, final verification evidence, and final-repair attempts while preserving legacy Plan reads.
- [x] 1.2 Implement the additive SQLite migration, indexes, repository row mappings, and idempotent upgrade behavior without changing completed historical runs.
- [x] 1.3 Add domain tests for evidence kinds, command references, maximum attempt bounds, repair-class allowlists, final validation commands, and the new final-verifying and action-required transitions.
- [x] 1.4 Implement execution-policy domain types and validation, including rejection of required SubTasks with no required guarded command or unresolved criterion evidence.
- [x] 1.5 Extend approval snapshot tests and repositories so discovery limitations, evidence bindings, retry policy, final commands, and non-secret Profile identity become immutable PlanRun policy.
- [x] 1.6 Add an idempotent migration, domain transitions, and repository tests for durable `verifying`/`repairing` PlanRun states plus the optional opaque originating OnePiece session association.

## 2. Project-aware OnePiece planning

- [x] 2.1 Add contract tests for the workspace-bounded discovery execution profile, exact read-only tool catalog, resource limits, Profile capture, prohibited operations, and credential exclusion.
- [x] 2.2 Extend Agent runtime APIs with a dedicated OnePiece planning discovery profile limited to file read, grep, glob, available workspace code search, and trusted read-only language-intelligence operations.
- [x] 2.3 Update the versioned planner instruction and strict response schema with discovery limitation metadata, evidence bindings, retry policy, and final validation commands.
- [x] 2.4 Extend task-orchestration planner parsing and validation tests for project-aware output, degraded discovery, limit exhaustion, invalid bindings, and malformed final commands.
- [x] 2.5 Implement bounded discovery session orchestration and persist only safe limitation metadata and the resulting validated draft rather than raw discovery tool payloads.

## 3. Plan review and approval contracts

- [x] 3.1 Extend shared Plan types with criterion evidence policies, structured validation commands, discovery status, attempt policy, final validation commands, and approval-transition summaries.
- [x] 3.2 Extend the native commands and Tauri Plan adapter for the new draft, validation, approval, and execution-policy fields without exposing SQLite to React.
- [x] 3.3 Extend the Web/mock Plan adapter with deterministic compatible validation and approval behavior that remains explicitly simulated.
- [x] 3.4 Add adapter conformance fixtures covering evidence bindings, retry policy, final checks, approval summaries, and validation error shapes.
- [x] 3.5 Add component tests and implement accessible Plan review editors for validation commands, criterion evidence bindings, retry limits, and final verification commands.
- [x] 3.6 Extend shared Plan types and both adapters with `verifying`/`repairing` projections, optional `originatingSessionId`, and a bounded nullable lookup by originating session id.

## 4. Durable autonomous driver

- [x] 4.1 Add driver state-machine tests for continuous serial progress, singleton activation, no-work projection, independent branches after failure, pause boundaries, cancellation, and terminal stop conditions.
- [x] 4.2 Implement persisted desired execution intent and an idempotent native per-PlanRun driver registry whose in-memory handles never override SQLite state.
- [x] 4.3 Move blocking attempt execution behind background worker activation so `start_plan_run` returns a prepared/running projection without blocking unrelated Tauri commands.
- [x] 4.4 Implement the continuous claim, execute, verify, project, and continue loop using existing transactional schedule claims and control boundaries.
- [x] 4.5 Add concurrency tests proving duplicate start, startup activation, and overlapping scheduler ticks create at most one Attempt for a claimed SubTask.
- [x] 4.6 Integrate driver shutdown, application lifecycle, pause, cancel, and shared session recovery so ambiguous in-flight work remains recovery-required and is never silently replayed.

## 5. Evidence-driven repair

- [x] 5.1 Add failure-classification tests covering eligible validation failure, exhausted budget, cancellation, safety rejection, missing credentials, timeout ambiguity, and inconclusive restart evidence.
- [x] 5.2 Extend attempt context construction with bounded failed command ids, redacted output summaries, changed-file summaries, attempt sequence, and remaining budget while excluding raw transcripts and tool payloads.
- [x] 5.3 Implement automatic repair dispatch as a new immutable Attempt and OnePiece session only for snapshotted eligible classes with remaining budget.
- [x] 5.4 Add repository and API tests proving repair retains prior sessions and evidence, descendants wait for verified success, and exhaustion projects an action-required state.
- [x] 5.5 Extend manual recovery controls so users can inspect evidence, retry when allowed, cancel, or return to planning without mutating the approved DAG.

## 6. Integrated final verification

- [x] 6.1 Add finalization repository tests for one active finalization run, guarded final command evidence, restart recovery, cancellation, and acceptance rejection before required checks pass.
- [x] 6.2 Implement final verification after all required SubTasks succeed and project `awaiting_acceptance` only after all required final evidence passes.
- [x] 6.3 Add bounded final-repair context and execution using separate finalization records rather than adding a hidden SubTask to the approved graph.
- [x] 6.4 Add tests and implement final repair budget exhaustion, action-required controls, evidence retention, and successful re-verification.

## 7. Plan and Agent mode experience

- [x] 7.1 Add component tests for persistent icon-and-text Plan and Agent labels, read-only/write-capable descriptions, keyboard navigation, visible focus, and accessible phase announcements without color-only semantics.
- [x] 7.2 Extend the OnePiece composer mode presentation with Plan-oriented versus Agent-oriented primary actions and explanatory capability status while preserving other agents' existing modes.
- [x] 7.3 Add the approval surface showing project, task count, verification scope, retained-worktree behavior, continue-planning, edit, and approve-and-execute actions.
- [x] 7.4 Add active-run mode-transition tests and implement association-backed pause-before-planning behavior that waits for a persisted safe boundary before the composer presents Plan safety.
- [x] 7.5 Update Plan Center progress, repair history, action-required recovery, final verification, and final acceptance views; remove the user-facing execute-next control.
- [x] 7.6 Keep Plan Center as the artifact/evidence surface and add keyboard-operable navigation from the explicitly associated originating OnePiece session without selecting a run by global recency or introducing a second mode state.

## 8. Runtime adapter parity and observability

- [x] 8.1 Extend bounded Plan polling or subscription projections so React observes native background progress without driving scheduling or transferring full transcripts.
- [x] 8.2 Add Tauri and Web/mock conformance tests for durable running, verifying, repairing, paused, action-required, final-verifying, awaiting-acceptance, and completed projections, including restart-stable association lookup.
- [x] 8.3 Add unified diagnostic events and execution-topology correlation for discovery, driver activation, claims, repairs, exhaustion, finalization, controls, and recovery using safe metadata only.
- [x] 8.4 Add logging privacy tests proving goals, task descriptions, prompts, credentials, raw tool payloads, full paths, and unredacted validation output do not enter diagnostics.
- [x] 8.5 Update localized UI resources and user/developer documentation for OnePiece Plan/Agent semantics, approval boundaries, automatic repair limits, retained worktrees, and Web/mock simulation.

## 9. End-to-end and release verification

- [x] 9.1 Add Rust integration tests for goal discovery through session-associated Plan approval, autonomous multi-SubTask execution, verifying/repairing transitions, final verification, restart recovery, pause, cancellation, and action-required outcomes.
- [x] 9.2 Add frontend integration tests for project discovery feedback, draft validation, approval, background progress, evidence inspection, manual criteria, repair exhaustion, and final acceptance.
- [x] 9.3 Add Playwright coverage for the OnePiece Plan-to-Agent happy path plus verification-failure repair and pause-before-mode-switch behavior in the supported test runtime.
- [x] 9.4 Run `npm run lint:ci`, fix all findings, and record the result.
- [x] 9.5 Run `npm run test`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`, fix all findings, and record the results.
- [x] 9.6 Run `npm run build` and `npx playwright test`, fix all findings, and record the results. (`npm run build` passed and full Playwright passed 93/93 after prewarming the reload-heavy Basic Settings and Agent Configuration modules.)
- [x] 9.7 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`, fix all findings, and record the results.
- [x] 9.8 Run `openspec validate complete-onepiece-plan-agent-loop --strict` and `openspec validate --specs --strict`, fix all findings, and record the results.
