## Context

See `proposal.md` for motivation and the delta specifications for behavior. The self-evolution subsystems already retain authoritative domain records, immutable attempts, audit events, history references, and idempotency receipts. Unified logs remain diagnostics and must not become a control or projection source. The result layer must tolerate independent subsystem retention and storage, including SQLite records and Overlay append-only history.

Existing interactive sessions assume an Agent, seats, mode, workflow state, chat composer, messages, and provider lifecycle. Reusing that entity directly would introduce invalid placeholder Agent identities and risk exposing normal session mutation commands. System activity therefore needs a separate domain model that can share visual navigation patterns without sharing execution semantics.

## Goals / Non-Goals

**Goals:**

- Provide one coherent, stable activity stream for global and workspace evolution.
- Project only committed safe outcomes and preserve source-domain authority.
- Keep storage locale-neutral and rebuildable from retained audit data.
- Coordinate timeline, dashboard, unread state, and notifications from one envelope.
- Make lag, gaps, failures, retention, and rebuild visible to users.
- Preserve append-only history and supersession rather than rewriting past results.
- Maintain behavioral parity across desktop and Web/mock while labeling durability differences.
- Keep every activity interaction non-mutating with respect to evolution.

**Non-Goals:**

- Treating the activity stream as an Agent, chat, or model execution context.
- Replacing subsystem audit records or unified diagnostics.
- Allowing users to respond to, edit, archive, categorize, or delete activity items.
- Executing approval, retry, cancel, apply, rollback, or breaker actions inline.
- Replaying source mutations during rebuild.
- Persisting fully localized prose for each item.
- Projecting raw content or making sensitive source data searchable.
- Delivering operating-system push notifications or remote synchronization.

## Decisions

### 1. Create a distinct SystemActivitySession model

`SystemActivitySession` is not a variant of the interactive `Session` database row. It has:

- stable `session_id`;
- `activity_kind = skill_evolution`;
- `scope_kind = global | workspace`;
- canonical scope id and optional safe display identity;
- current projection generation and sequence;
- unread/attention summary;
- visibility and preference revision;
- created, first-activity, last-activity, and last-projected timestamps.

The id is derived using a versioned namespace hash over activity kind and canonical scope id. Workspace aliases therefore resolve one record; moving to a genuinely different canonical workspace yields a distinct session. Global uses a reserved scope key.

The frontend session navigator receives `interactiveSessions` and `systemActivitySessions` as discriminated collections. Opening activity changes view route, not active workflow session. No fake Agent id, empty seat list, or pseudo interaction mode is stored.

Alternative considered: add `session_kind` to every existing session row and make Agent fields nullable. Rejected because it would weaken many established invariants and require every interactive consumer to handle a non-Agent case.

### 2. Create sessions lazily from committed activity

The projector resolves or creates the system session inside the first activity-item transaction. It does not precreate a session for every discovered workspace. Once created, identity and preferences remain even if retention removes all detailed items; visibility policy controls whether an empty historical session remains in navigation.

Only the native projector can create it. Public create-session input rejects system kinds. System sessions do not participate in pinning, archival, category, deletion, automatic inactivity, worktree metadata, terminal lifecycle, message FTS, or active session recovery.

Alternative considered: create a system session when a project is opened. Rejected because many projects will never use Skill evolution and empty system surfaces would add noise.

### 3. Define one canonical safe result envelope

`EvolutionActivityEnvelopeV1` contains:

```text
event_id
event_code
source_domain / source_id / source_revision / source_sequence
scope_kind / canonical_scope_id
occurred_at / committed_at
severity / status / attention_kind
safe_actor_kind
safe_identities[]
metrics{}
reason_codes[]
navigation{kind, stable_id, optional_child_id}
supersedes_event_id?
payload{schema, structured_fields}
projection_policy_version
content_hash
```

All fields have registries and size limits. Version 1 envelopes are at most 16 KiB; optional payload is at most 8 KiB; identities, metrics, reasons, and links have bounded counts. There is no generic prose or arbitrary JSON extension. Unknown codes or payload schemas stop that event before delivery.

The envelope is the shared input for timeline, dashboard, unread, and notifications. Target renderers may omit fields but cannot enrich from sensitive source stores.

Alternative considered: let every target query source domains directly. Rejected because summaries and notifications would drift and privacy filtering would be repeated inconsistently.

### 4. Map authoritative committed audit records through adapters

Each source domain implements `EvolutionProjectionSource` with:

- bounded scan after an opaque domain cursor;
- verify source record integrity and committed status;
- map a source revision to zero or more versioned safe envelopes;
- report next cursor, source sequence, and retention floor.

SQLite domains scan immutable event/audit tables in `(committed_at, stable_id, revision)` order. Domains that lack an immutable audit event add a small same-transaction activity outbox record. Overlay events use verified append-only history through its existing bounded reader; Curator application events remain the preferred representation when the mutation originated there.

Adapters never parse unified logs, UI notifications, model transcripts, chat messages, or terminal files. Transient retry state that never committed maps to no user activity.

Alternative considered: create a single global event bus and require all prior subsystems to rewrite around it. Rejected because authoritative stores and recovery already exist; adapters plus small transactional outboxes are incremental and auditable.

### 5. Use a durable projector with per-domain cursors

The desktop projector has one global coordinator lease and per-domain progress:

- active projection version and generation;
- source cursor and last sequence;
- last source event hash;
- last successful projection time;
- pending estimate and oldest pending time;
- gap/failure status and retry count.

It processes bounded batches of 100 envelopes or two seconds, whichever comes first, then checkpoints. Startup catch-up receives a 10-second foreground-safe budget and continues on the background scheduler. A domain gap stops only that domain; others advance independently.

The source receipt key is `(source_domain, source_id, source_revision, event_code, projection_version)`. Target receipt adds `(target_kind, target_scope)`. Unique constraints make replays idempotent.

Alternative considered: one global high-watermark. Rejected because a problem in one domain would block all other activity and source stores use different ordering semantics.

### 6. Persist append-only items and explicit supersession

Timeline items are immutable. Corrections, reassessments, regenerated drafts, Curator decisions, reversions, and rebuild outcomes append a new envelope with `supersedes_event_id` or safe relation fields. UI can visually collapse superseded items but history remains available within retention.

Projection mapping changes create a new projection version, not in-place prose edits. Localized presentation changes need no data migration because codes and parameters remain stable.

Alternative considered: update the original timeline row to show latest state. Rejected because users could not reconstruct what was visible or decided at the time.

### 7. Store locale-neutral parameters and render through registries

Frontend `ActivityPresentationRegistry` maps event code, status, severity, and payload schema to localization keys, icon semantics, and read-only Rich Block renderers. Supported payloads are:

- `status_card`
- `stage_timeline`
- `check_summary`
- `metric_summary`
- `navigation_list`
- `supersession_notice`

No `html_widget`, freeform Markdown, raw diff, interactive action block, audio, media, or file payload is accepted. Safe fallback displays event code, timestamp, severity, and a generic unavailable-schema label.

Localization uses all application-supported locales and shared fallback. Safe identity values are escaped as text. Formatting never interprets them as Markdown or HTML.

Alternative considered: render localized strings in Rust at projection time. Rejected because locale changes would require rewriting history and Web/desktop translations could diverge.

### 8. Deliver independently to four projection targets

For each canonical envelope, the coordinator evaluates target policies:

1. `system_timeline`: persist item if it meets timeline severity/retention policy.
2. `skill_dashboard`: update materialized counters/current-state references using envelope idempotency.
3. `unread_state`: increment session sequence and attention summary if applicable.
4. `notification`: create a notification request or digest bucket if threshold permits.

All target deliveries persist receipts. Failure in one target does not roll back committed deliveries to others; retries use the same event id. Dashboard materializations always retain the last envelope id used for each counter/state transition.

Alternative considered: publish notification only after the UI reads the timeline. Rejected because desktop background outcomes need attention even when the session view is closed.

### 9. Keep read state separate and monotonic by default

`SystemActivityReadState` stores user, system session, highest read sequence, optional bounded mark-unread sequence, last seen time, and revision. Reading newest visible activity advances the cursor via max semantics. Rebuild preserves logical source event identity and maps the read cursor using source committed order so restored old items do not become unread.

Unread counts are bounded in UI (`99+`) but exact within the retained session. Attention severity is computed from unread items. Dismissing a notification does not mark timeline read. Opening a notification advances the cursor only after the referenced item is loaded and visible.

Alternative considered: store a read flag on every item. Rejected because rebuild and large timelines would require bulk mutation and concurrent views would conflict more often.

### 10. Provide safe indexed filtering and search

SQLite indexes cover session/sequence, committed time, severity, event code/domain, status, Skill safe id, source run/candidate ids, and attention. Search uses only registered safe identity tokens and localized event-code keyword aliases supplied by the frontend query contract. It does not index payload free text or source evidence.

Pages use opaque `(generation, sequence)` cursors and return completeness and active-generation metadata. A generation change invalidates stale cursors with a typed response rather than mixing old and rebuilt rows.

Alternative considered: reuse interactive message FTS. Rejected because system activity is structured, locale-neutral, and intentionally excludes arbitrary message content.

### 11. Version preferences independently from source policy

`EvolutionActivityPreferences` are scoped to global or canonical workspace and include:

- visible in System Activity navigation;
- minimum timeline severity;
- notification threshold;
- digest mode (`off`, `hourly`, `daily`);
- read-state retention;
- detailed timeline retention from 30 to 365 days, default 180;
- export item/size limits.

Security, integrity, regression, application failure, and breaker events cannot be filtered below timeline `warning` while related authoritative audit exists, though users can hide navigation or change notification delivery. Preferences never alter source retention, Curator queue, Overlay history, or orchestration.

Alternative considered: reuse orchestration policy. Rejected because visibility and notification preferences must not affect whether evolution runs or mutations occur.

### 12. Build attention-oriented notification and digest projection

Notification renderer consumes only canonical envelopes. Immediate attention categories are security/integrity, application failure, probation regression, open breaker, and blocked human review above policy severity. Routine completed runs, healthy probation, and informational candidates enter optional digest buckets.

Digest keys are `(scope, cadence window, activity kind)`. A digest contains safe counts by code/severity, earliest/latest time, highest attention, and navigation to filtered system activity. It does not copy item summaries. Delivery receipts survive catch-up and rebuild so notifications are never replayed.

All notification actions navigate. The notification system remains presentation-oriented and does not become a durable source; projection receipts in native storage record delivery intent/result while frontend notification history retains its existing lifecycle.

Alternative considered: notify every completed stage. Rejected because it would overwhelm users and obscure important governance failures.

### 13. Rebuild using shadow projection generations

Rebuild creates a `projection_generation_id` and scoped source snapshot:

1. capture retained source floors and high-watermarks;
2. scan and map into shadow session items and dashboard materializations;
3. validate event counts, source receipt hashes, ordering, canonical envelope hashes, and required tombstones;
4. map read cursor and preserve notification delivery receipts by source identity;
5. atomically switch the session/dashboard active generation;
6. retire the previous generation after a recovery window.

New source events arriving during rebuild continue into the current generation and are caught up into shadow after its initial high-watermark. Activation requires no unresolved gap. Rebuild has time/item budgets and checkpoints and can be cancelled before activation.

It calls no model, assessment, generator, Curator decision, Overlay operation, or source retry. It is pure projection.

Alternative considered: delete timeline and rebuild in place. Rejected because a failed rebuild would leave users without the last valid activity view.

### 14. Integrate source purge and retention without hiding committed outcomes

Retention removes detailed timeline items and normalized payloads according to preferences, then keeps session summaries and read cursors consistent. Source evidence purge emits a safe purge event and causes source links to become unavailable. It may remove detailed derived items when policy requires.

Applied Overlay or created Skill outcomes retain minimal non-sensitive tombstones while their authoritative history exists: safe Skill identity, action kind, committed time, revision/history reference, provenance class, and unavailable-detail reason. These tombstones cannot reconstruct instructions or evidence.

Alternative considered: keep all activity indefinitely because it is sanitized. Rejected because identifiers and behavior history are still user data and should remain bounded.

### 15. Export deterministic safe projections

Export accepts system session, active generation, filters, format, and item/size limits. JSON contains canonical envelopes allowed by policy; Markdown contains localized renderings plus a manifest with projection version, generation, filters, range, completeness, redaction, and hash. Export uses the normal user-selected file boundary and unified logging for safe result diagnostics.

User-selected files are not automatically deleted by retention. The UI discloses this. Export never follows navigation links to include dossiers, diffs, evidence, or Overlay content.

Alternative considered: export a full self-evolution audit bundle. Rejected because that crosses multiple retention/privacy boundaries and belongs to a separately governed export capability.

### 16. Persist projection state in dedicated normalized tables

Add SQLite tables:

- `evolution_system_activity_sessions`
- `evolution_activity_envelopes`
- `evolution_activity_items`
- `evolution_activity_source_receipts`
- `evolution_activity_target_receipts`
- `evolution_activity_domain_cursors`
- `evolution_activity_dashboard_state`
- `evolution_activity_read_state`
- `evolution_activity_preferences`
- `evolution_activity_digest_buckets`
- `evolution_activity_projection_leases`
- `evolution_activity_rebuilds`
- `evolution_activity_rebuild_checkpoints`
- `evolution_activity_exports`

Envelope rows can be shared by targets and generations through immutable hashes. Payloads use typed bounded JSON validated before storage. Foreign keys and cleanup preserve current generation and tombstone invariants. No feature-local activity JSONL or log is introduced.

Alternative considered: store activity as ordinary chat messages. Rejected because message ownership, FTS, deletion cascades, status, Rich Blocks, and user/assistant roles imply semantics the activity system explicitly does not have.

### 17. Expose typed services and separate view routing

Extend frontend service contracts with system-session summaries, timeline pages, query filters, activity items, read state, health, preferences, rebuild, export, dashboard projection, and stable errors. `tauri-agent-client.ts` owns native invocation; `web-agent-client.ts` simulates source events, lag, gap, rebuild, read cursors, preferences, digest, and export in memory.

The main navigator adds a System Activity group. Selecting it renders `SystemActivityView`, not the interactive chat component with props disabled. Shared visual atoms such as timestamp, card, checklist, and navigation row can be reused, but no composer or Agent lifecycle hooks mount. URL/routes discriminate view kind so reload cannot treat the system id as an Agent session.

Skill Evolution settings embeds dashboard summaries and links to the same session scope. It does not duplicate the full timeline state in React.

Alternative considered: render the system session through the chat page and hide buttons with CSS. Rejected because keyboard handlers, subscriptions, and service commands could still activate interactive behavior.

### 18. Fail open for source systems and fail closed for projection claims

Source commits never wait synchronously for projector delivery. Projection errors write sanitized unified diagnostics and health state. They cannot change source outcomes, trigger retry of domain operations, or affect Agent/Skill runtime.

Envelope validation, privacy, gap, or integrity uncertainty stops projection for the affected event/domain and displays lag. It does not fabricate a generic success item. Rebuild activation fails closed on count/hash mismatch while the prior generation remains readable.

Desktop close-to-tray can continue bounded catch-up under existing background policy. Graceful quit checkpoints cursors and stops new batches. Web/mock projection lives only while the page state exists and labels durability accordingly.

Alternative considered: block evolution commits until visible projection succeeds. Rejected because a presentation outage must never compromise governance or runtime availability.

## Risks / Trade-offs

- [System sessions confuse users with Agent chats] → Separate entity, navigation group, identity, route, and component with no composer or Agent metadata.
- [Projection duplicates audit records] → Store bounded safe envelopes and references only; source remains authoritative and rebuildable.
- [Users mistake delayed dashboard data as current] → Show per-domain freshness, lag, gap, and completeness on every summary.
- [Event taxonomy grows without governance] → Closed versioned registries and unknown-code failure require explicit schema updates.
- [Localization keys drift from stored event codes] → Stable code registry, shared fallback, contract tests, and safe raw-code fallback.
- [Rebuild resends notifications] → Preserve target delivery receipts by source identity and never deliver from shadow generation.
- [Rebuild races with new events] → Capture high-watermarks, catch up shadow, and activate only with no gaps.
- [Read counts change after retention] → Maintain sequence/read invariants and recalculate bounded summaries transactionally.
- [Activity search leaks context] → Index only safe identities and registered code aliases, never arbitrary payload/source text.
- [Notification digest hides urgent events] → Keep security, integrity, regression, breaker, and application failures independently attention eligible.
- [Retention erases explanation of applied mutations] → Preserve minimal non-sensitive tombstones linked to authoritative Skill/Overlay history.
- [Projection worker consumes background resources] → Use bounded batches, per-domain cursors, idle/background budgets, and checkpoints.
- [Web mocks imply persistence] → Return explicit runtime provenance and document page-lifetime limitations.

## Migration Plan

1. Complete and verify all source self-evolution audit/event contracts and identify per-domain committed cursors and retention floors.
2. Add safe envelope registry, source adapters, system-session model, projection tables, privacy validators, and pure mapping tests with the projector disabled.
3. Add timeline-only projection for orchestration, assessment, Curator, and Overlay results; verify idempotency, ordering, source authority, and no sensitive payloads.
4. Add the remaining source domains, dashboard materializations, unread state, safe filtering/search, preferences, and projection health.
5. Add notification target receipts and digest buckets while proving catch-up and rebuild cannot duplicate notifications.
6. Add shadow-generation rebuild, gap handling, bounded catch-up, retention, purge/tombstones, and export.
7. Add service contracts, Tauri commands, Tauri/Web adapters, System Activity navigation/view, dashboard summaries, localization, accessibility, and E2E tests.
8. Run a one-time bounded projection build for retained source events and compare counts/hashes with source audit inventories before enabling navigation.
9. Enable background catch-up, monitor lag and failures, then expose user rebuild and retention controls after recovery testing.

Rollback stops new projection batches, checkpoints all domain cursors, hides System Activity navigation and dashboard projections, and disables rebuild/export commands. Source evolution records, Agent sessions, Curator, Skills, and Overlays remain unchanged. Projection tables may remain for later re-enable or be purged independently after exported data disclosure. Re-enabling validates source adapters and envelope versions, then rebuilds a shadow generation before making activity visible.
