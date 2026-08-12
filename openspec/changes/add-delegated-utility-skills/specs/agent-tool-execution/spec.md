## ADDED Requirements

### Requirement: Fixed Utility delegation tool
The native API Agent tool catalog SHALL include one fixed-schema `delegate_skill` tool when Utility delegation is supported. The tool SHALL resolve an eligible assigned Utility at call time and SHALL NOT add one provider tool definition per Utility.

#### Scenario: Tool declared for native API Agent
- **WHEN** a generation starts for a native API Agent with delegation support
- **THEN** the provider request SHALL declare `delegate_skill` using the same fixed schema regardless of Utility inventory

#### Scenario: No eligible Utilities
- **WHEN** the native API Agent currently has no eligible assigned Utilities
- **THEN** the fixed tool MAY remain available for stable catalog shape but every ineligible target SHALL be refused at dispatch

#### Scenario: Unsupported runtime
- **WHEN** a runtime cannot dispatch native Utility delegations
- **THEN** it SHALL not advertise `delegate_skill`

### Requirement: Child actions use the existing tool loop
Every child-Agent tool request SHALL use the existing validation, sandbox, permission evaluation, approval, cancellation, bounded input/output, and persisted tool-result behavior after applying the child's effective tool ceiling.

#### Scenario: Child read action
- **WHEN** an eligible child requests a declared bounded file read
- **THEN** the existing file-read validation and sandbox behavior SHALL apply under the child principal

#### Scenario: Child write action
- **WHEN** an eligible child requests a declared write operation
- **THEN** the existing permission and approval pipeline SHALL evaluate it under the child and parent principal chain before execution

#### Scenario: Child requests absent tool
- **WHEN** a child requests a tool outside its effective catalog
- **THEN** dispatch SHALL reject it without execution even if the parent Agent normally has that tool

#### Scenario: Child tool result persisted
- **WHEN** a child tool call reaches a terminal outcome
- **THEN** its bounded metadata SHALL be linked to the delegation attempt and parent completed message without duplicating an unbounded result

