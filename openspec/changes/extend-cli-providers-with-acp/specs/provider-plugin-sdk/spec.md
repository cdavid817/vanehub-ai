## MODIFIED Requirements

### Requirement: Complete internal provider SDK contract

The internal Provider Plugin SDK SHALL require every statically registered CLI provider to supply validated metadata, executable and version detection, side-effect-free readiness, declared transport-specific capabilities, supported launch and prompt translation, bounded input/output handling, cancellation, permission mapping, health diagnostics, and explicit handling of resume, model, reasoning, and usage. Optional unsupported concerns SHALL return classified unsupported-capability results rather than fabricated support. Mandatory lifecycle concerns for a declared transport MUST be implemented through the existing `agent_runtime` boundary.

#### Scenario: Register a complete provider

- **WHEN** the composition root registers a provider implementing every required SDK concern for each declared transport
- **THEN** the registry SHALL accept it after validating its declaration
- **AND** generic runtime callers SHALL access its behavior through the provider contract

#### Scenario: Reject an incomplete provider

- **WHEN** a provider declaration omits a mandatory transport concern or has inconsistent metadata and capabilities
- **THEN** registry construction SHALL fail with a classified provider-contract error
- **AND** no partial registration SHALL remain available

#### Scenario: Accept an explicitly unsupported optional feature

- **WHEN** a complete provider declares usage or resume unsupported and returns a classified result for that feature
- **THEN** registration SHALL succeed without claiming that the feature is available

### Requirement: Versioned data-only provider manifest

The SDK SHALL continue to accept documented manifest schema version `1` with its existing semantics and SHALL add schema version `2` for transport-specific, explicit capability declarations. Both schemas SHALL contain only stable identifiers, names, CLI runtime metadata, reviewed executable basenames, and validated capability data. Manifest validation MUST remain deterministic, strict, and free of execution side effects.

#### Scenario: Validate a version 1 manifest

- **WHEN** a version `1` manifest contains only supported fields and internally consistent values
- **THEN** validation SHALL produce the same normalized provider declaration for equivalent input
- **AND** it SHALL NOT launch, install, download, or probe an executable

#### Scenario: Reject an unsupported schema

- **WHEN** a manifest declares an unknown schema version or unknown field
- **THEN** validation SHALL fail with a classified manifest error
- **AND** it SHALL NOT guess compatibility

#### Scenario: Reject executable content

- **WHEN** a manifest contains an install or update hook, command, argument list, environment value, script, URL, absolute or traversing executable path, dynamic-library path, or executable entrypoint
- **THEN** validation SHALL reject the manifest
- **AND** no declared content SHALL be executed

#### Scenario: Reject an inconsistent capability declaration

- **WHEN** a manifest declares a capability combination that violates provider domain invariants
- **THEN** validation SHALL fail before registry construction

#### Scenario: Validate a version 2 declaration

- **WHEN** a version `2` manifest declares reviewed transports and explicitly unavailable optional features
- **THEN** validation SHALL normalize them without granting executable configuration or reinterpreting a version `1` record

### Requirement: Provider conformance test kit

The SDK SHALL provide a reusable, transport-aware conformance suite for every statically registered built-in provider. It SHALL cover deterministic registration, duplicate-id rejection, side-effect-free availability, launch and prompt mapping, cancellation, output handling, resume or its explicit rejection, unsupported features, redaction, version failure, manifest agreement, and classified errors. Capability-specific cases SHALL execute only for declared capabilities; their negative cases remain mandatory.

#### Scenario: Verify all built-in providers

- **WHEN** the conformance suite runs for the original five and all seven newly registered CLI identities
- **THEN** every provider SHALL satisfy the mandatory common contract and every declared transport contract
- **AND** provider-specific fixture expectations SHALL remain inside that provider adapter tests and iFlow SHALL NOT be treated as an ACP provider

#### Scenario: Verify a fixture provider

- **WHEN** a test-only fixture provider is added through the static test registry
- **THEN** it SHALL pass the same mandatory conformance suite without changing generic Session orchestration
- **AND** it SHALL NOT appear in the production registry

### Requirement: Provider parser conformance

An SDK output handler SHALL accept independently chunked stdout and stderr bytes and normalize session ids, usage when reported, tool events, incremental content, completion, and failure into the runtime event vocabulary. Parsing SHALL be invariant to valid byte partitions and use bounded buffers. Headless text fallback is permitted only where explicitly declared; ACP stdout MUST remain a protocol stream and MUST NOT fall back to ordinary text.

#### Scenario: Parse arbitrarily partitioned output

- **WHEN** equivalent valid provider output is divided at different byte boundaries
- **THEN** the handler SHALL emit an equivalent ordered normalized result
- **AND** split UTF-8 code points and partial structured records SHALL NOT be lost or corrupted

#### Scenario: Parse text fallback

- **WHEN** a non-ACP provider declares text fallback and emits non-structured valid text
- **THEN** the parser SHALL emit bounded text increments and a terminal outcome according to its adapter contract

#### Scenario: Reject oversized or malformed records

- **WHEN** undecoded output exceeds the declared buffer or record limit or violates its structured protocol
- **THEN** handling SHALL terminate with a classified bounded protocol error
- **AND** it SHALL NOT retain unbounded output or expose raw sensitive content

#### Scenario: Reject a banner on ACP stdout

- **WHEN** an ACP agent writes a non-protocol banner to stdout
- **THEN** the connection SHALL fail explicitly instead of emitting the banner as assistant content

## ADDED Requirements

### Requirement: Transport-specific effective capability assessment

The runtime SHALL assess capability per provider, installation identity, distribution, version, transport, platform, host implementation, peer negotiation, and policy. Assessment SHALL distinguish supported, unsupported, and unknown and include a stable reason code. Missing peer capabilities SHALL follow protocol semantics rather than optimistic defaults.

#### Scenario: Unknown version is discovered

- **WHEN** a runnable CLI reports a version outside the reviewed adapter profile
- **THEN** the runtime SHALL mark affected capabilities unknown or incompatible and require a compatible explicit check before managed execution

#### Scenario: One transport cannot enforce a policy

- **WHEN** a provider supports ACP approvals but its selected headless mode cannot enforce the requested policy
- **THEN** the runtime SHALL refuse that launch or require an explicit compatible transport selection
- **AND** it SHALL NOT silently add bypass flags or switch an active session transport

### Requirement: Additive provider compatibility preservation

The extension SHALL preserve the original five stable provider ids, old manifest semantics, original invocation fixtures, default selection, existing permission behavior, and persisted sessions. External provider loading SHALL remain disabled under the existing requirement.

#### Scenario: Existing session is opened

- **WHEN** a stored original-provider session lacks new transport metadata
- **THEN** the runtime SHALL resolve it through the original compatible path without converting it to ACP

#### Scenario: External provider manifest is offered

- **WHEN** a user-provided manifest names an arbitrary executable
- **THEN** the runtime SHALL reject external registration even when it uses the new schema
