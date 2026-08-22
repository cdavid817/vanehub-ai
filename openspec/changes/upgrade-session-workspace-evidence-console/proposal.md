## Why

VaneHub AI already exposes a broad session workspace: Changes, Terminal History, Documents, Files, Shell, Logs, Traces, Report, and the compact information panel. The individual surfaces are useful, but they currently project different sources of truth and cannot reliably explain one execution end to end.

The most important gaps are correctness and evidence correlation rather than another workspace tab:

- Terminal History is primarily reconstructed from loaded chat-message `toolUse` data, so it is not an authoritative command or process ledger.
- Report is calculated in React from the currently loaded messages, so historical pagination, compaction, child-Agent work, background tools, and native usage records can produce incomplete totals.
- Logs use bounded file scanning with offset pagination, which cannot distinguish a complete corpus from a newest-file subset and can duplicate or skip records while new entries are inserted.
- Trace topology is persisted, but the UI has no waterfall, structured span classification, live update, or links to commands, logs, files, review findings, and usage.
- The global seat selector is rendered for seat-scoped workspace tabs but is not consistently included in their service queries.
- The Shell view owns native Shell termination through React cleanup, so a component lifecycle can terminate work that should be retained by the native workspace runtime.
- The TypeScript Shell capability union omits the native `remote` value returned by Rust.
- Review hunk acceptance is specified as hunk-scoped, but the current frontend path calls the review-level decision mutation.
- Files, Documents, and Changes resolve local session roots, while a remote session can open an SSH Shell but cannot inspect the same workspace through a provider-neutral contract.
- The information panel repeats static metadata but does not summarize current execution health or navigate to the evidence that explains a warning.

These gaps can cause the UI to look complete while presenting incomplete or incorrectly scoped evidence. A coding-Agent workspace must make the fidelity, coverage, and correlation of every result explicit.

## What Changes

- Add a new `session-workspace-execution-evidence` capability that defines an append-only, metadata-only execution evidence journal and bounded projections for commands, tools, file mutations, review/test outcomes, workspace health, and run reports.
- Keep execution topology in `execution_observability`, usage accounting in `sessions`, file/Git/Shell behavior in `workspaces`, and unified logging in `operations`; connect them through published APIs, explicit application ports, and bootstrap-owned adapters rather than creating a new Rust bounded context.
- Add one serializable React workspace-evidence scope and navigation contract so tabs can share session, seat, run, trace, span, operation, command, relative-path, hunk, and timestamp selections without importing one another.
- Make seat scope effective: Terminal History, Shell, and Logs receive the selected seat in their service queries; session-scoped tabs do not pretend to change when a seat selector is changed.
- Upgrade Terminal History into an evidence-backed execution record surface with Commands, Tools, Delegations, and Legacy Activity views. Native records show duration, status, exit data, runtime kind, fidelity, output availability, and links to related evidence. Legacy message projections remain visible but are explicitly marked inferred.
- Retain local and remote Session Shell instances in a native registry across tab and session switches. React detaches from a Shell on hide/unmount and closes it only after an explicit close action. Add multiple Shell tabs, sequence-numbered replay, bounded scrollback, typed runtime capabilities, and deterministic Web simulation.
- Move interactive session-log queries behind an `operations`-owned SQLite query index while preserving the redacted unified log files as the durable export and repair source. Add stable keyset cursors, query coverage, live tail, structured correlation filters, and non-destructive refresh/pagination failures.
- Upgrade Traces with structured span kinds, a virtualized waterfall, live run state, a detail drawer, critical-path and retry/delegation indicators, and links to correlated logs, commands, files, reviews, usage, and report sections.
- Replace React message-derived Report totals with a backend session-run report read model. The report combines execution evidence, observability, usage-quality summaries, changes/reviews, tests, failures, and coverage without claiming provider billing precision.
- Add a provider-neutral, read-only workspace inspection port with local and SSH implementations for capabilities, directory listing, bounded text reads, search, Git status/diff, and change invalidation. Unsupported remote prerequisites return typed capability reasons instead of blank panels or generic errors.
- Upgrade Files with quick open, bounded content search, per-directory continuation, execution-driven invalidation, preview search/line navigation, and evidence links. Upgrade Documents with recent items, outline, source/preview modes, and evidence/reference links. Editing and Git mutation remain out of scope.
- Correct Review Center hunk decisions, add witnessed standard-patch generation, add per-file Viewed state and progress, and correlate review/test/security findings with execution evidence.
- Add compact workspace-tab badges and an evidence-aware Basic Info summary for run state, Shells, errors, changes, review progress, tests, and usage coverage, with navigation to the owning workspace tab.
- Preserve mounted workspace and information-panel state while suspending hidden live subscriptions and background refreshes that are not needed.

## Capabilities

### New Capabilities

- `session-workspace-execution-evidence`: Defines canonical workspace evidence scope, append-only evidence capture, bounded command/tool/file/review/test projections, cross-panel navigation, workspace health summaries, authoritative session-run reports, coverage/fidelity semantics, retention, and desktop/Web service parity.

### Modified Capabilities

- `agent-code-review`: Separate review-level and hunk-level decisions, add witnessed patch generation, file Viewed progress, and execution-evidence links.
- `agent-execution-observability`: Add structured span kinds, live timeline updates, waterfall/detail presentation, critical-path metadata, and cross-signal evidence links.
- `session-log-viewer`: Replace unstable offset paging with coverage-aware keyset queries, add live tail and structured filters, and preserve loaded rows during refresh or pagination failures.
- `unified-log-management`: Add an operations-owned rebuildable query index and live publication after redaction while preserving unified log files and redaction guarantees.
- `session-project-inspection`: Add provider-neutral local/remote inspection, typed capabilities, stable directory continuation, invalidation, quick open/search, richer read-only Files/Documents workflows, and evidence links.
- `remote-terminal-runtime`: Add typed Shell capability descriptors, retained attach/detach lifecycle, multiple Shell instances, sequence-numbered replay, and explicit close semantics for local, remote, and simulated Session Shells.
- `main-layout-ui`: Add effective workspace scope, panel visibility lifecycle, evidence badges, cross-panel navigation, execution-record UI, and an evidence-aware Basic Info summary.
- `usage-statistics`: Make session-run reports consume the existing usage-quality read model and expose coverage without converting estimates into reported Tokens or fabricated cost.

## Runtime Impact

### Tauri desktop runtime

The desktop runtime gains additive SQLite migrations for execution evidence, log indexing, and review hunk/file state; new and extended commands for evidence queries, Shell attach/detach, report queries, remote inspection, and review patch/hunk operations; and bounded native event publication after committed state changes. Existing Tauri command names remain compatible unless a new command is required.

### Browser/Web runtime

The Web adapter gains deterministic in-memory implementations for every new service method. It simulates evidence, Shell replay, log live events, reports, review decisions, and remote-capability states without claiming SQLite, process, filesystem, Git, SSH, or OTLP side effects. No HTTP backend is introduced by this change.

## Architecture Impact

- React components continue to depend only on frontend service interfaces. Tauri `invoke()` and event-listener setup remain confined to Tauri adapters.
- No new directory is added under `src-tauri/src/contexts/`.
- `execution_observability` owns the metadata-only evidence journal and evidence projections.
- `sessions` owns the session-run report application service because provider usage accounting remains a sessions read model.
- `operations` owns the log query index and live log-query contract; `platform::logging` continues to own redacted file persistence, rotation, archival, and active-directory behavior.
- `workspaces` owns local/remote workspace inspection and Session Shell lifecycle. It consumes only the published `ssh_connections::api` facade for remote channels.
- Producing contexts publish narrow semantic events through application-owned output ports. Bootstrap adapters translate those events to the execution-observability evidence API, avoiding domain-to-observability coupling and context dependency cycles.
- Report composition consumes narrow query ports backed by published context APIs; it does not reach into another context's repository or infrastructure module.

## Data and Compatibility Impact

- Migrations are additive and idempotent. Existing messages, traces, unified log files, reviews, usage records, and workspace data remain readable.
- Existing message `toolUse` history is not backfilled as native command evidence. It remains available through a legacy projection marked `inferred` and with explicit coverage limits.
- Existing log files are indexed in bounded background batches. Until indexing reaches the query boundary, the UI reports `indexing` or `partial` coverage rather than claiming completeness.
- Existing Review Sessions start with no hunk decision or Viewed rows; review-level decisions are preserved.
- Live Session Shell processes cannot survive an application restart and are not represented as restored. Persisted provider runtime session ids continue to be handled by the existing Agent Terminal capability, not by Session Shell replay.
- The change does not alter provider billing semantics or attempt invoice reconciliation.

## Non-Goals

- Editing or saving files from Files or Documents.
- Git stage, unstage, commit, branch, merge, or conflict-resolution mutations.
- Persisting raw prompts, complete model output, raw tool/MCP payloads, secrets, environment values, full terminal transcripts, or unrestricted command arguments in the evidence journal or OTLP.
- Replacing the existing Agent Terminal runtime or its provider resume model.
- Automatically replaying commands after local or remote disconnect.
- Adding a new HTTP backend, cloud synchronization service, team review server, or new Rust bounded context.
- Provider price catalogs, exact monetary cost calculation, or provider invoice reconciliation.
- A full IDE editor, language-server editor integration, notebook execution, or arbitrary binary preview.

## Success Criteria

- A user can start from a failed run, error log, command, changed file, or review finding and navigate to the other correlated evidence without manually searching identifiers.
- Hunk acceptance changes only the witnessed hunk decision and never the whole-review decision, Git index, or working tree.
- Switching workspace tabs or sessions does not terminate a retained Session Shell; explicit close does.
- A remote session reports honest read-only inspection capabilities and supports Files/Documents/Changes when its verified remote helper prerequisites are available.
- Log pagination remains stable while new logs arrive, and the UI always states whether the searchable corpus is complete, indexing, partial, or unavailable.
- Report numbers are produced by backend read models, include source-quality coverage, and remain independent of which chat messages happen to be loaded in React.
- Tauri and Web/mock adapters pass the same contract tests; Web behavior is clearly simulated.
- All new UI text is localized in every registered locale, both `futuristic` and `minimal` styles remain usable, and production TypeScript/TSX files remain within the project line-size rule.
