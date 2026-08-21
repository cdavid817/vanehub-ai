# cli-agent-permission-launch-flags Specification

## Purpose
Projects an agent principal's assigned policy template (`readonly`, `standard`, `trusted`, or `yolo`) into `gemini-cli`, `codex-cli`, and `opencode`'s own native launch-time approval and sandbox controls, reusing each tool's existing graduated modes rather than raw bypass flags, and takes precedence over the user's persisted CLI Parameter selections for the specific keys it governs.
## Requirements
### Requirement: Policy template governs managed CLI launch parameters
The system SHALL combine an agent principal's assigned policy template with the session execution mode and project the resolved effective policy into `claude-code`, `gemini-cli`, `codex-cli`, `opencode`, and `antigravity-cli` whenever a chat process or Agent Terminal process starts.

#### Scenario: Readonly effective policy projects to a silent-deny launch
- **WHEN** the resolved effective policy is `readonly`
- **THEN** the launch SHALL use that tool's most restrictive available execution, sandbox, and approval combination
- **AND** a chat or terminal launch SHALL NOT gain write capability from saved CLI settings

#### Scenario: Ask effective policy uses the provider's supported approval posture
- **WHEN** the resolved effective policy is `ask`
- **THEN** the launch SHALL use that tool's supported approval behavior for risky actions
- **AND** an interactive Agent Terminal SHALL keep any native prompt answerable through its PTY

#### Scenario: Allow effective policy uses non-bypass permissive controls
- **WHEN** the resolved effective policy is `allow`
- **THEN** the launch SHALL use the most permissive combination available within the tool's existing non-bypass controls

#### Scenario: Chat and terminal resolve consistently
- **WHEN** chat and Agent Terminal launches use the same stable agent id and effective policy
- **THEN** both launch scopes SHALL enforce the same safety posture even when provider grammar requires different argument placement

### Requirement: Only catalog-legal, non-bypass parameter values are used
The system SHALL express every policy-template projection using values already defined in that tool's existing CLI parameter catalog and SHALL NOT introduce a raw bypass flag (for example, any flag whose name contains "dangerously") to reach a template's intended behavior.

#### Scenario: Yolo template does not introduce a bypass flag
- **WHEN** the `yolo` template is projected for any of `codex-cli`, `gemini-cli`, or `opencode`
- **THEN** the resulting launch parameters SHALL be limited to values already present in that tool's catalog
- **AND** no flag whose name contains "dangerously" SHALL be introduced to achieve it

### Requirement: OpenCode's standard template injects an environment variable
Because no existing `opencode` catalog value expresses "ask before edits or shell commands, remain permissive for reads," the system SHALL inject an `OPENCODE_PERMISSION` environment variable expressing that posture when the `standard` template is projected for `opencode`.

#### Scenario: Standard template sets ask-level permissions for edits and shell
- **WHEN** an agent principal with the `standard` template starts an Agent Terminal for `opencode`
- **THEN** the generated terminal wrapper SHALL export `OPENCODE_PERMISSION` with `edit` and `bash` set to ask
- **AND** the user's saved `agent` parameter selection SHALL remain unchanged

### Requirement: Gemini's standard template explicitly emits its approval flag
The system SHALL explicitly emit `--approval-mode default` when the `standard` template is projected for `gemini-cli`, even though the general CLI-parameter convention omits a flag whose selected value is the literal string `default`.

#### Scenario: Standard template does not fall through to the user's own Gemini settings
- **WHEN** an agent principal with the `standard` template starts an Agent Terminal for `gemini-cli`
- **THEN** the launch arguments SHALL include `--approval-mode default`
- **AND** this SHALL hold regardless of the general convention that a `default`-valued selection omits its flag

### Requirement: Template reassignment affects only future launches
Reassigning an Agent policy template SHALL NOT alter a chat generation or Agent Terminal process already running for that Agent.

#### Scenario: Already-running terminal keeps its original parameters
- **WHEN** an Agent policy template is reassigned while a managed CLI process is already running
- **THEN** the running process SHALL continue with the parameters it was launched with
- **AND** the next chat generation or Agent Terminal launch SHALL use the newly assigned template

### Requirement: Template resolution failure fails the launch
If the system cannot resolve an Agent policy template when starting any managed CLI chat or interactive launch, it SHALL fail the launch rather than proceed with an unresolved or guessed template.

#### Scenario: Lookup failure surfaces an error
- **WHEN** policy-template lookup fails while starting a managed CLI process
- **THEN** the launch SHALL fail with an error
- **AND** the system SHALL NOT silently substitute a template

### Requirement: Policy template governs launch parameters for antigravity-cli
The system SHALL project Antigravity CLI's resolved effective policy into its native execution-mode and sandbox controls for chat and Agent Terminal launches, using `--mode` and `--sandbox` without a bypass flag.

#### Scenario: Readonly policy plans without applying changes
- **WHEN** the resolved effective policy is `readonly`
- **THEN** the launch SHALL use `--mode plan` and enable the sandbox

#### Scenario: Ask policy uses the CLI default approval posture
- **WHEN** the resolved effective policy is `ask`
- **THEN** the launch SHALL use the CLI's default execution mode without enabling a permissive bypass

#### Scenario: Allow policy accepts edits
- **WHEN** the resolved effective policy is `allow`
- **THEN** the launch SHALL use `--mode accept-edits` without enabling a raw bypass flag

#### Scenario: The bypass flag is never introduced
- **WHEN** any effective policy is projected for `antigravity-cli`
- **THEN** the resulting launch parameters SHALL NOT include `--dangerously-skip-permissions`

### Requirement: Claude Code keeps hook enforcement as the action-level boundary
Claude Code SHALL combine its effective launch mode with the existing authenticated permission hook, and the hook SHALL remain authoritative for every mapped action it intercepts.

#### Scenario: Claude Code launches under readonly policy
- **WHEN** Claude Code resolves to a read-only effective policy
- **THEN** its launch mode SHALL be `plan`
- **AND** mapped tool calls SHALL continue through the permission-hook decision pipeline

#### Scenario: Claude Code launches under ask or allow policy
- **WHEN** Claude Code resolves to `ask` or `allow`
- **THEN** its launch parameters SHALL NOT disable the existing permission hook
- **AND** hook-mapped MCP actions SHALL retain the permissions-core Ask floor

### Requirement: Claude Code launches declare managed hook ownership
The system SHALL inject an explicit managed permission-hook scope into Claude Code chat and interactive terminal processes launched by VaneHub. The scope SHALL be inherited by Claude Code hook subprocesses and SHALL NOT be injected into other managed CLIs or processes launched independently of VaneHub.

#### Scenario: VaneHub launches Claude Code chat
- **WHEN** VaneHub starts a chat process for stable Agent id `claude-code`
- **THEN** the process environment SHALL declare the managed permission-hook scope

#### Scenario: VaneHub launches a Claude Code terminal
- **WHEN** VaneHub starts an interactive terminal process for stable Agent id `claude-code`
- **THEN** the generated terminal launch environment SHALL declare the managed permission-hook scope

#### Scenario: VaneHub launches another managed CLI
- **WHEN** VaneHub starts Codex CLI, Gemini CLI, OpenCode, or Antigravity CLI
- **THEN** the Claude Code managed permission-hook scope SHALL NOT be present in that process environment
- **AND** the CLI's existing launch-time permission projection SHALL remain unchanged
