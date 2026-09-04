## MODIFIED Requirements

### Requirement: Shared Run service and minimal status presentation
The frontend SHALL query and control Runs through the shared Agent service interface with contract-compatible Tauri and Web/mock adapters. A reusable localized status presentation SHALL show status, elapsed time, explicit waiting reason, retry count, and only permitted cancel/resume actions using semantic visual tokens. Elapsed time for an active Run SHALL advance from its canonical creation or start timestamp against the current clock and SHALL freeze against its terminal timestamp after completion.

#### Scenario: Desktop queries a Run
- **WHEN** React requests Run status in desktop mode
- **THEN** it uses the shared service and the Tauri adapter invokes declared native commands

#### Scenario: Web simulates a Run
- **WHEN** the same surface runs in Web/mock mode
- **THEN** it receives the same state, reason, timestamp, retry, and action contract without claims of native persistence or process recovery

#### Scenario: Status renders across supported layouts
- **WHEN** the status component renders in futuristic or minimal style at desktop or narrow width
- **THEN** status and actions remain readable, keyboard accessible, non-overlapping, and distinguishable without color alone

#### Scenario: Active elapsed time advances
- **WHEN** a Run remains in a non-terminal active state while its persisted update timestamp is unchanged
- **THEN** the visible elapsed duration SHALL continue increasing from the Run's canonical timestamp

#### Scenario: Terminal elapsed time freezes
- **WHEN** a Run reaches a terminal state
- **THEN** its visible elapsed duration SHALL be calculated against its terminal update timestamp and SHALL no longer increase

#### Scenario: Managed CLI completion survives restart
- **WHEN** a managed CLI generation persists a completed, failed, or cancelled terminal message and Operation
- **THEN** the correlated canonical Run SHALL persist the matching terminal outcome before the execution is treated as finished
- **AND** a later client restart SHALL preserve that terminal Run instead of replacing it with `interrupted_restart`

### Requirement: Canonical Runs retain bounded Runner ownership
Every accepted Agent generation Run SHALL persist one immutable runner kind and stable bounded runner reference plus versioned capability, authority, recovery, and progress witnesses needed by its owner. Runner and progress metadata SHALL contain no credential, raw environment, prompt, unrestricted output, unrestricted path, or transport secret. Member progress projection SHALL identify the stable seat and bounded lifecycle milestone without creating a second Run lifecycle authority.

#### Scenario: Create a Local or SSH Run
- **WHEN** Agent generation is accepted for an eligible Runner
- **THEN** runner identity and recovery classification are committed with the canonical Run before it enters running

#### Scenario: Read an existing Run without runner metadata
- **WHEN** a legacy Run snapshot is loaded after migration
- **THEN** it remains readable and is conservatively projected as legacy Local only where existing ownership proves that classification
- **AND** no remote capability or live state is fabricated

#### Scenario: Project bounded member progress
- **WHEN** a child member Run starts, produces its first activity or output, waits, or terminates
- **THEN** the service SHALL expose its stable seat identity and bounded current milestone through the existing Run or stream boundary
- **AND** SHALL NOT expose secrets or unbounded raw process data in progress metadata
