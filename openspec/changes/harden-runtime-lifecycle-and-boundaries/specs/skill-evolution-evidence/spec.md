## ADDED Requirements

### Requirement: Observable evidence degradation
The native runtime MUST distinguish unavailable evidence from absent feedback and SHALL expose a safe diagnostic when credential, processor, or repository initialization fails.

#### Scenario: Feedback storage query fails
- **WHEN** message feedback cannot be loaded because its repository fails
- **THEN** the command SHALL NOT silently present every message as having no feedback and SHALL return or record a safe classified failure

#### Scenario: Evidence pipeline cannot initialize
- **WHEN** the installation key or ingestion processor cannot be initialized
- **THEN** the runtime SHALL expose disabled health with a safe reason and SHALL write a redacted unified diagnostic

