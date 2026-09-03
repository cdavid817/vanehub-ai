## ADDED Requirements

### Requirement: Tray-background evolution maintenance
While the desktop process remains active in the tray, the runtime SHALL permit due internal Skill-evolution maintenance only when workspace policy, idle gating, budgets, and mutation safety checks pass. Hiding the window MUST NOT weaken automatic-application consent or safety requirements.

#### Scenario: Hidden desktop becomes idle
- **WHEN** the window is hidden to the tray and an enabled workspace has pending evolution work
- **THEN** the runtime may execute a bounded run under the same gates as a visible desktop

#### Scenario: Explicit quit begins
- **WHEN** the user requests graceful application quit
- **THEN** the runtime stops scheduling new evolution stages and checkpoints or recovers in-progress work before exit

#### Scenario: Tray is unavailable
- **WHEN** native tray initialization failed and normal close exits the process
- **THEN** the runtime does not claim that evolution work continues after process exit

