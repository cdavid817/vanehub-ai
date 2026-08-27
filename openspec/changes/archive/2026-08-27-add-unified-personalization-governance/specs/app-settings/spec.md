# app-settings Delta Specification

## REMOVED Requirements

### Requirement: Personalization settings model
**Reason**: The generic shared application settings model is no longer the runtime source of truth for custom instructions or long-term memory policy. Personalization policy moves to the dedicated revisioned personalization service, so the previous scenarios — which asserted whole-object settings persistence, a single host-level toggle triple, and settings-level oversize validation as the authority — no longer describe the system.

**Migration**: Replaced by "Personalization settings migration boundary" below, which defines the compatibility window, one-time migration, and migration-generation marker instead of whole-object personalization mutation.

## ADDED Requirements

### Requirement: Personalization settings migration boundary
The system SHALL ensure that the generic shared application settings model is no longer the runtime source of truth for custom instructions or long-term memory policy after dedicated-personalization migration completes. Personalization policy SHALL be persisted and mutated through the revisioned personalization service. During the compatibility window, the settings layer MAY retain legacy custom-instruction and memory fields for deserialization and one-time migration, and SHALL persist a migration-generation marker without exposing whole-object personalization mutation to the new UI.

#### Scenario: Fresh installation loads personalization
- **WHEN** no legacy personalization settings or dedicated policy exist
- **THEN** the personalization service SHALL create or resolve its validated default global policy
- **AND** generic `AppSettings` SHALL not need to create a second personalization configuration

#### Scenario: Existing installation migrates personalization
- **WHEN** legacy about-you, response-style, custom-instruction enablement, memory enablement, or tool-assisted extraction fields exist and migration has not completed
- **THEN** the native runtime SHALL migrate them idempotently into dedicated policy records
- **AND** SHALL mark the migration generation only after the policy transaction succeeds

#### Scenario: Restore migrated personalization
- **WHEN** the application restarts after migration completes
- **THEN** runtime personalization SHALL load from the dedicated personalization service
- **AND** legacy `AppSettings` values SHALL not override newer policy revisions

#### Scenario: New UI saves personalization
- **WHEN** the user changes custom instructions or memory policy in the AI Personalization page
- **THEN** React SHALL call the dedicated personalization service with a typed scope patch and expected revision
- **AND** SHALL not submit or replace the entire `AppSettings` aggregate

#### Scenario: Legacy whole-settings save occurs during compatibility
- **WHEN** an older internal caller saves an `AppSettings` object containing legacy personalization fields after migration
- **THEN** the settings layer SHALL preserve ordinary non-personalization settings
- **AND** SHALL not overwrite the dedicated personalization policy from those deprecated fields

#### Scenario: Preserve Web/mock parity
- **WHEN** personalization is loaded or saved in Web/mock mode
- **THEN** the Web adapter SHALL implement the dedicated personalization contract and migration-shaped defaults deterministically
- **AND** generic mock `AppSettings` SHALL not be treated as the authoritative policy store

#### Scenario: Migration cannot establish a valid policy
- **WHEN** legacy migration or dedicated policy loading fails before any validated policy exists
- **THEN** the application SHALL retain a localized maintenance warning
- **AND** personalization runtime behavior SHALL use fail-closed instruction and memory defaults without blocking unrelated application startup
