## ADDED Requirements

### Requirement: Policy template governs interactive launch parameters for antigravity-cli
The system SHALL project an agent principal's assigned policy template (`readonly`, `standard`, `trusted`, or `yolo`) into Antigravity CLI's own native execution-mode and sandbox launch controls whenever that agent's Agent Terminal starts interactively, using `--mode` and `--sandbox`.

#### Scenario: Readonly template plans without applying changes
- **WHEN** an agent principal with the `readonly` template starts an Agent Terminal for `antigravity-cli`
- **THEN** the launch SHALL use the `plan` execution mode and enable the terminal sandbox

#### Scenario: Standard template leaves the CLI's own ask-before-acting default in charge
- **WHEN** an agent principal with the `standard` template starts an Agent Terminal for `antigravity-cli`
- **THEN** the launch SHALL NOT override the execution mode
- **AND** it SHALL NOT enable the sandbox, so the tool's own configured approval prompting governs risky actions

#### Scenario: Trusted and yolo templates project identically
- **WHEN** an agent principal with the `trusted` or `yolo` template starts an Agent Terminal for `antigravity-cli`
- **THEN** both templates SHALL use the `accept-edits` execution mode
- **AND** both SHALL produce the same launch parameters

#### Scenario: The bypass flag is never introduced by a template
- **WHEN** any policy template is projected for `antigravity-cli`
- **THEN** the resulting launch parameters SHALL NOT include `--dangerously-skip-permissions`
- **AND** because that flag is also absent from the parameter catalog, no launch path SHALL be able to introduce it

#### Scenario: Template resolution failure fails the Antigravity launch
- **WHEN** the policy template lookup fails while starting an Agent Terminal for `antigravity-cli`
- **THEN** the launch SHALL fail with an error
- **AND** the system SHALL NOT silently substitute a template to keep the launch proceeding
