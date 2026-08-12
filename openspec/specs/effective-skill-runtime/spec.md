# effective-skill-runtime Specification

## Purpose
Defines the effective Skill catalog and safe read-only loading behavior used by agents and management surfaces across project, user, registry, and system layers.

## Requirements

### Requirement: Independent Skill classification dimensions
The system SHALL represent each Skill with an independent `type`, `delivery`, `layer`, `origin`, trust state, and availability state. `type` SHALL be `role` or `utility`; `delivery` SHALL be `eager` or `on-demand`; and `layer` SHALL be `project`, `user`, `registry`, or `system`.

#### Scenario: Explicit role metadata
- **WHEN** a valid Skill declares `type: role` and `delivery: on-demand`
- **THEN** the effective catalog SHALL expose both values without deriving either value from its storage layer or origin

#### Scenario: Legacy metadata compatibility
- **WHEN** a previously supported Skill omits the new classification fields
- **THEN** the system SHALL classify it as a `role` Skill with `eager` delivery
- **AND** SHALL identify that classification as a compatibility default

#### Scenario: Utility execution unavailable
- **WHEN** a Skill declares `type: utility` before delegated Utility execution is available
- **THEN** the catalog SHALL expose the Skill with an unavailable reason
- **AND** the system SHALL NOT inject or execute it as a Role Skill

### Requirement: Four-layer effective Skill resolution
The system SHALL resolve one effective Skill per canonical Skill id using the precedence `project > user > registry > system`. Shadowed definitions SHALL remain observable for management and diagnostics but SHALL NOT participate in agent loading or prompt assembly.

#### Scenario: Project definition shadows lower layers
- **WHEN** the same canonical Skill id exists in project, user, registry, and system layers
- **THEN** the project definition SHALL be effective for that workspace
- **AND** the other definitions SHALL be reported as shadowed in precedence order

#### Scenario: Resolution without workspace
- **WHEN** the effective catalog is requested without an active workspace
- **THEN** the system SHALL exclude project-layer definitions and resolve from user, registry, and system layers

#### Scenario: Deterministic duplicate in one layer
- **WHEN** multiple valid definitions in the same layer claim the same canonical Skill id
- **THEN** the system SHALL select a deterministic winner
- **AND** SHALL mark the conflicting definitions unavailable with a safe diagnostic rather than loading an arbitrary definition

#### Scenario: Excluded traversal directories
- **WHEN** project Skill discovery encounters `.git`, `.venv`, `node_modules`, `__pycache__`, `build`, `dist`, or `target`
- **THEN** it SHALL NOT traverse those directories while discovering Skill packages

### Requirement: Canonical Skill ids and aliases
The system SHALL resolve Skill requests by canonical kebab-case id or registered alias. An exact canonical id match SHALL take precedence over alias matching, and ambiguous aliases SHALL be rejected.

#### Scenario: Alias resolves canonical Skill
- **WHEN** `load_skill` receives an unambiguous alias such as `dev`
- **THEN** it SHALL load the effective Skill registered under the alias's canonical id
- **AND** SHALL return the canonical id in the result

#### Scenario: Canonical id wins over alias
- **WHEN** a request value is both an exact canonical id and an alias for another Skill
- **THEN** the exact canonical id SHALL be selected

#### Scenario: Ambiguous alias rejected
- **WHEN** an alias maps to more than one effective canonical Skill
- **THEN** the system SHALL reject the request without loading either Skill
- **AND** SHALL return the conflicting canonical ids

### Requirement: Immutable system Skill packages
System-layer Skill packages SHALL be read-only authoritative application resources. Operations that mutate, delete, import over, or directly restore their content SHALL be rejected without changing the package.

#### Scenario: Preview system Skill
- **WHEN** a user or agent requests a system Skill preview
- **THEN** the system SHALL return its effective readable content and immutable layer metadata

#### Scenario: Direct edit rejected
- **WHEN** a caller attempts to edit a system Skill package
- **THEN** the system SHALL reject the operation with guidance that customization requires a higher-layer definition

#### Scenario: Disable without mutation
- **WHEN** a user disables an effective system Skill
- **THEN** the system SHALL persist enablement state separately from the immutable package
- **AND** SHALL leave the package content unchanged

### Requirement: Fixed read-only Skill tools
Native API agents SHALL receive fixed-schema `list_skills`, `load_skill`, and `read_skill_resource` tools. These tools SHALL read only from the effective catalog for the active session context and SHALL NOT mutate Skill packages, bindings, or configuration.

#### Scenario: List effective Skills
- **WHEN** an agent calls `list_skills`
- **THEN** the result SHALL include bounded metadata for effective Skills, including canonical id, name, description, type, delivery, layer, availability, and aliases
- **AND** SHALL exclude instruction bodies

#### Scenario: Load on-demand role
- **WHEN** an agent calls `load_skill` for an enabled, available on-demand Role Skill
- **THEN** the result SHALL include its effective instructions, canonical identity, logical base URI, and bounded resource index

#### Scenario: Refuse unavailable Skill
- **WHEN** an agent calls `load_skill` for a disabled, invalid, conflicting, or unsupported Skill
- **THEN** the tool SHALL return a structured refusal with a safe unavailable reason and no instruction body

#### Scenario: Read indexed resource
- **WHEN** an agent calls `read_skill_resource` with a logical URI returned by `load_skill`
- **THEN** the system SHALL return bounded content only when the resource belongs to the same effective Skill package and is an allowed readable file

### Requirement: Bounded progressive disclosure
`load_skill` SHALL return at most 12,000 Unicode characters of inline instructions and SHALL use logical `skill://` URIs for package and resource references. Resource indexes and resource reads SHALL be bounded and deterministic.

#### Scenario: Instruction body exceeds inline limit
- **WHEN** effective Skill instructions exceed 12,000 Unicode characters
- **THEN** `load_skill` SHALL return a prefix no longer than the limit
- **AND** SHALL identify the result as truncated and provide the logical URI needed for continued reading

#### Scenario: Resource directories indexed
- **WHEN** a Skill package contains readable files under `scripts`, `references`, `templates`, or `assets`
- **THEN** `load_skill` SHALL return a bounded deterministic index of logical resource URIs grouped by directory

#### Scenario: Base directory placeholder
- **WHEN** effective instructions contain `{skill_base_dir}`
- **THEN** the loaded content SHALL replace it with the package's logical base URI rather than an unrestricted host filesystem path

#### Scenario: Resource traversal rejected
- **WHEN** a resource request contains an absolute path, parent traversal, an unindexed path, a hidden path component, or a path outside the effective package
- **THEN** the system SHALL reject the request without reading the target

#### Scenario: Binary or oversized resource refused
- **WHEN** a requested Skill resource is binary or exceeds the configured read limit
- **THEN** the system SHALL refuse its content and return bounded safe metadata

### Requirement: Sidecar Skill usage tracking
The system SHALL track Skill view and use activity separately from immutable Skill content. Tracking failures SHALL NOT prevent an otherwise valid Skill load, and corrupt tracking state SHALL be recoverable.

#### Scenario: Successful load increments view
- **WHEN** `load_skill` successfully returns a Skill's instructions
- **THEN** the system SHALL increment its view count and update its last-viewed timestamp

#### Scenario: Eager inclusion increments use
- **WHEN** an eager Skill is included in an agent generation prompt
- **THEN** the system SHALL increment its use count and update its last-used timestamp once for that generation

#### Scenario: Corrupt usage sidecar
- **WHEN** usage state cannot be parsed
- **THEN** the system SHALL preserve a recoverable backup, replace the active state with valid empty records, and report a redacted warning through unified logging

#### Scenario: Tracking write fails
- **WHEN** usage tracking cannot be persisted after a successful Skill load
- **THEN** the Skill load SHALL still succeed
- **AND** the failure SHALL be reported through unified logging without logging Skill content

### Requirement: Safe Skill diagnostics
Skill discovery, resolution, loading, migration, and usage diagnostics SHALL use the unified logging service and SHALL NOT record instruction bodies, resource contents, secrets, or unrestricted host paths.

#### Scenario: Invalid package diagnostic
- **WHEN** a Skill package fails validation
- **THEN** the system SHALL log a safe Skill identity, layer, operation, and reason code at the appropriate level
- **AND** SHALL omit the package body and sensitive path data
