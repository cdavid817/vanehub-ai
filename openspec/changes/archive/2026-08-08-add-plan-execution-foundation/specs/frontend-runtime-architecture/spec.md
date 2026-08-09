## ADDED Requirements

### Requirement: Plan frontend service boundary
The frontend SHALL access Plan drafting, version inspection, editing, validation, approval, run summaries, run details, evidence, controls, and recovery actions through typed service interfaces implemented by both Tauri and Web/mock adapters, and React components SHALL NOT invoke Tauri commands or access SQLite directly.

#### Scenario: Use the desktop Plan adapter
- **WHEN** a Plan UI action runs in the Tauri desktop runtime
- **THEN** the Tauri-specific adapter SHALL invoke a declared native Plan command and normalize its typed response through the shared service interface

#### Scenario: Use the Web/mock Plan adapter
- **WHEN** the same Plan UI action runs in the browser Web/mock runtime
- **THEN** the Web/mock adapter SHALL return deterministic compatible state without importing Tauri APIs or claiming to execute a native provider, Git command, or SQLite mutation

### Requirement: Plan adapter contract parity
The Tauri and Web/mock Plan adapters SHALL expose the same normalized Plan, version, SubTask, dependency, validation, PlanRun summary, PlanRun detail, attempt evidence, and control result shapes.

#### Scenario: Run Plan adapter conformance tests
- **WHEN** shared adapter contract tests execute equivalent fixtures against both runtime adapters
- **THEN** both adapters SHALL satisfy the same method signatures, state values, graph shapes, validation errors, and control-result semantics

### Requirement: Bounded Plan UI projections
The Plan frontend contract SHALL separate paginated run summaries from bounded Plan and PlanRun detail projections and SHALL NOT require list polling to transfer full Agent transcripts, raw tool results, or complete historical evidence.

#### Scenario: Refresh active Plan progress
- **WHEN** the UI refreshes an active PlanRun
- **THEN** it SHALL request a bounded state projection containing task statuses, safe progress metadata, and available controls while loading detailed evidence only on explicit inspection

