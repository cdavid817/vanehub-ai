## 1. Architecture and persistence foundation

- [x] 1.1 Register the `task_orchestration` bounded context and its published API without exposing internal repositories to Agent runtime, sessions, workspaces, operations, or observability.
- [x] 1.2 Add migration tests for additive Plan, PlanVersion, SubTaskSpec, dependency, PlanRun, SubTaskRun, SubTaskAttempt, verification, and durable-control tables with foreign keys and indexes.
- [x] 1.3 Implement the idempotent SQLite migration and repository row mappings without backfilling or changing existing Loop, GroupChat, Agent, or session data.
- [x] 1.4 Add repository tests for atomic PlanRun snapshot creation, normalized dependency persistence, compare-and-set task claims, attempt history, and paginated summary/detail queries.

## 2. Plan domain and validation

- [x] 2.1 Add domain tests for Plan state, version isolation, stable identities, SubTask ordering, resource limits, and immutable approved snapshots.
- [x] 2.2 Implement Plan, PlanVersion, SubTaskSpec, dependency, PlanRun, SubTaskRun, SubTaskAttempt, evidence, and control domain types with explicit state-transition errors.
- [x] 2.3 Add graph-validation tests covering the 1–10 SubTask bound, one-to-three acceptance criteria, unknown endpoints, self-edges, duplicate edges, cycles, and deterministic topological rank.
- [x] 2.4 Implement strict graph validation and deterministic topological ordering, storing dependency edges only in the normalized dependency relation.
- [x] 2.5 Implement draft editing, complete-version revalidation, explicit approval, and transactional immutable PlanRun snapshot creation.

## 3. OnePiece planner integration

- [x] 3.1 Add contract tests for the versioned tool-less planner request, strict structured response parser, active-Profile readiness failures, and prohibited credential persistence.
- [x] 3.2 Add the bounded planner instruction asset with available tool descriptions, maximum task count, single-session granularity guidance, acceptance criteria, dependencies, and response schema.
- [x] 3.3 Extend the OnePiece native API with a tool-less planning generation that captures the active Profile configuration and returns planner content without creating a worktree or Worker session.
- [x] 3.4 Implement the task-orchestration planner service that parses, validates, persists valid drafts, and records redacted actionable failures for invalid output.

## 4. Attempt execution and bounded context

- [x] 4.1 Add OnePiece execution-profile tests for bounded root, permitted tools, tool-call limit, token budget, timeout, safe limit termination, and Plan correlation fields.
- [x] 4.2 Extend the OnePiece API process boundary to accept attempt execution profiles while continuing to resolve credentials only through the existing Profile credential store.
- [x] 4.3 Add context-builder tests for direct-predecessor selection, deterministic ordering, budget truncation priority, and exclusion of transcripts, prompts, tool arguments/results, and credentials.
- [x] 4.4 Implement bounded attempt prompts and predecessor context containing task identity, result summary, changed-file summary, verification summary, and truncation metadata.
- [x] 4.5 Implement distinct session and attempt creation for every dispatch and retry, retaining prior session identities and evidence after execution stops.

## 5. Worktree preparation and serial scheduler

- [x] 5.1 Add workspace tests for collision-safe Plan branch/worktree creation, recorded base OID, guarded operation failures, bounded roots, and retention in every terminal state.
- [x] 5.2 Extend the workspaces published API to create one PlanRun integration worktree and persist its canonical project, branch, base OID, name, and path without automatic commit, merge, push, reset, or removal.
- [x] 5.3 Add scheduler tests for deterministic eligibility, concurrency fixed to one, transactional claims, predecessor waiting, independent-branch continuation, descendant blocking, and exhausted-run failure.
- [x] 5.4 Implement the persisted serial scheduler and PlanRun status projection, treating verified SubTask success rather than Agent completion as the dependency-release condition.
- [x] 5.5 Integrate attempt dispatch with the PlanRun worktree, OnePiece session execution, usage accounting, timeout classification, and terminal attempt persistence.

## 6. Verification, controls, and restart recovery

- [x] 6.1 Add verification tests for guarded command execution, bounded evidence, multiple acceptance commands, execution errors, failed exits, and dependant release only after complete success.
- [x] 6.2 Implement SubTask verification through the existing guarded operation boundary and persist exit classifications, bounded output summaries, changed-file summaries, and timestamps.
- [x] 6.3 Add state-machine tests for durable pause, resume, cancellation, SubTask timeout, PlanRun timeout, retry, final acceptance, and rejected invalid transitions.
- [x] 6.4 Implement durable controls that persist intent before signaling active generations or operations and stop further claims at safe attempt boundaries.
- [x] 6.5 Add recovery tests that reconcile persisted attempts with session and operation evidence, classify ambiguous in-flight work as interrupted, and require an explicit user recovery action.
- [x] 6.6 Implement conservative startup recovery without resetting, deleting, or silently redispatching work from the retained integration worktree.

## 7. Commands, observability, and logging

- [x] 7.1 Define the task-orchestration published command API for draft generation, Plan/version CRUD, validation, approval, summaries, details, evidence, controls, retry, recovery, and final acceptance with `Result<T, String>` or typed command errors.
- [x] 7.2 Register Tauri commands and native application wiring while keeping SQLite, Git, provider, process, and recovery logic behind Rust context APIs.
- [x] 7.3 Add observability tests for PlanRun/SubTaskRun/Attempt-to-session/operation/execution correlation and rejection or redaction of goals, descriptions, prompts, credentials, raw tool payloads, and raw command output.
- [x] 7.4 Publish Plan lifecycle diagnostics through unified logging with classified levels and safe metadata, and expose user-facing output only through bounded session or operation presentation APIs.

## 8. Frontend contracts and runtime adapters

- [x] 8.1 Add strict TypeScript contracts for Plan summaries, version graphs, validation errors, PlanRun projections, attempt evidence, controls, recovery results, and explicit simulated-runtime metadata.
- [x] 8.2 Extend the frontend Plan service boundary and Tauri adapter with declared native calls; keep all `invoke()` usage out of React components.
- [x] 8.3 Implement deterministic in-memory Web/mock Plan generation, editing, approval, serial state transitions, controls, and evidence shapes without claiming native provider, Git, or SQLite behavior.
- [x] 8.4 Add shared adapter-conformance tests proving equivalent Tauri and Web/mock method signatures, graph/state shapes, validation errors, and control semantics.
- [x] 8.5 Implement bounded PlanRun polling or subscription helpers that fetch summaries and detail projections without transferring full Agent transcripts or historical evidence on every refresh.

## 9. Plan review and execution experience

- [x] 9.1 Add component tests and implement the goal-entry and OnePiece draft-generation state, including readiness and invalid-output errors.
- [x] 9.2 Add component tests and implement a Plan approval view for editing SubTasks, acceptance criteria, limits, order, and dependencies with complete-graph validation feedback.
- [x] 9.3 Add component tests and implement PlanRun progress with task states, safe attempt metadata, pause/cancel/resume/retry/recovery controls, and on-demand evidence inspection.
- [x] 9.4 Add component tests and implement final acceptance and retained-worktree presentation that clearly states no automatic commit, merge, push, or cleanup occurred.
- [x] 9.5 Add Playwright coverage for Web/mock draft generation, edit validation, approval, deterministic serial execution, controls, recovery presentation, and final acceptance.

## 10. Full verification

- [x] 10.1 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check` and resolve all failures.
- [x] 10.2 Run `npm run build` and `npx playwright test` and resolve all frontend, adapter, and UI behavior failures.
- [x] 10.3 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` and resolve all formatting failures.
- [x] 10.4 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml` and resolve all native failures.
- [x] 10.5 Run `openspec validate add-plan-execution-foundation --strict` and `openspec validate --specs --strict`, then record the implementation verification evidence before archival.

## 11. Verification remediation

- [x] 11.1 Complete the typed Plan frontend service boundary for independent validation, version inspection/deletion, and on-demand attempt evidence in both Tauri and Web/mock adapters, including full conformance coverage.
- [x] 11.2 Keep active PlanRun polling bounded by excluding verification evidence from run detail and load evidence only after explicit Attempt inspection, with component, adapter, and repository regression tests.
- [x] 11.3 Make Web/mock scheduling honor dependency success and deterministic topological rank/order, including reordered-task, multi-level DAG, and independent-branch tests.
- [x] 11.4 Rerun targeted and repository verification gates, strict OpenSpec validation, and update the implementation verification report with remediation evidence.
