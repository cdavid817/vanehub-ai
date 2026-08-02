# runtime-performance-governance Specification

## Purpose
TBD - created by archiving change optimize-runtime-performance-foundation. Update Purpose after archive.
## Requirements
### Requirement: Optimized release configuration guard
The project MUST automatically verify that distributable native builds use the approved optimized Cargo release profile and do not enable debug assertions or full debug information.

#### Scenario: Validate native build configuration
- **WHEN** the native architecture contract tests run
- **THEN** they SHALL require optimization level 3, ThinLTO, one code generation unit, and debuginfo stripping
- **AND** they SHALL reject release-profile debug assertions or debug information

### Requirement: Frontend artifact performance budget
The frontend production build MUST enforce versioned JavaScript artifact budgets using the generated Vite manifest.

#### Scenario: Validate a production frontend build
- **WHEN** the frontend chunk validation runs after a production build
- **THEN** the main static JavaScript closure SHALL NOT exceed 350 KiB gzip
- **AND** no emitted JavaScript chunk SHALL exceed 700 KiB raw
- **AND** a failure SHALL identify the measured artifact and budget

### Requirement: Deterministic performance regression coverage
The project MUST verify performance-sensitive data structures and query paths with deterministic automated tests rather than relying only on shared-host timing assertions.

#### Scenario: Run project validation
- **WHEN** automated tests validate settings loading, historical search, or retained terminal buffering
- **THEN** they SHALL verify first-visit mounting, indexed bounded query behavior, and bounded incremental transcript storage respectively
- **AND** the tests SHALL NOT require a fixed wall-clock latency on shared CI hosts
