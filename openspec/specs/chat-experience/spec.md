# chat-experience Specification

## Purpose
Defines the main-window chat experience, including prompt submission, selector-driven chat configuration, conversation history rendering, streamed assistant output, cancellation, persistence, and service boundary rules.
## Requirements
### Requirement: Chat input submits user messages
The system SHALL allow the user to submit a non-empty text message from the main chat input for the active session through the frontend agent service.

#### Scenario: Submit non-empty message
- **WHEN** an active session is selected and the chat input contains non-whitespace text
- **THEN** submitting the input SHALL send the message through the frontend agent service
- **AND** the submitted user message SHALL appear in the active session message list
- **AND** the input SHALL be cleared

#### Scenario: Do not submit empty message
- **WHEN** the chat input is empty or contains only whitespace
- **THEN** the send action SHALL be disabled or ignored
- **AND** no message SHALL be sent

#### Scenario: Preserve IME composition
- **WHEN** the user presses Enter while native IME composition is active
- **THEN** the system SHALL NOT submit the message
- **AND** the input composition SHALL continue normally

### Requirement: Chat configuration remains valid
The system SHALL keep chat configuration values valid when provider, agent, model, mode, or reasoning selections change.

#### Scenario: Provider change resets dependent selections
- **WHEN** the user changes the active provider
- **THEN** the system SHALL select a valid default model for that provider
- **AND** the system SHALL select a valid agent for that provider when one is available
- **AND** the reasoning depth SHALL be adjusted to a value supported by the selected model

#### Scenario: Unsupported reasoning is hidden
- **WHEN** the selected model does not support reasoning
- **THEN** the reasoning selector SHALL NOT be shown

#### Scenario: Unsupported permission mode is unavailable
- **WHEN** the active provider does not support a permission mode
- **THEN** that permission mode SHALL NOT be selectable

### Requirement: Message list displays conversation history
The system SHALL display chat messages for the active session in chronological order and attribute every Agent message through an immutable speaker identity.

#### Scenario: Empty session shows welcome screen
- **WHEN** the active session has no messages
- **THEN** the main chat area SHALL show the welcome screen
- **AND** no message item SHALL be shown

#### Scenario: Existing messages are listed
- **WHEN** the active session has existing messages
- **THEN** the message list SHALL display them in chronological order
- **AND** each message SHALL use role-appropriate rendering

#### Scenario: Multi-seat messages are attributed
- **WHEN** the active session has held more than one participant
- **THEN** each Agent message SHALL render the speaking participant's captured role avatar, role colour, and a label naming both the role and the Agent
- **AND** leaving or reordering the active roster SHALL NOT change historical attribution
- **AND** a participant recommended as a cross-family reviewer SHALL be marked as such

#### Scenario: Single-seat messages keep their existing presentation
- **WHEN** the active session has never held more than one participant
- **THEN** message presentation SHALL remain unchanged from the single-Agent experience

#### Scenario: Load earlier messages
- **WHEN** the active session has more messages than the initial page size and the user requests earlier messages
- **THEN** older messages SHALL be loaded before the current first message
- **AND** the current scroll position SHALL remain stable

#### Scenario: Preserve the visible conversation while the workspace resizes
- **WHEN** focus mode or a workspace visibility control changes the message viewport width
- **THEN** chronological message order SHALL remain unchanged
- **AND** a reader near the latest message SHALL remain pinned to the bottom
- **AND** a reader reviewing history SHALL retain the preceding bottom offset instead of jumping to another part of the thread

### Requirement: Assistant responses stream into the message list
The system SHALL display assistant responses incrementally as stream events arrive through the frontend agent service.

#### Scenario: Assistant response starts
- **WHEN** the agent service emits a `started` event for the active session
- **THEN** an assistant message with `streaming` status SHALL appear
- **AND** a waiting indicator SHALL be visible until response content arrives

#### Scenario: Token event appends content
- **WHEN** the agent service emits a `token` event for a streaming assistant message
- **THEN** the token content SHALL be appended to that assistant message
- **AND** the message SHALL NOT be duplicated

#### Scenario: Thinking event appends thinking content
- **WHEN** the agent service emits a `thinking` event for a streaming assistant message
- **THEN** the thinking content SHALL be appended to a collapsible thinking block for that message

#### Scenario: Tool event appends tool use
- **WHEN** the agent service emits a `tool_use` event whose stable tool-use id is not present on the streaming assistant message
- **THEN** one logical tool activity SHALL be added to that message

#### Scenario: Tool status event updates its logical activity
- **WHEN** the agent service emits another `tool_use` event with a stable tool-use id already present on the message
- **THEN** the existing logical activity SHALL be updated with the latest status, input, and output
- **AND** the status transition SHALL NOT create a duplicate visible activity

#### Scenario: Completion marks message complete
- **WHEN** the agent service emits a `completed` event for a streaming assistant message
- **THEN** the assistant message status SHALL become `completed`
- **AND** the waiting indicator SHALL be hidden

#### Scenario: Failure marks message failed
- **WHEN** the agent service emits a `failed` event for a streaming assistant message
- **THEN** the assistant message status SHALL become `failed`
- **AND** the error SHALL be visible to the user

### Requirement: Tool-heavy turns preserve an actionable visual hierarchy
The chat UI SHALL present tool activities in a localized compact group that keeps action-required and unsuccessful work discoverable without allowing completed history to dominate the assistant message.

#### Scenario: Summarize multiple tool activities
- **WHEN** an assistant message contains multiple tool activities
- **THEN** the UI SHALL show localized totals for active, approval-required, failed, and completed activities
- **AND** individual activities SHALL remain available through keyboard-accessible disclosure controls

#### Scenario: Prioritize actionable activities
- **WHEN** tool activities include approval-required, active, failed, and completed statuses
- **THEN** approval-required activities SHALL remain visible with their approval controls
- **AND** active activities SHALL remain visible before terminal history

#### Scenario: Collapse recoverable failure history
- **WHEN** one or more tool activities fail but the containing assistant message is not in terminal failed status
- **THEN** the UI SHALL show the failed activity count in a failure-history disclosure that is collapsed by default
- **AND** the user SHALL be able to expand the disclosure and inspect every failed activity

#### Scenario: Disclose a blocking failure
- **WHEN** one or more tool activities fail and the containing assistant message enters terminal failed status
- **THEN** the failure-history disclosure SHALL be open initially
- **AND** the most recent failure SHALL remain identifiable

#### Scenario: Aggregate repeated failures visually
- **WHEN** consecutive failed activities have the same tool, safe input preview, and error output signature
- **THEN** the failure history SHALL represent them as one row with an occurrence count
- **AND** expanding the row SHALL retain access to every occurrence payload

#### Scenario: Collapse completed history
- **WHEN** a tool activity is completed and does not require user action
- **THEN** the UI SHALL include it in a completed-history section that is collapsed by default
- **AND** the user SHALL be able to expand the section and inspect the activity input and output

#### Scenario: Explain an activity concisely
- **WHEN** a tool activity has structured input containing a command, path, query, or action
- **THEN** the UI SHALL show a bounded safe preview next to a localized tool label
- **AND** raw structured input and output SHALL remain bounded inside on-demand details

#### Scenario: Render a single completed activity
- **WHEN** an assistant message contains only one completed tool activity
- **THEN** the compact group SHALL still identify the activity and its completed status without requiring a tall standalone card

#### Scenario: Collapse the complete activity region after success
- **WHEN** an assistant message completes successfully with no pending approval and the user has not manually chosen the activity-region state
- **THEN** the complete tool-activity content SHALL collapse
- **AND** its localized status counts SHALL remain visible in the header

#### Scenario: Inspect or hide activity content manually
- **WHEN** the user activates the tool-activity header toggle and no approval is pending
- **THEN** the UI SHALL toggle the complete activity content
- **AND** SHALL retain that choice for subsequent tool snapshots on the same message

#### Scenario: Keep approval controls visible
- **WHEN** any tool activity requires approval
- **THEN** the complete activity region SHALL remain expanded regardless of a prior collapsed preference
- **AND** the approval controls SHALL remain operable

#### Scenario: Summarize collapsed active work
- **WHEN** the user collapses a region containing active work
- **THEN** the header SHALL continue to show the active count and a bounded preview of the current activity

### Requirement: User can stop active generation
The system SHALL allow the user to stop the active assistant generation for the current session.

#### Scenario: Stop while streaming
- **WHEN** an assistant response is streaming and the user activates the stop action
- **THEN** the system SHALL request cancellation through the frontend agent service
- **AND** the active assistant message SHALL be marked `cancelled`
- **AND** already generated content SHALL remain visible

#### Scenario: Stop has no effect when idle
- **WHEN** no assistant response is active and stop is requested
- **THEN** no backend cancellation SHALL be performed
- **AND** the chat input SHALL remain idle

### Requirement: Messages persist in desktop runtime
The desktop runtime SHALL persist chat messages for each session through the Rust/Tauri SQLite layer.

#### Scenario: Persist completed conversation
- **WHEN** the user sends a message and the assistant response completes in the desktop runtime
- **THEN** the user message SHALL be stored with the active session id
- **AND** the assistant message SHALL be stored with the active session id and `completed` status
- **AND** both messages SHALL be returned when the session messages are listed

#### Scenario: Persist failed message state
- **WHEN** an assistant response fails during generation in the desktop runtime
- **THEN** the assistant message SHALL be stored with `failed` status
- **AND** diagnostic metadata SHALL be retained when available

#### Scenario: Preserve Web runtime behavior
- **WHEN** the app runs in Web mode
- **THEN** the Web adapter SHALL provide the same message service contract without requiring SQLite or local Agent CLI access

### Requirement: Desktop chat uses session runtime execution
Desktop chat generation SHALL be produced through a session-scoped real Agent runtime execution path — a CLI process for `cli` agents or a direct provider API call for `api` agents — rather than a hard-coded preview or mock response.

#### Scenario: Send message to available runtime
- **WHEN** a user sends a message in the desktop runtime for a session whose selected Agent CLI is supported and installed
- **THEN** the desktop runtime SHALL run the message through the session-scoped real CLI runtime path
- **AND** stream events SHALL update the assistant message for that same session

#### Scenario: Send message to available API-based agent
- **WHEN** a user sends a message in the desktop runtime for a session whose agent has `launch_kind = api` and a valid stored credential
- **THEN** the desktop runtime SHALL run the message through the session-scoped direct provider API execution path
- **AND** stream events SHALL update the assistant message for that same session

#### Scenario: Runtime unavailable
- **WHEN** a user sends a message in the desktop runtime and the selected Agent CLI is unavailable, not installed, or unsupported
- **THEN** the user message SHALL remain persisted
- **AND** the assistant message SHALL be marked `failed`
- **AND** the failure SHALL identify the unavailable runtime without returning a fake or preview successful answer
- **AND** the chat UI SHALL show a concise user-facing error while detailed diagnostics are written to unified logs

### Requirement: Message status and session status stay synchronized
The chat service SHALL keep persisted message status and owning session lifecycle synchronized during generation.

#### Scenario: Streaming begins
- **WHEN** an assistant message starts streaming
- **THEN** the assistant message SHALL have `streaming` status
- **AND** the owning session SHALL have an active lifecycle state

#### Scenario: Streaming completes
- **WHEN** an assistant message completes
- **THEN** the assistant message SHALL have `completed` status
- **AND** the owning session SHALL no longer be marked running

#### Scenario: Streaming fails or is cancelled
- **WHEN** an assistant message fails or is cancelled
- **THEN** the assistant message SHALL retain already captured content and terminal status
- **AND** the owning session lifecycle SHALL reflect the failure or stopped state

### Requirement: Components use the chat service boundary
The system SHALL keep chat message operations behind the frontend agent service boundary.

#### Scenario: React sends message
- **WHEN** React UI code sends, lists, stops, or subscribes to chat messages
- **THEN** it SHALL call the frontend agent service interface
- **AND** it SHALL NOT call Tauri `invoke()` directly

#### Scenario: Tauri adapter handles native calls
- **WHEN** the desktop frontend performs a chat message operation
- **THEN** Tauri `invoke()` and event listening SHALL remain inside the Tauri-specific frontend adapter

### Requirement: Localized chat interface text
The chat UI SHALL render user-visible chat labels, selectors, placeholders, role labels, and status text through synchronized zh-CN and en translation resources.

#### Scenario: Chat composer and message labels localized
- **WHEN** the chat surface renders in Simplified Chinese or English
- **THEN** composer placeholders, send/enhance/stop actions, loading labels, message status labels, role labels, thinking labels, scroll controls, and welcome messages SHALL use the active locale

#### Scenario: Chat configuration selectors localized
- **WHEN** chat provider, agent, model, mode, permission, reasoning, or configuration controls render user-visible labels or descriptions
- **THEN** frontend-owned labels, button titles, and descriptions SHALL use the active locale
- **AND** provider names, model names, and Agent display names MAY remain literal identifiers

#### Scenario: Chat timestamps localized
- **WHEN** chat messages display timestamps
- **THEN** timestamp formatting SHALL use the active application language rather than a fixed locale

### Requirement: Desktop CLI chat streams provider runtime output
The desktop runtime SHALL stream assistant output from provider-specific Agent CLI execution for CLI sessions instead of parsing only after command completion.

#### Scenario: Stream provider CLI stdout
- **WHEN** a user sends a message to an active non-archived session whose interaction mode is `cli`
- **THEN** the desktop runtime SHALL start a provider-specific CLI invocation for the session's stable agent id
- **AND** stdout events SHALL be normalized into `started`, `token`, `thinking`, `tool_use`, `completed`, `failed`, or `cancelled` chat events for that session
- **AND** token events SHALL be emitted as output becomes available rather than only after process exit

#### Scenario: Use provider-specific command path
- **WHEN** the active session references `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, or `antigravity-cli`
- **THEN** the desktop runtime SHALL build the CLI invocation using that provider's supported headless command contract
- **AND** it SHALL NOT rely on a single generic `executable prompt` command shape for all providers

#### Scenario: Prefer stdin for prompt delivery
- **WHEN** a provider CLI supports reading the prompt from stdin
- **THEN** the desktop runtime SHALL send the prompt through stdin instead of placing the full prompt in process arguments
- **AND** command audit logs SHALL redact prompt content

### Requirement: Desktop CLI chat persists streamed content
The desktop runtime SHALL persist streamed assistant content and terminal status for CLI chat generations.

#### Scenario: Persist streamed assistant content
- **WHEN** a provider CLI emits token output for an assistant message
- **THEN** the desktop runtime SHALL append the emitted content to the persisted assistant message
- **AND** the visible chat event stream SHALL match the persisted message content after refresh

#### Scenario: Persist terminal runtime outcome
- **WHEN** the provider CLI exits successfully after streamed output
- **THEN** the assistant message SHALL be marked `completed`
- **AND** token usage SHALL be persisted when provider metadata is available

#### Scenario: Persist failed runtime outcome
- **WHEN** the provider CLI fails to start, exits unsuccessfully, or emits a structured error event
- **THEN** the user message SHALL remain persisted
- **AND** the assistant message SHALL be marked `failed` with a concise user-facing error
- **AND** detailed diagnostics SHALL be written through unified logging

### Requirement: Chat configuration reaches provider invocation
The desktop chat runtime SHALL apply supported per-message model, reasoning, permission, and related CLI configuration through the selected stable agent id's provider argument builder.

#### Scenario: Apply supported message configuration
- **WHEN** a user sends a message with a configuration value supported by the active provider
- **THEN** the native provider invocation for that message SHALL contain the provider-specific mapped value
- **AND** the mapping SHALL use the stable agent id rather than display-name matching

#### Scenario: Unsupported message configuration
- **WHEN** a message contains a configuration value that has no safe mapping for the active provider
- **THEN** the runtime SHALL omit or reject that value with a concise user-displayable reason
- **AND** it SHALL NOT guess an argument or silently replace a reserved runtime token

### Requirement: Chat invocation parameter precedence
The desktop chat runtime SHALL resolve a supported per-message value before the corresponding persisted CLI profile value and SHALL resolve a persisted value before the provider default.

#### Scenario: Per-message override is temporary
- **WHEN** a message overrides a value saved in the active CLI profile
- **THEN** only the process spawned for that message SHALL use the message value
- **AND** later messages without the override SHALL continue using the persisted profile

#### Scenario: Persisted default is applied
- **WHEN** a message does not override a saved logical parameter
- **THEN** the process spawned for that message SHALL use the saved profile value

### Requirement: Chat profile changes use next-process semantics
Saving or resetting a CLI profile SHALL NOT alter a provider process that is already running and SHALL be read again before the next fresh or resume process spawn.

#### Scenario: Change profile during stream
- **WHEN** a user changes the active provider's profile while a response is streaming
- **THEN** the current response SHALL continue using its original invocation arguments
- **AND** the next message process SHALL use the newly effective profile

### Requirement: Main-window chat operation failure reporting
The main chat surface SHALL show localized feedback and report durable diagnostics through the frontend service boundary when a chat send, stop, or configuration-persistence operation fails.

#### Scenario: Chat send or stop request fails
- **WHEN** the main-window send or stop request reaches a terminal service failure
- **THEN** the chat surface SHALL show a localized user-displayable error without clearing unrelated loaded messages
- **AND** it SHALL report a `critical-operation-failure` event through the settings service boundary

#### Scenario: Configuration persistence fails
- **WHEN** saving a changed session chat configuration fails
- **THEN** the chat surface SHALL show a localized user-displayable error
- **AND** it SHALL report a `critical-operation-failure` event through the settings service boundary

#### Scenario: Web runtime reports a chat failure
- **WHEN** the app runs through the Web/mock adapter and reports a chat operation failure
- **THEN** it SHALL preserve the same visible feedback and service call
- **AND** it SHALL NOT write a local log file

### Requirement: Chat messages support durable Rich Blocks
The system SHALL support structured Rich Blocks as durable attachments on chat messages while preserving existing text, thinking, and tool-use message fields.

#### Scenario: List message with persisted Rich Blocks
- **WHEN** a session message contains persisted Rich Blocks
- **THEN** listing messages for that session SHALL return the message with its `richBlocks` in stable order
- **AND** the existing `content`, `thinkingContent`, and `toolUse` fields SHALL remain available

#### Scenario: Existing message without Rich Blocks
- **WHEN** a message was created before Rich Block support or has no Rich Blocks
- **THEN** the chat service SHALL return it without requiring a Rich Block payload
- **AND** the message SHALL render using the existing text, thinking, and tool-use behavior

### Requirement: Rich Block stream events append structured blocks
The chat stream SHALL support a `rich_block` event that appends a structured block to the target assistant message through the frontend agent service boundary.

#### Scenario: Receive Rich Block event
- **WHEN** the agent service emits a `rich_block` event for the active session and assistant message
- **THEN** the message list SHALL show the new Rich Block on that assistant message without duplicating the assistant message
- **AND** the Rich Block SHALL remain visible after message data is reloaded

#### Scenario: Receive duplicate Rich Block id
- **WHEN** a `rich_block` event carries a block id that already exists on the target message
- **THEN** the client SHALL update or replace the existing block with that id rather than rendering duplicate blocks

### Requirement: Desktop runtime persists Rich Blocks
The desktop runtime SHALL persist Rich Blocks for chat messages through the Rust/Tauri SQLite layer.

#### Scenario: Persist streamed Rich Block
- **WHEN** the desktop runtime normalizes provider output into a `rich_block` event
- **THEN** the runtime SHALL append the block to the assistant message's persisted Rich Blocks
- **AND** the same block SHALL be returned by `list_messages` after refresh

#### Scenario: Preserve failed or cancelled Rich Blocks
- **WHEN** generation fails or is cancelled after one or more Rich Blocks have been received
- **THEN** already persisted Rich Blocks SHALL remain attached to the assistant message
- **AND** the assistant message SHALL still show its terminal failed or cancelled status

### Requirement: Web runtime preserves Rich Block contract parity
The Web/mock runtime SHALL implement the same Rich Block message and event contract as the desktop runtime.

#### Scenario: Web mock streams Rich Blocks
- **WHEN** the app runs through the Web/mock adapter and emits a mock Rich Block event
- **THEN** the message list SHALL render the block using the same React components as desktop mode
- **AND** re-listing Web/mock messages SHALL retain the mock Rich Block data

### Requirement: Rich Block renderers support first-version block kinds
The chat UI SHALL render supported first-version Rich Block kinds with localized labels and visual styling consistent with both configured visual styles.

#### Scenario: Render supported block kinds
- **WHEN** a message contains `card`, `diff`, `checklist`, `media_gallery`, `file`, `audio`, `html_widget`, or `interactive` Rich Blocks
- **THEN** the chat UI SHALL render each block with a stable layout that does not overlap message text, status labels, or adjacent blocks
- **AND** frontend-owned labels and fallback text SHALL use the active locale

#### Scenario: Render unknown or invalid block
- **WHEN** a message contains an unsupported, unknown, or invalid Rich Block
- **THEN** the chat UI SHALL render a localized fallback that identifies the unsupported block kind when available
- **AND** the rest of the message SHALL remain readable

### Requirement: HTML widget Rich Blocks are sandboxed
The chat UI SHALL render `html_widget` Rich Blocks inside a constrained sandbox rather than injecting provider HTML into the React document.

#### Scenario: Render HTML widget safely
- **WHEN** a message contains an `html_widget` block
- **THEN** the UI SHALL render the block in an iframe or equivalent sandboxed boundary
- **AND** the block height SHALL be bounded to prevent it from breaking the chat layout

### Requirement: Interactive Rich Blocks are read-only in first version
The first Rich Block implementation SHALL treat `interactive` blocks as read-only previews until an explicit interaction contract is added.

#### Scenario: Render interactive preview
- **WHEN** a message contains an `interactive` Rich Block
- **THEN** the UI SHALL show the title, description, and options without sending chat messages or invoking native commands from option clicks
- **AND** the UI SHALL show localized text indicating that interactive actions are not enabled yet

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
The chat composer SHALL allow users to reference files under the active session root by typing `@`. Candidate discovery for this completion SHALL be independent of the session document listing that backs the Documents tab, so that document-viewer type bounds do not constrain which files can be referenced.

#### Scenario: Show file candidates
- **WHEN** a user types `@` in the active-session chat composer
- **THEN** the composer SHALL request bounded file candidates through the frontend service boundary and show only files inside the active session root
- **AND** the candidate set SHALL NOT be derived from the Documents tab document listing

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

### Requirement: CLI chat applies Prompt Hooks before provider invocation
The desktop CLI chat runtime SHALL assemble enabled Prompt Hooks into the effective prompt before launching a provider CLI process.

#### Scenario: Apply hooks for bound CLI
- **WHEN** a user sends a message to an active non-archived CLI session whose stable agent id has enabled Prompt Hooks bound to it
- **THEN** the desktop runtime SHALL assemble those hooks with the user content before provider invocation
- **AND** the provider-specific invocation builder SHALL receive the assembled effective prompt

#### Scenario: Skip unbound hooks
- **WHEN** a Prompt Hook is not bound to the active session's stable agent id
- **THEN** the desktop runtime SHALL skip that hook for the invocation

#### Scenario: Preserve original user message
- **WHEN** Prompt Hooks are applied to a chat invocation
- **THEN** the persisted and displayed user message SHALL remain the original trimmed user input
- **AND** the assembled effective prompt SHALL NOT replace the user-visible message content

#### Scenario: Hook assembly failure
- **WHEN** Prompt Hook assembly fails during chat send
- **THEN** the user message SHALL remain persisted
- **AND** the assistant message SHALL be marked `failed` with a concise user-facing error
- **AND** detailed redacted diagnostics SHALL be written through unified logging

### Requirement: Web runtime preserves Prompt Hook chat parity
The Web/mock runtime SHALL preserve the Prompt Hook chat contract without claiming native CLI execution.

#### Scenario: Web mock applies deterministic hook preview
- **WHEN** the Web/mock adapter sends a mock chat message with enabled Prompt Hooks
- **THEN** it SHALL use deterministic Prompt Hook assembly behavior for mock response metadata or trace behavior
- **AND** it SHALL preserve the same original user-message display semantics as desktop mode

### Requirement: Session agent identity

The system SHALL show the selected agent/CLI icon on the session page.

#### Scenario: Codex session shows Codex identity

- **WHEN** a session is created for Codex CLI
- **THEN** the session page SHALL display the Codex CLI icon or registered visual identity alongside the session metadata.

#### Scenario: Session surfaces use stable CLI identity

- **WHEN** a session references Claude Code, Codex CLI, Gemini CLI, or OpenCode
- **THEN** session list and detail surfaces SHALL render the corresponding branded CLI icon from the stable agent id.

### Requirement: CLI chat records Prompt Hook version outcomes
The desktop CLI chat runtime SHALL report one safe terminal outcome and elapsed time for every published Prompt Hook version fired during an Agent invocation.

#### Scenario: Record successful generation
- **WHEN** a CLI Agent generation succeeds after published Prompt Hook versions fired
- **THEN** the runtime SHALL report `succeeded`, elapsed milliseconds, stable invocation id, stable agent id, and the fired Hook id/version references through the evaluation gateway

#### Scenario: Record failed generation
- **WHEN** a CLI Agent generation fails after published Prompt Hook versions fired
- **THEN** the runtime SHALL report `failed` with the same safe correlation fields
- **AND** it SHALL not include the raw failure or Prompt content in the evaluation observation

#### Scenario: Record cancelled generation separately
- **WHEN** a user cancels a generation after published Prompt Hook versions fired
- **THEN** the runtime SHALL report `cancelled`
- **AND** the cancelled observation SHALL not reduce the version's calculated success rate

#### Scenario: No fired Hook versions
- **WHEN** no Prompt Hook version fires for an Agent invocation
- **THEN** the runtime SHALL not create Prompt Hook evaluation observations for that invocation

### Requirement: Desktop API chat streams provider runtime output
The desktop runtime SHALL stream assistant output from a direct provider API call for sessions whose agent uses the `api` launch kind, normalizing the response into the same chat event vocabulary used for CLI sessions.

#### Scenario: Stream provider API response
- **WHEN** a user sends a message to an active non-archived session whose agent has `launch_kind = api`
- **THEN** the desktop runtime SHALL call the configured provider's API with the conversation history and the agent's configured model
- **AND** the response SHALL be normalized into `started`, `token`, `thinking`, `completed`, or `failed` chat events for that session
- **AND** token events SHALL be emitted as content becomes available rather than only after the full response completes

#### Scenario: No per-message CLI-style configuration
- **WHEN** chat generation runs for an `api` launch-kind agent
- **THEN** the desktop runtime SHALL use the agent's registered model
- **AND** it SHALL NOT apply CLI Parameter Management profile values or Prompt Hook assembly, which remain scoped to `cli` agents

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

### Requirement: Composer seat mention completion
In a multi-seat session the composer SHALL offer completion for seat mentions and SHALL make the line-leading routing rule discoverable.

#### Scenario: Completion lists seats
- **WHEN** the user types a mention trigger in the composer
- **THEN** the composer SHALL list the session's seats with their role name, Agent, and model family
- **AND** selecting one SHALL insert its mention

#### Scenario: Routing rule is discoverable
- **WHEN** the completion list is shown
- **THEN** the composer SHALL indicate that only a mention at the start of a line routes the message

#### Scenario: Single-seat session offers no completion
- **WHEN** the active session holds exactly one seat
- **THEN** the composer SHALL NOT offer seat mention completion

### Requirement: Non-duplicative conversation header
The chat header SHALL present session identity, runtime state, and conversation actions without duplicating member identity that belongs in the information panel.

#### Scenario: Keep multi-Agent identity out of the conversation header
- **WHEN** the active session holds more than one participant
- **THEN** the chat header SHALL show the session title and bounded multi-Agent summary
- **AND** it SHALL NOT render participant role chips, Agent names, or CLI ids
- **AND** member details SHALL remain available in the information panel

#### Scenario: Keep single-Agent identity out of the conversation header
- **WHEN** the active session holds one participant and has no departed participants
- **THEN** the chat header SHALL show the session title and interaction mode
- **AND** it SHALL NOT render a participant or CLI identity row

#### Scenario: Present a desktop messaging hierarchy
- **WHEN** the chat tab is displayed on a desktop viewport
- **THEN** the session identity and runtime state SHALL remain in a stable top header
- **AND** the message canvas SHALL use the available conversation width with bounded adaptive edge gutters
- **AND** individual message bubbles SHALL retain a readable maximum width
- **AND** the composer SHALL attach to the conversation bottom with a quiet top divider instead of floating as a separate card
- **AND** member details SHALL remain available from the information panel without reducing the message area unnecessarily

#### Scenario: Use released panel width without oversized blank margins
- **WHEN** focus mode or an overflow action collapses an adjacent workspace panel
- **THEN** the message canvas SHALL expand with the conversation surface
- **AND** it SHALL NOT retain a fixed centered width that creates oversized blank margins on both sides
- **AND** assistant and user bubbles SHALL remain aligned to their respective conversation edges

#### Scenario: Preserve header alignment in focus mode
- **WHEN** focus mode collapses the surrounding panels
- **THEN** the session title, runtime state, and overflow actions SHALL retain their relative order and alignment
- **AND** the workspace SHALL NOT animate layout-affecting grid tracks that can transiently reorder header content

### Requirement: Unified composer completion
The composer SHALL provide distinguishable completion results for participant routing and file references without allowing one kind to be mistaken for the other.

#### Scenario: Present one integrated composer surface
- **WHEN** the chat composer is available
- **THEN** it SHALL provide a spacious borderless editor within one quiet bordered container
- **AND** runtime selectors and message actions SHALL remain in a bottom toolbar inside that container
- **AND** selected references, completion, keyboard submission, disabled state, and visible keyboard focus SHALL remain available

#### Scenario: Complete a participant mention
- **WHEN** the user types `@` at the start of a line in a multi-Agent session
- **THEN** completion SHALL list active participants with role, Agent, and model-family identity
- **AND** selecting a participant SHALL insert its unique routing handle

#### Scenario: Complete a file reference
- **WHEN** the user requests file completion
- **THEN** completion SHALL identify results as files and SHALL preserve file attachment behavior
- **AND** a file result SHALL NOT be interpreted as a participant route

#### Scenario: Exclude departed participants
- **WHEN** a participant has left the session
- **THEN** participant completion SHALL NOT offer that participant as a routing target

### Requirement: Responsive message submission feedback
The composer SHALL acknowledge a valid submission immediately while preserving recoverability when the service rejects it.

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

### Requirement: Chat controls respect recovery safety
The chat experience SHALL derive send and stop availability from the service-backed lifecycle, recovery status, and active execution ownership rather than lifecycle alone.

#### Scenario: Disable sending during reconciliation
- **WHEN** the active session is `reconciling`, `action_required`, or `quarantined`
- **THEN** the composer SHALL prevent message submission and show the corresponding localized recovery state

#### Scenario: Allow a clean failed session to continue
- **WHEN** the active session lifecycle is `failed`, recovery status is `clean`, no execution run is active, and the session is not archived
- **THEN** the composer SHALL allow a new message to be submitted through the frontend service

#### Scenario: Stop targets only an active execution
- **WHEN** recovery has cleared an orphan active claim and no generation handle exists
- **THEN** the chat UI SHALL NOT offer stop as though an old native process were still running

### Requirement: Recovery review preserves user-visible evidence
The chat experience SHALL present interrupted content and safe recovery explanations without removing transcript evidence or exposing sensitive diagnostics.

#### Scenario: Show action-required recovery
- **WHEN** the active session requires recovery action
- **THEN** the UI SHALL preserve the existing transcript, display a localized safe explanation, and offer the allowed acknowledgement action through the service boundary

#### Scenario: Acknowledge recovery
- **WHEN** the user confirms acknowledgement for the currently displayed recovery revision
- **THEN** the UI SHALL submit it through the shared service, refresh the authoritative session state, and SHALL NOT represent the action as retrying or undoing tool effects

#### Scenario: Present quarantined session
- **WHEN** the active session is quarantined
- **THEN** the UI SHALL keep supported inspection and export surfaces available while disabling dependent mutations

### Requirement: Chat history displays context compaction evidence
The chat experience SHALL render each successful automatic context compaction as an accessible rich card in chronological message history, showing content-free before/after measurements, savings, trigger source, compaction path, and policy version.

#### Scenario: Successful compaction occurs during streaming
- **WHEN** the active OnePiece generation emits a successful compaction evidence card
- **THEN** the card SHALL appear with the corresponding assistant message without interrupting token streaming

#### Scenario: Token count is unavailable
- **WHEN** the evidence reports token measurements as unavailable
- **THEN** the card SHALL clearly distinguish unavailable token evidence from a zero token count

#### Scenario: Reload conversation history
- **WHEN** a conversation containing compaction evidence is restored
- **THEN** the evidence card SHALL retain its field values and chronological position

### Requirement: Explicit assistant-message feedback
The chat experience SHALL let users submit one current feedback state of `helpful`, `unhelpful`, or `corrected` for a completed assistant message. Corrected feedback MAY include one bounded optional correction note. Feedback SHALL be sent through the frontend service boundary and SHALL not edit the assistant message.

#### Scenario: Mark response helpful
- **WHEN** a user marks a completed assistant message helpful
- **THEN** the page SHALL persist structured helpful feedback and show the saved state on that message

#### Scenario: Mark response unhelpful
- **WHEN** a user marks a completed assistant message unhelpful
- **THEN** the page SHALL persist structured unhelpful feedback without requiring a free-form note

#### Scenario: Submit correction
- **WHEN** a user selects corrected and submits a note within the configured limit
- **THEN** the service SHALL sanitize and persist the feedback projection and show the corrected state

#### Scenario: Feedback on incomplete message
- **WHEN** a message is streaming, failed before producing a completed response, or belongs to an inaccessible session
- **THEN** feedback submission SHALL be unavailable or rejected without creating evidence

#### Scenario: Replace prior feedback
- **WHEN** a user changes feedback on the same completed message
- **THEN** the service SHALL retain one current feedback state while preserving an evidence audit transition without producing duplicate active feedback signals

### Requirement: Feedback privacy and failure behavior
Feedback correction notes SHALL display their character limit and privacy warning, SHALL be sanitized before evidence persistence, and SHALL not be written to frontend files or feature-specific logs. A failed save SHALL remain visible and retryable without changing the assistant message.

#### Scenario: Sensitive correction note
- **WHEN** a correction note contains a recognized sensitive value
- **THEN** persisted feedback evidence SHALL contain only the sanitized bounded form

#### Scenario: Feedback save fails
- **WHEN** the adapter reports that feedback was not persisted
- **THEN** the UI SHALL show a localized row-scoped error, retain unsaved input for retry, and SHALL not display the feedback as saved

#### Scenario: Web feedback parity
- **WHEN** feedback is submitted in Web/mock mode
- **THEN** the adapter SHALL simulate the same states, sanitization-result shape, replacement behavior, and failure contract without native persistence

### Requirement: Chat file reference candidate search
The system SHALL resolve chat file-reference candidates by searching the active session root for a caller-supplied query, returning a ranked and bounded result set rather than an unranked prefix of the workspace.

#### Scenario: Match source files
- **WHEN** the composer requests candidates with a query that matches a source or configuration file under the session root
- **THEN** the result SHALL include that file
- **AND** eligibility SHALL NOT be restricted to Markdown and plain-text documents

#### Scenario: Rank by match quality
- **WHEN** candidates are returned for a query
- **THEN** they SHALL be ordered so that an exact filename match ranks above a filename prefix match, a filename prefix match ranks above a filename substring match, and a filename substring match ranks above a match found only across path segments

#### Scenario: Exclude vendored and generated trees
- **WHEN** the session root contains dependency installs, build outputs, compiler caches, or virtual environments
- **THEN** candidate search SHALL NOT descend into those directories
- **AND** the result budget SHALL be spent on first-party files

#### Scenario: Documents tab listing is unaffected
- **WHEN** the Documents tab lists documents for the same session root
- **THEN** its listing SHALL retain its existing Markdown and text bounds and its existing traversal behavior
- **AND** the exclusions applied to candidate search SHALL NOT change what the Documents tab shows

#### Scenario: Bound the search
- **WHEN** candidate search runs
- **THEN** it SHALL enforce a traversal depth limit and return no more than the requested number of results
- **AND** it SHALL resolve only paths contained within the active session root

#### Scenario: No match
- **WHEN** no file under the session root matches the query
- **THEN** the system SHALL return an empty candidate set without error
- **AND** the composer SHALL NOT present a file completion list

#### Scenario: Session root unavailable
- **WHEN** candidate search is requested for a session that has no resolvable root
- **THEN** the system SHALL return an empty candidate set with concise localized feedback rather than raw native diagnostics

#### Scenario: Web runtime parity
- **WHEN** candidate search is requested in Web/mock mode
- **THEN** the Web adapter SHALL serve the same service contract so the composer remains usable in browser mode

### Requirement: Chat file reference line ranges
A chat file reference SHALL be able to carry an optional line range naming the region of the file the user means. A reference without a range SHALL mean the whole file.

#### Scenario: Express a line range in the composer
- **WHEN** a user completes a mention as `@<path>:<start>-<end>`
- **THEN** the resulting reference SHALL carry that start and end line
- **AND** the range SHALL be interpreted as 1-based and inclusive of both bounds

#### Scenario: Express a single line
- **WHEN** a user completes a mention as `@<path>:<line>`
- **THEN** the resulting reference SHALL carry that line as both its start and its end

#### Scenario: Reference without a range
- **WHEN** a user completes a mention with no range suffix
- **THEN** the reference SHALL carry no range
- **AND** it SHALL behave exactly as file references did before ranges existed

#### Scenario: Candidate search ignores the range suffix
- **WHEN** the composer requests candidates for a mention token that already carries a range suffix
- **THEN** the query SHALL be the path portion only
- **AND** completion SHALL stay available while the range is being typed

#### Scenario: Reject a malformed range
- **WHEN** a reference carries only one of the two bounds, a bound below 1, or an end line before its start line
- **THEN** the system SHALL reject the reference with concise localized feedback and SHALL NOT send the message

#### Scenario: Range extends past the end of the file
- **WHEN** a reference names an end line beyond the file's last line
- **THEN** the system SHALL clamp the range to the last line rather than rejecting the reference
- **AND** a start line beyond the last line SHALL yield an empty region rather than an error

#### Scenario: Reference two regions of one file
- **WHEN** a user references the same path twice with different line ranges
- **THEN** both references SHALL be accepted as distinct
- **AND** the overall maximum number of file references per message SHALL still apply

#### Scenario: Reject an exact duplicate
- **WHEN** a user references the same path twice with the same range, or twice with no range
- **THEN** the system SHALL reject the duplicate with concise localized feedback

#### Scenario: Chips show the range
- **WHEN** a ranged reference is displayed in the composer or in message history
- **THEN** its chip SHALL show the referenced line range alongside the file name
- **AND** a reference without a range SHALL be displayed without range decoration

#### Scenario: Remove one of several references to a file
- **WHEN** a user removes one chip while another reference to the same path is attached
- **THEN** only the selected reference SHALL be removed and the other SHALL remain attached

#### Scenario: Persist and restore the range
- **WHEN** a message with ranged references is persisted and later restored into history
- **THEN** each restored reference SHALL retain its line range
- **AND** references persisted before ranges existed SHALL restore as whole-file references

### Requirement: Line-bounded file reference injection
Prompt assembly SHALL inline only the lines a reference names, so that referencing a region of a large file spends context proportional to the region rather than to the file.

#### Scenario: Inject only the requested lines
- **WHEN** a message is sent with a reference carrying a line range
- **THEN** the Agent prompt SHALL contain only the lines within that range
- **AND** the injected block SHALL identify the file and the range it covers

#### Scenario: Injected lines carry their positions
- **WHEN** a ranged reference is injected
- **THEN** each injected line SHALL be labelled with its 1-based position in the source file, so positions cited by the Agent match the user's editor

#### Scenario: Whole-file injection is unchanged
- **WHEN** a message is sent with a reference carrying no range
- **THEN** the Agent prompt SHALL contain the whole file exactly as it did before ranges existed

#### Scenario: Safeguards are not widened
- **WHEN** a ranged reference is resolved for injection
- **THEN** the existing path containment, oversize, and binary-file safeguards SHALL apply unchanged
- **AND** a range SHALL only narrow what is injected, never permit reading a file that would otherwise be refused

### Requirement: File reference preview and range picker
Selecting a file candidate from `@` completion SHALL open a preview of that file in which the user can pick the lines to reference, so that choosing a file and choosing a region are one action rather than a trip to another tab.

#### Scenario: Selecting a candidate opens the preview
- **WHEN** a user selects a file candidate from `@` completion
- **THEN** the composer SHALL request that file's content through the frontend service boundary and present it in a dialog
- **AND** no reference SHALL be attached until the user confirms one

#### Scenario: A typed range bypasses the preview
- **WHEN** a user completes a mention that already carries a range suffix
- **THEN** the reference SHALL be attached directly without opening the preview

#### Scenario: Content is shown with positions that match the prompt
- **WHEN** the preview displays file content
- **THEN** each line SHALL be labelled with its 1-based position in the file
- **AND** those positions SHALL be the same ones used when the reference is injected into the Agent prompt

#### Scenario: Pick a range by clicking two lines
- **WHEN** the user clicks one line and then another
- **THEN** the pending selection SHALL cover both lines and every line between them
- **AND** the result SHALL be the same regardless of which of the two was clicked first

#### Scenario: Pick a single line
- **WHEN** the user clicks one line and confirms without clicking a second
- **THEN** the attached reference SHALL carry that line as both its start and its end

#### Scenario: The pending selection is visible
- **WHEN** a selection is pending
- **THEN** the selected lines SHALL be visually distinguished from unselected ones
- **AND** the range about to be referenced SHALL be stated in the dialog

#### Scenario: Confirm a selection
- **WHEN** the user confirms the selected range
- **THEN** a reference carrying that range SHALL be attached and the dialog SHALL close

#### Scenario: Confirm the whole file
- **WHEN** the user chooses to reference the whole file
- **THEN** a reference carrying no range SHALL be attached and the dialog SHALL close
- **AND** this SHALL remain available whether or not a selection is pending

#### Scenario: Dismiss without referencing
- **WHEN** the user dismisses the dialog
- **THEN** no reference SHALL be attached
- **AND** the composer draft SHALL be left as it was before the candidate was selected

#### Scenario: File cannot be displayed
- **WHEN** the runtime reports the file as oversized or binary
- **THEN** the dialog SHALL state that condition in place of content
- **AND** it SHALL offer no way to attach a reference, because such a file contributes no content to the prompt and the existing "Reject unsafe reference" requirement already calls for it to be refused

#### Scenario: File is unavailable
- **WHEN** the runtime reports the file as missing or the request fails
- **THEN** the dialog SHALL report that with concise localized feedback and SHALL offer no way to attach a reference

#### Scenario: Large files stay responsive
- **WHEN** the previewed file is large enough that rendering every line at once would stall the interface
- **THEN** the preview SHALL remain interactive
- **AND** a line outside the initially rendered region SHALL still be reachable and selectable

#### Scenario: Keyboard access
- **WHEN** the dialog is open
- **THEN** focus SHALL be confined to it, dismissing SHALL be possible from the keyboard, and focus SHALL return to the composer when it closes

#### Scenario: Web runtime parity
- **WHEN** the preview is opened in Web/mock mode
- **THEN** it SHALL read through the same service contract and behave the same over the mock workspace

### Requirement: Attach file references by dragging or pasting a workspace path
The Files tab SHALL be able to hand a file path to the chat composer by drag-and-drop and through the clipboard, so that a file already visible in the workspace does not have to be retyped as a mention.

#### Scenario: Drag a file onto the composer
- **WHEN** a user drags a file row from the Files tab and drops it on the chat composer
- **THEN** a reference to that file SHALL be attached
- **AND** the reference SHALL carry no line range

#### Scenario: Directories are not draggable
- **WHEN** a user attempts to drag a directory row
- **THEN** no drag SHALL start and nothing SHALL be attachable from it

#### Scenario: The drop target is discoverable
- **WHEN** a draggable file is over the composer
- **THEN** the composer SHALL show that it will accept the drop
- **AND** the affordance SHALL disappear when the pointer leaves or the drop completes

#### Scenario: Copy a path from the Files tab
- **WHEN** a user invokes the copy-path action on a file row
- **THEN** the session-relative path SHALL be placed on the clipboard as plain text
- **AND** it SHALL additionally carry the application's file-path clipboard type

#### Scenario: Paste a copied path into the composer
- **WHEN** a user pastes clipboard content carrying the application's file-path clipboard type into the composer
- **THEN** a reference to that file SHALL be attached instead of the path being inserted as text
- **AND** the reference SHALL carry no line range

#### Scenario: Ordinary paste is unaffected
- **WHEN** a user pastes content that does not carry that clipboard type
- **THEN** the composer SHALL insert it as text exactly as it did before
- **AND** the composer SHALL NOT inspect pasted text to guess whether it names a file

#### Scenario: Pasting a copied path elsewhere
- **WHEN** a copied path is pasted into any target other than the composer
- **THEN** it SHALL arrive as the plain-text path

#### Scenario: Attached the same way as a typed mention
- **WHEN** a reference is attached by drop or paste
- **THEN** duplicate detection, the maximum number of references per message, and chip display SHALL behave exactly as for a reference attached by typing a mention

#### Scenario: Dropping or pasting while sending is not allowed
- **WHEN** the composer is disabled or a generation is streaming
- **THEN** a drop or a path paste SHALL NOT attach a reference

#### Scenario: External sources are not accepted
- **WHEN** content is dragged or pasted from outside the application
- **THEN** it SHALL NOT be treated as a file reference
- **AND** ordinary text handling SHALL apply

