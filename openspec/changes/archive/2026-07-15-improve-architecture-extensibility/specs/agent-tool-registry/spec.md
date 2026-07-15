## ADDED Requirements

### Requirement: Generated agent registry contracts
Agent registry entry models used by the Rust/Tauri layer and frontend service layer SHALL participate in the shared contract generation or verification workflow.

#### Scenario: Agent model changes
- **WHEN** the backend agent registry entry shape changes
- **THEN** the matching TypeScript model used by frontend services SHALL be updated or verified by the contract workflow

#### Scenario: Stable ids preserved in contracts
- **WHEN** agent registry contracts are generated or verified
- **THEN** the contract SHALL preserve stable kebab-case agent ids as the canonical reference field
