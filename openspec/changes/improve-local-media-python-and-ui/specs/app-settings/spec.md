## ADDED Requirements

### Requirement: Local media settings SHALL provide detected Python environment selection

The Local media page SHALL load Python discovery through the frontend service boundary and present one shared candidate inventory that OCR, STT, and TTS can select from independently. The page SHALL distinguish compatible, incompatible, currently selected, and no-longer-detected environments; preserve custom executable configuration as an explicit fallback; and SHALL NOT call Tauri APIs directly.

#### Scenario: Compatible candidates are available

- **WHEN** native discovery returns one or more compatible Python environments
- **THEN** each engine's Python control SHALL allow selection from the shared candidate inventory without typing a path
- **AND** each option SHALL expose enough explicit path and version information to distinguish environments without exposing raw probe output

#### Scenario: The user needs a custom interpreter

- **WHEN** the required environment is absent from discovery
- **THEN** the page SHALL offer an explicit custom-path selection using the established native path-picker pattern
- **AND** choosing it SHALL update only the draft and SHALL NOT execute the selected file

#### Scenario: The saved interpreter is not currently detected

- **WHEN** an engine profile references an executable absent from the latest discovery result
- **THEN** its control SHALL retain and label the saved value as not detected
- **AND** the page SHALL NOT silently choose the first available candidate

#### Scenario: Discovery is unavailable or fails

- **WHEN** discovery is native-only, times out, or returns a stable failure
- **THEN** the page SHALL keep existing draft values editable, show localized actionable guidance, and allow retry where native discovery is supported
- **AND** unrelated engine configuration SHALL remain usable

### Requirement: Local media settings SHALL present a guided and progressively disclosed setup experience

The Local media page SHALL present a compact overview of master enablement, detected Python availability, engine configuration completeness, saved-profile state, and per-engine readiness before detailed fields. Each engine card SHALL keep required setup fields immediately visible, place optional or advanced tuning fields behind accessible progressive disclosure, and automatically reveal the section containing a blocking validation or readiness issue.

#### Scenario: A user opens an unconfigured profile

- **WHEN** the Local media page loads a profile with no ready engine
- **THEN** the overview SHALL identify the next incomplete setup steps without reporting runtime success
- **AND** the primary path SHALL lead from Python selection to required model fields, save, and readiness check

#### Scenario: An engine is configured and ready

- **WHEN** an engine has complete saved configuration and a Ready readiness result
- **THEN** its collapsed summary SHALL show enabled state, readiness, safe environment/model identity, and last check metadata
- **AND** advanced tuning controls SHALL not dominate the initial page hierarchy

#### Scenario: A hidden advanced field has an error

- **WHEN** validation or readiness attributes a blocking issue to a field inside a collapsed section
- **THEN** the page SHALL reveal that section, associate the localized message with the field, and move focus or provide an accessible error target
- **AND** it SHALL preserve the user's unsaved draft

#### Scenario: The profile contains unsaved changes

- **WHEN** any engine or shared environment selection differs from the saved revision
- **THEN** persistent save/discard affordances and dirty-state feedback SHALL remain visible without obscuring engine content
- **AND** readiness checks SHALL continue to state that only saved configuration can be probed

#### Scenario: The page is viewed at narrow width

- **WHEN** the Local media page is rendered on a narrow desktop window
- **THEN** overview, environment selection, engine summaries, fields, and actions SHALL reflow without horizontal clipping or overlapping controls
- **AND** status meaning and keyboard navigation order SHALL remain equivalent to the wide layout

#### Scenario: The page runs in Web mode

- **WHEN** the Local media page renders without a native host
- **THEN** it SHALL retain the guided layout and explain that interpreter discovery and inference require the desktop client
- **AND** it SHALL not display detected environments, successful readiness, or available local devices
