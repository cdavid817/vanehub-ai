## ADDED Requirements

### Requirement: Curator governance notifications
The notification system SHALL publish sanitized, deduplicated events for new reviewable candidates, deferral review dates, supersession, rejection, successful application, and application failure with navigation to the relevant Curator candidate.

#### Scenario: Reviewable candidate is enqueued
- **WHEN** a candidate first becomes ready for human review
- **THEN** the system publishes at most one pending-review notification for that candidate revision

#### Scenario: Application succeeds
- **WHEN** an approved draft commits to an Overlay
- **THEN** the system publishes a success notification containing safe Skill identity, scope, candidate id, and Overlay history navigation

#### Scenario: Sensitive reason data exists
- **WHEN** a candidate or failure contains bounded user notes or sensitive source context
- **THEN** the notification excludes that content and uses a stable localized summary

### Requirement: Curator notification actions are non-mutating
Curator notifications SHALL navigate to review or history but MUST NOT approve, reject, retry, apply, resume, or otherwise mutate a candidate directly.

#### Scenario: User opens pending review notification
- **WHEN** the user activates the notification
- **THEN** the application opens the candidate review surface without performing a decision

