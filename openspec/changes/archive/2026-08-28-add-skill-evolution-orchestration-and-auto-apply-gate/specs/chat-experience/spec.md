## ADDED Requirements

### Requirement: Reusable correction authorization
When recording corrected feedback, the chat experience SHALL offer a separate default-off authorization for the exact correction revision to be considered reusable Skill guidance. The authorization SHALL explain that normal feedback processing continues without it and SHALL be revocable before application.

#### Scenario: User submits correction without authorization
- **WHEN** the user records corrected feedback and leaves reusable guidance disabled
- **THEN** the correction is stored for evidence but is ineligible for deterministic automatic-draft production

#### Scenario: User authorizes reusable guidance
- **WHEN** the user explicitly enables authorization for the current bounded correction
- **THEN** the service stores a versioned authorization witness linked to that feedback revision

#### Scenario: Correction changes
- **WHEN** the user replaces authorized correction content
- **THEN** prior authorization is revoked and the new revision defaults to unauthorized

### Requirement: Reusable authorization revocation
The chat experience SHALL allow the user to revoke reusable-guidance authorization while no application has committed and SHALL show when a resulting draft or run is pending review.

#### Scenario: User revokes before auto application
- **WHEN** the user revokes authorization before final mutation preflight
- **THEN** derived automatic eligibility becomes stale and no automatic mutation occurs

