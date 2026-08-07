## MODIFIED Requirements

### Requirement: Agent policy list surfaces every eligible agent's current template
The system SHALL provide a settings surface listing every custom API agent, the built-in OnePiece agent, and the stable `claude-code` CLI principal, each showing its currently assigned policy template, without requiring the user to inspect storage directly.

#### Scenario: Custom agents and OnePiece appear in the list
- **WHEN** a user opens the agent policy settings surface
- **THEN** the system SHALL display every agent with `agentOrigin` of `user`, plus the OnePiece agent, each with its current policy template

#### Scenario: An agent with no explicit assignment shows the effective default
- **WHEN** a listed agent has never been assigned a policy template
- **THEN** the system SHALL display the current default template as its effective template, rather than an empty or unknown state

#### Scenario: The claude-code CLI principal appears in the list
- **WHEN** a user opens the agent policy settings surface
- **THEN** the system SHALL display the `claude-code` principal alongside custom agents and OnePiece, with its current policy template or effective default

## ADDED Requirements

### Requirement: Enabling Claude Code hook management requires a distinct first-use confirmation
The system SHALL, before the first policy template assignment to the `claude-code` principal takes effect, present a confirmation identifying that the action installs a permission hook into the user's global Claude Code configuration and affects Claude Code usage outside VaneHub, and SHALL NOT install that hook or apply the template until the user confirms. This confirmation is independent of, and in addition to, the existing trusted/yolo confirmation.

#### Scenario: First template assignment requires the installation confirmation
- **WHEN** a user assigns any policy template to the `claude-code` principal for the first time
- **THEN** the system SHALL present a confirmation naming the global `settings.json` side effect before installing the hook or applying the template

#### Scenario: Subsequent template changes do not repeat the installation confirmation
- **WHEN** a user changes the `claude-code` principal's template after the hook has already been installed
- **THEN** the system SHALL NOT present the installation confirmation again
- **AND** SHALL still present the existing trusted/yolo confirmation when the new template is `trusted` or `yolo`

#### Scenario: Declining the confirmation leaves the hook uninstalled
- **WHEN** a user declines the first-use confirmation
- **THEN** the system SHALL NOT write to `~/.claude/settings.json`
- **AND** the `claude-code` principal SHALL remain without an active hook
