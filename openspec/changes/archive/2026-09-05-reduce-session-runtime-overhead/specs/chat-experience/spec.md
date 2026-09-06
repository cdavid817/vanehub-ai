## ADDED Requirements

### Requirement: Streaming cost stays bounded as a response grows
A streaming assistant response SHALL keep its per-update rendering and persistence cost bounded as the accumulated response grows. Re-rendering of the streaming row SHALL be paced rather than driven by every incoming frame, and persisting a streamed delta SHALL write the delta rather than rewriting content the delta did not touch.

#### Scenario: A long response streams
- **WHEN** an assistant response streams for long enough to accumulate a large body of text
- **THEN** the streaming row SHALL re-render at a bounded rate rather than once per animation frame
- **AND** already-completed messages SHALL NOT re-render because of it

#### Scenario: A streamed delta is persisted
- **WHEN** a streamed content delta is flushed to durable storage
- **THEN** the write SHALL append the delta to the stored content
- **AND** it SHALL leave structured columns that the delta did not modify unchanged

#### Scenario: Streamed content survives interruption
- **WHEN** a client restart follows a stream that was interrupted mid-response
- **THEN** the content persisted before the interruption SHALL still be present and correctly ordered
