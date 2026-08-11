## ADDED Requirements

### Requirement: Immutable Registry-layer Skill packages
Skills installed from a registry SHALL be represented as immutable Registry-layer definitions with source id, publisher namespace, package id, Skill id, semantic version, content hash, metadata witness, installed snapshot id, compatibility, revocation, and lifecycle state. Editing the base content in place MUST be rejected; customization SHALL use Overlay or an explicit fork into User/Project scope.

#### Scenario: User attempts to edit installed base content
- **WHEN** an update request targets the `SKILL.md` or support files of a Registry snapshot
- **THEN** the system rejects the mutation and offers Overlay or fork workflows without changing the immutable package

#### Scenario: Registry package participates in resolution
- **WHEN** a healthy installed Registry definition shares an id with no Project or User definition
- **THEN** it participates below Project/User and above System according to effective Skill precedence

### Requirement: Registry package lifecycle integrates with effective state
Install, update, rollback, uninstall, revocation, source disablement, integrity drift, and recovery SHALL atomically refresh effective Skill resolution. In-flight contexts MAY retain their immutable snapshot unless a critical revocation requires cancellation; new contexts MUST use only the current eligible snapshot.

#### Scenario: Installed files drift locally
- **WHEN** an active Registry snapshot no longer matches its recorded content manifest
- **THEN** it becomes ineligible for new loads and is recoverable only by verified reinstall, rollback, or uninstall rather than drift adoption

#### Scenario: Registry source is disabled
- **WHEN** a user disables a source with installed packages
- **THEN** catalog refresh and new installs stop while installed packages remain locally governed and their source-disabled status is visible

### Requirement: Registry uninstall preserves user-owned state
Uninstalling a Registry snapshot SHALL remove only verified application-owned package files and registry install records. It SHALL preserve User/Project forks, Overlay history, Skill configuration, usage/audit history, and unrelated cache objects unless the user separately chooses an eligible cleanup action.

#### Scenario: Uninstall customized Registry Skill
- **WHEN** a Registry Skill has Overlay or configuration state and the user confirms uninstall
- **THEN** the preview identifies retained user-owned state and uninstall does not delete it

