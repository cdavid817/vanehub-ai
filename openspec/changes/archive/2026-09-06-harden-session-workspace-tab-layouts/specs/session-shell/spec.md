## ADDED Requirements

### Requirement: A confirmed close leaves no error behind
Closing a Shell through the confirm dialog SHALL NOT surface an error for calls the panel made on that Shell's behalf while it was being closed.

#### Scenario: Reflow during close
- **WHEN** the reader confirms the close and the panel reflows as the dialog leaves
- **THEN** the surface SHALL NOT send a resize to a Shell that is no longer running
- **AND** a resize or write refused after the surface has been removed SHALL be discarded rather than shown as a workspace error

#### Scenario: Size after opening
- **WHEN** a Shell was fitted while still opening and then starts accepting input
- **THEN** the surface SHALL send its current size once, so the PTY is not left at its default dimensions
