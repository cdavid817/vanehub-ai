## ADDED Requirements

### Requirement: API provider invocation usage accounting
Every API-based Agent model request SHALL emit a normalized accounting observation when valid provider usage is available, including user-visible, tool-continuation, compaction, memory-extraction, failed, cancelled, and retry attempts.

#### Scenario: Capture Anthropic streaming usage
- **WHEN** an Anthropic Messages stream reports input or cache usage at message start and output usage during message progress or completion
- **THEN** the runtime SHALL combine those events into one invocation observation
- **AND** it SHALL finalize that observation with the provider's authoritative values and semantic mapping

#### Scenario: Capture supported OpenAI-compatible streaming usage
- **WHEN** a catalog endpoint declares a supported streaming usage strategy and its final usage chunk arrives
- **THEN** the runtime SHALL normalize that chunk into one invocation observation
- **AND** endpoint-specific cache and reasoning dimensions SHALL follow the declared strategy

#### Scenario: Avoid speculative paid retry
- **WHEN** an endpoint does not declare support for an optional usage request parameter
- **THEN** the runtime SHALL NOT retry a potentially accepted model request merely to add or remove that parameter
- **AND** absent usage SHALL degrade to the configured estimation behavior

#### Scenario: Account for tool round trips
- **WHEN** an API Agent executes tools and sends their results through additional provider requests
- **THEN** every request SHALL receive a distinct invocation sequence and `tool-continuation` purpose
- **AND** all unique invocations SHALL contribute to the owning generation projection

#### Scenario: Account for internal model calls
- **WHEN** API-Agent context compaction or automatic memory extraction invokes the provider
- **THEN** the call SHALL be recorded with its internal purpose separately from final-response consumption

