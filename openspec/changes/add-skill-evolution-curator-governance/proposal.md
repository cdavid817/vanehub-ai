## Why

Assessed evolution candidates need a human-governed path from evidence to a safe, auditable Overlay mutation. Without a Curator boundary, uncertain candidates cannot be reviewed consistently and even high-confidence candidates have no witnessed approval process that preserves user control.

## What Changes

- Add a workspace-scoped Curator queue that snapshots qualifying assessment results, evidence lineage, ranked target, nine quality checks, risk, confidence, recommendation, and current effective Skill witness.
- Enqueue `advance` and `needs_human_review` results for manual governance in this stage; do not enqueue `drop`, `record_memory_only`, or `merge_duplicate` as approvable mutations.
- Add candidate states and conflict-safe transitions for pending, deferred, rejected, awaiting-draft, ready-for-review, applying, applied, apply-failed, and superseded outcomes.
- Support approve, reject, defer, resume, and edit-then-approve with required reasons where appropriate and immutable actor/time/version audit records.
- Add evidence-bound Overlay mutation drafts for exact instruction patches and learned-guidance blocks. Executable files, tool registration, permission expansion, target overrides, and direct base-package edits remain prohibited.
- Require draft sanitization, security scanning, exact-target reassessment, Overlay preview, user-visible effective diff, current revision witnesses, and explicit approval before commit.
- Apply approved drafts only through the existing Overlay mutation service, preserving pinned refusal, trust rules, CAS, size limits, history, usage counters, atomic recovery, and reconciliation behavior.
- Detect stale evidence, assessment, target revision, Overlay revision, policy, or preview witnesses and supersede or return the candidate for review rather than silently rebasing.
- Add Curator policy for queue routing, required decision reasons, deferral limits, candidate retention, and notifications. Automatic application remains disabled and is not configurable in this change.
- Expose queue, candidate detail, draft preview, decision history, policy, and actions through the Skill service boundary with matching Tauri and Web/mock adapters.
- Add a Curator workspace to the Skill Evolution UI with filters, counts, evidence and assessment review, safe draft editing, diff approval, stale/apply-failure recovery, and links to resulting Overlay history.
- Publish sanitized pending-review, decision, apply-success, apply-failure, and supersession events through the existing notification and unified logging boundaries.

## Capabilities

### New Capabilities

- `skill-evolution-curation`: Curator queue intake, candidate lifecycle, evidence-bound mutation drafts, witnessed human decisions, Overlay application, audit history, retention, and governance policy.

### Modified Capabilities

- `skill-management`: Adds scoped Curator queries, policy operations, draft preview, and conflict-safe decision commands through desktop and Web adapters.
- `settings-skill-management-ui`: Adds the Curator workspace, candidate review, draft editing, effective diff approval, history, policy, and recovery states.
- `notification-system`: Adds sanitized, deduplicated Curator review and application notifications with navigation targets.

## Impact

- Desktop/runtime: adds Rust Curator domain services, SQLite queue and audit persistence, assessment intake, draft validation, Overlay orchestration, Tauri commands, and unified-log projections.
- Web runtime: adds matching in-memory/mock queue, transition, conflict, preview, and application-result behavior without filesystem mutation.
- Frontend: extends `agent-service.ts` and both runtime adapters; React remains service-backed with no direct `invoke()` calls.
- Data: stores immutable candidate snapshots, mutation-draft revisions, preview witnesses, decisions, transition events, Overlay result references, policies, and delivery receipts. Raw prompts, terminal output, secrets, and rejected unsafe draft bodies are not persisted.
- Dependencies: requires the effective Skill runtime, Overlay governance, evidence pipeline, and target-selection/quality-gate changes. Later scheduling and automatic-application changes may consume Curator policy and audit data but are not implemented here.
- Security: Curator approval cannot bypass Overlay scanners, trust, pinning, scope, CAS, history, or executable-content restrictions.
