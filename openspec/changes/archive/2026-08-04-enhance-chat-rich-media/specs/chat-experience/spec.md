## ADDED Requirements

### Requirement: Chat replies render extended Markdown safely
The chat UI SHALL render assistant Markdown with GitHub Flavored Markdown tables, task lists, autolinks, and strikethrough, mathematical notation, and syntax-highlighted fenced code while keeping raw embedded HTML disabled.

#### Scenario: Render GitHub Flavored Markdown
- **WHEN** an assistant message contains a GFM table, task list, autolink, or strikethrough
- **THEN** the message SHALL render the corresponding structured Markdown element inside the message bounds

#### Scenario: Render mathematical notation
- **WHEN** an assistant message contains valid inline or display math notation
- **THEN** the message SHALL render the notation as readable mathematical output

#### Scenario: Render highlighted source code
- **WHEN** an assistant message contains a fenced code block with a recognized language
- **THEN** the message SHALL render syntax highlighting while preserving the source text and horizontal scrolling

#### Scenario: Reject raw provider HTML
- **WHEN** assistant Markdown contains raw HTML or executable script content
- **THEN** the renderer SHALL NOT inject that content as active HTML into the application document

### Requirement: Chat images render safely and responsively
The chat UI SHALL render supported reply images through a shared constrained image renderer in both desktop and Web runtimes.

#### Scenario: Render HTTPS image
- **WHEN** assistant Markdown or a media gallery references a valid HTTPS image URL
- **THEN** the image SHALL load lazily without sending the application referrer
- **AND** it SHALL remain within the message layout

#### Scenario: Preview rendered image
- **WHEN** the user activates a successfully rendered reply image
- **THEN** the UI SHALL open an accessible enlarged preview bounded by the application viewport
- **AND** the user SHALL be able to close it with the close action or Escape

#### Scenario: Reject unsafe image source
- **WHEN** reply content references an image using plain HTTP, JavaScript, or another unsupported scheme
- **THEN** the UI SHALL NOT load the resource
- **AND** it SHALL display a localized image-unavailable fallback

#### Scenario: Image load fails
- **WHEN** a supported image URL cannot be loaded or decoded
- **THEN** the message SHALL remain readable and show a localized image-unavailable fallback

### Requirement: Rich media failures preserve source context
The chat UI SHALL preserve readable source context when enhanced rich-media rendering fails.

#### Scenario: Mermaid rendering fails
- **WHEN** a Mermaid fenced code block cannot be parsed or rendered
- **THEN** the message SHALL show a localized failure notice and the original Mermaid source in a bounded code block

#### Scenario: Unknown highlighted language
- **WHEN** a fenced code block declares an unsupported language
- **THEN** the renderer SHALL display the unchanged source as a normal code block without failing the rest of the message
