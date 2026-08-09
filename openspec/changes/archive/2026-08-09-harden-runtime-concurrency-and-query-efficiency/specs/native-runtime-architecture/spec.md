## ADDED Requirements

### Requirement: Child process reaping must not hold the child lock across the blocking wait

A native monitor or stop path that reaps a managed child process held behind an `Arc<Mutex<…>>` SHALL NOT call the blocking `wait()` while holding that lock. It SHALL poll `try_wait()` with short lock holds so a concurrent cancellation path can acquire the lock to `kill()` the child. A drain that joins its worker thread after the child has been shut down SHALL bound that join with a deadline and abandon the worker on timeout rather than blocking indefinitely.

#### Scenario: CLI closes stdout but stays alive and the user cancels

- **WHEN** a managed CLI monitor reads stdout to EOF while the child process is still alive, and a stop request arrives to cancel it
- **THEN** the stop request SHALL acquire the child lock, kill the child, and complete rather than deadlock against the monitor's blocking wait
- **AND** the monitor SHALL eventually reap the child once `try_wait()` reports it exited

#### Scenario: Grandchild holds the stderr pipe after the child is shut down

- **WHEN** a managed MCP stdio relay shuts its child down and a grandchild that inherited the stderr pipe keeps the drain reader pending
- **THEN** the drain finish SHALL time out and abandon the worker rather than wedge the relay shutdown
- **AND** the abandoned worker SHALL terminate on its own once the pipe's last writer closes

#### Scenario: A process is monitored twice

- **WHEN** `monitor_generation` is called twice for the same process id
- **THEN** the second call SHALL be rejected rather than spawning a duplicate generation thread, mirroring the guard the CLI adapter already has

### Requirement: Migration application is transactional with startup density verification

The native runtime SHALL apply each SQLite migration inside a single transaction that records the version row on the same commit as the schema change, so a mid-migration failure rolls back both. After applying migrations, the runtime SHALL verify the recorded `schema_migrations` history is dense and within the expected version range, surfacing a diverged history as an explicit startup error.

#### Scenario: A migration fails partway through

- **WHEN** a migration's DDL or DML fails before completion
- **THEN** the transaction SHALL roll back so no partial schema change is committed without its version row

#### Scenario: A migration version was silently skipped

- **WHEN** the recorded `schema_migrations` history is not dense or exceeds the highest version the binary expects
- **THEN** startup SHALL fail with an explicit, diagnosable error rather than a later opaque "no such table" crash

### Requirement: Command errors redact forwarded lower-layer messages at the boundary

A `From<…> for CommandError` implementation that forwards a lower-layer message verbatim (e.g. an internal/infrastructure/repository variant whose payload may carry a filesystem path or provider diagnostic) SHALL redact that payload at the conversion boundary. Structured category-level error codes that are safe and matched by the frontend SHALL pass through unchanged. Command families that previously returned `Result<T, String>` via `to_string()` SHALL route through `CommandError` so the same redaction applies.

#### Scenario: A path-bearing CLI config error reaches the frontend

- **WHEN** a `cli_config` parse or filesystem error carries an absolute path and is returned to the frontend
- **THEN** the command SHALL surface a fixed category-level message and SHALL NOT forward the path

#### Scenario: A structured error code is returned

- **WHEN** a command error is a safe category-level code (e.g. `connector-credentials-required`)
- **THEN** the code SHALL be returned unchanged, not mangled by heuristic redaction
