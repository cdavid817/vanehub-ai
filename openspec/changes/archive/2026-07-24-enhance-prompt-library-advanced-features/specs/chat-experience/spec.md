## ADDED Requirements

### Requirement: CLI chat records Prompt Hook version outcomes
The desktop CLI chat runtime SHALL report one safe terminal outcome and elapsed time for every published Prompt Hook version fired during an Agent invocation.

#### Scenario: Record successful generation
- **WHEN** a CLI Agent generation succeeds after published Prompt Hook versions fired
- **THEN** the runtime SHALL report `succeeded`, elapsed milliseconds, stable invocation id, stable agent id, and the fired Hook id/version references through the evaluation gateway

#### Scenario: Record failed generation
- **WHEN** a CLI Agent generation fails after published Prompt Hook versions fired
- **THEN** the runtime SHALL report `failed` with the same safe correlation fields
- **AND** it SHALL not include the raw failure or Prompt content in the evaluation observation

#### Scenario: Record cancelled generation separately
- **WHEN** a user cancels a generation after published Prompt Hook versions fired
- **THEN** the runtime SHALL report `cancelled`
- **AND** the cancelled observation SHALL not reduce the version's calculated success rate

#### Scenario: No fired Hook versions
- **WHEN** no Prompt Hook version fires for an Agent invocation
- **THEN** the runtime SHALL not create Prompt Hook evaluation observations for that invocation
