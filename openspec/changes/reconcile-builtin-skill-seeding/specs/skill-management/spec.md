## MODIFIED Requirements

### Requirement: Built-in Skill seeds
The system SHALL provide six built-in Skills: `tdd-discipline`, `code-review`, `code-security-scan`, `api-doc-generation`, `unit-test-generation`, and `readme-generation`. Built-in initialization SHALL reconcile the registry with what already exists on disk rather than assuming an empty filesystem, and SHALL report a per-Skill outcome so one Skill's failure cannot leave the others unregistered.

#### Scenario: Idempotent built-in initialization
- **WHEN** built-in Skill initialization runs more than once
- **THEN** the system SHALL NOT create duplicate registry records or duplicate Skill directories

#### Scenario: Deleted built-in is not auto-restored
- **WHEN** a user deletes a built-in Skill and built-in initialization runs later
- **THEN** the system SHALL keep the Skill deleted until the user explicitly restores it

#### Scenario: Restore built-in Skill
- **WHEN** a user restores a deleted built-in Skill
- **THEN** the system SHALL recreate the standard `SKILL.md`, registry record, and source directory for that built-in Skill

#### Scenario: Adopt an existing source that has no registry record
- **WHEN** built-in initialization finds a built-in Skill's source directory already present while no registry record exists for it, and the Skill is not marked deleted
- **THEN** the system SHALL register the existing source instead of failing
- **AND** it SHALL leave the on-disk `SKILL.md` unmodified
- **AND** the resulting record SHALL describe the content that is actually on disk

#### Scenario: Adopted content that diverges from the shipped definition is reported, not overwritten
- **WHEN** an adopted source's content differs from the shipped built-in definition
- **THEN** the system SHALL surface that difference through Skill drift reporting
- **AND** it SHALL NOT silently replace the user's file

#### Scenario: One unusable built-in does not block the rest
- **WHEN** initialization cannot register one built-in Skill
- **THEN** the system SHALL still register every other built-in Skill it can
- **AND** it SHALL report which Skills succeeded and which failed, with a reason for each failure

#### Scenario: An already-present built-in is not an error
- **WHEN** initialization encounters a built-in Skill whose source is already present
- **THEN** the system SHALL NOT emit an `error`-level log for that condition
- **AND** any diagnostic it does emit SHALL be attributed to the operation that produced it

## ADDED Requirements

### Requirement: Unregistered Skill sources are repairable
The system SHALL resolve an `UnregisteredSource` drift issue by adopting the existing source into the registry, so that a source directory present on disk without a registry record does not remain permanently unusable.

#### Scenario: Synchronization adopts an unregistered source
- **WHEN** Skill synchronization runs and reports an `UnregisteredSource` issue for a source directory
- **THEN** the system SHALL register that source and clear the issue
- **AND** the Skill SHALL become listable, bindable, and mountable like any other registered Skill

#### Scenario: Adoption does not resurrect an intentionally deleted built-in
- **WHEN** an unregistered source belongs to a built-in Skill the user has deleted
- **THEN** the system SHALL leave it unregistered
- **AND** the existing intentional-deletion behavior SHALL continue to apply

#### Scenario: A failed adoption is reported rather than retried forever
- **WHEN** adopting an unregistered source fails
- **THEN** the system SHALL report the failure with its reason
- **AND** it SHALL NOT leave the user without a way to see why the Skill is absent
