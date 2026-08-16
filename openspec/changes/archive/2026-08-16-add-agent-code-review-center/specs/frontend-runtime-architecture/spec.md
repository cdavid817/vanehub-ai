## ADDED Requirements

### Requirement: Runtime-neutral code review service contract
The frontend Agent service SHALL expose review lifecycle, bounded file loading, comment/finding/decision, feedback, action, and guarded-revert methods whose models and terminal semantics are implemented consistently by Tauri and Web/mock adapters.

#### Scenario: React requests review work
- **WHEN** a Review Center component creates a review, loads a diff, comments, sends feedback, or starts an action
- **THEN** it SHALL call the shared service interface and SHALL NOT import or invoke Tauri APIs directly

#### Scenario: Adapters expose parity
- **WHEN** contract tests compare Tauri declarations and Web/mock behavior
- **THEN** both adapters SHALL expose matching request/response shapes, stale/error categories, operation states, and simulated-receipt semantics
