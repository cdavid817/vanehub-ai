## MODIFIED Requirements

### Requirement: Service-backed CLI refresh state
The frontend SHALL represent all-tool and single-tool CLI refresh loading, running, success, and failure states through service-backed data and operation status.

#### Scenario: Refresh state uses service boundary
- **WHEN** the CLI management settings page starts or observes a CLI refresh operation
- **THEN** React components SHALL use the Agent service and operation service interfaces to derive page or card refresh state
- **AND** React components SHALL NOT import or call Tauri APIs directly

#### Scenario: Targeted refresh preserves unrelated state
- **WHEN** a targeted refresh runs for one stable agent id
- **THEN** the frontend SHALL preserve cached status and interaction for unrelated CLI cards

#### Scenario: Web runtime simulates refresh state
- **WHEN** the CLI management settings page runs in the Web/mock runtime and a refresh is requested
- **THEN** the Web adapter SHALL return a mock operation that allows the page to show refresh-in-progress behavior without requiring native commands or writing local log files

## ADDED Requirements

### Requirement: Detailed CLI environment adapter parity
The Tauri and Web/mock Agent service adapters SHALL implement the same normalized detailed CLI environment contract.

#### Scenario: Desktop adapter returns native status
- **WHEN** the desktop frontend requests CLI status
- **THEN** only the Tauri adapter SHALL invoke native commands and SHALL return cached installation distribution, active entry, source, environment, health, conflict, and lifecycle eligibility fields

#### Scenario: Web adapter remains honest
- **WHEN** the Web/mock frontend requests CLI status
- **THEN** the Web adapter SHALL return the fixed supported catalog with unsupported native detection and empty installation distribution rather than fake host paths or versions

#### Scenario: Contract shape changes
- **WHEN** detailed CLI environment fields are added or changed
- **THEN** shared contract conformance and adapter tests SHALL verify equivalent field shapes for both runtime implementations

