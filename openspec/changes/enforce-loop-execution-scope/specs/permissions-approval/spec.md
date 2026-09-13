## ADDED Requirements

### Requirement: Loop approvals bind immutable scope and live ownership
A pending Loop approval SHALL bind stable principal, run/session/generation/epoch/role ownership, frozen scope digest, concrete resource identity and current authority revision. Before effect delivery, the native runtime MUST revalidate ownership, resource identity, cancellation, scope, current Deny and evaluation health. No remembered or Once approval SHALL widen the run scope or authorize a changed resource. A matching committed Once approval MAY satisfy a healthy Ask exactly once without creating an endless reapproval loop.

#### Scenario: Resource changes while approval is pending
- **WHEN** a file or ancestor is replaced, or scope/run/generation ownership changes before delivery
- **THEN** the old resolution SHALL not execute against the replacement context; a new request SHALL require fresh admission

#### Scenario: Policy is revoked before approved delivery
- **WHEN** an approved request encounters a current Deny or unhealthy evaluation before effect delivery
- **THEN** the runtime SHALL not perform the effect despite the stored Allow resolution

#### Scenario: Valid Once approval satisfies healthy Ask
- **WHEN** the immutable committed Once decision matches a still-live request and fresh evaluation is healthy Ask with all independent bounds satisfied
- **THEN** the original request SHALL be delivered at most once without persisting a grant or asking recursively

### Requirement: Deferred ACP file reads reuse durable approval delivery
ACP read approval SHALL use a typed deferred FileRead interaction and the existing native pending store, single-winner resolution, commit-before-effect delivery and acknowledgement lifecycle. Request metadata MUST NOT contain pre-read file content. Cancellation, expiry, ended generation, restart, duplicate or stale resolution and failed commit SHALL never leak read content or revive an invalid delivery.

#### Scenario: Approved read commits before accessing content
- **WHEN** a user resolves a pending FileRead to Allow and the resolution commits with valid live authority
- **THEN** the system SHALL safely read and deliver the bounded content only after commit and SHALL acknowledge through the existing lifecycle

#### Scenario: Read approval expires or restarts
- **WHEN** a FileRead request is cancelled, times out, loses its generation or encounters application restart before valid delivery
- **THEN** the request SHALL resolve fail closed with no file-content response and no automatic revival

#### Scenario: Two clients resolve the same read
- **WHEN** concurrent or repeated decisions target one pending FileRead
- **THEN** the existing single-winner and immutable-resolution rules SHALL permit at most one valid delivery and SHALL not perform a second read effect or activate an inconsistent grant

#### Scenario: Resolution persistence fails
- **WHEN** the approval resolution or required audit/grant transaction fails before commit
- **THEN** the handler SHALL return no file content and SHALL not execute a deferred read from the failed attempt
