# native-build-optimization Specification

## Purpose
TBD - created by archiving change optimize-rust-build-and-release. Update Purpose after archive.
## Requirements
### Requirement: Target-scoped native linkers
The project SHALL select a declared linker only for native target triples whose linker configuration and prerequisites are validated by the repository.

#### Scenario: Build Windows x86_64 MSVC target
- **WHEN** a native build targets `x86_64-pc-windows-msvc`
- **THEN** Cargo SHALL use the Rust-toolchain-provided LLD linker
- **AND** the build SHALL remain compatible with the required MSVC libraries and Windows packaging toolchain

#### Scenario: Build Linux x86_64 GNU target
- **WHEN** a native build targets `x86_64-unknown-linux-gnu`
- **THEN** Cargo SHALL use Clang as the linker driver and mold as the ELF linker
- **AND** the build environment SHALL provide both declared tools before compilation starts

#### Scenario: Build an undeclared target
- **WHEN** a native build targets a platform or architecture without a validated target-scoped linker policy
- **THEN** the project SHALL NOT apply incompatible linker flags from a different target

### Requirement: Optimized native release profile
The native application SHALL use an explicit release profile that enables optimization level 3, ThinLTO, one code generation unit, and debuginfo stripping for distributable builds.

#### Scenario: Build a release binary
- **WHEN** Cargo builds the native application with the release profile
- **THEN** the build SHALL use `opt-level = 3`, `lto = "thin"`, `codegen-units = 1`, and `strip = "debuginfo"`

#### Scenario: Build a development binary
- **WHEN** Cargo builds the native application with the development profile
- **THEN** the release-only LTO, codegen-unit, and stripping policy SHALL NOT disable incremental development behavior

### Requirement: Production diagnostic preservation
Native build optimization MUST preserve every production unified-log severity and MUST restrict `debug_assertions` conditional compilation to behavior without a production diagnostic contract.

#### Scenario: Compile a release build
- **WHEN** the native application is compiled without debug assertions
- **THEN** the release binary SHALL retain `error`, `warn`, `info`, and `debug` unified-log level semantics
- **AND** operational debug-level events, redaction, persistence, and serialization paths SHALL remain available

#### Scenario: Gate development-only behavior
- **WHEN** implementation code uses `#[cfg(debug_assertions)]`
- **THEN** the gated behavior SHALL be development-only
- **AND** removing it from a release build SHALL NOT remove required diagnostics or change a service/runtime contract

### Requirement: Comparable build optimization evidence
The implementation SHALL record comparable evidence for linker performance and release artifact size before claiming an optimization result, and SHALL report unavailable, neutral, or negative results without inferring an improvement.

#### Scenario: Compare incremental native builds
- **WHEN** linker performance is evaluated
- **THEN** baseline and optimized measurements SHALL use the same source state and comparable cache state
- **AND** the result SHALL distinguish cold dependency compilation from representative incremental relinking
- **AND** the result SHALL state whether the measured linker improved, regressed, or remained neutral

#### Scenario: Compare release artifact size
- **WHEN** release profile optimization is evaluated
- **THEN** every available baseline and optimized executable or package size SHALL identify the host, target, toolchain, profile, and artifact measured
- **AND** if a comparable baseline artifact is unavailable, the evidence SHALL identify the missing comparison and SHALL NOT claim a size reduction

### Requirement: Worktree build behavior documentation
Maintainer documentation SHALL explain that ignored Cargo target output is worktree-local by default and SHALL distinguish cold-build cost from incremental linker cost.

#### Scenario: Build from a fresh worktree
- **WHEN** a maintainer prepares to build the native application from a new Git worktree
- **THEN** the documentation SHALL identify that the first build may compile the complete dependency graph
- **AND** it SHALL describe how to verify the active platform linker before interpreting performance results
