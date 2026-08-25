## MODIFIED Requirements

### Requirement: Service-backed CLI refresh state

The frontend SHALL represent all-tool and single-tool CLI refresh loading, queued, running, success, partial-completion, cancellation, and failure states through the CLI service and common operation service.

#### Scenario: Refresh state uses service boundary

- **WHEN** the CLI Management page starts or observes a CLI refresh operation
- **THEN** React components SHALL use `AgentService`/`CliToolService` and `OperationService`
- **AND** React components SHALL NOT import or call Tauri APIs directly

#### Scenario: Targeted refresh preserves unrelated state

- **WHEN** a targeted refresh runs for one stable Agent id
- **THEN** the frontend SHALL preserve cached snapshots, controls, and interaction for unrelated tools

#### Scenario: Stale data remains visible

- **WHEN** a refresh starts while a cached snapshot exists
- **THEN** the page SHALL keep that snapshot visible with stale or refreshing state instead of replacing it with a blank blocking surface

#### Scenario: Web runtime simulates refresh state

- **WHEN** the page runs in Web/mock runtime and refresh is requested
- **THEN** the Web adapter SHALL return a deterministic observable operation without inspecting the host or writing a local log file

### Requirement: Detailed CLI environment adapter parity

The Tauri and Web/mock Agent service adapters SHALL implement the same normalized source-aware CLI environment, action-plan, bulk-plan, diagnostics, and operation result contracts.

#### Scenario: Desktop adapter returns native status

- **WHEN** the desktop frontend lists CLI environments
- **THEN** only the Tauri adapter SHALL invoke native commands
- **AND** it SHALL return cached installations, active identity, source confidence, orthogonal state, source catalogs, allowed actions, conflicts, freshness, and last mutation summary

#### Scenario: Web adapter remains honest

- **WHEN** the Web/mock frontend lists CLI environments
- **THEN** it SHALL return deterministic fixtures with the same contract shape
- **AND** it SHALL not invent real host paths, credentials, installed versions, or package-manager effects

#### Scenario: Contract shape changes

- **WHEN** CLI environment, plan, diagnostic, or result fields change
- **THEN** shared contract verification and adapter conformance tests SHALL fail until Rust, TypeScript, Tauri, and Web/mock mappings agree

### Requirement: Frontend critical CLI failure reporting

The frontend SHALL surface typed CLI planning, execution, refresh, Doctor, and adapter-start failures without parsing arbitrary error strings and SHALL report durable desktop diagnostics through the logging service boundary where required.

#### Scenario: Report refresh start failure

- **WHEN** a CLI refresh request fails before the backend returns an operation id
- **THEN** the frontend SHALL display a localized message derived from the typed category
- **AND** the Tauri runtime SHALL report durable diagnostic context through the logging service boundary

#### Scenario: Report package start failure

- **WHEN** a CLI planning, lifecycle execution, bulk, or Doctor request fails before the backend returns an operation id
- **THEN** the frontend SHALL display a localized message derived from the typed category
- **AND** the Tauri runtime SHALL report durable diagnostic context through the logging service boundary

#### Scenario: Operation reaches a terminal warning

- **WHEN** a CLI operation returns applied-unverified or changed-but-failed
- **THEN** the frontend SHALL show the typed warning and recommended next action
- **AND** it SHALL retain the affected cached snapshot with accurate freshness

## ADDED Requirements

### Requirement: Plan-driven CLI lifecycle adapter flow

The frontend SHALL prepare, retrieve, review, and execute persisted CLI plans through service interfaces rather than sending lifecycle command details directly to the runtime.

#### Scenario: User selects a version

- **WHEN** the user selects source `S`, channel `C`, and target `X`
- **THEN** the frontend SHALL pass exactly `S`, `C`, and `X` to `prepareCliAction`
- **AND** it SHALL not replace `X` with a latest-version field

#### Scenario: User confirms a plan

- **WHEN** a review dialog displays a persisted plan
- **THEN** execution SHALL submit only the plan id and expected revision
- **AND** the frontend SHALL not reconstruct or modify the command

#### Scenario: Target equals current

- **WHEN** the backend reports current state for the selected target
- **THEN** the frontend SHALL disable redundant mutation
- **AND** it SHALL not create an execution operation

### Requirement: Per-tool CLI operation isolation

The frontend SHALL derive each tool's busy, queued, progress, and terminal state from related CLI operations rather than one global mutation boolean.

#### Scenario: One tool is running

- **WHEN** an operation is related to one Agent id
- **THEN** only that tool SHALL show the operation as active
- **AND** unrelated tool details, filtering, and cached reads SHALL remain available

#### Scenario: Backend queues a conflicting action

- **WHEN** a prepared action is queued by the mutation coordinator
- **THEN** the affected tool SHALL show queued state
- **AND** the frontend SHALL not pretend that execution has started

### Requirement: CLI action and bulk plan Web parity

The Web/mock adapter SHALL support deterministic plan preparation, retrieval, review, execution, cancellation, stale-plan failure, and bulk item outcomes.

#### Scenario: Web plan succeeds

- **WHEN** a mock plan is prepared and executed
- **THEN** the adapter SHALL expose queued, running, and terminal states compatible with the desktop contract

#### Scenario: Web plan is cancelled or stale

- **WHEN** a mock operation is cancelled or a fixture fingerprint changes
- **THEN** the adapter SHALL return the corresponding typed terminal state without native side effects
