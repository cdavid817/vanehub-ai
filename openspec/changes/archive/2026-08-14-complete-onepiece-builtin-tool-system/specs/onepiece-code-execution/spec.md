## Purpose

Provides OnePiece with a dedicated code-execution tool whose programs run in a disposable, resource-bounded sandbox rather than through the general shell executor.

## ADDED Requirements

### Requirement: Dedicated code-execution contract
The system SHALL expose `code_execution` as a distinct OnePiece-only tool with versioned inputs for a supported runtime, source text or approved Artifact inputs, declared arguments, and bounded result expectations. It SHALL NOT translate the request into an invocation of the ordinary workspace shell tool.

#### Scenario: Execute supported source
- **WHEN** OnePiece submits valid source for an installed supported runtime
- **THEN** the system SHALL create an independent sandbox execution and return a structured terminal result

#### Scenario: Unsupported runtime
- **WHEN** the request names a runtime or version outside the reviewed runtime catalog
- **THEN** the system SHALL reject it before creating a process

### Requirement: Disposable least-privilege sandbox
Each code execution SHALL receive a fresh owned filesystem and process boundary with no access to the user's workspace, home directory, credential stores, browser profiles, application database, other executions, or arbitrary host paths. Network SHALL be denied by default, and environment variables SHALL be a minimal controller-owned allowlist with no inherited secrets.

#### Scenario: Program reads a host path
- **WHEN** sandboxed code attempts to read outside its owned input and work directories
- **THEN** the sandbox SHALL deny access without returning host content

#### Scenario: Program opens a network connection
- **WHEN** sandboxed code attempts network access under the default policy
- **THEN** the sandbox SHALL deny the connection even if another OnePiece tool has Web access

#### Scenario: Concurrent executions
- **WHEN** two eligible executions run concurrently
- **THEN** their files, processes, environment, outputs, limits, and cancellation ownership SHALL remain isolated

### Requirement: Read-only Artifact inputs and explicit outputs
The sandbox SHALL receive only explicitly selected immutable Artifact inputs, materialized under generated read-only names with verified hashes. Output files SHALL remain private until admitted by size, count, type, path, and policy checks and then sealed as new Artifacts with lineage to the execution and inputs.

#### Scenario: Program modifies an input
- **WHEN** sandboxed code attempts to overwrite a materialized Artifact input
- **THEN** the sandbox SHALL deny the write and preserve the original Artifact hash

#### Scenario: Program produces admitted files
- **WHEN** output files satisfy all admission limits
- **THEN** the system SHALL seal them as immutable output Artifacts and return their references

#### Scenario: Program produces an unsafe path or excessive output
- **WHEN** output includes traversal, links, special files, excessive counts, or content above declared limits
- **THEN** the system SHALL reject those outputs and SHALL NOT publish them as Artifacts

### Requirement: Hard execution budgets
Code execution SHALL enforce controller-owned limits for wall time, CPU, memory, process count, output bytes, filesystem bytes, file count, and event count. Request parameters MAY lower but SHALL NOT raise platform ceilings, and reaching any hard limit SHALL terminate the complete owned process tree.

#### Scenario: Program exceeds wall time
- **WHEN** execution reaches its effective wall-time limit
- **THEN** the system SHALL terminate all owned descendants and return a timeout outcome without a successful result

#### Scenario: Program floods stdout
- **WHEN** stdout or stderr reaches its hard byte limit
- **THEN** the system SHALL stop accepting further output, terminate or fail the execution according to policy, and explicitly report truncation or limit failure

### Requirement: Explicit approval and non-persistent environment
Starting arbitrary model-supplied code SHALL require the unified high-risk permission decision for the exact runtime, source hash, input Artifact hashes, and effective limits. An approval SHALL apply once, and the sandbox SHALL be destroyed after terminal handling unless a recovery record is required.

#### Scenario: Source changes after approval
- **WHEN** the source, runtime, inputs, or effective limits differ from the approved witness
- **THEN** the system SHALL reject the stale approval and SHALL NOT start execution

#### Scenario: Execution completes
- **WHEN** result and admitted outputs are sealed
- **THEN** the system SHALL remove the disposable filesystem and verify that no owned descendant remains

### Requirement: Safe result envelope
The code-execution result SHALL contain status, exit information, bounded stdout/stderr, usage and limit metadata when available, produced Artifact references, duration, and a safe error. It SHALL NOT treat non-zero exit, timeout, cancellation, sandbox violation, cleanup failure, or invalid Artifact sealing as successful execution.

#### Scenario: Program exits non-zero
- **WHEN** the main program exits with a non-zero status
- **THEN** the tool SHALL return a failed result with bounded diagnostics and SHALL not claim completion

#### Scenario: Web/mock execution
- **WHEN** `code_execution` is requested in Web/mock mode
- **THEN** the adapter SHALL return deterministic simulation or desktop-runtime-required without running code in the browser host

