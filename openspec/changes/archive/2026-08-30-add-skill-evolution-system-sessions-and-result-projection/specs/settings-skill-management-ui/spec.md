## ADDED Requirements

### Requirement: System Activity navigation and unread badges
The Skill Evolution experience SHALL provide a distinct System Activity destination for global and workspace scopes with unread counts and attention indicators that do not alter interactive session totals.

#### Scenario: Workspace has unread attention activity
- **WHEN** the workspace system session is visible
- **THEN** navigation shows its bounded unread count and highest attention severity

### Requirement: Activity dashboard projection
The Skill Evolution dashboard SHALL show safe current summaries for runs, candidates, generation, Curator queue, applications, probation, and breakers derived from canonical projected envelopes with freshness and completeness indicators.

#### Scenario: Projection is behind
- **WHEN** one source domain has pending events
- **THEN** the dashboard shows its last projected time and lag rather than presenting stale data as current

### Requirement: Projection preferences and retention UI
The UI SHALL provide service-backed controls for visibility, minimum severity, attention notifications, digest cadence, read-state retention, detailed retention, and export limits with clear effect descriptions.

#### Scenario: User reduces detailed retention
- **WHEN** the user confirms a valid shorter period
- **THEN** the UI updates preference version and explains that authoritative governance retention remains separate

### Requirement: Projection health and rebuild UI
The UI SHALL show per-domain cursors, lag, gaps, failed categories, active generation, rebuild attempts, validation, and last success, and SHALL offer scoped rebuild without implying source replay.

#### Scenario: Rebuild is requested
- **WHEN** the user confirms workspace projection rebuild
- **THEN** the UI preserves the current readable generation, shows bounded rebuild progress, and states that no model or mutation reruns

### Requirement: System activity export UI
The UI SHALL support sanitized JSON and Markdown export of the current scope and filters and SHALL disclose completeness, redaction, and exported-file retention responsibility.

#### Scenario: Export fails
- **WHEN** native export is cancelled or unavailable
- **THEN** the UI preserves timeline state and shows a localized actionable result

