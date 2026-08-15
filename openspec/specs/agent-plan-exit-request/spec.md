# agent-plan-exit-request Specification

## Purpose
TBD - created by archiving change add-agent-plan-exit-request. Update Purpose after archive.
## Requirements
### Requirement: Model-initiated request to leave plan mode
The system SHALL expose an `exit_plan_mode` tool that submits the plan the model proposes to carry out, publishes it to the session's chat surface, and blocks that tool call until the user approves or declines. The tool SHALL be offered only while the session is in plan mode. The system SHALL NOT leave plan mode on the model's request alone, and SHALL NOT treat an unanswered request, a cancellation, or a decline as approval.

#### Scenario: Plan is presented and approved
- **WHEN** the native agent calls `exit_plan_mode` in an interactive plan-mode session
- **THEN** the system SHALL publish the proposed plan to that session's chat surface
- **AND** the tool call SHALL remain unresolved until the user decides
- **AND** on approval the session SHALL move from plan mode to execute mode

#### Scenario: Plan is declined
- **WHEN** the user declines a published `exit_plan_mode` request
- **THEN** the session SHALL remain in plan mode
- **AND** the tool result SHALL report the decline rather than an approval

#### Scenario: Request is never decided
- **WHEN** an `exit_plan_mode` request is cancelled or its generation ends before a decision
- **THEN** the session SHALL remain in plan mode

#### Scenario: Tool is absent outside plan mode
- **WHEN** a generation starts in any mode other than plan mode
- **THEN** `exit_plan_mode` SHALL NOT be offered in that generation's tool catalog

#### Scenario: No interactive user
- **WHEN** the native agent calls `exit_plan_mode` in a non-interactive execution context
- **THEN** the system SHALL return an error result explaining that no decision can be taken there
- **AND** it SHALL NOT block the call

#### Scenario: Plan argument is unusable
- **WHEN** an `exit_plan_mode` call supplies an empty plan, or one longer than the declared maximum
- **THEN** the system SHALL reject the call with a message identifying the problem
- **AND** it SHALL NOT publish anything to the chat surface

### Requirement: Approval applies from the next turn
An approved `exit_plan_mode` request SHALL take effect for generations started after it. The system SHALL NOT change the tools declared to the provider for the generation that requested it, and the tool result SHALL state that write-capable tools become available on the following turn.

#### Scenario: Tools declared for the requesting generation
- **WHEN** an `exit_plan_mode` request is approved during a generation
- **THEN** that generation SHALL continue with the tools it was started with

#### Scenario: Following generation
- **WHEN** a generation starts after an approved `exit_plan_mode` request
- **THEN** it SHALL resolve its catalog and policy from execute mode

### Requirement: Leaving plan mode authorizes no action
A decision on an `exit_plan_mode` request SHALL NOT create a permission record or a standing grant. It SHALL authorize only that session's move out of plan mode, and only once.

#### Scenario: Approval is recorded
- **WHEN** a user approves an `exit_plan_mode` request
- **THEN** the system SHALL NOT record a permission grant for any action or resource

#### Scenario: A later plan-mode session
- **WHEN** a session later returns to plan mode
- **THEN** leaving it SHALL again require an explicit decision

### Requirement: Plan approval does not enter PlanRun execution
An approved `exit_plan_mode` request SHALL change only the session's execution mode. The system SHALL NOT freeze a Plan version, create a PlanRun, or provision an integration worktree in response to it.

#### Scenario: Approval does not create a PlanRun
- **WHEN** an `exit_plan_mode` request is approved
- **THEN** the system SHALL NOT create a PlanRun, Plan version, or integration worktree

