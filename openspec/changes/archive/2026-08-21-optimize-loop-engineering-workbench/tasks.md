## 1. Contracts and Test Fixtures

- [x] 1.1 Add strict TypeScript models for Loop project choices, branch choices, readiness reports, stable readiness check codes, simulation state, and remediation targets.
- [x] 1.2 Extend the Loop frontend service interface with bounded branch discovery and non-mutating readiness preflight operations without changing existing run mutations.
- [x] 1.3 Add contract tests proving the Loop service surface is implemented consistently by the Tauri and Web/mock adapters.
- [x] 1.4 Extend shared Loop fixtures with enabled, disabled, unavailable-selection, active-run, multi-iteration, recovery-required, and awaiting-acceptance cases.

## 2. Native Discovery and Readiness

- [x] 2.1 Add a read-only branch discovery use case to the `workspaces` context that validates the canonical project root and returns bounded local and remote branch references without invoking a shell.
- [x] 2.2 Add Loop readiness domain/application models with stable check categories and blocking semantics in `agent_runtime`.
- [x] 2.3 Implement readiness aggregation over definition enabled state, active runs, published workspace project/branch validation, stable-id Agent eligibility, structured commands, and path-scope validation without creating a run, worktree, process, or session.
- [x] 2.4 Add Tauri command DTOs, mappers, one-command-per-file handlers, registry entries, and command-safe errors for branch discovery and Loop readiness.
- [x] 2.5 Route native readiness diagnostics through the unified logging/operation boundary with redaction and no Loop-specific log file.
- [x] 2.6 Add Rust domain and application tests for ready, missing project, missing branch, ineligible Agent, invalid command, conflicting scope, disabled definition, active-run race, and zero-side-effect preflight behavior.
- [x] 2.7 Add Rust infrastructure/command tests for bounded branch results, unsafe project rejection, DTO serialization, mapper coverage, and command-safe errors.

## 3. Tauri and Web Adapter Parity

- [x] 3.1 Implement Tauri Loop adapter methods for branch discovery and readiness using only the declared native commands.
- [x] 3.2 Implement deterministic Web/mock project choices, branch choices, and simulated readiness reports through the same service contract.
- [x] 3.3 Add adapter tests proving native transport shapes, Web simulation labels, unavailable saved selections, stable Agent ids, and readiness-versus-start separation.
- [x] 3.4 Update contract snapshots or generated contract documentation for the additive Loop service operations and run `npm run contracts:check`.

## 4. Presentation Models and Queries

- [x] 4.1 Add focused query/mutation hooks for project choices, branches, preflight, direct start, enable/disable, duplicate, and delete with narrow query invalidation.
- [x] 4.2 Add pure selectors for current activity, budget consumption, latest decision, required-check outcomes, change statistics, and recovery guidance.
- [x] 4.3 Add pure consecutive-iteration comparison selectors that report resolved/new failures and only compare change counts when evidence exists on both iterations.
- [x] 4.4 Add unit tests proving absent evidence remains unknown or not evaluated and is never projected as a pass.
- [x] 4.5 Extract new production components and selectors so every changed or added TypeScript/TSX production file remains at or below 300 physical lines.

## 5. Definition Workbench

- [x] 5.1 Replace the empty-run message with a definition overview showing goal, criteria, scope, stable-id role Agents, verification policy, limits, enabled state, recent outcomes, and primary start action.
- [x] 5.2 Add state-aware definition actions for edit, direct start, enable/disable, duplicate-as-disabled-copy, and guarded delete with pending and consequence feedback.
- [x] 5.3 Update the four-step definition dialog to select known Git projects and discovered branches, preserve unavailable saved values visibly, and avoid silently replacing selections.
- [x] 5.4 Add the missing enabled-state control and complete the final review with goal, acceptance criteria, allowed/protected paths, Agents, commands, all limits, worktree behavior, and the mandatory human gate.
- [x] 5.5 Add a preflight dialog or panel that reports ordered readiness checks, remediation, simulation state, retry, and start confirmation while preserving loaded definitions and run history.
- [x] 5.6 Refresh preflight after an authoritative start rejection and retain the user's current selection and editor state.
- [x] 5.7 Add component tests for no-run start, disabled definition, duplicate naming, active-run guards, delete confirmation, unavailable project/branch, passing preflight, blocked preflight, and start-race recovery.

## 6. Run Workspace and Human Decision

- [x] 6.1 Add a persistent run header with status, phase, iteration, elapsed/remaining budget, current activity, simulation badge, and state-primary actions.
- [x] 6.2 Reuse one mutation controller for pause, resume, stop, accept, continue, and reject so pending state prevents duplicate submissions across layouts.
- [x] 6.3 Keep critical run and acceptance actions reachable in the center surface at narrow widths while retaining navigation and inspector drawers with correct focus restoration.
- [x] 6.4 Reduce the inspector to contextual limits, workspace metadata, identifiers, and inspection links after critical controls move to the run surface.
- [x] 6.5 Rework iteration cards to default to outcome, comparison, verification, Verifier, decision, feedback, and recovery summaries, with the chronological evidence list progressively disclosed once.
- [x] 6.6 Add the focused acceptance panel with criterion evidence state, required checks, Verifier advice/findings, change summary, risks, and accept/continue/reject consequences.
- [x] 6.7 Add run-workspace tests for every run status, exhausted continuation budget, multi-iteration comparison, no-progress, recovery-required, raw-evidence disclosure, and mutation failure recovery.

## 7. Localization, Accessibility, and Visual Quality

- [x] 7.1 Add semantically aligned Simplified Chinese and English resources for every new title, field, status, readiness check, remediation, confirmation, empty state, tooltip, accessible name, and frontend-owned error.
- [x] 7.2 Verify heading order, dialog focus trap and return, keyboard operation, live mutation announcements, explicit labels, icon-only accessible names, and non-color status indicators.
- [x] 7.3 Keep all visual changes on shared semantic tokens with compact 8px-based density, stable control dimensions, at-most-8px panel radii, subtle reduced-motion-compatible transitions, and no nested decorative card hierarchy.
- [x] 7.4 Extend responsive tests for desktop three-panel layout and narrow drawers while proving the persistent run actions do not overlap, clip, or require the inspector.
- [x] 7.5 Add Playwright coverage for create/select/preflight/start, active-run control, multi-iteration inspection, and human acceptance in representative desktop and narrow viewports.
- [x] 7.6 Perform visual QA in both `futuristic` and `minimal` styles at desktop and narrow widths, recording overlap, clipping, contrast, focus, and blank-panel results.

## 8. Required Verification

- [x] 8.1 Run `npm run lint:ci`.
- [x] 8.2 Run `npm run test`.
- [x] 8.3 Run `npm run test:coverage` and confirm the coverage thresholds pass.
- [x] 8.4 Run `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 8.5 Run `npm run build`.
- [x] 8.6 Run `npx playwright test` for the UI behavior change.
- [x] 8.7 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [x] 8.8 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [x] 8.9 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 8.10 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 8.11 Run `npm run desktop:unit:test` and `npm run test:desktop`, reporting the current operating system result without extrapolating to other platforms.
- [x] 8.12 Run `openspec validate optimize-loop-engineering-workbench --strict` and `openspec validate --specs --strict`.
