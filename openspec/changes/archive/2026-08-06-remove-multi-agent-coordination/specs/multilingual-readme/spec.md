## MODIFIED Requirements

### Requirement: README claims reflect implemented state
README feature claims SHALL distinguish delivered, preview, Web/mock-only, desktop-only, and planned behavior and SHALL not present a service-layer contract as an available user workflow when no user-visible path exists.

#### Scenario: Promote a feature to delivered
- **WHEN** a README changes a feature classification to delivered
- **THEN** the change SHALL reference an implemented and testable user-visible or documented developer path
