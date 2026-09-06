## ADDED Requirements

### Requirement: Panel-width responsive tab layouts
Session workspace tab layouts SHALL be sized by the width of the tab panel they render in, not by the window, because the panel sits between two collapsible side columns.

#### Scenario: Stack in a narrow panel
- **WHEN** a tab with a list beside a main surface (changes, documents, terminal records, traces) renders in a panel narrower than its side-by-side threshold
- **THEN** the list SHALL stack above the surface with a bounded height
- **AND** the surface SHALL keep the remaining height rather than being squeezed to a strip

#### Scenario: Reserve a side column only while it is used
- **WHEN** no detail drawer or comparison is open in terminal records or traces
- **THEN** the main surface SHALL span the whole panel width
- **AND** the column SHALL appear only while a drawer or comparison is open

#### Scenario: Keep the waterfall within its column
- **WHEN** the trace waterfall renders at any zoom
- **THEN** the time axis SHALL be scaled to the bar column's own width, its first and last labels SHALL be flush with the column edges on one line, and no bar SHALL extend past the last tick at zoom 1
- **AND** measuring the column SHALL NOT produce a resize-observer loop error

#### Scenario: Never lock the timeline wider than its panel
- **WHEN** the waterfall's minimum row width is derived from a measured viewport and the panel is narrower than that width
- **THEN** the timeline column SHALL shrink to the panel and scroll the rows horizontally
- **AND** the toolbar's zoom controls SHALL stay inside the panel at every window size down to the window's minimum

#### Scenario: Render rows after a remount
- **WHEN** the reader leaves the traces tab and returns to it, or resizes the window while it is open
- **THEN** the waterfall SHALL render its rows again rather than an empty area below the axis

#### Scenario: Files toolbar on its own row
- **WHEN** the files tab renders at any panel width
- **THEN** its toolbar SHALL occupy a full-width row above the tree and the preview, on one line, with the selected path beside the actions

#### Scenario: Keep the information panel out of the workspace grid rows
- **WHEN** the window is exactly at the narrow breakpoint
- **THEN** the information panel SHALL already be out of the grid flow, so the session list and the conversation keep the full height

### Requirement: Quiet tab badges
A tab badge SHALL show a number only when there is a number to show.

#### Scenario: Unknown count
- **WHEN** a tab's count is unknown because evidence is loading, indexing, partial, or unavailable
- **THEN** no badge glyph SHALL be rendered
- **AND** the tab's accessible description SHALL still state why there is no count

### Requirement: Folded terminal-record filters
The terminal-records toolbar SHALL keep the view strip and the search field visible and fold the status and fidelity chips behind a localized control.

#### Scenario: Fold and unfold
- **WHEN** no status or fidelity filter is active
- **THEN** the chips SHALL be hidden until the control is activated
- **AND** while any such filter is active the chips SHALL stay visible and the control SHALL show the active count
