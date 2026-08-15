## ADDED Requirements

### Requirement: OnePiece automatic compaction control
The OnePiece parameter page SHALL provide a localized, keyboard-accessible control for the persisted automatic-context-compaction preference and SHALL save changes through the shared settings service boundary.

#### Scenario: Open OnePiece parameter page
- **WHEN** a user opens the OnePiece area of CLI Parameter Management
- **THEN** the page SHALL show the current automatic-compaction preference
- **AND** SHALL explain that the preference applies to subsequent OnePiece generations

#### Scenario: Disable automatic compaction
- **WHEN** the user disables the control
- **THEN** the page SHALL persist the disabled value through the settings provider
- **AND** SHALL expose saving or failure feedback without directly invoking a native command

#### Scenario: Render supported themes and locales
- **WHEN** the application theme or locale changes
- **THEN** the control, description, status, and focus state SHALL remain readable and localized using existing semantic styles

