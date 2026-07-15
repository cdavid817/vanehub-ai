## ADDED Requirements

### Requirement: Observable SDK operations
SDK install, update, rollback, and uninstall operations SHALL run through the observable operation model when executed by the native runtime.

#### Scenario: SDK operation starts
- **WHEN** a user starts an SDK install, update, rollback, or uninstall
- **THEN** the SDK service SHALL expose an operation id and initial operation status

#### Scenario: SDK operation logs persist
- **WHEN** an SDK operation emits npm output, validation errors, completion, or failure
- **THEN** the system SHALL make logs available through the SDK service with the related SDK id and operation id

### Requirement: SDK storage uses native storage foundation
Managed SDK dependency metadata SHALL use the native runtime storage foundation for VaneHub-owned paths and migration-managed metadata.

#### Scenario: Read SDK status after migration
- **WHEN** the SDK service lists statuses after native storage migrations have run
- **THEN** it SHALL read SDK metadata from the VaneHub-owned storage path and return statuses through the SDK service boundary
