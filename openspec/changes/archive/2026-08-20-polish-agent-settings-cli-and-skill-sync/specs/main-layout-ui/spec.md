## ADDED Requirements

### Requirement: Continuous transcript and composer surface
The workspace chat SHALL present the transcript, status area, runner controls, and message composer as one continuous panel, and SHALL NOT decorate the attached composer as a second nested conversation card.

#### Scenario: Render an attached composer
- **WHEN** a Session chat with a message composer is displayed
- **THEN** the transcript and composer SHALL share one outer surface and one theme-aware separator
- **AND** the composer SHALL NOT add a competing outer shadow, detached gap, or mixed square-and-rounded conversation frame

#### Scenario: Focus the message input
- **WHEN** keyboard focus enters the message input
- **THEN** the input controls SHALL expose a visible semantic focus state without changing the outer workspace geometry
