## ADDED Requirements

### Requirement: Agent terminal palette theme
The embedded single-Agent CLI terminal SHALL render with one of two complete palettes, light or dark, selected by the `cliTerminalTheme` setting and applied to every such terminal that is open, hidden, or created afterwards.

#### Scenario: Provide two complete palettes
- **WHEN** either palette is active
- **THEN** it SHALL define background, default foreground, cursor, cursor accent, selection background and foreground, the sixteen ANSI colors, the terminal frame border, and the scrollbar slider colors
- **AND** the light palette SHALL use a white or near-white background with dark default text, and the terminal SHALL stay opaque

#### Scenario: Scope the palette to Agent terminals
- **WHEN** the CLI terminal theme is set to light
- **THEN** only single-Agent CLI terminal containers SHALL resolve the light variables
- **AND** ordinary Shell, remote terminals, native Agent chat, and other surfaces SHALL keep their existing colors

#### Scenario: Read the palette from the terminal container
- **WHEN** the theme object for an Agent terminal is built or rebuilt
- **THEN** the variables SHALL be resolved on the terminal's own container so the CSS frame and the xterm palette always agree
- **AND** existing callers that pass no element SHALL keep reading the document root

#### Scenario: Hot-update a running terminal
- **WHEN** the setting changes, a save is rolled back, defaults are restored, or a settings event arrives
- **THEN** each mounted Agent terminal SHALL receive a new theme object through `terminal.options.theme`
- **AND** the terminal SHALL NOT be disposed or recreated, opened or stopped again, resubscribed, cleared, replayed, or scrolled, and its id, state, and pending input SHALL be unchanged

#### Scenario: Use the current setting for new and revealed terminals
- **WHEN** a new Agent terminal is created or a hidden one becomes visible
- **THEN** it SHALL render with the palette matching the currently loaded setting

#### Scenario: Coalesce resize notifications
- **WHEN** the terminal host resizes continuously, as while a window edge is dragged
- **THEN** the terminal SHALL refit at most once per animation frame
- **AND** SHALL notify the native runtime only when the row or column count actually changed

#### Scenario: Surface input and control failures
- **WHEN** sending input, resizing, or stopping the terminal is rejected by the service
- **THEN** the tab SHALL show the existing localized workspace error instead of leaving an unhandled rejection
- **AND** the send control's availability SHALL follow the attached terminal id as rendered state, not a mutable reference

#### Scenario: Respect CLI-owned colors
- **WHEN** a CLI emits 256-color or truecolor sequences
- **THEN** the terminal SHALL render them as emitted
- **AND** the system SHALL NOT apply inversion filters, override ANSI classes, strip or rewrite color sequences, alter the user's CLI configuration, add CLI flags or environment variables, or restart the CLI to enforce the palette
