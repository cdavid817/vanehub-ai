## ADDED Requirements

### Requirement: Native Agent MCP runtime verification
The repository SHALL provide an isolated WebdriverIO desktop verification layer that creates a real deterministic MCP fixture and proves Agent-level tool availability and invocation rather than only settings persistence or connection testing.

#### Scenario: Verify single-Agent MCP use
- **WHEN** the Agent MCP desktop layer runs on a supported host
- **THEN** it SHALL launch the native desktop artifact and verify MCP use through `claude-code`, `codex-cli`, `opencode`, and `onepiece` single-Agent sessions
- **AND** the evidence SHALL distinguish persistence, catalog/configuration exposure, protocol invocation, and terminal outcome

#### Scenario: Verify multi-Agent MCP use
- **WHEN** the Agent MCP desktop layer creates a heterogeneous multi-Agent session and routes turns across its seats
- **THEN** it SHALL prove that the created MCP server is usable from every tested seat invocation

#### Scenario: Isolate Agent MCP verification
- **WHEN** the Agent MCP desktop layer starts
- **THEN** it SHALL use run-scoped application data, CLI configuration, provider fixtures, MCP fixtures, logs, and process cleanup
- **AND** it MUST NOT call a real model, read user credentials, or mutate provider-global configuration

#### Scenario: Agent MCP verification cannot run
- **WHEN** a platform prerequisite prevents the native desktop layer from executing
- **THEN** the layer SHALL report `BLOCKED` with the missing prerequisite and retain its run-scoped evidence
- **AND** it SHALL NOT report the unexecuted Agent or MCP path as passed
