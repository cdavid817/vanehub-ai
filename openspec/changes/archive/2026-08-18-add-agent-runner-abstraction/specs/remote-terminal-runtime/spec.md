## ADDED Requirements

### Requirement: Pooled SSH transport supports Agent Runner channels
The existing authenticated SSH pool SHALL publish the bounded channel operations required by SSH Agent Runs while preserving profile revision, host trust, credential, capacity, lease, keepalive, drain, and shutdown invariants. Agent Runner channels MUST remain independent from Terminal and quick-command channels sharing the transport.

#### Scenario: Reuse a transport for Terminal and Agent execution
- **WHEN** a remote Terminal and SSH Agent Run use the same current compatible profile revision
- **THEN** both lease the same healthy authenticated transport and own independent channels

#### Scenario: Cancel one SSH Agent Run
- **WHEN** the user cancels one Run on a shared transport
- **THEN** only its remote process/channel is terminated and unrelated Terminal or Agent channels remain usable

### Requirement: SSH Agent disconnect and reconnect are bounded
An SSH Agent Run SHALL detect transport or channel loss, stop consuming stale events, and attempt reconnect only when the Runner declares recovery support, policy budget remains, and profile/host/credential/permission authority is current. Reconnect MUST NOT replay provider input or destructive work.

#### Scenario: Network loss is transient
- **WHEN** a recoverable remote reference survives a transport drop and authority remains current
- **THEN** bounded reconnect inspection either resumes event observation or records a safe terminal/attention outcome

#### Scenario: Network loss is not recoverable
- **WHEN** no verified remote reference exists or reconnect budget is exhausted
- **THEN** the Run stops reporting running and cleanup releases its channel lease

