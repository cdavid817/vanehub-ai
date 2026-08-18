## ADDED Requirements

### Requirement: Reliable Runner discovery and presentation
Mission Control SHALL expose a safe runner kind, runner capability state, and bounded Local host or SSH profile/host label derived from canonical Run metadata. It SHALL support reliable Local and SSH filtering and MUST NOT infer remote state from workspace text or owner identity.

#### Scenario: Filter Runs by Runner
- **WHEN** the user selects Local or SSH filtering
- **THEN** every returned row has matching persisted Runner metadata and pagination restarts from the first page

#### Scenario: Present Runner identity responsively
- **WHEN** a Run card renders in futuristic or minimal style at desktop or narrow width
- **THEN** its localized runner badge, safe host label, canonical state, attention, and actions remain readable without clipping, layout shift, or color-only meaning

### Requirement: Background and recovery visibility
Mission Control SHALL continue to show a Run after its Session page is no longer visible and SHALL distinguish running, disconnected, reconnecting, interrupted, and attention-required runner outcomes through canonical state plus bounded reason classifications. Open and cancel actions SHALL route to existing owning services.

#### Scenario: Reopen a background Run
- **WHEN** the user navigates away from an active Session and later opens its Mission Control row
- **THEN** Mission Control reconciles persisted canonical state and navigates to the authoritative Session without creating another execution

#### Scenario: Remote connection drops
- **WHEN** an SSH Runner reports disconnect or reconnect exhaustion
- **THEN** the row displays a localized safe runner reason and only actions allowed by canonical state, Runner policy, version, and permissions

