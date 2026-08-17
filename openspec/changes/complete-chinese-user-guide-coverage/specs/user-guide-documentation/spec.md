## ADDED Requirements

### Requirement: Authoritative guide covers delivered user-facing capabilities

The authoritative user guide SHALL carry coverage for every delivered capability that a user exercises directly through the application. Coverage SHALL be a dedicated chapter or a named section of the chapter that already owns the subject; an incidental mention SHALL NOT count as coverage.

A delivered capability MAY be excluded only as a stated, reviewable decision recording why it is not user-facing. An unstated omission SHALL NOT be treated as an exclusion.

Coverage SHALL describe the capability's user-visible behavior using the labels the application actually renders, and SHALL NOT describe behavior that the capability's specification does not establish.

#### Scenario: A delivered capability has no coverage

- **WHEN** a capability that a user exercises directly through the application has shipped and the authoritative guide contains no chapter or named section covering it
- **THEN** the guide SHALL be considered non-compliant with this requirement
- **AND** an incidental mention of the capability elsewhere in the guide SHALL NOT satisfy it

#### Scenario: A capability is deliberately excluded

- **WHEN** a delivered capability is judged not user-facing
- **THEN** the exclusion SHALL be recorded as an explicit decision naming the capability and the reason
- **AND** the absence of that record SHALL be treated as a coverage gap rather than as an exclusion

#### Scenario: Coverage uses rendered labels

- **WHEN** guide coverage names a control, status, or surface of a capability
- **THEN** it SHALL use the label the application renders for that element in the guide's language
- **AND** it SHALL NOT assert behavior beyond what that capability's specification establishes

#### Scenario: A partial-locale guide inherits the coverage obligation

- **WHEN** the authoritative guide gains coverage for a capability while another locale's guide is in a declared transition
- **THEN** that locale SHALL represent the new chapter in its navigation as an explicit known gap
- **AND** it SHALL NOT silently omit the chapter
