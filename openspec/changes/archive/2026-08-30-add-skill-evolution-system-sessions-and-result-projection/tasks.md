## 1. System Activity Domain and Schema

- [x] 1.1 Add versioned Rust models for system activity sessions, canonical envelopes, payload schemas, source/target receipts, domain cursors, read state, preferences, health, rebuilds, digests, and exports.
- [x] 1.2 Implement stable global/workspace system-session identity from canonical scope without Agent ids, seats, interaction modes, or provider metadata.
- [x] 1.3 Add SQLite migrations for sessions, envelopes, items, receipts, cursors, dashboard state, read state, preferences, digests, leases, rebuilds, checkpoints, and exports.
- [x] 1.4 Implement canonical envelope hashing, strict registries, 16 KiB envelope and 8 KiB payload limits, immutable items, and optimistic versions.
- [x] 1.5 Add migration, serialization, unknown-version, size-limit, collision, and concurrent-version tests.

## 2. Source Projection Adapters

- [x] 2.1 Define the bounded `EvolutionProjectionSource` contract with committed scans, opaque cursors, integrity checks, sequence, retention floor, and safe mapping.
- [x] 2.2 Implement adapters for orchestration runs/stages, evidence/seeds, assessment, generation/dossiers, and Curator lifecycle/decisions.
- [x] 2.3 Implement adapters for Overlay history/applications, automatic eligibility/application, probation/breakers, generated Skill creation, recovery, retention, and purge.
- [x] 2.4 Add transactional activity outbox records only for source domains lacking immutable committed audit events.
- [x] 2.5 Prohibit unified logs, notifications, UI state, messages, model transcripts, and terminal data as projection sources.
- [x] 2.6 Add committed/rolled-back, corrupt source, unsupported version, missing sequence, retention-floor, and source-replay tests.

## 3. Canonical Privacy-Safe Mapping

- [x] 3.1 Implement the closed event-code, severity, status, attention, safe-identity, reason, navigation, and payload registries.
- [x] 3.2 Map each supported source outcome to bounded locale-neutral parameters and structured read-only payloads.
- [x] 3.3 Sanitize every bounded string before envelope persistence, target delivery, unified diagnostics, or export.
- [x] 3.4 Exclude raw prompts, messages, correction bodies, outputs, arguments, paths, diffs, model content, generated drafts, evidence excerpts, and notes.
- [x] 3.5 Add privacy corpus, prompt-injection, unknown navigation, arbitrary JSON, raw diff, HTML, and source-to-envelope golden tests.

## 4. Durable Projector and Catch-Up

- [x] 4.1 Implement the global projection coordinator lease and independent per-domain cursor, sequence, source hash, lag, gap, and failure state.
- [x] 4.2 Process batches of at most 100 envelopes or two seconds and checkpoint after each bounded batch.
- [x] 4.3 Implement source receipt and target receipt uniqueness for deterministic replay suppression.
- [x] 4.4 Detect gaps and stop only the affected domain while unrelated domains continue.
- [x] 4.5 Add a 10-second startup catch-up budget and background continuation without blocking application readiness.
- [x] 4.6 Add lease expiry, duplicate replay, same-timestamp ordering, large backlog, domain isolation, restart, and crash-point tests.

## 5. Timeline, Dashboard, and Unread Projection

- [x] 5.1 Lazily create system sessions inside first eligible timeline delivery and preserve stable identity/preferences after detail retention.
- [x] 5.2 Persist immutable timeline items with deterministic sequence and explicit supersession relations.
- [x] 5.3 Implement idempotent Skill Evolution dashboard materializations for current runs, candidates, generation, Curator, applications, probation, and breakers.
- [x] 5.4 Implement independent system-timeline, dashboard, unread, and notification target policy and receipts.
- [x] 5.5 Implement per-session monotonic read cursors, bounded mark-unread behavior, exact retained counts, and attention severity.
- [x] 5.6 Add multi-target partial failure, supersession, lazy creation, retention-empty session, read races, rebuild read mapping, and `99+` display tests.

## 6. Locale-Neutral Presentation Registry

- [x] 6.1 Add frontend registries mapping event/status/reason codes to localization keys, semantic icons, accessible labels, and supported payload renderers.
- [x] 6.2 Implement read-only status card, stage timeline, check summary, metric summary, navigation list, and supersession notice renderers.
- [x] 6.3 Reject executable HTML, freeform Markdown, raw diff, media, file, and interactive mutation payloads with a bounded safe fallback.
- [x] 6.4 Add all supported locale strings and shared fallback behavior without rewriting persisted envelopes.
- [x] 6.5 Add locale-switch, missing-key, escaped identity, unknown schema, accessibility, and visual-theme tests.

## 7. Filtering, Search, and Navigation

- [x] 7.1 Add indexes and paginated filters for time, severity, domain, status, Skill, run, Curator state, and attention.
- [x] 7.2 Implement safe search over registered event aliases and safe identity tokens without indexing payload/source free text.
- [x] 7.3 Use opaque projection-generation and sequence cursors and return a typed stale-generation response after rebuild activation.
- [x] 7.4 Implement allowlisted navigation descriptors for run, evidence, assessment, dossier, generation, Curator, Overlay, Skill, probation, and breaker details.
- [x] 7.5 Ensure navigation never performs approval, retry, cancellation, application, breaker acknowledgement, or revert.
- [x] 7.6 Add search privacy, stable ordering, pagination, stale cursor, unknown link, and navigation-only tests.

## 8. Preferences, Retention, and Purge

- [x] 8.1 Implement versioned global/workspace visibility, timeline severity, notification threshold, digest cadence, read retention, 30–365 day detail retention, and export limits.
- [x] 8.2 Keep security, integrity, regression, application failure, and breaker timeline events at warning-or-higher retention regardless of routine filters.
- [x] 8.3 Apply detailed activity retention transactionally while preserving current session/dashboard/read summaries.
- [x] 8.4 Integrate source purge by removing drill-down/detail and preserving minimal non-sensitive committed Skill/Overlay tombstones.
- [x] 8.5 Ensure preference and projection retention never mutate authoritative source retention or governance policy.
- [x] 8.6 Add preference conflict, hidden session, reduced retention, purge, tombstone, summary consistency, and source-isolation tests.

## 9. Notifications and Digests

- [x] 9.1 Derive notification requests from canonical envelopes with independent target receipts and threshold policy.
- [x] 9.2 Implement immediate attention eligibility for security, integrity, application failure, regression, breaker, and blocked human review events.
- [x] 9.3 Implement bounded hourly/daily digest buckets with counts, highest severity, time range, and filtered activity navigation.
- [x] 9.4 Coordinate notification opening with read cursor only after the referenced timeline item becomes visible.
- [x] 9.5 Preserve delivery receipts through catch-up and rebuild and keep all notification actions non-mutating.
- [x] 9.6 Add deduplication, digest window, urgent bypass, delayed timeline, dismissal, read coordination, privacy, and navigation tests.

## 10. Shadow-Generation Rebuild

- [x] 10.1 Implement scoped rebuild records, source snapshots/high-watermarks, budgets, checkpoints, cancellation, and shadow projection generations.
- [x] 10.2 Reproject retained sources without model calls, assessments, generation, Curator decisions, Overlay actions, source retries, or notifications.
- [x] 10.3 Validate source/event counts, receipt hashes, ordering, envelope hashes, gaps, required tombstones, and dashboard state before activation.
- [x] 10.4 Catch up source events committed during rebuild and atomically activate only a complete gap-free generation.
- [x] 10.5 Preserve read state and notification receipts by source identity and retain the prior generation through a recovery window.
- [x] 10.6 Add corrupted current generation, failed validation, concurrent new events, cancellation, activation crash, notification non-replay, and rollback tests.

## 11. Safe Activity Export

- [x] 11.1 Implement deterministic JSON export of allowed canonical envelopes and localized Markdown export with manifest metadata.
- [x] 11.2 Bind exports to system session, active generation, filters, range, limits, completeness, redaction, and content hash.
- [x] 11.3 Use the normal user-selected export boundary and disclose that exported files are outside automatic retention.
- [x] 11.4 Prevent exports from following navigation links or including dossiers, evidence, diffs, drafts, Overlay content, or source records.
- [x] 11.5 Add cancellation, path boundary, size/item limit, filter parity, deterministic hash, localization, and sensitive-data tests.

## 12. Service Boundaries and Session Integration

- [x] 12.1 Add discriminated system-session collections without changing interactive Session entity invariants or active workflow selection.
- [x] 12.2 Reject create, rename, pin, archive, category, delete, send, stop, terminal, provider-resume, and chat-configuration commands for system sessions.
- [x] 12.3 Keep ordinary Agent-session counts, search, archival, categories, deletion, discovery, and recovery separate from system activity.
- [x] 12.4 Add typed timeline, filter, search, read-state, health, preference, rebuild, export, and dashboard contracts to `agent-service.ts`.
- [x] 12.5 Add Rust/Tauri commands with typed errors and isolate all native invocations in Tauri frontend adapters.
- [x] 12.6 Implement Web/mock sessions, timelines, lag/gaps, read state, preferences, rebuild, digests, dashboard, and export with explicit in-memory provenance.
- [x] 12.7 Add adapter contracts and mutation-refusal, active-workflow isolation, bulk-delete, ordinary-search exclusion, and Web parity tests.

## 13. System Activity UI

- [x] 13.1 Add a distinct System Activity navigation group for global/workspace sessions with unread and attention badges.
- [x] 13.2 Implement `SystemActivityView` separately from interactive chat so composer and Agent lifecycle hooks never mount.
- [x] 13.3 Add timeline virtualization, filtering/search, read state, structured items, safe navigation, lag/gap banners, empty/loading/error states, and export.
- [x] 13.4 Add Skill Evolution dashboard projection summaries with freshness/completeness and links to scoped activity.
- [x] 13.5 Add preferences, per-domain health, rebuild progress/validation, retention, and export controls.
- [x] 13.6 Keep production modules below 300 lines and add responsive, dark-theme, keyboard, focus, screen-reader, locale-switch, and Web/mock tests.

## 14. Full Verification

- [x] 14.1 Add E2E flows for lazy system-session creation, separate listing/selection, no composer, timeline delivery, unread, search/filter, navigation, and export.
- [x] 14.2 Add E2E flows for lag, source gap, startup catch-up, preferences, digest, source purge, shadow rebuild, failed rebuild, and notification non-replay.
- [x] 14.3 Run privacy, event-registry, projection idempotency, ordering, gap, retention, rebuild, localization, read-state, export, and mutation-refusal corpora.
- [x] 14.4 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 14.5 Run `npm run build` and `npx playwright test` for System Activity, dashboard, notifications, rebuild, and desktop/Web separation.
- [x] 14.6 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 14.7 Run `openspec validate add-skill-evolution-system-sessions-and-result-projection --strict`, `openspec validate --specs --strict`, and repository documentation checks.
- [x] 14.8 Verify projector-disabled, database-unavailable, source-gap, corrupted generation, notification failure, export failure, shutdown, and rollback scenarios leave all Agent and evolution source systems operational.
