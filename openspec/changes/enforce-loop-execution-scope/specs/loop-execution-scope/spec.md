## ADDED Requirements

### Requirement: Canonical workspace mutation scope
The native runtime SHALL interpret a versioned Loop scope as a nonempty set of literal worktree-relative allowed paths minus protected paths and reserved control resources. Matching MUST use complete path components and actual filesystem identity semantics, not string prefixes. An explicit `.` MAY select the whole worktree; an empty allowed set MUST NOT mean unrestricted access. Scope constrains mutation and SHALL NOT independently grant or deny file-read permission. Scope input MUST be normalized and validated consistently, with unsupported syntax and capacity overflow rejected rather than truncated.

#### Scenario: Match components and protected descendants
- **WHEN** a mediated request or preventive-required execution uses allowed paths `src` and protected paths `src/generated`
- **THEN** an otherwise authorized write to `src/app.ts` SHALL be eligible, while writes to `src-old/app.ts` and `src/generated/a.ts` SHALL be rejected or prevented before effects

#### Scenario: Reject ambiguous or escaping configuration
- **WHEN** configuration contains parent traversal, an absolute/drive/UNC/device/ADS path, control characters, glob syntax, an empty effective allowed scope, or an unsupported path representation
- **THEN** validation SHALL reject the configuration without widening or reinterpreting it as full-workspace access

#### Scenario: Preserve complete scope independently of prompt size
- **WHEN** a valid scope exceeds the prompt display budget, or an input exceeds the versioned configuration capacity
- **THEN** the runtime SHALL retain every valid scope entry independently of prompt summaries, and SHALL reject capacity overflow explicitly instead of silently dropping entries

#### Scenario: Reserve host control resources
- **WHEN** an allowed scope includes `.` and a mediated request or preventive-required execution attempts mutation of Git control metadata, the Git common directory, or host policy/evidence storage
- **THEN** scope admission or proven containment SHALL reject the mutation; host-owned worktree administration SHALL use a separate narrowly bounded internal capability unavailable to Agent calls

#### Scenario: Keep read policy independent
- **WHEN** a request reads a protected mutation path
- **THEN** the runtime SHALL evaluate file.read policy and trusted ownership without treating the mutation protection alone as permission to read or as a read prohibition

### Requirement: Immutable native scope binding
Every Loop role launch, host-received tool call and verification request SHALL derive its scope reference from authoritative native run ownership. The binding MUST include supported schema, immutable relative scope and requested mode, definition revision, role/session ownership, scope digest, actual worktree identity, baseline reference and executor witnesses before Agent or verification side effects begin. Opaque audit-mode internals SHALL be attributed to the owned execution tree and its frozen limitations without claiming per-action mediation. A frontend or model-supplied path, digest or run id MUST NOT establish authority. The frozen scope is an upper bound; current policy revocation SHALL remain effective at native admission and owned execution control.

#### Scenario: Bind after worktree preparation before first effect
- **WHEN** a queued run has prepared its worktree
- **THEN** the runtime SHALL durably bind its real root identity and baseline before creating an executing role session or verification process; binding failure SHALL produce no Agent or verification side effect

#### Scenario: Reject missing or foreign binding
- **WHEN** a host-received child request has missing, unknown-version, corrupted, stale or foreign run/session scope context
- **THEN** the runtime SHALL reject it before execution rather than infer authority from cwd, display name or a caller-provided path

#### Scenario: Prevent later scope expansion
- **WHEN** a saved definition or remembered grant is broadened after the run starts
- **THEN** the existing run SHALL retain its original scope upper bound while independently honoring current narrower policy

### Requirement: Resource checks cover every affected target
Before a mediated mutation, the runtime SHALL resolve and check every affected real target against the bound worktree, allowed scope, protected scope and role constraints. This SHALL include both rename endpoints, the descendants affected by recursive operations, nearest existing parents for creation, and content, mode, type and link changes. A partially authorized mutation MUST NOT begin.

#### Scenario: Reject a rename with one unauthorized endpoint
- **WHEN** a rename has either source or destination outside scope or inside a protected subtree
- **THEN** the runtime SHALL reject the entire rename before modifying either endpoint

#### Scenario: Reject recursive deletion containing protected content
- **WHEN** an allowed ancestor is recursively deleted but an affected descendant is protected
- **THEN** the runtime SHALL reject the deletion before removing any descendant

#### Scenario: Check new file parents and metadata changes
- **WHEN** a request creates a missing file or changes file mode, type or link metadata
- **THEN** the runtime SHALL check its real parent and all affected resources with the same mutation boundary

### Requirement: Race-resistant resource execution
Mediated filesystem checks and effects SHALL share a race-resistant boundary using safe handle-relative, no-follow operations or a verified equivalent. Canonicalizing a string and reopening it later MUST NOT count as enforcement. The runtime MUST reject unprovable symlink/junction/reparse traversal and shared-inode writes, use actual volume case semantics, and report unsupported platform guarantees rather than weaken enforcement.

#### Scenario: Swap a parent after a path check
- **WHEN** a target ancestor is replaced with a link between authorization and delivery
- **THEN** execution SHALL remain bound to the authorized resource or fail before changing the replacement target

#### Scenario: Write through a hardlink
- **WHEN** an in-place write may mutate a shared inode with an alias outside the allowed scope
- **THEN** the runtime SHALL reject it unless it proves all affected aliases safe or performs a safe replacement that does not mutate the external alias

#### Scenario: Cannot establish platform identity semantics
- **WHEN** the adapter cannot prove case, link or safe-open behavior on the target filesystem
- **THEN** it SHALL report the required enforcement capability as unavailable and SHALL NOT substitute cross-platform lowercasing or lexical prefix checks

### Requirement: Execution capability evidence
The runtime SHALL assess every role and side-effect surface with evidence bound to stable Agent/provider identity, transport, Runner/platform, executable identity/version, adapter revision, enabled channels and containment configuration. Coverage SHALL distinguish `complete-enforcement`, `mediated-tools-only`, `artifact-validation-only`, `unsupported` and `unknown`. ACP support, worktree isolation, launch flags and mapped hooks alone MUST NOT establish complete enforcement. Witnesses SHALL be revalidated before spawn and when execution authority changes.

#### Scenario: Uncovered CLI internal tool
- **WHEN** a CLI can mutate through an internal tool or subprocess outside host mediation and no qualified containment covers it
- **THEN** the assessment SHALL disclose that surface and SHALL NOT report complete-enforcement

#### Scenario: Executable changes after readiness
- **WHEN** the selected executable, adapter or enabled tool configuration changes before spawn
- **THEN** the runtime SHALL invalidate the assessment and reject or pause execution until the frozen requested mode is satisfied by fresh evidence

#### Scenario: Policy narrows during an unmediated execution
- **WHEN** current permission narrowing cannot be applied to an active owned process through mediation
- **THEN** the runtime SHALL cancel that execution and pause with actionable authority detail rather than claim the process has received new launch parameters

### Requirement: Explicit requested enforcement mode
New Loop definitions SHALL default to `preventive-required`, which admits only complete execution-before-effect mutation coverage. `artifact-audited` SHALL require an explicit definition selection and a fresh human start acknowledgement bound to the exact definition revision, scope and known execution assessment. It MUST preserve all mediated guards, complete artifact validation and Verifier read-only enforcement. Unsupported identity, missing artifact validation, scheduled/unattended start and silent mode downgrade MUST NOT be admitted.

#### Scenario: Strict mode has one uncovered surface
- **WHEN** any reachable mutating surface lacks complete enforcement in preventive-required mode
- **THEN** authoritative start SHALL reject before queuing a run and SHALL identify the uncovered surface

#### Scenario: Human starts a known partially mediated CLI
- **WHEN** artifact-audited is explicitly selected, complete artifact validation is available, every role meets its hard constraints, and a human acknowledges the current exact limitations
- **THEN** the runtime SHALL snapshot the acknowledgement and MAY start with persistent artifact-audited labeling, without claiming prevention of unmediated effects

#### Scenario: Audit acknowledgement is absent or stale
- **WHEN** audit mode is requested by an unattended caller or with missing/stale acknowledgement
- **THEN** the runtime SHALL reject start and SHALL NOT infer acknowledgement from a prior run or readiness success

### Requirement: Closed side-effect surfaces
Execution admission SHALL account for mediated tools, raw CLI actions, interpreters, build scripts, shell, MCP, Skill tools, delegation and background descendants. An executable or argv allowlist alone MUST NOT establish a mutation boundary. Strict-mode execution SHALL disable uncovered channels or use a qualified environment constraining every reachable descendant. Host-mediated nested operations MUST inherit the intersection of the parent Loop scope and all other applicable bounds; opaque audit-mode descendants SHALL retain execution-tree attribution without a per-action enforcement claim.

#### Scenario: Allowlisted interpreter attempts external write
- **WHEN** an allowed Python, Node or build command can write outside the configured scope through its program body
- **THEN** strict mode SHALL reject the uncontained execution or prevent the external write through proven containment, regardless of the executable allowlist

#### Scenario: Nested operation drops ownership
- **WHEN** a host-received Skill, MCP, tool or subprocess request cannot establish required parent ownership and scope
- **THEN** the system SHALL reject the request before granting operational authority

#### Scenario: Opaque audit channel has no per-action mediation
- **WHEN** artifact-audited execution uses a declared opaque internal CLI channel
- **THEN** the runtime SHALL keep it attributed to its owned execution tree and persist its uncovered limitation without claiming per-action authorization; unknown ownership or unreconciled completion SHALL block phase sealing

#### Scenario: Background writer survives a role result
- **WHEN** a role reports completion while its owned descendants can still mutate artifacts
- **THEN** the runtime SHALL stop and reconcile those writers before sealing phase evidence or proceeding

### Requirement: Complete workspace artifact evidence
The native runtime SHALL capture a complete baseline and complete final-state manifests for each Worker, verification and Verifier boundary, with integrity guarantees and trust assumptions explicitly defined by the execution mode. Evidence MUST account for tracked, untracked, ignored and hidden entries, binary content digests, deletion, rename endpoints, type, permissions and link metadata without following links outside the root. Bounded UI Git output MUST NOT be an authorization input. Incomplete, unstable, cancelled, over-budget or unreadable scans SHALL be unverifiable and SHALL NOT pass. Generated artifacts remain scoped mutations; executor temporary resources MUST be narrowly declared and assessed.

#### Scenario: Ignored file violates protection
- **WHEN** an ignored or untracked file is created or changed inside a protected subtree by a role or verification command
- **THEN** complete artifact validation SHALL record a scope violation even when the bounded Git diff omits the file

#### Scenario: Scanner cannot complete
- **WHEN** any required directory or metadata cannot be inspected, a scan is truncated/cancelled, or writers prevent a stable snapshot
- **THEN** the runtime SHALL persist unverifiable evidence and SHALL block progression or acceptance

#### Scenario: Verification generates a cache outside its scope
- **WHEN** a verification process changes a worktree cache path outside its allowed scope without an explicit bounded executor-resource provision
- **THEN** the runtime SHALL treat that change as a scope violation rather than silently exempt generated or ignored content

### Requirement: Sticky violations and bounded claims
An observed scope violation SHALL remain attached to its run and SHALL prevent that run from succeeding even if later reverted. The runtime SHALL preserve worktree and redacted evidence instead of automatically deleting or resetting user artifacts. Artifact validation SHALL claim only inspected final-state properties and MUST NOT claim absence of unobserved transient or outside-root effects in artifact-audited mode.

#### Scenario: A recorded violation is later reverted
- **WHEN** a later phase restores the original content after an earlier recorded scope violation
- **THEN** the violation SHALL remain effective and the run SHALL NOT become acceptance-ready

#### Scenario: Audit mode has a clean final workspace
- **WHEN** artifact-audited execution produces a clean complete final-state manifest
- **THEN** the UI and stored evidence SHALL still identify unmediated limitations and SHALL NOT assert that no transient or outside-root write occurred

### Requirement: Sealed acceptance evidence
Acceptance SHALL bind a succeeded run to sealed artifact contents and their manifest under the declared integrity boundary, not a mutable working tree or an old pass flag. Native acceptance MUST first acquire the run/worktree write lease, reconcile owned writers, copy the actual accepted contents into host-managed immutable artifact objects, and compute and validate their manifest against the verified input/output fingerprints and current root, scope, policy and executor witnesses before committing with run revision and evidence identity compare-and-set. A lease alone MUST NOT claim exclusion of external writers. Existing acceptance entry points MUST route through this gate. Variable-duration copying/scanning SHALL use the existing asynchronous operation model.

#### Scenario: Workspace changes after awaiting acceptance
- **WHEN** the working tree or authorization evidence changes after the displayed acceptance-ready result
- **THEN** acceptance SHALL reject stale evidence and require revalidation instead of marking the modified tree succeeded

#### Scenario: Accept races with continue or another writer
- **WHEN** a concurrent operation changes run revision, write ownership or evidence identity
- **THEN** only a valid compare-and-set against reconciled sealed evidence SHALL succeed; conflicting acceptance SHALL not produce a second inconsistent outcome

#### Scenario: External process writes during artifact capture
- **WHEN** an external editor or process changes files during copying or manifest validation
- **THEN** the runtime SHALL require a stable content snapshot matching verification evidence and SHALL return scope-unverifiable if it cannot establish that consistency; SQLite revision alone SHALL NOT establish filesystem consistency

#### Scenario: Files change after a valid acceptance
- **WHEN** an external actor changes the live worktree after success was committed
- **THEN** the accepted result SHALL continue to identify the exact stored artifact contents and manifest and SHALL NOT describe the new mutable contents as already accepted

### Requirement: Evidence integrity follows the execution trust boundary
Preventive-required execution SHALL prove its execution boundary prevents Agent mutation of host policy, baseline and sealed artifacts. Artifact-audited execution SHALL explicitly assume a cooperative CLI and trusted host control plane, disclose that uncontained same-identity processes can actively tamper with host storage, and MUST NOT claim resistance to that threat merely from a host directory or digest. Detected corruption SHALL make evidence unverifiable in either mode; original baselines MUST NOT be reconstructed as invented history.

#### Scenario: Strict execution targets evidence storage
- **WHEN** a preventive-required Agent attempts to alter baseline or artifact storage
- **THEN** the proven execution boundary SHALL prevent the effect

#### Scenario: Audit mode lacks independent storage isolation
- **WHEN** a known uncontained CLI runs under the same OS identity as the host
- **THEN** confirmation and run evidence SHALL disclose the cooperative-CLI and trusted-control-plane assumption and SHALL NOT assert resistance to active same-identity tampering

#### Scenario: Evidence integrity fails
- **WHEN** stored baseline, manifest or artifact content is corrupted or inconsistent
- **THEN** the runtime SHALL record unverifiable evidence and block acceptance without fabricating a replacement original baseline

### Requirement: Audit acknowledgements are operation bound
Readiness SHALL return factual assessment and acknowledgementRequired independently of acknowledgement completion. A native admission challenge and explicit audit acknowledgement SHALL produce a stored receipt bound to action, target definition/run, expected revision, scope, capability assessment, client context and creation time. The receipt SHALL expire after five minutes, be invalidated by restart, clock rollback or context changes, and be consumed once atomically with the intended start/resume/continue transition. It MUST NOT grant tool permission. Control requests SHALL carry expected revision, an idempotency key and the receipt reference when required; legacy missing fields MUST NOT authorize audit execution.

#### Scenario: Confirm start then use the receipt for resume
- **WHEN** an audit receipt for start is supplied to resume or another run
- **THEN** the native service SHALL reject it without scheduling execution

#### Scenario: Repeated control submission uses one receipt
- **WHEN** the same committed control request is retried with the same idempotency key
- **THEN** the runtime SHALL return the existing operation rather than consume authority twice or create another execution

#### Scenario: Readiness precedes acknowledgement
- **WHEN** a user opens audit-mode preflight without an acknowledgement
- **THEN** the service SHALL return assessment and acknowledgementRequired so the UI can obtain explicit confirmation; it SHALL create no run, worktree or role session

#### Scenario: Receipt expires or its context changes
- **WHEN** five minutes elapse or scope, authority, application epoch or target revision changes
- **THEN** the receipt SHALL not authorize execution and a fresh current assessment and confirmation SHALL be required

### Requirement: Scope diagnostics preserve privacy
Scope admission, scanning, approval and recovery SHALL produce bounded redacted reason and evidence references through existing audit and unified operations/logging boundaries. Raw file contents, secrets and unbounded tool payloads MUST NOT enter those records. Scope decisions and canonical Run projection MUST agree.

#### Scenario: A permission or scope check fails
- **WHEN** a file action is denied or evidence becomes unverifiable
- **THEN** the system SHALL provide an actionable bounded reason with run/operation attribution without persisting file content or creating a feature-specific log

#### Scenario: Scope failure reaches canonical lifecycle
- **WHEN** a Loop fails or pauses because of scope evidence
- **THEN** its canonical Run SHALL reflect the same failed or blocked business outcome rather than a successful terminal projection
