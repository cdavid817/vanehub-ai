## Purpose

Defines how Skill packages contribute trusted, bounded tools without gaining arbitrary access to the host, user data, or unrelated agent sessions.

## ADDED Requirements

### Requirement: Versioned Skill tool manifest
The system SHALL discover Skill tools only from a versioned manifest in the effective Skill revision. Every tool SHALL declare a stable local id, description, implementation kind, input and output JSON Schemas, requested capabilities, and implementation integrity hash.

#### Scenario: Valid manifest is discovered
- **WHEN** the effective Skill revision contains a supported, schema-valid tool manifest
- **THEN** the system lists each declared tool with its canonical Skill-scoped id and validation state

#### Scenario: Unknown manifest version
- **WHEN** a manifest uses an unsupported version
- **THEN** the system marks its tools unavailable without attempting to execute them

#### Scenario: Undeclared implementation file
- **WHEN** a file exists in the Skill tool directory but is not integrity-bound by the manifest
- **THEN** the system does not register or execute that file

### Requirement: Safe implementation kinds
The system SHALL support bounded declarative tools and optional WebAssembly module tools. It MUST NOT execute Python, shell scripts, batch files, native executables, dynamic libraries, or unrestricted host processes as Skill tool implementations.

#### Scenario: Declarative tool targets an allowed operation
- **WHEN** a declarative tool references an existing registered operation allowed by its manifest
- **THEN** the system can validate and register the tool without creating a new host process

#### Scenario: Host script is supplied
- **WHEN** a Skill tool manifest or package points to a host script or native executable
- **THEN** validation fails closed and no tool from the invalid entry is registered

#### Scenario: Module imports an unavailable host capability
- **WHEN** a WebAssembly module imports a capability outside the runtime allowlist or its manifest declaration
- **THEN** validation fails before the module can run

### Requirement: Capability-based isolated execution
The system SHALL expose no filesystem, network, process, environment, credential, clock, random, or host-tool capability by default. An allowed module invocation SHALL be cancellable and bounded by configured limits for wall time, fuel, memory, host calls, call depth, concurrency, input size, and output size.

#### Scenario: Pure computation tool executes
- **WHEN** a trusted module needs no host capability and remains within every limit
- **THEN** the system returns its schema-valid output to the caller

#### Scenario: Execution limit is exceeded
- **WHEN** a module exceeds any configured resource limit
- **THEN** the system terminates that invocation, records the breached limit, and keeps other tools and sessions available

#### Scenario: Invocation is cancelled
- **WHEN** the parent generation or delegated task is cancelled
- **THEN** the system cancels the Skill tool and any pending host operation without accepting a late result

### Requirement: Independent trust and permission gates
Executable Skill tools SHALL be disabled until the exact effective revision, manifest, and implementation hashes have an accepted trust decision. Trust SHALL only make an implementation eligible to run and MUST NOT grant any requested operational permission.

#### Scenario: Trusted revision requests a protected operation
- **WHEN** a trusted Skill tool delegates an operation governed by permission policy
- **THEN** the system evaluates that operation independently using the Skill tool principal and current execution context

#### Scenario: Effective revision changes
- **WHEN** any integrity-bound manifest or implementation content changes
- **THEN** the previous trust decision no longer enables the changed tool revision

#### Scenario: Untrusted tool is invoked
- **WHEN** a caller attempts to invoke a tool whose effective revision is not trusted
- **THEN** the system rejects the invocation before instantiating its implementation

### Requirement: Executable content cannot arrive through Overlay
The system MUST NOT allow an Overlay patch, learning block, or Overlay file to add, replace, or alter a Skill tool manifest, module, executable implementation, or integrity witness.

#### Scenario: Overlay targets executable content
- **WHEN** an Overlay mutation targets a tool manifest or implementation path
- **THEN** the system rejects the mutation and leaves the prior effective executable revision unchanged

### Requirement: Atomic lifecycle and failure containment
Only tools from the winning effective Skill revision SHALL be available. Registration updates SHALL be atomic; invocations already started SHALL use their immutable revision snapshot while later invocations use the new registry snapshot. Repeated validation or runtime failures SHALL quarantine only the affected tool revision.

#### Scenario: Higher-priority Skill shadows a lower-priority Skill
- **WHEN** effective Skill resolution changes the winning revision
- **THEN** the registry atomically removes the old revision's tools and considers only the new revision's tools

#### Scenario: Refresh occurs during invocation
- **WHEN** a registry refresh occurs while a tool invocation is running
- **THEN** that invocation completes or is cancelled against its original immutable snapshot without mixing revisions

#### Scenario: Tool is quarantined
- **WHEN** the configured failure threshold is reached for one tool revision
- **THEN** new invocations of that revision are rejected while unrelated tools remain available

### Requirement: Skill tool audit and usage telemetry
Discovery, trust changes, enablement changes, validation, invocation lifecycle, delegated host calls, limit breaches, quarantine, and recovery SHALL emit redacted structured events through unified log management. Successful invocation attempts SHALL update Skill usage tracking without storing raw secrets or unrestricted tool payloads.

#### Scenario: Invocation completes
- **WHEN** a Skill tool finishes successfully
- **THEN** the system records its Skill id, tool id, revision hash, duration, outcome, and bounded usage metrics

#### Scenario: Payload contains sensitive data
- **WHEN** a logged event contains sensitive input or output fields
- **THEN** the system redacts them before persistence and retains only policy-approved summaries or hashes

### Requirement: Canonical filesystem sandbox
Filesystem host calls SHALL resolve targets through the existing canonical workspace boundary, distinguish read from write authority, reject hidden or system-sensitive targets by policy, revalidate symlinks at use time, use only invocation-owned temporary directories, and enforce per-file plus aggregate byte ceilings.

#### Scenario: Tool writes outside the workspace
- **WHEN** a tool supplies an absolute path, `../` traversal, or a target whose canonical parent is outside the authorized workspace
- **THEN** the write is rejected before any outside file is created or modified

#### Scenario: Symlink escapes the workspace
- **WHEN** an authorized lexical path resolves through a symlink to a target outside the workspace
- **THEN** the operation is rejected even if the symlink changed after initial manifest validation

### Requirement: Structured process sandbox
Process host calls SHALL use an allowlisted executable identity, structured argv, canonical authorized cwd, explicit environment allowlist, bounded child count, wall timeout, process-tree cancellation, and bounded stdout/stderr. Skill input MUST NOT be concatenated into a shell command string.

#### Scenario: Argument attempts shell injection
- **WHEN** an argument contains shell metacharacters for a direct-exec operation
- **THEN** it remains one literal argument and cannot create an additional command

#### Scenario: Process exceeds its deadline
- **WHEN** an allowed command or descendant remains alive after its wall-time budget or parent cancellation
- **THEN** the runtime terminates the observed process tree and rejects late output

#### Scenario: Process emits excessive output
- **WHEN** stdout or stderr reaches the invocation output ceiling
- **THEN** the runtime safely truncates or terminates according to policy and reports the breached limit without unbounded buffering

### Requirement: Origin-bound network sandbox
Network host calls SHALL be denied by default and SHALL require an admitted HTTPS origin. DNS results, redirects, proxy routing, credential forwarding, response size, network bytes, and timeouts SHALL be checked by the managed native network boundary; every redirect target MUST be re-evaluated and loopback/private destinations MUST be denied unless an explicit higher policy allows the exact destination.

#### Scenario: Tool calls an undeclared host
- **WHEN** a tool requests a URL whose origin is absent from its manifest
- **THEN** no network connection is attempted

#### Scenario: Allowed origin redirects to a private address
- **WHEN** an admitted URL redirects to loopback, link-local, or a private-network address not explicitly allowed
- **THEN** the redirect is rejected without forwarding credentials or response content

### Requirement: Secret capability isolation
Secret values SHALL remain in the existing credential boundary and SHALL be resolved only for an exact declared capability after permission approval. They MUST NOT enter Skill prompts, frontend DTOs, inherited child environments, raw tool diagnostics, transcripts, or persisted logs.

#### Scenario: Process does not request a secret
- **WHEN** a child process is launched without an approved secret capability
- **THEN** its environment contains no managed Skill secret even if the secret is configured

#### Scenario: Tool attempts secret exfiltration
- **WHEN** a tool returns, logs, or embeds a granted secret in output
- **THEN** redaction occurs before display or persistence and the security outcome is recorded without the value

### Requirement: Aggregate invocation resource budget
Every invocation SHALL atomically account for wall time, child process count, output bytes, file bytes, network bytes, host calls, and concurrent jobs across nested delegation. WASM invocations SHALL additionally enforce fuel and linear-memory ceilings. The product MUST report platform enforcement honestly and MUST NOT claim native CPU or memory hard isolation where it is unavailable.

#### Scenario: Nested calls exhaust a shared budget
- **WHEN** nested host calls collectively reach any invocation ceiling
- **THEN** no additional resource reservation succeeds and unrelated invocations retain their own budgets
