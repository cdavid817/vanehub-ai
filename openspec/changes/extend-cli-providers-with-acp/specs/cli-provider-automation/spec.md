## ADDED Requirements

### Requirement: Shared capability gating for automated execution

Multi-agent, scheduled-task and applicable CLI-delegation entry points SHALL use the same runtime preflight and provider capability assessment as single-agent managed sessions. Capability SHALL be checked at configuration time and again immediately before execution.

#### Scenario: Saved schedule starts after a CLI update

- **WHEN** the active CLI version or policy differs from the saved configuration
- **THEN** the run SHALL revalidate compatibility and policy before submitting a prompt

#### Scenario: Legacy provider is selected for automation

- **WHEN** a caller attempts unattended iFlow execution in this change
- **THEN** the backend SHALL reject it regardless of frontend filtering

### Requirement: Per-seat and per-run isolation

Each agent seat and scheduled run SHALL have its own execution binding, process ownership, workspace, external session, connection epoch and scoped credential/policy context. Provider-owned subagents SHALL remain child activity of that binding. Cross-binding process reuse is out of scope.

#### Scenario: Two seats use the same provider

- **WHEN** two Qwen or Copilot seats execute concurrently
- **THEN** their prompts, output, approvals, cwd and credentials SHALL not cross between bindings

#### Scenario: Provider creates internal subagents

- **WHEN** a CLI reports subagent events
- **THEN** they SHALL retain parent provenance and SHALL not become independent VaneHub seats automatically

### Requirement: Explicit unattended interaction policy

An unattended run SHALL have an explicit bounded interaction policy. Missing required human approval or input SHALL cause a documented blocked, escalation, rejection or cancellation outcome through existing task facilities. Scheduling a task MUST NOT imply broad tool approval or bypass permissions.

#### Scenario: A scheduled run requests approval

- **WHEN** an automated agent requests a permission outside its explicit preauthorization scope
- **THEN** the run SHALL require intervention or cancel according to configured policy and SHALL NOT auto-approve

#### Scenario: A background run asks a question

- **WHEN** a blocking provider extension requests user input without an available responder
- **THEN** the task SHALL record a needs-intervention or cancelled outcome before the configured deadline

#### Scenario: A narrow preauthorization exists

- **WHEN** the requested action matches an explicit preauthorized tool and scope
- **THEN** the runtime SHALL apply that authorization only to the matching action without widening it

### Requirement: Checkpointed interruption without duplicate effects

Automated runs SHALL preserve existing scheduler checkpoints and task-verification semantics while recording uncertain CLI effects after interruption. Automatic retry SHALL require an existing verified idempotent workflow rule and SHALL NOT blindly replay a whole prompt that may have produced side effects.

#### Scenario: Agent disconnects after a possible commit

- **WHEN** a connection is lost before its prompt outcome but a repository change may exist
- **THEN** the scheduler SHALL record uncertain effects and avoid repeating the commit instruction blindly

#### Scenario: One seat is cancelled

- **WHEN** a multi-agent seat is cancelled while other seats continue
- **THEN** only that seat resources SHALL be stopped unless the existing group policy explicitly cancels the group
