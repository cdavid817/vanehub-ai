## Context

See `proposal.md` for motivation and the delta specifications for behavior. This change consumes immutable current assessment attempts from `add-skill-evolution-target-selection-and-quality-gates` and commits only through the application boundary established by `add-skill-overlay-governance`. Assessment supplies evidence, target, risk, confidence, and routing; Overlay governance remains authoritative for effective content, preview, scanning, pinning, trust, CAS, history, usage, and filesystem recovery.

Curator coordinates SQLite governance state with filesystem-backed Overlay transactions. Those stores cannot share one database transaction, so approval requires a recoverable idempotent protocol. The UI also has to prove that the user approved the exact effective diff that was committed, not merely a candidate title or stale draft.

## Goals / Non-Goals

**Goals:**

- Give every approvable assessment a bounded human-review lifecycle.
- Preserve a complete immutable chain from evidence through assessment, draft, preview, decision, and Overlay result.
- Let users author or refine safe non-executable guidance when no generated draft exists.
- Guarantee that approval applies only the exact current diff the user reviewed.
- Fail closed for mutation while remaining isolated from Agent execution and evidence collection.
- Keep desktop and Web/mock contracts behaviorally equivalent.
- Establish governance records that later scheduling and auto-application policy can reference.

**Non-Goals:**

- Generating a mutation draft with an LLM or autonomous Skill-generation Agent.
- Automatically applying low-risk candidates, bulk approving, or approving from notifications.
- Changing assessment targets inside Curator.
- Adding scripts, tools, executable resources, supporting files, or permissions.
- Directly editing base Skill packages or bypassing Overlay trust and pinned state.
- Automatically resuming deferred candidates at a timestamp.
- Implementing multi-user remote identity, role-based access, or two-person approval.

## Decisions

### 1. Use a dedicated Curator application service

Add a Rust `skill_evolution_curation` context with ports for assessment reads, evidence projection, effective Skill snapshots, draft assessment, Overlay preview/application, persistence, trusted actor/time, notifications, and unified logging. Curator never opens Overlay manifests or Skill files itself.

The service has separate command groups:

- intake and supersession;
- queue and candidate queries;
- draft creation and revision;
- preview;
- defer, resume, and reject;
- approval and application recovery;
- policy and retention.

This keeps governance orchestration out of the Overlay security-critical replay engine and prevents UI-specific state from becoming mutation authority.

Alternative considered: add Curator methods directly to the Overlay service. Rejected because assessments and queue lifecycle are not Overlay concerns, while Overlay invariants must remain usable outside Curator.

### 2. Snapshot intake and keep source records authoritative

Assessment completion emits an idempotent intake envelope containing only assessment id, revision, current flag, route, and witness hash. Curator reloads the assessment and sanitized evidence from their authoritative repositories before creating a candidate.

The candidate snapshot contains identifiers, hashes, enumerated check results, bounded explanations, risk/confidence, selected target, and safe evidence references. It does not copy raw execution content or model prompts. Snapshot fields needed for later review remain immutable; current source state is queried alongside them so the UI can distinguish “reviewed then” from “current now.”

Only `advance` and `needs_human_review` create candidates. During this manual-only phase both routes require human approval. Other routes remain visible in assessment history but cannot be turned into mutations through Curator.

Alternative considered: enqueue all routes and let users override them. Rejected because it turns hard quality results into advisory warnings and expands the approval attack surface.

### 3. Model candidate state as transitions plus orthogonal staleness

The persisted state machine is:

```text
pending
  ├─> awaiting_draft ─> ready_for_review ─> applying ─> applied
  │                         │                  └─> apply_failed
  │                         ├─> deferred ─> awaiting_draft | ready_for_review
  │                         └─> rejected
  └──────────────────────────────────────────> superseded
```

`pending` is short-lived intake validation. `awaiting_draft` has no current approvable draft. `ready_for_review` means draft-bound checks pass but does not mean preview remains current. `apply_failed` can request a new preview but cannot reuse approval. Applied, rejected, and superseded are terminal.

Staleness is an orthogonal reason set, not another state:

- assessment or target changed → supersede candidate;
- evidence was purged → redact lineage and block open candidate;
- draft changed → return to draft assessment;
- Overlay/base/pin/trust/conflict changed → require new preview or reconciliation;
- policy changed → re-evaluate decision prerequisites.

Alternative considered: encode each stale combination as a state. Rejected because state explosion obscures the core decision lifecycle.

### 4. Require optimistic concurrency for every transition

Each candidate and draft has a monotonically increasing revision. Mutating commands require expected candidate revision plus draft revision where relevant. Preview and approval add assessment, target, Overlay, effective-content, pin, trust, policy, and scanner witnesses.

The repository updates with a compare-and-swap predicate and returns the current sanitized candidate on conflict. It never performs last-write-wins. Decision commands also have client-generated idempotency keys scoped to candidate and action type so UI retries do not duplicate events.

Alternative considered: lock a candidate while its detail page is open. Rejected because locks survive poorly across crashes and multiple windows and do not protect external Overlay changes.

### 5. Support only two safe draft kinds

Version 1 drafts are:

- `LearnBlockDraft`: bounded Markdown guidance appended through `OverlayLearnBlock`;
- `ExactPatchDraft`: one exact `old_string → new_string` instruction patch with `replace_all` default false.

Both bind to target Skill id, effective revision, Overlay scope, evidence references, bounded rationale, and expected lesson shape. Target and scope come from the candidate and cannot be edited. Project targets use project Overlay scope; global targets default to User Overlay scope unless assessment policy provides an already-valid System governance scope. The UI does not expose System scope escalation.

Draft limits are stricter than Overlay maximums: 8 KiB learned guidance, 16 KiB combined patch strings, 1 draft mutation per candidate, and 2 KiB rationale. Draft input passes evidence sanitizer, injection scanner, UTF-8 and Markdown checks, and Overlay dry validation before persistence. Rejected unsafe content is not stored; only scanner version and reason codes are audited.

Alternative considered: allow arbitrary Overlay documents. Rejected because Curator review should express one evidence-backed lesson and should not become a second general-purpose Overlay editor.

### 6. Reassess the exact draft without generating content

Draft validation projects the draft into a bounded `DraftLessonShape` and reuses the nine deterministic quality gates with draft hash as an additional witness. Optional model judging follows the user's existing evaluation consent but receives only the sanitized draft projection, not raw Overlay or Skill files.

The draft cannot change target selection. If target compatibility fails or the lesson implies a different target, the candidate is blocked and must be reassessed upstream. Executable-content risk cannot be “approved anyway”; because executable draft kinds are prohibited, the user must rewrite the draft as non-executable guidance or reject it.

Every edit creates an immutable draft revision and invalidates its prior assessment and preview. A valid current draft becomes `ready_for_review` only after draft-bound results allow human governance.

Alternative considered: trust the original candidate assessment after arbitrary edits. Rejected because the user could transform a low-risk lesson into unrelated or unsafe guidance after the gate.

### 7. Bind approval to a complete Overlay preview

Curator calls the Overlay service's normal preview method with the current draft. The returned `CuratorPreview` includes:

- base-to-current effective diff;
- current-to-proposed diff;
- base-to-proposed effective diff;
- scanner, size, trust, pin, conflict, and replay status;
- base package, current effective, target revision, Overlay revision, policy, candidate, draft, and assessment hashes.

Curator stores only the preview hash, witnesses, bounded diff projection, and expiry. The full approved input remains the safe current draft. Preview tokens expire after 15 minutes and after any relevant mutation. Approval requires an explicit `confirmed_preview_hash` matching the current token.

The commit path re-runs Overlay validation rather than trusting the preview output. Any resulting diff hash mismatch fails stale; no automatic rebase occurs.

Alternative considered: approve the draft before preview and show the result afterward. Rejected because user consent would not cover the actual effective change.

### 8. Derive approval authority at the native boundary

Version 1 supports one trusted actor class: `local_interactive_user`. Tauri commands derive it from an active application interaction context and native time; actor ids and timestamps supplied by TypeScript are ignored. Background workers have `system` actor class and may enqueue or supersede but never approve.

Web/mock mode uses a deterministic mock interactive actor for behavioral testing but performs no filesystem mutation. A future HTTP adapter must replace this with authenticated server identity without changing decision contracts.

Notifications can only navigate. There are no inline approval/rejection actions, global shortcuts, bulk commands, or model-issued decisions.

Alternative considered: treat any local command invocation as approval. Rejected because background code or a notification action could then acquire mutation authority unintentionally.

### 9. Coordinate SQLite audit and Overlay commit with an outbox saga

SQLite and Overlay filesystem storage cannot commit atomically. Approval therefore uses an idempotent application id:

1. In SQLite, CAS the candidate to `applying`, append approval and application-intent events, and commit an outbox record containing witnesses and application id.
2. Call Overlay commit with that application id and the exact approved draft.
3. Overlay history records the Curator application id as mutation provenance inside its own recoverable transaction.
4. Finalize the candidate as `applied` with Overlay revision/history reference, or `apply_failed` with a safe category.

Recovery scans nonterminal outbox records and queries Overlay history by application id. If Overlay committed but SQLite finalization crashed, recovery marks the candidate applied. If no commit exists, it checks witnesses; stale attempts fail closed and valid explicit intents can be marked failed, never silently replayed without a new user preview and approval. Duplicate Overlay commits are rejected idempotently.

This guarantees durable approval intent exists before mutation and a committed Overlay is always discoverable even if finalization fails.

Alternative considered: write Overlay first and audit afterward. Rejected because a database failure could leave a mutation without a governance decision record.

### 10. Keep reject, defer, and resume explicit

Rejection requires one category (`incorrect_target`, `unsupported_lesson`, `duplicate`, `too_risky`, `not_useful`, or `other`) and permits a sanitized note up to 1,000 characters. It is terminal; reconsideration starts from a new upstream assessment rather than rewriting the decision.

Deferral requires a category and permits a review-after time from 1 to 180 days. The time is informational in this change: it can trigger a notification when an existing maintenance process checks it, but does not resume the candidate. Manual resume validates current witnesses and returns to `awaiting_draft` or `ready_for_review`.

Alternative considered: automatically approve or resume at a deadline. Rejected because time does not resolve evidence, risk, or staleness.

### 11. Store normalized Curator state and hash-chained audit events

Add SQLite tables:

- `evolution_curator_candidates`
- `evolution_curator_candidate_sources`
- `evolution_curator_drafts`
- `evolution_curator_draft_assessments`
- `evolution_curator_previews`
- `evolution_curator_decisions`
- `evolution_curator_events`
- `evolution_curator_applications`
- `evolution_curator_outbox`
- `evolution_curator_policy`
- `evolution_curator_notification_receipts`

Events are append-only and hash chained per candidate with sequence, prior hash, event type, actor class, timestamp, versions, state transition, and sanitized reason. Draft bodies are stored only after validation and encrypted-at-rest behavior follows the existing local database boundary; history events contain hashes and safe summaries rather than body copies.

Applied candidates retain an Overlay link and minimal decision tombstone as long as the Overlay history entry exists. Detailed evidence and draft content follow evolution retention and user purge. Default open-candidate retention is 180 days and terminal detail retention is 365 days; policy can shorten within safe bounds but cannot erase Overlay history through Curator.

Alternative considered: write a separate Curator JSONL log. Rejected by unified logging policy and because transactional lifecycle queries belong in SQLite.

### 12. Version governance policy but keep auto-apply impossible

Workspace policy version 1 includes:

- enqueue routes, fixed to `advance` and `needs_human_review` in this phase;
- rejection/defer reason requirements;
- defer and retention bounds;
- notification enablement and digest preference;
- draft and diff display limits.

The schema contains no writable auto-apply flag. Service input with unknown or future auto-apply fields fails validation rather than being ignored. Later changes must add a distinct policy version and capability with its own safety review.

Policy revisions append audit events and invalidate open previews whose prerequisites changed. Existing applied decisions retain the policy witness they used.

Alternative considered: add disabled auto-apply settings now. Rejected because dormant controls invite assumptions before the run engine and automatic application gate are specified.

### 13. Publish sanitized deduplicated notifications through the existing service

Curator emits structured notification requests after committed state changes. The notification service owns display, localization, expiration, and global scope. A receipt key of candidate id, candidate revision, and event type prevents duplicates during recovery.

Pending notifications include candidate id, safe Skill display identity, risk, route, and navigation target. They exclude user notes, draft text, evidence summaries, model rationale, scanner matches, and failure details. Apply notifications link to the Overlay history view; activating them performs no mutation.

Notification failure is best effort and never rolls back a valid Curator decision or Overlay transaction. It is recorded through unified diagnostics, not a feature-local file.

Alternative considered: make notification delivery part of approval atomicity. Rejected because presentation availability should not determine mutation consistency.

### 14. Expose Curator through service-backed UI modules

Extend `agent-service.ts` with discriminated models for queue pages, candidate detail, draft revisions, preview, decisions, policy, audit events, and stable errors. `tauri-agent-client.ts` is the only frontend layer invoking native commands. `web-agent-client.ts` simulates revisions, conflicts, supersession, preview expiry, pinned refusal, apply recovery, and notification navigation.

The Skill Evolution UI gains a Curator workspace with:

- queue summary and filters;
- evidence and assessment review;
- constrained draft editor;
- three-part effective diff and validation report;
- explicit approval confirmation;
- reject/defer/resume dialogs;
- audit timeline and Overlay history link;
- policy and retention panel.

Components remain below 300 lines and use existing Tailwind, localization, focus, keyboard, and responsive patterns. Candidate actions remain individual; no bulk selection appears.

Alternative considered: add approval controls to each Skill inventory card. Rejected because cards cannot present the evidence and diff needed for informed approval.

### 15. Separate mutation safety from runtime availability

Curator intake and notifications are asynchronous. Agent work and evidence ingestion never await them. Curator queries can fail without changing Agent availability. Mutation commands fail closed on any missing witness, audit error, actor uncertainty, stale state, scanner failure, or Overlay error.

Unified diagnostics contain candidate/application ids, state, duration, and safe error category. They never contain draft bodies, evidence text, notes, diffs, or provider output. Health surfaces count stuck applying records, stale candidates, delivery failures, and audit-chain failures.

The Web adapter never claims a real Overlay filesystem commit; its applied result is explicitly mock-runtime provenance while retaining the same state machine.

Alternative considered: make Curator availability part of general Skill readiness. Rejected because governance downtime must not prevent ordinary Skill consumption.

## Risks / Trade-offs

- [Manual review creates queue backlog] → Add priority filters, retention, deferral, clear readiness states, and later introduce auto-apply only under a separate governed change.
- [Users mistake assessment for a finished patch] → Distinguish candidate, draft, preview, and applied Overlay states throughout the UI.
- [Draft editing bypasses original assessment] → Reassess every immutable draft hash and prohibit target changes.
- [Approval races with Overlay edits] → Bind preview to all witnesses, revalidate at commit, and never auto-rebase.
- [SQLite and filesystem diverge after crash] → Persist application intent first, use idempotent Overlay provenance, and recover from the outbox.
- [Curator becomes an alternate unsafe Overlay editor] → Limit to one learn block or exact patch and reuse every Overlay validation invariant.
- [Unsafe rejected text leaks into logs] → Scan before persistence and audit only reason codes and hashes.
- [Pinned refusal frustrates reviewers] → Show pin state before preview and link to normal Skill governance without offering bypass.
- [Candidate history retains too much personal data] → Normalize safe snapshots, apply bounded retention, cascade evidence purge, and retain only minimal applied tombstones.
- [Notification spam] → Deduplicate per revision/event and support digest preferences.
- [Web mock implies native mutation] → Label mock provenance and maintain contract parity without filesystem claims.
- [Hash-chain corruption blocks review] → Fail closed for decisions, expose repair diagnostics, and preserve source assessments and Overlays independently.

## Migration Plan

1. Complete and verify effective Skill runtime, Overlay governance, evidence pipeline, and assessment prerequisites.
2. Add Curator enums, state machine, witnesses, policies, draft limits, pure transition tests, and disabled schema migrations.
3. Add intake, deduplication, snapshot persistence, supersession, retention, purge, and hash-chained audit events without exposing mutation commands.
4. Add constrained draft creation, sanitization, Overlay dry validation, immutable revisions, and draft-bound assessment.
5. Add Overlay preview orchestration, preview expiry and witness checks, and read-only candidate queries.
6. Add trusted interactive actor derivation, reject/defer/resume, and conflict-safe service commands.
7. Add approval outbox saga, Overlay application provenance, crash recovery, idempotency, and failure/retry behavior behind a disabled feature flag.
8. Exercise crash points between every saga step and verify no mutation lacks durable approval intent and no failure produces duplicate Overlay history.
9. Add Tauri commands, shared frontend contracts, Tauri and Web/mock adapters, notification events, Curator UI, localization, accessibility, and E2E coverage.
10. Enable manual Curator intake and approval after privacy, stale-preview, pin, injection, audit-integrity, recovery, and full project validation pass.

Rollback disables new intake and mutation commands, lets an in-progress outbox recovery reach a consistent terminal state, then hides Curator UI entry points. Existing applied Overlays remain effective and auditable through Overlay history; rollback never edits base packages or automatically reverts approved changes. Pending candidates and drafts remain inert for later re-enable or can be purged under retention policy. Re-enabling verifies audit chains, reconciles outbox records, and revalidates every open witness before allowing preview.
