## ADDED Requirements

### Requirement: Session usage frontend service boundary
The frontend SHALL expose session-scoped usage summary data through the Agent service interface and matching runtime adapters rather than direct runtime calls from React components.

#### Scenario: Desktop session usage adapter
- **WHEN** the frontend runs inside the Tauri desktop runtime and session usage is requested
- **THEN** the Tauri adapter SHALL call a declared Tauri command through the Agent service boundary
- **AND** it SHALL return reported-token totals, estimated-character totals, coverage counts, response counts, and generation time using the shared frontend contract

#### Scenario: Web session usage adapter
- **WHEN** the frontend runs outside the Tauri desktop runtime and session usage is requested
- **THEN** the Web adapter SHALL provide compatible mock session usage data without importing or invoking Tauri APIs
- **AND** it SHALL preserve the same reported-token-primary and estimated-character-separate semantics as the desktop contract

#### Scenario: Workspace panel service consumption
- **WHEN** the workspace information panel loads Basic Info, Token Usage, or Skill data
- **THEN** React components SHALL request data through the Agent service and shared data-fetching foundation
- **AND** they SHALL preserve already loaded panel data during an in-progress refresh

#### Scenario: Runtime contract parity for session usage
- **WHEN** Tauri and Web adapters expose session-scoped usage
- **THEN** shared TypeScript contracts and adapter tests SHALL verify equivalent method signatures and normalized result shape
