## MODIFIED Requirements

### Requirement: Shell resource cleanup
The system MUST terminate managed PTY children when their owning session workspace can no longer retain them, and every termination MUST complete within a bounded, monotonic deadline rather than blocking indefinitely on the child.

#### Scenario: Switch sessions
- **WHEN** the active session changes and the old mounted tab set is reset
- **THEN** the old session Shell SHALL be killed and its frontend subscription SHALL be removed

#### Scenario: Archive or delete session
- **WHEN** a session with a managed Shell is archived or deleted
- **THEN** the native runtime SHALL kill that Shell before completing lifecycle cleanup

#### Scenario: Exit application
- **WHEN** the desktop application exits
- **THEN** the Shell manager SHALL attempt to terminate every managed child

#### Scenario: Repeated kill
- **WHEN** cleanup requests kill for an already exited or previously killed shell
- **THEN** the operation SHALL succeed idempotently without affecting another shell

#### Scenario: Termination never blocks without a bound
- **WHEN** the runtime terminates a managed shell
- **THEN** it SHALL observe the child's exit by non-blocking polling against a monotonic deadline
- **AND** it SHALL NOT call a blocking wait that can outlive that deadline

#### Scenario: A wedged shell does not stall any other shell
- **WHEN** one shell's child does not exit after a kill
- **THEN** input, resize, and cleanup for every other shell SHALL continue to be served
- **AND** the registry routing lock SHALL NOT be held while any kill or reap is in progress

#### Scenario: A second termination does not start a second reap
- **WHEN** termination is requested for a shell whose reap is already in flight
- **THEN** the second request SHALL report that a reap is in progress and return without starting another one or waiting on the first

## ADDED Requirements

### Requirement: Bounded shell termination outcomes
The desktop shell runtime SHALL report the result of terminating a managed PTY child as one of a closed set of outcomes, and SHALL NOT represent a termination that did not complete as one that did.

#### Scenario: The child had already exited
- **WHEN** a non-blocking poll shows the child has exited before any signal is sent
- **THEN** the outcome SHALL be `already_exited` and no signal SHALL be sent

#### Scenario: The child is observed to exit after the signal
- **WHEN** the child exits within the deadline after a kill is requested
- **THEN** the outcome SHALL be `reaped`

#### Scenario: The deadline is reached with the child still alive
- **WHEN** the deadline passes and no exit has been observed
- **THEN** the outcome SHALL be the stable code `reap_timed_out`
- **AND** it SHALL NOT be reported as terminated, reaped, or successful
- **AND** the runtime SHALL transfer ownership of the child to the pending-reap registry before releasing the shell, so the sole handle to a live process is never dropped
- **AND** the cleanup state SHALL be `pending`
- **AND** the runtime SHALL record redacted evidence naming the session and shell whose child was left unreaped

#### Scenario: The signal itself is refused
- **WHEN** the kill operation returns an error and a fresh poll shows the child still running
- **THEN** the outcome SHALL be `kill_failed`, distinct from a reap that timed out
- **AND** the child SHALL be transferred to the pending-reap registry, because a refused signal also leaves a live process

#### Scenario: Polling the child reports an error
- **WHEN** a non-blocking poll returns an error rather than an exit status
- **THEN** the outcome SHALL be `reap_failed`, distinct from `reap_timed_out`

### Requirement: Pending shell reaps are owned until they resolve
The desktop shell runtime SHALL retain ownership of any managed PTY child that outlives its termination attempt, SHALL reclaim it without blocking when it later exits, and SHALL report the cleanup state separately from the outcome of the attempt that failed to reap it.

#### Scenario: A later exit is reclaimed
- **WHEN** a child held as `pending` is observed to have exited during a subsequent sweep
- **THEN** its cleanup state SHALL become `reaped_later`
- **AND** the original termination outcome SHALL be left unchanged, because a reap that timed out and was later reclaimed is not the same history as one that succeeded

#### Scenario: Sweeps never block
- **WHEN** the runtime reclaims pending children
- **THEN** it SHALL use only non-blocking polling
- **AND** it SHALL NOT call a blocking wait, which would rebuild inside the recovery path the hang that recovery path exists for

#### Scenario: Shutdown names what it could not resolve
- **WHEN** the runtime shuts down with children still pending
- **THEN** their cleanup state SHALL become `unresolved_at_shutdown` with redacted evidence naming each session and shell
- **AND** the runtime SHALL NOT report those children as cleaned up

### Requirement: Shell runtime shutdown is explicit and bounded
The desktop shell runtime SHALL own its shutdown signal, its monitor threads, its active shells, and its pending reaps, and SHALL complete shutdown within a bounded, monotonic deadline shared across all shells.

#### Scenario: Shutdown ends every monitor
- **WHEN** the last handle to the shell runtime is released
- **THEN** the runtime SHALL signal shutdown, wake every exit monitor, and wait for them to exit
- **AND** no monitor SHALL remain running afterwards
- **AND** shutdown SHALL NOT be skipped on the basis of how many references some internal structure happens to have

#### Scenario: One wedged child does not consume the whole shutdown
- **WHEN** several shells are terminated during shutdown and one child does not exit
- **THEN** every other child SHALL still be signalled and reaped
- **AND** all of them SHALL share one deadline rather than one deadline each

#### Scenario: Outcomes carry no raw shell content
- **WHEN** any termination outcome is written to unified logging
- **THEN** it SHALL carry the outcome code with session and shell identifiers and SHALL NOT include raw interactive commands or raw PTY output
