## ADDED Requirements

### Requirement: Native linker prerequisite validation
Continuous integration SHALL provision and exercise the declared linker prerequisites for every supported target that has a repository target-scoped linker policy.

#### Scenario: Validate Linux x86_64 native code
- **WHEN** CI validates `x86_64-unknown-linux-gnu`
- **THEN** the runner SHALL install or verify Clang and mold before invoking native compilation
- **AND** at least one validation step SHALL link a native artifact using the declared linker

#### Scenario: Validate Windows x86_64 MSVC native code
- **WHEN** CI validates `x86_64-pc-windows-msvc`
- **THEN** the runner SHALL verify that the selected Rust toolchain provides the declared LLD linker
- **AND** at least one validation step SHALL link a native artifact using that linker

#### Scenario: Linker prerequisite is unavailable
- **WHEN** a required linker or linker driver cannot be provisioned on a declared CI target
- **THEN** native validation SHALL fail before reporting the target as successfully validated
