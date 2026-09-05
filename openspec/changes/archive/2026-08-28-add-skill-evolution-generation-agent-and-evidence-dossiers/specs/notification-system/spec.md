## ADDED Requirements

### Requirement: Generation notifications
The notification system SHALL publish sanitized, deduplicated notifications for review-ready generation, generation failure requiring attention, cancellation, and supersession with navigation to the generation job or Curator candidate.

#### Scenario: Draft becomes reviewable
- **WHEN** a generation job packages a validated draft
- **THEN** one notification identifies safe Skill or proposal identity, draft kind, job id, and Curator navigation without including generated content

#### Scenario: Routine stage advances
- **WHEN** a job moves between normal internal stages
- **THEN** the system does not emit repetitive notifications

### Requirement: Generation notification actions are non-mutating
Generation notification actions SHALL only navigate and MUST NOT enable consent, regenerate, cancel, approve, install, or apply a draft.

#### Scenario: User activates review-ready notification
- **WHEN** the notification is opened
- **THEN** the application navigates to the review surface without changing job or Curator state

