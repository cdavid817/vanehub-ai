# chat-experience Specification Delta

## ADDED Requirements

### Requirement: Progressive-disclosure run configuration
The chat composer SHALL keep routine message composition visually primary while exposing model, provider, runner, reasoning, thinking, permission, profile, and per-message override controls through a bounded Run Configuration surface.

#### Scenario: Render default composer
- **WHEN** a user can send a normal message with the current effective configuration
- **THEN** the composer SHALL show the multiline input, context or attachment controls, a concise Agent and model summary, and Send or Stop
- **AND** advanced configuration fields SHALL not each occupy permanent toolbar space

#### Scenario: Open run configuration
- **WHEN** the user activates the effective-configuration summary
- **THEN** a keyboard-accessible popover or sheet SHALL show grouped supported controls and each effective value's source
- **AND** unsupported fields SHALL be absent or disabled with an explanation

#### Scenario: Apply one-message override
- **WHEN** the user changes a value for the current message only
- **THEN** the composer SHALL identify the value as a temporary override
- **AND** submitting the message SHALL not silently persist it to the CLI profile

#### Scenario: Select high-risk permission
- **WHEN** the effective permission or sandbox choice carries elevated risk
- **THEN** a warning summary SHALL remain visible after the configuration surface closes
- **AND** the user SHALL see the consequence before submission

#### Scenario: Reset configuration
- **WHEN** the user resets an override
- **THEN** the field SHALL resolve to the persisted profile or provider default according to existing precedence
- **AND** the UI SHALL identify the resulting source

### Requirement: Inspectable conversation evidence
Messages, tool calls, Rich Blocks, errors, approvals, and compaction evidence SHALL expose a stable selection affordance that can drive the workbench Inspector without changing message chronology or execution semantics.

#### Scenario: Select a message
- **WHEN** the user activates a message selection affordance
- **THEN** the Inspector selection SHALL include the validated Session and message identities
- **AND** the message SHALL expose selected styling in addition to focus styling

#### Scenario: Select a tool call
- **WHEN** the user activates a tool summary or failure
- **THEN** the Inspector SHALL request bounded tool status, timing, safe input or output summary, and available evidence links from the owning service
- **AND** restricted or unavailable fields SHALL not be fabricated

#### Scenario: Select an approval or error
- **WHEN** an item requires user action or failed
- **THEN** its action controls SHALL remain available in the conversation
- **AND** selection SHALL provide additional evidence without moving the authoritative decision action into a generic Inspector

#### Scenario: Selection becomes stale
- **WHEN** the selected item is removed, compacted, or no longer belongs to the active Session
- **THEN** the Inspector SHALL show an unavailable state or return to Session Overview
- **AND** it SHALL not display another item's evidence under the stale identity

### Requirement: Windowed dynamic-height conversation history
Long conversation history SHALL use a dynamic-height windowing model that preserves stable message identity, streaming updates, prepend history, focus, selection, and reader scroll anchors.

#### Scenario: Follow new output near the bottom
- **WHEN** the reader is within the documented bottom threshold and the active message grows or a new message arrives
- **THEN** the conversation SHALL remain pinned to the latest visible content

#### Scenario: Preserve history reading position
- **WHEN** the reader is outside the bottom threshold and content below or above changes height
- **THEN** the conversation SHALL preserve the reader's anchored message and relative offset rather than jumping to the latest output

#### Scenario: Prepend older messages
- **WHEN** the user loads an earlier page
- **THEN** the first previously visible stable message SHALL remain at the same relative viewport position after insertion

#### Scenario: Render a five-thousand-message fixture
- **WHEN** the large history fixture opens
- **THEN** the DOM SHALL remain bounded to the visible window and overscan budget
- **AND** selected and focused offscreen items SHALL be recoverable through navigation

#### Scenario: Measure rich dynamic content
- **WHEN** Markdown, Tool, Mermaid, media, or Rich Block content changes size
- **THEN** the window model SHALL update measurement without reordering messages or losing the current scroll anchor

### Requirement: Conversation content hierarchy
The conversation surface SHALL use a continuous readable document hierarchy in which message content is primary and metadata, controls, tool detail, and diagnostic identifiers are progressively disclosed.

#### Scenario: Render an ordinary assistant message
- **WHEN** a completed text response has no error or required action
- **THEN** the response content SHALL be visually primary
- **AND** provider, model, ids, token detail, and low-frequency actions SHALL be available without occupying the default reading line

#### Scenario: Render a tool-heavy turn
- **WHEN** one assistant turn contains multiple tool calls
- **THEN** the UI SHALL present a bounded summary with failure, approval, and active work prioritized
- **AND** completed low-value calls MAY be collapsed

#### Scenario: Reveal message actions
- **WHEN** a pointer, keyboard, or touch user requests message actions
- **THEN** copy, quote, feedback, retry, and other permitted actions SHALL be available through an accessible toolbar or menu
- **AND** the actions SHALL not appear only on hover

#### Scenario: Render status without color
- **WHEN** a message or tool is running, complete, failed, cancelled, blocked, or waiting
- **THEN** text or an accessible description and icon or shape SHALL identify the state in addition to semantic color

### Requirement: Unified multi-seat navigation semantics
Any participant or seat selector used by the conversation or evidence surfaces SHALL provide a consistent roving-focus selection model and SHALL distinguish all-seats, selected-seat, current-speaker, departed, and unavailable states.

#### Scenario: Navigate participant choices
- **WHEN** focus is in a horizontal seat selector
- **THEN** Left and Right Arrow, Home, and End SHALL move focus among available choices
- **AND** activation behavior SHALL be documented and consistent with the selector's ARIA role

#### Scenario: Choose all seats
- **WHEN** a seat-optional evidence surface opens
- **THEN** All seats SHALL be the truthful default unless a validated route scope requests a concrete seat

#### Scenario: Open a seat-required surface
- **WHEN** Shell or another registered seat-required surface opens for a multi-seat Session
- **THEN** the UI SHALL require one active seat and SHALL not send a request with an implied or stale seat

#### Scenario: Render departed participant
- **WHEN** historical evidence belongs to a departed seat
- **THEN** the participant MAY remain inspectable as history
- **AND** it SHALL not appear as an active routing target

### Requirement: Composer and conversation responsive safety
The conversation and composer SHALL remain operable with touch, virtual keyboards, narrow widths, reduced motion, and assistive technology.

#### Scenario: Open a virtual keyboard
- **WHEN** a narrow-screen user focuses the composer
- **THEN** the active input, context chips, validation message, and Send or Stop control SHALL remain visible or scrollable above the keyboard

#### Scenario: Use a touch device
- **WHEN** message or tool actions are needed without hover
- **THEN** an explicit touch-accessible action entry SHALL be available with a sufficient hit target

#### Scenario: Use reduced motion
- **WHEN** the user prefers reduced motion
- **THEN** streaming, panel, selection, and action transitions SHALL avoid nonessential motion while preserving state changes

#### Scenario: Focus a composer error
- **WHEN** submission validation fails
- **THEN** focus or an announced error summary SHALL lead the user to the affected field without clearing the draft

### Requirement: Conversation rendering cost isolation
Expensive Markdown, Mermaid, syntax highlighting, media, Tool, and Rich Block rendering SHALL be isolated so one changing streaming item does not force unrelated completed history to rerender.

#### Scenario: Stream the active message
- **WHEN** content appends to one assistant message
- **THEN** completed messages outside the active item SHALL retain stable props and avoid update work attributable only to the append

#### Scenario: Collapse an expensive block
- **WHEN** a large tool or Rich Block is not expanded and not visible
- **THEN** its expensive renderer MAY be deferred
- **AND** the summary, status, and accessible name SHALL remain truthful

#### Scenario: Switch Inspector selection
- **WHEN** the selected conversation object changes
- **THEN** only affected selection styling and Inspector providers SHALL update
- **AND** the entire message history SHALL not be reconstructed

## MODIFIED Requirements

### Requirement: Responsive message submission feedback
The composer SHALL acknowledge a valid submission immediately, retain recoverable draft or persisted-message state, and keep unrelated reading and inspection interactions available while service reconciliation completes.

#### Scenario: Submit a valid message
- **WHEN** the user submits a non-empty valid draft and no conflicting execution owns the Session
- **THEN** the composer SHALL acknowledge submission without waiting for the provider to complete
- **AND** duplicate submission SHALL be prevented

#### Scenario: Service accepts but streaming has not started
- **WHEN** the send request returns an accepted operation or persisted user message before assistant output
- **THEN** the UI SHALL show a bounded pending or queued state tied to that operation
- **AND** the input MAY clear only after recoverability is established

#### Scenario: Submission fails before acceptance
- **WHEN** the service rejects the request before a recoverable user message or operation exists
- **THEN** the original draft SHALL remain available and the error SHALL be shown near the composer

#### Scenario: Submission fails after persistence
- **WHEN** the user message exists but generation fails
- **THEN** the user message SHALL remain in chronology and the assistant or operation state SHALL show a concise failure with recovery actions
- **AND** unrelated messages, Inspector content, and Runtime Panel access SHALL remain usable

#### Scenario: Optimistically display a submitted user message
- **WHEN** the user submits a valid message
- **THEN** the draft and selected references SHALL clear immediately
- **AND** the shared thread SHALL immediately display a temporary user message without waiting for native prompt assembly or CLI launch
- **AND** the send action SHALL remain protected from duplicate submission while the command is pending

#### Scenario: Roll back a rejected optimistic message
- **WHEN** the message service rejects the submission
- **THEN** the temporary user message SHALL be removed
- **AND** the submitted draft and file references SHALL be restored
- **AND** the existing localized error feedback SHALL remain available
