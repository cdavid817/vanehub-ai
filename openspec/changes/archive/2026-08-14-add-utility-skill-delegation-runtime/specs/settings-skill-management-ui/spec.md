## MODIFIED Requirements

### Requirement: Skill card controls
Each effective Skill inventory row SHALL use a progressive information hierarchy that keeps identity, enabled state, effective layer, Skill type, description, and the context-specific primary operation immediately scannable. Delivery, origin, trust, version, usage, compatibility, resource, shadowing, and Utility delegation capability details SHALL remain available through bounded secondary text or the associated detail inspector. Mutation controls SHALL reflect whether the selected definition is mutable.

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
- **THEN** its source, delivery, origin, trust, effective layer, type, availability, version, and delegation capability SHALL remain available without opening the `SKILL.md` preview
- **AND** unavailable, compatibility-defaulted, immutable, and shadowing states SHALL be expressed with concise text and an icon or label rather than color alone

#### Scenario: Present immutable System definition
- **WHEN** an effective System Skill row renders
- **THEN** it SHALL present a concise read-only indication and SHALL NOT offer edit or delete actions
- **AND** the full immutability explanation SHALL remain available in the detail inspector

#### Scenario: Utility delegation available
- **WHEN** a Utility Skill can be delegated by the active native runtime
- **THEN** its row SHALL present a concise delegatable status and SHALL NOT offer Role Skill load or eager-injection actions
- **AND** the detail inspector SHALL identify the supported native runtime boundary

#### Scenario: Utility delegation unavailable
- **WHEN** a Utility Skill cannot be delegated by the active runtime
- **THEN** its row SHALL present a concise unavailable reason and SHALL NOT offer an action that treats it as an active Role Skill
- **AND** the full reason SHALL remain available in the detail inspector

