## ADDED Requirements

### Requirement: Session rows stay inside the sidebar
A session row SHALL never be wider than the session sidebar column, whatever its title length and whatever width the divider has been dragged to.

#### Scenario: Long title in a narrow sidebar
- **WHEN** a session title is wider than the sidebar and the sidebar is at its minimum or a dragged width
- **THEN** the row SHALL truncate the title inside the column
- **AND** no part of the row SHALL lie under the divider or the conversation surface

#### Scenario: Labels never fold
- **WHEN** a badge such as a source label is placed in a tight row
- **THEN** it SHALL keep its width and stay on one line rather than wrapping its characters
