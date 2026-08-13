## ADDED Requirements

### Requirement: OnePiece project discovery execution profile
The native OnePiece runtime SHALL support a planning discovery profile that is bound to one canonical local project, advertises only the approved read-only discovery tools, applies independent tool, token, context, and time limits, and returns structured Plan output through the captured active Profile without copying credentials.

#### Scenario: Run bounded discovery
- **WHEN** task orchestration requests project-aware planning with a ready captured OnePiece Profile
- **THEN** OnePiece SHALL perform only allowed workspace-scoped discovery and return the requested strict Plan structure with discovery limitation metadata

#### Scenario: Model requests a prohibited planning tool
- **WHEN** OnePiece requests shell, file mutation, MCP, memory mutation, arbitrary network, or an operation outside the canonical project during discovery
- **THEN** the runtime SHALL reject the call regardless of model output and SHALL preserve an actionable planning failure

### Requirement: OnePiece repair execution profile
The native OnePiece runtime SHALL support a repair profile that starts a distinct attempt session in the retained PlanRun worktree and receives only the current SubTask or final-repair instructions, acceptance criteria, bounded prior failure evidence, current changed-file summary, and snapshotted limits.

#### Scenario: Start a repair Attempt
- **WHEN** task orchestration dispatches an eligible repair
- **THEN** OnePiece SHALL receive bounded failed-check evidence without raw predecessor transcripts, credentials, unbounded command output, or unrelated historical attempts

#### Scenario: Repair reaches a limit
- **WHEN** the repair session reaches its tool, token, or timeout limit
- **THEN** OnePiece SHALL stop through the existing safe limit boundary and the attempt SHALL retain a classified terminal outcome

