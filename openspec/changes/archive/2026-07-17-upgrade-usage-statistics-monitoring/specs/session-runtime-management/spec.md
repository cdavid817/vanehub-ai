## ADDED Requirements

### Requirement: Session runtime captures normalized CLI usage
The desktop session runtime SHALL extract and normalize valid usage reported by supported Agent CLI events while keeping each generation's accounting isolated by session and assistant message id.

#### Scenario: Capture supported reported usage
- **WHEN** Claude Code, Codex CLI, Gemini CLI, or OpenCode emits valid usage during a VaneHub-managed generation
- **THEN** the runtime SHALL normalize available fresh-input, output, cache-read, and cache-creation token categories for that generation
- **AND** it SHALL retain optional provider/model identity when reported by the CLI

#### Scenario: Handle repeated or cumulative usage events
- **WHEN** a CLI emits repeated terminal observations or cumulative counters
- **THEN** the runtime SHALL derive one non-negative response-level usage observation without double counting
- **AND** accounting for another active session SHALL remain unchanged

#### Scenario: Complete without reported usage
- **WHEN** a generation completes successfully without a valid reported usage observation
- **THEN** the runtime SHALL create a character-count estimate labeled as estimated rather than reported tokens
- **AND** missing reported usage alone SHALL NOT fail the generation

#### Scenario: Record reported usage before unsuccessful termination
- **WHEN** a generation fails or is cancelled after the CLI has emitted valid reported usage
- **THEN** the runtime SHALL retain that reported usage because consumption may already have occurred
- **AND** it SHALL NOT fabricate estimated usage for an unsuccessful generation with no reported usage

#### Scenario: Diagnose malformed usage safely
- **WHEN** a supported CLI emits an unrecognized or malformed usage shape
- **THEN** the runtime MAY write a rate-limited `debug` or `warn` diagnostic through unified logging with session and Agent context
- **AND** it SHALL NOT persist raw prompts, responses, complete CLI events, credentials, or unredacted sensitive values in the diagnostic
