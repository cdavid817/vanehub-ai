## ADDED Requirements

### Requirement: Workflow-oriented settings navigation order
The Settings sidebar SHALL order destinations by expected workflow frequency: general setup and recurring Agent behavior first, reusable capabilities and customization next, one-time CLI installation and external integrations after that, and diagnostics and product information last.

#### Scenario: Render settings destinations
- **WHEN** the Settings sidebar renders
- **THEN** destinations SHALL appear in the order Basic, Agent Configuration, Agent Policies, CLI Parameters, MCP, Skills, Personalization, Prompt Hooks, Expert Roles, CLI Management, Extensions, Plugin Integrations, IM, SSH Connections, Observability, Usage Statistics, and About
- **AND** existing destination ids and deep-link behavior SHALL remain unchanged
