# remote-terminal-runtime Specification

## Purpose
TBD - created by archiving change add-remote-terminal-management. Update Purpose after archive.
## Requirements
### Requirement: Authenticated remote Terminal
The desktop runtime SHALL open an SSH-backed PTY channel for a remote session only after resolving a valid SSH profile binding, verifying the server host key, and authenticating with native-owned credentials.

#### Scenario: Open bound remote Terminal
- **WHEN** a remote session has a current SSH profile binding and trusted host key
- **THEN** the runtime SHALL authenticate and open a PTY channel in the session remote path
- **AND** React SHALL receive the connected state through the existing service boundary

#### Scenario: Reject unbound remote Terminal
- **WHEN** a remote session has no SSH profile binding
- **THEN** the system SHALL keep its remote workspace snapshot readable and require an explicit profile bind before opening Terminal

#### Scenario: Reject stale binding
- **WHEN** the bound SSH profile revision or endpoint no longer matches the session binding and snapshot
- **THEN** the runtime SHALL block connection and require explicit rebind

### Requirement: SSH host identity verification
The desktop runtime MUST verify SSH server host identity before sending authentication credentials.

#### Scenario: Confirm first-seen host key
- **WHEN** a server presents an unknown host key
- **THEN** the service SHALL expose a bounded endpoint, algorithm, and fingerprint challenge
- **AND** it SHALL persist trust only after explicit user confirmation

#### Scenario: Reject changed host key
- **WHEN** a trusted endpoint presents a different host key
- **THEN** the runtime SHALL block authentication and identify the key change without automatically replacing trust

### Requirement: Reusable SSH connection pool
The desktop runtime SHALL reuse one authenticated SSH transport for compatible concurrent Terminal and quick-command channels.

#### Scenario: Reuse matching transport
- **WHEN** multiple operations use the same SSH connection id and profile revision while its transport is healthy
- **THEN** the pool SHALL establish at most one authenticated transport and open independent channels on it

#### Scenario: Do not share incompatible credentials
- **WHEN** operations use different profile ids or revisions even if host, port, and user match
- **THEN** the pool SHALL NOT reuse the same authenticated transport entry

#### Scenario: Drain edited profile
- **WHEN** an SSH profile revision changes or the profile is deleted
- **THEN** the pool SHALL reject new leases from the old entry and close it after active leases end or the drain timeout expires

#### Scenario: Evict idle connection
- **WHEN** a healthy pooled transport has no leases beyond the idle limit or the pool exceeds its capacity
- **THEN** the runtime SHALL close it without terminating channels owned by another pool entry

### Requirement: Independent remote channel lifecycle
Each remote Terminal SHALL own an independent PTY channel even when its SSH transport is shared.

#### Scenario: Resize remote PTY
- **WHEN** the visible remote Terminal dimensions change
- **THEN** the runtime SHALL resize only the matching PTY channel

#### Scenario: Disconnect one Terminal
- **WHEN** the user disconnects one remote Terminal
- **THEN** the runtime SHALL close that channel without closing a shared healthy transport still leased by other operations

#### Scenario: Shared transport fails
- **WHEN** a pooled transport becomes unavailable
- **THEN** every dependent channel SHALL transition to failed with a concise error
- **AND** the system SHALL NOT claim that their interactive remote processes were restored

### Requirement: Simulated Web remote Terminal
The Web/mock runtime SHALL provide deterministic remote Terminal semantics without opening a network connection or storing real credentials.

#### Scenario: Open Web remote Terminal
- **WHEN** a Web user opens a bound mock remote session
- **THEN** the adapter SHALL return a clearly labelled simulated channel and deterministic output

#### Scenario: Web host trust
- **WHEN** Web mode exercises a host-trust flow
- **THEN** it SHALL label the result simulated and SHALL NOT claim native SSH verification

