# agent-provider-runtime Specification

## Purpose

Defines how VaneHub resolves agent runtime behavior through stable provider contracts while preserving existing Agent identities, session behavior, and runtime adapter boundaries.
## Requirements
### Requirement: Stable provider resolution
The Agent Runtime SHALL resolve supported built-in CLI runtime behavior through a provider registry using the Agent registry entry's stable id, and provider-neutral application and Session modules SHALL NOT require provider-identity branching to select that behavior.

#### Scenario: Resolve a registered CLI provider
- **WHEN** runtime work targets a registered built-in CLI Agent id
- **THEN** the Agent Runtime SHALL resolve exactly one provider contract for that stable id
- **AND** the Session application layer SHALL NOT select behavior by matching that id

#### Scenario: Reject an unknown provider
- **WHEN** runtime work targets an Agent id with no compatible provider registration
- **THEN** the Agent Runtime SHALL return a classified unsupported-provider error
- **AND** SHALL NOT fall back to another provider

### Requirement: Provider metadata and capabilities
Each registered provider SHALL declare validated metadata, readiness prerequisites, and supported runtime capabilities independently of display-name matching or caller inference from provider identity.

#### Scenario: Enumerate provider declarations
- **WHEN** the runtime enumerates registered providers
- **THEN** each result SHALL contain a non-empty stable id and display name
- **AND** SHALL declare its supported interaction, resume, structured-output, terminal, usage, permission, model-selection, and reasoning capabilities

#### Scenario: Unsupported capability
- **WHEN** a provider does not declare a requested capability
- **THEN** the runtime SHALL report that capability as unavailable
- **AND** callers SHALL NOT infer support from the provider id or display name

#### Scenario: Availability remains side-effect free
- **WHEN** provider readiness is assessed from its declared prerequisites
- **THEN** the assessment SHALL NOT start an interactive Agent session or generation process

### Requirement: Deterministic static registration
The first provider framework version SHALL use explicit in-process registration and SHALL reject ambiguous registrations.

#### Scenario: Register current built-in CLI providers
- **WHEN** the desktop runtime composition root starts
- **THEN** it SHALL register compatibility providers for `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, and `antigravity-cli`
- **AND** provider enumeration SHALL be deterministic

#### Scenario: Reject duplicate registration
- **WHEN** two providers declare the same stable id
- **THEN** registry construction SHALL fail with a classified duplicate-provider error

### Requirement: Opaque provider session references
The Agent Runtime SHALL treat a provider-native resume identifier as an opaque value associated with both the owning VaneHub Session and provider id.

#### Scenario: Restore an existing resume identifier
- **WHEN** a persisted Session contains a provider-native runtime session id
- **THEN** the Agent Runtime SHALL reconstruct a provider session reference using the Session's Agent id and the persisted opaque id
- **AND** provider-neutral Session code SHALL NOT interpret provider-specific id semantics

#### Scenario: Start without a resume identifier
- **WHEN** a persisted Session has no provider-native runtime session id
- **THEN** the runtime SHALL request a fresh provider session without fabricating an external id

### Requirement: Compatibility during provider-contract introduction
Introducing the provider contract SHALL preserve existing desktop CLI execution, terminal, resume, event, usage, logging, Tauri command, persistence, and Web/mock service behavior.

#### Scenario: Existing CLI operation during compatibility phase
- **WHEN** any currently supported built-in CLI is launched after the provider registry is introduced
- **THEN** its existing invocation arguments, prompt delivery, output parsing, cancellation, terminal behavior, usage accounting, and resume behavior SHALL remain compatible

#### Scenario: Existing clients require no migration
- **WHEN** an existing desktop database or frontend client uses the new runtime build
- **THEN** no data migration or frontend service-contract change SHALL be required by this change

### Requirement: Image content-block translation
The provider runtime SHALL translate an image carried by a user turn or a tool result into the image content shape required by the session's `interface_format`, using one shared internal representation rather than a per-provider image path. A request that carries no image SHALL be byte-identical to what the same request produced before image support existed.

#### Scenario: Anthropic format translation
- **WHEN** a request carrying an image is built for `interface_format` `anthropic`
- **THEN** the image SHALL be declared using that format's image content block

#### Scenario: OpenAI-compatible format translation
- **WHEN** a request carrying an image is built for `interface_format` `openai-compatible`
- **THEN** the image SHALL be declared using that format's image content shape

#### Scenario: Text-only requests are unchanged
- **WHEN** a request carries no image
- **THEN** its body SHALL be identical to the body produced before image support was added

### Requirement: Capability-negotiated provider execution
Provider-neutral runtime callers SHALL resolve a provider by stable id, inspect its declared capabilities, and request only supported behavior. They SHALL NOT select launch, resume, permission, parsing, usage, model, reasoning, cancellation, review, evaluation, or monitoring behavior by matching provider identity or display name.

#### Scenario: Request a supported provider capability
- **WHEN** a caller resolves a provider and requests a capability it declares
- **THEN** the runtime SHALL delegate the request to that provider adapter

#### Scenario: Request an unsupported provider capability
- **WHEN** a caller requests a capability the resolved provider does not declare
- **THEN** the runtime SHALL return a classified unsupported-capability error containing the stable provider id and capability
- **AND** SHALL NOT fall back to identity-specific behavior

### Requirement: Provider-owned translation and lifecycle mapping
Each built-in provider adapter SHALL own its executable/version declaration, prompt and input translation, managed launch arguments, resume mapping, cancellation semantics, permission flags, model/reasoning options, output parser, usage extraction, and health classification. Generic Session and generation orchestration SHALL execute adapter-produced specifications through existing application ports.

#### Scenario: Launch a built-in provider
- **WHEN** generic orchestration launches a supported built-in CLI
- **THEN** the resolved provider adapter SHALL produce the invocation and parser contract
- **AND** generic orchestration SHALL NOT add provider-specific arguments or parse provider-specific event fields

#### Scenario: Cancel a provider invocation
- **WHEN** cancellation is requested for a running provider invocation
- **THEN** generic orchestration SHALL apply the provider's declared bounded cancellation semantics through the existing process boundary
- **AND** the terminal result SHALL use the unified classified error vocabulary

### Requirement: Side-effect-free detection and health diagnostics
Executable availability, version detection, readiness, and health assessment SHALL use bounded non-interactive probe specifications and SHALL NOT start an interactive Agent session or generation. Failures SHALL be classified and persisted only through unified redacted diagnostics.

#### Scenario: Check provider availability
- **WHEN** runtime availability is refreshed for a registered provider
- **THEN** the provider SHALL return or execute only its bounded detection specifications
- **AND** SHALL NOT deliver a prompt or create a provider session

#### Scenario: Version detection fails
- **WHEN** an executable is missing, times out, exits unsuccessfully, or returns an invalid version
- **THEN** the runtime SHALL return a classified safe diagnostic
- **AND** unified logs SHALL omit credentials, prompt content, sensitive arguments, environment values, and unbounded raw output

### Requirement: Unified provider error classification
Provider contract, manifest, capability, preparation, detection, parser, permission, cancellation, and process failures SHALL map to the existing command-safe Agent Runtime error boundary with stable classifications and concise messages.

#### Scenario: Provider parser reports a failure
- **WHEN** a provider parser emits a terminal protocol or provider-reported failure
- **THEN** the application SHALL preserve its safe classification and retry semantics
- **AND** user-facing output SHALL not expose raw stderr or sensitive payloads

#### Scenario: Permission mapping is rejected
- **WHEN** requested permission intent cannot be represented safely by a provider
- **THEN** the runtime SHALL return a classified unsupported-capability or permission error before launch

### Requirement: Provider-neutral fixture extensibility
A test-only provider SHALL be addable through static registry composition without editing generic Session orchestration, generation lifecycle, usage projection, or frontend runtime adapters.

#### Scenario: Run a fixture provider end to end
- **WHEN** a desktop integration test registers and launches the fixture provider against a fake CLI executable
- **THEN** the existing runtime path SHALL stream normalized output, capture its opaque session id and usage, and support bounded cancellation as declared
- **AND** production Agent enumeration SHALL remain unchanged

### Requirement: Provider and Runner remain orthogonal
Provider adapters SHALL continue to own provider invocation, prompt/input translation, output parsing, usage, provider sessions, cancellation semantics, and provider error mapping, while Runner adapters SHALL own execution location, transport, process/channel I/O, inspection, cleanup, and runner errors. Generic orchestration MUST NOT select Runner behavior by provider id or provider behavior by Runner kind.

#### Scenario: Run one provider locally and remotely
- **WHEN** a provider declares the capabilities required by both an eligible Local and SSH Runner
- **THEN** the same provider adapter prepares both invocations and the selected Runner executes the resulting bounded specification

#### Scenario: Runner transport fails
- **WHEN** transport fails before a provider terminal protocol event
- **THEN** provider parsing does not fabricate a provider error and orchestration preserves the Runner classification

