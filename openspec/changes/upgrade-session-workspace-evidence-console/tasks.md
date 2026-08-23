## 0. Baseline, Ownership, and Failing Tests

- [x] 0.1 Read `AGENTS.md`, `openspec/config.yaml`, `openspec/project.md`, every delta spec in this change, and the existing affected capability specs before editing implementation code.
- [x] 0.2 Run `openspec validate upgrade-session-workspace-evidence-console --strict` and resolve change-document errors before implementation.
- [x] 0.3 Inventory the current frontend service methods, Tauri/Web session-workspace adapters, Rust commands, SQLite migrations, Tauri events, and tests for Changes, Terminal History, Documents, Files, Shell, Logs, Traces, Report, Review, Usage, and Basic Info.
- [x] 0.4 Record the current command names and serialized DTO shapes that must remain compatible during the migration.
- [x] 0.5 Add or update architecture tests proving the change adds no Rust bounded-context directory and cross-context access uses published APIs or explicit ports.
- [x] 0.6 Add failing frontend contract tests for the native `remote` Shell descriptor, effective seat query propagation, hunk-scoped decision mutation, and loaded-log preservation after a load-more failure.

Tasks 0.7, 0.8, and 0.9 are deferred test gates. Each one is a test that can only pass once the
group that owns its subject exists, so landing it here would leave the suite red for every
intervening group rather than proving anything. Each is instead due immediately before the task
named below, and blocks only that task.

- [ ] 0.7 Add failing Rust tests demonstrating stable newest-first keyset pagination while newer evidence/log rows are inserted. Deferred test gate, split by subject: the evidence pagination fixture is due before 3.9, and the log pagination fixture is due before Group 8. It does not block Groups 2-7. Tick 0.7 only once both fixtures exist and pass.
- [ ] 0.8 Add a controlled repository fixture in which a generated review patch can be checked with `git apply --check`. Deferred test gate, due before 13.7, which is the task that first renders a patch to check. It does not block Groups 2-12.
- [x] 0.9 Establish test builders for run/trace/span/operation/seat/command/file correlations and complete/indexing/partial/unavailable coverage states. Deferred test gate, due before 3.1, which is the task that first defines the correlation and coverage types these builders construct.

## 1. Immediate Contract and Correctness Fixes

- [x] 1.1 Replace the string-only frontend `ShellCapability` union with the discriminated `ShellRuntimeDescriptor`, including `native`, `remote`, `simulated`, and `unavailable` variants.
- [x] 1.2 Map the existing Rust local and remote Shell results to the new descriptor without changing the current create-shell command name during the transition.
- [x] 1.3 Update Tauri and Web/mock serialization fixtures and contract tests for every Shell runtime descriptor variant.
- [x] 1.4 Pass the selected seat id into Terminal History, Shell, and Logs service queries according to `tab-scope.ts`.
- [x] 1.5 Hide or disable the workspace seat switcher for session-scoped tabs and add an accessible explanation when a tab has no seat scope.
- [x] 1.6 Add tests proving a multi-Agent seat change alters seat-scoped query keys/results and does not reset session-scoped tabs.
- [x] 1.7 Add a dedicated `setCodeReviewHunkDecision` service method and transport DTO; stop routing hunk Accept through `setCodeReviewDecision`.
- [x] 1.8 Implement a Web/mock hunk decision mutation that changes only the selected fixture hunk and labels the mutation simulated.
- [x] 1.9 Add review tests proving hunk Accept leaves the review decision, Git index, and working tree unchanged.
- [x] 1.10 Refactor Logs state so initial-load failure may show a blocking error, but refresh/load-more/live failures preserve already loaded entries and show an inline retry state.
- [x] 1.11 Attach error handling to Shell input and resize promises; display typed connection/process failure instead of silently discarding rejected operations.
- [x] 1.12 Split the current Copy Diff action into Copy Displayed Lines and a disabled placeholder for Copy Standard Patch until Task Group 13 implements the backend operation.

## 2. Frontend Evidence Types, Service Boundary, and Runtime Adapters

- [x] 2.1 Add `src/types/session-workspace-evidence.ts` with branded/validated ids, fidelity, status, coverage, cursor-page, evidence-scope, target, record, summary, and report DTOs.
- [x] 2.2 Add Zod transport schemas for new discriminated unions, opaque cursors, evidence notices, Shell frames, workspace capabilities, and report coverage.
- [x] 2.3 Add `SessionWorkspaceEvidenceService` as a focused frontend service interface and compose it through the existing application service injection path.
- [x] 2.4 Add Tauri adapter methods for evidence summary, record pages/details, evidence subscription, and session-run reports; keep all `invoke()` and native event APIs outside React. Group 2 implements only the injectable Tauri transport client, its serialization, schemas, and fixture transport. No method may call a command that is not yet registered: the production binding returns a stable typed unavailable reason code until its command exists. Evidence native methods are activated in 3.15; report native methods are activated in 10.8.
- [x] 2.5 Add deterministic Web/mock implementations with seeded ids, monotonic sequences, bounded pages, and explicit simulated side-effect metadata.
- [x] 2.6 Add one shared contract-conformance suite and run it against both Tauri serialization fixtures and the Web/mock implementation. In Group 2 the suite runs against the Web/mock implementation and the Tauri fixture transport. The same suite is re-run against the native evidence cases in 3.15 and against the native report cases in 10.8, so activation is proven by the cases that already exist rather than by new ones written after the fact.
- [x] 2.7 Define centralized evidence query keys so session, seat, run, trace, span, operation, command, path, filters, and cursor cannot be omitted accidentally.
- [x] 2.8 Keep production service and adapter files within the project line-size rule by extracting evidence, Shell, log, and workspace-inspection clients as focused modules.
- [x] 2.9 Add synchronized locale keys for coverage states, fidelity, evidence record kinds, reason codes, and cross-panel actions in every registered locale.

## 3. Execution Evidence Domain, Schema, Repository, and API

- [x] 3.1 Add evidence ids, correlation, kind, status, safe payload, redaction receipt, coverage, and domain errors under `execution_observability/domain/`.
- [x] 3.2 Enforce maximum safe-payload size, allowlisted fields, valid status/kind combinations, required session id, and fidelity invariants in domain constructors.
- [x] 3.3 Add an application `RecordExecutionEvidence` use case and query use cases for summary, record page, record detail, and correlation counts. Correlation counts are served through the record-detail query rather than a separate entry point: the counts and the record come from one store read, so a caller cannot receive a record whose related counts were computed against a different moment. A standalone pass-through was removed because nothing called it and an uncalled query is one nobody can vouch for.
- [x] 3.4 Add narrow application ports for evidence repository, clock, id generation, redaction validation, post-commit notice publication, and gap diagnostics.
- [x] 3.5 Add an additive SQLite migration for `execution_evidence_events`, projection tables, coverage/gap metadata, and required indexes.
- [x] 3.6 Implement atomic event insert plus monotonic projection update; publish notices only after the transaction commits.
- [x] 3.7 Enforce idempotency through `(source_context, source_event_id)` and treat identical duplicates as success.
- [x] 3.8 Reject conflicting duplicate source ids, preserve the original row, mark affected coverage partial, and emit one rate-limited redacted diagnostic.
- [x] 3.9 Implement opaque query-bound keyset cursors using timestamp, sequence, id, version, and filter fingerprint.
- [x] 3.10 Reject a cursor reused with different filters as `cursor_filter_mismatch` without returning an unstable page.
- [x] 3.11 Implement deterministic projection replay from journal events and a test-only projection reset/rebuild path.
- [x] 3.12 Add retention maintenance aligned with configured execution-timeline retention; delete expired projection rows consistently without per-event scans.
- [x] 3.13 Publish the narrow evidence recorder/query contract through `execution_observability::api` and keep repository/infrastructure modules private.
- [x] 3.14 Add Tauri commands and command-safe error mapping for evidence summary, record list, record detail, and subscription bootstrap data.
- [x] 3.15 Register new commands in the grouped command registry and add serialized DTO compatibility tests. This activates the evidence native methods added in 2.4: the production Tauri binding stops returning unavailable and invokes the registered commands, and the 2.6 conformance suite runs against those native cases. Report native methods stay typed unavailable until 10.8.

Tasks 3.13-3.15 are implemented and their focused tests pass, but their final verification depends
on the recorder being reachable from a real production caller. Until a producer publishes, the
whole write half of the journal -- the recorder, its ports, the notice publisher, the redaction
gate, and the encoding that feeds them -- is syntactically unreachable in the library build, and
`cargo clippy --workspace --all-targets -- -D warnings` fails on it. That is an accurate report of
the code's state, not a lint artefact, so it must be resolved by connecting a caller rather than by
silencing the lint.

Tasks 4.1-4.3 may therefore be implemented before 3.13-3.15 are ticked, and all six ticked together
once the bridge is live and clippy is clean. The exception covers 4.1-4.3 only: it does not permit
starting 4.4-4.11, whose subject is which events each producer records and with what field
coverage, not whether a path from producer to journal exists.

While resolving this, the following remain prohibited:

- `#[allow(dead_code)]`, `#[allow(unused)]`, or any other suppression of the unreachability report.
- Synthetic evidence: a startup marker, a dummy event, or any record describing work that did not
  happen. An event that exists to satisfy a linter is indistinguishable, once recorded, from an
  observation of real work.
- Widening the crate's public API to make an internal symbol appear reachable. `pub` would say the
  recorder is consumed from outside the crate, which is false; it would be the same suppression in
  a costlier form.
- Constructing a value only to discard it, `if false`, or an unreachable call.

Startup projection replay must be repair-if-needed rather than an unconditional full rebuild. A
projection that already agrees with the journal is rebuilt into itself, so an unconditional replay
buys nothing and costs a full journal scan on every launch -- and a scan whose only purpose is to
give the replay code a caller is exactly the kind of fake wiring this note forbids.

- [x] 3.16 Add domain tests that run without Tauri, SQLite, filesystem, network, or process dependencies.
- [x] 3.17 Add SQLite infrastructure tests for migration from the current schema, transaction rollback, indexes, cursor stability, replay, and retention.

## 4. Evidence Producer Integration and Coverage Gaps

- [x] 4.1 Define producer-owned semantic evidence output ports in Agent runtime, workspaces, operations/review, and sessions use cases that need them.
- [x] 4.2 Add bootstrap adapters that map producer semantic events to `execution_observability::api` inputs without exposing the evidence aggregate to producer domains.
- [x] 4.3 Add a bounded non-blocking publication queue and make evidence failure non-blocking to the owning Agent, Shell, log, review, or usage operation.
- [x] 4.4 Record safe run start/completion and observable tool/delegation lifecycle references from the existing canonical execution path.
- [x] 4.5 Record Session Shell opened/closed and structured command start/completion references when boundaries are observable.
- [x] 4.6 Record safe file-mutation observations after trusted workspace mutations or witnessed snapshot comparison; do not persist file content or full paths.
- [x] 4.7 Record review-level decision and automated verification outcome references. Hunk-level decisions and file Viewed resets are deferred to 13.2 and 13.5: neither has an authoritative store until 13.1 adds one, so a producer here would have to derive them from review-level state, and a derived observation recorded as an observed one is the confusion this journal exists to remove. The evidence contract for both is defined in design.md now so 13.2 and 13.5 publish against a settled shape rather than inventing one.
- [x] 4.8 Record usage-observed references that point to sessions-owned accounting observations without duplicating usage totals in the journal.
- [x] 4.9 Emit a bounded coverage-gap marker after queue overflow or persistence recovery, including counts and safe reason codes only.
- [x] 4.10 Add tests proving producer success is unchanged when the evidence recorder is unavailable or its queue is full.
- [x] 4.11 Add tests proving raw prompts, output, tool payloads, terminal text, code, diffs, secrets, environment values, and absolute paths are rejected or removed before evidence persistence.

## 5. Workspace Evidence Scope, Navigation, Summary, and Tab Lifecycle

- [x] 5.1 Add `WorkspaceEvidenceScopeProvider` scoped to the selected Session Workspace, using React Context and serializable state only.
- [x] 5.2 Implement atomic `navigate(target)` behavior that changes the active tab and evidence scope without resetting unrelated mounted panel state.
- [x] 5.3 Clear run/trace/span/operation/command/path/hunk scope when the selected session changes; preserve per-session panel view state where valid.
- [x] 5.4 Add active-filter chips and Clear Scope actions to panels that consume cross-panel scope.
- [x] 5.5 Add a `WorkspaceTabCapability` registry describing seat mode, live support, and retention policy instead of scattered tab-id conditionals.
- [x] 5.6 Pass `isVisible` to every mounted workspace panel.
- [x] 5.7 Suspend Logs/Traces live subscriptions, Report refreshes, and Files/Documents polling when their mounted panel is hidden.
- [x] 5.8 Detach the Shell xterm view when hidden while preserving the native Shell instance.
- [ ] 5.9 Add one bounded `WorkspaceEvidenceSummary` query for tab badges and Basic Info health rather than mounting every panel query.
- [ ] 5.10 Add accessible badge labels for Changes, Terminal History, Shell, Logs, Traces, and Report, and omit zero badges where the existing UI rule requires it.
- [ ] 5.11 Debounce/coalesce identifier-only evidence notices and invalidate only affected query keys.
- [ ] 5.12 Add frontend tests for navigation from log to span, span to command, command to file, finding to run, and summary row to owning tab.
- [ ] 5.13 Add tests proving hidden mounted panels retain local selection/forms while live work is suspended.

## 6. Terminal History as Execution Records

- [ ] 6.1 Add backend execution-record queries over command/tool/delegation/verification projections with stable cursors and coverage.
- [ ] 6.2 Add a legacy message-history activity adapter without inserting historical `toolUse` rows into the native evidence journal.
- [ ] 6.3 Mark legacy activity `inferred`, identify its message-history source, and expose coverage limitations.
- [ ] 6.4 Refactor `terminal-tab.tsx` into a thin composition component plus record toolbar, virtualized list, row, detail drawer, and legacy-source notice components.
- [ ] 6.5 Add filters for kind, status, seat, run, fidelity, and bounded text search.
- [ ] 6.6 Render command runtime, redacted display, duration, status, exit code/signal, working-directory display, output availability, truncation, fidelity, and coverage without fabricating unavailable fields.
- [ ] 6.7 Render tool, delegation, and verification records with their structured safe fields and source fidelity.
- [ ] 6.8 Add cross-panel actions for Trace, Logs, Files/Changes, Report, and Shell when the corresponding target is available.
- [ ] 6.9 Add page append that de-duplicates by stable record id and preserves loaded rows on failure.
- [ ] 6.10 Virtualize the loaded record list and verify bounded mounted rows with maximum-page fixtures.
- [ ] 6.11 Add empty, partial, indexing, unavailable, and no-filter-match states using synchronized locale resources.
- [ ] 6.12 Add tests for native, proxied, inferred, opaque, running, failed, cancelled, incomplete, redacted, output-unavailable, and partial-coverage rows.

## 7. Retained Multi-Shell Lifecycle

- [ ] 7.1 Add `SessionShellDescriptor`, state, runtime descriptor, output frame, attach snapshot, replay gap, and typed Shell errors to the workspaces domain/application contracts.
- [ ] 7.2 Replace the one-view lifecycle with a native `SessionShellRegistry` keyed by Shell id and indexed by session/seat.
- [ ] 7.3 Implement `list`, `create`, `attach`, `detach`, `write`, `resize`, `rename`, and `close` use cases through application ports.
- [ ] 7.4 Retain UTF-8-safe sequence-numbered Shell frames up to 1 MiB per Shell and insert one gap marker when old content is evicted.
- [ ] 7.5 Serialize concurrent create/attach requests so duplicate default Shells are not spawned for the same requested identity.
- [ ] 7.6 Keep local PTY and remote SSH channel infrastructure behind workspaces ports; preserve independent remote channel lifecycle on pooled transports.
- [ ] 7.7 Close inactive Shells after the configured idle window and close/join all Shell-owned workers during application shutdown.
- [ ] 7.8 Publish state/output notices with sequence and timestamp; never publish unbounded replay in a Tauri event.
- [ ] 7.9 Implement Tauri commands and adapters for all Shell registry operations, keeping existing create/kill commands as temporary delegating compatibility paths until frontend migration passes.
- [ ] 7.10 Implement deterministic Web/mock multiple Shells, detach/attach replay, gap behavior, and explicit close.
- [ ] 7.11 Refactor the Shell tab to render Shell tabs, Add, Rename, runtime/status metadata, and explicit Close confirmation.
- [ ] 7.12 Change component cleanup and hidden-tab handling from close/kill to detach.
- [ ] 7.13 Reattach using `nextSequence`, de-duplicate replay/live frames, and display a gap marker when content was evicted or dropped.
- [ ] 7.14 Keep a foreground-process warning visible before explicit close when the runtime can report it; do not claim process state when opaque.
- [ ] 7.15 Add local registry, remote channel, Web/mock, React lifecycle, and desktop E2E tests proving session/tab switches do not terminate Shells.
- [ ] 7.16 Remove obsolete kill-on-unmount code only after the desktop Shell lifecycle tests pass.

## 8. Operations-Owned Log Query Index and Live Logs

- [ ] 8.1 Define an `operations` application contract for indexed session-log queries, coverage, backfill status, live notices, and safe export preparation.
- [ ] 8.2 Add additive SQLite migrations for the redacted log query index, correlation indexes, source-file checkpoints, gaps, and repair status.
- [ ] 8.3 Publish an already-redacted log record notice from `platform::logging` after durable file append; do not expose unredacted input to the indexer.
- [ ] 8.4 Implement idempotent index insertion keyed by stable record id and source file/offset witness.
- [ ] 8.5 Implement stable keyset pagination and structured filters for level, text, session, run, trace, span, operation, agent, seat, and time.
- [ ] 8.6 Add complete/indexing/partial/unavailable coverage with oldest/newest/indexed-through timestamps and dropped counts.
- [ ] 8.7 Add a bounded asynchronous repair/backfill operation with stable operation id, cancellation, checkpoints, and unified diagnostics.
- [ ] 8.8 Handle rotation, truncation, retention deletion, and configured log-directory changes without stale index rows claiming complete coverage.
- [ ] 8.9 Keep unified redacted log files as the durable export/repair source and keep export behavior compatible.
- [ ] 8.10 Move the implementation of session-log query commands from workspaces-owned file scanning to operations-owned APIs while preserving public command names/DTO compatibility during migration.
- [ ] 8.11 Publish identifier-bounded post-commit live log notices through the Tauri adapter and deterministic Web/mock event stream.
- [ ] 8.12 Upgrade Logs toolbar with Follow/Pause, correlation filters, coverage state, active-scope chips, and Jump to latest.
- [ ] 8.13 Stop automatic viewport movement when Follow is paused or the user scrolls away from the newest edge.
- [ ] 8.14 Insert a live row locally only when current filters can be evaluated safely; otherwise invalidate the first page.
- [ ] 8.15 Keep loaded entries visible on refresh, live, and page-append errors and render inline retry/error status.
- [ ] 8.16 Add tests for live insertion during pagination, cursor/filter mismatch, index repair, rotation, directory change, gap notice, retention, and export consistency.
- [ ] 8.17 Add a maximum-fixture performance test proving the interactive query does not scan unbounded log files or block the shared registry on file I/O.

## 9. Trace Waterfall, Structured Span Kinds, and Evidence Links

- [ ] 9.1 Add `ExecutionSpanKind` and safe evidence-count/link fields to observability timeline DTOs.
- [ ] 9.2 Map pinned OpenTelemetry semantic conventions and documented `vanehub.*` attributes to span kind in Rust; remove React span-name substring classification.
- [ ] 9.3 Derive bounded depth, start offset, completed duration, attempt, delegation, and critical-path metadata without inventing unavailable values.
- [ ] 9.4 Add identifier-only run/span transition notices after committed timeline updates.
- [ ] 9.5 Add deterministic Web/mock running-to-terminal trace transitions.
- [ ] 9.6 Refactor the Traces tab into Run list, waterfall viewport, span row, legend/filter toolbar, and detail drawer components.
- [ ] 9.7 Implement horizontal time scaling/scrolling and vertical virtualization for bounded span sets.
- [ ] 9.8 Add keyboard selection, accessible status/fidelity labels, focus visibility, and narrow-width drawer behavior.
- [ ] 9.9 Add detail sections for Overview, safe Attributes, Events, Logs, Commands, Files, Findings, Usage, and Error.
- [ ] 9.10 Query linked evidence from its owning service using the shared scope rather than embedding raw log/file data in the trace DTO.
- [ ] 9.11 Add live refresh only while Traces is visible and coalesce rapid notices.
- [ ] 9.12 Add critical-path, retry, delegation, failed-span, fidelity, and coverage filters.
- [ ] 9.13 Add optional two-run comparison for status, duration, usage quality, tool counts, failures, and changes without comparing raw content.
- [ ] 9.14 Add unit, accessibility, maximum-span performance, and desktop rendering tests for both visual styles.

## 10. Backend Session-Run Report

- [ ] 10.1 Add report query/result DTOs, section coverage, overview, usage, latency, Agent, tool, command, change, verification, failure, and evidence-link models under the sessions application boundary.
- [ ] 10.2 Add narrow sessions-owned ports for execution evidence, observability timing, operations log summaries, workspace/review summaries, and existing session usage summaries.
- [ ] 10.3 Wire report ports through bootstrap adapters to published context APIs without direct repository or infrastructure imports.
- [ ] 10.4 Implement bounded report scope validation for session, run ids, seat ids, date range, and group-by dimensions.
- [ ] 10.5 Preserve reported, reported-derived, and estimated usage separately and preserve internal-purpose versus user-response consumption.
- [ ] 10.6 Return unknown/partial coverage rather than substituting zero for missing evidence.
- [ ] 10.7 Omit monetary cost or mark it unavailable unless an explicitly versioned provider-pricing observation exists; do not introduce a pricing catalog in this change.
- [ ] 10.8 Add Tauri and Web/mock report methods and contract tests. This registers the session-run report command and activates the report native methods added in 2.4: the production Tauri binding stops returning unavailable, and the 2.6 conformance suite runs against those native report cases.
- [ ] 10.9 Replace React message aggregation in Report with the backend service query; retain a legacy comparison test until parity behavior is understood.
- [ ] 10.10 Build Report sections for Overview, Usage, Latency, Agents, Tools, Changes, Tests, Failures, and Evidence.
- [ ] 10.11 Add scope controls for run, seat, time, and group-by, preserving previous report content while refresh is in flight.
- [ ] 10.12 Add evidence links from every report section to the corresponding workspace tab and filter.
- [ ] 10.13 Add JSON export of the bounded report through a service-backed native destination flow and a clearly simulated/unavailable Web path, if the existing export primitive can be reused without adding arbitrary frontend file writes.
- [ ] 10.14 Add application tests for complete, partial, unavailable, no-usage, child-Agent, retry, test-failure, changed-file, and no-evidence reports.
- [ ] 10.15 Add UI tests proving Report results do not change when message pagination or mounted chat-message count changes.

## 11. Provider-Neutral Local and SSH Workspace Inspection

- [ ] 11.1 Add `WorkspaceTarget`, `WorkspaceInspectionCapabilities`, typed provider errors, and a `WorkspaceInspectionProvider` application port under workspaces.
- [ ] 11.2 Adapt existing confined local file/document/search/Git implementations behind `LocalWorkspaceInspectionProvider` without weakening path, symlink, size, locale, or diff bounds.
- [ ] 11.3 Resolve provider selection from the registered Session binding; never accept a frontend-supplied absolute root as authority.
- [ ] 11.4 Publish only the required remote exec/channel contract through `ssh_connections::api`; do not import its pool or infrastructure modules.
- [ ] 11.5 Implement a versioned static SSH helper protocol that sends bounded JSON over stdin and receives bounded JSON output.
- [ ] 11.6 Probe POSIX remote host, current profile revision/host trust, Python 3 helper support, Git, and ripgrep capabilities and return typed per-feature availability.
- [ ] 11.7 Implement remote realpath confinement, symlink-escape rejection, bounded directory listing, bounded text preview, and deterministic sorting inside the helper.
- [ ] 11.8 Implement bounded remote path/content search through `rg --json` when available and return typed unavailability otherwise.
- [ ] 11.9 Implement remote Git status/diff using argument-array subprocess calls and the existing normalized structured DTOs; preserve index/worktree distinction and locale-independent classification.
- [ ] 11.10 Make remote inspection retries idempotent and revalidate profile revision and host trust; never replay a mutation or Shell command.
- [ ] 11.11 Add deterministic Web/mock local/remote capability fixtures and read-only inspection data.
- [ ] 11.12 Add provider contract tests that run the same list/read/search/status/diff/path-escape cases against local, remote-helper fixture, and Web/mock implementations where applicable.
- [ ] 11.13 Add typed unavailable UI states that preserve Shell access and explain missing helper/Git/ripgrep prerequisites.
- [ ] 11.14 Add an optional isolated SSH integration test gated by explicit environment configuration; keep normal CI independent of external SSH.

## 12. Files and Documents Workflow Upgrade

- [ ] 12.1 Add stable per-directory continuation cursors and query-bound validation to file listing DTOs and providers.
- [ ] 12.2 Add normalized workspace invalidation notices from local watch, remote polling, and execution-evidence file mutations.
- [ ] 12.3 Invalidate only affected tree, preview, document, search, Git, diff, and review query keys; retain current selection when still valid.
- [ ] 12.4 Add Quick Open path search with keyboard navigation, cancellation, stable ordering, provider coverage, and bounded pages.
- [ ] 12.5 Add content search with line, column, bounded redacted snippet, provider coverage, cancellation, and result-to-preview navigation.
- [ ] 12.6 Add Files toolbar actions for Quick Open, Content Search, Refresh, Copy Relative Path, Reveal/Open externally when supported, and Open Shell at directory when supported.
- [ ] 12.7 Upgrade text preview with line numbers, syntax highlighting through existing libraries, in-preview find, line navigation, encoding/newline metadata, and evidence actions.
- [ ] 12.8 Keep the previous file preview visible while a new selection or refresh loads; show stale/refreshing status and preserve another selection after failure.
- [ ] 12.9 Add Recent Documents, document path search, source/preview modes, and heading outline derived from bounded content.
- [ ] 12.10 Reuse existing safe Markdown, link, image, code, math, and Mermaid handling; do not introduce direct HTML execution.
- [ ] 12.11 Add document/file links to related execution runs, commands, Changes, and review findings through the shared evidence scope.
- [ ] 12.12 Add per-provider partial/truncated/unavailable states instead of one ambiguous global partial marker.
- [ ] 12.13 Preserve read-only semantics and omit create/edit/rename/delete/save controls from this change.
- [ ] 12.14 Split Files/Documents production components and hooks to remain within the line-size rule.
- [ ] 12.15 Add maximum-directory, maximum-search, large-preview, invalidation, stale-content, remote-unavailable, keyboard, and visual-style tests.

## 13. Review Hunk State, Viewed Progress, Patch Copy, and Evidence

- [ ] 13.1 Add additive SQLite migrations for review hunk decisions and review file Viewed state keyed to current snapshot witnesses.
- [ ] 13.2 Extend the review aggregate/application service to persist review-level and hunk-level decisions independently, and publish a hunk decision evidence reference after the decision commits. This completes the half of 4.7 that had no authoritative store to observe.
- [ ] 13.3 Reject stale hunk decisions when review, file, or hunk witnesses no longer match and return `stale_witness` without mutation.
- [ ] 13.4 Add `setCodeReviewHunkDecision` Tauri command, Tauri/Web adapters, DTO schemas, and contract tests.
- [ ] 13.5 Add `setCodeReviewFileViewed` and reset a file to unviewed when its snapshot fingerprint changes, publishing a file Viewed evidence reference after the state commits. This completes the other half of 4.7.
- [ ] 13.6 Add review summary fields for viewed/current file counts and unresolved comment/finding counts.
- [ ] 13.7 Add `getCodeReviewPatch` using the existing native structured diff/patch renderer and requiring the current snapshot witness.
- [ ] 13.8 Bound patch output, return its fingerprint, and reject binary, oversized, ambiguous, or stale requests.
- [ ] 13.9 Enable Copy Standard Patch and keep Copy Displayed Lines as a separate action.
- [ ] 13.10 Add Review header progress, Mark Viewed/Unviewed, independent hunk decision controls, and stale-state presentation.
- [ ] 13.11 Link automated findings to run/operation/span evidence and navigate through the shared scope.
- [ ] 13.12 Add tests proving review acceptance and hunk acceptance remain independent, Viewed resets on snapshot change, and no decision action stages or rewrites Git content.
- [ ] 13.13 Add native fixture tests proving generated file/hunk patches pass `git apply --check` for current snapshots and fail closed for stale snapshots.

## 14. Information Panel, Visual Integration, Accessibility, and i18n

- [ ] 14.1 Add the evidence-aware runtime/workspace/Shell/change/verification/diagnostic/usage summary to the existing Basic Info pane through `WorkspaceEvidenceSummary`.
- [ ] 14.2 Make every summary row navigate to its owning workspace tab and preserve the current Session.
- [ ] 14.3 Preserve the existing Basic Info, Token Usage, Skill, optional Members, IM, and Code Index tab set and state semantics.
- [ ] 14.4 Keep inactive information-panel panes mounted but disable queries/subscriptions while inactive unless an in-flight mutation must finish.
- [ ] 14.5 Add compact tab/badge, record list, Shell tabs, Logs toolbar, waterfall, Report, Files, Documents, Review, and Basic Info styles using shared semantic tokens only.
- [ ] 14.6 Verify no hover, loading, active, failure, or badge state changes control dimensions or shifts adjacent content.
- [ ] 14.7 Add synchronized translations for every new label, tooltip, filter, status, coverage state, error, empty state, confirmation, drawer tab, and accessibility name in every registered locale.
- [ ] 14.8 Run and fix i18n resource parity and visible-text guardrails without broad allowlisting.
- [ ] 14.9 Add keyboard navigation and focus management for execution rows, Shell tabs, log follow controls, waterfall rows, detail drawers, quick open, document outline, and review actions.
- [ ] 14.10 Verify icon-only controls have localized accessible names/tooltips and status is not communicated by color alone.
- [ ] 14.11 Perform visual tests in `futuristic` and `minimal` styles at desktop and narrow widths, including partial/error/live states and long localized labels.

## 15. Migration, Backfill, Retention, and Cleanup

- [ ] 15.1 Make every new migration additive, idempotent, centrally ordered, and covered by a current-database upgrade fixture.
- [ ] 15.2 Start evidence capture only for new observable events; keep legacy message activity separate and labeled inferred.
- [ ] 15.3 Start log-index repair asynchronously after startup/configuration without blocking the main thread, React, or unrelated Tauri commands.
- [ ] 15.4 Persist repair checkpoints and resume after restart without duplicating indexed rows.
- [ ] 15.5 Coordinate evidence/log-index retention with source trace/log retention and expose partial coverage when source rows are no longer recoverable.
- [ ] 15.6 Add database capacity/maintenance tests for maximum bounded fixtures and delete expired projections without per-event full scans.
- [ ] 15.7 Remove obsolete offset-only log query internals after indexed query parity and migration tests pass.
- [ ] 15.8 Remove obsolete Shell kill-on-unmount and string capability compatibility paths after frontend and desktop tests pass.
- [ ] 15.9 Remove React message-derived Report computation after backend report parity tests pass; retain only explicitly labeled legacy activity projection.
- [ ] 15.10 Update `src-tauri/ARCHITECTURE.md` with evidence-journal, operations log-index, report composition, and workspace-provider decisions.
- [ ] 15.11 Update developer documentation for evidence fidelity/coverage, Shell attach/detach, remote helper prerequisites, log indexing, and cross-panel navigation.

## 16. Required Verification

- [ ] 16.1 Run `npm run lint:ci`, `npm run test`, and `npm run build`.
- [ ] 16.2 Run `npm run test:coverage` and keep the project coverage policy satisfied.
- [ ] 16.3 Run `npm run contracts:check` and resolve Tauri/Web service contract drift.
- [ ] 16.4 Run `npm run architecture:check` and resolve every context, visibility, service-boundary, direct-invoke, and production-file-size violation.
- [ ] 16.5 Run `npm run desktop:unit:test`.
- [ ] 16.6 Run `npm run test:desktop:session-workspace` against the real desktop artifact and record any platform-specific skip with an existing approved mechanism only.
- [ ] 16.7 Run `npx playwright test` for Web/mock workspace behavior, responsive layouts, keyboard behavior, and both visual styles.
- [ ] 16.8 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [ ] 16.9 Run `cargo check --workspace`.
- [ ] 16.10 Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] 16.11 Run `npm run native:panic:check`.
- [ ] 16.12 Run `cargo test --workspace`.
- [ ] 16.13 Run focused evidence, log-index, Shell, report, remote-provider, review-patch, and migration performance fixtures and record their bounded results.
- [ ] 16.14 Run `openspec validate upgrade-session-workspace-evidence-console --strict`.
- [ ] 16.15 Run `openspec validate --specs --strict`.
- [ ] 16.16 Review `git diff --check`, ensure no generated artifacts/secrets/test SSH credentials were committed, and confirm every completed task has corresponding test evidence before marking it complete.
