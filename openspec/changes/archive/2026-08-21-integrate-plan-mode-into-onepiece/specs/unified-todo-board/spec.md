## MODIFIED Requirements

### Requirement: Multi-source identity and projection
The system SHALL allow one work item to link Session and Scheduled Task sources and SHALL project current source status separately from the user-controlled board stage.

#### Scenario: Link multiple sources
- **WHEN** a work item links a Session and a Scheduled Task
- **THEN** the same card SHALL expose both source identities and their current statuses
- **AND** it SHALL match both Session and Scheduled Task source filters

#### Scenario: Runtime status changes
- **WHEN** a linked source changes runtime status
- **THEN** the card SHALL refresh its source projection
- **AND** the work item's board stage SHALL remain unchanged

#### Scenario: Source becomes unavailable
- **WHEN** a linked source is deleted or cannot be resolved
- **THEN** the work item SHALL remain available with an unavailable-source indication

### Requirement: Automatic source reconciliation
The system SHALL idempotently reconcile existing and future top-level Sessions and Scheduled Tasks into work items without producing duplicate cards.

#### Scenario: Reconcile existing sources
- **WHEN** the board is first loaded after upgrade
- **THEN** each eligible unlinked source SHALL be represented by exactly one work item

#### Scenario: Reconcile new sources
- **WHEN** an eligible Session or Scheduled Task is created after upgrade
- **THEN** a subsequent board reconciliation SHALL create or update its work item

#### Scenario: Suppress child Session duplication
- **WHEN** a Session was created for a Scheduled Task run
- **THEN** it SHALL appear as activity under the owning work item
- **AND** it SHALL NOT create an independent top-level work item

#### Scenario: Preserve archived reconciliation
- **WHEN** a source linked to an archived work item is reconciled again
- **THEN** the system SHALL retain the archived work item and SHALL NOT create a replacement

### Requirement: Work item archive lifecycle
The system SHALL allow work items to be archived, restored, and permanently deleted independently of their linked sources.

#### Scenario: Archive work item
- **WHEN** a user archives a work item
- **THEN** the item SHALL leave the default board while all linked sources remain unchanged

#### Scenario: Restore work item
- **WHEN** a user restores an archived work item
- **THEN** it SHALL return to its persisted stage and ordering position

#### Scenario: Permanently delete work item
- **WHEN** a user permanently deletes an archived work item
- **THEN** only the work item and its source links SHALL be deleted
- **AND** linked Sessions and Scheduled Tasks SHALL remain unchanged
