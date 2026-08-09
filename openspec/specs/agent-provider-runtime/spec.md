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
