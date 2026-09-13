## ADDED Requirements

### Requirement: Loop launch posture does not imply confinement
Managed CLI launch parameter projection SHALL retain its existing template and non-bypass rules, but a Loop launch MUST separately satisfy the frozen scope and actual execution-capability assessment. Provider name, advertised ACP support, sandbox flag, readonly label, worktree cwd or a subset of mapped hooks SHALL NOT alone prove complete mutation or Verifier read-only enforcement. Decisions MUST use stable Agent ids and concrete runtime witnesses.

#### Scenario: Launch flags are valid but internal writes are unguarded
- **WHEN** a CLI receives catalog-legal restrictive flags while its actual reachable mutation surfaces are not proven contained
- **THEN** the runtime SHALL report limited coverage and SHALL reject preventive-required execution

#### Scenario: A CLI has a proven qualifying execution boundary
- **WHEN** the actual versioned CLI/adapter/platform/tool combination provides evidence covering every enabled mutation surface
- **THEN** the runtime MAY admit strict Loop execution while continuing normal launch policy projection and per-action permission evaluation

### Requirement: Loop capability invalidation is actionable
A Loop SHALL reject stale launch capability evidence and SHALL not silently widen scope or change requested mode when provider, tool, executable, Runner or authority changes. Existing running CLI parameters MUST NOT be described as dynamically updated by template reassignment; native mediated actions SHALL honor current restrictions, and an owned execution unable to honor narrowing MUST be cancelled and paused.

#### Scenario: Provider update invalidates a capability witness
- **WHEN** the executable or adapter fingerprint differs from the assessed launch context
- **THEN** launch SHALL be rejected or the affected run paused pending fresh proof meeting the same requested mode

#### Scenario: Template narrows during an unmediated run
- **WHEN** current restrictions cannot be enforced for a still-running Loop-owned CLI through action mediation
- **THEN** the runtime SHALL cancel and pause that execution with actionable detail while retaining the original launch-parameter history
