## MODIFIED Requirements

### Requirement: Evaluation failure fails closed
The system SHALL treat an internal failure to complete an evaluation (including a storage failure) as equivalent to `Ask`, and SHALL NOT execute the requested action as a result of that failure. The same authoritative evaluator SHALL expose evaluation health alongside the three-way effect so delivery can distinguish a healthy Ask from evaluation failure. This rule SHALL apply to ACP reads as well as writes and other mediated operations. Approval MUST NOT convert a failed evaluation into execution without a new complete healthy evaluation.

#### Scenario: Storage failure during evaluation does not auto-allow
- **WHEN** the system cannot complete an evaluation due to an internal or storage failure
- **THEN** the system SHALL NOT resolve the evaluation to `Allow`
- **AND** SHALL treat the action as requiring human approval or fail-closed denial

#### Scenario: Storage failure while serving an ACP read
- **WHEN** file.read evaluation or its required audit cannot complete successfully
- **THEN** the ACP handler SHALL NOT read or return file content, even if the compatibility effect is Ask or an earlier approval exists

## ADDED Requirements

### Requirement: Loop scope admission precedes permission resolution
A host-mediated Loop-owned operation SHALL pass an independent native scope and role admission bound before normal permission resolution and again at delivery. The bound MUST NOT be widened by trusted/yolo templates, remembered grants, Once approvals, Skill trust or delegated requests. The existing stable Agent principal and MCP Ask floor SHALL remain unchanged; host-received out-of-bound MCP requests SHALL be rejected by scope admission without redefining mcp.tool evaluation. Opaque artifact-audited CLI internals SHALL be disclosed as uncovered rather than falsely described as passing this evaluator.

#### Scenario: Global grant allows a protected write
- **WHEN** a current grant or template would Allow file.write but the actual resource is protected by the run scope
- **THEN** scope admission SHALL reject the request before effects or a scope-expanding approval prompt

#### Scenario: MCP request remains independently gated
- **WHEN** an in-scope MCP request reaches the normal permission evaluator
- **THEN** mcp.tool SHALL still resolve to Ask under its existing rule, while an out-of-bound request SHALL never reach execution

#### Scenario: A request supplies another run identity
- **WHEN** a tool or model supplies an unowned run id, scope digest or session context
- **THEN** the runtime SHALL reject scope admission rather than create or substitute a new principal identity

### Requirement: ACP file reads honor all permission effects
Every ACP file.read handler, inside and outside Loops, SHALL explicitly handle Allow, Deny and Ask using the existing authoritative permission service. It MUST validate request ranges before queuing or reading. Allow SHALL read only after safe resource validation; Deny SHALL neither read content nor prompt; a healthy interactive Ask SHALL defer to durable approval without reading content first. An unanswerable Ask or unhealthy evaluation MUST fail closed.

#### Scenario: Read is allowed
- **WHEN** a healthy policy evaluation returns Allow for a valid ACP read and the authorized resource identity remains valid
- **THEN** the handler SHALL return only the requested bounded content after safe validation

#### Scenario: Read is denied
- **WHEN** file.read resolves to Deny
- **THEN** the handler SHALL return a denial without reading content or creating a pending approval

#### Scenario: Read asks for a human decision
- **WHEN** file.read resolves to healthy Ask and an interactive approval channel exists
- **THEN** the handler SHALL enqueue the pending read through existing approval machinery and SHALL return no content before valid approved delivery

#### Scenario: Read has no available approval channel
- **WHEN** file.read returns Ask in an unattended or otherwise unanswerable context
- **THEN** the handler SHALL refuse the read without accessing or returning file content
