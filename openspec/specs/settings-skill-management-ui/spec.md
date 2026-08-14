# settings-skill-management-ui Specification

## Purpose
TBD - created by archiving change split-settings-center-ui-spec. Update Purpose after archive.
## Requirements
### Requirement: Service-backed Skills settings page
The Skills settings page SHALL render the global Skill library from one service-backed global overview and SHALL expose explicit loading, retryable error, stale-conflict, empty, pending, and partial-operation states rather than interpreting absent data as an empty healthy result.

#### Scenario: Load global Skills settings data
- **WHEN** a user opens the Skills settings page
- **THEN** the page SHALL load global Skills, compatible CLI/API Agents, global Agent mount paths, both binding types, global Skill statistics, global drift status, and restore candidates through the frontend service boundary
- **AND** SHALL use `{ scope: "global", workspacePath: null }` for its overview and mutation context

#### Scenario: Exclude project selection from Settings
- **WHEN** the Skills settings page renders
- **THEN** it SHALL NOT ask the user to select or enter a workspace path
- **AND** SHALL NOT load or mutate project Skills from the Settings surface

#### Scenario: Load failure
- **WHEN** the global overview request fails or global drift is not yet available
- **THEN** the page SHALL display a loading or retryable error state
- **AND** SHALL NOT display an empty list or in-sync state as confirmed data

#### Scenario: Targeted mutation state
- **WHEN** an operation targets a specific global Skill, binding, Agent mount path, or dialog
- **THEN** its pending or failed state SHALL be presented at the affected control or interaction surface
- **AND** unrelated global Skill controls SHALL remain available unless the service operation requires broader serialization

#### Scenario: No static demo data
- **WHEN** the Skills settings page renders
- **THEN** it SHALL NOT use hard-coded demo Skill arrays as the source of displayed Skill data

### Requirement: Skills page module composition
The Skills settings page SHALL use focused child components for dynamic Agent navigation, global inventory filtering, assigned/available lists, global Skill dialogs, global drift status, and selected-CLI mount configuration while keeping service query and mutation orchestration in the page container.

#### Scenario: Render global Skill management modules
- **WHEN** the Skills settings page has loaded data
- **THEN** it SHALL render a responsive two-column Agent-navigation and inventory layout consistent with CLI Parameter Management
- **AND** each child component SHALL receive service-backed data and callbacks instead of invoking runtime APIs directly

#### Scenario: Preserve maintainable component boundaries
- **WHEN** Agent navigation, Skill lifecycle dialogs, or inventory rendering require additional UI surfaces
- **THEN** the implementation SHALL keep those concerns in focused components without causing the page container or a child component to exceed the project file-size limit

### Requirement: Skill statistics and summary
The Skills settings page SHALL display a compact global summary and SHALL display counts relevant to the selected All Skills, Agent, or Unassigned view.

#### Scenario: Display global Skill summary
- **WHEN** the page renders the All Skills view
- **THEN** it SHALL show total, enabled, CLI-bound, API-bound, and unassigned global Skill counts without requiring multiple full-size statistic cards

#### Scenario: Display selected Agent summary
- **WHEN** a user selects a compatible Agent
- **THEN** the inventory SHALL show assigned, active or paused, and available global Skill counts for that stable Agent id

#### Scenario: Display filtered result count
- **WHEN** a user changes selected Agent, assigned/available view, category, source, status, sort order, or keyword search
- **THEN** the inventory toolbar SHALL reflect the current visible global Skill count

### Requirement: Agent mount path panel
The Skills settings page SHALL provide the selected CLI-capable Agent's editable global Skill mount path inside a default-collapsed advanced disclosure and SHALL not show filesystem mount controls for API-only Agents.

#### Scenario: Keep mount paths secondary
- **WHEN** a CLI Agent view loads without a failed mount-path migration
- **THEN** its mount-path editor SHALL remain collapsed by default
- **AND** assigned and available global Skills SHALL remain reachable before expanding advanced settings

#### Scenario: Display selected CLI mount path
- **WHEN** a user expands advanced settings for a selected CLI Agent
- **THEN** the page SHALL display that Agent's current Skill mount path as a code-style editable value

#### Scenario: Exclude API mount configuration
- **WHEN** an API Agent view is active
- **THEN** the page SHALL NOT display a filesystem mount-path editor

#### Scenario: Edit Agent mount path
- **WHEN** a user changes the selected CLI Agent's mount path
- **THEN** the page SHALL submit the change through the frontend service boundary and display the migration result returned by the service

#### Scenario: Failed migration remains visible
- **WHEN** a mount-path migration reports one or more failures
- **THEN** the page SHALL expose the failure summary without requiring the user to discover it inside a collapsed disclosure

### Requirement: Skill filtering and search
The Skills settings page SHALL use the settings top-bar query as its single keyword search and SHALL allow users to filter and sort the current global inventory view without changing persisted Skill data.

#### Scenario: Keyword search
- **WHEN** a user enters a query in the settings top-bar Skill search
- **THEN** the current global inventory SHALL match Skills by id, name, description, category, triggers, or localized source label
- **AND** the page SHALL NOT present a second competing keyword input

#### Scenario: Combine inventory filters
- **WHEN** a user selects category, source, or enabled-state filters within an All, selected-Agent, or Unassigned view
- **THEN** the global Skill inventory SHALL apply all active filters together
- **AND** SHALL display the resulting count

#### Scenario: Sort Skill inventory
- **WHEN** a user selects a supported sort order
- **THEN** the current inventory SHALL reorder the filtered global Skills deterministically without changing persisted Skill data

#### Scenario: Clear Skill filters
- **WHEN** one or more inventory filters are active and the user activates clear filters
- **THEN** the page SHALL restore the default filters and sort order while preserving the selected Agent or inventory view

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

### Requirement: Skill drift banner
The Skills settings page SHALL present healthy global drift status as a compact indicator and SHALL display a prominent actionable banner when global Skill registry, source files, or CLI mount paths are inconsistent or a synchronization result needs review.

#### Scenario: Display healthy global drift status
- **WHEN** global drift detection completes with no issues and no synchronization result requires review
- **THEN** the page SHALL show a compact in-sync indicator without inserting a full-width success banner above the global inventory

#### Scenario: Display global drift issues
- **WHEN** global drift detection reports one or more issues
- **THEN** the page SHALL show a prominent banner with the issue count, a bounded issue summary, and a path to synchronize the issues

#### Scenario: Synchronize global drift
- **WHEN** a user activates one-click global drift synchronization
- **THEN** the page SHALL call the frontend service boundary and display the synchronization report, including backup, overwrite, restored, and failed results

#### Scenario: Preserve actionable synchronization result
- **WHEN** global synchronization completes with failures or backup/overwrite activity
- **THEN** the result SHALL remain reviewable until the user explicitly dismisses it or leaves the page

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

### Requirement: Global Skill Agent navigation
The Skills settings page SHALL organize the global Skill library through dynamic stable-Agent navigation with All Skills, compatible CLI Agent, compatible API Agent, and Unassigned views.

#### Scenario: Render Agent navigation
- **WHEN** the global Skill overview contains compatible Agents
- **THEN** the page SHALL render each Agent from its stable id and display name without hard-coded provider branches
- **AND** known Agents SHALL use the registered visual identity and brand icon

#### Scenario: Select CLI Agent
- **WHEN** a user selects a CLI-capable Agent
- **THEN** the page SHALL show separately labeled Assigned and Available global Skills for that Agent
- **AND** SHALL describe the assignment as a CLI mount binding

#### Scenario: Select API Agent
- **WHEN** a user selects an API Agent
- **THEN** the page SHALL show separately labeled Assigned and Available global Skills for that Agent
- **AND** SHALL describe the assignment as API prompt injection rather than a filesystem mount

#### Scenario: Show unassigned global Skills
- **WHEN** a global Skill has no CLI or API Agent binding
- **THEN** the Unassigned view SHALL include that Skill regardless of its enabled state

### Requirement: Actionable mount-root assignment failure
The Skills settings page SHALL keep a failed CLI Agent assignment attached to the affected Skill row and SHALL not present the Skill as assigned when native mount-root preflight rejects the operation.

#### Scenario: Show externally managed root failure
- **WHEN** assignment fails because the selected CLI Agent's Skill root is an externally managed directory link
- **THEN** the affected row SHALL show a concise error identifying the selected Agent and explaining that the whole-directory link must be migrated before assignment
- **AND** the Skill SHALL remain in the selected Agent's Available group after the overview refreshes

#### Scenario: Show broken root failure
- **WHEN** assignment fails because the selected CLI Agent's Skill root is a broken or unavailable directory link
- **THEN** the affected row SHALL show a concise error identifying the selected Agent and explaining that the stale link must be repaired or removed before assignment
- **AND** unrelated Skill and Agent controls SHALL remain available

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

### Requirement: Per-Skill evidence Evolution area
Skill details SHALL provide an evidence-only Evolution area showing collection state, the runtime-event-to-signal-to-seed funnel, extractor counts, attribution distribution, source-Agent distribution, category and polarity distribution, retention, quota, and dropped counts. It SHALL clearly state that target selection and Skill modification are not active in this change.

#### Scenario: Evidence funnel displayed
- **WHEN** a user opens Evolution for a Skill with retained evidence
- **THEN** the page SHALL show bounded event, signal, grouped, and seed counts with their time range and collection status

#### Scenario: Correlated CLI evidence displayed
- **WHEN** evidence includes correlated, weak, or unattributed CLI signals
- **THEN** the UI SHALL distinguish each attribution class and explain which classes cannot drive automatic targeting

#### Scenario: Collection degraded
- **WHEN** queue drops, storage failure, retention failure, or quota pressure degraded evidence collection
- **THEN** the area SHALL show a safe status and affected counts without implying the originating Agent tasks failed

#### Scenario: No evidence
- **WHEN** a Skill has no retained evidence
- **THEN** the area SHALL show an explanatory empty state, active source coverage, and retention policy rather than fabricated metrics

### Requirement: Evidence signal and seed inspection
The Evolution area SHALL provide bounded filters and read-only detail for sanitized signals and candidate seeds, including source kind, stable Agent, workspace, extractor and sanitizer version, category, polarity, severity, attribution rationale, Skill revision, lineage, and occurrence time.

#### Scenario: Inspect signal
- **WHEN** a user opens one signal
- **THEN** the detail SHALL show sanitized bounded evidence and safe source references without raw prompts, transcripts, commands, tool results, files, credentials, or full paths

#### Scenario: Inspect seed lineage
- **WHEN** a user opens one candidate seed
- **THEN** the detail SHALL show grouping reason, readiness, attribution limits, source distribution, and contributing sanitized signals

#### Scenario: Filter evidence
- **WHEN** a user combines source Agent, extractor, attribution, category, polarity, severity, readiness, and time filters
- **THEN** the page SHALL preserve the canonical Skill and workspace scope and update bounded counts and results through the service boundary

### Requirement: Evidence privacy and retention presentation
The Evolution area SHALL display the active sanitizer version, metadata-only or redacted-summary mode, twelve redaction classes, 90-day retention, quota status, and dropped or expired counts using localized explanations.

#### Scenario: Privacy details opened
- **WHEN** a user opens evidence privacy details
- **THEN** the page SHALL explain what is retained and explicitly identify prohibited raw content that is not copied into evidence storage

#### Scenario: Quota pressure displayed
- **WHEN** evidence was discarded because of quota pressure
- **THEN** the page SHALL show bounded discard counts and retention priority without exposing discarded content

### Requirement: Scoped evidence purge UI
The Evolution area SHALL provide a localized confirmation flow for purging evidence by current Skill and workspace, plus navigation to broader purge scope when available. It SHALL explain that source conversations, traces, logs, usage, Skills, and Overlays remain unchanged.

#### Scenario: Confirm Skill purge
- **WHEN** a user confirms purge for the current Skill scope
- **THEN** the page SHALL submit the operation through the frontend service boundary, prevent duplicate submission, and refresh evidence only after success

#### Scenario: Purge fails
- **WHEN** purge fails
- **THEN** the dialog SHALL remain open with a safe actionable error and existing evidence SHALL remain visible

#### Scenario: Purge accessibility
- **WHEN** the purge dialog opens or closes
- **THEN** it SHALL expose a localized accessible name, contain keyboard focus, support safe dismissal, and restore focus to its trigger

### Requirement: Evidence UI adapter parity
Desktop and Web/mock Skills settings SHALL consume the same frontend evidence contracts and render equivalent healthy, empty, correlated-CLI, degraded, quota-pressure, lineage, and purge states.

#### Scenario: Web evidence simulation
- **WHEN** the Web/mock adapter emits representative evidence summaries and seed lineage
- **THEN** the UI SHALL render the same scope, privacy, attribution, filtering, and purge semantics as equivalent desktop data
