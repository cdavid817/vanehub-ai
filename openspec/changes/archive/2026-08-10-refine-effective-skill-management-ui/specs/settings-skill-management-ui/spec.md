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

### Requirement: Explicit Agent Skill selection board
The Skills settings page SHALL present the selected stable Agent's Skill relationships as an assignment-focused, responsive selection board without representing immediate binding mutations as checkboxes. Assign or Remove SHALL remain the primary row action while detail inspection and `SKILL.md` preview remain secondary actions.

#### Scenario: Compare Assigned and Available Skills on a wide layout
- **WHEN** a user selects a compatible CLI or API Agent on a wide settings layout
- **THEN** the page SHALL present separately labeled Assigned and Available panels in parallel columns
- **AND** each panel SHALL show its own deterministic count and empty state
- **AND** opening Skill details SHALL NOT prevent the user from understanding which panel contains the selected row

#### Scenario: Preserve selection order on a narrow layout
- **WHEN** the selected-Agent view is rendered below the wide-layout breakpoint
- **THEN** the Assigned and Available panels SHALL stack in a single document order with Assigned first
- **AND** every row action SHALL remain visible without horizontal page scrolling

#### Scenario: Assign an Available Skill
- **WHEN** a user activates Assign for a Skill in the Available panel
- **THEN** the page SHALL invoke the existing granular bind operation with the selected stable Agent id
- **AND** SHALL keep the Skill in its original panel until the refreshed overview confirms success
- **AND** SHALL disable duplicate actions only for the affected Skill while the operation is pending

#### Scenario: Remove an Assigned Skill
- **WHEN** a user activates Remove for a Skill in the Assigned panel
- **THEN** the page SHALL invoke the existing granular unbind operation with the selected stable Agent id
- **AND** SHALL keep global enablement and every other Agent assignment unchanged

#### Scenario: Keep a failed relationship mutation attached to its row
- **WHEN** an Agent assignment or removal fails
- **THEN** the Skill SHALL remain in its original panel
- **AND** the actionable error SHALL remain associated with that Skill row
- **AND** unrelated rows, details, previews, and filters SHALL remain operable

#### Scenario: Focus selected-Agent rows on relationship management
- **WHEN** the page renders a selected-Agent Skill row
- **THEN** the row SHALL show global enabled or paused state, Agent binding state, one explicit Assign or Remove action, and secondary Details and Preview actions
- **AND** SHALL NOT render mutable global enablement, edit, or delete controls
- **AND** the relationship action accessible name SHALL identify the selected Agent without using its display name as the service identity

#### Scenario: Distinguish CLI and API relationships
- **WHEN** the selected Agent is CLI-kind or API-kind
- **THEN** the page SHALL continue to describe CLI relationships as configured, mounted, or paused and API relationships as prompt injection or paused
- **AND** SHALL use the same selection-board interaction without hard-coded provider branches

## ADDED Requirements

### Requirement: Skill detail inspector
The Skills settings page SHALL provide a dedicated, read-only detail inspector for the selected effective Skill while keeping `SKILL.md` content preview as a distinct secondary action.

#### Scenario: Inspect an effective Skill
- **WHEN** a user activates Details for an effective Skill
- **THEN** the inspector SHALL identify the selected Skill and present type, delivery, effective layer, origin, trust, version, availability, enabled state, compatibility state, usage counts, and resource summary when available
- **AND** the corresponding row SHALL expose a selected or expanded state that does not rely on color alone

#### Scenario: Inspect precedence and shadowed definitions
- **WHEN** the selected Skill has one or more shadowed definitions
- **THEN** the inspector SHALL present the effective definition first and each shadowed definition in deterministic precedence order
- **AND** each entry SHALL identify its layer, origin, version, availability, and whether it is effective or shadowed
- **AND** the inspector SHALL explain that shadowed definitions are inspection-only and do not participate in execution

#### Scenario: Change the inspected Skill
- **WHEN** a user activates Details on a different visible row
- **THEN** the inspector SHALL replace its content with that Skill without changing filters, Agent selection, assignments, or enabled state

#### Scenario: Selected Skill leaves the visible inventory
- **WHEN** filtering, view selection, or refreshed data removes the selected Skill from the visible inventory
- **THEN** the page SHALL close the stale inspector selection or move selection to an explicitly predictable visible Skill
- **AND** SHALL NOT continue showing details for an absent row

### Requirement: Responsive and accessible Skill inspection
Skill detail inspection SHALL adapt to the available settings viewport while preserving keyboard operation, focus visibility, reading order, and equivalent content in desktop and Web/mock runtimes.

#### Scenario: Inspect on a wide settings viewport
- **WHEN** the settings content region has sufficient width for a list-detail layout
- **THEN** the selected Skill details SHALL appear in a clearly labeled supporting inspector beside the inventory
- **AND** the inventory, selected row, filters, and context-specific primary actions SHALL remain visible and operable

#### Scenario: Inspect on a narrow settings viewport
- **WHEN** the settings content region cannot fit the inventory and inspector without compressing row actions or causing horizontal scrolling
- **THEN** activating Details SHALL open the same content in a focus-managed application panel or sheet above the inventory
- **AND** dismissing it SHALL restore focus to the Details trigger

#### Scenario: Operate the inspector with a keyboard
- **WHEN** a keyboard user opens, traverses, or closes Skill details
- **THEN** every control SHALL have a visible focus indicator and translated accessible name
- **AND** modal presentation SHALL contain focus, support Escape dismissal when safe, and return focus to the originating row
- **AND** non-modal presentation SHALL follow the inventory in a logical document and heading order

#### Scenario: Respect visual accessibility preferences
- **WHEN** the page is viewed at 200 percent zoom, in a supported dark theme, or with reduced motion enabled
- **THEN** detail content and row actions SHALL remain readable and operable without horizontal page scrolling
- **AND** state SHALL NOT be conveyed by color alone
- **AND** optional transitions SHALL be removed or reduced according to the user's motion preference

#### Scenario: Preserve desktop and Web parity
- **WHEN** equivalent effective Skill data is returned by the Tauri and Web/mock adapters
- **THEN** both runtimes SHALL expose the same row hierarchy, inspector content, precedence semantics, responsive behavior, and immutable or unavailable explanations
