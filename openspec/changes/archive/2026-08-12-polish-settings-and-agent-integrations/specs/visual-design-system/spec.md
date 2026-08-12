## ADDED Requirements

### Requirement: OnePiece branded Agent identity
Every shared Agent identity surface SHALL render OnePiece with a dedicated recognizable icon instead of the generic fallback Agent icon.

#### Scenario: Render OnePiece identity
- **WHEN** a component requests the shared Agent icon for stable Agent id `onepiece`
- **THEN** the component SHALL render the dedicated OnePiece vector mark
- **AND** it SHALL preserve the same sizing, accessibility, and theme behavior as other built-in CLI icons

