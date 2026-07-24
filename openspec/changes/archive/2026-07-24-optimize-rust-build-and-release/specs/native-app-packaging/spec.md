## ADDED Requirements

### Requirement: Optimized distributable native artifacts
Native package commands and release workflows SHALL build distributable binaries with the declared optimized Cargo release profile.

#### Scenario: Run local native packaging
- **WHEN** a maintainer runs a supported local Tauri package command
- **THEN** the packaged native binary SHALL use the declared release optimization and debuginfo-stripping policy

#### Scenario: Run tagged release packaging
- **WHEN** the release workflow builds a supported native target
- **THEN** the packaged native binary SHALL use the same declared release profile as local release packaging
- **AND** the workflow SHALL fail rather than publish an artifact built without required linker prerequisites

### Requirement: Packaging optimization documentation
Packaging documentation SHALL identify the release profile, platform linker prerequisites, expected release-build tradeoffs, and retained production diagnostic behavior.

#### Scenario: Review release build guidance
- **WHEN** a maintainer prepares a local or CI package build
- **THEN** the documentation SHALL identify required linker tools for the target platform
- **AND** it SHALL explain that ThinLTO and a single codegen unit can increase release build time and can affect optimization or distributable size without guaranteeing a size reduction
- **AND** it SHALL distinguish measured artifact sizes from a demonstrated before/after size improvement
- **AND** it SHALL state that operational debug-level unified logs remain available in release packages
