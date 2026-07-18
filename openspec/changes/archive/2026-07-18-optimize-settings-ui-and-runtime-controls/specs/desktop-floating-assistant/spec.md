## ADDED Requirements

### Requirement: Efficient floating assistant settings synchronization
The floating-assistant settings surface SHALL avoid unnecessary UI refresh work while remaining synchronized with native configuration changes.

#### Scenario: Load floating assistant settings once per mount
- **WHEN** the Basic Configuration floating-assistant section mounts
- **THEN** it SHALL load runtime information and configuration through the floating-assistant service without repeated polling

#### Scenario: Update from configuration events
- **WHEN** the floating-assistant service emits a configuration-changed event
- **THEN** the settings surface SHALL update only the relevant configuration state

#### Scenario: Clean up event subscriptions
- **WHEN** the floating-assistant settings section unmounts
- **THEN** it SHALL release its event subscription so navigating settings pages does not accumulate listeners
