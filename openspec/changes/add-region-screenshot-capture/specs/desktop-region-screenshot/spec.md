## Purpose

Defines a private, bounded desktop screenshot workflow in which the user selects one visible screen region and VaneHub returns only that captured region to the initiating operation.

## ADDED Requirements

### Requirement: Desktop region capture SHALL require an explicit bounded selection
The desktop client SHALL start capture only from an explicit user action, SHALL cover each available display with a selection surface, and SHALL accept exactly one rectangular region contained by one display. The selection surface SHALL show a visible crosshair, dim unselected content, show the selected dimensions, and reject zero-sized or below-minimum regions.

#### Scenario: User selects a region on one display
- **WHEN** the user presses the screenshot action and drags a valid rectangle on an available display
- **THEN** the client SHALL capture only that rectangle and close every selection surface

#### Scenario: User starts on one display and crosses its boundary
- **WHEN** a drag leaves the display on which it started
- **THEN** the selection SHALL be clamped to that display rather than combining pixels from displays with different scale factors

### Requirement: Capture cancellation SHALL be complete and non-disruptive
The user SHALL be able to cancel with `Escape`, secondary click, or an explicit cancel control. Cancellation SHALL close every capture surface, restore the initiating VaneHub window and focus, return a typed cancelled outcome, and SHALL NOT start OCR, change the draft, or show an error notification.

#### Scenario: User cancels with Escape
- **WHEN** the selection surface is open and the user presses `Escape`
- **THEN** capture SHALL end without producing an image or changing the conversation draft

#### Scenario: Initiating session changes during selection
- **WHEN** the active composer scope changes before selection is committed
- **THEN** the pending capture SHALL be cancelled and its later result SHALL NOT enter the new composer

### Requirement: Coordinates SHALL map safely across display scale factors
The desktop runtime SHALL bind each selection surface to a stable run-scoped display token and SHALL convert its logical selection coordinates to physical capture pixels using the matching display origin, bounds, and scale factor snapshot. It SHALL reject stale, non-finite, negative-size, overflowed, out-of-bounds, unknown-display, and mismatched-run selections before capture.

#### Scenario: Capture on a scaled display
- **WHEN** a valid logical selection is committed on a display whose scale factor is not one
- **THEN** the resulting PNG dimensions SHALL match the corresponding bounded physical pixel rectangle

#### Scenario: Renderer submits an out-of-bounds rectangle
- **WHEN** a selection lies outside its bound display snapshot
- **THEN** the runtime SHALL reject it without invoking the screen capture adapter

### Requirement: Screenshot content SHALL remain private and ephemeral
Captured pixels SHALL remain local and SHALL be owned only by the active capture operation. The default adapter SHALL keep snapshots in memory and SHALL drop them after handoff or on cancellation, error, timeout, session change, and shutdown. If a platform adapter requires temporary files, it SHALL use only the local-media operation temporary root with opaque identifiers and SHALL remove them on every terminal path and bounded startup cleanup. Logs and diagnostics SHALL NOT contain pixels, image encodings, coordinates, display names, window titles, full paths, OCR text, or raw platform errors.

#### Scenario: Capture succeeds
- **WHEN** a selected region is encoded successfully
- **THEN** diagnostics MAY record only allowlisted outcome, reason code, dimension bucket, display-count bucket, and duration bucket fields

#### Scenario: Capture terminates before handoff
- **WHEN** encoding, cancellation, timeout, OCR handoff, or response delivery terminates the operation
- **THEN** every operation-owned in-memory snapshot and any adapter-required temporary file SHALL be released before the outcome becomes terminal

### Requirement: Platform permission and compositor failures SHALL be typed
The desktop runtime SHALL distinguish unsupported platform/compositor, permission denied, no display, busy capture, invalid selection, timeout, and generic capture failure outcomes without exposing raw operating-system text. Unsupported or denied capture SHALL leave existing OCR, microphone, TTS, send, and stop controls usable.

#### Scenario: macOS screen recording permission is denied
- **WHEN** the operating system refuses desktop capture permission
- **THEN** the client SHALL show a localized permission-denied action with no captured artifact

#### Scenario: Linux compositor cannot provide pixels
- **WHEN** the active compositor does not support the configured capture backend
- **THEN** the client SHALL report screenshot capture as unavailable without claiming success or disabling other local-media engines
