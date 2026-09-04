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

### Requirement: Unified board toolbar and saved views
The unified work board SHALL use the shared management toolbar for search, filters, sort, view, active filter chips, and versioned saved views without placing the create or edit form in the page header.

#### Scenario: Apply filters
- **WHEN** the user changes text, Agent, project, source, priority, due, or status filters
- **THEN** the visible query SHALL update without changing the selected view mode or current item
- **AND** active filters SHALL be individually removable

#### Scenario: Save a view
- **WHEN** the user saves the current supported query and presentation
- **THEN** the view SHALL receive a stable local identity and bounded name
- **AND** secrets and unrestricted content SHALL not be stored

#### Scenario: Open a saved view
- **WHEN** the user selects a saved view
- **THEN** the board SHALL restore its supported filters, sort, grouping, and presentation
- **AND** stale filter values SHALL be retained and identified rather than silently replaced

#### Scenario: Create work item
- **WHEN** the user activates the primary New work item action
- **THEN** a shared editor sheet SHALL open
- **AND** the board header SHALL remain a compact navigation and query surface

### Requirement: Nonblocking work-item mutations
Work-item create, update, stage, priority, assignment, link, and archive mutations SHALL affect only the target entity and conflicting actions while preserving the rest of the loaded board.

#### Scenario: Move one card
- **WHEN** a stage mutation begins
- **THEN** the card MAY move optimistically when rollback is safe and SHALL show a local pending state
- **AND** other cards, columns, filters, and navigation SHALL remain operable

#### Scenario: Mutation fails
- **WHEN** the service rejects an optimistic or canonical mutation
- **THEN** the target card SHALL reconcile or roll back to canonical state and show a local retryable error
- **AND** the board SHALL not clear and fully reload as the only feedback

#### Scenario: Mutation races
- **WHEN** another client or process changes the same versioned work item
- **THEN** the UI SHALL show the current canonical item and offer an explicit reapply or reload path rather than overwriting silently

### Requirement: Work-item editor sheet
Create and edit workflows SHALL use one accessible sheet or dialog with structured fields, validation, relationship pickers, reviewable consequences, and stable focus return.

#### Scenario: Create from board
- **WHEN** the editor opens in create mode
- **THEN** it SHALL preserve the current board query and scroll position behind the modal surface

#### Scenario: Edit a card
- **WHEN** the editor opens for a selected work item
- **THEN** it SHALL load the canonical current version and keep validation errors adjacent to fields

#### Scenario: Close the editor
- **WHEN** the user saves, discards, or cancels
- **THEN** focus SHALL return to the originating card or primary create action
- **AND** the board selection SHALL remain valid when possible

### Requirement: Canonical and accessible stage movement
The board SHALL provide one canonical stage-change command with drag, keyboard, touch, and action-menu entry points that produce equivalent service mutations.

#### Scenario: Drag a card
- **WHEN** a pointer user drops a card on an eligible stage
- **THEN** the board SHALL invoke the same stage-change command used by non-drag paths
- **AND** the drop target and pending result SHALL be visibly identified

#### Scenario: Move without dragging
- **WHEN** a user activates Move to from the card menu or keyboard command
- **THEN** a localized stage picker SHALL expose eligible destinations and current stage
- **AND** choosing a destination SHALL run the same mutation

#### Scenario: Reject an invalid move
- **WHEN** domain rules or current version disallow a destination
- **THEN** that destination SHALL be absent or disabled with an explanation
- **AND** the board SHALL not fabricate a local transition

### Requirement: Board batch management
The board SHALL support explicit multi-selection with bounded batch actions for operations that the domain service safely supports.

#### Scenario: Enter batch mode
- **WHEN** the user starts batch management
- **THEN** visible cards SHALL expose selection controls and a shared selected-count action bar
- **AND** normal drag activation SHALL not conflict with selection

#### Scenario: Move selected items
- **WHEN** all selected items are eligible for the target stage
- **THEN** the UI SHALL submit bounded operations and report per-item outcomes
- **AND** one failure SHALL not hide successful canonical updates

#### Scenario: Mixed eligibility
- **WHEN** some selected items cannot perform the chosen action
- **THEN** the confirmation SHALL identify eligible and ineligible counts before submission

### Requirement: Responsive stage-list presentation
At compact widths the board SHALL offer a grouped stage-list presentation that preserves search, filters, item detail, creation, and stage movement without requiring horizontal multi-column dragging.

#### Scenario: Use compact width
- **WHEN** the board cannot keep readable columns
- **THEN** stage groups SHALL render vertically with counts and collapsible content
- **AND** the user SHALL be able to open and move items without horizontal scrolling

#### Scenario: Return to wide width
- **WHEN** the board can render columns again
- **THEN** the prior view preference MAY be restored
- **AND** selected item, filters, and saved view SHALL remain unchanged

### Requirement: Bounded work-item card metadata
Work-item cards SHALL prioritize title, actionable state, stage context, and at most three secondary metadata groups while keeping complete detail available in the editor or Inspector.

#### Scenario: Render a dense card
- **WHEN** priority, project, source, due date, Agent, relationship, and path metadata all exist
- **THEN** the card SHALL apply the documented metadata budget and move overflow detail out of the default card body

#### Scenario: Render path metadata
- **WHEN** a project or workspace path is shown
- **THEN** the board SHALL use the existing safe normalized display path while retaining canonical identity for filtering and navigation

#### Scenario: Show state
- **WHEN** priority, overdue, blocked, archived, or attention state is visible
- **THEN** the state SHALL be understandable without color alone

### Requirement: Optional board WIP guidance
The board MAY display configured work-in-progress limits and warnings, and any displayed limit SHALL NOT change canonical stage-transition rules unless an owning service explicitly enforces them.

#### Scenario: Column reaches a visual WIP limit
- **WHEN** a configured presentation limit is reached
- **THEN** the column SHALL show a bounded warning and count
- **AND** the UI SHALL not reject a move unless the domain service reports that policy

#### Scenario: No limit exists
- **WHEN** a stage has no reliable WIP configuration
- **THEN** the board SHALL omit the limit rather than inventing one

