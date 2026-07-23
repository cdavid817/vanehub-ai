## 1. Contracts and Persistence

- [x] 1.1 Add synchronized TypeScript Loop definition, run, iteration, evidence, command, control, and event models under `src/types` and `src/contracts`, including contract-conformance assertions.
- [x] 1.2 Extend `AgentService` with Loop definition CRUD, manual start, run reads, pause, resume, cancel, accept, continue, reject, and update-subscription contracts.
- [x] 1.3 Add Rust Loop domain values, aggregates, transition errors, stop-limit invariants, and application view models under the `agent_runtime` context.
- [x] 1.4 Add additive SQLite migrations for Loop definitions, immutable run snapshots, iterations, evidence, and Loop ownership metadata on role sessions.
- [x] 1.5 Implement Loop repositories with atomic definition versioning and run/iteration transition operations, plus migration and repository tests.

## 2. Native Execution Foundations

- [x] 2.1 Publish a guarded workspaces API for collision-safe Loop worktree creation from a canonical local Git project and base branch.
- [x] 2.2 Persist Loop worktree metadata, reject path or branch conflicts before side effects, and test that terminal runs never auto-remove, commit, merge, or delete branches.
- [x] 2.3 Extend the sessions API with non-activating Loop-owned Worker and Verifier session creation and explicit ownership/role metadata.
- [x] 2.4 Exclude Loop-owned sessions from normal and archived navigation by default while preserving direct id-based inspection and usage access.
- [x] 2.5 Enforce Verifier read-only session policy across file, Git, shell, terminal, and Agent tool mutation boundaries with typed errors and tests.
- [x] 2.6 Add exactly-once role-generation terminal completion delivery and cancellation behavior for Loop orchestration.
- [x] 2.7 Implement a structured verification process port with root confinement, executable policy, argument validation, timeout, cancellation, bounded output, and deterministic test doubles.
- [x] 2.8 Associate worktree, role-generation, verification, decision, cancellation, and recovery operations with unified redacted logs and stable run/iteration context.

## 3. Loop State Machine and Orchestration

- [x] 3.1 Implement fixed queued, running, paused, awaiting-acceptance, succeeded, failed, and cancelled run transitions with explicit preparing, acting, verifying, deciding, and finalizing phases.
- [x] 3.2 Implement manual start validation, immutable definition snapshots, one-active-run-per-definition policy, and asynchronous preparation operation creation.
- [x] 3.3 Implement bounded Worker context construction from goal, acceptance criteria, Git state, prior evidence, user feedback, and remaining limits, using a fresh role session per iteration.
- [x] 3.4 Implement ordered deterministic verification evidence collection and prevent acceptance readiness when any required command fails or times out.
- [x] 3.5 Implement fresh read-only Verifier execution with structured pass, revise, or blocked recommendation parsing and validation.
- [x] 3.6 Implement native decision policy that separates deterministic evidence, Verifier advice, user feedback, and terminal outcomes.
- [x] 3.7 Implement diff and required-check failure fingerprints, consecutive no-progress accounting, and objective no-progress termination tests.
- [x] 3.8 Implement maximum-iteration, elapsed-time, consecutive-runtime-error, and phase-timeout enforcement with stable terminal reasons.
- [x] 3.9 Implement pause-after-current-step, immediate cancellation, human accept/continue/reject actions, and duplicate-action protection.
- [x] 3.10 Implement startup reconciliation that marks orphaned nonterminal runs paused with recovery-required detail and resumes only from a durable boundary after user confirmation.
- [x] 3.11 Expose the Loop application API through thin Tauri commands and bootstrap dependency assembly without SQL, Git, process, or policy logic in command handlers.

## 4. Frontend Runtime Adapters

- [x] 4.1 Implement all Loop command mappings and event or polling updates in `tauri-agent-client.ts`, keeping `invoke()` out of React components.
- [x] 4.2 Implement in-memory Web/mock Loop definitions, asynchronous fixed-phase runs, representative evidence, controls, recovery states, and explicit simulated-runtime metadata.
- [x] 4.3 Add Tauri adapter mapping tests, Web/mock behavioral tests, and service contract parity tests for successful, revision, failed, paused, cancelled, and awaiting-acceptance runs.
- [x] 4.4 Add React Query keys and reusable service-backed hooks/models that retain loaded run history while refreshes are pending.

## 5. Loop Center UI

- [x] 5.1 Add the localized icon-only Loops activity entry and preserve mounted session workspace selection and tab state when switching destinations.
- [x] 5.2 Build the Loop Center desktop layout with a bounded definition/run list, flexible timeline, inspector, empty/loading/error states, and internal scrolling.
- [x] 5.3 Build narrow-width list and inspector drawers with keyboard, focus, tooltip, and accessible-name behavior.
- [x] 5.4 Build the four-step Loop creation and editing flow for goal/scope, role Agents, verification/limits, and review/save-or-run.
- [x] 5.5 Build the repeatable structured verification-command editor with program, arguments, relative working directory, timeout, required flag, and localized validation.
- [x] 5.6 Build active-run status, phase progression, limits, elapsed-time, operation status, and expandable iteration evidence presentation.
- [x] 5.7 Build state-aware pause, resume, stop, accept, feedback-and-continue, and reject controls with pending, disabled, confirmation, and failure behavior.
- [x] 5.8 Link role sessions and worktree evidence to existing transcript, Changes, Files, Terminal, Logs, Report, and usage inspection surfaces without showing role sessions in normal navigation.
- [x] 5.9 Add synchronized zh-CN/en resources and semantic-token styling for both `futuristic` and `minimal` themes without hard-coded visible UI text.

## 6. Automated Tests and Visual Verification

- [x] 6.1 Add frontend model, validation, state-control, refresh-retention, localization parity, and adapter-boundary unit tests.
- [x] 6.2 Add React component tests for empty, running, paused, recovery-required, awaiting-acceptance, terminal, and narrow-drawer Loop Center states.
- [x] 6.3 Add Rust domain and application tests using deterministic ports for every valid transition, invalid transition, hard limit, no-progress decision, role isolation, and recovery path.
- [x] 6.4 Add Rust infrastructure tests for SQLite compatibility, guarded worktrees, structured verification safety, process cancellation, bounded output, and unified-log redaction/association.
- [x] 6.5 Add Playwright coverage for creating and manually running a Web/mock Loop, inspecting an iteration, pausing/resuming, continuing with feedback, accepting, rejecting, and preserving the session workspace across navigation.
- [x] 6.6 Capture and inspect representative desktop and narrow screenshots in both visual styles for clipping, overlap, contrast, stable control sizing, drawers, and nonblank run content.

## 7. Final Verification

- [x] 7.1 Run `npm run lint` and resolve all frontend lint failures.
- [x] 7.2 Run `npm run test` and resolve all frontend unit/component failures.
- [x] 7.3 Run `npm run build` and resolve all strict TypeScript and production build failures.
- [x] 7.4 Run `cargo test --manifest-path src-tauri/Cargo.toml` and resolve all native, migration, contract, and architecture test failures.
- [x] 7.5 Run `cargo check --manifest-path src-tauri/Cargo.toml` and `cargo clippy --manifest-path src-tauri/Cargo.toml` and resolve all Rust errors and warnings required by project policy.
- [x] 7.6 Run `npm run tauri -- info` and record the desktop toolchain/environment result.
- [x] 7.7 Run `openspec validate add-loop-engineering-runtime --strict` and `openspec validate --specs --strict`, then record the complete implementation verification results in the change artifacts.
