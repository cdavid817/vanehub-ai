## Why

Skill evolution now has durable evidence, assessment, generation, governance, and orchestration records, but users must inspect several technical screens to understand what happened. VaneHub needs a coherent read-only system activity session and deterministic result projection so autonomous background work remains visible, traceable, and actionable without becoming another mutation channel.

## What Changes

- Add one stable system-owned Skill Evolution session for each canonical workspace and one global session for global Skill activity. These sessions are distinct from interactive Agent sessions and have no Agent id, seats, provider runtime, terminal, composer, or workflow activation.
- Create system sessions lazily from committed evolution activity and expose them in a dedicated System Activity group without mixing them into ordinary active/archived Agent-session counts.
- Make session content append-only and system-authored. Users may view, filter, search safe fields, mark read/unread, change visibility/notification preferences, export sanitized activity, and follow navigation links, but cannot send messages, rename, pin, categorize, archive, delete, or mutate evolution state from the session.
- Add a deterministic result projector that consumes committed evidence, assessment, generation, Curator, Overlay, orchestration, automatic-application, probation, breaker, purge, and recovery events through idempotent receipts and stable high-watermarks.
- Project versioned structured activity items rather than prose: event code, severity, status, safe identities, counts, timestamps, source references, navigation target, supersession relation, and optional bounded Rich Block payload.
- Localize activity at render time from event codes so stored records remain locale-neutral, searchable, and rebuildable.
- Add activity types for runs and stages, evidence/seed readiness, assessment routing, generation/dossier completion, Curator lifecycle and decisions, Overlay application, automatic eligibility/application, probation, circuit breakers, new Skill creation, recovery, retention, and purge.
- Keep projections secondary to authoritative subsystem records. Projection lag, failure, corruption, or rebuild never changes source state, retries mutations, or affects Agent execution.
- Support deterministic rebuild from retained audit records, compare-and-swap projector leases, duplicate suppression, gap detection, supersession events, and bounded catch-up after startup.
- Add per-workspace projection policy for session visibility, minimum displayed severity, attention notifications, digest cadence, read-state retention, detailed activity retention, and export limits.
- Project outcomes simultaneously to the system session timeline, Skill Evolution dashboard summaries, unread badges, and notification requests using one canonical safe event envelope and per-target delivery receipts.
- Keep notifications attention-oriented and deduplicated. Navigation may open run, evidence, dossier, Curator, Overlay history, probation, or breaker details but cannot approve, apply, retry, close a breaker, or revert content.
- Add service operations and matching Tauri/Web adapters for listing system sessions, reading timeline pages, filtering/searching, unread state, projection health, rebuild, preferences, and export.
- Add a read-only system-session experience with stage timelines, status cards, source links, projection-health banners, catch-up state, empty states, and explicit separation from interactive chat.

## Capabilities

### New Capabilities

- `skill-evolution-system-activity`: System-owned read-only activity sessions, canonical result envelopes, deterministic multi-target projection, rebuild/recovery, preferences, retention, export, and safe navigation.

### Modified Capabilities

- `session-management`: Adds separately listed system activity sessions with stable scope identity and read-only lifecycle semantics without changing interactive Agent-session behavior.
- `chat-experience`: Adds a read-only system activity presentation that removes interactive chat controls and renders localized structured evolution items.
- `skill-management`: Adds system-activity timeline, projection health, preference, read-state, rebuild, and export operations through desktop and Web adapters.
- `settings-skill-management-ui`: Adds activity-session navigation, dashboard projection summaries, unread badges, projection health, preferences, rebuild, retention, and export controls.
- `notification-system`: Adds canonical evolution projection delivery, attention/digest policy, deduplication, read-state coordination, and navigation-only result notifications.

## Impact

- Desktop/runtime: adds Rust activity-envelope, projection worker, session/timeline repository, high-watermark, rebuild, read-state, preferences, export, and Tauri command services backed by SQLite.
- Web runtime: adds behaviorally equivalent in-memory system sessions, projection lag/rebuild simulation, preferences, read state, timeline, and export without claiming native background catch-up.
- Frontend: extends session and Skill service contracts and both adapters; React components remain isolated from direct Tauri invocation.
- Data: stores locale-neutral sanitized activity items, source/projection receipts, target deliveries, high-watermarks, read cursors, preferences, rebuild attempts, and export manifests. It does not duplicate raw evidence, prompts, transcripts, diffs, model payloads, terminal output, or secrets.
- Dependencies: consumes durable records from the completed self-evolution planning capabilities. It does not introduce a new Agent, provider call, model session, automatic action, or alternative governance path.
- Logging: projection diagnostics use unified redacted logging; the system activity timeline is a product projection, not a feature-local log file.
