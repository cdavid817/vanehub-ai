## ADDED Requirements

### Requirement: Policy template governs interactive launch parameters for gemini-cli, codex-cli, and opencode
The system SHALL project an agent principal's assigned policy template (`readonly`, `standard`, `trusted`, or `yolo`) into `gemini-cli`, `codex-cli`, and `opencode`'s own native approval and sandbox launch parameters whenever that agent's Agent Terminal starts interactively.

#### Scenario: Readonly template projects to a silent-deny launch
- **WHEN** an agent principal with the `readonly` template starts an Agent Terminal for `codex-cli`, `gemini-cli`, or `opencode`
- **THEN** the launch SHALL use that tool's most restrictive available sandbox/approval combination, denying risky actions without prompting

#### Scenario: Standard template enables native ask-every-time prompting
- **WHEN** an agent principal with the `standard` template starts an Agent Terminal for `codex-cli`, `gemini-cli`, or `opencode`
- **THEN** the launch SHALL use that tool's own native interactive approval prompting for risky actions
- **AND** the tool's prompt SHALL render in the terminal and be answerable by the user, since the process is always launched through a real interactive PTY

#### Scenario: Trusted and yolo templates project identically
- **WHEN** an agent principal with the `trusted` or `yolo` template starts an Agent Terminal for `codex-cli`, `gemini-cli`, or `opencode`
- **THEN** both templates SHALL project to the same launch parameters for that tool
- **AND** those parameters SHALL be the most permissive combination available within the tool's existing non-bypass catalog

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
Reassigning an agent's policy template SHALL NOT alter the launch parameters of an Agent Terminal process already running for that agent.

#### Scenario: Already-running terminal keeps its original parameters
- **WHEN** an agent's policy template is reassigned while its Agent Terminal process is already running
- **THEN** the running process SHALL continue with the parameters it was launched with
- **AND** the next launch of that agent's Agent Terminal SHALL use the newly assigned template

### Requirement: Template resolution failure fails the launch
If the system cannot resolve an agent's policy template when starting an interactive launch for `codex-cli`, `gemini-cli`, or `opencode`, it SHALL fail the launch with an error rather than proceed with an unresolved or default-guessed template.

#### Scenario: Lookup failure surfaces an error
- **WHEN** the policy template lookup fails while starting an Agent Terminal for a managed CLI agent
- **THEN** the launch SHALL fail with an error
- **AND** the system SHALL NOT silently substitute a template to keep the launch proceeding
