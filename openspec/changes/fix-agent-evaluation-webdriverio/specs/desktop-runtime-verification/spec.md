## ADDED Requirements

### Requirement: Agent evaluation has a focused WebdriverIO layer
The desktop verification orchestrator SHALL expose an independently runnable Agent-evaluation layer that builds or reuses the test desktop artifact, starts with isolated application state, drives the rendered evaluation workflow through WebdriverIO, and reports `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN` with a bounded evidence directory.

#### Scenario: Required fixture gate runs
- **WHEN** CI or a developer runs the required Agent-evaluation fixture path
- **THEN** the layer uses the repository OpenCode fixture, exercises IPC and rendered UI behavior, and requires no model credential

#### Scenario: Live-provider qualification runs
- **WHEN** a developer opts into live Agent-evaluation qualification
- **THEN** the layer preserves the host Agent executable path, checks provider-specific prerequisites before launch, and never joins the hermetic required gate

#### Scenario: One focused spec is diagnosed
- **WHEN** the operator selects the evaluation domain or UI spec for diagnosis
- **THEN** the layer runs only the selected allowlisted evaluation spec without allowing arbitrary filesystem paths

### Requirement: Agent evaluation qualification isolates credentials and state
The Agent-evaluation WebdriverIO layer SHALL use a fresh application data directory, SHALL pass live credentials only through a process-scoped environment boundary, and SHALL audit generated logs, screenshots, reports, and result metadata so credentials cannot be persisted as evidence.

#### Scenario: Provider credential is supplied
- **WHEN** a live OnePiece credential is available to the launcher
- **THEN** the desktop process can configure the isolated OnePiece profile without writing the credential to command arguments, repository files, or result metadata

#### Scenario: Qualification completes
- **WHEN** the WebdriverIO process exits in any status
- **THEN** the run context preserves only bounded safe evidence and cleans isolated runtime state according to the desktop harness policy
