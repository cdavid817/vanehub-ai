## MODIFIED Requirements

### Requirement: Native Agent executes conditional read-only LSP tools
The system SHALL conditionally add its read-only LSP tools to the provider-agnostic native API Agent tool catalog for a configured trusted local workspace. It SHALL execute them as cancellable workspace-read operations with hard input, request, result-count, content, and serialized-output limits, and SHALL preserve their inputs, bounded outputs, and outcomes through the existing visible persisted tool-use lifecycle. Newly added LSP tools SHALL be appended to the catalog rather than inserted among existing entries, so the tool-definition prefix a provider caches stays stable.

#### Scenario: LSP tools are translated for the provider
- **WHEN** a native API Agent generation starts with LSP available for its current trusted local workspace
- **THEN** every offered LSP tool SHALL be declared using the session provider's existing tool-definition translation

#### Scenario: LSP request is cancelled with the generation
- **WHEN** the user stops a generation while an LSP tool request is pending
- **THEN** the runtime SHALL cancel the pending wait and complete bounded protocol cleanup
- **AND** it SHALL NOT continue the request after generation cancellation

#### Scenario: LSP output reaches a hard limit
- **WHEN** accepted LSP locations, symbols, call relations, diagnostic messages, hover text, previews, or serialized output exceed a declared limit
- **THEN** the tool SHALL return only the bounded result
- **AND** it SHALL explicitly report truncation where applicable

#### Scenario: Session has no eligible local workspace
- **WHEN** the model requests an LSP tool for a session without a trusted eligible local workspace
- **THEN** the runtime SHALL reject the call without starting a server or accessing another workspace

#### Scenario: A new LSP tool joins the catalog
- **WHEN** a build adds an LSP tool to the read-only set
- **THEN** it SHALL appear after every tool the previous build declared
- **AND** the declaration order of the previously existing tools SHALL be unchanged

#### Scenario: A multi-step LSP tool is cancelled between its steps
- **WHEN** the user stops a generation after a call-hierarchy preparation has resolved but before its calls request completes
- **THEN** the runtime SHALL cancel the pending wait and complete bounded protocol cleanup
- **AND** it SHALL NOT issue the remaining step
