## ADDED Requirements

### Requirement: Per-Skill Overlay workspace
The Skills settings experience SHALL provide an Overlay area in Skill details that presents base and effective content, active scopes, trust, pinned state, mutation summaries, resource overrides, conflicts, and history without representing the Overlay as a separate active Skill.

#### Scenario: Open healthy Overlay details
- **WHEN** a user opens the Overlay area for a Skill with healthy active mutations
- **THEN** the page SHALL show the effective diff, active Overlay scopes, revision witnesses, mutation types, and affected resources

#### Scenario: Skill has no Overlay
- **WHEN** a user opens the Overlay area for a mutable or immutable Skill with no Overlay
- **THEN** the page SHALL explain that the base is unchanged and offer permitted manual Overlay actions

#### Scenario: Pinned Skill controls
- **WHEN** the selected Skill is pinned
- **THEN** the page SHALL render Overlay content and history read-only and SHALL explain that unpinning is required before mutation

### Requirement: Overlay mutation dialogs
The UI SHALL provide accessible service-backed dialogs for exact patches, learned-guidance blocks, supporting files, import review, trust promotion, disable, revert, and reconciliation. Dialogs SHALL preserve unsaved input after stale-witness or validation failures.

#### Scenario: Preview exact patch
- **WHEN** a user enters an exact patch
- **THEN** the dialog SHALL request a non-persisting replay preview and show match count, effective diff, scan result, and expected witnesses before enabling submission

#### Scenario: Stale mutation response
- **WHEN** submission fails because the Overlay revision or base hash is stale
- **THEN** the dialog SHALL remain open with the user's input and offer reload and re-preview without overwriting live state

#### Scenario: Imported Overlay review
- **WHEN** a user imports an Overlay package
- **THEN** the UI SHALL identify it as untrusted and show source metadata, hashes, scan results, base/effective diff, files, and conflicts before trust promotion is available

#### Scenario: Executable file rejected
- **WHEN** a user selects a prohibited script or executable file
- **THEN** the dialog SHALL show the safe rejection reason and SHALL NOT present the file as uploaded or effective

#### Scenario: Dialog accessibility
- **WHEN** an Overlay dialog opens or closes
- **THEN** it SHALL expose a localized accessible name, contain keyboard focus while open, support safe keyboard dismissal, and restore focus to its trigger

### Requirement: Overlay conflict reconciliation UI
The UI SHALL provide a three-way reconciliation view containing the witnessed base, current base, and proposed effective mutation, with per-conflict resolution and a final complete preview before commit.

#### Scenario: Resolve patch conflict
- **WHEN** a user edits a conflicted patch into a form that previews successfully
- **THEN** the UI SHALL show the resulting full effective diff and require explicit confirmation against current witnesses

#### Scenario: Ignore conflict
- **WHEN** a user chooses to ignore a conflict
- **THEN** the UI SHALL explain that the affected mutation will be disabled but retained in history before requesting confirmation

#### Scenario: Base changed during reconciliation
- **WHEN** the base or Overlay revision changes while reconciliation is open
- **THEN** submission SHALL fail safely and the UI SHALL retain edits while requiring a fresh comparison

### Requirement: Overlay history and rollback UI
The UI SHALL display a paginated, bounded history timeline with action, actor, scope, revision transition, trust, conflict, timestamp, and safe diff summary. Revert SHALL create a new revision and SHALL never appear to erase prior history.

#### Scenario: Inspect mutation history
- **WHEN** a user opens Overlay history
- **THEN** the page SHALL load bounded entries through the frontend service boundary and indicate any verification failure

#### Scenario: Revert active mutation
- **WHEN** a user confirms revert for an active mutation using current witnesses
- **THEN** the page SHALL submit a revert operation, refresh effective content and history, and show the newly created revision

