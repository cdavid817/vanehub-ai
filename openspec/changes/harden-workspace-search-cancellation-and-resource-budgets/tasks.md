## 0. Baseline, overlap check, and failing characterization

- [x] 0.1 Read `AGENTS.md`, `openspec/project.md`, the current `session-project-inspection` and `runtime-performance-governance` specs, and every active change touching files, documents, search, remote inspection, runtime admission, or workspace UI.
- [x] 0.2 Map current local/remote directory listing, path search, content search, document discovery, cancellation registry, runtime/API `spawn_blocking`, frontend services/adapters, coverage types, cursor format, ignore rules, and tests.
- [x] 0.3 Run `openspec validate harden-workspace-search-cancellation-and-resource-budgets --strict`; fix change artifacts before code if current main/active changes conflict.
- [x] 0.4 Add deterministic characterization for the A/B same-id interleaving where A's finish removes B, and for abort/drop leaving a running worker or stale registration.
- [ ] 0.5 Add instrumented characterization showing current full candidate collection/full directory sort, work performed beyond result count, dependency-directory traversal, and blocking work launched without per-workspace admission.
- [x] 0.6 Inventory current limits and reason codes. Preserve or tighten them through explicit requirements; do not raise limits or increase concurrency merely to retain old completion behavior.

## 1. Generation-safe cancellation primitives

- [x] 1.1 Add `SearchGeneration`, cancellation cause (`Cancelled`, `Superseded`, owner dropped), token, slot, and RAII `SearchRegistration` in the workspaces application/domain boundary.
- [x] 1.2 Make `begin(search_id)` atomically allocate/replace a generation and signal the previous token before returning the new registration.
- [x] 1.3 Implement compare-remove by generation plus token identity for explicit complete and Drop; remove the old unconditional id-only finish API after all callers migrate.
- [x] 1.4 Keep the guard in the async owner and pass only immutable generation/token clones into blocking or remote workers.
- [x] 1.5 Implement precise explicit cancel of the current slot and optional generation-qualified internal cancel; return stable not-found/already-finished semantics.
- [x] 1.6 Add concurrency/model tests for A/B interleavings, three generations, conflicting cancel/complete, normal error, panic unwind, future abort, late result, and generation wrap.
- [x] 1.7 Add a regression test proving A's Drop/complete cannot remove B and B remains cancellable.

## 2. Budget, coverage, clock, and admission contracts

- [x] 2.1 Define finite `WorkspaceInspectionBudgetLimits`, tracker/snapshot, stop reasons, and one consume/checkpoint API for directories, entries, files, bytes, metadata/canonicalization, candidates, results, depth, deadline, and cancellation.
- [x] 2.2 Use an injected monotonic clock/deadline for process-local work and deterministic virtual/fake time in tests; do not use wall-clock rollback-sensitive TTL logic.
- [ ] 2.3 Standardize `Complete`, `Partial`, `Unavailable`, stable primary reason codes, bounded counter summary, and unknown-code compatibility across Rust DTOs and TypeScript.
- [ ] 2.4 Define `WorkspaceInspectionAdmission` with finite global/per-workspace active limits and finite queue/wait policy; acquire before `spawn_blocking` or remote launch.
- [ ] 2.5 Ensure admission stays held until the actual worker exits after caller cancellation/abort, not merely until the async caller drops its response future.
- [x] 2.6 Add exact-boundary tests for every budget dimension, off-by-one behavior, combined stop reasons, admission busy, permit leak, and structural redaction.
- [ ] 2.7 Assemble clock, budget profiles, admission, and cancellation registry in bootstrap/application services rather than Tauri commands.

## 3. Shared recursive ignore policy

- [ ] 3.1 Inventory and consolidate current hard-coded exclusions across path search, content search, document discovery, local adapters, and remote provider flags.
- [ ] 3.2 Add one `WorkspaceIgnorePolicy` with operation modes for recursive discovery/search versus direct navigation/explicit read.
- [ ] 3.3 Support repository `.gitignore` and `.ignore` rules, negation/order semantics, and bounded default dependency/generated exclusions using one reviewed matcher implementation.
- [ ] 3.4 Preserve explicit direct access to ignored paths subject to existing root/safety/type/size checks; add tests proving ignore is not authorization.
- [ ] 3.5 Add tests for `.git`, `node_modules`, `target`, `dist`, `build`, `coverage`, `.next`, `.nuxt`, `vendor`, Python caches, custom ignores, negated includes, nested rules, and malformed/unreadable ignore files.
- [ ] 3.6 Define and test effective policy identity/version for cursor compatibility without returning sensitive absolute paths or full rules.

## 4. Local path search bounded selection

- [ ] 4.1 Refactor local path search to receive one execution context containing generation/token, budget tracker, ignore policy, and operation metadata.
- [x] 4.2 Count every visited directory/entry and metadata/canonicalization operation before filtering or matching; stop with precise coverage when a limit is reached.
- [x] 4.3 Replace full candidate collection/sort with a bounded top-K/heap/selection structure preserving the current stable ordering and tie-break behavior.
- [x] 4.4 Bound retained candidates independently of returned results and expose instrumentation for tests.
- [ ] 4.5 Add deterministic large-tree tests for no matches, many matches, equal sort keys, deep tree, unreadable entries, ignored trees, cancellation, supersession, deadline, and every relevant budget.
- [x] 4.6 Verify existing search syntax, path normalization, and result DTO compatibility remain unchanged outside explicit coverage additions.

## 5. Local streaming content search

- [x] 5.1 Remove the whole-workspace candidate-file vector and implement one streaming traversal/open/read/match pipeline.
- [x] 5.2 Consume entry/metadata/file/byte/result budgets at the exact operation boundaries and check cancellation/deadline before directories, entry batches, file opens, read chunks, result append, and return serialization.
- [x] 5.3 Preserve current binary detection, file-size handling, snippet/result behavior, and case-sensitivity semantics; do not mix the separate Unicode-offset or handle-confinement scope into this change.
- [x] 5.4 Ensure one growing/large file cannot exceed aggregate byte budget or delay cancellation beyond the configured chunk-checkpoint bound.
- [ ] 5.5 Add instrumented tests proving no full candidate list, bounded current-file/chunk memory, exact byte/file/result caps, partial coverage on unreadable/skipped content, and stale-generation result suppression.
- [ ] 5.6 Add error/fault tests for directory read failure, metadata failure, open failure, mid-read failure, invalid UTF-8/binary content, cancellation during read, and serialization after supersession.

## 6. Directory pagination and document discovery

- [ ] 6.1 Implement bounded-memory immediate directory page selection retaining at most `limit + 1` selected entries plus fixed traversal state.
- [ ] 6.2 Add cursor V2 with version, directory/workspace identity, order mode, last key/tie-break, detectable fingerprint/generation, and applicable navigation-policy identity.
- [ ] 6.3 Return typed invalid/stale cursor results and update page semantics so `has_more`/`truncated` is distinct from incomplete scan coverage.
- [ ] 6.4 Add frontend-compatible decoding/migration for existing cursor fixtures or intentionally invalidate old cursors with a stable restart behavior.
- [ ] 6.5 Refactor document discovery to the shared recursive ignore and budget pipeline and remove tests that require default descent into dependency/generated trees.
- [ ] 6.6 Add tests for very large immediate directories, duplicate names/tie-breaks, directory mutation between pages, wrong-directory/order cursor, malformed cursor, entry/deadline limit, and restart-with-replacement behavior.
- [ ] 6.7 Add document discovery tests for defaults, ignore negation, explicit direct navigation, unreadable trees, metadata/snippet byte budgets, cancellation, and complete/partial semantics.

## 7. Remote inspection parity and cleanup

- [ ] 7.1 Extend remote inspection requests/providers with generation, cancellation, deadline, result/entry/file/byte limits, ignore inputs, and structural count response where supported.
- [ ] 7.2 Acquire admission before launching remote commands/channels and retain it until remote work is confirmed exited or transferred to an existing bounded cleanup owner.
- [ ] 7.3 Implement bounded remote cancellation/termination for cancel, supersede, deadline, and caller drop; discard late output/results for stale generations.
- [ ] 7.4 Map provider-specific truncation, timeout, unreadable, and failure into the common coverage/reason contract without exposing raw remote commands, secrets, or unrestricted paths.
- [ ] 7.5 Align remote recursive ignore behavior with the shared policy to the extent supported; document any provider limitation as typed partial coverage rather than silent divergence.
- [ ] 7.6 Add deterministic fake-provider tests for busy admission, supersession, cancel failure, provider timeout, result/byte limits, late stale output, cleanup completion, and local/remote reason parity.

## 8. Runtime/API and frontend service integration

- [ ] 8.1 Refactor runtime/API search entry points so admission and RAII registration surround blocking/remote work and commands remain DTO-only adapters.
- [ ] 8.2 Prevent stale result delivery by comparing request generation before application-store/event/frontend update.
- [ ] 8.3 Extend the frontend workspace service with generation, coverage, reason, bounded budget summary, and cursor V2 types while preserving existing result fields.
- [ ] 8.4 Update the Tauri adapter only; React components/hooks must not add direct `invoke()` calls.
- [ ] 8.5 Implement deterministic Web/mock cancellation, supersession, budget consumption, ignore fixtures, busy admission, cursor stale/invalid behavior, and structural counters without native-scan claims.
- [ ] 8.6 Update search/document/directory UI to distinguish empty Complete from empty Partial/Unavailable, show localized reason, ignore stale generations, and restart pagination on stale cursor.
- [ ] 8.7 Add all new reason/coverage/cursor messages to every registered locale and pass key/interpolation parity tests.
- [ ] 8.8 Add frontend unit/component tests for rapid query replacement, explicit cancel, unmount/abort, stale result, busy state, each budget notice, Web parity, and stale-directory pagination restart.

## 9. Structural performance, architecture, and documentation

- [ ] 9.1 Add instrumented providers/temporary-tree builders that count visits, opens, bytes, metadata operations, retained candidates, checkpoints, active workers, and queue depth without depending on production timing.
- [ ] 9.2 Add performance gates proving candidate/page memory bounds, exact budget stops, cancellation checkpoint bounds, and global/per-workspace admission limits on large synthetic trees.
- [ ] 9.3 Add/update architecture fitness tests proving workspaces application owns ports/policy, infrastructure owns filesystem/provider calls, runtime acquires admission through public service APIs, and commands/components do not coordinate workers.
- [ ] 9.4 Update developer documentation with generation-safe cancellation, budget semantics, coverage/reason codes, ignore modes, directory cursor V2, admission/backpressure, local/remote parity, and limitations without an index/snapshot.
- [ ] 9.5 Review logs, DTOs, fixtures, and metrics for raw file content, search secrets, unrestricted absolute paths, remote command bodies, or unbounded error lists; enforce redaction and bounded summaries.
- [ ] 9.6 Remove duplicated old limit/ignore/cancel implementations only after all local, remote, and Web callers use the shared contracts.

## 10. Verification

- [ ] 10.1 Run focused workspaces cancellation, path, content, document, directory, remote-provider, runtime-admission, frontend service, Web/mock, cursor, and structural performance tests; record exact counts/results.
- [ ] 10.2 Run `npm run architecture:check` and resolve every context/Tauri/frontend-boundary failure without blanket exceptions.
- [ ] 10.3 Run relevant Playwright and desktop workspace/search flows using fixed large-tree fixtures and explicit partial/cancel/busy assertions.
- [ ] 10.4 Run the full validation command set from `AGENTS.md`: `npm run lint:ci`, `npm run test`, `npm run build`, Cargo fmt/check/clippy/panic-check/test, and `openspec validate --specs --strict`.
- [ ] 10.5 Run `openspec validate harden-workspace-search-cancellation-and-resource-budgets --strict` after all task/spec edits.
- [ ] 10.6 Record Windows, macOS, and Linux as PASSED/FAILED/BLOCKED/NOT RUN for local filesystem behavior, remote provider behavior, cursor behavior, and cancellation; do not infer untested results.
- [ ] 10.7 Compare the final implementation against every scenario, leave unmet tasks unchecked, report chosen default budgets/admission limits with evidence, and document residual O(N) scans and snapshot limitations before archive.
