# settings-center-ui Specification Delta

## ADDED Requirements

### Requirement: Searchable settings metadata registry
Every primary settings page SHALL register its category, localized label and description, search keywords, searchable field metadata, save mode, risk level, loader, and lifecycle policy in one settings registry.

#### Scenario: Build the settings index
- **WHEN** the Settings shell initializes
- **THEN** it SHALL build search entries from static registered metadata without mounting every settings page
- **AND** secret values and runtime field contents SHALL not be indexed

#### Scenario: Register a new settings field
- **WHEN** a feature adds, removes, or renames a visible settings field
- **THEN** the same change SHALL update field id, localized label, keywords, anchor behavior, and relevant tests

#### Scenario: Detect duplicate metadata
- **WHEN** two pages or fields register the same stable search key or route anchor
- **THEN** a deterministic architecture or unit test SHALL fail with the conflicting definitions

### Requirement: Cross-page field-level settings search
Settings search SHALL find pages, sections, fields, and help keywords across the complete registered settings center and navigate directly to the matching field.

#### Scenario: Search a field label
- **WHEN** the user enters a term matching a field on another page
- **THEN** the result SHALL identify the destination page, section, field, and bounded description
- **AND** selecting it SHALL navigate to the field and place it in view

#### Scenario: Search a synonym
- **WHEN** the query matches a registered keyword but not the visible label
- **THEN** the relevant field or page SHALL be returned with the visible localized label

#### Scenario: Highlight a result
- **WHEN** a search result opens a page
- **THEN** the target field SHALL receive focus when appropriate or a temporary non-motion highlight
- **AND** the highlight SHALL not change layout or persist as saved state

#### Scenario: Return no results
- **WHEN** the query matches no registered metadata
- **THEN** the search surface SHALL show a localized no-results state and a clear-query action
- **AND** the active page SHALL remain accessible

### Requirement: Unified settings save semantics
Each settings page SHALL declare immediate, draft, or mixed save semantics, and the UI SHALL present pending, saved, error, retry, discard, and conflict states consistently.

#### Scenario: Save an immediate setting
- **WHEN** the user changes an independent toggle or select declared immediate
- **THEN** only the affected row SHALL show pending state
- **AND** a failure SHALL restore or reconcile the previous canonical value and show a row-level error

#### Scenario: Edit a draft form
- **WHEN** the user changes one or more fields on a draft page
- **THEN** a shared sticky Draft Action Bar SHALL show unsaved state with Save and Discard
- **AND** navigation, copying, and unrelated settings rows SHALL remain usable

#### Scenario: Save a draft
- **WHEN** the user submits a valid settings draft
- **THEN** the page SHALL send one bounded mutation through its service boundary
- **AND** success SHALL reconcile canonical values and clear dirty state

#### Scenario: Encounter a version conflict
- **WHEN** a draft save loses a version or freshness race
- **THEN** the page SHALL keep the user's draft, show the canonical change, and offer explicit reload or reapply paths

### Requirement: Settings unsaved-change protection
The Settings shell SHALL coordinate unsaved draft navigation and closing behavior without allowing each page to create an independent blocking dialog.

#### Scenario: Navigate away with a draft
- **WHEN** the active page has unsaved permitted values and the user chooses another page or workbench destination
- **THEN** the shell SHALL offer Save, Discard, or Stay according to the page contract

#### Scenario: Close with a secret draft
- **WHEN** the unsaved draft contains a secret value
- **THEN** the secret SHALL remain only in the owning component memory
- **AND** the shell SHALL not serialize it to route, local storage, logs, or generic draft storage

#### Scenario: Return to a preserved draft
- **WHEN** the user chooses an allowed keep-draft path and later returns
- **THEN** the page SHALL restore the draft with a clear unsaved indicator
- **AND** it SHALL revalidate against current canonical settings before save

### Requirement: Workflow-grouped settings navigation
The Settings shell SHALL group registered pages into bounded workflow categories and SHALL not expose all page entries as an unstructured horizontal strip at compact widths.

#### Scenario: Render desktop settings
- **WHEN** sufficient width is available
- **THEN** the sidebar SHALL show localized category headings and page entries with selected, attention, and disabled states
- **AND** About SHALL remain a product-information destination rather than a high-frequency configuration entry

#### Scenario: Render compact settings
- **WHEN** the sidebar and content cannot fit together at usable widths
- **THEN** navigation SHALL become a searchable sheet or selector
- **AND** the selected page SHALL remain identifiable and reachable without horizontal scrolling through every page

#### Scenario: Show page status
- **WHEN** a page has unsaved changes, configuration errors, unavailable dependency, update available, or restart required
- **THEN** its navigation entry MAY show one bounded semantic indicator
- **AND** the indicator SHALL have a localized accessible description

### Requirement: Settings danger and sensitivity hierarchy
Sensitive and destructive settings actions SHALL use shared risk presentation that separates ordinary configuration, credentials, restart-required changes, and Danger Zone operations.

#### Scenario: Render a credential field
- **WHEN** a setting contains a token, password, or private key reference
- **THEN** the UI SHALL avoid displaying the secret by default and SHALL not include it in copied diagnostics
- **AND** clear, replace, reveal, and validation actions SHALL follow the owning security contract

#### Scenario: Render a destructive action
- **WHEN** reset, uninstall, disconnect, delete, revoke, or erase-data action is available
- **THEN** it SHALL be separated from routine Save controls and require consequence-aware confirmation when destructive

#### Scenario: Require restart
- **WHEN** a change only applies after application, service, or Agent restart
- **THEN** the affected row and page SHALL state that consequence before save
- **AND** the resulting restart-required state SHALL be visible after save

### Requirement: Copyable safe settings diagnostics
Settings pages with service-backed health or configuration status SHALL provide a bounded safe diagnostic summary that can be copied without exposing secrets or unrestricted user content.

#### Scenario: Copy page diagnostics
- **WHEN** the user requests a diagnostic summary
- **THEN** the page SHALL include version, status classifications, relevant stable ids, safe paths, timestamps, and remediation codes supported by the page
- **AND** it SHALL exclude credentials, prompts, raw logs, environment values, and private external identifiers

#### Scenario: Diagnostic data is unavailable
- **WHEN** a diagnostic field has no reliable source
- **THEN** the summary SHALL mark it unavailable rather than inventing or inferring a value

## MODIFIED Requirements

### Requirement: Lazy settings module loading
The Settings center SHALL lazy-load registered page modules and SHALL retain, suspend, cache, or unmount them according to each page's explicit lifecycle policy rather than permanently mounting every visited page.

#### Scenario: Open an unvisited page
- **WHEN** a registered settings page has not been visited
- **THEN** its module SHALL remain unloaded and SHALL not start page-owned service work

#### Scenario: Visit a page
- **WHEN** the user opens a registered page for the first time
- **THEN** the content region SHALL show a localized retryable loading boundary while navigation remains usable

#### Scenario: Leave an ordinary page
- **WHEN** the page declares keepAlive never and the user navigates away
- **THEN** its component SHALL unmount and release subscriptions, timers, observers, and polling
- **AND** query-cache data MAY remain for a bounded period

#### Scenario: Leave a page with a protected draft
- **WHEN** the page declares draft-only retention and has unsaved permitted data
- **THEN** the Settings shell SHALL preserve or protect the draft according to the unsaved-change requirement

#### Scenario: Return to a page
- **WHEN** the user revisits a page after unmount or suspension
- **THEN** the page SHALL restore canonical data and any explicitly preserved draft
- **AND** it SHALL not resume stale polling without a fresh visibility check

#### Scenario: Module load fails
- **WHEN** a settings page module cannot be loaded
- **THEN** only the page content region SHALL show a localized retry action
- **AND** navigation to other pages SHALL remain available

### Requirement: Polished settings visual system
The Settings center SHALL use the shared workbench PageHeader, navigation, FormSection, SettingsRow, DraftActionBar, DataTable, AsyncBoundary, status, and dialog primitives with one consistent hierarchy in both registered themes.

#### Scenario: Render a settings page
- **WHEN** a settings page has title, help text, filters, fields, status, and actions
- **THEN** the shell SHALL avoid repeating the same page title in both top bar and page content
- **AND** the page SHALL present one clear content heading and bounded actions

#### Scenario: Render sections
- **WHEN** a page contains multiple logical settings groups
- **THEN** sections SHALL use one documented surface style and spacing rhythm
- **AND** nested cards SHALL be avoided unless a nested object has independent interaction and state

#### Scenario: Render resource management
- **WHEN** CLI, Skill, Extension, Plugin, MCP, SSH, or another resource list renders
- **THEN** it SHALL use shared collection, filter, status, row-action, empty, and error patterns rather than a page-specific CRUD shell

#### Scenario: Render both themes
- **WHEN** the page renders in futuristic or minimal
- **THEN** structure, density, status, focus, disabled state, and responsive behavior SHALL be equivalent
