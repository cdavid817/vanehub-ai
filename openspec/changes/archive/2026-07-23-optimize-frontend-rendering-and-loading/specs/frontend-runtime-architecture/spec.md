## ADDED Requirements

### Requirement: Feature module code splitting
The shared React frontend SHALL dynamically import designated heavy, non-default feature modules in both Tauri desktop and browser Web/mock runtimes without bypassing the frontend service boundary.

#### Scenario: Start the application
- **WHEN** the application loads before the user opens a designated non-default feature
- **THEN** the initial entry chunk SHALL NOT contain that feature module
- **AND** the surrounding shell SHALL remain interactive

#### Scenario: Open a lazy feature
- **WHEN** the user first opens a designated lazy-loaded feature
- **THEN** the frontend SHALL load its module and show a size-stable localized loading state until it resolves
- **AND** the feature SHALL use the same `agentService` boundary as an eagerly loaded feature

#### Scenario: Recover from a module load failure
- **WHEN** a designated feature module fails to load
- **THEN** the feature boundary SHALL show a localized error with a retry action
- **AND** unrelated application surfaces SHALL remain mounted and operable

#### Scenario: Retry a failed module
- **WHEN** the user activates retry after a module load failure
- **THEN** the frontend SHALL make a new module-load attempt without resetting unrelated shell state
