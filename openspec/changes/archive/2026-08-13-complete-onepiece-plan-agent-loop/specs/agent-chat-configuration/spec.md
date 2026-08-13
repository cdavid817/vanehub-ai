## ADDED Requirements

### Requirement: OnePiece Plan and Agent modes remain visibly distinct
The OnePiece conversation surface SHALL present the effective permission mode with persistent icon and text semantics that distinguish read-only Plan behavior from write-capable Agent behavior, SHALL describe the effective capability boundary without relying on color alone, and SHALL adapt the primary composer action to the current mode.

#### Scenario: Work in Plan mode
- **WHEN** a OnePiece session uses Plan permission mode
- **THEN** the surface SHALL identify the mode as read-only and SHALL present planning-oriented composer guidance and actions
- **AND** the runtime SHALL continue enforcing the restricted Plan tool catalog independently of the presentation

#### Scenario: Work in Agent mode
- **WHEN** a OnePiece session uses Agent permission mode
- **THEN** the surface SHALL identify that approved workspace mutations and guarded validation may occur
- **AND** it SHALL continue exposing the applicable approval and stop controls

#### Scenario: Announce mode accessibly
- **WHEN** the effective OnePiece mode or PlanRun phase changes
- **THEN** assistive technology SHALL receive the mode name, capability descriptor, and phase without requiring color interpretation

### Requirement: Approved Plan transition controls write capability
The system SHALL NOT transition a reviewed OnePiece Plan into write-capable PlanRun execution without explicit user approval, and it SHALL present the project, task count, verification scope, worktree behavior, and available continue-planning action at that boundary.

#### Scenario: Continue planning
- **WHEN** a user declines approval or chooses to continue planning
- **THEN** the session SHALL remain in Plan mode and SHALL NOT create a PlanRun, integration worktree, or write-capable execution session

#### Scenario: Approve and execute
- **WHEN** a user approves a valid current Plan version
- **THEN** the system SHALL freeze that version, prepare its PlanRun, and present the session as entering Agent execution

#### Scenario: Request planning during active execution
- **WHEN** a user requests Plan mode while a PlanRun attempt is active
- **THEN** the system SHALL require a durable pause request and a safe attempt boundary before allowing planning changes
- **AND** the composer SHALL continue presenting the effective write-capable Agent state until the associated PlanRun projection confirms that boundary

### Requirement: OnePiece sessions retain a single PlanRun navigation source
An approved PlanRun MAY retain the opaque id of the OnePiece session from which planning began. The composer and Plan Center SHALL resolve that association through the shared Plan service, SHALL NOT infer it from the most recent global run, and SHALL keep attempt-scoped execution sessions distinct from the originating session.

#### Scenario: Open the associated PlanRun
- **WHEN** an originating OnePiece session has an associated PlanRun
- **THEN** its conversation surface SHALL expose a clear keyboard-operable action that opens that PlanRun in Plan Center

#### Scenario: Session has no associated PlanRun
- **WHEN** a OnePiece session has no associated PlanRun
- **THEN** the conversation surface SHALL omit the PlanRun navigation action and SHALL NOT navigate to an unrelated run
