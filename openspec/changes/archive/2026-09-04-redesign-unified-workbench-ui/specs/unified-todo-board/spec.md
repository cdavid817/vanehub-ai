# unified-todo-board Specification Delta

## ADDED Requirements

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
