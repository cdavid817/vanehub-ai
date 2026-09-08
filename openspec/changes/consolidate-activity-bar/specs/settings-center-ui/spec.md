## MODIFIED Requirements

### Requirement: Workflow-oriented settings navigation order
The Settings sidebar SHALL order destinations by expected workflow frequency: general setup and recurring Agent behavior first, reusable capabilities and customization next, one-time CLI installation and external integrations after that, and diagnostics and product information last, with Help pinned in a bottom group.

#### Scenario: Render settings destinations
- **WHEN** the Settings sidebar renders
- **THEN** destinations SHALL appear in the order Basic, Agent Configuration, Agent Policies, CLI Parameters, Agent Evaluation, Code Intelligence, MCP, Skills, Personalization, Prompt Hooks, Expert Roles, Local Media, CLI Management, Extensions, Plugin Integrations, IM, SSH Connections, Observability, Usage Statistics, and About
- **AND** Help SHALL render in a bottom group after those destinations
- **AND** existing destination ids and deep-link behavior SHALL remain unchanged

#### Scenario: Open the evaluation page
- **WHEN** the user activates Agent Evaluation in the settings sidebar or opens `/settings?section=evaluation`
- **THEN** the settings center SHALL render the existing evaluation center with its behavior unchanged

#### Scenario: Manage system activity projections from observability
- **WHEN** the user opens the Observability settings page
- **THEN** it SHALL show a System activity section hosting the export, rebuild, and projection health controls
- **AND** those controls SHALL call the same frontend service methods they called on the System Activity surface
