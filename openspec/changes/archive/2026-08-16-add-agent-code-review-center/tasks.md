## 1. Native review domain and persistence

- [x] 1.1 Add `sessions` review domain models and invariants for sessions, files, fingerprinted anchors, comments, findings, decisions, bounds, and stale transitions with deterministic domain tests.
- [x] 1.2 Add application ports/use cases for review create/recover/query, comment/finding/decision mutations, feedback preparation, and action-result projection without concrete SQLite, Git, logging, or Tauri dependencies.
- [x] 1.3 Add an additive transactional SQLite migration, repositories, indexes, row/domain mappings, restart recovery, migration compatibility, rollback-tolerance, and failure-injection tests for review state.
- [x] 1.4 Publish the narrow `sessions` review API and assemble its dependencies in bootstrap without private cross-context imports.

## 2. Bounded workspace Git review operations

- [x] 2.1 Extend `workspaces` models/ports with bounded review snapshots, file/hunk/context fingerprints, binary/oversize/truncation metadata, and deterministic single-pass parsing tests.
- [x] 2.2 Implement canonical path/traversal/symlink confinement and sorted working-tree/file witnesses for tracked, added, deleted, renamed, and bounded untracked text files with negative security tests.
- [x] 2.3 Implement exact guarded whole-file and reverse-hunk revert under a workspace mutation guard, including confirmation/permission input, current-witness checks, zero-fuzz application, atomic failure behavior, and unrelated-change preservation tests.
- [x] 2.4 Add maximum-bound benchmark/structural measurements proving linear parsing, fingerprinting, matching, and bounded allocations without fragile wall-clock assertions.

## 3. Native coordination, commands, logging, and operations

- [x] 3.1 Coordinate `sessions`, `workspaces`, `permissions`, `operations`, and Agent runtime through published APIs for review opening, guarded revert, feedback delivery, and allowlisted automated actions.
- [x] 3.2 Normalize Review Agent, Tests, and Security Checks terminal results into bounded findings while preserving page-visible action output and rejecting invalid finding payloads.
- [x] 3.3 Emit metadata-only redacted review lifecycle diagnostics and add negative tests proving code, diffs, comments/findings, prompts, secrets, paths, and raw output cannot reach persisted logs.
- [x] 3.4 Add thin Tauri commands/DTO mapping, invoke registration, stable operation ids for variable-duration work, command-safe errors, serialization compatibility tests, and cancellation/timeout coverage.

## 4. Frontend service contract and adapters

- [x] 4.1 Add strict shared TypeScript review models and Agent service methods for lifecycle, bounded files, comments/findings/decisions, feedback, actions, receipts, stale/error states, and subscriptions without `any`.
- [x] 4.2 Implement the Tauri adapter mappings only in the runtime adapter layer and add declaration/DTO contract tests.
- [x] 4.3 Implement deterministic asynchronous Web/mock review create/recovery, anchor staleness/relocation, comments/findings/decisions, simulated revert, feedback, and action states without native side effects.
- [x] 4.4 Add parity and negative contract tests proving Tauri/Web method and shape agreement and honest `simulated` semantics.

## 5. Review Center user interface

- [x] 5.1 Refactor Changes into small review hooks/state utilities that retain loaded data during refresh, navigate files, track drafts/selections, relocate stale anchors, and avoid quadratic row reconstruction; add Vitest coverage.
- [x] 5.2 Build the responsive Review Center file rail, summary, unified/split bounded diff, line/hunk selection, next/previous navigation, copy, loading/error/empty/binary/oversize/truncation, and stale states using shared semantic tokens.
- [x] 5.3 Add accessible inline comment editing/resolution/selection, decision controls, destructive confirmations, guarded hunk/file revert receipts, and Web simulated-state labels with component tests.
- [x] 5.4 Add structured feedback review/acknowledgement and automated Review Agent/Tests/Security actions with operation progress, output, normalized findings, cancellation, and error recovery.
- [x] 5.5 Add every user-visible key to all registered locale resources, preserve hard-coded text and i18n parity guardrails, and keep each production TS/TSX file at or below 300 lines.

## 6. End-to-end, visual, desktop, security, and performance acceptance

- [x] 6.1 Add Playwright coverage from Session Changes through three-file listing, inline comment, feedback send, stale anchor, and simulated hunk revert.
- [x] 6.2 Add stable visual assertions/screenshots for `futuristic` and `minimal` at desktop and narrow widths, checking overlap, clipping, contrast, focus, comment usability, collapsible rail, and recoverable diff overflow.
- [x] 6.3 Extend desktop E2E to create a real temporary Git repository, open its review, read changed files/diff, exercise a guarded hunk revert, and verify stale external-edit rejection.
- [x] 6.4 Run focused Rust, Vitest, contract, Playwright, visual, desktop E2E, security negative, and performance benchmark suites and record evidence in this change before marking the task complete.

## 7. Repository quality gates and final verification

- [x] 7.1 Run `npm run lint:ci`, `npm run test`, `npm run build`, and `npm run test:coverage`.
- [x] 7.2 Run `npm run contracts:check`, `npx playwright test`, `npm run desktop:unit:test`, and `npm run test:desktop`.
- [x] 7.3 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 7.4 Run `openspec validate --specs --strict` and `openspec validate add-agent-code-review-center --strict`, verify task/spec/design coherence, and record Linux desktop status plus Windows/macOS as `NOT RUN` unless actually executed.

## Verification evidence

- Frontend quality: `npm run lint:ci`, `npm run test` (269 files, 1246 tests), `npm run build`, and `npm run test:coverage` passed; coverage was 70.22% statements, 66.42% branches, 65.86% functions, and 74.30% lines.
- Contracts and policy: `npm run contracts:check`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run desktop:unit:test` passed.
- Browser/UI: `CI=1 PLAYWRIGHT_PORT=5184 npx playwright test` passed 123/123 tests. Review Center behavior plus futuristic/minimal desktop/narrow visual runs passed 5/5 and emitted screenshots.
- Native: exact `cargo fmt`, `cargo clippy`, `cargo test`, and `cargo check` gates passed. The Rust suite passed 3303 lib tests plus command-contract, migration, Git safety/revert, security-negative, structural performance, Architecture Fitness, and integration tests.
- Desktop Smoke: Linux `PASSED` using the real Tauri/WebKitGTK client and a real temporary Git repository; Windows `NOT RUN`; macOS `NOT RUN`.
- OpenSpec: main specs (128/128) and `add-agent-code-review-center` passed strict validation before implementation and after verification.
