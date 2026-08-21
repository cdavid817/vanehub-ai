## ADDED Requirements

### Requirement: CLI Agent terminal round-trip verification
Desktop verification SHALL prove, against the real native runtime, that a managed CLI Agent terminal starts, streams its output to the frontend, accepts input, and stops cleanly. The Agent binary under this layer MUST be a deterministic fixture that performs no network I/O and reads no credential store, so the layer's result depends on the runtime under test rather than on an installed Agent, a model provider, or an account.

#### Scenario: CLI terminal round trip succeeds
- **WHEN** the layer opens an Agent terminal for a CLI session whose Agent resolves to the fixture executable
- **THEN** the terminal SHALL reach `running` state with native capability
- **AND** the fixture's ready banner SHALL arrive at the frontend as terminal output
- **AND** content written through the Agent terminal input boundary SHALL come back in that Agent's terminal output
- **AND** stopping the terminal SHALL leave no owned Agent process running

#### Scenario: Fixture Agent cannot be resolved
- **WHEN** the fixture executable is absent from the resolution path, is not executable, or the Agent terminal never reaches `running`
- **THEN** the layer SHALL report `FAILED` and preserve its evidence
- **AND** it SHALL NOT fall back to a real installed CLI Agent

#### Scenario: Layer isolation from other desktop layers
- **WHEN** the CLI terminal layer runs
- **THEN** its fixture executable resolution SHALL NOT alter the environment of any other desktop verification layer
- **AND** each desktop verification layer SHALL report its own `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN` result and its own evidence directory
