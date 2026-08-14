## ADDED Requirements

### Requirement: OnePiece fine-grained Token accounting
OnePiece SHALL account for every provider request through the shared API-Agent invocation accounting contract and SHALL preserve Profile, endpoint, provider, and model attribution captured at invocation start.

#### Scenario: Switch Profile after generation starts
- **WHEN** the active OnePiece Profile changes while a generation is running
- **THEN** usage from the running generation SHALL remain attributed to its immutable starting Profile snapshot
- **AND** later requests SHALL use the newly active Profile only when their generation starts

#### Scenario: Compare OnePiece consumption purposes
- **WHEN** OnePiece performs visible response, tool-continuation, compaction, or memory-extraction calls
- **THEN** usage consumers SHALL be able to distinguish each purpose while also viewing total OnePiece consumption

#### Scenario: OnePiece provider omits usage
- **WHEN** an otherwise successful OnePiece request completes without valid provider usage
- **THEN** the runtime SHALL expose reduced reported coverage and apply only the permitted estimation fallback
- **AND** OnePiece SHALL remain usable

#### Scenario: Preserve Web/mock parity
- **WHEN** OnePiece runs in Web/mock mode
- **THEN** the adapter SHALL expose deterministic invocation, purpose, provider, model, and quality fixtures through the shared service contract
- **AND** it SHALL NOT contact a provider

