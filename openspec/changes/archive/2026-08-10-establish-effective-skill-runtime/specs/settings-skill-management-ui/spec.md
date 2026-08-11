## MODIFIED Requirements

### Requirement: Skill card controls
Each effective Skill inventory row SHALL use a progressive information hierarchy that keeps identity, enabled state, effective layer, Skill type, description, and the context-specific primary operation immediately scannable. Delivery, origin, trust, version, usage, compatibility, resource, and shadowing details SHALL remain available through bounded secondary text or the associated detail inspector. Mutation controls SHALL reflect whether the selected definition is mutable.

#### Scenario: Compact inventory remains bounded by Agent count
- **WHEN** the overview contains many compatible Agents
- **THEN** a Skill row SHALL NOT render the complete Agent checkbox matrix
- **AND** Agent-specific assignment SHALL be performed in the selected Agent view

#### Scenario: Render a scannable default row
- **WHEN** an effective Skill row renders before its details are selected
- **THEN** it SHALL emphasize the Skill name, enabled or paused state, effective layer, Skill type, description, and primary operation
- **AND** it SHALL NOT render delivery, origin, trust, usage, compatibility, resource, and every shadowed definition as an equally prominent badge set
- **AND** long identity or metadata values SHALL truncate without resizing the inventory or hiding row actions

#### Scenario: Toggle global Skill enabled state
- **WHEN** a user toggles an effective Skill enabled state from All Skills
- **THEN** the page SHALL explain the management scope to which enablement applies
- **AND** SHALL submit the change through the frontend service boundary, prevent a duplicate pending mutation, and refresh the effective overview
- **AND** SHALL preserve every existing CLI and API Agent assignment without assigning the Skill to any additional Agent

#### Scenario: Keep global enablement read-only in selected Agent views
- **WHEN** a user views Assigned or Available Skills for a selected CLI or API Agent
- **THEN** each row SHALL present enabled, paused, or unavailable status without rendering a mutable global enablement control
- **AND** a paused assigned Skill MAY provide navigation to All Skills where enablement is managed

#### Scenario: Assign global Skill to CLI Agent
- **WHEN** a user assigns or removes an effective Skill in a selected CLI Agent view
- **THEN** the page SHALL submit a granular CLI bind or unbind operation using the selected stable Agent id and canonical Skill id
- **AND** SHALL NOT change Skill enablement or any other Agent assignment
- **AND** SHALL NOT lose another completed binding change

#### Scenario: Assign global Skill to API Agent
- **WHEN** a user assigns or removes an effective Skill in a selected API Agent view
- **THEN** the page SHALL submit the non-mount API bind or unbind operation using the selected stable Agent id and canonical Skill id
- **AND** SHALL NOT change Skill enablement or any other Agent assignment
- **AND** SHALL NOT create or edit a filesystem mount path

#### Scenario: Explain configured and active CLI bindings
- **WHEN** an effective Skill is disabled while retaining a CLI Agent binding
- **THEN** the selected CLI view SHALL identify it as assigned but paused rather than currently mounted

#### Scenario: Source and version labels
- **WHEN** an effective Skill row renders
- **THEN** its source, delivery, origin, trust, effective layer, type, availability, and version SHALL remain available without opening the `SKILL.md` preview
- **AND** unavailable, compatibility-defaulted, immutable, and shadowing states SHALL be expressed with concise text and an icon or label rather than color alone

#### Scenario: Present immutable System definition
- **WHEN** an effective System Skill row renders
- **THEN** it SHALL present a concise read-only indication and SHALL NOT offer edit or delete actions
- **AND** the full immutability explanation SHALL remain available in the detail inspector

#### Scenario: Utility delegation unavailable
- **WHEN** a Utility Skill cannot yet be delegated
- **THEN** its row SHALL present a concise unavailable reason and SHALL NOT offer an action that treats it as an active Role Skill
- **AND** the full reason SHALL remain available in the detail inspector

### Requirement: Skill dialogs
The Skills settings page SHALL provide accessible application dialogs for readable effective `SKILL.md` preview, mutable Skill creation, conflict-aware editing of mutable definitions, bounded external import, confirmed deletion of mutable definitions, and restoration of intentional built-in deletion state. Immutable System package content SHALL be previewable but SHALL NOT be directly editable or deleted.

#### Scenario: Preview global SKILL.md
- **WHEN** a user opens Skill preview
- **THEN** the dialog SHALL load the effective `SKILL.md` content through the frontend service boundary
- **AND** SHALL provide a readable Markdown presentation, source content access, layer information, and a bounded resource summary

#### Scenario: Create global Skill
- **WHEN** a user submits a valid create Skill form from Settings
- **THEN** the page SHALL create a User-layer Skill with immutable canonical id and valid `SKILL.md` frontmatter through the frontend service boundary

#### Scenario: Edit global Skill
- **WHEN** a user opens a mutable User-layer Skill for editing
- **THEN** the form SHALL load its current metadata and body, prevent changing the id, and submit the loaded content hash for conflict detection
- **AND** SHALL provide Edit and Preview modes for the Markdown body

#### Scenario: Edit immutable system Skill
- **WHEN** a user opens an effective System package
- **THEN** the dialog SHALL omit direct edit and delete controls
- **AND** SHALL explain that the package is immutable and that higher-layer customization is not part of this change unless an existing create flow is used explicitly

#### Scenario: Stale edit conflict
- **WHEN** the submitted content hash no longer matches the live mutable Skill document
- **THEN** the dialog SHALL remain open, explain that the Skill changed, and offer a reload without overwriting the newer document

#### Scenario: Import global Skill
- **WHEN** a user imports an external Skill directory from Settings
- **THEN** the page SHALL create it in the User layer, display validation or limit failures, and refresh the effective overview only after success

#### Scenario: Restore built-in global Skill
- **WHEN** a user opens built-in restore
- **THEN** the dialog SHALL list only System Skill ids hidden by an intentional legacy deletion state and returned by the service
- **AND** restoration SHALL clear that state without creating a mutable System copy

#### Scenario: Guard destructive deletion
- **WHEN** a user requests deletion of a mutable User-layer Skill
- **THEN** the page SHALL use a localized application confirmation dialog before removing its managed source directory
- **AND** SHALL NOT rely on the browser-native confirmation prompt

#### Scenario: Dialog accessibility
- **WHEN** a Skill dialog opens or closes
- **THEN** it SHALL expose a translated accessible name, contain keyboard focus while open, support keyboard dismissal when safe, and restore focus to the triggering control

## ADDED Requirements

### Requirement: Effective and shadowed definition presentation
The Skills settings page SHALL present the effective definition as the primary row and SHALL provide a bounded, non-editing view of shadowed definitions and the precedence reason that selected the winner.

#### Scenario: User override shadows system package
- **WHEN** a User-layer definition shadows a System package
- **THEN** the primary row SHALL identify the User definition as effective
- **AND** a details view SHALL identify the System package as shadowed without presenting it as a second active Skill

#### Scenario: Workspace context changes winner
- **WHEN** the active workspace adds a Project-layer definition for an otherwise User-layer Skill
- **THEN** the workspace inventory SHALL identify the Project definition as effective while the global inventory remains free of that project definition

### Requirement: Desktop and Web Skill UI parity
The desktop and Web/mock Skills settings experiences SHALL consume the same frontend service contracts and SHALL render the same classification, layer, availability, and immutable-state semantics for equivalent adapter responses.

#### Scenario: Web mock system Skill
- **WHEN** the Web/mock adapter returns an immutable System Skill
- **THEN** the settings page SHALL render the same preview-only controls and explanatory state used by the desktop runtime
