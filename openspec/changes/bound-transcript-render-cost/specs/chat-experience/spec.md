## ADDED Requirements

### Requirement: Transcript rendering cost is bounded by what is visible
The rendering work the message list asks of the browser SHALL be governed by how much of the transcript is on screen rather than by how much history has been loaded. A message that has scrolled out of view SHALL remain in the document and reachable, and SHALL NOT keep costing style, layout and paint work while it is out of view.

#### Scenario: A long transcript is displayed
- **WHEN** a session has loaded far more messages than fit on screen
- **THEN** messages that are out of view SHALL NOT contribute their rendering work to each frame
- **AND** the messages in view SHALL render unchanged

#### Scenario: Scrolling through loaded history
- **WHEN** the reader scrolls back through messages that had left the viewport
- **THEN** those messages SHALL render with their real content and previous size
- **AND** the scroll position SHALL NOT shift as they come back into view

#### Scenario: Out-of-view messages stay reachable
- **WHEN** an out-of-view message is the target of find-in-page, assistive technology, or an in-page link
- **THEN** it SHALL still be found and presented
