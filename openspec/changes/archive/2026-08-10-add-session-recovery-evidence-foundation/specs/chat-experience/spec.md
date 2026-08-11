## ADDED Requirements

### Requirement: Chat controls respect recovery safety
The chat experience SHALL derive send and stop availability from the service-backed lifecycle, recovery status, and active execution ownership rather than lifecycle alone.

#### Scenario: Disable sending during reconciliation
- **WHEN** the active session is `reconciling`, `action_required`, or `quarantined`
- **THEN** the composer SHALL prevent message submission and show the corresponding localized recovery state

#### Scenario: Allow a clean failed session to continue
- **WHEN** the active session lifecycle is `failed`, recovery status is `clean`, no execution run is active, and the session is not archived
- **THEN** the composer SHALL allow a new message to be submitted through the frontend service

#### Scenario: Stop targets only an active execution
- **WHEN** recovery has cleared an orphan active claim and no generation handle exists
- **THEN** the chat UI SHALL NOT offer stop as though an old native process were still running

### Requirement: Recovery review preserves user-visible evidence
The chat experience SHALL present interrupted content and safe recovery explanations without removing transcript evidence or exposing sensitive diagnostics.

#### Scenario: Show action-required recovery
- **WHEN** the active session requires recovery action
- **THEN** the UI SHALL preserve the existing transcript, display a localized safe explanation, and offer the allowed acknowledgement action through the service boundary

#### Scenario: Acknowledge recovery
- **WHEN** the user confirms acknowledgement for the currently displayed recovery revision
- **THEN** the UI SHALL submit it through the shared service, refresh the authoritative session state, and SHALL NOT represent the action as retrying or undoing tool effects

#### Scenario: Present quarantined session
- **WHEN** the active session is quarantined
- **THEN** the UI SHALL keep supported inspection and export surfaces available while disabling dependent mutations

