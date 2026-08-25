## ADDED Requirements

### Requirement: Oversized output records degrade gracefully during streaming
When a single CLI output record exceeds the bounded parser limit during generation streaming, the runtime SHALL discard that record through its terminating newline and continue parsing subsequent records, instead of failing the generation. The effective bound SHALL be the parser policy's domain maximum rather than a smaller hardcoded value. Every discarded record SHALL be counted and reported after the stream ends as a redacted `warn` diagnostic in unified logs; genuinely malformed output (such as invalid UTF-8) SHALL continue to fail closed.

#### Scenario: One oversized record inside an otherwise healthy stream
- **WHEN** a CLI generation stream contains a record larger than the bounded parser limit followed by further well-formed records and a terminal completion event
- **THEN** the oversized record is discarded and the later records are parsed and delivered
- **AND** the generation completes according to the stream's terminal event rather than failing with a protocol error
- **AND** unified logs record a redacted `warn` naming the number of discarded oversized records

#### Scenario: The stream ends inside an oversized record
- **WHEN** the stream ends before an oversized record's terminating newline arrives
- **THEN** the partial oversized record is discarded rather than surfaced as truncated output or a protocol error

#### Scenario: Malformed output still fails closed
- **WHEN** a record within the bound contains invalid UTF-8
- **THEN** the generation fails with the existing protocol error semantics
