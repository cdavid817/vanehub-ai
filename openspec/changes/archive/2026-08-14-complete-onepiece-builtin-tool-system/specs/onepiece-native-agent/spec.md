## ADDED Requirements

### Requirement: OnePiece extended tool capability and readiness
The system SHALL project the Browser, Web research, code-execution, OCR, Artifact-publication, and CLI-delegation capabilities only on the built-in OnePiece identity and SHALL expose mode-specific readiness and safe reason codes without making OnePiece chat readiness depend on every optional tool. User-created API Agents SHALL not inherit these capabilities from provider configuration or capability-tag editing.

#### Scenario: Optional dependency is unavailable
- **WHEN** OnePiece's provider Profile is ready but an optional browser, OCR, sandbox, Artifact, or delegated-CLI dependency is unavailable
- **THEN** OnePiece SHALL remain available for ordinary chat and baseline tools while the affected extended operation is excluded or reported unavailable

#### Scenario: OnePiece has one usable delegated target
- **WHEN** at least one supported target/mode passes delegation readiness
- **THEN** OnePiece MAY receive the fixed `delegate_cli` definition while unavailable targets remain dispatch-time errors with actionable reasons

#### Scenario: Custom API Agent copies capability metadata
- **WHEN** a user-created API Agent has metadata resembling OnePiece
- **THEN** the native tool registry SHALL still deny eligibility because its stable id is not `onepiece`

### Requirement: Safe defaults for extended effects
OnePiece SHALL default to explicit unified approval for arbitrary code execution, effectful browser actions, retained downloads, external CLI delegation start, and delegated ChangeSet application. ChangeSet application approval SHALL always be once-only and SHALL not become automatically allowed through session, project, global, trusted, or YOLO-style remembered scopes.

#### Scenario: First use of code execution
- **WHEN** a newly configured OnePiece requests `code_execution`
- **THEN** the system SHALL request approval bound to the exact source, runtime, inputs, and limits unless a non-remembered explicit policy decision for that call already exists

#### Scenario: User previously trusted ordinary OnePiece tools
- **WHEN** OnePiece's existing policy allows shell or file writes automatically
- **THEN** `apply_delegation_changes` SHALL still require its specialized once-only exact-ChangeSet approval

### Requirement: Extended readiness is available through shared adapters
The frontend SHALL obtain OnePiece extended-capability readiness and operation state through shared service contracts with Tauri and Web/mock implementations. The Web/mock registry SHALL preserve the same OnePiece identity and capability presentation while identifying native execution as simulated or desktop-required.

#### Scenario: Desktop settings inspect readiness
- **WHEN** a user opens OnePiece capability diagnostics
- **THEN** the Tauri adapter SHALL return native per-capability and per-mode readiness without starting a browser, OCR inference, sandbox program, or delegated model call

#### Scenario: Web settings inspect readiness
- **WHEN** the same surface runs in Web/mock mode
- **THEN** the Web adapter SHALL return deterministic non-native readiness without implying installed desktop dependencies

