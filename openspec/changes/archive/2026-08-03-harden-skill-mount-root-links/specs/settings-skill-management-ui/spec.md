## ADDED Requirements

### Requirement: Actionable mount-root assignment failure
The Skills settings page SHALL keep a failed CLI Agent assignment attached to the affected Skill row and SHALL not present the Skill as assigned when native mount-root preflight rejects the operation.

#### Scenario: Show externally managed root failure
- **WHEN** assignment fails because the selected CLI Agent's Skill root is an externally managed directory link
- **THEN** the affected row SHALL show a concise error identifying the selected Agent and explaining that the whole-directory link must be migrated before assignment
- **AND** the Skill SHALL remain in the selected Agent's Available group after the overview refreshes

#### Scenario: Show broken root failure
- **WHEN** assignment fails because the selected CLI Agent's Skill root is a broken or unavailable directory link
- **THEN** the affected row SHALL show a concise error identifying the selected Agent and explaining that the stale link must be repaired or removed before assignment
- **AND** unrelated Skill and Agent controls SHALL remain available
