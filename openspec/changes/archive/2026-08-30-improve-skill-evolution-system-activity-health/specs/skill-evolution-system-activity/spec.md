## MODIFIED Requirements

### Requirement: Projection health and lag
The system SHALL expose projector state, lease owner, last successful source cursor by domain, pending count, oldest pending age, failed event categories, gaps, rebuild state, and last completed projection time. The maintenance UI SHALL present lease state, per-domain cursor sequence, pending count, oldest pending time, gap and failure codes, and a bounded recent rebuild history using locale-aware labels without exposing source payloads.

#### Scenario: One source domain is delayed
- **WHEN** generation events lag while other domains project normally
- **THEN** health identifies the affected domain without marking unrelated source outcomes failed

#### Scenario: Operator inspects unhealthy projection state
- **WHEN** a domain has pending work, a source gap, or a projection failure
- **THEN** the maintenance UI identifies that domain and displays its safe cursor, backlog, gap, and failure diagnostics

#### Scenario: Recent rebuild evidence is available
- **WHEN** projection health includes rebuild records
- **THEN** the maintenance UI shows a bounded recent history with scope identity, status, and processed-item totals

### Requirement: Deterministic projection rebuild
The system SHALL support scoped rebuild from retained authoritative audit records into a new projection generation, validate counts and hashes, and atomically activate the rebuilt generation. Rebuild MUST NOT call models, rerun assessments, modify Skills or Overlays, resend already delivered notifications, or change governance decisions. While a user-requested rebuild is active, the maintenance UI SHALL display its current phase and processed-item progress, prevent a duplicate start, and provide cancellation through the system-activity service boundary. Cancelling MUST leave the previous valid generation available.

#### Scenario: User requests workspace rebuild
- **WHEN** projection health reports corruption or a version upgrade requires rebuild
- **THEN** the system creates a bounded rebuild attempt and keeps the last valid generation readable until replacement validates

#### Scenario: Rebuild output differs unexpectedly
- **WHEN** source receipts and projection version predict a different count or hash
- **THEN** activation fails and the previous valid generation remains active

#### Scenario: Rebuild advances through maintenance phases
- **WHEN** a rebuild processes, validates, catches up, or activates a shadow generation
- **THEN** the maintenance UI reports the current phase and processed items and does not allow another rebuild to start concurrently

#### Scenario: User cancels an active rebuild
- **WHEN** the user requests cancellation while a rebuild is in progress
- **THEN** the system cancels through the service boundary, reports completion of cancellation, and keeps the previous projection available
