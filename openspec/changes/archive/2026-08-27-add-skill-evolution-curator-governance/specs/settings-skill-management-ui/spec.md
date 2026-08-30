## ADDED Requirements

### Requirement: Curator workspace
The Skill Evolution experience SHALL provide a Curator workspace with queue counts, stable filters, candidate summaries, draft readiness, risk, route, staleness, and deferred status while retaining per-Skill navigation context.

#### Scenario: User opens Curator
- **WHEN** Curator candidates exist
- **THEN** the UI shows service-backed queue data ordered by governance priority and age

#### Scenario: Queue is empty
- **WHEN** no candidates match the current filters
- **THEN** the UI shows an accurate empty state without demo candidates

### Requirement: Complete candidate review
The Curator workspace SHALL display sanitized source lineage, assessment target ranking, all nine quality checks, risk, confidence, route, target revision, current Skill and Overlay state, draft history, and decision audit before actions.

#### Scenario: High-risk candidate is reviewed
- **WHEN** the user opens a high-risk candidate
- **THEN** the UI foregrounds the blocking risk and shows why manual review cannot bypass Overlay restrictions

### Requirement: Safe Curator draft editor
The Curator workspace SHALL support only learned-guidance and exact-patch drafts, preserve unsaved input on validation errors, and clearly prohibit target override, base editing, executable content, and supporting-file mutation.

#### Scenario: Draft validation fails
- **WHEN** the service rejects unsafe or stale draft input
- **THEN** the UI retains safe unsaved input where permitted and presents the specific corrective action

### Requirement: Diff-bound approval experience
The UI SHALL require a current Overlay preview and explicit confirmation of the displayed effective diff before enabling approval, and SHALL invalidate approval controls whenever any witness changes.

#### Scenario: User reviews a valid preview
- **WHEN** preview is current and all required checks pass
- **THEN** the UI shows base-to-current, current-to-proposed, and final effective changes and enables explicit approval

#### Scenario: Preview becomes stale
- **WHEN** candidate, draft, assessment, Skill, Overlay, pin, or policy state changes
- **THEN** approval is disabled and the UI requires refresh or reassessment

### Requirement: Reject defer and resume experience
The UI SHALL collect required reason categories for rejection and deferral, support an optional bounded note and review-after time, and explain that deferred candidates require manual resume.

#### Scenario: Required reason is missing
- **WHEN** the user attempts to reject or defer without a reason
- **THEN** the UI prevents submission and identifies the required field

### Requirement: Curator recovery and history
The UI SHALL preserve the last valid candidate state during failures, show stale, superseded, and apply-failed recovery paths, and link applied candidates to the resulting Overlay history.

#### Scenario: Application fails
- **WHEN** Overlay application returns a recoverable failure
- **THEN** the UI shows the stable failure category and requires a new preview before retry approval

### Requirement: Manual-governance boundary
The Curator UI SHALL NOT provide automatic apply, approve-all, bulk approval, model approval, pin bypass, direct base mutation, or notification action buttons that commit a mutation.

#### Scenario: Multiple low-risk candidates exist
- **WHEN** the queue contains several `advance` candidates
- **THEN** each candidate still requires its own witnessed preview and explicit approval

