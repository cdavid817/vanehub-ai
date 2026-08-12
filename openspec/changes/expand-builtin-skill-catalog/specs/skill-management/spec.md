## MODIFIED Requirements

### Requirement: Built-in Skill seeds
The system SHALL provide the exact 28-package first-party catalog defined by `builtin-skill-catalog` as immutable System packages. Startup reconciliation SHALL preserve the six existing canonical ids and state, add the 22 new packages idempotently, preserve higher-layer overrides and intentional deletion or disablement, and report per-Skill outcomes so one package failure cannot block the remaining catalog.

#### Scenario: Idempotent built-in initialization
- **WHEN** 28-package catalog reconciliation runs more than once
- **THEN** the system SHALL NOT create duplicate System definitions, registry records, overrides, aliases, tombstones, usage records, Overlay records, or package directories

#### Scenario: Deleted built-in is not auto-restored
- **WHEN** a user intentionally deleted or hid an existing or newly introduced first-party Skill and reconciliation runs later
- **THEN** the system SHALL preserve that intent and SHALL NOT make the package effective automatically

#### Scenario: Restore built-in Skill
- **WHEN** a user restores a deleted first-party Skill
- **THEN** the system SHALL clear deletion intent and expose the immutable System package unless a higher-layer definition is effective
- **AND** SHALL NOT create a mutable copy of the System package

#### Scenario: Adopt an existing source that has no registry record
- **WHEN** reconciliation finds a legacy first-party source directory with no registry record and no deletion marker
- **THEN** the system SHALL inspect the existing source without modifying it
- **AND** SHALL preserve divergent valid content as a higher-priority User definition
- **AND** SHALL represent content identical to the shipped package through the immutable System definition

#### Scenario: Adopted content that diverges from the shipped definition is reported, not overwritten
- **WHEN** existing valid content for any first-party canonical id differs from its shipped System package
- **THEN** the system SHALL preserve that content as a User-layer override and report the migration or reconciliation outcome
- **AND** SHALL NOT silently replace the user's files, bindings, usage, Overlay, or enabled state

#### Scenario: One unusable built-in does not block the rest
- **WHEN** validation or reconciliation cannot make one first-party package usable
- **THEN** the system SHALL still reconcile every other package it can
- **AND** SHALL name succeeded, unavailable, and failed package ids with safe reasons

#### Scenario: An already-present built-in is not an error
- **WHEN** reconciliation encounters an already-reconciled first-party package with matching manifest identity and hashes
- **THEN** the system SHALL treat it as an idempotent success rather than an error
- **AND** any diagnostic SHALL be attributed to catalog reconciliation

#### Scenario: Unchanged legacy source cleanup
- **WHEN** a legacy mutable built-in source exactly matches its shipped System package and its state has been migrated successfully
- **THEN** the system SHALL stop treating that source as authoritative
- **AND** SHALL remove it only through a recoverable, idempotent migration step after retaining the state needed for rollback

#### Scenario: Expanded catalog preserves existing state
- **WHEN** a user upgrades from the six-package catalog to the 28-package catalog
- **THEN** the six existing canonical ids SHALL retain bindings, enabled state, deletion intent, usage, aliases, and Overlay association
- **AND** the 22 new packages SHALL begin with explicit first-party defaults without being assigned silently to every Agent

#### Scenario: Utility dependency unavailable
- **WHEN** a new Utility package requires delegation or another capability not present in the running version
- **THEN** reconciliation SHALL keep the package visible with an unavailable reason rather than failing catalog initialization or treating it as a Role

## ADDED Requirements

### Requirement: First-party catalog summary
Skill management overview responses SHALL provide bounded totals grouped by first-party versus other origin, Role versus Utility, category, delivery, available versus unavailable, dependency reason, assigned versus unassigned, and overridden versus unshadowed state.

#### Scenario: Catalog summary loaded
- **WHEN** a client requests the Skill overview after successful catalog reconciliation
- **THEN** the response SHALL report exactly 28 first-party canonical packages before higher-layer shadowing and SHALL distinguish their effective states

#### Scenario: Higher-layer override
- **WHEN** a User or Project definition shadows a first-party package
- **THEN** the summary SHALL count one first-party base package and one effective overridden identity without counting two active Skills

### Requirement: First-party package detail
Skill detail and preview responses SHALL expose first-party category, type, delivery, aliases, package version, dependencies, required modalities, resource counts, body-budget status, validation status, effective layer, and immutable System origin.

#### Scenario: Preview first-party package
- **WHEN** a client previews a first-party Skill
- **THEN** it SHALL receive bounded metadata, effective instructions, resource index, dependency status, and immutable-state information through the service boundary

#### Scenario: Package unavailable by dependency
- **WHEN** a package has an unmet declared dependency
- **THEN** detail responses SHALL identify the dependency and setup guidance without exposing credentials or triggering the dependency
