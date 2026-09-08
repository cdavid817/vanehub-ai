## ADDED Requirements

### Requirement: Policy projection before all new CLI launches

Every new CLI launch, including terminal and unattended paths, SHALL evaluate the current permission policy through the existing permissions boundary. If the chosen execution mode cannot enforce the requested requirements, launch SHALL be rejected with a stable reason. ACP support MUST NOT be presented as an OS sandbox.

#### Scenario: Requested policy cannot be enforced

- **WHEN** a CLI mode cannot enforce a required restriction
- **THEN** preflight SHALL reject the launch before submitting the task
- **AND** no bypass option SHALL be added to make it succeed

#### Scenario: Delegated tool enforcement is used

- **WHEN** a tool executes inside the CLI without host proxy control
- **THEN** the UI SHALL identify the guarantee as provider-delegated or unverified rather than host-enforced

### Requirement: Permission request ownership and exact decision mapping

Permission interactions SHALL bind connection epoch, session, turn, RPC request, tool call and policy revision. Responses SHALL consume a request at most once and select only an original offered option with equivalent scope. A narrower user decision MUST NOT become a broader authorization.

#### Scenario: User allows once

- **WHEN** the user selects a one-time permission option
- **THEN** the adapter SHALL return the corresponding offered one-time option id and SHALL NOT grant persistent approval

#### Scenario: Duplicate or cross-session reply

- **WHEN** a reply repeats a consumed request or belongs to another session or epoch
- **THEN** the backend SHALL reject it without sending an approval

#### Scenario: Policy changes before reply

- **WHEN** the applicable policy revision changes while a request is pending
- **THEN** the backend SHALL re-evaluate or invalidate the interaction before responding

### Requirement: Safe cancellation dismissal and deadlines

Closing, timing out, cancelling, or losing the connection for an interaction SHALL NOT approve it. The host SHALL return an appropriate protocol cancellation or rejection when possible and clear pending state. UI refresh alone SHALL not destroy live backend-owned requests.

#### Scenario: User dismisses a dialog

- **WHEN** a user closes the decision dialog without selecting approval
- **THEN** the task SHALL remain explicitly waiting or receive a documented cancellation; it SHALL NOT proceed as approved

#### Scenario: Interaction deadline expires

- **WHEN** no response is available before the configured deadline
- **THEN** the host SHALL reject or cancel using the documented protocol outcome

#### Scenario: UI refreshes

- **WHEN** the frontend reloads while a live connection has pending interactions
- **THEN** the frontend SHALL recover valid requests from the backend without re-executing them

### Requirement: Permission-controlled client file operations

Client filesystem methods SHALL execute only after capability advertisement, session ownership validation, and authorization against canonical allowed roots using existing file and permission APIs. Path traversal, symlink/junction escape, device paths, and unsafe races MUST be handled without treating a string prefix as an authorization boundary.

#### Scenario: Agent writes inside an allowed workspace

- **WHEN** a validated file request targets an authorized file inside the session workspace
- **THEN** the host SHALL apply the authorized operation and return an accurate result

#### Scenario: Path escapes through a link

- **WHEN** a nominally contained path resolves outside authorized roots
- **THEN** the host SHALL reject the operation without modifying the target

#### Scenario: Unadvertised file method is invoked

- **WHEN** the peer invokes a file capability the host did not advertise
- **THEN** the host SHALL reject it instead of enabling capabilities dynamically

### Requirement: Permission-controlled client terminal lifecycle

Client terminal methods SHALL reuse the existing governed process execution boundary and implement required create, output, wait, kill and release semantics before advertising support. Every terminal id and operation SHALL be scoped to its connection and session, with bounded output, explicit cwd and controlled environment.

#### Scenario: Authorized command is requested

- **WHEN** a terminal request satisfies the policy and required isolation guarantees
- **THEN** the host SHALL execute it with its scoped cwd and credentials and expose bounded output

#### Scenario: Another session refers to a terminal

- **WHEN** a request references a terminal owned by a different session or epoch
- **THEN** the host SHALL reject access and SHALL NOT reveal its output

#### Scenario: Terminal owner is destroyed

- **WHEN** the owning ACP connection or cancelled run is being torn down
- **THEN** the host SHALL stop and reap its remaining proxy terminals without touching unrelated processes

### Requirement: Provider extension interaction handling

Reviewed blocking provider extensions SHALL enter the same scoped interaction system with provider-specific schema validation and exact response semantics. Unknown methods requiring a response SHALL fail promptly using a protocol error or documented rejection. Non-blocking notifications SHALL not be treated as approval requests.

#### Scenario: Known question extension arrives

- **WHEN** a reviewed blocking extension requests user input
- **THEN** the host SHALL display and validate its supported answer format before replying

#### Scenario: Unknown request contains instructions

- **WHEN** an unreviewed extension asks the host to execute instructions
- **THEN** the host SHALL reject the method and SHALL NOT execute the payload as an instruction

#### Scenario: Plan approval is requested

- **WHEN** an agent requests a decision on a plan
- **THEN** the host SHALL show the plan as untrusted content and require the applicable explicit decision

### Requirement: Scoped credentials MCP and content handling

Credentials SHALL be resolved through existing safe references or CLI-owned authentication and scoped to the selected process/profile. Diagnostic logs SHALL be redacted before persistence. MCP, model and attachment capabilities SHALL be enabled only when both the selected adapter and host support them. Untrusted content SHALL not change permission policy.

#### Scenario: Child process environment is assembled

- **WHEN** a provider starts with an explicit account profile
- **THEN** the host SHALL include only required environment values and SHALL NOT expose unrelated provider secrets

#### Scenario: Unsupported MCP scope is requested

- **WHEN** the selected CLI or transport does not support the requested MCP source or scope
- **THEN** the host SHALL return an unsupported capability result without editing global configuration

#### Scenario: Agent content tries to alter policy

- **WHEN** tool output, a plan or a source file instructs the host to disable approvals
- **THEN** the host SHALL treat it as data and keep the approved policy unchanged
