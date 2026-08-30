## ADDED Requirements

### Requirement: Claude hook Ask responses use committed immutable resolutions

A Claude Code `PreToolUse` request that resolves to Ask SHALL remain blocked until the permissions application use case has claimed the pending request, verified the current hook waiter, and committed an immutable approval resolution and audit. The loopback response SHALL carry or be correlated with the immutable resolution id, and the hook waiter SHALL apply that resolution at most once.

#### Scenario: Human Allow cannot precede persistence

- **WHEN** a user approves a Claude Code hook request and the resolution transaction has not committed
- **THEN** the loopback server SHALL NOT return Allow to the hook wrapper
- **AND** Claude Code SHALL remain blocked or receive a fail-closed typed failure rather than execute early

#### Scenario: The same resolution is delivered twice

- **WHEN** retry logic attempts to deliver the same committed resolution id to one hook waiter more than once
- **THEN** the waiter SHALL apply the first valid delivery at most once
- **AND** subsequent deliveries SHALL return an idempotent acknowledgement without releasing another tool execution

#### Scenario: Hook waiter ended before reservation

- **WHEN** the HTTP request, hook timeout, or originating generation ends before the resolution use case reserves the waiter
- **THEN** the resolution SHALL be classified stale
- **AND** no Allow response or remembered grant SHALL be produced for that ended waiter

### Requirement: Hook delivery uncertainty fails closed across restart

A committed hook resolution without an acknowledged loopback delivery SHALL NOT be replayed after application restart. Its remembered-grant intent SHALL remain inactive and a later Claude Code invocation SHALL undergo a new permission evaluation.

#### Scenario: Application restarts during hook delivery

- **WHEN** the application restarts after committing a hook approval but before recording acknowledgement
- **THEN** the old hook request SHALL remain unresolved only as durable evidence
- **AND** a new hook invocation SHALL NOT inherit execution authority from that uncertain delivery
