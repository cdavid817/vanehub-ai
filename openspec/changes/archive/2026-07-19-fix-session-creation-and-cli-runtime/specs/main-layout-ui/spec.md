## ADDED Requirements

### Requirement: Session context menu pointer positioning

The main session context menu SHALL open near the user's right-click pointer and remain inside the visible viewport.

#### Scenario: Right-click lower sessions

- **WHEN** the user opens the context menu on any visible session row
- **THEN** the menu SHALL appear near the pointer position
- **AND** it SHALL NOT drift to the top of the page solely because the row is lower in the sidebar.

#### Scenario: Menu reaches viewport edge

- **WHEN** the preferred pointer-adjacent menu position would overflow the viewport
- **THEN** the menu SHALL flip or clamp using its measured rendered dimensions.
