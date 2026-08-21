## ADDED Requirements

### Requirement: Complete native dependency enforcement
The native architecture test MUST inspect domain, application, infrastructure, and command source files and SHALL reject private cross-context infrastructure dependencies outside the composition root.

#### Scenario: Infrastructure imports another context repository
- **WHEN** a bounded context infrastructure module imports another context's concrete repository
- **THEN** the architecture test SHALL fail with the importing file, line, and dependency

#### Scenario: Command executes infrastructure behavior
- **WHEN** a command handler imports or invokes a context's private infrastructure implementation
- **THEN** the architecture test SHALL fail unless the command uses a deliberately published API contract

