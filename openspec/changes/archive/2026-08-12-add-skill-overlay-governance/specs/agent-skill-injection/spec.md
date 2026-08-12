## ADDED Requirements

### Requirement: Overlay-applied instruction consumption
Every API Agent Skill instruction consumer SHALL use content produced by successful trusted Overlay replay for the active workspace context. It SHALL NOT inject untrusted, pinned-after mutation, invalid, or unresolved conflicted Overlay changes.

#### Scenario: Healthy Overlay affects eager instructions
- **WHEN** an applicable eager Skill has a healthy trusted Overlay
- **THEN** prompt assembly SHALL use the Overlay-applied effective instructions within the existing per-Skill and aggregate budgets

#### Scenario: Healthy Overlay affects on-demand load
- **WHEN** an applicable on-demand Role Skill has a healthy trusted Overlay
- **THEN** its load result SHALL use the same Overlay-applied effective instructions and resource view

#### Scenario: Untrusted Overlay excluded
- **WHEN** an applicable imported Overlay has not been promoted to trusted
- **THEN** agent-visible instructions and resources SHALL remain unchanged by that Overlay

#### Scenario: Conflicted Overlay falls back safely
- **WHEN** an applicable Overlay has unresolved replay conflicts
- **THEN** agent-visible content SHALL exclude that Overlay scope and use the last deterministic lower-scope or base content
- **AND** the system SHALL report a redacted warning through unified logging

