# provider-plugin-sdk Specification

## Purpose
TBD - created by archiving change extend-provider-runtime-plugin-sdk. Update Purpose after archive.
## Requirements
### Requirement: Complete internal provider SDK contract
The internal Provider Plugin SDK SHALL require every statically registered CLI provider to supply validated metadata, executable and version detection, side-effect-free readiness, declared capabilities, generation and interactive launch translation, prompt/input translation, incremental output parsing, resume, cancellation, permission mapping, model and reasoning options, usage extraction, and bounded health diagnostics through the existing `agent_runtime` boundary.

#### Scenario: Register a complete provider
- **WHEN** the composition root registers a provider implementing every required SDK concern
- **THEN** the registry SHALL accept it after validating its declaration
- **AND** generic runtime callers SHALL access its behavior through the provider contract

#### Scenario: Reject an incomplete provider
- **WHEN** a provider declaration cannot supply a required SDK concern or contains inconsistent metadata and capabilities
- **THEN** registry construction SHALL fail with a classified provider-contract error
- **AND** no partial registration SHALL remain available

### Requirement: Versioned data-only provider manifest
The SDK SHALL define and document provider manifest schema version `1` with a stable id, display name, CLI runtime kind, reviewed executable basenames, and explicit capability declarations. Manifest validation SHALL be deterministic, strict, and free of execution side effects.

#### Scenario: Validate a version 1 manifest
- **WHEN** a version `1` manifest contains only supported fields and internally consistent values
- **THEN** validation SHALL produce the same normalized provider declaration for equivalent input
- **AND** SHALL NOT launch, install, download, or probe an executable

#### Scenario: Reject an unsupported schema
- **WHEN** a manifest declares an unknown schema version or unknown field
- **THEN** validation SHALL fail with a classified manifest error
- **AND** SHALL NOT guess compatibility

#### Scenario: Reject executable content
- **WHEN** a manifest contains an install or update hook, command, argument list, environment value, script, URL, absolute or traversing executable path, dynamic-library path, or executable entrypoint
- **THEN** validation SHALL reject the manifest
- **AND** no declared content SHALL be executed

#### Scenario: Reject an inconsistent capability declaration
- **WHEN** a manifest declares a capability combination that violates the provider domain invariants
- **THEN** validation SHALL fail before registry construction

### Requirement: External providers remain disabled
This SDK version SHALL NOT discover, load, install, update, execute, disable, or quarantine external provider packages. External provider enablement MUST require a later approved specification for SDK compatibility, package provenance and signatures, permissions, Sandbox isolation, lifecycle, update, disablement, and quarantine.

#### Scenario: External manifest is presented
- **WHEN** runtime input references a provider manifest that is not part of the reviewed static build
- **THEN** the runtime SHALL return a classified external-provider-unsupported result
- **AND** SHALL NOT add it to the provider registry or execute its declared executable

### Requirement: Provider conformance test kit
The SDK SHALL provide one reusable conformance suite that verifies deterministic registration, duplicate-id rejection, side-effect-free availability, launch and prompt mapping, cancellation, output parsing, resume, unsupported capabilities, sensitive-argument redaction, version-detection failure, manifest agreement, and classified provider errors.

#### Scenario: Verify all built-in providers
- **WHEN** the conformance suite runs for `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, and `antigravity-cli`
- **THEN** every provider SHALL satisfy the same mandatory contract cases
- **AND** provider-specific fixture expectations SHALL remain inside that provider's adapter tests

#### Scenario: Verify a fixture provider
- **WHEN** a test-only fixture provider is added through the static test registry
- **THEN** it SHALL pass the same mandatory conformance suite without changing generic Session orchestration
- **AND** it SHALL NOT appear in the production registry

### Requirement: Provider parser conformance
An SDK parser SHALL accept independently chunked stdout and stderr bytes, structured JSON events when declared, and text fallback, and SHALL normalize session ids, usage, tool events, incremental content, completion, and failure into the existing runtime event vocabulary. Parsing SHALL be invariant to valid chunk partitioning, including partial UTF-8 boundaries, and SHALL use bounded buffers.

#### Scenario: Parse arbitrarily partitioned output
- **WHEN** equivalent valid provider output is divided at different byte boundaries
- **THEN** the parser SHALL emit an equivalent ordered normalized result
- **AND** split UTF-8 code points and partial structured records SHALL NOT be lost or corrupted

#### Scenario: Parse text fallback
- **WHEN** a provider declares text fallback and emits non-structured valid text
- **THEN** the parser SHALL emit bounded text increments and a terminal outcome according to its adapter contract

#### Scenario: Reject oversized or malformed records
- **WHEN** undecoded output exceeds the SDK buffer or record limit or violates a provider's structured protocol
- **THEN** parsing SHALL terminate with a classified bounded protocol error
- **AND** SHALL NOT retain unbounded output or expose raw sensitive content

### Requirement: Provider SDK developer documentation
The repository SHALL document the provider contract, manifest schema, test-only example provider, conformance workflow, compatibility policy, and security restrictions under `docs/provider-sdk/`.

#### Scenario: Developer evaluates a provider adapter
- **WHEN** a developer follows the Provider SDK documentation
- **THEN** the documentation SHALL identify every mandatory contract method and conformance case
- **AND** SHALL state that external providers and executable manifest hooks are unsupported

### Requirement: Provider SDK compatibility and performance evidence
The SDK SHALL preserve existing built-in invocation and event compatibility and SHALL include reproducible evidence for bounded parser memory, chunk-partition correctness, parser throughput, and deterministic registry resolution without establishing the global runtime budgets reserved for a later change.

#### Scenario: Measure fixed provider fixtures
- **WHEN** the SDK benchmark suite runs against fixed repository fixtures
- **THEN** it SHALL report parser throughput and registry resolution measurements with fixture and environment context
- **AND** correctness tests SHALL enforce declared buffer limits independently of wall-clock timing

