## ADDED Requirements

### Requirement: The native host SHALL discover compatible Python interpreters without mutating the machine

The local-media capability SHALL expose a bounded, explicitly requested discovery operation that returns installed Python interpreter candidates supported by the shipped local-media bridge. Discovery SHALL validate that each candidate is an executable Python runtime, report its normalized version and compatibility, order and deduplicate results deterministically, and perform no inference, package installation, model access, network access, profile mutation, or worker launch.

#### Scenario: Compatible interpreters are installed

- **WHEN** the desktop client requests Python environment discovery
- **THEN** the result SHALL contain each distinct compatible interpreter at most once with its executable path, normalized version, compatibility state, and safe discovery source
- **AND** compatible candidates SHALL appear before incompatible candidates in deterministic order

#### Scenario: A discovered command is not a usable Python runtime

- **WHEN** a candidate cannot be executed within the discovery timeout, does not identify itself as Python, returns an unsupported version, or resolves outside its claimed executable identity
- **THEN** discovery SHALL omit it or report it as incompatible with a stable reason
- **AND** raw process output and operating-system errors SHALL NOT cross the service boundary

#### Scenario: No compatible interpreter exists

- **WHEN** bounded discovery completes without a compatible candidate
- **THEN** the operation SHALL return an empty compatible set with stable manual-configuration guidance
- **AND** it SHALL NOT install Python or alter the saved local-media profile

#### Scenario: Discovery is requested in Web mode

- **WHEN** the production Web adapter receives a Python discovery request without a native host
- **THEN** it SHALL return a native-only unavailable result through the same service contract
- **AND** it SHALL NOT invent host interpreter candidates

### Requirement: A detected interpreter SHALL become active only through explicit profile selection and save

Interpreter discovery SHALL be advisory. Selecting a candidate SHALL update only the local-media settings draft for the engines chosen by the user, and the selected absolute executable path SHALL become available to probes and workers only after the ordinary validated optimistic-concurrency save succeeds. Discovery SHALL NOT silently replace a configured interpreter and worker launch SHALL NOT fall back to another detected interpreter.

#### Scenario: The user selects one interpreter for an engine

- **WHEN** the user selects a compatible detected interpreter for OCR, STT, or TTS
- **THEN** only the chosen engine draft SHALL receive that executable path
- **AND** active workers and saved profile revisions SHALL remain unchanged until save succeeds

#### Scenario: The user applies one interpreter to multiple engines

- **WHEN** the user explicitly chooses to apply a compatible candidate to multiple engine drafts
- **THEN** the same discovered executable path SHALL be copied only to the selected drafts
- **AND** each engine's model and device configuration SHALL remain unchanged

#### Scenario: A configured interpreter is no longer discovered

- **WHEN** discovery does not return the interpreter already stored in the profile
- **THEN** the system SHALL retain the stored selection and mark it as not currently detected
- **AND** it SHALL require an explicit user selection and save before replacing it

#### Scenario: Runtime launch encounters an invalid saved interpreter

- **WHEN** an engine probe or inference cannot execute the interpreter captured in its saved profile snapshot
- **THEN** the operation SHALL fail with the existing stable Python configuration error
- **AND** it SHALL NOT retry with another discovered candidate
