## ADDED Requirements

### Requirement: Chat Mermaid rendering
The chat message renderer SHALL render Mermaid flow charts from Markdown fenced code blocks marked with the `mermaid` language.

#### Scenario: Render Mermaid code block
- **WHEN** a chat message contains a fenced `mermaid` code block with valid Mermaid flow chart content
- **THEN** the message SHALL render the diagram in place while preserving the surrounding message content

#### Scenario: Mermaid render failure fallback
- **WHEN** Mermaid parsing or rendering fails
- **THEN** the message SHALL show a localized render error and preserve the original Mermaid source text

#### Scenario: Preserve Markdown safety
- **WHEN** chat Markdown contains Mermaid or other Markdown content
- **THEN** the renderer SHALL NOT execute raw embedded HTML

### Requirement: Chat file references
The chat composer SHALL allow users to reference files under the active session root by typing `@`.

#### Scenario: Show file candidates
- **WHEN** a user types `@` in the active-session chat composer
- **THEN** the composer SHALL request bounded file candidates through the frontend service boundary and show only files inside the active session root

#### Scenario: Select file reference
- **WHEN** a user selects a file candidate
- **THEN** the composer SHALL show a visible file-reference chip and keep the reference associated with the draft until it is removed or sent

#### Scenario: Send message with references
- **WHEN** the user sends a message with one or more file references
- **THEN** the frontend service SHALL submit the text and file references together and the native runtime SHALL inject bounded file content into the Agent prompt

#### Scenario: Reject unsafe reference
- **WHEN** a referenced file is outside the session root, binary, oversized, or unavailable
- **THEN** the system SHALL reject or omit that reference with concise localized feedback without sending unrelated local files

#### Scenario: Persist reference metadata
- **WHEN** a message is sent with file references
- **THEN** the persisted user message SHALL retain safe reference metadata for history display and export
