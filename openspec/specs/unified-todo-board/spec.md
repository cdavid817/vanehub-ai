# unified-todo-board Specification

## Purpose
Defines one durable, runtime-neutral workspace for organizing manual work together with Session and Scheduled Task sources while preserving each source's independent lifecycle.
## Requirements
### Requirement: Durable unified work item
The system SHALL persist work items with stable identity, title, description, stage, priority, relative order, optional project path and due timestamp, archive state, timestamps, and zero or more source links.

#### Scenario: Create manual work
- **WHEN** a user creates a work item without a runtime source
- **THEN** the system SHALL place it in Inbox and persist it across restart

#### Scenario: Organize a work item
- **WHEN** a user edits, prioritizes, reorders, or moves a work item
- **THEN** the system SHALL persist the updated board metadata without mutating linked source lifecycle state

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

### Requirement: Board discovery and filtering
The board SHALL provide search plus combinable source, stage, priority, project, and archive filters.

#### Scenario: Filter by source
- **WHEN** a user selects one or more source kinds
- **THEN** the board SHALL show work items linked to any selected source kind while retaining multi-source cards as single cards

#### Scenario: Default active view
- **WHEN** the board opens without an explicit archive filter
- **THEN** it SHALL exclude archived work items and display active items grouped by stage

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

### Requirement: Runtime-neutral board service
All board operations SHALL use a frontend service boundary with contract-compatible Tauri desktop and Web/mock adapters.

#### Scenario: Desktop board operation
- **WHEN** React lists or mutates work items in desktop mode
- **THEN** it SHALL call a frontend service interface and Tauri invocation SHALL remain in the Tauri-specific adapter

#### Scenario: Web board operation
- **WHEN** the board runs in Web mode
- **THEN** the Web adapter SHALL provide equivalent creation, reconciliation, filtering, movement, and archive behavior without SQLite

### Requirement: Accessible responsive board interaction
The board SHALL support pointer, keyboard, and compact-layout operation without requiring drag and drop.

#### Scenario: Move without dragging
- **WHEN** a keyboard or assistive-technology user changes a card stage or position
- **THEN** the board SHALL provide explicit controls that perform the same persisted mutation as pointer movement

#### Scenario: Compact viewport
- **WHEN** the available width cannot show all stages at once
- **THEN** the board SHALL preserve access to every stage, filter, card action, and source status without clipping required controls
