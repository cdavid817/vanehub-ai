## MODIFIED Requirements

### Requirement: Independent Skill classification dimensions
The system SHALL represent each Skill with an independent `type`, `delivery`, `layer`, `origin`, trust state, and availability state. `type` SHALL be `role` or `utility`; `delivery` SHALL be `eager` or `on-demand`; and `layer` SHALL be `project`, `user`, `registry`, or `system`. Utility availability SHALL reflect whether the active runtime can delegate that effective definition and SHALL remain independent from Role Skill loading.

#### Scenario: Explicit role metadata
- **WHEN** a valid Skill declares `type: role` and `delivery: on-demand`
- **THEN** the effective catalog SHALL expose both values without deriving either value from its storage layer or origin

#### Scenario: Legacy metadata compatibility
- **WHEN** a previously supported Skill omits the new classification fields
- **THEN** the system SHALL classify it as a `role` Skill with `eager` delivery
- **AND** SHALL identify that classification as a compatibility default

#### Scenario: Utility execution supported
- **WHEN** a valid Utility Skill is effective and the active native runtime supports delegated execution
- **THEN** the catalog SHALL expose it as available for delegation
- **AND** the system SHALL NOT inject or load it as a Role Skill

#### Scenario: Utility execution unavailable
- **WHEN** a Utility Skill is viewed through a runtime that cannot delegate it
- **THEN** the catalog SHALL retain the Skill with a runtime-specific unavailable reason
- **AND** the system SHALL NOT inject or execute it as a Role Skill

## ADDED Requirements

### Requirement: Fixed Utility delegation discovery
Native API Agents that support Utility delegation SHALL discover eligible effective Utility Skills through bounded metadata and SHALL invoke them only through the fixed delegation operation. The existing `load_skill` operation SHALL continue to refuse Utility instruction bodies.

#### Scenario: List delegatable Utility
- **WHEN** a supported native API Agent lists Skills in an active workspace
- **THEN** eligible Utility entries SHALL identify delegation as their supported operation without returning instruction bodies

#### Scenario: Utility load remains refused
- **WHEN** an Agent calls `load_skill` for an otherwise delegatable Utility Skill
- **THEN** the system SHALL return a structured refusal directing the Agent to the delegation operation

