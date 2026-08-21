## ADDED Requirements

### Requirement: Authorization rules use a structured versioned model

The system SHALL represent each AuthorizationRule with stable id, source provenance, principal matcher, operation kind, operation-specific matcher, effect, risk, allowed approval scopes, auto-approve flag, priority, optional expiry, enabled state, and schema version. Unsupported fields or operation/matcher combinations SHALL fail validation.

#### Scenario: Rule uses a file matcher for an MCP operation

* WHEN a rule declares `operation: mcp_tool` but contains an unsupported filesystem-only matcher field
* THEN preview/save rejects the rule with a field-specific validation error

#### Scenario: Rule expires

* WHEN the evaluation time is at or after an enabled rule's `expires_at`
* THEN the rule no longer participates and the decision trace records it as expired when diagnostics request excluded-rule detail

### Requirement: Operation requests are normalized before matching

The rule engine SHALL normalize permission requests into versioned operation types for shell command, file read, file write, code modification, Git operation, network request, MCP tool, extension tool, and connector operation. Normalization SHALL retain principal, workspace/project, resource, structured arguments, provenance, and risk inputs while bounding untrusted text.

#### Scenario: Shell request contains equivalent spacing

* WHEN semantically equivalent shell requests differ only in normalized spacing or argument representation supported by the classifier
* THEN matching uses the same canonical operation fields rather than raw UI text alone

#### Scenario: Request cannot be safely normalized

* WHEN a security-sensitive request cannot be classified into a supported operation model
* THEN the engine returns the immutable fallback floor, which is never less restrictive than Ask

### Requirement: Rule patterns are safe and bounded

Rules MAY use operation-specific exact values, sets, globs, and Rust-compatible regular expressions. Pattern length, input length, compiled complexity/resource use, and evaluation duration SHALL be bounded. The engine SHALL NOT execute user code, PCRE extensions, shell expansion, or network lookups during matching.

#### Scenario: Pattern exceeds limits

* WHEN a rule pattern exceeds an application ceiling or cannot compile within supported semantics
* THEN the rule set is rejected before publication and the prior known-good generation remains active

#### Scenario: Untrusted input is very large

* WHEN an operation contains an over-limit command, URL, path, or argument payload
* THEN matching uses bounded normalized data and the request follows a conservative safety decision

### Requirement: Rule sources and provenance are explicit

The system SHALL support immutable safety-floor rules, built-in default rules, global/user rules, project `.vanehub/authorization.yaml` rules, extension-contributed rules, and explicitly created session rules. Every active rule and decision trace SHALL identify source type, source identity, source version/hash where applicable, and effective priority.

#### Scenario: Extension is disabled

* WHEN an extension contributing rules is disabled or rolled back
* THEN its rule source is atomically removed or replaced for new decisions while audit records retain prior provenance

#### Scenario: Project rule file is absent

* WHEN a project has no authorization file
* THEN evaluation continues with the other sources and does not create or mutate a project file implicitly

### Requirement: Rule compilation publishes immutable generations

The system SHALL parse, normalize, validate, conflict-check, and compile the complete applicable rule source set into an immutable `CompiledPolicySet` generation. A new generation SHALL become current only after all required sources compile successfully.

#### Scenario: One project rule is malformed

* WHEN a project-file change contains one malformed required rule
* THEN no partial subset replaces the current generation and diagnostics identify the failing source/rule/field

#### Scenario: Full compilation succeeds

* WHEN all sources validate and compile
* THEN the system atomically publishes one new rule-set generation and subsequent evaluations identify that generation

### Requirement: Project rule reload retains last-known-good policy

The system SHALL watch or explicitly reload `.vanehub/authorization.yaml` with debounce, canonical project-root resolution, symlink-swap protection, partial-write tolerance, and last-known-good fallback. A failed reload SHALL NOT erase or weaken the current active project rules.

#### Scenario: Editor saves through a temporary partial file

* WHEN the watched file is temporarily incomplete during an atomic-save sequence
* THEN the loader waits for debounce/stability and retains the prior generation until a complete valid file is available

#### Scenario: Project file becomes a symlink outside the project

* WHEN canonical resolution shows the configured project rule path escapes the project boundary
* THEN reload is rejected and the prior known-good policy remains active

### Requirement: Immutable safety floors cannot be weakened

The effective decision sequence SHALL apply immutable safety floors before mutable rules, templates, Hooks, and grants. No extension, user/project rule, mode preset, Hook, remembered grant, or approval scope SHALL turn a floor Deny into Ask/Allow or a floor Ask into Allow.

#### Scenario: User Allow conflicts with critical floor

* WHEN a user rule allows an operation that the immutable floor requires explicit Ask or Deny
* THEN the floor remains effective and the trace explains the suppressed Allow

#### Scenario: Remembered grant conflicts with floor

* WHEN a remembered grant would otherwise cover an operation but the current floor requires a fresh Once approval or Deny
* THEN the grant does not bypass the floor

### Requirement: Deny dominates Ask and Ask dominates Allow

For matching mutable rules, the effective rule outcome SHALL be Deny when any applicable Deny exists; otherwise Ask when any applicable Ask exists; otherwise Allow when at least one applicable Allow exists; otherwise NoMatch. Priority and specificity SHALL order diagnostics and same-effect selection but SHALL NOT let a lower-severity effect override Deny/Ask.

#### Scenario: High-priority Allow and lower-priority Deny match

* WHEN both rules apply
* THEN the rule outcome is Deny regardless of numeric priority

#### Scenario: Allow and Ask match

* WHEN both rules apply and no Deny applies
* THEN the rule outcome is Ask

### Requirement: Existing policy templates remain fallback posture

When the compiled mutable rule outcome is NoMatch and no safety floor decides, the system SHALL use the existing assigned policy template/PDP behavior. Rules SHALL augment operation-specific decisions rather than silently rewriting template assignments.

#### Scenario: No rule matches a file read

* WHEN an Agent's current policy template permits the read and no floor or rule requires a stronger result
* THEN the existing PDP result remains effective

#### Scenario: Rule requires Ask over Trusted template

* WHEN a matching rule returns Ask while the assigned template would Allow
* THEN the operation requires Ask

### Requirement: Hooks may only preserve or strengthen rule decisions

Permission/risk Hooks SHALL receive the normalized request, risk, safety-floor result, compiled rule trace, and template result. Their accepted decision SHALL only preserve or strengthen the pending result before grant lookup and user approval.

#### Scenario: Hook escalates Allow to Ask

* WHEN the current result is Allow and a matching Hook returns Ask
* THEN the effective pending result becomes Ask

#### Scenario: Hook attempts to weaken Ask

* WHEN the current result is Ask and a Hook returns Continue or an allow-like decision
* THEN the pending result remains Ask and the attempted weakening is audited

### Requirement: Extension-contributed rules cannot self-authorize

Rules from downloaded external extensions SHALL be limited to Ask or Deny. Allow rules MAY originate only from reviewed built-in Trusted extensions and SHALL remain subject to immutable floors and install-time contribution review.

#### Scenario: External extension declares Allow

* WHEN a downloaded `.vhext` contains an authorization rule with effect Allow
* THEN package/contribution validation rejects that rule or marks the contribution ineligible without activating a weaker policy

#### Scenario: Built-in connector adapter contributes read-only Allow

* WHEN a reviewed built-in Trusted adapter contributes an allowed low-risk read operation
* THEN it may compile only if it does not conflict with a floor and its source is visible in rule inspection

### Requirement: Approval scope is constrained by effective rules

A rule SHALL declare which approval scopes are eligible when its effective outcome is Ask. The approval broker SHALL offer only the intersection of rule-allowed scopes, safety-floor scopes, operation/risk constraints, and current application policy.

#### Scenario: Critical rule allows Once only

* WHEN the operation matches a Critical Ask rule whose allowed scopes are `[once]`
* THEN Session, Project, and Global approval choices are not offered and existing broader grants do not satisfy it

#### Scenario: Multiple Ask rules constrain scopes

* WHEN multiple Ask rules match
* THEN eligible scopes are the safe intersection; an empty intersection uses Once or Deny according to the immutable floor

### Requirement: Rule simulation is non-executing and explanatory

The system SHALL provide a simulation API that accepts a synthetic principal and operation and returns normalization, risk, safety floors, matching/excluded rules, source/priority/specificity, mutable outcome, template fallback, Hook strengthening policy, grant eligibility, approval scopes, rule-set generation, and final simulated decision. It SHALL NOT execute the target, call mutating Hooks, create grants, or persist approvals.

#### Scenario: User simulates force push

* WHEN a synthetic `git push --force` operation is simulated
* THEN the response identifies matching critical/floor rules and the approval scopes without invoking Git

#### Scenario: Simulation uses stale project generation

* WHEN the user requests simulation against a no-longer-current generation id
* THEN the service returns a stale-generation diagnostic or explicitly labels the historical generation rather than mixing generations

### Requirement: Rule changes use preview and auditable operations

Create, update, enable, disable, delete, and project reload operations SHALL validate and preview effective impact before publication where they may change authority. Immutable rules SHALL be read-only. Every successful change SHALL record source, actor, prior/new generation, affected rule ids, authority diff, and operation id.

#### Scenario: User edits a global Allow rule to match more resources

* WHEN preview detects an authority expansion
* THEN the UI must show the expansion before confirmation and publication

#### Scenario: User tries to edit an extension rule

* WHEN an extension-owned immutable rule is edited directly
* THEN the system rejects the mutation and offers extension disable or user-scope override workflows only where safe
