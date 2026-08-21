## MODIFIED Requirements

### Requirement: OnePiece Plan and Agent modes remain visibly distinct
The OnePiece conversation bar SHALL present the effective execution mode with persistent icon and text semantics that distinguish read-only Plan behavior from write-capable Agent behavior, SHALL describe the effective capability boundary without relying on color alone, and SHALL adapt the primary composer action to the current mode.

#### Scenario: Work in Plan mode
- **WHEN** a OnePiece session uses Plan execution mode
- **THEN** the conversation bar SHALL identify the mode as read-only and SHALL present planning-oriented composer guidance and actions
- **AND** the runtime SHALL continue enforcing the restricted Plan tool catalog independently of the presentation

#### Scenario: Work in Agent mode
- **WHEN** a OnePiece session resolves to write-capable Agent behavior
- **THEN** the conversation bar SHALL identify that approved workspace mutations and guarded validation may occur
- **AND** it SHALL continue exposing the applicable approval and stop controls

#### Scenario: Announce mode accessibly
- **WHEN** the effective OnePiece execution mode changes
- **THEN** assistive technology SHALL receive the mode name and capability descriptor without requiring color interpretation

## REMOVED Requirements

### Requirement: Approved Plan transition controls write capability
**Reason**: The transition was coupled to the retired versioned Plan and PlanRun workflow. OnePiece now changes only its session execution mode through the existing in-conversation approval contract.

**Migration**: Use the OnePiece conversation bar to remain in Plan mode or use the session-scoped `exit_plan_mode` approval flow to enable write-capable behavior on a later turn.

### Requirement: OnePiece sessions retain a single PlanRun navigation source
**Reason**: PlanRun and Plan Center navigation are removed, so a OnePiece session no longer has a separate execution destination to resolve.

**Migration**: Keep planning, approval, and subsequent Agent-mode conversation in the originating OnePiece session.

