## Purpose

Defines the human governance boundary that turns assessed Skill-evolution evidence into reviewable, witnessed Overlay mutations while preserving safety, auditability, and explicit user control.

## ADDED Requirements

### Requirement: Curator candidate intake
The system SHALL create Curator candidates only from current immutable assessments whose route is `advance` or `needs_human_review`, and SHALL snapshot the assessment, sanitized evidence lineage, target revision, confidence, risk, quality checks, and policy witness.

#### Scenario: Advance result arrives
- **WHEN** a current assessment recommends `advance`
- **THEN** the system enqueues it for manual governance without applying any mutation

#### Scenario: Non-approvable route arrives
- **WHEN** an assessment recommends `drop`, `record_memory_only`, or `merge_duplicate`
- **THEN** the system records its route but does not create an approvable Curator candidate

#### Scenario: Superseded assessment arrives
- **WHEN** intake receives a non-current assessment attempt
- **THEN** the system rejects intake with a stable superseded reason

### Requirement: Candidate identity and deduplication
The system SHALL use assessment revision and target witness to create an idempotent candidate identity and SHALL maintain at most one current candidate for the same complete intake witness.

#### Scenario: Intake is delivered twice
- **WHEN** the same assessment witness is submitted repeatedly or concurrently
- **THEN** the system returns the existing candidate without duplicating queue or audit entries

#### Scenario: Assessment revision changes
- **WHEN** reassessment creates a new current result
- **THEN** the system supersedes the old open candidate and creates a separately linked candidate if the new route is approvable

### Requirement: Curator candidate lifecycle
The system SHALL enforce versioned transitions among `pending`, `awaiting_draft`, `ready_for_review`, `deferred`, `rejected`, `applying`, `applied`, `apply_failed`, and `superseded`. Terminal or superseded candidates MUST NOT return to an approvable state.

#### Scenario: Candidate lacks a mutation draft
- **WHEN** an eligible assessment is enqueued without a valid draft
- **THEN** the candidate enters `awaiting_draft`

#### Scenario: Invalid transition requested
- **WHEN** a caller attempts to approve a rejected, applied, or superseded candidate
- **THEN** the system rejects the transition without changing history

### Requirement: Immutable candidate snapshot
Each candidate SHALL retain immutable references to its source seed revision, assessment attempt, target Skill and effective revision, Overlay scope, risk, confidence, routing reason, and sanitized evidence snapshot.

#### Scenario: Source evidence later changes
- **WHEN** evidence is corrected, purged, or reassessed after candidate creation
- **THEN** the candidate preserves safe lineage references, becomes stale or redacted as appropriate, and is not silently rewritten

### Requirement: Evidence-bound mutation drafts
The system SHALL support versioned Curator drafts only for an `OverlayLearnBlock` or an exact-match `OverlayPatch`, bound to one assessed target and Overlay scope. A draft SHALL include evidence references, rationale, expected effective change, and current base and Overlay witnesses.

#### Scenario: User authors learned guidance
- **WHEN** the user creates bounded non-executable guidance for an awaiting-draft candidate
- **THEN** the system stores a sanitized draft revision tied to that candidate and target

#### Scenario: Target override attempted
- **WHEN** draft input names a different Skill, revision, or scope
- **THEN** the system rejects the draft and requires a new assessment rather than overriding the selected target

### Requirement: Prohibited Curator draft content
Curator drafts MUST NOT add executable files, scripts, tool registration, permission expansion, arbitrary commands, supporting-file payloads, direct base-package edits, or mutation types outside learned guidance and exact instruction patches.

#### Scenario: Executable draft submitted
- **WHEN** draft content or metadata requests executable behavior or a prohibited mutation type
- **THEN** the system rejects it before persistence and stores only a sanitized rejection event

#### Scenario: Unsafe instruction pattern submitted
- **WHEN** a draft fails Overlay injection or unsafe-content scanning
- **THEN** the system rejects it and does not place the unsafe body in logs or audit history

### Requirement: Draft revision and reassessment
Every draft edit SHALL create a new immutable draft revision, run privacy and Overlay validation, and obtain an assessment result bound to the exact draft hash before it becomes `ready_for_review`.

#### Scenario: User materially edits guidance
- **WHEN** an existing ready draft is changed
- **THEN** prior preview and approval eligibility are invalidated until the new draft revision is reassessed and previewed

#### Scenario: Edited draft fails quality review
- **WHEN** draft-bound reassessment produces a hard stop or non-approvable result
- **THEN** the candidate remains non-approvable and shows the blocking checks

### Requirement: Witnessed Overlay preview
Before approval, the system SHALL run the Overlay service's complete preview pipeline and return the base-to-current, current-to-proposed, and base-to-proposed effective diffs plus scanner, trust, conflict, pinned, size, and revision results.

#### Scenario: Preview succeeds
- **WHEN** the current draft and all witnesses pass Overlay preview
- **THEN** the system issues a short-lived preview witness bound to the candidate, draft, assessment, target, Overlay revision, effective hashes, and diff

#### Scenario: Preview detects drift
- **WHEN** the base, target, Overlay, pin, trust, or conflict state differs from the candidate witness
- **THEN** the system withholds approval and requires reassessment or reconciliation as appropriate

### Requirement: Explicit human approval
Approval SHALL require an authenticated interactive local-user action, the candidate and draft revisions, the current preview witness, and confirmation of the displayed effective diff. System processes, model evaluators, notifications, and background tasks MUST NOT issue approval.

#### Scenario: User approves displayed diff
- **WHEN** the user confirms a valid current preview
- **THEN** the system records the approval decision and attempts the exact witnessed Overlay mutation

#### Scenario: Approval lacks current preview
- **WHEN** approval omits, expires, or mismatches the preview witness
- **THEN** the system rejects approval without applying a mutation

### Requirement: Overlay-only application
Approved drafts SHALL be committed exclusively through the Overlay mutation service and SHALL preserve its scan, trust, pin, CAS, size, history, usage, transaction-recovery, and reconciliation invariants.

#### Scenario: Approved learned guidance applies
- **WHEN** approval witnesses remain current and the Overlay transaction commits
- **THEN** the candidate becomes `applied` and references the committed Overlay revision and history event

#### Scenario: Skill is pinned
- **WHEN** pin state blocks the approved mutation
- **THEN** application is refused and Curator cannot bypass or automatically unpin the Skill

### Requirement: Stale approval prevention
The system SHALL revalidate candidate, assessment, draft, target, policy, pin, base, effective, Overlay, and preview witnesses immediately before commit and SHALL never silently rebase an approved mutation.

#### Scenario: Overlay changes after preview
- **WHEN** another mutation advances the Overlay revision before approval commits
- **THEN** the system rejects the stale approval and returns the candidate for a new preview

#### Scenario: Assessment is superseded during review
- **WHEN** a newer current assessment changes the target or route
- **THEN** the open candidate becomes `superseded` and cannot apply

### Requirement: Reject decision
The system SHALL allow the user to reject an open candidate with a required bounded reason category and optional sanitized note, and rejection SHALL be terminal and auditable.

#### Scenario: User rejects candidate
- **WHEN** the user supplies a valid rejection reason
- **THEN** the candidate becomes `rejected`, pending previews are invalidated, and no Overlay mutation occurs

### Requirement: Defer and resume decisions
The system SHALL allow a user to defer an open candidate with a required reason and optional review-after time within policy limits, and SHALL require an explicit user resume before further review in this change.

#### Scenario: Candidate is deferred
- **WHEN** the user defers a candidate
- **THEN** it leaves the active queue, retains its draft and history, and does not auto-resume at the review-after time

#### Scenario: User resumes deferred candidate
- **WHEN** the user resumes a non-stale deferred candidate
- **THEN** it returns to `awaiting_draft` or `ready_for_review` according to current draft eligibility

### Requirement: Apply failure and retry
Application failure SHALL leave the prior Overlay unchanged, record a sanitized stable failure category, and move the candidate to `apply_failed`. Retrying SHALL require current witness validation and a new preview and user confirmation.

#### Scenario: Overlay transaction fails
- **WHEN** the approved commit cannot complete atomically
- **THEN** recovery restores or completes one consistent Overlay revision and Curator records the resulting failure or success once

#### Scenario: User retries failed application
- **WHEN** the user requests retry
- **THEN** the system invalidates the old approval witness and requires a fresh preview before another explicit approval

### Requirement: Immutable decision audit
The system SHALL append an immutable, ordered audit event for intake, draft revision, assessment binding, preview, defer, resume, reject, approve, apply, failure, supersession, and policy changes. Events SHALL include trusted actor class, timestamp, prior and next state, object versions, and sanitized reason data.

#### Scenario: Audit history is queried
- **WHEN** the user inspects a candidate
- **THEN** the system returns a verifiable chronological history without unsafe draft bodies, secrets, or raw model prompts

#### Scenario: Client supplies actor identity
- **WHEN** a frontend request includes a forged actor or timestamp
- **THEN** the native boundary ignores it and derives trusted actor and time locally

### Requirement: Curator governance policy
The system SHALL provide versioned workspace policy for queue inclusion, required decision reasons, deferral range, retention, and notification preferences. Automatic application SHALL remain disabled and MUST NOT be enabled through this policy in this change.

#### Scenario: Policy changes during review
- **WHEN** governance policy advances after preview
- **THEN** affected approval witnesses become stale and require policy-aware preview again

#### Scenario: Automatic application requested
- **WHEN** a caller attempts to enable automatic application
- **THEN** the system rejects the unsupported setting and preserves manual approval

### Requirement: Candidate retention and evidence purge
The system SHALL apply bounded retention to non-applied candidate content and SHALL integrate evidence purge with candidate redaction while preserving the minimum sanitized decision and Overlay-link audit required to explain an applied mutation.

#### Scenario: Open candidate expires
- **WHEN** an untouched open candidate exceeds configured retention
- **THEN** it becomes superseded or expired, loses approval eligibility, and its removable draft content is purged

#### Scenario: Applied evidence is purged
- **WHEN** the user purges source evolution evidence
- **THEN** detailed lineage is removed while the Overlay history reference and a non-sensitive decision tombstone remain

### Requirement: Curator queue queries
The system SHALL expose workspace- and Skill-scoped paginated queries with stable filters for state, route, risk, target, age, draft readiness, staleness, and notification status.

#### Scenario: Queue is filtered
- **WHEN** the user filters pending high-risk candidates for one Skill
- **THEN** the system returns only authorized sanitized summaries in stable order with total counts

### Requirement: Curator is fail-closed for mutation and fail-open for Agents
Curator query, preview, policy, database, notification, or application failure MUST NOT affect Agent execution or evidence collection, while any uncertainty in mutation authorization SHALL prevent the Overlay write.

#### Scenario: Curator database is unavailable
- **WHEN** an Agent completes work while Curator persistence is unavailable
- **THEN** Agent completion remains unchanged and no unrecorded mutation occurs

#### Scenario: Audit write fails during approval
- **WHEN** the approval audit cannot be committed atomically with application coordination
- **THEN** the system does not report or perform an unaudited successful mutation

