## MODIFIED Requirements

### Requirement: Loop start readiness preflight
The Loop Center SHALL run a non-launching readiness preflight before starting a definition and SHALL report readiness for the project, base branch, Worker and Verifier eligibility, structured verification commands, path scope, and active-run constraint using the same validation semantics as native start. Readiness and enforcement coverage SHALL be distinct fields. The preflight SHALL always display requested mode, actual per-role/per-surface coverage and limitations, including when readiness passes; a ready result MUST NOT imply complete confinement.

#### Scenario: Preflight passes
- **WHEN** every factual readiness check passes
- **THEN** the UI SHALL show the factual ready result and any acknowledgementRequired state, and SHALL allow start submission only after any required current acknowledgement is obtained
- **AND** the preflight SHALL NOT create a run, worktree, or Agent session

#### Scenario: Preflight finds a blocking issue
- **WHEN** a required readiness check fails
- **THEN** the UI SHALL identify the failed check, explain the cause, and offer an actionable remediation when one exists
- **AND** it SHALL prevent start without discarding the loaded definition or recent run history

#### Scenario: Readiness changes before start commits
- **WHEN** readiness passed but the authoritative start operation detects a conflicting active run, newly unavailable dependency, changed definition/authority or stale enforcement acknowledgement
- **THEN** start SHALL remain rejected by the native runtime
- **AND** the UI SHALL refresh readiness and present the authoritative failure without creating a partial run

## ADDED Requirements

### Requirement: Visible enforcement assessment
Loop configuration, preflight and run inspection SHALL distinguish preventive-required from artifact-audited and show actual coverage with actionable limitations. Artifact-audited start/resume/continue SHALL require a fresh operation-bound explicit human acknowledgement of the exact uncovered surfaces, scope and cooperative-CLI/trusted-control-plane assumption. Readiness SHALL expose acknowledgementRequired without depending on an existing receipt. Run views MUST display frozen run evidence rather than infer historical capability from current Agent settings. Web/mock assessments MUST be visibly simulated.

#### Scenario: Readiness passes in artifact-audited mode
- **WHEN** all audit-mode start conditions are satisfied
- **THEN** the UI SHALL keep the unmediated limitations and audit-only meaning visible rather than hide them behind a passed check

#### Scenario: Strict mode cannot be enforced
- **WHEN** a selected runtime combination lacks complete coverage
- **THEN** the UI SHALL block start and identify a supported remediation without silently selecting audit mode

#### Scenario: Inspect a historical or simulated run
- **WHEN** a run has historical missing evidence or simulated Web evidence
- **THEN** the UI SHALL label that fact explicitly and SHALL NOT present current desktop enforcement as a property of that run

### Requirement: Scope configuration survives management actions
The four-step Loop editor SHALL explain literal mutation paths, protected precedence and the requested mode, preserve full versioned scope through create/edit/duplicate/enable/save-load operations, and present normalized values at review. An old unverified definition MUST require explicit valid configuration before start. Stable Agent ids and existing disabled-copy behavior SHALL remain unchanged.

#### Scenario: Duplicate or toggle a scoped definition
- **WHEN** a user duplicates a definition or changes its enabled state
- **THEN** all scope/version/mode fields SHALL survive service and adapter roundtrips, and duplication SHALL retain its existing new-id disabled-copy semantics

#### Scenario: Edit a legacy definition
- **WHEN** an older definition has no verified scope version or requested mode
- **THEN** the UI SHALL preserve its visible values, explain required remediation and SHALL not silently turn an empty allowed list into full-project access

### Requirement: Scope failures remain actionable
Loop headers, timelines and iteration evidence SHALL show scope violations, unverifiable evidence, missing bindings and capability changes with appropriate next actions. Acceptance SHALL remain pending while native asynchronous evidence checks run, and SHALL only render success after the native gate commits. New text MUST support synchronized Simplified Chinese and English, both existing styles and accessible narrow layouts.

#### Scenario: Accept needs a new complete scan
- **WHEN** a user requests acceptance while native evidence validation is running
- **THEN** the UI SHALL show a pending operation, prevent conflicting duplicate submissions and render the authoritative result without optimistic success

#### Scenario: Resume cannot clear a scope blocker
- **WHEN** a run is paused for missing or invalid scope authority
- **THEN** the UI SHALL show why resume is blocked and the valid recovery or new-run action without offering an unqualified resume
