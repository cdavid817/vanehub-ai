## ADDED Requirements

### Requirement: Hook installation requires the wrapper binary to exist
VaneHub SHALL NOT write a permission-hook entry into Claude Code's global settings unless the wrapper binary named by that entry is present on disk. When it is absent, enabling hook management SHALL fail with an error identifying the expected location, and SHALL leave the user's Claude Code settings unmodified.

#### Scenario: Wrapper binary is absent
- **WHEN** hook management is enabled and the wrapper binary is not present at its resolved path
- **THEN** the operation SHALL fail with an error naming that path
- **AND** Claude Code's global settings SHALL NOT be modified

#### Scenario: Wrapper binary is present
- **WHEN** hook management is enabled and the wrapper binary exists at its resolved path
- **THEN** the hook entries SHALL be written as before

#### Scenario: Removing a hook installed by an earlier build
- **WHEN** hook management is disabled while the wrapper binary is absent
- **THEN** the removal SHALL still clear VaneHub's entries from Claude Code's settings

### Requirement: Packaged applications carry the permission-hook wrapper
Every supported Tauri package SHALL include the permission-hook wrapper as a target-specific external binary installed beside the main application executable. Runtime hook configuration SHALL resolve that installed location before any resource-directory fallback.

#### Scenario: A supported target is packaged
- **WHEN** a package command runs for Windows x64, macOS arm64, macOS x64, or Linux x64
- **THEN** it SHALL build and stage the wrapper for the same Rust target before Tauri bundling
- **AND** the resulting package SHALL contain the wrapper beside the main executable

#### Scenario: Packaged hook management is enabled
- **WHEN** the installed wrapper is present beside the main executable
- **THEN** hook configuration SHALL name that executable

#### Scenario: An installation is incomplete or damaged
- **WHEN** the resolved packaged wrapper is absent
- **THEN** enabling hook management SHALL fail without modifying Claude Code's global settings
