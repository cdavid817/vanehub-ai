## Purpose

Allows OnePiece to delegate bounded analysis or isolated edits to installed Claude Code and Codex CLI processes and to review and exactly apply immutable delegated ChangeSets.

## ADDED Requirements

### Requirement: Fixed OnePiece CLI delegation tools
The system SHALL expose the fixed `delegate_cli` tool only to stable Agent id `onepiece`, with target limited initially to `claude-code` or `codex-cli`, mode limited to `analyze` or `edit`, a bounded task, an optional bounded context summary, and an optional bounded list of immutable Artifact ids. It SHALL expose `apply_delegation_changes` as a separate OnePiece-only operation and SHALL NOT dynamically register one tool per installed CLI.

#### Scenario: Valid delegation request
- **WHEN** OnePiece requests a supported target and mode with valid bounded inputs
- **THEN** the system SHALL resolve current mode-specific readiness and prepare one immutable delegation context snapshot

#### Scenario: Unsupported caller or target
- **WHEN** a non-OnePiece Agent, unsupported CLI target, unsupported mode, unknown Artifact, or oversized input is supplied
- **THEN** the system SHALL reject the request before approval or process launch

### Requirement: Mode-specific delegation readiness
Delegation readiness SHALL be separate from ordinary CLI chat readiness and SHALL be evaluated per target and mode using executable/version identity, required protocol flags, authentication status when safely observable, instruction isolation, structured output, controller sandbox, process-tree control, Artifact storage, and adapter compatibility. Availability checks SHALL not launch an interactive session or consume model quota.

#### Scenario: Analyze is ready but edit is blocked
- **WHEN** a target passes read-only execution checks but cannot prove isolated write and ChangeSet sealing
- **THEN** analyze MAY be eligible while edit SHALL remain unavailable with stable reason codes

#### Scenario: Binary changes after readiness
- **WHEN** the executable or launcher fingerprint changes
- **THEN** cached readiness and live self-test results SHALL become stale and dispatch SHALL require re-evaluation

#### Scenario: User runs a live self-test
- **WHEN** the user explicitly requests a provider canary that may contact the configured service
- **THEN** the system SHALL run it in a disposable no-remote repository, disclose that it may use provider quota, and bind the result to the binary and adapter fingerprints

### Requirement: Immutable context and approval witness
Before start approval, the system SHALL freeze the task, context summary, selected Artifact hashes, exact repository identity and clean base commit, repository-instruction snapshot, target, mode, provider/model configuration, capabilities, limits, and protocol adapter version. Overflow SHALL be rejected rather than silently truncated, and any later change SHALL invalidate the approval.

#### Scenario: Delegation context changes after approval
- **WHEN** any approval-bound context field or hash changes before launch
- **THEN** the system SHALL mark the approval stale and SHALL not run the CLI

#### Scenario: Parent transcript contains additional context
- **WHEN** OnePiece has access to a larger chat transcript, memories, or hidden reasoning
- **THEN** the system SHALL not copy those inputs automatically into the delegation context

### Requirement: Independent no-remote Git execution environment
Every delegation SHALL run in a newly owned temporary clone detached at the captured clean commit, with an independent Git object store and no configured remote. Artifact inputs SHALL be materialized read-only outside the Git worktree, controller metadata SHALL be inaccessible to the child, and the child SHALL not run in or write to the user's target worktree.

#### Scenario: Prepare an analyze delegation
- **WHEN** preparation succeeds
- **THEN** the clone SHALL be read-only to the child and any observed workspace mutation SHALL fail the attempt

#### Scenario: Prepare an edit delegation
- **WHEN** preparation succeeds
- **THEN** only the isolated clone's admitted worktree paths SHALL be writable, while inputs, controller data, credentials, target worktree, and external paths remain unavailable

#### Scenario: Child inspects Git remotes
- **WHEN** the delegated CLI queries repository remotes
- **THEN** it SHALL find no configured remote capable of fetch or push

### Requirement: CLI-owned authentication and minimal environment
The controller SHALL allow each target CLI to use its existing CLI-owned authentication mechanism without copying raw OAuth tokens or injecting API keys into prompts, arguments, logs, SQLite, Artifacts, or child-visible general environment. The child SHALL receive a minimal allowlisted environment; provider control-plane connectivity SHALL not grant network access to child commands or tools. In V1, Claude Code SHALL receive no Bash or command-execution tool, and Codex delegation on Windows SHALL remain unavailable until an independent provider-versus-child network isolation canary passes.

#### Scenario: Authentication is unavailable
- **WHEN** the target CLI cannot authenticate through its owned mechanism
- **THEN** the attempt SHALL fail with a safe authentication category and SHALL not request or extract raw credentials

#### Scenario: Child command attempts network access
- **WHEN** an action inside the delegated workspace attempts network access under the default child policy
- **THEN** it SHALL remain denied independently of the CLI's provider connection

#### Scenario: Claude requests command execution in V1
- **WHEN** a Claude Code delegation attempts to invoke Bash or another command-execution tool
- **THEN** the invocation allowlist SHALL exclude that tool before the request reaches the delegated process

#### Scenario: Codex is probed on Windows before network isolation is proven
- **WHEN** Codex delegation readiness runs on Windows without a passing independent network-isolation canary
- **THEN** analyze and edit SHALL remain blocked with a stable isolation reason and SHALL NOT launch Codex

### Requirement: Explicit prompt authority and untrusted inputs
Controller-owned safety, mode, limits, result-schema, and no-external-effect instructions SHALL use the target's supported high-priority instruction mechanism. The user task SHALL be delivered as the instruction payload, while context summaries, repository content, and Artifact contents SHALL be clearly delimited as lower-authority or untrusted data and SHALL never expand runtime permission.

#### Scenario: Artifact contains instructions
- **WHEN** an input Artifact directs the CLI to ignore policy or access another resource
- **THEN** runtime enforcement SHALL continue to deny unauthorized actions and the Artifact SHALL remain data rather than an approval source

#### Scenario: Repository guidance is loaded
- **WHEN** target-specific project guidance is admitted for the captured commit
- **THEN** the system SHALL freeze its relative paths and hashes, exclude unsafe ambient project extensions where required, and treat guidance as behavioral context rather than permission

### Requirement: Target-specific non-interactive invocation
Claude Code delegations SHALL use a fresh non-persistent print-mode session with stream JSON, explicit structured-output schema, strict tools/MCP configuration, declared turn and cost limits, and no browser integration. Codex delegations SHALL use ephemeral `codex exec` JSONL mode, explicit output schema and private final-output capture, the effective read-only or workspace-write sandbox for the requested mode, and no bypass/YOLO option.

#### Scenario: Claude target launches
- **WHEN** a Claude delegation passes approval and readiness
- **THEN** the controller SHALL construct only the reviewed managed invocation and SHALL reject user or profile arguments that conflict with owned delegation flags

#### Scenario: Codex target launches
- **WHEN** a Codex delegation passes approval and readiness
- **THEN** the controller SHALL construct only the reviewed ephemeral exec invocation and SHALL keep the final-output file inside the private attempt directory

### Requirement: Stateful strict protocol adapters
The system SHALL parse Claude and Codex delegation output through separate stateful adapters that normalize initialization, progress, action lifecycle, usage, retry, final candidate, provider error, and unknown-event observations. Unknown fields and valid unknown events SHALL be tolerated without inventing semantics, while malformed JSON stdout, missing or duplicate terminals, exit/terminal mismatches, and invalid structured output SHALL fail explicitly.

#### Scenario: Unknown valid event arrives
- **WHEN** a JSONL event type is not recognized
- **THEN** the adapter SHALL retain only bounded type/hash/size diagnostics, continue safely, and still require the known success terminal contract

#### Scenario: Non-JSON stdout arrives
- **WHEN** a target launched in JSON output mode emits non-JSON stdout beyond admitted encoding markers
- **THEN** the adapter SHALL fail with `protocol_malformed_stdout` rather than displaying it as an ordinary success token

#### Scenario: Provider claims success but exits non-zero
- **WHEN** a success terminal conflicts with the process exit status
- **THEN** the attempt SHALL fail with an exit-mismatch category

### Requirement: Bounded attempt lifecycle and cancellation
The system SHALL persist a logical delegation and append-only attempts with monotonic states covering preparation, approvals, queueing, running, sealing, cleanup, stopping, and terminal outcomes. Global and per-session concurrency, queue, duration, turn, cost where available, output, event, and disk limits SHALL be enforced, and no model attempt SHALL retry automatically.

#### Scenario: Parent generation is cancelled
- **WHEN** OnePiece's owning generation is cancelled
- **THEN** the active delegation, provider stream, child actions, approval waits, and complete owned process tree SHALL be cancelled and cleaned up

#### Scenario: Application restarts during an attempt
- **WHEN** startup finds a non-terminal attempt
- **THEN** the system SHALL mark it interrupted, stale pending approvals, reap or report remaining owned resources, and SHALL not replay provider or child actions

#### Scenario: Limit is exceeded
- **WHEN** an attempt reaches any hard resource or protocol limit
- **THEN** it SHALL terminate with the corresponding explicit status and SHALL not create an applyable ChangeSet

### Requirement: Delegation results use host-verified evidence
The final result SHALL combine the provider's validated structured report with controller-observed actions, process exit and usage, policy outcomes, independently computed Git state, and Artifact sealing. Provider statements about changed files, test success, or safety SHALL remain provider-reported claims and SHALL not replace host verification.

#### Scenario: Analyze completes without mutation
- **WHEN** the provider succeeds, structured output validates, the process exits successfully, and the isolated worktree is unchanged
- **THEN** the system SHALL seal a bounded analysis result and complete the attempt

#### Scenario: Edit completes with changes
- **WHEN** all success predicates pass and the controller independently computes an admitted complete diff
- **THEN** the system SHALL seal an immutable `DelegationChangeSetV1` Artifact with exact base, file manifest, full diff, hashes, evidence, and limitations

#### Scenario: Cleanup or sealing fails
- **WHEN** result sealing, integrity verification, or required cleanup cannot be proven
- **THEN** the attempt SHALL not be successful and SHALL not expose an applyable ChangeSet

### Requirement: Reviewable complete ChangeSets
The frontend SHALL present a ChangeSet summary, risk classification, exact base and hashes, complete per-file diff, binary metadata, provider-reported versus host-verified evidence, limitations, and applyability through the shared service boundary. A truncated or incomplete review SHALL not permit application, and V1 SHALL not support editing or partially selecting files or hunks.

#### Scenario: User reviews a complete ChangeSet
- **WHEN** all text and binary change evidence is retrievable within review limits
- **THEN** the UI SHALL permit an explicit acknowledgement bound to the exact Artifact and diff hash

#### Scenario: Review content is incomplete
- **WHEN** any required manifest, patch, file evidence, or integrity check is unavailable or irrecoverably truncated
- **THEN** application SHALL remain disabled and the Artifact MAY be offered only for manual export according to policy

### Requirement: Exact once-only ChangeSet application
Applying delegated changes SHALL require a specialized once-only unified approval bound to the Artifact id/hash, diff hash, target workspace identity, exact HEAD, and clean-state witness. The system SHALL require the same repository and base commit with a clean worktree and index, SHALL acquire an exclusive mutation lease, and SHALL never stash, merge, rebase, cherry-pick, commit, push, resolve conflicts, or apply only part of the ChangeSet.

#### Scenario: Exact clean target is approved
- **WHEN** the approval witness remains current and all preflight checks pass
- **THEN** the system SHALL apply the complete ChangeSet, verify the resulting target diff hash, leave changes uncommitted and unstaged, and persist the apply result

#### Scenario: Target changed after review
- **WHEN** HEAD, worktree, index, Git operation state, workspace identity, Artifact, or diff witness differs
- **THEN** the system SHALL return stale or blocked without modifying or cleaning the target

#### Scenario: Caller requests partial application
- **WHEN** a request selects only files or hunks from the immutable ChangeSet
- **THEN** V1 SHALL reject it rather than invalidating the original evidence silently

### Requirement: Atomic apply rollback and recovery
Before target mutation, the system SHALL create a bounded rollback capsule for every touched path and metadata item. It SHALL verify applicability, apply the complete change, and verify the actual diff. Any failure SHALL trigger rollback and verification; inability to prove rollback SHALL enter a durable recovery-required state that blocks further automatic workspace mutation.

#### Scenario: Apply fails and rollback succeeds
- **WHEN** application or post-apply verification fails but the exact pre-apply state is restored and verified
- **THEN** the apply attempt SHALL finish as failed-and-rolled-back and the ChangeSet SHALL remain unapplied

#### Scenario: Rollback cannot be proven
- **WHEN** restoration or its verification fails
- **THEN** the system SHALL preserve the recovery capsule, identify bounded affected paths, mark recovery required, and SHALL not retry automatically

### Requirement: Compatibility testing and local circuit breaking
The system SHALL maintain delegation compatibility per target, mode, executable fingerprint, adapter version, OS, and policy revision using passive no-cost probes, offline real-capture fixtures, fake-CLI fault injection, and optional explicit live canaries. Repeated protocol, sandbox, cleanup, or process-tree integrity failures for the same fingerprint SHALL open a local circuit without counting ordinary task or model-quality failures.

#### Scenario: Newer unverified CLI version is found
- **WHEN** passive capability probes pass but the version is newer than the tested range
- **THEN** analyze MAY be marked degraded while edit remains blocked until a compatible live canary or updated compatibility policy exists

#### Scenario: Circuit opens
- **WHEN** the configured threshold of integrity-class failures is reached for one binary fingerprint
- **THEN** the system SHALL block or degrade affected modes, expose stable reasons, and SHALL not retry delegations automatically
