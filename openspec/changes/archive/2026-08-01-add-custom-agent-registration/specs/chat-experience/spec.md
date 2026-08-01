## ADDED Requirements

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

## MODIFIED Requirements

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
