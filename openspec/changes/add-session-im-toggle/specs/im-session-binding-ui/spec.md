## ADDED Requirements

### Requirement: Session IM opt-in control
The session information panel SHALL expose a localized, keyboard-operable IM switch for the selected session, and the switch SHALL represent persisted session state rather than connector-global state.

#### Scenario: Display the default state
- **WHEN** an eligible session with no stored IM enablement state is selected
- **THEN** the information panel SHALL show the IM switch as off
- **AND** pairing and binding mutation controls SHALL remain unavailable

#### Scenario: Enable from the information panel
- **WHEN** the user turns on the IM switch
- **THEN** the UI SHALL persist the selected session's enabled state through the runtime-neutral service
- **AND** after success it SHALL reveal Feishu connector health, pairing, and binding controls

#### Scenario: Disable a bound session
- **WHEN** the user turns off the IM switch for a session with an active Feishu binding
- **THEN** the UI SHALL explain that inbound execution will pause and require confirmation
- **AND** after confirmation it SHALL persist the disabled state and present the binding as paused by session opt-out

#### Scenario: Persist across desktop restart
- **WHEN** a user changes a session's IM switch and restarts the desktop client
- **THEN** the information panel SHALL restore the value from the native session service
- **AND** SHALL NOT use browser storage as persistence evidence

#### Scenario: Use the Web mock runtime
- **WHEN** the same control is used in the Web/mock runtime
- **THEN** the Web adapter SHALL provide deterministic equivalent state transitions through the same typed service contract

