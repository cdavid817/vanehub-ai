## MODIFIED Requirements

### Requirement: Dedicated Loop Center
The workspace SHALL provide a dedicated Loop Center, hosted as the Loops tab of the Automations destination, for managing definitions and runs without presenting Loop execution as a normal chat tab or scheduled-task dialog.

#### Scenario: Open Loop Center
- **WHEN** the user activates the Loops tab of the Automations destination or opens `/workspace/automations/loops`
- **THEN** the workspace SHALL show the Loop definition and run list, selected run timeline, and configuration or control inspector

#### Scenario: Render empty state
- **WHEN** no Loop definitions exist
- **THEN** the Loop Center SHALL show a localized empty state with a create-Loop action
